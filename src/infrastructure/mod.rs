//! Infrastructure layer: adapters for storage ports.
//!
//! Holds the Gio-based implementations; may depend on `domain` and `services`
//! and on GLib/Gio, but not on the UI.

#[cfg(target_os = "linux")]
pub mod gio_storage;
