//! Main-context delivery of file verification requests.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use gio::{FileMonitorEvent, FileMonitorFlags, glib, prelude::*};

use crate::services::storage::ResourceRef;

/// Wait for an editor's write and rename events to settle before reading.
pub const DEBOUNCE: Duration = Duration::from_millis(200);
/// Cap the debounce so continuous writes still get checked during the burst.
const MAX_DEBOUNCE_WAIT: Duration = Duration::from_secs(1);
/// Bound detection latency on portal files while limiting periodic reads.
pub const POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckTrigger {
    /// At least one filesystem event arrived in the batch. The caller may
    /// reload a clean session after verifying its snapshot.
    Monitor,
    /// Focus or timer only. Record the observation without reloading or
    /// advancing the session baseline (D-19.6).
    Poll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("file monitoring is unavailable")]
pub struct MonitorUnavailable;

/// One watcher per open resource. Create, use and drop it on the GTK thread
/// with its thread-default main context. Dispatch the callback's work to a
/// worker, serializing session reads and saves. Use
/// `DocumentSession::refresh_external` for either trigger. Apply
/// `reload_if_clean` only for monitor-triggered reloads. A verified own
/// save is recognized by snapshot equality in the session.
/// Call [Self::poll_now] on window focus on both the host and in Flatpak.
/// Renaming a parent directory can silently invalidate a filesystem watch.
pub struct FileMonitor {
    state: Rc<State>,
    timer: Option<glib::JoinHandle<()>>,
}

struct State {
    file: gio::File,
    context: glib::MainContext,
    monitor: RefCell<Option<gio::FileMonitor>>,
    pending: RefCell<Option<glib::JoinHandle<()>>>,
    monitor_deadline: Cell<Option<Instant>>,
    needs_rearm: Cell<bool>,
    is_polling_paused: Cell<bool>,
    on_check: Box<dyn Fn(CheckTrigger)>,
}

impl FileMonitor {
    /// Enable periodic polling for portal resources. With polling enabled,
    /// the watcher can operate even when the provider cannot install a watch.
    pub fn new(
        resource: &ResourceRef,
        should_poll_periodically: bool,
        on_check: impl Fn(CheckTrigger) + 'static,
    ) -> Result<Self, MonitorUnavailable> {
        let resource_uri = resource.uri();
        if resource_uri.contains('\0') {
            return Err(MonitorUnavailable);
        }
        let uri = glib::Uri::parse(resource_uri, glib::UriFlags::NONE)
            .or_else(|_| glib::Uri::parse(resource_uri, glib::UriFlags::ENCODED_PATH))
            .map_err(|_| MonitorUnavailable)?;
        let file = gio::File::for_uri(resource_uri);
        if !file.has_uri_scheme("file")
            || file.path().is_none()
            || uri.userinfo().is_some()
            || uri.port() != -1
            || uri
                .host()
                .is_some_and(|host| !host.is_empty() && !host.eq_ignore_ascii_case("localhost"))
        {
            return Err(MonitorUnavailable);
        }
        let state = Rc::new(State {
            file,
            context: glib::MainContext::ref_thread_default(),
            monitor: RefCell::new(None),
            pending: RefCell::new(None),
            monitor_deadline: Cell::new(None),
            needs_rearm: Cell::new(true),
            is_polling_paused: Cell::new(false),
            on_check: Box::new(on_check),
        });
        if state.arm().is_err() && !should_poll_periodically {
            return Err(MonitorUnavailable);
        }
        let timer = should_poll_periodically.then(|| {
            let weak = Rc::downgrade(&state);
            state.context.spawn_local(async move {
                loop {
                    glib::timeout_future(POLL_INTERVAL).await;
                    let Some(state) = weak.upgrade() else { break };
                    state.poll();
                }
            })
        });
        Ok(Self { state, timer })
    }

    /// Call on window focus on both the host and in Flatpak. The resulting
    /// verification can report a missing file even if a parent rename
    /// produced no monitor event.
    pub fn poll_now(&self) {
        self.state.poll();
    }

    /// Pause polling while a save is in flight. The caller still serializes
    /// checks already dispatched to its worker with the save.
    pub fn set_polling_paused(&self, is_paused: bool) {
        self.state.is_polling_paused.set(is_paused);
        if is_paused && self.state.monitor_deadline.get().is_none() {
            self.state.cancel_pending();
        }
    }
}

impl State {
    fn arm(self: &Rc<Self>) -> Result<(), MonitorUnavailable> {
        // A failed re-arm stays pending so future checks can retry it.
        self.needs_rearm.set(true);
        let monitor = self
            .context
            .with_thread_default(|| {
                self.file
                    .monitor_file(FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
            })
            .map_err(|_| MonitorUnavailable)?
            .map_err(|_| MonitorUnavailable)?;

        let weak = Rc::downgrade(self);
        monitor.connect_changed(move |_, _, _, event| {
            if let Some(state) = weak.upgrade() {
                if matches!(
                    event,
                    FileMonitorEvent::Created
                        | FileMonitorEvent::Deleted
                        | FileMonitorEvent::Moved
                        | FileMonitorEvent::Renamed
                        | FileMonitorEvent::MovedIn
                        | FileMonitorEvent::MovedOut
                        | FileMonitorEvent::Unmounted
                ) {
                    state.needs_rearm.set(true);
                }
                if state.monitor_deadline.get().is_none() {
                    state
                        .monitor_deadline
                        .set(Some(Instant::now() + MAX_DEBOUNCE_WAIT));
                }
                state.schedule();
            }
        });
        if let Some(previous) = self.monitor.replace(Some(monitor)) {
            previous.cancel();
        }
        self.needs_rearm.set(false);
        Ok(())
    }

    fn poll(self: &Rc<Self>) {
        if !self.is_polling_paused.get() && self.pending.borrow().is_none() {
            self.schedule();
        }
    }

    fn schedule(self: &Rc<Self>) {
        let debounce_end = Instant::now() + DEBOUNCE;
        let wake_at = self
            .monitor_deadline
            .get()
            .map_or(debounce_end, |deadline| deadline.min(debounce_end));
        if Instant::now() >= wake_at && self.pending.borrow().is_some() {
            return;
        }
        self.cancel_pending();
        let weak = Rc::downgrade(self);
        let task = self.context.spawn_local(async move {
            glib::timeout_future(wake_at.saturating_duration_since(Instant::now())).await;
            if let Some(state) = weak.upgrade() {
                state.pending.take();
                let has_monitor_event = state.monitor_deadline.take().is_some();
                if state.needs_rearm.get()
                    || state
                        .monitor
                        .borrow()
                        .as_ref()
                        .is_some_and(gio::FileMonitor::is_cancelled)
                {
                    // Keep the old watch if the provider cannot re-arm yet.
                    let _ = state.arm();
                }
                let trigger = if has_monitor_event {
                    CheckTrigger::Monitor
                } else {
                    CheckTrigger::Poll
                };
                (state.on_check)(trigger);
            }
        });
        self.pending.replace(Some(task));
    }

    fn cancel_pending(&self) {
        if let Some(task) = self.pending.take() {
            task.abort();
        }
    }
}

impl Drop for FileMonitor {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
        self.state.cancel_pending();
        if let Some(monitor) = self.state.monitor.take() {
            monitor.cancel();
        }
    }
}
