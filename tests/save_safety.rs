#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "unexpected I/O failures fail the test")]

mod common;

use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::Arc,
};

use envy::{
    domain::{edit::ConversionPolicy, history::EditRequest, redact::Redacted},
    infrastructure::gio_storage::GioStorage,
    services::{
        document_service::{DocumentSession, SaveAsTarget, SaveOutcome, SessionError},
        storage::{DocumentStorage, ResourceRef, StorageError},
    },
};
use gio::prelude::*;

// Same marker as the crate-private unit test support and the Gio integration tests.
const SENTINEL: &str = "ENVY_TEST_SENTINEL_9F2C";

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn save_after_unobserved_deletion_reports_missing_or_conflict_without_recreating() {
    let root = TempDir::new();
    let path = root.0.join(SENTINEL);
    fs::write(&path, format!("{SENTINEL}={SENTINEL}\n")).unwrap();
    let resource = ResourceRef::from_uri(gio::File::for_path(&path).uri().as_str());
    let mut session = DocumentSession::open(Arc::new(GioStorage), resource).unwrap();
    session
        .apply(EditRequest::InsertAssignment {
            key: Redacted::new("LOCAL"),
            value: Redacted::new(SENTINEL),
            policy: ConversionPolicy::Reject,
        })
        .unwrap();
    let edited = session.document().unwrap().serialize();

    fs::remove_file(&path).unwrap();
    let result = session.save();

    assert!(fs::read_dir(&root.0).unwrap().next().is_none());
    check_disk(&root.0, &[]);
    assert!(session.is_missing());
    assert!(session.is_dirty() && session.can_undo());
    assert!(session.document().unwrap().serialize() == edited);
    assert!(
        matches!(
            result,
            Err(SessionError::Conflict
                | SessionError::Storage(StorageError::NotFound | StorageError::Conflict))
        ),
        "expected missing or conflict, got {result:?}"
    );
}

impl TempDir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("envy-t067-{}-{serial}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        Self(root)
    }
}

fn resource(path: &Path) -> ResourceRef {
    ResourceRef::from_uri(gio::File::for_path(path).uri().as_str())
}

fn open(path: &Path) -> DocumentSession {
    DocumentSession::open(Arc::new(GioStorage), resource(path)).unwrap()
}

fn original() -> String {
    format!("{SENTINEL}={SENTINEL}\n")
}

fn edit(session: &mut DocumentSession, key: &str) -> Vec<u8> {
    session
        .apply(EditRequest::InsertAssignment {
            key: Redacted::new(key),
            value: Redacted::new(SENTINEL),
            policy: ConversionPolicy::Reject,
        })
        .unwrap();
    session.document().unwrap().serialize()
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn check_disk(directory: &Path, files: &[(&Path, &[u8])]) {
    for (path, expected) in files {
        assert!(
            fs::read(path).unwrap() == *expected,
            "unexpected bytes on disk"
        );
    }
    let paths = files.iter().map(|(path, _)| *path);
    check_entries(directory, paths);
}

fn check_entries<'a>(directory: &Path, paths: impl IntoIterator<Item = &'a Path>) {
    let expected: std::collections::BTreeSet<_> = paths
        .into_iter()
        .map(|path| path.file_name().unwrap().to_owned())
        .collect();
    let actual: std::collections::BTreeSet<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(actual == expected, "unexpected directory entries");
}

fn check_saved(
    result: Result<SaveOutcome, SessionError>,
    session: &DocumentSession,
    directory: &Path,
    files: &[(&Path, &[u8])],
) {
    check_disk(directory, files);
    assert_eq!(result, Ok(SaveOutcome::Saved));
    assert!(!session.is_dirty() && !session.is_conflicted());
}

#[test]
fn chained_saves_preserve_mode_and_replace_inode() {
    for mode in [0o600, 0o644] {
        let root = TempDir::new();
        let path = root.0.join(SENTINEL);
        fs::write(&path, original()).unwrap();
        set_mode(&path, mode);
        let mut session = open(&path);
        for key in ["LOCAL", "SECOND"] {
            let before = fs::metadata(&path).unwrap();
            let edited = edit(&mut session, key);
            check_saved(session.save(), &session, &root.0, &[(&path, &edited)]);
            let after = fs::metadata(&path).unwrap();
            assert_eq!(after.mode() & 0o777, mode);
            assert_ne!(before.ino(), after.ino());
        }
    }
}

#[test]
fn read_only_file_and_parent_refuse_save() {
    let root = TempDir::new();
    let path = root.0.join(SENTINEL);
    let bytes = original();
    fs::write(&path, &bytes).unwrap();
    let mut session = open(&path);
    edit(&mut session, "LOCAL");
    set_mode(&path, 0o400);
    let result = session.save();
    check_disk(&root.0, &[(&path, bytes.as_bytes())]);
    assert_eq!(result, Err(SessionError::Storage(StorageError::ReadOnly)));

    set_mode(&path, 0o600);
    set_mode(&root.0, 0o555);
    let result = session.save();
    set_mode(&root.0, 0o700);
    check_disk(&root.0, &[(&path, bytes.as_bytes())]);
    // GioStorage reports no atomic replace when the parent cannot create entries.
    assert_eq!(
        result,
        Err(SessionError::SaveAsRequired {
            reason: StorageError::AtomicReplaceUnsupported,
        })
    );
}

#[test]
fn symlink_document_and_dangling_save_as_are_refused() {
    let root = TempDir::new();
    let path = root.0.join(SENTINEL);
    let link = path.with_extension("link");
    let missing = path.with_extension("missing");
    let bytes = original();
    fs::write(&path, &bytes).unwrap();
    symlink(&path, &link).unwrap();
    assert!(matches!(
        DocumentSession::open(Arc::new(GioStorage), resource(&link)),
        Err(SessionError::Storage(StorageError::Symlink))
    ));
    fs::remove_file(&link).unwrap();
    symlink(&missing, &link).unwrap();
    let mut session = open(&path);
    edit(&mut session, "LOCAL");
    assert_eq!(
        session.save_as(resource(&link), SaveAsTarget::New),
        Err(SessionError::Storage(StorageError::Symlink))
    );
    assert!(fs::symlink_metadata(&missing).is_err());
    assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
    check_entries(&root.0, [path.as_path(), link.as_path()]);
    fs::remove_file(&link).unwrap();
    check_disk(&root.0, &[(&path, bytes.as_bytes())]);
}

#[test]
fn unobserved_disk_changes_require_review_before_overwrite() {
    let initial = original();
    let external = format!("{SENTINEL}={}\n", SENTINEL.to_ascii_lowercase());
    for change in ["content", "chmod", "touch", "restored mtime", "rename"] {
        let root = TempDir::new();
        let path = root.0.join(SENTINEL);
        fs::write(&path, &initial).unwrap();
        set_mode(&path, 0o600);
        let mut session = open(&path);
        let edited = edit(&mut session, "LOCAL");
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        let expected = match change {
            "chmod" => {
                set_mode(&path, 0o644);
                &initial
            }
            "touch" => {
                fs::File::open(&path)
                    .unwrap()
                    .set_modified(mtime - std::time::Duration::from_secs(2))
                    .unwrap();
                &initial
            }
            "rename" => {
                let replacement = path.with_extension("replacement");
                fs::write(&replacement, &initial).unwrap();
                fs::rename(replacement, &path).unwrap();
                &initial
            }
            _ => {
                fs::write(&path, &external).unwrap();
                if change == "restored mtime" {
                    fs::File::open(&path).unwrap().set_modified(mtime).unwrap();
                }
                &external
            }
        };
        let outcome = session.save();
        check_disk(&root.0, &[(&path, expected.as_bytes())]);
        assert_eq!(
            outcome,
            Err(SessionError::Storage(StorageError::Conflict)),
            "{change}"
        );
        assert!(session.is_dirty() && session.can_undo() && session.is_conflicted());
        assert!(session.document().unwrap().serialize() == edited);
        let reviewed = session.external_snapshot().unwrap().version.clone();
        check_saved(
            session.overwrite_confirmed(&reviewed),
            &session,
            &root.0,
            &[(&path, &edited)],
        );
    }
}

#[test]
fn save_as_checks_aliases_and_switches_only_after_publication() {
    let root = TempDir::new();
    let path = root.0.join(SENTINEL);
    let new_path = path.with_extension("new");
    let existing = path.with_extension("existing");
    let hardlink = path.with_extension("hardlink");
    let parent_alias = path.with_extension("directory-link");
    let bytes = original();
    fs::write(&path, &bytes).unwrap();
    fs::write(&existing, &bytes).unwrap();
    let mut session = open(&path);
    let edited = edit(&mut session, "LOCAL");
    let changed = format!("{SENTINEL}=EXTERNAL\n");
    fs::write(&path, &changed).unwrap();
    assert_eq!(
        session.save(),
        Err(SessionError::Storage(StorageError::Conflict))
    );
    fs::hard_link(&path, &hardlink).unwrap();
    symlink(&root.0, &parent_alias).unwrap();
    for alias in [&hardlink, &parent_alias.join(SENTINEL)] {
        assert_eq!(
            session.save_as(resource(alias), SaveAsTarget::ReplaceExisting),
            Err(SessionError::InvalidSaveAs)
        );
    }
    assert_eq!(
        session.save_as(resource(&existing), SaveAsTarget::New),
        Err(SessionError::Storage(StorageError::AlreadyExists))
    );
    check_entries(
        &root.0,
        [path.as_path(), &existing, &hardlink, &parent_alias],
    );
    assert!(fs::read(&hardlink).unwrap() == changed.as_bytes());
    assert!(fs::symlink_metadata(&parent_alias).unwrap().is_symlink());
    fs::remove_file(&hardlink).unwrap();
    fs::remove_file(&parent_alias).unwrap();
    check_disk(
        &root.0,
        &[(&path, changed.as_bytes()), (&existing, bytes.as_bytes())],
    );
    assert!(session.is_dirty() && session.is_conflicted());
    check_saved(
        session.save_as(resource(&new_path), SaveAsTarget::New),
        &session,
        &root.0,
        &[
            (&path, changed.as_bytes()),
            (&new_path, &edited),
            (&existing, bytes.as_bytes()),
        ],
    );
    assert!(session.resource() == &resource(&new_path));
    let edited_again = edit(&mut session, "SECOND");
    check_saved(
        session.save_as(resource(&existing), SaveAsTarget::ReplaceExisting),
        &session,
        &root.0,
        &[
            (&path, changed.as_bytes()),
            (&new_path, &edited),
            (&existing, &edited_again),
        ],
    );
    assert!(session.resource() == &resource(&existing));
}

#[test]
fn save_as_rejects_alias_of_externally_replaced_original() {
    let root = TempDir::new();
    let path = root.0.join(SENTINEL);
    let replacement = path.with_extension("replacement");
    let alias = path.with_extension("alias");
    let bytes = original();
    fs::write(&path, &bytes).unwrap();
    let mut session = open(&path);
    let edited = edit(&mut session, "LOCAL");

    fs::write(&replacement, &bytes).unwrap();
    fs::rename(&replacement, &path).unwrap();
    fs::hard_link(&path, &alias).unwrap();

    assert_eq!(
        session.save_as(resource(&alias), SaveAsTarget::ReplaceExisting),
        Err(SessionError::InvalidSaveAs)
    );
    assert!(session.resource() == &resource(&path));
    assert!(session.is_dirty() && session.document().unwrap().serialize() == edited);
    check_disk(
        &root.0,
        &[(&path, bytes.as_bytes()), (&alias, bytes.as_bytes())],
    );
}

fn backdate(path: &Path) {
    fs::File::open(path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(11 * 60))
        .unwrap();
}

#[test]
fn open_removes_user_file_with_reserved_staging_name_after_ten_minutes() {
    let root = TempDir::new();
    let path = root.0.join(SENTINEL);
    let reserved = root.0.join(".envy-0123456789abcdef.tmp");
    let bytes = original();
    fs::write(&path, &bytes).unwrap();
    fs::write(&reserved, &bytes).unwrap();
    backdate(&reserved);
    open(&path);
    assert!(!reserved.exists());
    check_disk(&root.0, &[(&path, bytes.as_bytes())]);
}

#[test]
fn ancestor_symlink_loop_refuses_save_and_create_new() {
    let root = TempDir::new();
    let directory = root.0.join("directory");
    fs::create_dir(&directory).unwrap();
    let path = directory.join(SENTINEL);
    let bytes = original();
    fs::write(&path, &bytes).unwrap();
    let mut session = open(&path);
    edit(&mut session, "LOCAL");
    let saved_directory = directory.with_extension("original");
    fs::rename(&directory, &saved_directory).unwrap();
    symlink("directory", &directory).unwrap();
    let destination = directory.join("new");
    assert!(session.save().is_err());
    assert!(matches!(
        GioStorage.create_new(&resource(&destination), bytes.as_bytes()),
        Err(StorageError::Io { kind }) if kind == std::io::Error::from_raw_os_error(libc::ELOOP).kind()
    ));
    fs::remove_file(&directory).unwrap();
    fs::rename(&saved_directory, &directory).unwrap();
    assert!(!destination.exists());
    check_disk(&directory, &[(&path, bytes.as_bytes())]);
    check_entries(&root.0, [directory.as_path()]);
}

const UMASK_CHILD: &str = "ENVY_T067_UMASK";
const CRASH_CHILD: &str = "ENVY_T067_CRASH_DIRECTORY";
const CRASH_BYTES: usize = 8 * 1024 * 1024;

fn child(test: &str) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", test, "--ignored", "--nocapture"]);
    command
}

#[test]
fn save_and_save_as_preserve_modes_under_three_umasks() {
    for mask in ["000", "002", "022"] {
        let marker_dir = TempDir::new();
        let marker = marker_dir.0.join("complete");
        assert!(
            child("umask_child")
                .env(UMASK_CHILD, mask)
                .env("ENVY_T067_COMPLETION", &marker)
                .status()
                .unwrap()
                .success()
        );
        assert!(marker.is_file(), "umask subprocess did not complete");
    }
}

#[test]
#[ignore = "subprocess entry point"]
#[allow(
    unsafe_code,
    reason = "change the umask only in the isolated test process"
)]
fn umask_child() {
    let Ok(mask) = std::env::var(UMASK_CHILD) else {
        return;
    };
    let mask = u32::from_str_radix(&mask, 8).unwrap();
    // SAFETY: This isolated process owns its umask and runs only this row.
    unsafe { libc::umask(mask) };
    let root = TempDir::new();
    let path = root.0.join(SENTINEL);
    let destination = path.with_extension("new");
    let bytes = original();
    fs::write(&path, &bytes).unwrap();
    set_mode(&path, 0o644);
    let mut session = open(&path);
    let edited = edit(&mut session, "LOCAL");
    check_saved(session.save(), &session, &root.0, &[(&path, &edited)]);
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o644);
    check_saved(
        session.save_as(resource(&destination), SaveAsTarget::New),
        &session,
        &root.0,
        &[(&path, &edited), (&destination, &edited)],
    );
    assert_eq!(fs::metadata(destination).unwrap().mode() & 0o777, 0o600);
    fs::write(std::env::var_os("ENVY_T067_COMPLETION").unwrap(), []).unwrap();
}

#[test]
fn supplementary_group_directories_preserve_file_group() {
    let root = TempDir::new();
    let primary = std::process::Command::new("id").arg("-g").output().unwrap();
    assert!(primary.status.success());
    let primary_gid = String::from_utf8_lossy(&primary.stdout)
        .trim()
        .parse::<u32>()
        .unwrap();
    let output = std::process::Command::new("id").arg("-G").output();
    let group = output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .filter_map(|value| value.parse::<u32>().ok())
                .find(|&gid| gid != primary_gid && gid != 65534)
        });
    let gid = group.unwrap_or_else(|| {
        common::skip_or_fail("supplementary-group");
        primary_gid
    });
    for mode in [0o770, 0o750, 0o2770, 0o2750] {
        let directory = root.0.join(format!("{SENTINEL}-{mode:o}"));
        fs::create_dir(&directory).unwrap();
        std::os::unix::fs::chown(&directory, None, Some(gid)).unwrap();
        set_mode(&directory, mode);
        let path = directory.join(SENTINEL);
        let bytes = original();
        fs::write(&path, &bytes).unwrap();
        std::os::unix::fs::chown(&path, None, Some(gid)).unwrap();
        set_mode(&path, 0o640);
        let mut session = open(&path);
        let edited = edit(&mut session, "LOCAL");
        if mode & 0o020 != 0 {
            assert_eq!(
                session.save(),
                Err(SessionError::SaveAsRequired {
                    reason: StorageError::AtomicReplaceUnsupported,
                })
            );
            check_disk(&directory, &[(&path, bytes.as_bytes())]);
            continue;
        }
        check_saved(session.save(), &session, &directory, &[(&path, &edited)]);
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(metadata.gid(), gid);
        assert_eq!(metadata.mode() & 0o777, 0o640);
    }
}

#[test]
#[ignore = "subprocess entry point"]
fn crash_child() {
    let Some(directory) = std::env::var_os(CRASH_CHILD) else {
        return;
    };
    let directory = PathBuf::from(directory);
    let destination = resource(&directory.join(SENTINEL));
    let baseline = GioStorage.read(&destination).unwrap().version;
    let content = SENTINEL.as_bytes().repeat(CRASH_BYTES / SENTINEL.len());
    fs::write(directory.join("ready"), []).unwrap();
    GioStorage
        .replace_if_unchanged(&destination, &baseline, &content)
        .unwrap();
}

#[test]
fn sigkill_during_atomic_replace_preserves_complete_bytes() {
    use std::{
        os::unix::process::ExitStatusExt,
        time::{Duration, Instant},
    };

    let old = original();
    let new = SENTINEL.as_bytes().repeat(CRASH_BYTES / SENTINEL.len());
    let random = fs::File::open("/dev/urandom").unwrap();
    let mut delays = [0_u8; 16];
    std::io::Read::read_exact(&mut &random, &mut delays).unwrap();

    let mut killed = 0;
    for delay in delays {
        let root = TempDir::new();
        let path = root.0.join(SENTINEL);
        let ready = root.0.join("ready");
        fs::write(&path, &old).unwrap();
        set_mode(&path, 0o600);
        let mut process = child("crash_child")
            .env(CRASH_CHILD, &root.0)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !ready.exists() && Instant::now() < deadline {
            if process.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_micros(100));
        }
        let reached_write = ready.exists();
        std::thread::sleep(Duration::from_micros(u64::from(delay) * 20));
        let _ = process.kill();
        let status = process.wait().unwrap();
        assert!(reached_write);
        let actual = fs::read(&path).unwrap();
        if status.success() {
            assert!(actual == new, "successful write published different bytes");
        } else {
            assert_eq!(status.signal(), Some(libc::SIGKILL));
            killed += 1;
            assert!(
                actual == old.as_bytes() || actual == new,
                "interrupted write published partial bytes"
            );
        }
        for entry in fs::read_dir(&root.0).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name().to_string_lossy().starts_with(".envy-") {
                assert_eq!(entry.metadata().unwrap().mode() & 0o777, 0o600);
                backdate(&entry.path());
            }
        }
        open(&path);
        check_disk(&root.0, &[(&path, &actual), (&ready, b"")]);
    }
    assert!(killed > 0, "no subprocess was interrupted");
}
