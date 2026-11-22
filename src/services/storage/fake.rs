//! In-memory test support shared by unit and integration tests.
//! Production adapters have no dependency on this module.

use std::{
    collections::HashMap,
    io,
    sync::{Mutex, MutexGuard},
};

use super::{
    DiskVersion, DocumentStorage, FileIdentity, ParentDirectory, PostWriteFailure, ReadSnapshot,
    ResourceRef, StorageCapabilities, StorageError, content_hash,
};
use crate::domain::redact::Redacted;

const OVERFLOW_GID: u32 = 65_534;
const FILE_TYPE_MASK: u32 = 0o170000;
const REGULAR_FILE_TYPE: u32 = 0o100000;

/// Build an observed regular-file snapshot for tests.
pub fn snapshot_from_bytes(bytes: Vec<u8>) -> ReadSnapshot {
    let version = DiskVersion {
        hash: Redacted::new(content_hash(&bytes)),
        etag: None,
        size: bytes.len() as u64,
        mtime: None,
        identity: Some(FileIdentity {
            device_and_inode: Some((1, 1)),
            uid: Some(1000),
            gid: Some(1000),
            mode: Some(0o600),
            nlink: Some(1),
            is_symlink: Some(false),
            parent: Some(ParentDirectory {
                uid: Some(1000),
                mode: Some(0o700),
                is_sticky: Some(false),
                has_default_acl: Some(false),
            }),
            has_access_acl: Some(false),
            selinux_label: Some(None),
        }),
    };
    ReadSnapshot::new(bytes, version)
}

/// All settings are per resource and remain in effect until replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FakeSettings {
    pub capabilities: StorageCapabilities,
    pub read_error: Option<StorageError>,
    pub write_error: Option<StorageError>,
    pub post_write_failure: Option<PostWriteFailure>,
    /// Model the D-19 preflight exceptions for a single file via the portal.
    pub is_portal_file: bool,
}

impl Default for FakeSettings {
    fn default() -> Self {
        Self {
            capabilities: StorageCapabilities {
                supports_conditional_replace: true,
                supports_atomic_replace: true,
                can_preserve_permissions: true,
                is_writable: true,
            },
            read_error: None,
            write_error: None,
            post_write_failure: None,
            is_portal_file: false,
        }
    }
}

#[derive(Default)]
struct Entry {
    snapshot: Option<ReadSnapshot>,
    settings: FakeSettings,
}

/// Deterministic fake. A single lock covers comparison and replacement.
#[derive(Default)]
pub struct FakeStorage {
    entries: Mutex<HashMap<ResourceRef, Entry>>,
}

impl FakeStorage {
    /// Seed a file, simulate an external edit or metadata change, or delete it
    /// with None. Capabilities and injected failures are bypassed.
    pub fn set_snapshot(
        &self,
        resource: &ResourceRef,
        snapshot: Option<ReadSnapshot>,
    ) -> Result<(), StorageError> {
        self.entries()?
            .entry(resource.clone())
            .or_default()
            .snapshot = snapshot;
        Ok(())
    }

    pub fn configure(
        &self,
        resource: &ResourceRef,
        settings: FakeSettings,
    ) -> Result<(), StorageError> {
        self.entries()?
            .entry(resource.clone())
            .or_default()
            .settings = settings;
        Ok(())
    }

    fn entries(&self) -> Result<MutexGuard<'_, HashMap<ResourceRef, Entry>>, StorageError> {
        self.entries.lock().map_err(|_| StorageError::Io {
            kind: io::ErrorKind::Other,
        })
    }
}

fn check_regular(snapshot: &ReadSnapshot) -> Result<(), StorageError> {
    if let Some(identity) = &snapshot.version.identity {
        if identity.is_symlink == Some(true) {
            return Err(StorageError::Symlink);
        }
        if let Some(mode) = identity.mode {
            let file_type = mode & FILE_TYPE_MASK;
            if file_type != 0 && file_type != REGULAR_FILE_TYPE {
                return Err(StorageError::NotRegularFile);
            }
        }
    }
    Ok(())
}

fn check_identity(
    identity: Option<&FileIdentity>,
    is_portal_file: bool,
) -> Result<(u64, u64), StorageError> {
    let identity = identity.ok_or(StorageError::PermissionsNotPreserved)?;
    let parent_is_known = identity.parent.as_ref().map_or(is_portal_file, |parent| {
        parent.uid.is_some()
            && parent.mode.is_some()
            && parent.is_sticky.is_some()
            && parent.has_default_acl.is_some()
    });
    if identity.is_symlink.is_none()
        || identity.nlink.is_none()
        || identity.mode.is_none()
        || identity.uid.is_none()
        || identity.gid.is_none_or(|gid| gid == OVERFLOW_GID)
        || identity.has_access_acl.is_none()
        || !parent_is_known
        || (!is_portal_file && identity.selinux_label.is_none())
    {
        return Err(StorageError::PermissionsNotPreserved);
    }
    identity
        .device_and_inode
        .ok_or(StorageError::PermissionsNotPreserved)
}

// The dedicated post-write knob is the only injection that can report a
// verification failure after bytes have been installed.
fn pre_write_error(error: StorageError) -> StorageError {
    match error {
        StorageError::PostWriteVerificationFailed { .. } => StorageError::Io {
            kind: io::ErrorKind::InvalidInput,
        },
        error => error,
    }
}

impl DocumentStorage for FakeStorage {
    fn read(&self, resource: &ResourceRef) -> Result<ReadSnapshot, StorageError> {
        let entries = self.entries()?;
        let entry = entries.get(resource).ok_or(StorageError::NotFound)?;
        let snapshot = entry.snapshot.as_ref().ok_or(StorageError::NotFound)?;
        if let Some(error) = entry.settings.read_error {
            return Err(pre_write_error(error));
        }
        check_regular(snapshot)?;
        Ok(snapshot.clone())
    }

    fn capabilities(&self, resource: &ResourceRef) -> StorageCapabilities {
        self.entries()
            .ok()
            .and_then(|entries| {
                entries
                    .get(resource)
                    .filter(|entry| entry.snapshot.is_some())
                    .map(|entry| entry.settings.capabilities)
            })
            .unwrap_or_default()
    }

    fn replace_if_unchanged(
        &self,
        resource: &ResourceRef,
        expected: &DiskVersion,
        content: &[u8],
    ) -> Result<DiskVersion, StorageError> {
        let mut entries = self.entries()?;
        let entry = entries.get_mut(resource).ok_or(StorageError::Conflict)?;
        let current = entry.snapshot.as_ref().ok_or(StorageError::Conflict)?;
        check_regular(current)?;
        if let Some(error) = entry.settings.read_error {
            return Err(match pre_write_error(error) {
                StorageError::NotFound => StorageError::Conflict,
                error => error,
            });
        }

        let capabilities = entry.settings.capabilities;
        if !capabilities.is_writable {
            return Err(StorageError::ReadOnly);
        }
        if !capabilities.supports_conditional_replace {
            return Err(StorageError::ConditionalReplaceUnsupported);
        }
        if !capabilities.supports_atomic_replace
            || current
                .version
                .identity
                .as_ref()
                .and_then(|identity| identity.nlink)
                .is_some_and(|nlink| nlink > 1)
        {
            return Err(StorageError::AtomicReplaceUnsupported);
        }
        if !capabilities.can_preserve_permissions {
            return Err(StorageError::PermissionsNotPreserved);
        }
        let (device, inode) = check_identity(
            current.version.identity.as_ref(),
            entry.settings.is_portal_file,
        )?;
        if &current.version != expected {
            return Err(StorageError::Conflict);
        }
        if let Some(error) = entry.settings.write_error {
            return Err(pre_write_error(error));
        }

        let mut version = current.version.clone();
        version.hash = Redacted::new(content_hash(content));
        version.size = content.len() as u64;
        // Model rename with a new inode. Etag and mtime intentionally stay
        // unchanged, since a provider can return the same values after a save.
        if let Some(identity) = &mut version.identity {
            identity.device_and_inode = Some((device, inode.wrapping_add(1)));
        }
        entry.snapshot = Some(ReadSnapshot::new(content.to_vec(), version.clone()));
        if let Some(reason) = entry.settings.post_write_failure {
            return Err(StorageError::PostWriteVerificationFailed { reason });
        }
        Ok(version)
    }
}
