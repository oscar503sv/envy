//! ValueCodec over the ENV-T007 fixture corpus (ENV-T014).
//!
//! Failure messages carry only fixture names and node indices, never content.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test helpers may panic with context when the corpus is broken"
)]

use std::fs;
use std::path::{Path, PathBuf};

use envy::domain::codec::{EncodeContext, decode, encode, encode_in_node};
use envy::domain::document::NodeKind;
use envy::domain::parser::parse;

/// `(fixture, node index)` pairs whose raw value is not what `encode` writes
/// for its own displayed value. Each is legal: SYNTAX §6.2 only requires
/// `decode(encode(v)) == v`, and edits rewrite a value only if it changed.
const NON_CANONICAL: &[(&str, usize)] = &[
    // `\t` is shown as TAB and TAB is written verbatim (§6.2, SYN-37).
    ("quoted-double.env", 0),
    // Unknown `\q` / `\$` are shown literally and rewritten as `\\X` (§6.2, Q-06).
    ("quoted-double.env", 1),
    // Same rule for `\u`, `\0` and `\'` (§6.2, Q-06).
    ("quoted-double.env", 2),
    // The internal CRLF is not the dominant terminator, so it is written as LF (Q-05).
    ("quoted-double.env", 5),
    // `\` + real terminator is an unknown escape, rewritten as `\\` + terminator (§5.3, Q-06).
    ("quoted-multiline.env", 6),
    // The internal CRLF is not the dominant terminator, so it is written as LF (Q-05).
    ("quoted-single.env", 4),
];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixtures_dir())
        .expect("fixtures dir")
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".env"))
        .collect();
    names.sort();
    names
}

#[test]
fn codec_over_fixture_assignments() {
    let mut assignments = 0_usize;
    let mut non_canonical_seen = Vec::new();

    for name in fixture_names() {
        let bytes = fs::read(fixtures_dir().join(&name)).expect("fixture readable");
        let Ok(doc) = parse(bytes) else {
            continue;
        };
        let dominant = doc.dominant_terminator();
        for (index, node) in doc.nodes().iter().enumerate() {
            let NodeKind::Assignment(syntax) = node.kind() else {
                continue;
            };
            assignments += 1;
            let node_bytes = std::str::from_utf8(doc.current_bytes(node).expect("node bytes"))
                .expect("parsed documents are UTF-8");
            let raw = node_bytes
                .get(syntax.value_range.clone())
                .expect("value range inside node");
            let ctx = EncodeContext {
                dominant,
                is_multiline: syntax.is_multiline,
            };
            let displayed = decode(syntax.quote, raw);

            let Ok(encoded) = encode(syntax.quote, &displayed, ctx) else {
                panic!("{name} node {index}: displayed value must be encodable");
            };
            assert!(
                decode(syntax.quote, &encoded) == displayed,
                "{name} node {index}: decode(encode(decode(raw))) differs"
            );

            let is_listed = NON_CANONICAL.contains(&(name.as_str(), index));
            let is_canonical = encoded == raw;
            assert!(
                is_canonical != is_listed,
                "{name} node {index}: canonical={is_canonical} but allow-listed={is_listed}"
            );
            if is_listed {
                non_canonical_seen.push((name.clone(), index));
                continue;
            }

            let edited = encode_in_node(node_bytes, syntax, &displayed, dominant);
            assert!(
                edited.as_deref() == Ok(node_bytes),
                "{name} node {index}: encode_in_node must reproduce the node"
            );
        }
    }

    assert!(assignments > 0, "no assignments found");
    assert!(
        non_canonical_seen.len() == NON_CANONICAL.len(),
        "allow-list has entries that no fixture produced"
    );
}
