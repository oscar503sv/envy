//! Envy's core library.
//!
//! Layering rule: `domain` has no I/O and no GTK/GLib/Gio; `services` depends
//! on `domain` and storage ports, never on GTK; `infrastructure` holds the
//! Gio-based adapters. The GTK UI lives in the binary.

pub mod domain;
pub mod infrastructure;
pub mod services;

#[cfg(test)]
pub(crate) mod test_support;
