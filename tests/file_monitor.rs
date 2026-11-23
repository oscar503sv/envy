#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test setup must succeed")]

use std::{
    cell::RefCell,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use envy::{
    infrastructure::{
        file_monitor::{CheckTrigger, DEBOUNCE, FileMonitor, POLL_INTERVAL},
        gio_storage::GioStorage,
    },
    services::{
        document_service::{DocumentSession, ExternalChange},
        storage::ResourceRef,
    },
};
use gio::{glib, prelude::*};

const WAIT_LIMIT: Duration = Duration::from_secs(5);
const QUIET_PERIOD: Duration = Duration::from_millis(700);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("envy-monitor-{}-{id}", std::process::id()));
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
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

type Triggers = Rc<RefCell<Vec<CheckTrigger>>>;

fn resource(path: &Path) -> ResourceRef {
    ResourceRef::from_uri(gio::File::for_path(path).uri().as_str())
}

fn watch(path: &Path, should_poll: bool) -> (FileMonitor, Triggers) {
    let triggers = Triggers::default();
    let received = triggers.clone();
    let watcher = FileMonitor::new(&resource(path), should_poll, move |trigger| {
        received.borrow_mut().push(trigger);
    })
    .unwrap();
    (watcher, triggers)
}

fn in_context(test: impl AsyncFnOnce()) {
    let context = glib::MainContext::new();
    context
        .with_thread_default(|| context.block_on(test()))
        .unwrap();
}

async fn expect_trigger(triggers: &Triggers, expected: CheckTrigger) {
    let deadline = Instant::now() + WAIT_LIMIT;
    while triggers.borrow().is_empty() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {expected:?}"
        );
        glib::timeout_future(Duration::from_millis(10)).await;
    }
    assert_eq!(*triggers.take(), [expected]);
}

async fn expect_quiet(triggers: &Triggers) {
    glib::timeout_future(QUIET_PERIOD).await;
    assert!(triggers.borrow().is_empty());
}

#[test]
fn burst_coalesces_with_focus_poll() {
    in_context(async || {
        let dir = TempDir::new();
        let path = dir.path(".env");
        fs::write(&path, "A=0\n").unwrap();
        let (watcher, triggers) = watch(&path, false);

        for value in 1..=12 {
            fs::write(&path, format!("A={value}\n")).unwrap();
        }
        watcher.poll_now();
        expect_trigger(&triggers, CheckTrigger::Monitor).await;
        expect_quiet(&triggers).await;
    });
}

#[test]
fn replacement_twice_and_delete_recreate_are_detected() {
    in_context(async || {
        let dir = TempDir::new();
        let path = dir.path(".env");
        let staging = dir.path("staging");
        fs::write(&path, "A=0\n").unwrap();
        let (_watcher, triggers) = watch(&path, false);

        for contents in ["A=1\n", "A=2\n"] {
            fs::write(&staging, contents).unwrap();
            fs::rename(&staging, &path).unwrap();
            expect_trigger(&triggers, CheckTrigger::Monitor).await;
            expect_quiet(&triggers).await;
        }
        fs::remove_file(&path).unwrap();
        expect_trigger(&triggers, CheckTrigger::Monitor).await;
        expect_quiet(&triggers).await;
        fs::write(&path, "A=3\n").unwrap();
        expect_trigger(&triggers, CheckTrigger::Monitor).await;
    });
}

#[test]
fn equal_confirmed_snapshot_ignores_save_echo() {
    in_context(async || {
        let dir = TempDir::new();
        let path = dir.path(".env");
        fs::write(&path, "A=0\n").unwrap();
        let (_watcher, triggers) = watch(&path, false);

        // Opening from the finished write models the exact snapshot a
        // verified replacement would install as the session baseline.
        fs::write(&path, "A=1\n").unwrap();
        let mut session = DocumentSession::open(Arc::new(GioStorage), resource(&path)).unwrap();
        expect_trigger(&triggers, CheckTrigger::Monitor).await;
        assert_eq!(
            session.refresh_external().unwrap(),
            ExternalChange::Unchanged
        );
        assert!(session.external_snapshot().is_none());

        fs::write(&path, "A=2\n").unwrap();
        expect_trigger(&triggers, CheckTrigger::Monitor).await;
        assert_eq!(
            session.refresh_external().unwrap(),
            ExternalChange::ReloadAvailable
        );
    });
}

#[test]
fn focus_and_timer_poll_and_drop_cancels_pending_work() {
    in_context(async || {
        let dir = TempDir::new();
        let path = dir.path(".env");
        fs::write(&path, "A=0\n").unwrap();
        let mut session = DocumentSession::open(Arc::new(GioStorage), resource(&path)).unwrap();
        let baseline = session.baseline().clone();
        // A write before the watch starts models a host edit whose event
        // never arrives through the document portal.
        fs::write(&path, "A=1\n").unwrap();
        let (watcher, triggers) = watch(&path, true);

        watcher.poll_now();
        watcher.set_polling_paused(true);
        watcher.poll_now();
        expect_quiet(&triggers).await;
        watcher.set_polling_paused(false);

        for _ in 0..10 {
            watcher.poll_now();
        }
        expect_trigger(&triggers, CheckTrigger::Poll).await;
        assert_eq!(
            session.refresh_external().unwrap(),
            ExternalChange::ReloadAvailable
        );
        assert_eq!(session.baseline(), &baseline);
        expect_quiet(&triggers).await;
        expect_trigger(&triggers, CheckTrigger::Poll).await;

        watcher.poll_now();
        drop(watcher);
        fs::write(&path, "A=1\n").unwrap();
        glib::timeout_future(POLL_INTERVAL + DEBOUNCE + QUIET_PERIOD).await;
        assert!(triggers.borrow().is_empty());
    });
}

#[test]
fn continuous_writes_are_checked_during_the_burst() {
    in_context(async || {
        const WRITES: usize = 30;
        let dir = TempDir::new();
        let path = dir.path(".env");
        fs::write(&path, "A=0\n").unwrap();
        let (_watcher, triggers) = watch(&path, false);

        for value in 1..=WRITES {
            fs::write(&path, format!("A={value}\n")).unwrap();
            glib::timeout_future(Duration::from_millis(100)).await;
        }

        let observed = triggers.borrow();
        assert!((2..WRITES / 3).contains(&observed.len()), "{observed:?}");
        assert!(
            observed
                .iter()
                .all(|trigger| *trigger == CheckTrigger::Monitor)
        );
    });
}

#[test]
fn focus_poll_reports_missing_after_parent_rename() {
    in_context(async || {
        let dir = TempDir::new();
        let parent = dir.path("parent");
        fs::create_dir(&parent).unwrap();
        let path = parent.join(".env");
        fs::write(&path, "A=0\n").unwrap();
        let mut session = DocumentSession::open(Arc::new(GioStorage), resource(&path)).unwrap();
        let (watcher, triggers) = watch(&path, false);

        fs::rename(&parent, dir.path("renamed")).unwrap();
        glib::timeout_future(QUIET_PERIOD).await;
        triggers.take();

        watcher.poll_now();
        expect_trigger(&triggers, CheckTrigger::Poll).await;
        assert_eq!(session.refresh_external().unwrap(), ExternalChange::Missing);
    });
}

#[test]
fn only_local_file_uris_are_supported() {
    in_context(async || {
        for uri in [
            "sftp://example.org/.env",
            "trash:///.env",
            "file://example.org/tmp/.env",
            "file://user@localhost/tmp/.env",
            "file://localhost:123/tmp/.env",
        ] {
            assert!(
                FileMonitor::new(&ResourceRef::from_uri(uri), true, |_| {}).is_err(),
                "{uri}"
            );
        }

        let dir = TempDir::new();
        let path = dir.path(".env");
        fs::write(&path, "A=0\n").unwrap();
        let uri = gio::File::for_path(&path).uri();
        let uri = uri.replacen("file://", "file://LOCALHOST", 1);
        assert!(FileMonitor::new(&ResourceRef::from_uri(uri), false, |_| {}).is_ok());
    });
}

#[test]
fn callback_can_poll_again_and_drop_the_monitor() {
    in_context(async || {
        let dir = TempDir::new();
        let path = dir.path(".env");
        fs::write(&path, "A=0\n").unwrap();

        let watcher = Rc::new(RefCell::<Option<FileMonitor>>::new(None));
        let weak_watcher = Rc::downgrade(&watcher);
        let triggers = Triggers::default();
        let received = triggers.clone();
        watcher.replace(Some(
            FileMonitor::new(&resource(&path), false, move |trigger| {
                received.borrow_mut().push(trigger);
                let watcher = weak_watcher.upgrade().unwrap();
                match trigger {
                    CheckTrigger::Monitor => watcher.borrow().as_ref().unwrap().poll_now(),
                    CheckTrigger::Poll => drop(watcher.take()),
                }
            })
            .unwrap(),
        ));

        fs::write(&path, "A=1\n").unwrap();
        expect_trigger(&triggers, CheckTrigger::Monitor).await;
        expect_trigger(&triggers, CheckTrigger::Poll).await;
        assert!(watcher.borrow().is_none());
        fs::write(&path, "A=2\n").unwrap();
        expect_quiet(&triggers).await;
    });
}
