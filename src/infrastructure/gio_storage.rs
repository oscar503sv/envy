//! Synchronous storage for local file URIs, including document portal paths.
//! Run operations on a worker thread off the GTK main thread.

use std::{
    ffi::CStr,
    fs::{File, Metadata, OpenOptions},
    io::{self, Read},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

use gio::{FileInfo, FileQueryInfoFlags, FileType, IOErrorEnum, glib, prelude::*};

use crate::{
    domain::redact::Redacted,
    services::storage::{
        DiskVersion, DocumentStorage, FileIdentity, ParentDirectory, ReadSnapshot, ResourceRef,
        StorageCapabilities, StorageError, content_hash,
    },
};

/// The lower end of the approved range limits downstream parser memory amplification.
pub const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;

const FILE_ATTRIBUTES: &str = concat!(
    "standard::type,standard::is-symlink,standard::size,etag::value,time::modified,",
    "unix::device,unix::inode,unix::uid,unix::gid,unix::mode,unix::nlink,",
    "access::can-read,access::can-write"
);
const FUSE_SUPER_MAGIC: u32 = 0x6573_5546;
const PERMISSIONS_MASK: u32 = 0o7777;
const STICKY_BIT: u32 = 0o1000;
const WRITE_BITS: u32 = 0o222;

/// Stateless adapter. Each operation creates its own GIO handles.
#[derive(Debug, Default, Clone, Copy)]
pub struct GioStorage;

impl DocumentStorage for GioStorage {
    fn read(&self, resource: &ResourceRef) -> Result<ReadSnapshot, StorageError> {
        let (file, path) = resolve(resource)?;
        let info = query(&file, FILE_ATTRIBUTES)?;
        let size = checked_size(&info)?;
        let mut identity = file_identity(&info, &path, None);
        if info.has_attribute("access::can-read") && !info.boolean("access::can-read") {
            return Err(StorageError::PermissionDenied);
        }

        // O_PATH pins the leaf without opening a raced-in special file for I/O.
        // O_NOFOLLOW gives us the symlink itself if the leaf changed to one.
        let pinned = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|error| match map_io(error) {
                StorageError::NotFound => StorageError::Conflict,
                error => error,
            })?;
        let before = pinned.metadata().map_err(map_io)?;
        verify_descriptor(&before, &identity, size)?;

        // Reopen the verified inode through procfs while its O_PATH handle is live.
        // Reopen errors describe access through procfs and carry a generic I/O
        // kind so callers do not interpret them as resource access revocation.
        let mut stream = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(format!("/proc/self/fd/{}", pinned.as_raw_fd()))
            .map_err(|error| StorageError::Io { kind: error.kind() })?;
        verify_descriptor(&stream.metadata().map_err(map_io)?, &identity, size)?;
        identity = file_identity(&info, &path, Some(&stream));

        let mut bytes = Vec::with_capacity(size as usize + 1);
        (&mut stream)
            .take(MAX_FILE_SIZE + 1)
            .read_to_end(&mut bytes)
            .map_err(map_io)?;
        if bytes.len() as u64 > MAX_FILE_SIZE {
            return Err(StorageError::TooLarge);
        }

        let current = query(&file, FILE_ATTRIBUTES).map_err(|error| match error {
            StorageError::NotFound => StorageError::Conflict,
            error => error,
        })?;
        // Compare GIO path identity with GIO path identity, and sample xattrs
        // from the same verified read descriptor on both sides of the read.
        let current_identity = file_identity(&current, &path, Some(&stream));
        let after = stream.metadata().map_err(map_io)?;
        verify_descriptor(&after, &identity, size)?;
        if bytes.len() as u64 != size
            || before.modified().ok() != after.modified().ok()
            || (before.ctime(), before.ctime_nsec()) != (after.ctime(), after.ctime_nsec())
            || checked_size(&current).map_err(|_| StorageError::Conflict)? != size
            || current.etag() != info.etag()
            || current_identity != identity
        {
            return Err(StorageError::Conflict);
        }

        let version = DiskVersion {
            hash: Redacted::new(content_hash(&bytes)),
            etag: info.etag().map(|etag| Redacted::new(etag.into())),
            size,
            mtime: after.modified().ok(),
            identity: Some(identity),
        };
        Ok(ReadSnapshot::new(bytes, version))
    }

    fn capabilities(&self, resource: &ResourceRef) -> StorageCapabilities {
        let is_writable = resolve(resource)
            .and_then(|(file, _)| query(&file, FILE_ATTRIBUTES))
            .is_ok_and(|info| {
                info.file_type() == FileType::Regular
                    && !info.is_symlink()
                    && info.boolean("access::can-write")
                    && permission_mode(&info).is_none_or(|mode| mode & WRITE_BITS != 0)
            });
        // Replacement capabilities stay conservative until ENV-T028.
        StorageCapabilities {
            is_writable,
            ..StorageCapabilities::default()
        }
    }

    /// Replacement is reserved for ENV-T028. This operation performs no I/O.
    fn replace_if_unchanged(
        &self,
        _resource: &ResourceRef,
        _expected: &DiskVersion,
        _content: &[u8],
    ) -> Result<DiskVersion, StorageError> {
        Err(StorageError::ConditionalReplaceUnsupported)
    }
}

fn resolve(resource: &ResourceRef) -> Result<(gio::File, PathBuf), StorageError> {
    if resource.uri().contains('\0') {
        return Err(StorageError::Io {
            kind: io::ErrorKind::InvalidInput,
        });
    }
    let file = gio::File::for_uri(resource.uri());
    if file.has_uri_scheme("file") {
        let uri = glib::Uri::parse(resource.uri(), glib::UriFlags::NONE)
            // Local filenames can contain non-UTF-8 bytes. Let GIO decode the path.
            .or_else(|_| glib::Uri::parse(resource.uri(), glib::UriFlags::ENCODED_PATH))
            .map_err(|_| StorageError::Io {
                kind: io::ErrorKind::InvalidInput,
            })?;
        // Only local authorities are valid for the local storage adapter.
        if uri.userinfo().is_some()
            || uri.port() != -1
            || uri
                .host()
                .is_some_and(|host| !host.is_empty() && !host.eq_ignore_ascii_case("localhost"))
        {
            return Err(StorageError::Io {
                kind: io::ErrorKind::InvalidInput,
            });
        }
    }
    let path = file
        .path()
        .filter(|_| file.has_uri_scheme("file"))
        .ok_or(StorageError::Io {
            kind: io::ErrorKind::Unsupported,
        })?;
    Ok((file, path))
}

fn query(file: &gio::File, attributes: &str) -> Result<FileInfo, StorageError> {
    file.query_info(
        attributes,
        FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
        gio::Cancellable::NONE,
    )
    .map_err(map_gio)
}

fn checked_size(info: &FileInfo) -> Result<u64, StorageError> {
    if info.is_symlink() || info.file_type() == FileType::SymbolicLink {
        return Err(StorageError::Symlink);
    }
    if info.file_type() != FileType::Regular {
        return Err(StorageError::NotRegularFile);
    }
    let size = info
        .has_attribute("standard::size")
        .then(|| u64::try_from(info.size()).ok())
        .flatten()
        .ok_or(StorageError::Io {
            kind: io::ErrorKind::InvalidData,
        })?;
    if size > MAX_FILE_SIZE {
        return Err(StorageError::TooLarge);
    }
    Ok(size)
}

fn verify_descriptor(
    metadata: &Metadata,
    identity: &FileIdentity,
    size: u64,
) -> Result<(), StorageError> {
    if !metadata.is_file()
        || identity.device_and_inode != Some((metadata.dev(), metadata.ino()))
        || identity.uid.is_some_and(|uid| uid != metadata.uid())
        || identity.gid.is_some_and(|gid| gid != metadata.gid())
        || identity
            .mode
            .is_some_and(|mode| mode != metadata.mode() & PERMISSIONS_MASK)
        || identity
            .nlink
            .is_some_and(|nlink| nlink != metadata.nlink())
        || size != metadata.len()
    {
        return Err(StorageError::Conflict);
    }
    Ok(())
}

fn uint32(info: &FileInfo, name: &str) -> Option<u32> {
    info.has_attribute(name)
        .then(|| info.attribute_uint32(name))
}

fn permission_mode(info: &FileInfo) -> Option<u32> {
    uint32(info, "unix::mode").map(|mode| mode & PERMISSIONS_MASK)
}

fn file_identity(info: &FileInfo, path: &Path, stream: Option<&File>) -> FileIdentity {
    let portal_path = portal_relative_path(path);
    FileIdentity {
        device_and_inode: uint32(info, "unix::device").map(u64::from).zip(
            info.has_attribute("unix::inode")
                .then(|| info.attribute_uint64("unix::inode")),
        ),
        uid: uint32(info, "unix::uid"),
        gid: uint32(info, "unix::gid"),
        mode: permission_mode(info),
        nlink: uint32(info, "unix::nlink").map(u64::from),
        is_symlink: info
            .has_attribute("standard::is-symlink")
            .then(|| info.is_symlink()),
        parent: if portal_path
            .as_ref()
            .is_some_and(|path| path.components().count() <= 2)
        {
            // A single-file export has a synthetic parent directory.
            None
        } else {
            stream.and_then(|_| parent_identity(path.parent()?))
        },
        has_access_acl: stream.and_then(|fd| acl_presence(fd, c"system.posix_acl_access")),
        selinux_label: if portal_path.is_some() {
            None
        } else {
            stream.and_then(selinux_label)
        },
    }
}

fn selinux_label(stream: &File) -> Option<Option<Redacted<Vec<u8>>>> {
    let Some(mut label) = read_xattr(stream, c"security.selinux")? else {
        return Some(None);
    };
    while label.last() == Some(&0) {
        label.pop();
    }
    if label
        .split(|byte| *byte == b':')
        .any(|part| part == b"fusefs_t")
    {
        return None;
    }
    Some(Some(Redacted::new(label)))
}

fn portal_relative_path(path: &Path) -> Option<&Path> {
    let mut components = path.strip_prefix("/run/user").ok()?.components();
    let uid = components.next()?.as_os_str().to_str()?;
    if uid.parse::<u32>().is_err() || components.next()?.as_os_str() != "doc" {
        return None;
    }
    Some(components.as_path())
}

fn parent_identity(path: &Path) -> Option<ParentDirectory> {
    // Follow parent symlinks as ordinary path lookup does. O_NOFOLLOW here
    // would reject a symlink naming the directory containing the file.
    let parent = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(path)
        .ok()?;
    let info = parent.metadata().ok()?;
    let mode = info.mode() & PERMISSIONS_MASK;
    Some(ParentDirectory {
        uid: Some(info.uid()),
        mode: Some(mode),
        is_sticky: Some(mode & STICKY_BIT != 0),
        has_default_acl: acl_presence(&parent, c"system.posix_acl_default"),
    })
}

fn acl_presence(file: &File, name: &CStr) -> Option<bool> {
    read_xattr(file, name).map(|value| value.is_some())
}

fn read_xattr(file: &File, name: &CStr) -> Option<Option<Vec<u8>>> {
    let read = || {
        let size = get_xattr(file, name, &mut [])?;
        let mut value = vec![0; size];
        let size = get_xattr(file, name, &mut value)?;
        // A zero-length buffer acts as a size query if the value grew.
        if size > value.len() {
            return Err(libc::ERANGE);
        }
        value.truncate(size);
        Ok(value)
    };
    let result = match read() {
        Err(libc::ERANGE) => read(),
        result => result,
    };
    match result {
        Ok(value) => Some(Some(value)),
        Err(libc::ENODATA) => Some(None),
        Err(libc::ENOTSUP) if is_non_fuse(file) => Some(None),
        Err(_) => None,
    }
}

#[allow(unsafe_code, reason = "Linux descriptor-based xattr query")]
fn get_xattr(file: &File, name: &CStr, buffer: &mut [u8]) -> Result<usize, i32> {
    // SAFETY: File keeps the fd live, name is NUL-terminated, and buffer is
    // writable for its full length. A zero length requests only the size.
    let result = unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            name.as_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
        )
    };
    usize::try_from(result).map_err(|_| io::Error::last_os_error().raw_os_error().unwrap_or(0))
}

#[allow(
    unsafe_code,
    reason = "Linux descriptor-based filesystem identification"
)]
fn is_non_fuse(file: &File) -> bool {
    let mut stats = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: The fd is live and stats points to writable statfs storage.
    // fstatfs initializes the structure on success.
    unsafe {
        libc::fstatfs(file.as_raw_fd(), stats.as_mut_ptr()) == 0
            && stats.assume_init().f_type as u32 != FUSE_SUPER_MAGIC
    }
}

fn map_gio(error: glib::Error) -> StorageError {
    let kind = match error.kind::<IOErrorEnum>() {
        Some(IOErrorEnum::NotFound | IOErrorEnum::NotDirectory) => return StorageError::NotFound,
        Some(IOErrorEnum::PermissionDenied) => return StorageError::PermissionDenied,
        Some(IOErrorEnum::IsDirectory | IOErrorEnum::NotRegularFile) => {
            return StorageError::NotRegularFile;
        }
        Some(IOErrorEnum::NotSupported) => io::ErrorKind::Unsupported,
        Some(IOErrorEnum::InvalidArgument | IOErrorEnum::InvalidFilename) => {
            io::ErrorKind::InvalidInput
        }
        Some(IOErrorEnum::TimedOut) => io::ErrorKind::TimedOut,
        Some(IOErrorEnum::WouldBlock) => io::ErrorKind::WouldBlock,
        _ => io::ErrorKind::Other,
    };
    StorageError::Io { kind }
}

fn map_io(error: io::Error) -> StorageError {
    match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => StorageError::NotFound,
        io::ErrorKind::PermissionDenied => StorageError::PermissionDenied,
        kind => StorageError::Io { kind },
    }
}
