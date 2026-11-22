//! Service layer: application use cases.
//!
//! May depend on `domain` and on storage ports, but never on GTK, GDK or
//! libadwaita.

pub mod document_service;
pub mod storage;
