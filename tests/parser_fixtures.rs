//! Compares the parser (ENV-T010) with the ENV-T007 oracle descriptors.
//!
//! Multi-line rows belong to ENV-T011. For fixtures that contain them, every
//! row before the first multi-line row is still compared, because T010 nodes
//! are independent per line. Failure messages never print keys or values.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test helpers may panic with context when the corpus is broken"
)]

use std::fs;
use std::path::{Path, PathBuf};

use envy::domain::document::{EnvDocument, NodeKind, NodeOrigin};
use envy::domain::error::DomainError;
use envy::domain::parser::parse;
use envy::domain::syntax::{LineTerminator, QuoteStyle, split_lines};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn terminator_tag(terminator: Option<LineTerminator>) -> &'static str {
    match terminator {
        Some(LineTerminator::Lf) => "lf",
        Some(LineTerminator::CrLf) => "crlf",
        Some(LineTerminator::Cr) => "cr",
        None => "none",
    }
}

fn quote_tag(quote: QuoteStyle) -> &'static str {
    match quote {
        QuoteStyle::Unquoted => "unquoted",
        QuoteStyle::Single => "single",
        QuoteStyle::Double => "double",
    }
}

/// Descriptor values are the displayed value with `\\`, `\n`, `\r`, `\t` escaped.
fn unescape_descriptor(field: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buf = [0_u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match chars.next() {
            Some('n') => out.push(b'\n'),
            Some('r') => out.push(b'\r'),
            Some('t') => out.push(b'\t'),
            Some('\\') => out.push(b'\\'),
            _ => panic!("bad escape in descriptor"),
        }
    }
    out
}

/// Displayed value per SYNTAX §5.3 / §6.1 for single-line raw bytes.
fn display_value(quote: QuoteStyle, raw: &[u8]) -> Vec<u8> {
    if quote != QuoteStyle::Double {
        return raw.to_vec();
    }
    let mut out = Vec::new();
    let mut iter = raw.iter().copied();
    while let Some(byte) = iter.next() {
        if byte != b'\\' {
            out.push(byte);
            continue;
        }
        match iter.next() {
            Some(b'\\') => out.push(b'\\'),
            Some(b'"') => out.push(b'"'),
            Some(b'n') => out.push(b'\n'),
            Some(b'r') => out.push(b'\r'),
            Some(b't') => out.push(b'\t'),
            Some(other) => out.extend_from_slice(&[b'\\', other]),
            None => out.push(b'\\'),
        }
    }
    out
}

fn is_multi_line(range: &str) -> bool {
    match range.split_once('-') {
        Some((a, b)) => a != b,
        None => false,
    }
}

fn line_number_of(doc: &EnvDocument, start: usize) -> usize {
    split_lines(doc.source(), doc.body_start())
        .iter()
        .position(|line| line.span.start == start)
        .map(|index| index + 1)
        .expect("node starts at a physical line")
}

#[test]
fn parser_agrees_with_oracle_descriptors() {
    let mut full = Vec::new();
    let mut partial = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(fixtures_dir().join("expected"))
        .expect("expected/ must exist")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    entries.sort();

    for path in entries {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let Some(fixture) = file_name.strip_suffix(".nodes") else {
            continue;
        };
        let descriptor = fs::read_to_string(&path).expect("descriptor must be UTF-8");
        let mut rows = descriptor.lines();
        let header = rows.next().unwrap_or_default();
        let bytes = fs::read(fixtures_dir().join(fixture)).expect("fixture readable");

        if header == "protected" {
            assert!(
                matches!(parse(bytes), Err(DomainError::InvalidUtf8 { .. })),
                "{fixture}: protected fixture must fail with InvalidUtf8"
            );
            continue;
        }

        let doc = parse(bytes).unwrap_or_else(|_| panic!("{fixture}: must parse"));
        assert!(doc.check_invariants().is_ok(), "{fixture}: invariants");
        assert!(doc.has_bom() == (header == "bom=true"), "{fixture}: bom");

        let rows: Vec<&str> = rows.filter(|r| !r.is_empty()).collect();
        let comparable = rows
            .iter()
            .position(|row| is_multi_line(row.split('\t').nth(1).unwrap_or_default()))
            .unwrap_or(rows.len());
        let is_full = comparable == rows.len();
        if is_full {
            assert!(doc.nodes().len() == rows.len(), "{fixture}: node count");
            full.push(fixture.to_owned());
        } else {
            assert!(doc.nodes().len() >= comparable, "{fixture}: node count");
            partial.push(format!("{fixture} ({comparable}/{} rows)", rows.len()));
        }

        for (index, row) in rows.iter().take(comparable).enumerate() {
            let row_number = index + 1;
            let fields: Vec<&str> = row.split('\t').collect();
            let node = doc.nodes().get(index).expect("node exists");
            let NodeOrigin::Source(span) = node.origin() else {
                panic!("{fixture}: node {row_number} must be a source node");
            };
            let expected_line: usize = fields[1].parse().expect("single line number");
            assert!(
                line_number_of(&doc, span.start) == expected_line,
                "{fixture}: row {row_number} line number"
            );
            assert!(
                terminator_tag(node.terminator()) == fields[2],
                "{fixture}: row {row_number} terminator"
            );
            let kind_tag = match node.kind() {
                NodeKind::Assignment(_) => "assignment",
                NodeKind::Comment => "comment",
                NodeKind::Blank => "blank",
                NodeKind::Raw => "raw",
            };
            assert!(kind_tag == fields[0], "{fixture}: row {row_number} kind");

            let NodeKind::Assignment(syntax) = node.kind() else {
                continue;
            };
            assert!(
                *syntax.key.expose() == fields[3],
                "{fixture}: row {row_number} key"
            );
            assert!(
                quote_tag(syntax.quote) == fields[4],
                "{fixture}: row {row_number} quote"
            );
            assert!(
                syntax.is_export == (fields[5] == "true"),
                "{fixture}: row {row_number} export"
            );
            assert!(
                syntax.is_multiline == (fields[6] == "true"),
                "{fixture}: row {row_number} multiline"
            );
            let node_bytes = doc.current_bytes(node).expect("node bytes");
            let raw = node_bytes
                .get(syntax.value_range.clone())
                .expect("value range inside node");
            assert!(
                display_value(syntax.quote, raw) == unescape_descriptor(fields[7]),
                "{fixture}: row {row_number} value"
            );
        }
    }

    println!("fully compared: {full:?}");
    println!("prefix compared (rest is ENV-T011): {partial:?}");
    assert!(!full.is_empty());
}

#[test]
fn invalid_utf8_fixture_is_rejected() {
    let bytes = fs::read(fixtures_dir().join("invalid-utf8.bin")).expect("fixture readable");
    assert!(matches!(parse(bytes), Err(DomainError::InvalidUtf8 { .. })));
}
