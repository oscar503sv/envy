//! Domain layer: pure dotenv model and rules.
//!
//! Performs no I/O and must not depend on GTK, GDK, libadwaita, GLib or Gio.

pub mod codec;
pub mod compare;
pub mod document;
pub mod edit;
pub mod environment;
pub mod error;
pub mod history;
pub mod parser;
pub mod paste;
pub mod redact;
pub mod secrets;
pub mod syntax;
