//! Compares the parser with the ENV-T007 oracle descriptors, row by row.
//!
//! Failure messages never print keys or values.

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

/// Real terminators are displayed as LF (SYNTAX §6.1).
fn normalize_terminators(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut iter = raw.iter().copied().peekable();
    while let Some(byte) = iter.next() {
        if byte == b'\r' {
            iter.next_if_eq(&b'\n');
            out.push(b'\n');
        } else {
            out.push(byte);
        }
    }
    out
}

/// Displayed value per SYNTAX §5.3 / §6.1 for single-line raw bytes.
fn display_value(quote: QuoteStyle, raw: &[u8]) -> Vec<u8> {
    let raw = normalize_terminators(raw);
    if quote != QuoteStyle::Double {
        return raw;
    }
    let mut out = Vec::new();
    let mut iter = raw.into_iter();
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

/// Parses the descriptor line field `N` or `N-M` into (first, last).
fn parse_line_range(field: &str) -> (usize, usize) {
    let (first, last) = field.split_once('-').unwrap_or((field, field));
    (
        first.parse().expect("line number"),
        last.parse().expect("line number"),
    )
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
    let mut compared = Vec::new();
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
        assert!(doc.nodes().len() == rows.len(), "{fixture}: node count");
        compared.push(fixture.to_owned());

        for (index, row) in rows.iter().enumerate() {
            let row_number = index + 1;
            let fields: Vec<&str> = row.split('\t').collect();
            let node = doc.nodes().get(index).expect("node exists");
            let NodeOrigin::Source(span) = node.origin() else {
                panic!("{fixture}: node {row_number} must be a source node");
            };
            let (first_line, last_line) = parse_line_range(fields[1]);
            assert!(
                line_number_of(&doc, span.start) == first_line,
                "{fixture}: row {row_number} first line"
            );
            let covered_lines = split_lines(doc.source(), doc.body_start())
                .iter()
                .filter(|line| line.span.start < span.end)
                .count();
            assert!(
                covered_lines == last_line,
                "{fixture}: row {row_number} last line"
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

    println!("compared: {compared:?}");
    assert!(compared.len() >= 20);
}

#[test]
fn unclosed_quote_diagnostic_matches_fixtures() {
    {
        let (fixture, expected_line) = ("unclosed-quotes.env", 3);
        let bytes = fs::read(fixtures_dir().join(fixture)).expect("fixture readable");
        let doc = parse(bytes).expect("fixture parses");
        assert!(
            doc.unclosed_quote_line().map(|line| line.get()) == Some(expected_line),
            "{fixture}: unclosed quote line"
        );
    }
    for fixture in [
        "unclosed-quote-middle.env",
        "quoted-multiline.env",
        "only-cr.env",
        "plain.env",
    ] {
        let bytes = fs::read(fixtures_dir().join(fixture)).expect("fixture readable");
        let doc = parse(bytes).expect("fixture parses");
        assert!(
            doc.unclosed_quote_line().is_none(),
            "{fixture}: no diagnostic"
        );
    }
}

#[test]
fn invalid_utf8_fixture_is_rejected() {
    let bytes = fs::read(fixtures_dir().join("invalid-utf8.bin")).expect("fixture readable");
    assert!(matches!(parse(bytes), Err(DomainError::InvalidUtf8 { .. })));
}
