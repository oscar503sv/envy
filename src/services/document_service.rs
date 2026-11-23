//! Per-resource editing and synchronous save orchestration.

use std::{fmt, sync::Arc};

use crate::domain::{
    document::EnvDocument,
    edit::EditCommand,
    history::{EditHistory, EditRequest},
    parser::parse,
    redact::Redacted,
};

use super::storage::{
    DiskVersion, DocumentStorage, PostWriteFailure, ReadSnapshot, ResourceRef, StorageCapabilities,
    StorageError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("structured editing is unavailable")]
    Protected,
    #[error("document edit was rejected")]
    EditRejected,
    #[error("document validation failed")]
    InvalidDocument,
    #[error("a save is in progress")]
    Saving,
    #[error("the disk version must be reviewed before overwriting")]
    Conflict,
    #[error("the disk version could not be recovered after writing")]
    BaselineUnavailable,
    #[error("the post-write warning must be acknowledged before saving")]
    PostWriteWarningUnacknowledged,
    #[error("save as is required: {reason}")]
    SaveAsRequired { reason: StorageError },
    #[error("save-as destination refers to the current document")]
    InvalidSaveAs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAsTarget {
    New,
    /// The file chooser confirmed replacing this destination.
    ReplaceExisting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalChange {
    Unchanged,
    MetadataOnly,
    ReloadAvailable,
    Conflict,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome {
    Unchanged,
    Reloaded,
    Saved,
    WrittenWithWarning {
        reason: PostWriteFailure,
        baseline_recovered: bool,
        recovery_error: Option<StorageError>,
    },
}

/// Owns its storage handle so it can move to a worker. Saves take exclusive
/// access for the entire operation, so edits cannot interleave with a save.
pub struct DocumentSession {
    storage: Arc<dyn DocumentStorage>,
    resource: ResourceRef,
    content: SessionContent,
    baseline: DiskVersion,
    capabilities: StorageCapabilities,
    external: Option<ReadSnapshot>,
    is_conflicted: bool,
    is_missing: bool,
    is_saving: bool,
    needs_baseline_recovery: bool,
    permission_warning: Option<PostWriteFailure>,
    last_observation_error: Option<StorageError>,
}

struct SaveGuard<'a>(&'a mut bool);

impl Drop for SaveGuard<'_> {
    fn drop(&mut self) {
        *self.0 = false;
    }
}

enum SessionContent {
    Editable(EditHistory),
    Protected(Redacted<Vec<u8>>),
}

impl SessionContent {
    fn history(&self) -> Option<&EditHistory> {
        match self {
            Self::Editable(history) => Some(history),
            Self::Protected(_) => None,
        }
    }
}

impl fmt::Debug for DocumentSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocumentSession").finish_non_exhaustive()
    }
}

impl DocumentSession {
    pub fn open(
        storage: Arc<dyn DocumentStorage>,
        resource: ResourceRef,
    ) -> Result<Self, SessionError> {
        let snapshot = storage.read(&resource)?;
        storage.remove_stale_staging(&resource);
        let capabilities = storage.capabilities(&resource);
        let content = Self::load_content(snapshot.bytes)?;
        Ok(Self {
            storage,
            resource,
            content,
            baseline: snapshot.version,
            capabilities,
            external: None,
            is_conflicted: false,
            is_missing: false,
            is_saving: false,
            needs_baseline_recovery: false,
            permission_warning: None,
            last_observation_error: None,
        })
    }

    fn load_content(bytes: Redacted<Vec<u8>>) -> Result<SessionContent, SessionError> {
        // Validate UTF-8 before moving the bytes into the parser so protected
        // content can be retained without cloning every editable document.
        if std::str::from_utf8(bytes.expose()).is_err() {
            return Ok(SessionContent::Protected(bytes));
        }
        let document = parse(bytes.into_inner()).map_err(|_| SessionError::InvalidDocument)?;
        let history = EditHistory::new(document).map_err(|_| SessionError::InvalidDocument)?;
        Ok(SessionContent::Editable(history))
    }

    pub fn resource(&self) -> &ResourceRef {
        &self.resource
    }

    pub fn baseline(&self) -> &DiskVersion {
        &self.baseline
    }

    pub fn document(&self) -> Option<&EnvDocument> {
        self.content.history().map(EditHistory::document)
    }

    pub fn protected_bytes(&self) -> Option<&Redacted<Vec<u8>>> {
        match &self.content {
            SessionContent::Editable(_) => None,
            SessionContent::Protected(bytes) => Some(bytes),
        }
    }

    pub fn capabilities(&self) -> StorageCapabilities {
        self.capabilities
    }

    pub fn external_snapshot(&self) -> Option<&ReadSnapshot> {
        self.external.as_ref()
    }

    pub fn is_dirty(&self) -> bool {
        self.content.history().is_some_and(EditHistory::is_dirty)
    }

    pub fn is_conflicted(&self) -> bool {
        self.is_conflicted
            || (self.is_dirty()
                && self
                    .external
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.version.hash != self.baseline.hash))
    }

    pub fn permission_warning(&self) -> Option<PostWriteFailure> {
        self.permission_warning
    }

    /// Record that the user saw the warning. Reloading the file leaves the
    /// warning pending because the permissions or identity may still differ.
    pub fn acknowledge_post_write_warning(&mut self) {
        self.permission_warning = None;
    }

    pub fn last_observation_error(&self) -> Option<StorageError> {
        self.last_observation_error
    }

    /// A missing file also reports as read-only. UI should prioritize missing.
    pub fn is_read_only(&self) -> bool {
        self.is_missing || !self.capabilities.is_writable
    }

    pub fn is_protected(&self) -> bool {
        matches!(self.content, SessionContent::Protected(_))
    }

    pub fn is_missing(&self) -> bool {
        self.is_missing
    }

    pub fn is_saving(&self) -> bool {
        self.is_saving
    }

    pub fn can_undo(&self) -> bool {
        self.content.history().is_some_and(EditHistory::can_undo)
    }

    pub fn can_redo(&self) -> bool {
        self.content.history().is_some_and(EditHistory::can_redo)
    }

    pub fn apply(&mut self, request: EditRequest<'_>) -> Result<Option<EditCommand>, SessionError> {
        self.editable_history()?
            .apply(request)
            .map_err(|_| SessionError::EditRejected)
    }

    pub fn apply_batch<'a>(
        &mut self,
        requests: impl IntoIterator<Item = EditRequest<'a>>,
    ) -> Result<Option<EditCommand>, SessionError> {
        self.editable_history()?
            .apply_batch(requests)
            .map_err(|_| SessionError::EditRejected)
    }

    pub fn undo(&mut self) -> Result<bool, SessionError> {
        self.editable_history()?
            .undo()
            .map_err(|_| SessionError::EditRejected)
    }

    pub fn redo(&mut self) -> Result<bool, SessionError> {
        self.editable_history()?
            .redo()
            .map_err(|_| SessionError::EditRejected)
    }

    fn editable_history(&mut self) -> Result<&mut EditHistory, SessionError> {
        self.ensure_idle()?;
        match &mut self.content {
            SessionContent::Editable(history) => Ok(history),
            SessionContent::Protected(_) => Err(SessionError::Protected),
        }
    }

    fn ensure_idle(&self) -> Result<(), SessionError> {
        if self.is_saving {
            return Err(SessionError::Saving);
        }
        Ok(())
    }

    /// Read the current resource and update the session's external state.
    /// T031 polling and monitor handlers should use this entry point.
    pub fn refresh_external(&mut self) -> Result<ExternalChange, SessionError> {
        self.ensure_idle()?;
        let result = self.storage.read(&self.resource);
        Ok(self.record_observation(result)?)
    }

    /// Record a fresh result for this resource. Callers must supply observations
    /// in read order. Prefer refresh_external for polling and monitor events.
    pub fn observe_external(
        &mut self,
        result: Result<ReadSnapshot, StorageError>,
    ) -> Result<ExternalChange, SessionError> {
        self.ensure_idle()?;
        Ok(self.record_observation(result)?)
    }

    fn record_observation(
        &mut self,
        result: Result<ReadSnapshot, StorageError>,
    ) -> Result<ExternalChange, StorageError> {
        self.capabilities = self.storage.capabilities(&self.resource);
        self.last_observation_error = result.as_ref().err().copied();
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(StorageError::NotFound) => {
                self.is_missing = true;
                self.external = None;
                return Ok(ExternalChange::Missing);
            }
            Err(error) => {
                self.external = None;
                return Err(self.record_storage_error(error));
            }
        };
        self.is_missing = false;
        if snapshot.version == self.baseline && !self.needs_baseline_recovery {
            self.external = None;
            self.is_conflicted = false;
            return Ok(ExternalChange::Unchanged);
        }
        let same_content = snapshot.version.hash == self.baseline.hash;
        let metadata_only = same_content && !self.needs_baseline_recovery;
        self.is_conflicted = self.is_conflicted
            || self.needs_baseline_recovery
            || (self.is_dirty() && !metadata_only);
        self.external = Some(snapshot);
        Ok(if self.is_conflicted {
            ExternalChange::Conflict
        } else if metadata_only {
            ExternalChange::MetadataOnly
        } else {
            ExternalChange::ReloadAvailable
        })
    }

    /// Perform the clean reload indicated by an observation. Recheck dirty
    /// state so an edit made after observation takes precedence. A reload
    /// replaces edit history, including undo and redo for already-saved edits.
    pub fn reload_if_clean(&mut self) -> Result<bool, SessionError> {
        self.ensure_idle()?;
        if self.is_dirty()
            || self.is_missing
            || self.is_conflicted()
            || self.needs_baseline_recovery
            || self.permission_warning.is_some()
        {
            return Ok(false);
        }
        let Some(snapshot) = self.external.as_ref() else {
            return Ok(false);
        };
        if snapshot.version.hash == self.baseline.hash {
            return Ok(false);
        }
        // Re-read so a delayed notification never installs a stale snapshot.
        let snapshot = self
            .storage
            .read(&self.resource)
            .map_err(|e| self.record_storage_error(e))?;
        if snapshot.version.hash == self.baseline.hash {
            self.record_observation(Ok(snapshot))?;
            return Ok(false);
        }
        self.install_reload(snapshot)?;
        Ok(true)
    }

    /// Caller confirms discard before invoking this operation.
    pub fn reload_and_discard(&mut self) -> Result<(), SessionError> {
        self.ensure_idle()?;
        let snapshot = self
            .storage
            .read(&self.resource)
            .map_err(|e| self.record_storage_error(e))?;
        self.install_reload(snapshot)
    }

    fn install_reload(&mut self, snapshot: ReadSnapshot) -> Result<(), SessionError> {
        self.content = Self::load_content(snapshot.bytes)?;
        self.accept_version(snapshot.version);
        Ok(())
    }

    fn accept_version(&mut self, version: DiskVersion) {
        self.baseline = version;
        self.capabilities = self.storage.capabilities(&self.resource);
        self.external = None;
        self.is_conflicted = false;
        self.is_missing = false;
        self.needs_baseline_recovery = false;
        self.last_observation_error = None;
    }

    fn ensure_save_allowed(&mut self) -> Result<(), SessionError> {
        self.ensure_idle()?;
        if self.permission_warning.is_some() {
            return Err(SessionError::PostWriteWarningUnacknowledged);
        }
        if self.is_missing {
            return Err(StorageError::NotFound.into());
        }
        self.capabilities = self.storage.capabilities(&self.resource);
        if self.is_read_only() {
            return Err(StorageError::ReadOnly.into());
        }
        Ok(())
    }

    /// Saving a clean session may reload it and replace its edit history,
    /// including undo and redo for already-saved edits.
    pub fn save(&mut self) -> Result<SaveOutcome, SessionError> {
        self.ensure_save_allowed()?;
        if self.needs_baseline_recovery {
            return Err(SessionError::BaselineUnavailable);
        }
        if !self.is_dirty() {
            return match self.refresh_external()? {
                ExternalChange::Missing => Err(StorageError::NotFound.into()),
                _ if self.reload_if_clean()? => Ok(SaveOutcome::Reloaded),
                _ => Ok(SaveOutcome::Unchanged),
            };
        }
        if self.is_conflicted() {
            return Err(SessionError::Conflict);
        }
        self.persist(&self.baseline.clone())
    }

    /// Caller confirms overwrite of the reviewed version. Revalidate that
    /// version before writing and pass it to the port's conditional replace.
    pub fn overwrite_confirmed(
        &mut self,
        reviewed: &DiskVersion,
    ) -> Result<SaveOutcome, SessionError> {
        self.ensure_save_allowed()?;
        if !self.is_conflicted()
            || self
                .external
                .as_ref()
                .is_none_or(|s| &s.version != reviewed)
        {
            return Err(SessionError::Conflict);
        }
        let current = self
            .storage
            .read(&self.resource)
            .map_err(|e| self.record_storage_error(e))?;
        if &current.version != reviewed {
            self.record_observation(Ok(current))?;
            return Err(SessionError::Conflict);
        }
        self.persist(&current.version)
    }

    fn persist(&mut self, expected: &DiskVersion) -> Result<SaveOutcome, SessionError> {
        let capabilities = self.capabilities;
        let reason = if !capabilities.supports_conditional_replace {
            Some(StorageError::ConditionalReplaceUnsupported)
        } else if !capabilities.supports_atomic_replace {
            Some(StorageError::AtomicReplaceUnsupported)
        } else if !capabilities.can_preserve_permissions {
            Some(StorageError::PermissionsNotPreserved)
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(SessionError::SaveAsRequired { reason });
        }
        let document = self.document().ok_or(SessionError::Protected)?;
        document
            .check_invariants()
            .map_err(|_| SessionError::InvalidDocument)?;
        let bytes = document.serialize();
        let result = {
            self.is_saving = true;
            let _saving = SaveGuard(&mut self.is_saving);
            // The port owns fresh preflight and write error precedence.
            self.storage
                .replace_if_unchanged(&self.resource, expected, &bytes)
        };
        match result {
            Ok(version) => {
                self.accept_version(version);
                if let SessionContent::Editable(history) = &mut self.content {
                    history.mark_saved();
                }
                Ok(SaveOutcome::Saved)
            }
            Err(StorageError::PostWriteVerificationFailed { reason }) => {
                Ok(self.recover_written_version(&bytes, reason))
            }
            Err(StorageError::Conflict) => {
                self.is_conflicted = self.refresh_external().is_ok() && self.external.is_some();
                Err(StorageError::Conflict.into())
            }
            Err(
                reason @ (StorageError::ConditionalReplaceUnsupported
                | StorageError::AtomicReplaceUnsupported
                | StorageError::PermissionsNotPreserved),
            ) => Err(SessionError::SaveAsRequired { reason }),
            Err(error) => {
                self.capabilities = self.storage.capabilities(&self.resource);
                Err(self.record_storage_error(error).into())
            }
        }
    }

    fn record_storage_error(&mut self, error: StorageError) -> StorageError {
        #[allow(
            unreachable_patterns,
            reason = "handle future storage errors without changing session state"
        )]
        match error {
            StorageError::NotFound => {
                self.is_missing = true;
                self.external = None;
            }
            StorageError::ReadOnly | StorageError::PermissionDenied => {
                self.capabilities.is_writable = false;
            }
            _ => {}
        }
        error
    }

    fn recover_written_version(&mut self, bytes: &[u8], reason: PostWriteFailure) -> SaveOutcome {
        if reason != PostWriteFailure::Content {
            self.permission_warning = Some(reason);
        }
        self.needs_baseline_recovery = true;
        self.is_conflicted = true;
        self.external = None;
        let (baseline_recovered, recovery_error) = match self.storage.read(&self.resource) {
            Ok(snapshot)
                if reason != PostWriteFailure::Identity && snapshot.bytes.expose() == bytes =>
            {
                self.accept_version(snapshot.version);
                (true, None)
            }
            result => match self.record_observation(result) {
                Ok(_) => (false, self.last_observation_error),
                Err(error) => (false, Some(error)),
            },
        };
        // Keep history and the dirty checkpoint until a verified save.
        SaveOutcome::WrittenWithWarning {
            reason,
            baseline_recovered,
            recovery_error,
        }
    }

    pub fn save_as(
        &mut self,
        destination: ResourceRef,
        target: SaveAsTarget,
    ) -> Result<SaveOutcome, SessionError> {
        self.ensure_idle()?;
        let document = self.document().ok_or(SessionError::Protected)?;
        document
            .check_invariants()
            .map_err(|_| SessionError::InvalidDocument)?;
        if destination == self.resource {
            return Err(SessionError::InvalidSaveAs);
        }
        let bytes = document.serialize();
        let result = {
            self.is_saving = true;
            let _saving = SaveGuard(&mut self.is_saving);
            match target {
                SaveAsTarget::New => self.storage.create_new(&destination, &bytes),
                SaveAsTarget::ReplaceExisting => {
                    let version = self.storage.read(&destination)?.version;
                    let inode = |version: &DiskVersion| {
                        version
                            .identity
                            .as_ref()
                            .and_then(|identity| identity.device_and_inode)
                    };
                    // Without a known identity the destination could alias the
                    // original and bypass its conflict detection.
                    let Some(destination_inode) = inode(&version) else {
                        return Err(SessionError::InvalidSaveAs);
                    };
                    if inode(&self.baseline) == Some(destination_inode)
                        || self
                            .storage
                            .read(&self.resource)
                            .ok()
                            .is_some_and(|snapshot| {
                                inode(&snapshot.version) == Some(destination_inode)
                            })
                    {
                        return Err(SessionError::InvalidSaveAs);
                    }
                    self.storage
                        .replace_if_unchanged(&destination, &version, &bytes)
                }
            }
        };
        match result {
            Ok(version) => {
                self.resource = destination;
                self.permission_warning = None;
                self.accept_version(version);
                self.editable_history()?.mark_saved();
                Ok(SaveOutcome::Saved)
            }
            Err(StorageError::PostWriteVerificationFailed { reason }) => {
                self.resource = destination;
                self.permission_warning = None;
                self.is_missing = false;
                self.last_observation_error = None;
                self.capabilities = self.storage.capabilities(&self.resource);
                Ok(self.recover_written_version(&bytes, reason))
            }
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests;
