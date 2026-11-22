use std::io;

use super::fake::{FakeSettings, FakeStorage, snapshot_from_bytes};
use super::*;
use crate::test_support::SENTINEL;

const ORIGINAL: &[u8] = b"A=old";
const EXTERNAL: &[u8] = b"A=new";
const EDITED: &[u8] = b"A=mine";

fn opened(uri: &str, snapshot: ReadSnapshot) -> (FakeStorage, ResourceRef, DiskVersion) {
    let storage = FakeStorage::default();
    let resource = ResourceRef::from_uri(uri);
    let version = snapshot.version.clone();
    storage.set_snapshot(&resource, Some(snapshot)).unwrap();
    (storage, resource, version)
}

#[test]
fn external_edit_is_observable_and_stale_save_conflicts_without_monitor() {
    let mut snapshot = snapshot_from_bytes(ORIGINAL.to_vec());
    snapshot.version.etag = Some(Redacted::new("same-etag".into()));
    let (storage, resource, baseline) = opened("file:///example", snapshot.clone());
    snapshot.bytes = Redacted::new(EXTERNAL.to_vec());
    snapshot.version.hash = Redacted::new(content_hash(EXTERNAL));
    storage
        .set_snapshot(&resource, Some(snapshot.clone()))
        .unwrap();
    assert!(storage.read(&resource).unwrap() == snapshot);
    assert_eq!(
        storage.replace_if_unchanged(&resource, &baseline, EDITED),
        Err(StorageError::Conflict)
    );
    assert!(storage.read(&resource).unwrap() == snapshot);
}

#[test]
fn metadata_only_replacement_conflicts() {
    let mut snapshot = snapshot_from_bytes(ORIGINAL.to_vec());
    let (storage, resource, baseline) = opened("file:///example", snapshot.clone());
    snapshot.version.identity.as_mut().unwrap().device_and_inode = Some((1, 3));
    storage
        .set_snapshot(&resource, Some(snapshot.clone()))
        .unwrap();
    assert_eq!(
        storage.replace_if_unchanged(&resource, &baseline, EDITED),
        Err(StorageError::Conflict)
    );
    assert!(storage.read(&resource).unwrap() == snapshot);
}

#[test]
fn missing_and_deleted_files_stay_absent() {
    let (storage, resource, baseline) =
        opened("file:///example", snapshot_from_bytes(ORIGINAL.to_vec()));
    assert!(matches!(
        storage.read(&ResourceRef::from_uri("file:///missing")),
        Err(StorageError::NotFound)
    ));
    storage.set_snapshot(&resource, None).unwrap();
    assert_eq!(
        storage.replace_if_unchanged(&resource, &baseline, EDITED),
        Err(StorageError::Conflict)
    );
    assert!(matches!(
        storage.read(&resource),
        Err(StorageError::NotFound)
    ));
}

#[test]
fn private_mode_is_preserved_on_success() {
    let snapshot = snapshot_from_bytes(ORIGINAL.to_vec());
    let (storage, resource, baseline) = opened("file:///private", snapshot);
    let saved = storage
        .replace_if_unchanged(&resource, &baseline, EDITED)
        .unwrap();
    let mut identity = saved.identity.clone().unwrap();
    let baseline_identity = baseline.identity.unwrap();
    assert_ne!(
        identity.device_and_inode,
        baseline_identity.device_and_inode
    );
    identity.device_and_inode = baseline_identity.device_and_inode;
    assert_eq!(identity, baseline_identity);
    assert_eq!(saved.etag, baseline.etag);
    assert_eq!(saved.mtime, baseline.mtime);
    assert!(saved.hash.expose() == &content_hash(EDITED));
    assert_eq!(saved.size, EDITED.len() as u64);
    let reread = storage.read(&resource).unwrap();
    assert!(reread.bytes.expose() == EDITED);
    assert_eq!(reread.version, saved);
}

#[test]
fn unsafe_capabilities_block_save_without_touching_bytes() {
    let original = snapshot_from_bytes(ORIGINAL.to_vec());
    let (storage, resource, _) = opened("file:///example", original.clone());
    let baseline = snapshot_from_bytes(EXTERNAL.to_vec()).version;
    let supported = FakeSettings::default().capabilities;
    for (capabilities, expected) in [
        (
            StorageCapabilities {
                is_writable: false,
                ..supported
            },
            StorageError::ReadOnly,
        ),
        (
            StorageCapabilities {
                supports_conditional_replace: false,
                ..supported
            },
            StorageError::ConditionalReplaceUnsupported,
        ),
        (
            StorageCapabilities {
                supports_atomic_replace: false,
                ..supported
            },
            StorageError::AtomicReplaceUnsupported,
        ),
        (
            StorageCapabilities {
                can_preserve_permissions: false,
                ..supported
            },
            StorageError::PermissionsNotPreserved,
        ),
    ] {
        storage
            .configure(
                &resource,
                FakeSettings {
                    capabilities,
                    ..FakeSettings::default()
                },
            )
            .unwrap();
        assert_eq!(storage.capabilities(&resource), capabilities);
        assert_eq!(
            storage.replace_if_unchanged(&resource, &baseline, EDITED),
            Err(expected)
        );
        assert!(storage.read(&resource).unwrap() == original);
    }
}

#[test]
fn symlink_is_neither_read_nor_replaced() {
    let mut snapshot = snapshot_from_bytes(ORIGINAL.to_vec());
    snapshot.version.identity = Some(FileIdentity {
        is_symlink: Some(true),
        ..FileIdentity::default()
    });
    let (storage, resource, _) = opened("file:///link", snapshot);
    let baseline = snapshot_from_bytes(EXTERNAL.to_vec()).version;
    let target_snapshot = snapshot_from_bytes(ORIGINAL.to_vec());
    let target = ResourceRef::from_uri("file:///target");
    storage
        .set_snapshot(&target, Some(target_snapshot.clone()))
        .unwrap();
    assert!(matches!(
        storage.read(&resource),
        Err(StorageError::Symlink)
    ));
    assert_eq!(
        storage.replace_if_unchanged(&resource, &baseline, EDITED),
        Err(StorageError::Symlink)
    );
    assert!(storage.read(&target).unwrap() == target_snapshot);
    assert!(matches!(
        storage.read(&resource),
        Err(StorageError::Symlink)
    ));
}

#[test]
fn non_regular_file_takes_precedence_over_a_stale_baseline() {
    let mut snapshot = snapshot_from_bytes(ORIGINAL.to_vec());
    snapshot.version.identity.as_mut().unwrap().mode = Some(0o040700);
    let (storage, resource, _) = opened("file:///directory", snapshot);
    let baseline = snapshot_from_bytes(EXTERNAL.to_vec()).version;
    assert!(matches!(
        storage.read(&resource),
        Err(StorageError::NotRegularFile)
    ));
    assert_eq!(
        storage.replace_if_unchanged(&resource, &baseline, EDITED),
        Err(StorageError::NotRegularFile)
    );
}

#[test]
fn portal_resource_can_save_or_report_missing_guarantees() {
    let mut snapshot = snapshot_from_bytes(ORIGINAL.to_vec());
    let identity = snapshot.version.identity.as_mut().unwrap();
    identity.parent = None;
    identity.selinux_label = None;
    let (storage, resource, baseline) = opened("file:///run/user/1000/doc/grant/example", snapshot);
    storage
        .configure(
            &resource,
            FakeSettings {
                is_portal_file: true,
                ..FakeSettings::default()
            },
        )
        .unwrap();
    let saved = storage
        .replace_if_unchanged(&resource, &baseline, EDITED)
        .unwrap();
    let actual = storage.read(&resource).unwrap();
    assert!(actual.bytes.expose() == EDITED);
    assert_eq!(actual.version, saved);
    storage
        .configure(
            &resource,
            FakeSettings {
                capabilities: StorageCapabilities {
                    is_writable: true,
                    ..StorageCapabilities::default()
                },
                ..FakeSettings::default()
            },
        )
        .unwrap();
    assert_eq!(
        storage.replace_if_unchanged(&resource, &saved, EXTERNAL),
        Err(StorageError::ConditionalReplaceUnsupported)
    );
    assert!(storage.read(&resource).unwrap() == actual);
}

#[test]
fn injected_failures_are_value_free_and_leave_snapshot_intact() {
    let snapshot = snapshot_from_bytes(SENTINEL.as_bytes().to_vec());
    let (storage, resource, baseline) = opened("file:///example", snapshot.clone());
    for error in [
        StorageError::NotFound,
        StorageError::PermissionDenied,
        StorageError::NotRegularFile,
        StorageError::Symlink,
        StorageError::TooLarge,
        StorageError::ReadOnly,
        StorageError::Conflict,
        StorageError::ConditionalReplaceUnsupported,
        StorageError::AtomicReplaceUnsupported,
        StorageError::PermissionsNotPreserved,
        StorageError::Io {
            kind: io::ErrorKind::StorageFull,
        },
    ] {
        assert!(!format!("{error:?} {error}").contains(SENTINEL));
        storage
            .configure(
                &resource,
                FakeSettings {
                    read_error: Some(error),
                    ..FakeSettings::default()
                },
            )
            .unwrap();
        assert!(matches!(storage.read(&resource), Err(actual) if actual == error));
        let preflight_error = if error == StorageError::NotFound {
            StorageError::Conflict
        } else {
            error
        };
        assert_eq!(
            storage.replace_if_unchanged(&resource, &baseline, EDITED),
            Err(preflight_error)
        );
        storage
            .configure(
                &resource,
                FakeSettings {
                    write_error: Some(error),
                    ..FakeSettings::default()
                },
            )
            .unwrap();
        assert_eq!(
            storage.replace_if_unchanged(&resource, &baseline, EDITED),
            Err(error)
        );
        assert!(storage.read(&resource).unwrap() == snapshot);
    }
}

#[test]
fn debug_hides_uri_content_etag_hash_and_label() {
    let resource = ResourceRef::from_uri(format!("file:///{SENTINEL}"));
    let mut snapshot = snapshot_from_bytes(SENTINEL.as_bytes().to_vec());
    snapshot.version.size = 9_876_543_210;
    snapshot.version.etag = Some(Redacted::new(SENTINEL.into()));
    snapshot.version.identity = Some(FileIdentity {
        selinux_label: Some(Some(Redacted::new(SENTINEL.as_bytes().to_vec()))),
        ..FileIdentity::default()
    });
    for output in [
        format!("{resource:#?}"),
        format!("{snapshot:#?}"),
        format!("{:#?}", snapshot.version),
    ] {
        assert!(!output.contains(SENTINEL));
        assert!(!output.contains("9876543210"));
        assert!(!output.contains(&format!("{:?}", SENTINEL.as_bytes())));
        assert!(!output.contains(&format!("{:?}", snapshot.version.hash.expose())));
    }
}

#[test]
fn content_hash_matches_sha256_known_vector() {
    assert!(
        content_hash(b"abc")
            == [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
    );
}

#[test]
fn missing_resource_takes_precedence_and_is_never_created() {
    let storage = FakeStorage::default();
    let resource = ResourceRef::from_uri("file:///missing");
    let baseline = snapshot_from_bytes(ORIGINAL.to_vec()).version;
    assert_eq!(
        storage.capabilities(&resource),
        StorageCapabilities::default()
    );
    assert_eq!(
        storage.replace_if_unchanged(&resource, &baseline, EDITED),
        Err(StorageError::Conflict)
    );
    assert!(matches!(
        storage.read(&resource),
        Err(StorageError::NotFound)
    ));
    assert_eq!(
        storage.capabilities(&resource),
        StorageCapabilities::default()
    );

    storage
        .configure(
            &resource,
            FakeSettings {
                read_error: Some(StorageError::PermissionDenied),
                ..FakeSettings::default()
            },
        )
        .unwrap();
    assert!(matches!(
        storage.read(&resource),
        Err(StorageError::NotFound)
    ));
    assert_eq!(
        storage.replace_if_unchanged(&resource, &baseline, EDITED),
        Err(StorageError::Conflict)
    );
}

#[test]
fn unsafe_identity_blocks_replacement_with_bytes_intact() {
    let original = snapshot_from_bytes(ORIGINAL.to_vec());
    let identity = original.version.identity.clone().unwrap();
    for (observed, error) in [
        (None, StorageError::PermissionsNotPreserved),
        (
            Some(FileIdentity {
                is_symlink: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                nlink: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                mode: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                uid: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                gid: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                gid: Some(65_534),
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                device_and_inode: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                has_access_acl: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                parent: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                selinux_label: None,
                ..identity.clone()
            }),
            StorageError::PermissionsNotPreserved,
        ),
        (
            Some(FileIdentity {
                nlink: Some(2),
                ..identity
            }),
            StorageError::AtomicReplaceUnsupported,
        ),
    ] {
        let mut snapshot = original.clone();
        snapshot.version.identity = observed;
        let (storage, resource, baseline) = opened("file:///example", snapshot.clone());
        assert_eq!(
            storage.replace_if_unchanged(&resource, &baseline, EDITED),
            Err(error)
        );
        assert!(storage.read(&resource).unwrap() == snapshot);
    }
}

#[test]
fn post_write_failures_leave_new_bytes_and_require_a_new_baseline() {
    for reason in [
        PostWriteFailure::Mode,
        PostWriteFailure::Owner,
        PostWriteFailure::Acl,
        PostWriteFailure::SelinuxLabel,
        PostWriteFailure::Content,
        PostWriteFailure::Identity,
    ] {
        let (storage, resource, baseline) = opened(
            &format!("file:///{SENTINEL}"),
            snapshot_from_bytes(ORIGINAL.to_vec()),
        );
        storage
            .configure(
                &resource,
                FakeSettings {
                    post_write_failure: Some(reason),
                    ..FakeSettings::default()
                },
            )
            .unwrap();
        let error = StorageError::PostWriteVerificationFailed { reason };
        assert_eq!(
            storage.replace_if_unchanged(&resource, &baseline, SENTINEL.as_bytes()),
            Err(error)
        );
        assert!(storage.read(&resource).unwrap().bytes.expose() == SENTINEL.as_bytes());
        assert_eq!(
            storage.replace_if_unchanged(&resource, &baseline, EDITED),
            Err(StorageError::Conflict)
        );
        assert!(!format!("{reason:?} {error:?} {error}").contains(SENTINEL));
    }
}
