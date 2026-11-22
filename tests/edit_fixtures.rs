//! SetValue over the ENV-T007 fixture corpus (ENV-T016).
//!
//! Failure messages carry only fixture names and node indices, never content.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test helpers may panic with context when the corpus is broken"
)]

use std::fs;
use std::path::{Path, PathBuf};

use envy::domain::codec::decode;
use envy::domain::document::{EnvDocument, NodeKind};
use envy::domain::edit::{ConversionPolicy, SetValueOutcome, set_value};
use envy::domain::parser::parse;
use envy::domain::syntax::QuoteStyle;

const VALUES: &[&str] = &[
    "x",
    "a b",
    "multi\nline",
    "é世",
    "",
    "it's \"q\" #",
    " pad ",
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

/// Absolute byte offsets of node `index` in the serialization, with the BOM
/// counted, as `(start, end)`.
fn node_span(doc: &EnvDocument, index: usize) -> (usize, usize) {
    let mut start = doc.body_start();
    for node in &doc.nodes()[..index] {
        start += doc.current_bytes(node).expect("bytes").len();
    }
    let len = doc.current_bytes(&doc.nodes()[index]).expect("bytes").len();
    (start, start + len)
}

fn node_text(doc: &EnvDocument, index: usize) -> String {
    String::from_utf8(
        doc.current_bytes(&doc.nodes()[index])
            .expect("bytes")
            .to_vec(),
    )
    .expect("utf8 node")
}

#[test]
fn set_value_touches_only_the_edited_value_in_every_fixture() {
    let mut edits = 0_usize;
    for name in fixture_names() {
        let bytes = fs::read(fixtures_dir().join(&name)).expect("fixture readable");
        let Ok(original) = parse(bytes) else {
            continue;
        };
        for index in 0..original.nodes().len() {
            let NodeKind::Assignment(old_syntax) = original.nodes()[index].kind() else {
                continue;
            };
            let old_text = node_text(&original, index);
            let old_range = old_syntax.value_range.clone();
            let old_quote = old_syntax.quote;
            let (start, end) = node_span(&original, index);
            let serialized = original.serialize();

            for value in VALUES {
                let mut doc = original.clone();
                let node_id = doc.nodes()[index].id();
                let outcome =
                    set_value(&mut doc, node_id, value, ConversionPolicy::ConvertToDouble)
                        .unwrap_or_else(|_| panic!("{name} node {index}: edit failed"));
                edits += 1;
                assert!(
                    doc.check_invariants().is_ok(),
                    "{name} node {index}: invariants"
                );
                let out = doc.serialize();
                let (new_start, new_end) = node_span(&doc, index);
                assert!(new_start == start, "{name} node {index}: start moved");

                // Bytes before and after the node.
                assert!(
                    out[..start] == serialized[..start],
                    "{name} node {index}: bytes before the node changed"
                );
                assert!(
                    out[new_end..] == serialized[end..],
                    "{name} node {index}: bytes after the node changed"
                );

                let new_text = node_text(&doc, index);
                let NodeKind::Assignment(new_syntax) = doc.nodes()[index].kind() else {
                    panic!("{name} node {index}: no longer an assignment");
                };
                let raw = new_text
                    .get(new_syntax.value_range.clone())
                    .expect("range inside node");
                assert!(
                    decode(new_syntax.quote, raw) == *value,
                    "{name} node {index}: value mismatch"
                );

                let converted = old_quote != QuoteStyle::Double
                    && new_syntax.quote == QuoteStyle::Double
                    && old_quote != new_syntax.quote;
                // The new value range is not compared: an empty value may
                // absorb whitespace on either side of it, but the bytes around
                // the old value must survive as a prefix and a suffix.
                let delim = usize::from(converted && old_quote == QuoteStyle::Single);
                let prefix_len = old_range.start - delim;
                let old_suffix = &old_text.as_bytes()[old_range.end + delim..];
                assert!(
                    new_text.as_bytes()[..prefix_len] == old_text.as_bytes()[..prefix_len],
                    "{name} node {index}: prefix changed"
                );
                assert!(
                    new_text.as_bytes().ends_with(old_suffix),
                    "{name} node {index}: suffix changed"
                );
                if !converted {
                    assert!(
                        new_syntax.quote == old_quote,
                        "{name} node {index}: quote style changed without conversion"
                    );
                }

                if matches!(outcome, SetValueOutcome::Unchanged) {
                    assert!(
                        out == serialized,
                        "{name} node {index}: Unchanged altered the document"
                    );
                }

                // Re-parse the output.
                let reparsed = parse(out).expect("serialization parses");
                assert!(
                    reparsed.nodes().len() == original.nodes().len(),
                    "{name} node {index}: node count changed"
                );
                let NodeKind::Assignment(syntax) = reparsed.nodes()[index].kind() else {
                    panic!("{name} node {index}: reparsed node is not an assignment");
                };
                let text = node_text(&reparsed, index);
                assert!(
                    decode(
                        syntax.quote,
                        text.get(syntax.value_range.clone()).expect("range")
                    ) == *value,
                    "{name} node {index}: reparsed value mismatch"
                );
            }
        }
    }
    assert!(edits > 0, "no assignments found");
}

#[test]
fn reject_policy_never_changes_the_quote_style_in_any_fixture() {
    for name in fixture_names() {
        let bytes = fs::read(fixtures_dir().join(&name)).expect("fixture readable");
        let Ok(original) = parse(bytes) else {
            continue;
        };
        for index in 0..original.nodes().len() {
            let NodeKind::Assignment(old) = original.nodes()[index].kind() else {
                continue;
            };
            let old_quote = old.quote;
            for value in VALUES {
                let mut doc = original.clone();
                let node_id = doc.nodes()[index].id();
                match set_value(&mut doc, node_id, value, ConversionPolicy::Reject) {
                    Ok(_) => {
                        let NodeKind::Assignment(new) = doc.nodes()[index].kind() else {
                            panic!("{name} node {index}: not an assignment");
                        };
                        assert!(new.quote == old_quote, "{name} node {index}: quote changed");
                        assert!(doc.check_invariants().is_ok(), "{name} node {index}");
                    }
                    Err(_) => assert!(
                        doc.serialize() == original.serialize(),
                        "{name} node {index}: failed edit changed the document"
                    ),
                }
            }
        }
    }
}
