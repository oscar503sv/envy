#![cfg(target_os = "linux")]

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
    let set_access_acl = |entry: &str| {
        Command::new("setfacl")
            .args(["-m", entry])
            .arg(&path)
            .output()
    };
    let entry = match set_access_acl("u:nobody:r") {
        Err(error) if error.kind() == ErrorKind::NotFound => return,
        Ok(output) if output.status.success() => "u:nobody:r".to_owned(),
        result => {
            assert!(result.is_ok());
            // User namespaces can leave nobody unmapped. A named entry for
            // the file owner still creates an extended ACL.
            let owner = fs::metadata(&path).unwrap().uid();
            let entry = format!("u:{owner}:r");
            assert!(set_access_acl(&entry).unwrap().status.success());
            entry
        }
    };
    assert!(
        Command::new("setfacl")
            .args(["-d", "-m", &entry])
            .arg(&dir.0)
            .output()
            .unwrap()
            .status
            .success()
    );
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

#[test]
fn repeated_reads_do_not_leak_descriptors() {
    const CHILD: &str = "ENVY_FD_LEAK_TEST_CHILD";
    // Run this test alone in a separate process so parallel tests cannot
    // change the descriptor count between measurements.
    if std::env::var_os(CHILD).is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "repeated_reads_do_not_leak_descriptors"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        return;
    }
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
            eprintln!("skipping ACL fixture: setfacl or filesystem ACL support unavailable");
            return;
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
    const CHILD: &str = "ENVY_GIO_FSIZE_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "partial_write_preserves_original_and_removes_temporary_file",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        return;
    }

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
        eprintln!("skipping SELinux fixture: SELinux is unavailable");
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
        eprintln!("skipping SELinux fixture: chcon could not set ssh_home_t");
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

#[test]
fn permission_race_preserves_complete_file_with_and_without_write_limit() {
    if let Some(mode) = std::env::var_os(RACE_CHILD) {
        if mode == "normal" {
            clear_child_umask();
            replaces_with_verified_hash_and_new_inode_in_sticky_directory();
        }
        run_permission_race(mode == "limited");
        return;
    }
    for mode in ["normal", "limited"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "permission_race_preserves_complete_file_with_and_without_write_limit",
                "--nocapture",
            ])
            .env(RACE_CHILD, mode)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
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
    for _ in 0..RACE_ATTEMPTS {
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
                    std::thread::sleep(std::time::Duration::from_micros(40));
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
        "permission race (limited={is_limited}): {RACE_ATTEMPTS} attempts, {successes} successful, {write_errors} I/O failures ({size_limit_errors} file-size limits), {refusals} refusals, {post_failures} post-rename failures, 0 corruptions or leftover temp files"
    );
}

#[test]
fn preserves_supplementary_group() {
    let dir = TempDir::new();
    let path = dir.path(SENTINEL);
    fs::write(&path, b"old").unwrap();
    let primary_gid = fs::metadata(&path).unwrap().gid();
    let Ok(output) = Command::new("id").arg("-G").output() else {
        return;
    };
    assert!(output.status.success());
    let Some(gid) = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|value| value.parse::<u32>().ok())
        .find(|&gid| gid != primary_gid && gid != 65534)
    else {
        eprintln!("skipping supplementary-group fixture: no eligible group");
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
