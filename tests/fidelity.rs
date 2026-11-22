//! Byte-for-byte round trip over the whole fixture corpus.
//!
//! Failure messages carry only the fixture name, an offset and lengths.

#![allow(
    clippy::expect_used,
    reason = "test helpers may panic with context when the corpus is broken"
)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{first_difference, mismatch_message};
use envy::domain::error::DomainError;
use envy::domain::parser::parse;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn is_protected(fixture: &str) -> bool {
    let descriptor = fixtures_dir()
        .join("expected")
        .join(format!("{fixture}.nodes"));
    fs::read_to_string(descriptor)
        .expect("every fixture has a descriptor")
        .lines()
        .next()
        == Some("protected")
}

fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixtures_dir())
        .expect("fixtures dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        // A non-UTF-8 name must fail loudly instead of being skipped.
        .map(|entry| {
            entry
                .file_name()
                .into_string()
                .unwrap_or_else(|_| panic!("fixture names must be UTF-8"))
        })
        .filter(|name| name != "SHA256SUMS")
        .collect();
    names.sort();
    names
}

#[test]
fn serialize_parse_is_identity_for_every_fixture() {
    let mut checked = 0_usize;
    let mut expected_checked = 0_usize;
    for name in fixture_names() {
        let bytes = fs::read(fixtures_dir().join(&name)).expect("fixture readable");
        if is_protected(&name) {
            assert!(
                matches!(parse(bytes), Err(DomainError::InvalidUtf8 { .. })),
                "{name}: protected fixture must fail with InvalidUtf8"
            );
            continue;
        }
        expected_checked += 1;
        let doc = parse(bytes.clone()).unwrap_or_else(|_| panic!("{name}: must parse"));
        if let Some(difference) = first_difference(&bytes, &doc.serialize()) {
            panic!("{}", mismatch_message(&name, difference));
        }
        checked += 1;
    }
    assert!(expected_checked > 0, "no fixtures found");
    assert!(checked == expected_checked, "a fixture was skipped");
}

#[test]
fn mismatch_message_contains_only_numbers() {
    const SENTINEL: &str = "ENVY_TEST_SENTINEL_9F2C";
    let expected = format!("A={SENTINEL}\n").into_bytes();
    let actual = format!("A={SENTINEL}X\n").into_bytes();
    let difference = first_difference(&expected, &actual).expect("slices differ");
    let message = mismatch_message("sample.env", difference);

    assert!(message.contains(&difference.offset.to_string()));
    assert!(message.contains(&expected.len().to_string()));
    assert!(message.contains(&actual.len().to_string()));
    assert!(!message.contains(SENTINEL));
    assert!(!message.contains("A="));
    assert!(!format!("{difference:?}").contains(SENTINEL));
    assert!(first_difference(&expected, &expected).is_none());
    assert!(first_difference(b"ab", b"abc").map(|d| d.offset) == Some(2));
}
