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
    sync::atomic::{AtomicU64, Ordering},
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
        assert!(!capabilities.supports_conditional_replace);
        assert!(!capabilities.supports_atomic_replace);
        assert!(!capabilities.can_preserve_permissions);
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
    assert_eq!(
        result.unwrap_err(),
        StorageError::ConditionalReplaceUnsupported
    );
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
