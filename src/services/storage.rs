//! Backend-independent document storage and versioning.

use std::{
    fmt,
    hash::{Hash, Hasher},
    io,
    time::SystemTime,
};

use sha2::{Digest, Sha256};

use crate::domain::redact::Redacted;

#[cfg(any(test, feature = "test-support"))]
pub mod fake;

/// Synchronous storage port. Run real backend operations, including hashing,
/// off the GTK main thread. Implementations can be shared with worker threads.
pub trait DocumentStorage: Send + Sync {
    /// Read an existing regular file without following symlinks.
    fn read(&self, resource: &ResourceRef) -> Result<ReadSnapshot, StorageError>;

    /// Report advisory capabilities for UI and read-only mode. Safety checks
    /// belong to the replacement operation itself.
    fn capabilities(&self, resource: &ResourceRef) -> StorageCapabilities;

    /// Repeat lstat and safe-save preflight immediately before replacement.
    /// Check in this order: absence (Conflict), symlink or non-regular file,
    /// read-only, conditional replacement, atomic replacement (including
    /// multiple hard links), permission preservation, and baseline equality.
    /// Unknown preflight observations block the save. D-19 condition 7 allows
    /// the SELinux label to be unobservable for any file accessed through the
    /// Flatpak document portal, and the parent directory only for a single
    /// exported file.
    ///
    /// Compare the entire baseline (hash, etag, size, mtime, and identity).
    /// Any observed change, including chmod, touch, or rename, is a Conflict.
    /// Callers may classify a conflict with the same hash as metadata-only.
    ///
    /// Return the new version only after verification. Every error except
    /// PostWriteVerificationFailed guarantees that this call left the stored
    /// bytes untouched. After PostWriteVerificationFailed, the new bytes were
    /// published, but a concurrent writer may already have replaced them, so
    /// the caller must re-read the baseline.
    fn replace_if_unchanged(
        &self,
        resource: &ResourceRef,
        expected: &DiskVersion,
        content: &[u8],
    ) -> Result<DiskVersion, StorageError>;
}

/// Opaque resource identity. The caller supplies the resolved URI where
/// available. Debug hides it because even a basename may contain private data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRef(Redacted<String>);

impl ResourceRef {
    pub fn from_uri(uri: impl Into<String>) -> Self {
        Self(Redacted::new(uri.into()))
    }

    pub fn uri(&self) -> &str {
        self.0.expose()
    }
}

impl Hash for ResourceRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uri().hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadSnapshot {
    pub bytes: Redacted<Vec<u8>>,
    pub version: DiskVersion,
}

impl ReadSnapshot {
    pub fn new(bytes: Vec<u8>, version: DiskVersion) -> Self {
        Self {
            bytes: Redacted::new(bytes),
            version,
        }
    }
}

/// SHA-256 over the exact stored bytes, shared by storage adapters.
/// Compare only in-process. Never log, persist, or export the digest, since
/// it can serve as an offline brute-force oracle for low-entropy files.
pub fn content_hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Observed baseline for strict comparison. The hash is for in-process
/// comparison only. Never log, persist, or export it.
#[derive(Clone, PartialEq, Eq)]
pub struct DiskVersion {
    pub hash: Redacted<[u8; 32]>,
    pub etag: Option<Redacted<String>>,
    pub size: u64,
    pub mtime: Option<SystemTime>,
    pub identity: Option<FileIdentity>,
}

impl fmt::Debug for DiskVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiskVersion")
            .field("hash", &self.hash)
            .field("etag", &self.etag)
            .field("mtime", &self.mtime)
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

/// Metadata used by safe-save preflight and verification. None means unknown
/// and blocks a save when preflight requires the observation. D-19 condition 7
/// allows only the SELinux label and parent directory to be unobservable for a
/// single file accessed through the Flatpak portal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileIdentity {
    pub device_and_inode: Option<(u64, u64)>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub mode: Option<u32>,
    pub nlink: Option<u64>,
    pub is_symlink: Option<bool>,
    pub parent: Option<ParentDirectory>,
    pub has_access_acl: Option<bool>,
    /// Outer None means unknown. Some(None) means verified absent.
    /// A provider's synthetic label must be treated as unknown.
    pub selinux_label: Option<Option<Redacted<Vec<u8>>>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParentDirectory {
    pub uid: Option<u32>,
    pub mode: Option<u32>,
    pub is_sticky: Option<bool>,
    pub has_default_acl: Option<bool>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StorageCapabilities {
    /// The backend can enforce a version precondition. The precondition
    /// check and the atomic replacement may be separate operations.
    pub supports_conditional_replace: bool,
    pub supports_atomic_replace: bool,
    pub can_preserve_permissions: bool,
    pub is_writable: bool,
}

/// Value-free diagnostics. I/O failures retain only the error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StorageError {
    #[error("file not found")]
    NotFound,
    #[error("permission denied")]
    PermissionDenied,
    #[error("resource is not a regular file")]
    NotRegularFile,
    #[error("resource is a symbolic link")]
    Symlink,
    #[error("file exceeds the size limit")]
    TooLarge,
    #[error("resource is read-only")]
    ReadOnly,
    #[error("file changed since it was opened")]
    Conflict,
    #[error("conditional replacement is unavailable")]
    ConditionalReplaceUnsupported,
    #[error("atomic replacement is unavailable")]
    AtomicReplaceUnsupported,
    /// Preflight could not guarantee preservation. No bytes were written.
    #[error("file permissions could not be preserved")]
    PermissionsNotPreserved,
    #[error("replacement was written but verification failed: {reason:?}")]
    PostWriteVerificationFailed { reason: PostWriteFailure },
    #[error("storage I/O error: {kind}")]
    Io { kind: io::ErrorKind },
}

/// Value-free reason for a verification failure after replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PostWriteFailure {
    Mode,
    Owner,
    Acl,
    SelinuxLabel,
    Content,
    Identity,
}

#[cfg(test)]
mod tests;
