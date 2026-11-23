#![allow(
    clippy::unwrap_used,
    reason = "unexpected failures should fail the test"
)]

use super::*;
use crate::{
    domain::{document::NodeKind, edit::ConversionPolicy},
    services::storage::fake::{FakeSettings, FakeStorage, snapshot_from_bytes},
    test_support::SENTINEL,
};

const ORIGINAL: &[u8] = b"A=original\n";
const EXTERNAL: &[u8] = b"A=external\n";
const EDITED: &[u8] = b"A=original\nB=local\n";

fn open_with(bytes: &[u8]) -> (Arc<FakeStorage>, DocumentSession) {
    let storage = Arc::new(FakeStorage::default());
    let resource = ResourceRef::from_uri(SENTINEL);
    let mut snapshot = snapshot_from_bytes(bytes.to_vec());
    snapshot.version.etag = Some(Redacted::new(SENTINEL.to_owned()));
    storage.set_snapshot(&resource, Some(snapshot)).unwrap();
    let session = DocumentSession::open(storage.clone(), resource).unwrap();
    (storage, session)
}

fn open() -> (Arc<FakeStorage>, DocumentSession) {
    open_with(ORIGINAL)
}

fn edit(session: &mut DocumentSession) {
    session
        .apply(EditRequest::InsertAssignment {
            key: Redacted::new("B"),
            value: Redacted::new("local"),
            policy: ConversionPolicy::Reject,
        })
        .unwrap();
}

fn bytes(session: &DocumentSession) -> Vec<u8> {
    session.document().unwrap().serialize()
}

fn change(storage: &FakeStorage, session: &DocumentSession, content: &[u8]) -> ReadSnapshot {
    let snapshot = snapshot_from_bytes(content.to_vec());
    storage
        .set_snapshot(session.resource(), Some(snapshot.clone()))
        .unwrap();
    snapshot
}

#[test]
fn external_identical_keeps_baseline_and_local_edits() {
    for dirty in [false, true] {
        let (storage, mut session) = open();
        let baseline = storage.read(session.resource()).unwrap().version;
        assert!(session.baseline() == &baseline);
        assert!(baseline.etag.is_some() && baseline.identity.is_some());
        if dirty {
            edit(&mut session);
        }
        let before = bytes(&session);
        let mut snapshot = change(&storage, &session, ORIGINAL);
        snapshot.version.identity.as_mut().unwrap().device_and_inode = Some((1, 2));
        assert!(snapshot.version.identity != baseline.identity);
        assert_eq!(
            session.observe_external(Ok(snapshot)),
            Ok(ExternalChange::MetadataOnly)
        );
        assert!(bytes(&session) == before);
        assert!(session.baseline() == &baseline);
        assert!(!session.is_conflicted());
        assert!(!session.reload_if_clean().unwrap());
    }
}

#[test]
fn external_different_reloads_clean_or_conflicts_with_dirty() {
    for dirty in [false, true] {
        let (storage, mut session) = open();
        if dirty {
            edit(&mut session);
        }
        let baseline = session.baseline().clone();
        let before = bytes(&session);
        let snapshot = change(&storage, &session, EXTERNAL);
        let result = session.observe_external(Ok(snapshot.clone())).unwrap();
        assert_eq!(
            result,
            if dirty {
                ExternalChange::Conflict
            } else {
                ExternalChange::ReloadAvailable
            }
        );
        assert!(session.baseline() == &baseline);
        assert!(bytes(&session) == before);
        assert_eq!(session.is_conflicted(), dirty);
        assert_eq!(session.is_dirty(), dirty);
        assert_eq!(session.reload_if_clean().unwrap(), !dirty);
        if dirty {
            assert!(session.external_snapshot() == Some(&snapshot));
            assert!(bytes(&session) == EDITED);
        } else {
            assert!(bytes(&session) == EXTERNAL);
            assert!(session.baseline() == &snapshot.version);
            assert_eq!(
                session.observe_external(Ok(snapshot)),
                Ok(ExternalChange::Unchanged)
            );
        }
    }
}

#[test]
fn deleted_or_moved_blocks_save_and_reappearance_keeps_edits() {
    for dirty in [false, true] {
        let (storage, mut session) = open();
        if dirty {
            edit(&mut session);
        }
        let before = bytes(&session);
        storage.set_snapshot(session.resource(), None).unwrap();
        assert_eq!(
            session.observe_external(storage.read(session.resource())),
            Ok(ExternalChange::Missing)
        );
        assert!(session.is_missing());
        assert_eq!(session.save(), Err(StorageError::NotFound.into()));
        assert!(matches!(
            storage.read(session.resource()),
            Err(StorageError::NotFound)
        ));
        assert!(bytes(&session) == before);
        let snapshot = change(&storage, &session, EXTERNAL);
        session.observe_external(Ok(snapshot)).unwrap();
        assert!(!session.is_missing());
        assert_eq!(session.is_conflicted(), dirty);
        assert!(bytes(&session) == before);
    }
}

#[test]
fn save_matching_version_noops_clean_or_persists_dirty() {
    let (storage, mut session) = open();
    let baseline = session.baseline().clone();
    assert_eq!(session.save(), Ok(SaveOutcome::Unchanged));
    assert!(session.baseline() == &baseline);
    edit(&mut session);
    assert_eq!(session.save(), Ok(SaveOutcome::Saved));
    let disk = storage.read(session.resource()).unwrap();
    assert!(disk.bytes.expose() == EDITED);
    assert!(session.baseline() == &disk.version);
    assert!(session.baseline().identity != baseline.identity);
    assert!(!session.is_dirty());
    assert!(session.undo().unwrap());
    assert!(session.is_dirty());
    assert!(bytes(&session) == ORIGINAL);
    assert!(session.redo().unwrap());
    assert!(!session.is_dirty());
    assert!(bytes(&session) == EDITED);
}

#[test]
fn save_different_version_reloads_clean_or_blocks_dirty() {
    for dirty in [false, true] {
        let (storage, mut session) = open();
        if dirty {
            edit(&mut session);
        }
        let baseline = session.baseline().clone();
        let disk = change(&storage, &session, EXTERNAL);
        if dirty {
            assert_eq!(session.save(), Err(StorageError::Conflict.into()));
            assert!(session.is_conflicted());
            assert!(session.baseline() == &baseline);
            assert!(session.external_snapshot() == Some(&disk));
            assert!(bytes(&session) == EDITED);
        } else {
            assert_eq!(session.save(), Ok(SaveOutcome::Reloaded));
            assert!(bytes(&session) == EXTERNAL);
        }
        assert!(storage.read(session.resource()).unwrap() == disk);
    }
}

#[test]
fn reload_and_discard_reloads_and_clears_history() {
    for dirty in [false, true] {
        let (storage, mut session) = open();
        if dirty {
            edit(&mut session);
        }
        let disk = change(&storage, &session, EXTERNAL);
        session.reload_and_discard().unwrap();
        assert!(bytes(&session) == EXTERNAL);
        assert!(session.baseline() == &disk.version);
        assert!(!session.is_dirty());
        assert!(!session.can_undo());
        assert!(!session.can_redo());
    }
}

#[test]
fn save_as_adopts_independent_copy_and_preserves_original() {
    for dirty in [false, true] {
        let (storage, mut session) = open();
        let original = session.resource().clone();
        if dirty {
            edit(&mut session);
            let disk = change(&storage, &session, EXTERNAL);
            session.observe_external(Ok(disk)).unwrap();
        }
        let before = bytes(&session);
        let original_snapshot = storage.read(&original).unwrap();
        let destination = ResourceRef::from_uri("copy");
        assert_eq!(
            session.save_as(destination.clone(), SaveAsTarget::New),
            Ok(SaveOutcome::Saved)
        );
        assert!(session.resource() == &destination);
        let copy = storage.read(&destination).unwrap();
        assert!(session.baseline() == &copy.version);
        assert!(copy.bytes.expose() == &before);
        assert!(storage.read(&original).unwrap() == original_snapshot);
        assert!(bytes(&session) == before);
        assert!(!session.is_dirty() && !session.is_conflicted());
        assert_eq!(session.can_undo(), dirty);
    }
}

#[test]
fn confirmed_overwrite_requires_the_reviewed_full_version() {
    let (storage, mut session) = open();
    edit(&mut session);
    let reviewed = change(&storage, &session, EXTERNAL);
    session.observe_external(Ok(reviewed.clone())).unwrap();
    let mut newer = reviewed.clone();
    newer.version.etag = Some(Redacted::new("new".into()));
    storage
        .set_snapshot(session.resource(), Some(newer.clone()))
        .unwrap();
    assert_eq!(
        session.overwrite_confirmed(&reviewed.version),
        Err(SessionError::Conflict)
    );
    assert!(storage.read(session.resource()).unwrap() == newer);
    assert!(bytes(&session) == EDITED);
    assert_eq!(
        session.overwrite_confirmed(&newer.version),
        Ok(SaveOutcome::Saved)
    );
    assert!(storage.read(session.resource()).unwrap().bytes.expose() == EDITED);
}

#[test]
fn post_write_failure_recovers_version_warns_and_keeps_history() {
    let (storage, mut session) = open();
    edit(&mut session);
    storage
        .configure(
            session.resource(),
            FakeSettings {
                post_write_failure: Some(PostWriteFailure::Mode),
                ..FakeSettings::default()
            },
        )
        .unwrap();
    assert_eq!(
        session.save(),
        Ok(SaveOutcome::WrittenWithWarning {
            reason: PostWriteFailure::Mode,
            baseline_recovered: true,
            recovery_error: None,
        })
    );
    let disk = storage.read(session.resource()).unwrap();
    assert!(disk.bytes.expose() == EDITED);
    assert!(session.baseline() == &disk.version);
    assert!(session.is_dirty() && session.can_undo());
    assert!(!session.is_saving());
}

#[test]
fn read_only_allows_in_memory_edits_but_blocks_save() {
    let (storage, mut session) = open();
    let mut settings = FakeSettings::default();
    settings.capabilities.is_writable = false;
    storage.configure(session.resource(), settings).unwrap();
    session
        .observe_external(storage.read(session.resource()))
        .unwrap();
    assert!(session.is_read_only());
    edit(&mut session);
    assert_eq!(session.save(), Err(StorageError::ReadOnly.into()));
    assert!(bytes(&session) == EDITED);
    assert!(storage.read(session.resource()).unwrap().bytes.expose() == ORIGINAL);
}

#[test]
fn protected_utf8_and_unclosed_quotes_follow_parser_modes() {
    let (_, mut session) = open_with(b"A=\xff");
    assert!(session.is_protected());
    assert!(session.protected_bytes().unwrap().expose() == b"A=\xff");
    assert_eq!(session.undo(), Err(SessionError::Protected));
    let (_, mut session) = open_with(b"A=x\nB=\"unclosed");
    assert!(!session.is_protected());
    assert!(matches!(
        session.document().unwrap().nodes().last().unwrap().kind(),
        NodeKind::Raw
    ));
    assert!(session.document().unwrap().serialize() == b"A=x\nB=\"unclosed");
    assert_eq!(session.redo(), Ok(false));
}

#[test]
fn events_preserve_unsaved_bytes_except_edits_and_confirmed_discard() {
    let (storage, mut session) = open();
    edit(&mut session);
    let before = bytes(&session);
    let failure = StorageError::Io {
        kind: std::io::ErrorKind::Other,
    };
    storage
        .configure(
            session.resource(),
            FakeSettings {
                write_error: Some(failure),
                ..FakeSettings::default()
            },
        )
        .unwrap();
    let events: [fn(&FakeStorage, &mut DocumentSession); 8] = [
        |_, s| {
            assert_eq!(s.save(), Err(failure_for_test().into()));
        },
        |_, s| {
            assert!(s.observe_external(Err(failure_for_test())).is_err());
        },
        |disk, s| {
            s.observe_external(Ok(change(disk, s, ORIGINAL))).unwrap();
        },
        |disk, s| {
            s.observe_external(Ok(change(disk, s, EXTERNAL))).unwrap();
        },
        |_, s| {
            assert!(!s.reload_if_clean().unwrap());
        },
        |_, s| {
            assert!(s.save_as(s.resource().clone(), SaveAsTarget::New).is_err());
        },
        |disk, s| {
            disk.set_snapshot(s.resource(), None).unwrap();
            s.observe_external(Err(StorageError::NotFound)).unwrap();
        },
        |disk, s| {
            s.observe_external(Ok(change(disk, s, EXTERNAL))).unwrap();
        },
    ];
    for event in events {
        event(&storage, &mut session);
        assert!(bytes(&session) == before);
        assert!(session.is_dirty() && session.can_undo());
        assert!(!session.is_saving());
    }
    assert!(session.undo().unwrap());
    assert!(bytes(&session) == ORIGINAL);
    assert!(session.redo().unwrap());
    assert!(bytes(&session) == before);
    session.reload_and_discard().unwrap();
    assert!(bytes(&session) == EXTERNAL);
}

fn failure_for_test() -> StorageError {
    StorageError::Io {
        kind: std::io::ErrorKind::Other,
    }
}

#[test]
fn session_snapshot_and_errors_redact_sentinel() {
    fn require_send<T: Send>() {}
    require_send::<DocumentSession>();
    let content = format!("{SENTINEL}={SENTINEL}\n");
    let (storage, mut session) = open_with(content.as_bytes());
    edit(&mut session);
    let snapshot = change(&storage, &session, format!("{content}A=x\n").as_bytes());
    session.observe_external(Ok(snapshot)).unwrap();
    let error = session.save().unwrap_err();
    for output in [
        format!("{session:?}"),
        format!("{session:#?}"),
        format!("{:#?}", session.external_snapshot()),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!output.contains(SENTINEL));
    }
}

#[test]
fn metadata_only_observation_still_requires_full_version_for_save() {
    let (storage, mut session) = open();
    edit(&mut session);
    let baseline = session.baseline().clone();
    let mut disk = storage.read(session.resource()).unwrap();
    disk.version.etag = Some(Redacted::new("changed".into()));
    storage
        .set_snapshot(session.resource(), Some(disk.clone()))
        .unwrap();
    assert_eq!(
        session.observe_external(Ok(disk.clone())),
        Ok(ExternalChange::MetadataOnly)
    );
    assert_eq!(session.save(), Err(StorageError::Conflict.into()));
    assert!(session.is_dirty() && session.is_conflicted());
    assert!(session.baseline() == &baseline);
    assert!(session.external_snapshot() == Some(&disk));
    assert!(bytes(&session) == EDITED);
}

#[test]
fn saving_rejects_edits_and_other_transitions() {
    let (_, mut session) = open();
    edit(&mut session);
    session.is_saving = true;
    assert!(session.is_saving());
    assert_eq!(session.undo(), Err(SessionError::Saving));
    assert_eq!(session.redo(), Err(SessionError::Saving));
    assert_eq!(session.apply_batch([]), Err(SessionError::Saving));
    assert_eq!(
        session.observe_external(Err(StorageError::NotFound)),
        Err(SessionError::Saving)
    );
    assert_eq!(session.reload_and_discard(), Err(SessionError::Saving));
    assert_eq!(session.save(), Err(SessionError::Saving));
    assert!(bytes(&session) == EDITED);
}

#[test]
fn mode_warning_survives_reload_until_acknowledged() {
    let (storage, mut session) = open();
    edit(&mut session);
    storage
        .configure(
            session.resource(),
            FakeSettings {
                post_write_failure: Some(PostWriteFailure::Mode),
                ..FakeSettings::default()
            },
        )
        .unwrap();
    assert!(matches!(
        session.save(),
        Ok(SaveOutcome::WrittenWithWarning { .. })
    ));
    assert_eq!(session.permission_warning(), Some(PostWriteFailure::Mode));
    assert_eq!(
        session.save(),
        Err(SessionError::PostWriteWarningUnacknowledged)
    );
    session.reload_and_discard().unwrap();
    change(&storage, &session, EXTERNAL);
    assert_eq!(
        session.refresh_external(),
        Ok(ExternalChange::ReloadAvailable)
    );
    assert_eq!(session.reload_if_clean(), Ok(false));
    assert!(bytes(&session) == EDITED);
    assert_eq!(
        session.save(),
        Err(SessionError::PostWriteWarningUnacknowledged)
    );
    storage
        .configure(session.resource(), FakeSettings::default())
        .unwrap();
    session.acknowledge_post_write_warning();
    assert_eq!(session.save(), Ok(SaveOutcome::Reloaded));
    edit(&mut session);
    assert_eq!(session.save(), Ok(SaveOutcome::Saved));
}

#[test]
fn identity_warning_requires_reviewed_overwrite_and_content_warning_can_retry() {
    for reason in [PostWriteFailure::Identity, PostWriteFailure::Content] {
        let (storage, mut session) = open();
        edit(&mut session);
        let baseline = session.baseline().clone();
        storage
            .configure(
                session.resource(),
                FakeSettings {
                    post_write_failure: Some(reason),
                    ..FakeSettings::default()
                },
            )
            .unwrap();
        let is_identity = reason == PostWriteFailure::Identity;
        assert_eq!(
            session.save(),
            Ok(SaveOutcome::WrittenWithWarning {
                reason,
                baseline_recovered: !is_identity,
                recovery_error: None,
            })
        );
        storage
            .configure(session.resource(), FakeSettings::default())
            .unwrap();
        let disk = storage.read(session.resource()).unwrap();
        assert!(disk.bytes.expose() == EDITED);
        if is_identity {
            assert!(session.baseline() == &baseline);
            assert!(session.is_conflicted());
            assert!(session.external_snapshot() == Some(&disk));
            assert_eq!(
                session.overwrite_confirmed(&disk.version),
                Err(SessionError::PostWriteWarningUnacknowledged)
            );
            session.undo().unwrap();
            assert!(!session.is_dirty());
            assert_eq!(session.reload_if_clean(), Ok(false));
            session.redo().unwrap();
            session.acknowledge_post_write_warning();
            assert_eq!(
                session.overwrite_confirmed(&disk.version),
                Ok(SaveOutcome::Saved)
            );
        } else {
            assert_eq!(session.permission_warning(), None);
            assert_eq!(session.save(), Ok(SaveOutcome::Saved));
        }
    }
}

#[test]
fn save_as_rejects_existing_name_and_inode_alias_then_accepts_distinct_destination() {
    let (storage, mut session) = open();
    edit(&mut session);
    session.permission_warning = Some(PostWriteFailure::Mode);
    let original = session.resource().clone();
    let destination = ResourceRef::from_uri("copy");
    let alias = storage.read(&original).unwrap();
    storage
        .set_snapshot(&destination, Some(alias.clone()))
        .unwrap();
    assert_eq!(
        session.save_as(destination.clone(), SaveAsTarget::New),
        Err(StorageError::AlreadyExists.into())
    );
    assert_eq!(
        session.save_as(destination.clone(), SaveAsTarget::ReplaceExisting),
        Err(SessionError::InvalidSaveAs)
    );
    assert_eq!(session.permission_warning(), Some(PostWriteFailure::Mode));
    assert!(session.resource() == &original);
    assert!(session.is_dirty() && session.can_undo());
    let mut distinct = alias.clone();
    distinct.version.identity.as_mut().unwrap().device_and_inode = Some((1, 42));
    storage.set_snapshot(&destination, Some(distinct)).unwrap();
    assert_eq!(
        session.save_as(destination.clone(), SaveAsTarget::ReplaceExisting),
        Ok(SaveOutcome::Saved)
    );
    assert!(session.resource() == &destination);
    assert!(storage.read(&original).unwrap() == alias);
    assert!(storage.read(&destination).unwrap().bytes.expose() == &bytes(&session));
    assert_eq!(session.permission_warning(), None);
}

#[test]
fn save_as_replace_rejects_destination_with_unknown_identity() {
    let (storage, mut session) = open();
    edit(&mut session);
    let original = session.resource().clone();
    let destination = ResourceRef::from_uri("unknown");
    let mut unknown = storage.read(&original).unwrap();
    unknown.version.identity = None;
    storage.set_snapshot(&destination, Some(unknown)).unwrap();
    assert_eq!(
        session.save_as(destination.clone(), SaveAsTarget::ReplaceExisting),
        Err(SessionError::InvalidSaveAs)
    );
    assert!(session.resource() == &original);
    assert!(session.is_dirty() && session.can_undo());
    assert_eq!(storage.replace_calls(), 0);
}

#[test]
fn conflict_without_snapshot_recovers_through_fresh_observation() {
    for read_error in [None, Some(StorageError::NotFound)] {
        let (storage, mut session) = open();
        edit(&mut session);
        storage
            .configure(
                session.resource(),
                FakeSettings {
                    read_error,
                    write_error: Some(StorageError::Conflict),
                    ..FakeSettings::default()
                },
            )
            .unwrap();
        assert_eq!(session.save(), Err(StorageError::Conflict.into()));
        assert!(!session.is_conflicted());
        assert!(session.external_snapshot().is_none());
        assert_eq!(session.last_observation_error(), read_error);
        storage
            .configure(session.resource(), FakeSettings::default())
            .unwrap();
        let disk = change(&storage, &session, EXTERNAL);
        assert_eq!(session.refresh_external(), Ok(ExternalChange::Conflict));
        assert_eq!(session.last_observation_error(), None);
        assert_eq!(
            session.overwrite_confirmed(&disk.version),
            Ok(SaveOutcome::Saved)
        );
    }
}

#[test]
fn read_only_overwrite_stops_before_storage_read() {
    let (storage, mut session) = open();
    edit(&mut session);
    let disk = change(&storage, &session, EXTERNAL);
    let mut settings = FakeSettings::default();
    settings.capabilities.is_writable = false;
    storage.configure(session.resource(), settings).unwrap();
    session.refresh_external().unwrap();
    settings.read_error = Some(StorageError::TooLarge);
    storage.configure(session.resource(), settings).unwrap();
    assert_eq!(
        session.overwrite_confirmed(&disk.version),
        Err(StorageError::ReadOnly.into())
    );
    assert_eq!(session.save(), Err(StorageError::ReadOnly.into()));
}

#[test]
fn clean_reload_requires_explicit_action_for_each_blocker() {
    for (is_conflicted, needs_baseline_recovery, permission_warning) in [
        (true, false, None),
        (false, true, None),
        (false, false, Some(PostWriteFailure::Mode)),
    ] {
        let (storage, mut session) = open();
        change(&storage, &session, EXTERNAL);
        assert_eq!(
            session.refresh_external(),
            Ok(ExternalChange::ReloadAvailable)
        );
        session.is_conflicted = is_conflicted;
        session.needs_baseline_recovery = needs_baseline_recovery;
        session.permission_warning = permission_warning;
        assert_eq!(session.reload_if_clean(), Ok(false));
        assert!(bytes(&session) == ORIGINAL);
        session.reload_and_discard().unwrap();
        assert!(bytes(&session) == EXTERNAL);
        assert_eq!(session.permission_warning(), permission_warning);
    }
}

#[test]
fn save_routes_missing_guarantees_to_save_as_without_losing_edits() {
    for (reason, advisory) in [
        (StorageError::ConditionalReplaceUnsupported, true),
        (StorageError::AtomicReplaceUnsupported, true),
        (StorageError::PermissionsNotPreserved, true),
        (StorageError::PermissionsNotPreserved, false),
    ] {
        let (storage, mut session) = open();
        edit(&mut session);
        let mut settings = FakeSettings::default();
        match (reason, advisory) {
            (StorageError::ConditionalReplaceUnsupported, true) => {
                settings.capabilities.supports_conditional_replace = false;
            }
            (StorageError::AtomicReplaceUnsupported, true) => {
                settings.capabilities.supports_atomic_replace = false;
            }
            (StorageError::PermissionsNotPreserved, true) => {
                settings.capabilities.can_preserve_permissions = false;
            }
            _ => settings.write_error = Some(reason),
        }
        storage.configure(session.resource(), settings).unwrap();
        let error = session.save().unwrap_err();
        assert_eq!(error, SessionError::SaveAsRequired { reason });
        assert!(!format!("{error:?} {error}").contains(SENTINEL));
        assert_eq!(storage.replace_calls(), usize::from(!advisory));
        assert!(session.is_dirty() && session.can_undo());
        assert!(bytes(&session) == EDITED);
        assert!(storage.read(session.resource()).unwrap().bytes.expose() == ORIGINAL);
    }
}

#[test]
fn save_as_switches_to_published_file_even_when_verification_fails() {
    let (storage, mut session) = open();
    edit(&mut session);
    let original = session.resource().clone();
    let destination = ResourceRef::from_uri(format!("{SENTINEL}-copy"));
    let reason = PostWriteFailure::Mode;
    storage
        .configure(
            &destination,
            FakeSettings {
                create_error: Some(StorageError::PostWriteVerificationFailed { reason }),
                ..FakeSettings::default()
            },
        )
        .unwrap();
    assert_eq!(
        session.save_as(destination.clone(), SaveAsTarget::New),
        Ok(SaveOutcome::WrittenWithWarning {
            reason,
            baseline_recovered: true,
            recovery_error: None
        })
    );
    assert!(session.resource() == &destination);
    assert!(storage.read(&destination).unwrap().bytes.expose() == EDITED);
    assert!(storage.read(&original).unwrap().bytes.expose() == ORIGINAL);
    assert!(session.is_dirty() && session.can_undo());
    assert_eq!(session.permission_warning(), Some(reason));
}

#[test]
fn staging_cleanup_is_once_per_successful_open() {
    let (storage, mut session) = open_with(b"A=\xff");
    assert_eq!(storage.cleanup_calls(), 1);
    session.refresh_external().unwrap();
    assert_eq!(storage.cleanup_calls(), 1);
    assert_eq!(
        session.save_as(ResourceRef::from_uri(SENTINEL), SaveAsTarget::New),
        Err(SessionError::Protected)
    );
}

#[test]
fn save_as_prepublication_failures_keep_original_and_conflict() {
    for failure in [
        failure_for_test(),
        StorageError::PermissionsNotPreserved,
        StorageError::TooLarge,
    ] {
        let (storage, mut session) = open();
        edit(&mut session);
        let original = session.resource().clone();
        let external = change(&storage, &session, EXTERNAL);
        session.observe_external(Ok(external.clone())).unwrap();
        let baseline = session.baseline().clone();
        let destination = ResourceRef::from_uri(format!("{SENTINEL}-destination"));
        let mut settings = FakeSettings::default();
        if failure == StorageError::TooLarge {
            settings.max_file_size = 1;
        } else {
            settings.create_error = Some(failure);
        }
        storage.configure(&destination, settings).unwrap();

        let error = session
            .save_as(destination.clone(), SaveAsTarget::New)
            .unwrap_err();
        assert_eq!(error, failure.into());
        assert!(!format!("{error:?} {error}").contains(SENTINEL));
        assert!(session.resource() == &original);
        assert!(session.baseline() == &baseline);
        assert!(session.external_snapshot() == Some(&external));
        assert!(session.is_conflicted() && session.is_dirty() && session.can_undo());
        assert!(bytes(&session) == EDITED);
        assert!(storage.read(&original).unwrap() == external);
        assert_eq!(storage.read(&destination), Err(StorageError::NotFound));
    }
}

#[test]
fn save_as_rejects_alias_of_original_after_external_replacement() {
    let (storage, mut session) = open();
    edit(&mut session);
    let original = session.resource().clone();
    let destination = ResourceRef::from_uri(format!("{SENTINEL}-alias"));
    let mut external = snapshot_from_bytes(EXTERNAL.to_vec());
    external.version.identity.as_mut().unwrap().device_and_inode = Some((1, 99));
    storage
        .set_snapshot(&original, Some(external.clone()))
        .unwrap();
    storage
        .set_snapshot(&destination, Some(external.clone()))
        .unwrap();
    assert_eq!(
        session.save_as(destination, SaveAsTarget::ReplaceExisting),
        Err(SessionError::InvalidSaveAs)
    );
    assert_eq!(storage.replace_calls(), 0);
    assert!(storage.read(&original).unwrap() == external);
    assert!(session.resource() == &original);
    assert!(session.is_dirty());
}
