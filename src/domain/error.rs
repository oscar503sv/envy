//! Secret-free diagnostics policy.
//!
//! Logs and errors may include 1-based line numbers, byte offsets and lengths,
//! node kind tags, node IDs, counts, the user-visible file URI or display path,
//! and error variant names.
//!
//! Logs, error messages, Debug output, and panic messages must never include
//! variable values, variable keys, raw line text, file content or excerpts,
//! clipboard or paste content, or the content of `Redacted<T>`.
//!
//! When adding `tracing`, use only fields from the allowed list. Never use
//! `?value` or `%value` on anything holding user data. Types holding user data
//! must wrap it in `Redacted<T>` or implement Debug manually.
//!
//! Never derive Debug on a type that holds a value; wrap the field in
//! `Redacted<T>` instead.
//!
//! The file URI or display path may appear in the UI and in local debug logs
//! only. It must never reach analytics, exportable preferences or crash
//! reports, where the base name (or the path with `$HOME` shown as `~`) is used
//! instead, because absolute paths reveal user, client and project names.
//!
//! Use static strings or only allowed fields in `panic!` and `expect` messages.
//! Never format user data in a panic message. Standard-library panics can embed
//! user data on their own: slicing a `&str` at a bad index quotes the string,
//! `unwrap`/`expect` on a `Result` print the error's Debug, and `assert_eq!`
//! prints both operands. Production code therefore avoids string slicing by
//! index, `unwrap`/`expect` and `assert_eq!`; the panic hook installed by the
//! binary (ENV-T033) must discard the panic payload and keep only the location.

use std::{fmt, num::NonZeroUsize};

/// A node's kind without any of its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKindTag {
    Assignment,
    Comment,
    Blank,
    Raw,
}

impl fmt::Display for NodeKindTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Assignment => "assignment",
            Self::Comment => "comment",
            Self::Blank => "blank",
            Self::Raw => "raw",
        })
    }
}

/// A 1-based physical line number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineNumber(NonZeroUsize);

impl LineNumber {
    pub fn new(n: usize) -> Option<Self> {
        NonZeroUsize::new(n).map(Self)
    }

    pub fn get(self) -> usize {
        self.0.get()
    }
}

impl fmt::Display for LineNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// Derived Debug is safe because no variant carries a value or raw user data.
// `Copy` keeps it that way: a `String` or `Vec` field would stop compiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DomainError {
    #[error("file is not valid UTF-8 at byte {byte_offset}")]
    InvalidUtf8 { byte_offset: usize },
    #[error("line {line} ({kind}) cannot be edited structurally")]
    NotEditable { line: LineNumber, kind: NodeKindTag },
    #[error("node {node_id} does not exist in the document")]
    NodeNotFound { node_id: u64 },
    #[error(
        "unclosed quote starting at line {line}; the rest of the file is read-only until it is fixed"
    )]
    UnclosedQuote { line: LineNumber },
}

#[cfg(test)]
mod tests {
    use super::{DomainError, LineNumber, NodeKindTag};
    use crate::test_support::SENTINEL;

    #[test]
    fn diagnostics_are_value_free() {
        let line = LineNumber::new(3).unwrap();
        let errors = [
            DomainError::InvalidUtf8 { byte_offset: 7 },
            DomainError::NotEditable {
                line,
                kind: NodeKindTag::Raw,
            },
            DomainError::NodeNotFound { node_id: 9 },
            DomainError::UnclosedQuote { line },
        ];

        for error in errors {
            // Match every field so changes to the diagnostic payload require review.
            let expected = match error {
                DomainError::InvalidUtf8 { byte_offset: _ } => "file is not valid UTF-8 at byte 7",
                DomainError::NotEditable { line: _, kind: _ } => {
                    "line 3 (raw) cannot be edited structurally"
                }
                DomainError::NodeNotFound { node_id: _ } => "node 9 does not exist in the document",
                DomainError::UnclosedQuote { line: _ } => {
                    "unclosed quote starting at line 3; the rest of the file is read-only until it is fixed"
                }
            };
            let display = error.to_string();
            assert!(!display.contains(SENTINEL));
            assert!(!format!("{error:?}").contains(SENTINEL));
            assert_eq!(display, expected);
        }
    }

    #[test]
    fn line_numbers_start_at_one() {
        assert!(LineNumber::new(0).is_none());
        let line = LineNumber::new(1).unwrap();
        assert_eq!(line.get(), 1);
        assert_eq!(line.to_string(), "1");
    }

    #[test]
    fn node_kind_tags_display_in_lowercase() {
        for (kind, expected) in [
            (NodeKindTag::Assignment, "assignment"),
            (NodeKindTag::Comment, "comment"),
            (NodeKindTag::Blank, "blank"),
            (NodeKindTag::Raw, "raw"),
        ] {
            assert_eq!(kind.to_string(), expected);
        }
    }
}
