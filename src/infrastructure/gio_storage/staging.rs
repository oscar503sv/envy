use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::*;

const PRIVATE_MODE: u32 = 0o600;
const STALE_STAGING_AGE: Duration = Duration::from_secs(10 * 60);

const IDENTITY_ERROR: StorageError = StorageError::PostWriteVerificationFailed {
    reason: PostWriteFailure::Identity,
};

pub(super) fn create_new(
    resource: &ResourceRef,
    content: &[u8],
) -> Result<DiskVersion, StorageError> {
    let (_, path) = resolve(resource)?;
    match path.symlink_metadata() {
        Ok(info) if info.is_symlink() => return Err(StorageError::Symlink),
        Ok(_) => return Err(StorageError::AlreadyExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(map_io(error)),
    }
    if content.len() as u64 > MAX_FILE_SIZE {
        return Err(StorageError::TooLarge);
    }
    let uid = effective_ids().0;
    let parent = path
        .parent()
        .ok_or(StorageError::AtomicReplaceUnsupported)?;
    check_parent_access(parent)?;
    let directory = pin_parent(parent, uid)?;
    let name = basename(&path)?;
    if acl_presence(&directory, c"system.posix_acl_default") != Some(false) {
        return Err(StorageError::PermissionsNotPreserved);
    }
    let mut pending = PendingFile::create(&directory).map_err(write_error)?;
    if acl_presence(&pending.file, c"system.posix_acl_access") != Some(false) {
        return Err(StorageError::PermissionsNotPreserved);
    }
    set_mode(&pending.file, PRIVATE_MODE).map_err(|_| StorageError::PermissionsNotPreserved)?;
    pending.file.write_all(content).map_err(write_error)?;
    pending.file.sync_all().map_err(write_error)?;
    let written = pending.file.metadata().map_err(write_error)?;
    let current_parent = pin_parent(parent, uid)?;
    if inode(&current_parent.metadata().map_err(write_error)?)
        != inode(&directory.metadata().map_err(write_error)?)
        || acl_presence(&directory, c"system.posix_acl_default") != Some(false)
    {
        return Err(StorageError::PermissionsNotPreserved);
    }
    if let Err(error) = pending.publish_new(&name) {
        if relative_inode(&directory, &name).ok() == Some(inode(&written)) {
            return Err(IDENTITY_ERROR);
        }
        return Err(if error.kind() == io::ErrorKind::AlreadyExists {
            StorageError::AlreadyExists
        } else {
            write_error(error)
        });
    }
    directory.sync_all().map_err(|_| IDENTITY_ERROR)?;
    verify_created(resource, content, inode(&written), uid)
}

fn verify_created(
    resource: &ResourceRef,
    content: &[u8],
    written_inode: (u64, u64),
    uid: u32,
) -> Result<DiskVersion, StorageError> {
    let post_error = |reason| StorageError::PostWriteVerificationFailed { reason };
    let version = GioStorage
        .read(resource)
        .map_err(|error| match error {
            StorageError::TooLarge => post_error(PostWriteFailure::Content),
            _ => IDENTITY_ERROR,
        })?
        .version;
    let identity = version.identity.as_ref().ok_or(IDENTITY_ERROR)?;
    if identity.device_and_inode != Some(written_inode)
        || identity.nlink != Some(1)
        || identity.is_symlink != Some(false)
    {
        return Err(IDENTITY_ERROR);
    }
    if version.hash.expose() != &content_hash(content) {
        return Err(post_error(PostWriteFailure::Content));
    }
    if identity.mode != Some(PRIVATE_MODE) {
        return Err(post_error(PostWriteFailure::Mode));
    }
    if identity.uid != Some(uid) {
        return Err(post_error(PostWriteFailure::Owner));
    }
    if identity.has_access_acl != Some(false) {
        return Err(post_error(PostWriteFailure::Acl));
    }
    Ok(version)
}

impl PendingFile<'_> {
    #[allow(
        unsafe_code,
        reason = "publish without replacing any destination entry"
    )]
    fn publish_new(&mut self, destination: &CStr) -> io::Result<()> {
        let fd = self.directory.as_raw_fd();
        // SAFETY: The directory stays open and both names are NUL-terminated.
        let result = syscall_result(unsafe {
            libc::renameat2(
                fd,
                self.name.as_ptr(),
                fd,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        });
        if let Err(error) = result {
            if !matches!(error.raw_os_error(), Some(libc::EINVAL | libc::ENOSYS)) {
                return Err(error);
            }
            // SAFETY: The directory stays open and both names are NUL-terminated.
            syscall_result(unsafe {
                libc::linkat(fd, self.name.as_ptr(), fd, destination.as_ptr(), 0)
            })?;
            // Publication already happened. Leave the guard armed on unlink failure
            // so verification sees any remaining hard link and the guard can retry.
            self.committed = unlink_relative(self.directory, &self.name).is_ok();
        } else {
            self.committed = true;
        }
        Ok(())
    }
}

pub(super) fn remove_stale_staging(resource: &ResourceRef) {
    let Ok((_, path)) = resolve(resource) else {
        return;
    };
    let Some(parent) = path.parent() else { return };
    let uid = effective_ids().0;
    let Ok(directory) = pin_parent(parent, uid) else {
        return;
    };
    if portal_relative_path(&path).is_some_and(|path| path.components().count() == 2)
        && filesystem_type(&directory) == Some(FUSE_SUPER_MAGIC)
    {
        return;
    }
    let Some(cutoff) = SystemTime::now()
        .checked_sub(STALE_STAGING_AGE)
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
    else {
        return;
    };
    // A crash inside the linkat fallback leaves the staging name as a second
    // link to the published document; only that exact pairing may be unlinked.
    let document_inode = basename(&path)
        .ok()
        .and_then(|name| relative_stat(&directory, &name).ok())
        .map(|stat| (stat.st_dev, stat.st_ino));
    // Enumerate the pinned directory even if its original pathname changes.
    let Ok(entries) = std::fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd())) else {
        return;
    };
    for entry in entries.flatten() {
        let filename = entry.file_name();
        if path.file_name() == Some(filename.as_os_str()) || !is_staging_name(filename.as_bytes()) {
            continue;
        }
        let Ok(name) = CString::new(filename.as_bytes()) else {
            continue;
        };
        let Ok(stat) = relative_stat(&directory, &name) else {
            continue;
        };
        if stat.st_mode & libc::S_IFMT == libc::S_IFREG
            && stat.st_uid == uid
            && (stat.st_nlink == 1
                || (stat.st_nlink == 2 && document_inode == Some((stat.st_dev, stat.st_ino))))
            && (i128::from(stat.st_mtime), i128::from(stat.st_mtime_nsec))
                < (
                    i128::from(cutoff.as_secs()),
                    i128::from(cutoff.subsec_nanos()),
                )
        {
            let _ = unlink_relative(&directory, &name);
        }
    }
}

fn is_staging_name(name: &[u8]) -> bool {
    name.strip_prefix(b".envy-")
        .and_then(|name| name.strip_suffix(b".tmp"))
        .is_some_and(|suffix| {
            suffix.len() == 16
                && suffix
                    .iter()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
}
