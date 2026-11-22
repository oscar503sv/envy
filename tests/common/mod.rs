//! Content-free byte comparison shared by unit and integration tests.
//!
//! Nothing here ever formats input bytes, only offsets and lengths.

#![allow(dead_code, reason = "each test crate uses a different subset")]

use std::fmt;

/// Location of the first mismatch between two byte slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstDifference {
    pub offset: usize,
    pub expected_len: usize,
    pub actual_len: usize,
}

impl fmt::Display for FirstDifference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "first difference at offset {} (expected {} bytes, actual {} bytes)",
            self.offset, self.expected_len, self.actual_len
        )
    }
}

/// Returns `None` when equal; otherwise the offset of the first differing
/// byte, or the shorter length when one slice is a prefix of the other.
pub fn first_difference(expected: &[u8], actual: &[u8]) -> Option<FirstDifference> {
    if expected == actual {
        return None;
    }
    let offset = expected
        .iter()
        .zip(actual)
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| expected.len().min(actual.len()));
    Some(FirstDifference {
        offset,
        expected_len: expected.len(),
        actual_len: actual.len(),
    })
}

/// Builds the failure message for a fixture, never including bytes.
pub fn mismatch_message(fixture: &str, difference: FirstDifference) -> String {
    format!("{fixture}: round trip mismatch, {difference}")
}
