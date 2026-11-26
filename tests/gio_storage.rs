#![cfg(target_os = "linux")]

mod common;

use std::{
    ffi::OsStr,
    fs::{self, OpenOptions, Permissions},
    io::ErrorKind,
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, PermissionsExt, symlink},
    },
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use envy::{
    infrastructure::gio_storage::{GioStorage, MAX_FILE_SIZE},
    services::storage::{DocumentStorage, ResourceRef, StorageError, content_hash},
};
use gio::prelude::*;

const SENTINEL: &str = "ENVY_TEST_SENTINEL_9F2C";

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!("envy-gio-{}-{id}", std::process::id()));
            match fs::create_dir(&root) {
                Ok(()) => return Self(root),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(_) => panic!("cannot create test directory"),
            }
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn listing(&self) -> Vec<std::ffi::OsString> {
        let mut entries: Vec<_> = fs::read_dir(&self.0)
            .unwrap_or_else(|_| panic!("cannot inspect test directory"))
            .map(|entry| {
                entry
                    .unwrap_or_else(|_| panic!("cannot inspect test directory"))
                    .file_name()
            })
            .collect();
        entries.sort();
        entries
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn resource(path: &Path) -> ResourceRef {
    ResourceRef::from_uri(gio::File::for_path(path).uri().as_str())
}

fn set_mode(path: &Path, mode: u32) {
    assert!(fs::set_permissions(path, Permissions::from_mode(mode)).is_ok());
}

#[test]
fn reads_regular_file_with_version_metadata_and_capabilities() {
    let dir = TempDir::new();
    set_mode(&dir.0, 0o1700);
    let path = dir.path(SENTINEL);
    fs::write(&path, SENTINEL).unwrap();
    for mode in [0o600, 0o400] {
        set_mode(&path, mode);
        let resource = resource(&path);
        let snapshot = GioStorage.read(&resource).unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let identity = snapshot.version.identity.as_ref().unwrap();
        assert!(snapshot.bytes.expose() == SENTINEL.as_bytes());
        assert!(snapshot.version.hash.expose() == &content_hash(SENTINEL.as_bytes()));
        assert_eq!(snapshot.version.size, metadata.len());
        assert_eq!(snapshot.version.mtime, metadata.modified().ok());
        assert!(snapshot.version.etag.is_some());
        assert!(!snapshot.version.etag.as_ref().unwrap().expose().is_empty());
        assert_eq!(
            identity.device_and_inode,
            Some((metadata.dev(), metadata.ino()))
        );
        assert_eq!(identity.uid, Some(metadata.uid()));
        assert_eq!(identity.gid, Some(metadata.gid()));
        assert_eq!(identity.mode, Some(mode));
        assert_eq!(identity.nlink, Some(1));
        assert_eq!(identity.is_symlink, Some(false));
        let parent = identity.parent.as_ref().unwrap();
        assert_eq!(parent.uid, Some(fs::metadata(&dir.0).unwrap().uid()));
        assert_eq!(parent.mode, Some(0o1700));
        assert_eq!(parent.is_sticky, Some(true));
        assert_eq!(parent.has_default_acl, Some(false));
        assert_eq!(identity.has_access_acl, Some(false));
        if Path::new("/sys/fs/selinux/enforce").exists() {
            assert!(identity.selinux_label.as_ref().is_some_and(|label| {
                label
                    .as_ref()
                    .is_some_and(|label| !label.expose().is_empty())
            }));
        } else {
            assert_eq!(identity.selinux_label, Some(None));
        }
        let capabilities = GioStorage.capabilities(&resource);
        assert_eq!(capabilities.is_writable, mode == 0o600);
        assert_eq!(capabilities.supports_conditional_replace, mode == 0o600);
        assert_eq!(capabilities.supports_atomic_replace, mode == 0o600);
        assert_eq!(capabilities.can_preserve_permissions, mode == 0o600);
        assert!(!format!("{resource:?} {snapshot:?}").contains(SENTINEL));
    }
}

fn assert_unchanged_error(dir: &TempDir, path: &Path, expected: StorageError) {
    let entries = dir.listing();
    // Record all node mtimes, and all regular-file bytes when readable.
    let state = || {
        dir.listing()
            .into_iter()
            .map(|name| {
                let path = dir.0.join(name);
                let metadata = fs::symlink_metadata(&path)
                    .unwrap_or_else(|_| panic!("cannot inspect test metadata"));
                let bytes = metadata.is_file().then(|| fs::read(path).ok()).flatten();
                (
                    metadata
                        .modified()
                        .unwrap_or_else(|_| panic!("cannot inspect test metadata")),
                    bytes,
                )
            })
            .collect::<Vec<_>>()
    };
    let parent_mtime = fs::metadata(&dir.0)
        .unwrap_or_else(|_| panic!("cannot inspect test metadata"))
        .modified()
        .unwrap_or_else(|_| panic!("cannot inspect test metadata"));
    let before = state();
    let Err(error) = GioStorage.read(&resource(path)) else {
        panic!("expected the read request to fail");
    };
    assert_eq!(error, expected);
    assert!(!format!("{error:?} {error}").contains(SENTINEL));
    assert!(entries == dir.listing());
    assert!(before == state());
    assert_eq!(
        parent_mtime,
        fs::metadata(&dir.0)
            .unwrap_or_else(|_| panic!("cannot inspect test metadata"))
            .modified()
            .unwrap_or_else(|_| panic!("cannot inspect test metadata"))
    );
}

#[test]
fn rejects_missing_directory_fifo_symlink_and_oversize_without_mutation() {
    let dir = TempDir::new();
    let target = dir.path(SENTINEL);
    fs::write(&target, SENTINEL).unwrap();

    assert_unchanged_error(&dir, &dir.path("missing"), StorageError::NotFound);
    let directory = dir.path("directory");
    fs::create_dir(&directory).unwrap();
    assert_unchanged_error(&dir, &directory, StorageError::NotRegularFile);

    let fifo = dir.path("fifo");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    assert_unchanged_error(&dir, &fifo, StorageError::NotRegularFile);

    let link = dir.path("link");
    symlink(&target, &link).unwrap();
    assert_unchanged_error(&dir, &link, StorageError::Symlink);
    assert!(fs::read_link(link).unwrap() == target);

    let oversized = dir.path("oversized");
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&oversized)
        .unwrap();
    file.set_len(MAX_FILE_SIZE + 1).unwrap();
    assert_unchanged_error(&dir, &oversized, StorageError::TooLarge);
}

#[test]
fn unreadable_file_is_rejected_without_mutation() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    fs::write(&path, SENTINEL).unwrap();
    if fs::metadata(&path).unwrap().uid() == 0 {
        common::skip_or_fail("unprivileged-user");
        return;
    }
    set_mode(&path, 0);
    assert_unchanged_error(&dir, &path, StorageError::PermissionDenied);
    set_mode(&path, 0o600);
    assert!(fs::read(&path).unwrap() == SENTINEL.as_bytes());
}

#[test]
fn reads_hardlink_and_rejects_replacement_without_mutation() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    let alias = dir.path("alias");
    fs::write(&path, SENTINEL).unwrap();
    fs::hard_link(&path, &alias).unwrap();
    let resource = resource(&path);
    let before = GioStorage.read(&resource).unwrap();
    assert_eq!(before.version.identity.as_ref().unwrap().nlink, Some(2));

    let entries = dir.listing();
    let result = GioStorage.replace_if_unchanged(&resource, &before.version, b"new");
    assert_eq!(result.unwrap_err(), StorageError::AtomicReplaceUnsupported);
    assert!(GioStorage.read(&resource).unwrap() == before);
    assert!(fs::read(alias).unwrap() == SENTINEL.as_bytes());
    assert!(dir.listing() == entries);
}

#[test]
fn detects_access_and_default_acls() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    fs::write(&path, SENTINEL).unwrap();
    let owner = fs::metadata(&path).unwrap().uid();
    let entry = format!("u:{owner}:r");
    for (flag, target) in [("-m", &path), ("-dm", &dir.0)] {
        if !Command::new("setfacl")
            .args([flag, &entry])
            .arg(target)
            .output()
            .is_ok_and(|output| output.status.success())
        {
            common::skip_or_fail("acl");
            return;
        }
    }
    let snapshot = GioStorage.read(&resource(&path)).unwrap();
    let identity = snapshot.version.identity.unwrap();
    assert_eq!(identity.has_access_acl, Some(true));
    assert_eq!(identity.parent.unwrap().has_default_acl, Some(true));
}

#[test]
fn accepts_only_local_file_uri_authorities() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    fs::write(&path, SENTINEL).unwrap();
    let uri = gio::File::for_path(&path).uri();
    let suffix = uri.strip_prefix("file://").unwrap();
    for host in ["localhost", "LOCALHOST"] {
        let local = ResourceRef::from_uri(format!("file://{host}{suffix}"));
        let snapshot = GioStorage.read(&local).unwrap();
        assert!(snapshot.bytes.expose() == SENTINEL.as_bytes());
        assert!(!format!("{local:?} {snapshot:?}").contains(SENTINEL));
    }
    let remote = ResourceRef::from_uri(format!("file://otherhost{suffix}"));
    let error = GioStorage.read(&remote).unwrap_err();
    assert_eq!(
        error,
        StorageError::Io {
            kind: ErrorKind::InvalidInput
        }
    );
    assert!(!format!("{remote:?} {error:?} {error}").contains(SENTINEL));
}

#[test]
fn reads_non_utf8_local_filenames() {
    let dir = TempDir::new();
    let path = dir.0.join(OsStr::from_bytes(b"file-\xff"));
    fs::write(&path, SENTINEL).unwrap();
    let snapshot = GioStorage.read(&resource(&path)).unwrap();
    assert!(snapshot.bytes.expose() == SENTINEL.as_bytes());
}

const GIO_COMPLETION: &str = "ENVY_GIO_COMPLETION";

fn gio_child(test: &str) -> Command {
    let executable = std::env::current_exe();
    assert!(executable.is_ok(), "test executable is unavailable");
    let mut command = Command::new(executable.unwrap_or_default());
    command.args(["--exact", test, "--ignored"]);
    command
}

fn run_gio_child(command: &mut Command) {
    let marker_dir = TempDir::new();
    let marker = marker_dir.path("complete");
    let status = command
        .env(GIO_COMPLETION, &marker)
        .output()
        .map(|o| o.status);
    assert!(
        status.is_ok_and(|status| status.success()),
        "test subprocess failed"
    );
    assert!(marker.is_file(), "test subprocess did not complete");
}

fn complete_gio_child() {
    let marker = std::env::var_os(GIO_COMPLETION);
    assert!(marker.is_some(), "completion marker is not configured");
    assert!(fs::write(marker.unwrap_or_default(), []).is_ok());
}

#[test]
fn repeated_reads_do_not_leak_descriptors() {
    run_gio_child(&mut gio_child("fd_leak_child"));
}

#[test]
#[ignore = "subprocess entry point"]
fn fd_leak_child() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    fs::write(&path, SENTINEL).unwrap();
    let resource = resource(&path);
    let count = || fs::read_dir("/proc/self/fd").unwrap().count();
    let before = count();
    for _ in 0..200 {
        GioStorage.read(&resource).unwrap();
    }
    assert_eq!(count(), before);
    complete_gio_child();
}

#[test]
fn replaces_with_verified_hash_and_new_inode_in_sticky_directory() {
    for mode in [0o600, 0o640, 0o644] {
        let dir = TempDir::new();
        set_mode(&dir.0, 0o1777);
        let path = dir.path(SENTINEL);
        fs::write(&path, b"old").unwrap();
        set_mode(&path, mode);
        let resource = resource(&path);
        let base = GioStorage.read(&resource).unwrap().version;
        let saved = GioStorage
            .replace_if_unchanged(&resource, &base, SENTINEL.as_bytes())
            .unwrap();
        let identity = saved.identity.as_ref().unwrap();
        assert_ne!(
            identity.device_and_inode,
            base.identity.unwrap().device_and_inode
        );
        assert_eq!(identity.mode, Some(mode));
        assert!(saved.hash.expose() == &content_hash(SENTINEL.as_bytes()));
        let readback = GioStorage.read(&resource).unwrap();
        assert!(readback.version == saved);
        assert!(readback.bytes.expose() == SENTINEL.as_bytes());
        assert!(dir.listing() == [SENTINEL]);
        assert!(!format!("{saved:?}").contains(SENTINEL));
        let again = GioStorage
            .replace_if_unchanged(&resource, &saved, b"again")
            .unwrap();
        assert!(GioStorage.read(&resource).unwrap().version == again);
        assert!(dir.listing() == [SENTINEL]);
    }
}

#[test]
fn changed_deleted_and_symlink_destinations_are_never_written() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    let target = dir.path("target");
    let destination = resource(&path);
    fs::write(&path, b"old").unwrap();
    set_mode(&path, 0o600);
    let base = GioStorage.read(&destination).unwrap().version;
    let replace = |error| {
        let actual = GioStorage
            .replace_if_unchanged(&destination, &base, SENTINEL.as_bytes())
            .unwrap_err();
        assert_eq!(actual, error);
        assert!(!format!("{actual:?} {actual}").contains(SENTINEL));
    };
    set_mode(&path, 0o644);
    replace(StorageError::Conflict);
    assert!(fs::read(&path).unwrap() == b"old");
    fs::write(&path, b"external").unwrap();
    replace(StorageError::Conflict);
    assert!(fs::read(&path).unwrap() == b"external");
    fs::remove_file(&path).unwrap();
    replace(StorageError::Conflict);
    assert!(dir.listing().is_empty());

    fs::write(&target, b"target").unwrap();
    symlink(&target, &path).unwrap();
    replace(StorageError::Symlink);
    assert!(fs::read_link(path).unwrap() == target);
    assert!(fs::read(target).unwrap() == b"target");
}

#[test]
fn unsafe_directories_and_read_only_files_block_save() {
    let dir = TempDir::new();
    let parent = dir.path("parent");
    let alias = dir.path("alias");
    fs::create_dir(&parent).unwrap();
    symlink(&parent, &alias).unwrap();
    let path = parent.join(SENTINEL);
    let destination = resource(&path);
    fs::write(&path, b"old").unwrap();
    set_mode(&path, 0o600);
    let base = GioStorage.read(&destination).unwrap().version;
    for (mode, error) in [
        (0o555, StorageError::AtomicReplaceUnsupported),
        (0o770, StorageError::PermissionsNotPreserved),
        (0o777, StorageError::PermissionsNotPreserved),
    ] {
        set_mode(&parent, mode);
        assert_eq!(
            GioStorage.replace_if_unchanged(&destination, &base, b"new"),
            Err(error)
        );
        let capabilities = GioStorage.capabilities(&destination);
        assert!(capabilities.is_writable);
        assert!(capabilities.supports_conditional_replace);
        assert!(!capabilities.supports_atomic_replace);
        assert!(!capabilities.can_preserve_permissions);
    }
    set_mode(&parent, 0o700);
    assert_eq!(
        GioStorage.replace_if_unchanged(&resource(&alias.join(SENTINEL)), &base, b"new"),
        Err(StorageError::PermissionsNotPreserved)
    );
    set_mode(&path, 0o400);
    assert_eq!(
        GioStorage.replace_if_unchanged(&destination, &base, b"new"),
        Err(StorageError::ReadOnly)
    );
    assert!(fs::read(path).unwrap() == b"old");
}

#[test]
fn refuses_access_and_default_acl_before_publication() {
    for is_default in [false, true] {
        let dir = TempDir::new();
        let path = dir.path(SENTINEL);
        fs::write(&path, b"old").unwrap();
        set_mode(&path, 0o640);
        let owner = fs::metadata(&path).unwrap().uid();
        let mut command = Command::new("setfacl");
        if is_default {
            command.arg("-d");
        }
        let acl = command
            .args(["-m", &format!("u:{owner}:r")])
            .arg(if is_default { &dir.0 } else { &path })
            .output();
        if !acl.is_ok_and(|output| output.status.success()) {
            common::skip_or_fail("acl");
            continue;
        }
        let destination = resource(&path);
        let before = GioStorage.read(&destination).unwrap();
        let identity = before.version.identity.as_ref().unwrap();
        assert_eq!(identity.has_access_acl, Some(!is_default));
        assert_eq!(
            identity.parent.as_ref().unwrap().has_default_acl,
            Some(is_default)
        );
        let directory_mtime = fs::metadata(&dir.0).unwrap().modified().unwrap();
        assert_eq!(
            GioStorage.replace_if_unchanged(&destination, &before.version, b"new"),
            Err(StorageError::PermissionsNotPreserved)
        );
        assert!(GioStorage.read(&destination).unwrap() == before);
        assert!(dir.listing() == [SENTINEL]);
        assert_eq!(
            fs::metadata(&dir.0).unwrap().modified().unwrap(),
            directory_mtime
        );
    }
}

#[test]
fn partial_write_preserves_original_and_removes_temporary_file() {
    run_gio_child(&mut gio_child("partial_write_child"));
}

#[test]
#[ignore = "subprocess entry point"]
fn partial_write_child() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    fs::write(&path, b"old").unwrap();
    let destination = resource(&path);
    let before = GioStorage.read(&destination).unwrap();
    limit_child_file_size();
    let bytes = SENTINEL.repeat(8192);
    let error = GioStorage
        .replace_if_unchanged(&destination, &before.version, bytes.as_bytes())
        .unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }));
    assert!(!format!("{error:?} {error}").contains(SENTINEL));
    assert!(GioStorage.read(&destination).unwrap() == before);
    assert!(dir.listing() == [SENTINEL]);
    complete_gio_child();
}

#[allow(
    unsafe_code,
    reason = "inject a partial file write in an isolated test process"
)]
fn limit_child_file_size() {
    let limit = libc::rlimit {
        rlim_cur: 4096,
        rlim_max: 4096,
    };
    // SAFETY: SIG_IGN is valid for SIGXFSZ, and limit points to initialized
    // storage. The child runs only the write-failure test.
    unsafe {
        assert_ne!(libc::signal(libc::SIGXFSZ, libc::SIG_IGN), libc::SIG_ERR);
        assert_eq!(libc::setrlimit(libc::RLIMIT_FSIZE, &limit), 0);
    }
}

#[test]
fn refuses_custom_selinux_label_before_publication() {
    if !Path::new("/sys/fs/selinux/enforce").exists() {
        common::skip_or_fail("selinux");
        return;
    }
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    fs::write(&path, b"old").unwrap();
    set_mode(&path, 0o600);
    let destination = resource(&path);
    let original = GioStorage.read(&destination).unwrap().version;
    let relabel = Command::new("chcon")
        .args(["-t", "ssh_home_t"])
        .arg(&path)
        .output();
    if !relabel.is_ok_and(|output| output.status.success()) {
        common::skip_or_fail("selinux");
        return;
    }
    let base = GioStorage.read(&destination).unwrap().version;
    let label =
        |version: envy::services::storage::DiskVersion| version.identity.unwrap().selinux_label;
    assert!(label(base.clone()) != label(original.clone()));
    assert_eq!(
        GioStorage.replace_if_unchanged(&destination, &base, b"new"),
        Err(StorageError::PermissionsNotPreserved)
    );
    let after = GioStorage.read(&destination).unwrap();
    assert!(after.bytes.expose() == b"old");
    assert!(after.version == base);
    assert!(dir.listing() == [SENTINEL]);
}

const RACE_CHILD: &str = "ENVY_GIO_RACE_TEST_CHILD";
const RACE_ATTEMPTS: usize = 200;
const LONG_WINDOW_ATTEMPTS: usize = 40;

#[test]
fn permission_race_preserves_complete_file_with_and_without_write_limit() {
    for mode in ["normal", "limited"] {
        run_gio_child(gio_child("permission_race_child").env(RACE_CHILD, mode));
    }
}

#[test]
#[ignore = "subprocess entry point"]
fn permission_race_child() {
    let mode = std::env::var_os(RACE_CHILD).unwrap();
    if mode == "normal" {
        clear_child_umask();
        replaces_with_verified_hash_and_new_inode_in_sticky_directory();
    }
    run_permission_race(mode == "limited");
    complete_gio_child();
}

#[allow(
    clippy::unwrap_used,
    reason = "test fixture setup should fail immediately"
)]
fn run_permission_race(is_limited: bool) {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    let destination = resource(&path);
    let old = b"old";
    let new = vec![b'n'; 128 * 1024];
    if is_limited {
        limit_child_file_size();
    }
    let stop = AtomicBool::new(false);
    let (mut successes, mut write_errors, mut refusals, mut post_failures) = (0, 0, 0, 0);
    let mut size_limit_errors = 0;
    let attempts = RACE_ATTEMPTS + LONG_WINDOW_ATTEMPTS;
    for attempt in 0..attempts {
        // Keep the short races and also exercise a writable window long
        // enough for descriptor checks, fsync, and verification on slower hosts.
        let writable_window = if attempt < RACE_ATTEMPTS {
            std::time::Duration::from_micros(40)
        } else {
            std::time::Duration::from_millis(2)
        };
        set_mode(&dir.0, 0o700);
        fs::write(&path, old).unwrap();
        set_mode(&path, 0o600);
        let base = GioStorage.read(&destination).unwrap().version;
        stop.store(false, Ordering::Relaxed);
        let result = std::thread::scope(|scope| {
            let (ready, start) = std::sync::mpsc::sync_channel(0);
            let (stop_flag, parent) = (&stop, &dir.0);
            scope.spawn(move || {
                ready.send(()).unwrap();
                while !stop_flag.load(Ordering::Relaxed) {
                    set_mode(parent, 0o700);
                    std::thread::sleep(writable_window);
                    set_mode(parent, 0o555);
                    std::thread::sleep(std::time::Duration::from_micros(40));
                }
                set_mode(parent, 0o700);
            });
            start.recv().unwrap();
            let result = GioStorage.replace_if_unchanged(&destination, &base, &new);
            stop.store(true, Ordering::Relaxed);
            result
        });
        let bytes = fs::read(&path).unwrap();
        assert!(bytes == old || bytes == new);
        match result {
            Ok(_) => {
                successes += 1;
                assert!(bytes == new);
            }
            Err(StorageError::Io { kind }) => {
                write_errors += 1;
                if kind == ErrorKind::FileTooLarge {
                    size_limit_errors += 1;
                }
                assert!(bytes == old);
            }
            Err(StorageError::PostWriteVerificationFailed { .. }) => {
                post_failures += 1;
                assert!(!is_limited);
                assert!(bytes == new);
            }
            Err(error) => {
                refusals += 1;
                assert!(matches!(
                    error,
                    StorageError::AtomicReplaceUnsupported
                        | StorageError::PermissionsNotPreserved
                        | StorageError::Conflict
                ));
                assert!(bytes == old);
            }
        }
        assert!(dir.listing() == [SENTINEL]);
    }
    if is_limited {
        assert_eq!(successes, 0);
        assert!(
            size_limit_errors > 0,
            "expected at least one file-size limit failure"
        );
    } else {
        assert!(successes > 0, "expected at least one verified save");
    }
    eprintln!(
        "permission race (limited={is_limited}): {attempts} attempts, {successes} successful, {write_errors} I/O failures ({size_limit_errors} file-size limits), {refusals} refusals, {post_failures} post-rename failures, 0 corruptions or leftover temp files"
    );
}

#[test]
fn preserves_supplementary_group() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    fs::write(&path, b"old").unwrap();
    let primary_gid = fs::metadata(&path).unwrap().gid();
    let Ok(output) = Command::new("id").arg("-G").output() else {
        common::skip_or_fail("supplementary-group");
        return;
    };
    let Some(gid) = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|value| value.parse::<u32>().ok())
        .find(|&gid| gid != primary_gid && gid != 65534)
    else {
        common::skip_or_fail("supplementary-group");
        return;
    };
    std::os::unix::fs::chown(&path, None, Some(gid)).unwrap();
    set_mode(&path, 0o640);
    let destination = resource(&path);
    let base = GioStorage.read(&destination).unwrap().version;
    let saved = GioStorage
        .replace_if_unchanged(&destination, &base, b"new")
        .unwrap();
    let identity = saved.identity.unwrap();
    assert_eq!((identity.mode, identity.gid), (Some(0o640), Some(gid)));
    assert_eq!(dir.listing(), [SENTINEL]);
}

#[allow(
    unsafe_code,
    reason = "exercise save under umask 000 in an isolated subprocess"
)]
fn clear_child_umask() {
    // SAFETY: Zero is a valid umask and this subprocess owns its process state.
    unsafe { libc::umask(0) };
}

#[test]
fn create_new_is_private_and_cleans_staging_under_both_umasks() {
    for mask in ["000", "077"] {
        run_gio_child(
            Command::new("sh")
                .args([
                    "-c",
                    "umask \"$1\" && exec \"$2\" --exact create_new_umask_child --ignored",
                    "sh",
                    mask,
                ])
                .arg(std::env::current_exe().unwrap()),
        );
    }
}

#[test]
#[ignore = "subprocess entry point"]
fn create_new_umask_child() {
    let dir = TempDir::new();
    set_mode(&dir.0, 0o700);
    let path = dir.path(SENTINEL);
    GioStorage
        .create_new(&resource(&path), SENTINEL.as_bytes())
        .unwrap();
    assert!(fs::read(&path).unwrap() == SENTINEL.as_bytes());
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o7777, 0o600);
    assert!(dir.listing() == [OsStr::new(SENTINEL)]);
    complete_gio_child();
}

fn assert_create_error(
    result: Result<envy::services::storage::DiskVersion, StorageError>,
    expected: StorageError,
) {
    let Err(error) = result else {
        panic!("creation unexpectedly succeeded");
    };
    assert_eq!(error, expected);
    assert!(!format!("{error:?} {error}").contains(SENTINEL));
}

#[test]
fn create_new_refuses_existing_entries_oversize_and_unsafe_parent() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    let destination = resource(&path);
    fs::write(&path, SENTINEL).unwrap();
    assert_create_error(
        GioStorage.create_new(&destination, b""),
        StorageError::AlreadyExists,
    );
    assert!(fs::read(&path).unwrap() == SENTINEL.as_bytes());
    fs::remove_file(&path).unwrap();
    let target = dir.path("missing");
    symlink(&target, &path).unwrap();
    assert_create_error(
        GioStorage.create_new(&destination, SENTINEL.as_bytes()),
        StorageError::Symlink,
    );
    assert!(!target.exists());
    fs::remove_file(&path).unwrap();
    let oversized = vec![0; MAX_FILE_SIZE as usize + 1];
    assert_create_error(
        GioStorage.create_new(&destination, &oversized),
        StorageError::TooLarge,
    );
    set_mode(&dir.0, 0o777);
    assert_create_error(
        GioStorage.create_new(&destination, SENTINEL.as_bytes()),
        StorageError::PermissionsNotPreserved,
    );
    assert!(dir.listing().is_empty());
}

#[test]
fn create_new_refuses_default_acl() {
    let dir = TempDir::new();
    if !Command::new("setfacl")
        .args(["-m", "d:u:12345:rwx"])
        .arg(&dir.0)
        .output()
        .is_ok_and(|output| output.status.success())
    {
        common::skip_or_fail("acl");
        return;
    }
    assert_create_error(
        GioStorage.create_new(&resource(&dir.path(SENTINEL)), SENTINEL.as_bytes()),
        StorageError::PermissionsNotPreserved,
    );
    assert!(dir.listing().is_empty());
}

#[test]
fn create_new_never_overwrites_a_racing_creator() {
    let dir = TempDir::new();
    let ours = SENTINEL.repeat(8192);
    for attempt in 0..12 {
        let path = dir.path(&format!("file-{attempt}"));
        let destination = resource(&path);
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let competitor = scope.spawn(|| {
                barrier.wait();
                if attempt % 2 == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                use std::io::Write;
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                file.write_all(SENTINEL.as_bytes())
            });
            barrier.wait();
            let result = GioStorage.create_new(&destination, ours.as_bytes());
            match competitor.join().unwrap() {
                Ok(()) => {
                    assert_create_error(result, StorageError::AlreadyExists);
                    assert!(fs::read(&path).unwrap() == SENTINEL.as_bytes());
                }
                Err(error) => {
                    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
                    assert!(result.is_ok());
                    assert!(fs::read(&path).unwrap() == ours.as_bytes());
                }
            }
        });
    }
    assert_eq!(dir.listing().len(), 12);
}

#[test]
fn stale_staging_cleanup_only_removes_old_single_link_regular_files() {
    let dir = TempDir::new();
    let document = dir.path(SENTINEL);
    let old = dir.path(".envy-0123456789abcdef.tmp");
    let young = dir.path(".envy-1123456789abcdef.tmp");
    let link = dir.path(".envy-2123456789abcdef.tmp");
    let linked = dir.path(".envy-3123456789abcdef.tmp");
    let other = dir.path(".envy-0123456789abcdeF.tmp");
    for path in [&document, &old, &young, &linked, &other] {
        fs::write(path, SENTINEL).unwrap();
    }
    for path in [&document, &old, &linked, &other] {
        fs::File::open(path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH)
            .unwrap();
    }
    fs::hard_link(&linked, dir.path("other-link")).unwrap();
    symlink(&document, &link).unwrap();
    GioStorage.remove_stale_staging(&resource(&document));
    assert!(!old.exists());
    for path in [&document, &young, &linked, &other] {
        assert!(fs::read(path).unwrap() == SENTINEL.as_bytes());
    }
    assert!(link.symlink_metadata().unwrap().is_symlink());
}

#[test]
fn stale_staging_cleanup_removes_leftover_link_to_the_document_only() {
    let dir = TempDir::new();
    let document = dir.path(SENTINEL);
    let leftover = dir.path(".envy-4123456789abcdef.tmp");
    let foreign = dir.path(".envy-5123456789abcdef.tmp");
    fs::write(&document, SENTINEL).unwrap();
    fs::write(dir.path("unrelated"), SENTINEL).unwrap();
    fs::File::open(&document)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH)
        .unwrap();
    fs::File::open(dir.path("unrelated"))
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH)
        .unwrap();
    fs::hard_link(&document, &leftover).unwrap();
    fs::hard_link(dir.path("unrelated"), &foreign).unwrap();
    GioStorage.remove_stale_staging(&resource(&document));
    assert!(!leftover.exists());
    assert!(foreign.exists());
    assert!(fs::read(&document).unwrap() == SENTINEL.as_bytes());
    assert_eq!(fs::metadata(&document).unwrap().nlink(), 1);
}
