//! Domain layer: pure dotenv model and rules.
//!
//! Performs no I/O and must not depend on GTK, GDK, libadwaita, GLib or Gio.

pub mod document;
pub mod environment;
pub mod error;
pub mod parser;
pub mod redact;
pub mod syntax;
