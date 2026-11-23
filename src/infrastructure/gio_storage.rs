//! Synchronous storage for local file URIs, including document portal paths.
//! Run operations on a worker thread off the GTK main thread.

use std::{
    ffi::{CStr, CString},
    fs::{File, Metadata, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
        },
    },
    path::{Path, PathBuf},
};

use gio::{FileInfo, FileQueryInfoFlags, FileType, IOErrorEnum, glib, prelude::*};

use crate::{
    domain::redact::Redacted,
    services::storage::{
        DiskVersion, DocumentStorage, FileIdentity, ParentDirectory, PostWriteFailure,
        ReadSnapshot, ResourceRef, StorageCapabilities, StorageError, content_hash,
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
        read_snapshot(&file, &path, &info).map(|(snapshot, _)| snapshot)
    }

    /// Advisory only. The save repeats preflight, including the bounded read.
    fn capabilities(&self, resource: &ResourceRef) -> StorageCapabilities {
        let mut capabilities = StorageCapabilities::default();
        let _ = preflight(resource, &mut capabilities);
        capabilities
    }

    /// Best-effort change detection. Another writer can still race the final
    /// identity check and rename (D-12.1 / R-01).
    fn replace_if_unchanged(
        &self,
        resource: &ResourceRef,
        expected: &DiskVersion,
        content: &[u8],
    ) -> Result<DiskVersion, StorageError> {
        let prepared = preflight(resource, &mut StorageCapabilities::default())?;
        if &prepared.version != expected {
            return Err(StorageError::Conflict);
        }
        if content.len() as u64 > MAX_FILE_SIZE {
            return Err(StorageError::TooLarge);
        }
        let base = prepared
            .version
            .identity
            .as_ref()
            .ok_or(StorageError::PermissionsNotPreserved)?;
        let post_error = |reason| StorageError::PostWriteVerificationFailed { reason };
        let written_inode = prepared.replace(base, content)?;
        let version = self
            .read(resource)
            .map_err(|error| {
                post_error(match error {
                    StorageError::TooLarge => PostWriteFailure::Content,
                    _ => PostWriteFailure::Identity,
                })
            })?
            .version;
        if version.hash.expose() != &content_hash(content) {
            return Err(post_error(PostWriteFailure::Content));
        }
        let identity = version
            .identity
            .as_ref()
            .ok_or_else(|| post_error(PostWriteFailure::Identity))?;
        if identity.device_and_inode != Some(written_inode)
            || identity.device_and_inode == base.device_and_inode
            || identity.nlink != Some(1)
            || identity.is_symlink != Some(false)
        {
            return Err(post_error(PostWriteFailure::Identity));
        }
        if identity.mode != base.mode {
            return Err(post_error(PostWriteFailure::Mode));
        }
        if (identity.uid, identity.gid) != (base.uid, base.gid) {
            return Err(post_error(PostWriteFailure::Owner));
        }
        if identity.has_access_acl != base.has_access_acl {
            return Err(post_error(PostWriteFailure::Acl));
        }
        if !prepared.is_portal && identity.selinux_label != base.selinux_label {
            return Err(post_error(PostWriteFailure::SelinuxLabel));
        }
        Ok(version)
    }
}

fn missing_is_conflict(error: StorageError) -> StorageError {
    match error {
        StorageError::NotFound => StorageError::Conflict,
        error => error,
    }
}

fn preflight(
    resource: &ResourceRef,
    capabilities: &mut StorageCapabilities,
) -> Result<PreparedSave, StorageError> {
    let (file, path) = resolve(resource)?;
    let info = query(&file, FILE_ATTRIBUTES).map_err(missing_is_conflict)?;
    check_regular(&info)?;
    if !info.boolean("access::can-write")
        || permission_mode(&info).is_some_and(|mode| mode & WRITE_BITS == 0)
    {
        return Err(StorageError::ReadOnly);
    }
    capabilities.is_writable = true;
    if info.etag().is_none_or(|etag| etag.is_empty()) {
        return Err(StorageError::ConditionalReplaceUnsupported);
    }
    capabilities.supports_conditional_replace = true;
    let parent = path
        .parent()
        .ok_or(StorageError::AtomicReplaceUnsupported)?;
    if uint32(&info, "unix::nlink") != Some(1) {
        return Err(StorageError::AtomicReplaceUnsupported);
    }
    let parent_access = query(
        &gio::File::for_path(parent),
        "access::can-write,access::can-execute,unix::mode",
    )
    .map_err(|_| StorageError::AtomicReplaceUnsupported)?;
    if !parent_access.boolean("access::can-write")
        || !parent_access.boolean("access::can-execute")
        || permission_mode(&parent_access).is_none_or(|mode| mode & WRITE_BITS == 0)
    {
        return Err(StorageError::AtomicReplaceUnsupported);
    }

    let (uid, egid) = effective_ids();
    if uint32(&info, "unix::uid") != Some(uid)
        || !uint32(&info, "unix::gid")
            .is_some_and(|gid| gid != 65534 && (gid == egid || supplementary_group(gid)))
    {
        return Err(StorageError::PermissionsNotPreserved);
    }

    let validated_parent =
        safe_ancestors(parent, uid).ok_or(StorageError::PermissionsNotPreserved)?;
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent)
        .map_err(|_| StorageError::PermissionsNotPreserved)?;
    let directory_metadata = directory.metadata().map_err(map_io)?;
    if inode(&validated_parent) != inode(&directory_metadata)
        || !safe_directory(&directory_metadata, uid)
    {
        return Err(StorageError::PermissionsNotPreserved);
    }

    let (snapshot, checked_base) =
        read_snapshot(&file, &path, &info).map_err(missing_is_conflict)?;
    let version = snapshot.version;
    let identity = version
        .identity
        .as_ref()
        .ok_or(StorageError::PermissionsNotPreserved)?;
    let name = path
        .file_name()
        .filter(|name| !name.as_bytes().contains(&b'/'))
        .and_then(|name| CString::new(name.as_bytes()).ok())
        .ok_or(StorageError::Io {
            kind: io::ErrorKind::InvalidInput,
        })?;
    let pinned = open_relative(
        &directory,
        &name,
        libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        0,
    )
    .map_err(|error| missing_is_conflict(map_io(error)))?;
    let pinned_metadata = pinned.metadata().map_err(map_io)?;
    verify_descriptor(&pinned_metadata, identity, version.size)?;
    if !same_revision(&checked_base, &pinned_metadata) {
        return Err(StorageError::Conflict);
    }

    let portal_path =
        portal_relative_path(&path).filter(|_| filesystem_type(&pinned) == Some(FUSE_SUPER_MAGIC));
    let is_single_export = portal_path.is_some_and(|relative| relative.components().count() == 2);
    let is_portal = portal_path.is_some();
    let default_acl = acl_presence(&directory, c"system.posix_acl_default");
    let parent_is_acceptable = match &identity.parent {
        Some(parent) => {
            parent.uid == Some(directory_metadata.uid())
                && parent.mode == Some(directory_metadata.mode() & PERMISSIONS_MASK)
                && parent.is_sticky.is_some()
                && parent.has_default_acl == Some(false)
                && default_acl == Some(false)
        }
        None => is_single_export && default_acl != Some(true),
    };
    if identity.is_symlink != Some(false)
        || identity.mode.is_none()
        || identity.has_access_acl != Some(false)
        || version.mtime.is_none()
        || !parent_is_acceptable
        || (!is_portal && identity.selinux_label.is_none())
    {
        return Err(StorageError::PermissionsNotPreserved);
    }
    capabilities.supports_atomic_replace = true;
    capabilities.can_preserve_permissions = true;
    Ok(PreparedSave {
        directory,
        name,
        pinned,
        checked_base,
        version,
        is_portal,
        is_single_export,
    })
}

// lstat each ancestor, returning the exact parent identity that passed the check.
fn safe_ancestors(parent: &Path, uid: u32) -> Option<Metadata> {
    for path in parent
        .ancestors()
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if !safe_directory(&path.symlink_metadata().ok()?, uid) {
            return None;
        }
    }
    let info = parent.symlink_metadata().ok()?;
    safe_directory(&info, uid).then_some(info)
}

fn safe_directory(info: &Metadata, uid: u32) -> bool {
    info.is_dir()
        && (info.uid() == uid || info.uid() == 0)
        && (info.mode() & 0o022 == 0 || info.mode() & STICKY_BIT != 0)
}

fn inode(info: &Metadata) -> (u64, u64) {
    (info.dev(), info.ino())
}

// ctime catches content and metadata changes, including writes with restored
// mtime. The snapshot already hashed the bytes at this revision.
fn same_revision(before: &Metadata, after: &Metadata) -> bool {
    (
        before.ctime(),
        before.ctime_nsec(),
        before.mtime(),
        before.mtime_nsec(),
        before.len(),
    ) == (
        after.ctime(),
        after.ctime_nsec(),
        after.mtime(),
        after.mtime_nsec(),
        after.len(),
    )
}

#[allow(unsafe_code, reason = "read effective Linux process credentials")]
fn effective_ids() -> (u32, u32) {
    // SAFETY: These calls take no pointers and have no preconditions.
    unsafe { (libc::geteuid(), libc::getegid()) }
}

#[allow(unsafe_code, reason = "read supplementary Linux process groups")]
fn supplementary_group(gid: u32) -> bool {
    // SAFETY: A zero count requests the size without dereferencing the pointer.
    let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
    if count <= 0 {
        return false;
    }
    let mut groups = vec![0; count as usize];
    // SAFETY: The buffer can hold count gid_t entries and stays live for the call.
    let written = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
    usize::try_from(written)
        .ok()
        .and_then(|written| groups.get(..written))
        .is_some_and(|groups| groups.contains(&gid))
}

fn read_snapshot(
    file: &gio::File,
    path: &Path,
    info: &FileInfo,
) -> Result<(ReadSnapshot, Metadata), StorageError> {
    let size = checked_size(info)?;
    let mut identity = file_identity(info, path, None);
    if info.has_attribute("access::can-read") && !info.boolean("access::can-read") {
        return Err(StorageError::PermissionDenied);
    }

    // O_PATH pins the leaf without opening a raced-in special file for I/O.
    // O_NOFOLLOW gives us the symlink itself if the leaf changed to one.
    let pinned = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
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
    identity = file_identity(info, path, Some(&stream));

    let mut bytes = Vec::with_capacity(size as usize + 1);
    (&mut stream)
        .take(MAX_FILE_SIZE + 1)
        .read_to_end(&mut bytes)
        .map_err(map_io)?;
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(StorageError::TooLarge);
    }

    let current = query(file, FILE_ATTRIBUTES).map_err(missing_is_conflict)?;
    // Compare GIO path identity with GIO path identity, and sample xattrs
    // from the same verified read descriptor on both sides of the read.
    let current_identity = file_identity(&current, path, Some(&stream));
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
    Ok((ReadSnapshot::new(bytes, version), after))
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

fn check_regular(info: &FileInfo) -> Result<(), StorageError> {
    if info.is_symlink() || info.file_type() == FileType::SymbolicLink {
        return Err(StorageError::Symlink);
    }
    if info.file_type() != FileType::Regular {
        return Err(StorageError::NotRegularFile);
    }
    Ok(())
}

fn checked_size(info: &FileInfo) -> Result<u64, StorageError> {
    check_regular(info)?;
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

fn device_and_inode(info: &FileInfo) -> Option<(u64, u64)> {
    uint32(info, "unix::device").map(u64::from).zip(
        info.has_attribute("unix::inode")
            .then(|| info.attribute_uint64("unix::inode")),
    )
}

fn file_identity(info: &FileInfo, path: &Path, stream: Option<&File>) -> FileIdentity {
    let portal_path = portal_relative_path(path)
        .filter(|_| stream.is_some_and(|fd| filesystem_type(fd) == Some(FUSE_SUPER_MAGIC)));
    FileIdentity {
        device_and_inode: device_and_inode(info),
        uid: uint32(info, "unix::uid"),
        gid: uint32(info, "unix::gid"),
        mode: permission_mode(info),
        nlink: uint32(info, "unix::nlink").map(u64::from),
        is_symlink: info
            .has_attribute("standard::is-symlink")
            .then(|| info.is_symlink()),
        parent: if portal_path
            .as_ref()
            .is_some_and(|path| path.components().count() == 2)
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
        Err(libc::ENOTSUP)
            if filesystem_type(file).is_some_and(|kind| kind != FUSE_SUPER_MAGIC) =>
        {
            Some(None)
        }
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
fn filesystem_type(file: &File) -> Option<u32> {
    let mut stats = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: The fd is live and stats points to writable statfs storage.
    // fstatfs initializes the structure on success.
    unsafe {
        (libc::fstatfs(file.as_raw_fd(), stats.as_mut_ptr()) == 0)
            .then(|| stats.assume_init().f_type as u32)
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

struct PreparedSave {
    directory: File,
    name: CString,
    pinned: File,
    checked_base: Metadata,
    version: DiskVersion,
    is_portal: bool,
    is_single_export: bool,
}

impl PreparedSave {
    fn replace(&self, base: &FileIdentity, content: &[u8]) -> Result<(u64, u64), StorageError> {
        let mode = base.mode.ok_or(StorageError::PermissionsNotPreserved)?;
        let gid = base.gid.ok_or(StorageError::PermissionsNotPreserved)?;
        let mut pending = PendingFile::create(&self.directory).map_err(write_error)?;
        let temporary = pending.file.metadata().map_err(write_error)?;
        if temporary.gid() != gid {
            set_group(&pending.file, gid).map_err(|_| StorageError::PermissionsNotPreserved)?;
        }
        let acl = acl_presence(&pending.file, c"system.posix_acl_access");
        if acl != Some(false)
            || (!self.is_portal && selinux_label(&pending.file) != base.selinux_label)
        {
            return Err(StorageError::PermissionsNotPreserved);
        }

        // Keep staging private throughout the write and apply the final mode
        // afterward because writing can clear set-id bits on Linux.
        pending.file.write_all(content).map_err(write_error)?;
        set_mode(&pending.file, mode).map_err(|_| StorageError::PermissionsNotPreserved)?;
        let written = pending.file.metadata().map_err(write_error)?;
        if written.mode() & PERMISSIONS_MASK != mode
            || Some(written.uid()) != base.uid
            || written.gid() != gid
        {
            return Err(StorageError::PermissionsNotPreserved);
        }
        pending.file.sync_all().map_err(write_error)?;

        let directory_metadata = self.directory.metadata().map_err(write_error)?;
        let default_acl = acl_presence(&self.directory, c"system.posix_acl_default");
        if !safe_directory(&directory_metadata, self.checked_base.uid())
            || (default_acl != Some(false) && !(self.is_single_export && default_acl.is_none()))
        {
            return Err(StorageError::PermissionsNotPreserved);
        }
        self.check_base()?;
        if let Err(error) = pending.publish(&self.name) {
            // FUSE and network filesystems can lose the reply of a rename that
            // was applied; reporting "not saved" would break the contract.
            if relative_inode(&self.directory, &self.name).ok() == Some(inode(&written)) {
                return Err(StorageError::PostWriteVerificationFailed {
                    reason: PostWriteFailure::Identity,
                });
            }
            return Err(if error.kind() == io::ErrorKind::NotFound {
                StorageError::Conflict
            } else {
                write_error(error)
            });
        }
        // Rename already made the bytes visible. Identity is the closest
        // existing reason for failure to establish a durable directory entry.
        self.directory
            .sync_all()
            .map_err(|_| StorageError::PostWriteVerificationFailed {
                reason: PostWriteFailure::Identity,
            })?;
        Ok(inode(&written))
    }

    fn check_base(&self) -> Result<(), StorageError> {
        let current = self.pinned.metadata().map_err(write_error)?;
        if current.nlink() != 1 || !same_revision(&self.checked_base, &current) {
            return Err(StorageError::Conflict);
        }
        let named_inode = relative_inode(&self.directory, &self.name)
            .map_err(|error| missing_is_conflict(map_io(error)))?;
        if named_inode != inode(&current) {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }
}

struct PendingFile<'a> {
    directory: &'a File,
    name: CString,
    file: File,
    committed: bool,
}

impl<'a> PendingFile<'a> {
    fn create(directory: &'a File) -> io::Result<Self> {
        let mut attempts = 0;
        loop {
            let name = CString::new(format!(".envy-{:016x}.tmp", random_suffix()?))
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
            let flags =
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
            match open_relative(directory, &name, flags, 0o600) {
                Ok(file) => {
                    return Ok(Self {
                        directory,
                        name,
                        file,
                        committed: false,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists && attempts < 3 => {
                    attempts += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }

    #[allow(
        unsafe_code,
        reason = "publish the staged inode relative to the pinned directory"
    )]
    fn publish(&mut self, destination: &CStr) -> io::Result<()> {
        let fd = self.directory.as_raw_fd();
        // SAFETY: The directory stays open and both names are NUL-terminated.
        syscall_result(unsafe {
            libc::renameat(fd, self.name.as_ptr(), fd, destination.as_ptr())
        })?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for PendingFile<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        // A transient directory permission change can block unlink just as it
        // can block rename. Retry briefly while the descriptor stays pinned.
        for attempt in 0..=20 {
            match unlink_relative(self.directory, &self.name) {
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied && attempt < 20 => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                _ => break,
            }
        }
    }
}

fn write_error(error: io::Error) -> StorageError {
    StorageError::Io { kind: error.kind() }
}

fn syscall_result(value: libc::c_int) -> io::Result<()> {
    if value == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[allow(
    unsafe_code,
    reason = "open a basename relative to the pinned directory"
)]
fn open_relative(
    directory: &File,
    name: &CStr,
    flags: libc::c_int,
    mode: libc::mode_t,
) -> io::Result<File> {
    // SAFETY: The directory stays open, the name is NUL-terminated, and
    // the variadic mode argument is supplied for O_CREAT.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags, mode) };
    syscall_result(fd)?;
    // SAFETY: The successful open returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[allow(
    unsafe_code,
    reason = "inspect the destination entry without following symlinks"
)]
fn relative_inode(directory: &File, name: &CStr) -> io::Result<(u64, u64)> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: The directory stays open, the name is NUL-terminated, and stat
    // points to writable storage for the result.
    syscall_result(unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    })?;
    // SAFETY: Successful fstatat initialized stat.
    let stat = unsafe { stat.assume_init() };
    Ok((stat.st_dev, stat.st_ino))
}

#[allow(unsafe_code, reason = "remove the unpublished staging entry")]
fn unlink_relative(directory: &File, name: &CStr) -> io::Result<()> {
    // SAFETY: The directory stays open and the name is NUL-terminated.
    syscall_result(unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) })
}

#[allow(unsafe_code, reason = "apply the original group to the staging inode")]
fn set_group(file: &File, gid: u32) -> io::Result<()> {
    // SAFETY: File keeps the descriptor live. The all-ones uid leaves uid unchanged.
    syscall_result(unsafe { libc::fchown(file.as_raw_fd(), libc::uid_t::MAX, gid) })
}

#[allow(
    unsafe_code,
    reason = "apply the original permission bits to the staging inode"
)]
fn set_mode(file: &File, mode: u32) -> io::Result<()> {
    // SAFETY: File keeps the descriptor live and mode contains the saved permission bits.
    syscall_result(unsafe { libc::fchmod(file.as_raw_fd(), mode) })
}

#[allow(
    unsafe_code,
    reason = "obtain an unpredictable temporary name from Linux"
)]
fn random_suffix() -> io::Result<u64> {
    let mut bytes = [0; 8];
    let mut remaining = bytes.as_mut_slice();
    while !remaining.is_empty() {
        // SAFETY: The slice is writable for its full length and lives across the call.
        let count = unsafe { libc::getrandom(remaining.as_mut_ptr().cast(), remaining.len(), 0) };
        if count == -1 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if count == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        remaining = remaining
            .get_mut(count as usize..)
            .ok_or(io::ErrorKind::InvalidData)?;
    }
    Ok(u64::from_ne_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deletion_after_preflight_is_a_conflict_without_recreation() {
        let directory =
            std::env::temp_dir().join(format!("envy-delete-{:016x}", random_suffix().unwrap()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("document");
        std::fs::write(&path, b"original").unwrap();
        let resource = ResourceRef::from_uri(gio::File::for_path(&path).uri().as_str());
        let prepared = preflight(&resource, &mut StorageCapabilities::default());
        if let Ok(prepared) = &prepared {
            let staging = PendingFile::create(&prepared.directory).unwrap();
            assert_eq!(
                staging.file.metadata().unwrap().mode() & (PERMISSIONS_MASK & !0o600),
                0
            );
        }
        std::fs::remove_file(&path).unwrap();
        let result = prepared.map(|prepared| {
            prepared.replace(prepared.version.identity.as_ref().unwrap(), b"replacement")
        });
        let listing = std::fs::read_dir(&directory).unwrap().count();
        std::fs::remove_dir_all(directory).unwrap();
        assert_eq!(result.unwrap(), Err(StorageError::Conflict));
        assert_eq!(listing, 0);
    }
}
