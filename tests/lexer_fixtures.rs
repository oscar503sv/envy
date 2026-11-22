//! Ties the lexer (ENV-T009) to the fixture corpus and its ENV-T007 oracle.

use std::fs;
use std::path::{Path, PathBuf};

use envy::domain::document::EnvDocument;
use envy::domain::error::DomainError;
use envy::domain::syntax::{LineTerminator, check_utf8, split_lines};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn read(name: &str) -> Vec<u8> {
    fs::read(fixtures_dir().join(name)).unwrap_or_else(|_| panic!("cannot read fixture {name}"))
}

fn tag(terminator: Option<LineTerminator>) -> &'static str {
    match terminator {
        Some(LineTerminator::Lf) => "lf",
        Some(LineTerminator::CrLf) => "crlf",
        Some(LineTerminator::Cr) => "cr",
        None => "none",
    }
}

#[test]
fn utf8_bom_fixture_starts_after_bom_and_is_lossless() {
    let bytes = read("utf8-bom.env");
    let doc = EnvDocument::new(bytes.clone());
    assert!(doc.has_bom());
    assert_eq!(doc.body_start(), 3);
    let lines = split_lines(&bytes, doc.body_start());
    assert_eq!(lines.first().map(|l| l.span.start), Some(3));
    let mut rebuilt = bytes.get(..3).unwrap_or_default().to_vec();
    for line in &lines {
        rebuilt.extend_from_slice(bytes.get(line.span.clone()).unwrap_or_default());
    }
    assert!(rebuilt == bytes);
}

#[test]
fn mixed_newlines_fixture_terminator_sequence_matches_bytes() {
    let bytes = read("mixed-newlines.env");
    let mut expected = Vec::new();
    let mut iter = bytes.iter().peekable();
    while let Some(byte) = iter.next() {
        match byte {
            b'\n' => expected.push("lf"),
            b'\r' if iter.next_if_eq(&&b'\n').is_some() => expected.push("crlf"),
            b'\r' => expected.push("cr"),
            _ => {}
        }
    }
    let actual: Vec<_> = split_lines(&bytes, 0)
        .iter()
        .filter(|l| l.terminator.is_some())
        .map(|l| tag(l.terminator))
        .collect();
    assert_eq!(actual, expected);
    assert!(expected.contains(&"lf") && expected.contains(&"crlf") && expected.contains(&"cr"));
}

#[test]
fn invalid_utf8_fixture_reports_first_bad_byte_offset() {
    let bytes = read("invalid-utf8.bin");
    let expected = bytes
        .iter()
        .enumerate()
        .find_map(|(i, _)| {
            std::str::from_utf8(bytes.get(..=i).unwrap_or_default())
                .err()
                .filter(|e| e.error_len().is_some())
                .map(|e| e.valid_up_to())
        })
        .expect("fixture must contain invalid UTF-8");
    assert!(matches!(
        check_utf8(&bytes, 0),
        Err(DomainError::InvalidUtf8 { byte_offset }) if byte_offset == expected
    ));
    assert!(read("invalid-utf8.bin") == bytes);
}

#[test]
fn lexer_agrees_with_oracle_descriptors() {
    let mut checked = 0;
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
        if header == "protected" {
            continue;
        }
        let bytes = read(fixture);
        let doc = EnvDocument::new(bytes.clone());
        assert_eq!(header == "bom=true", doc.has_bom(), "{fixture}: bom flag");
        let lines = split_lines(&bytes, doc.body_start());
        let mut rebuilt = bytes.get(..doc.body_start()).unwrap_or_default().to_vec();
        for line in &lines {
            rebuilt.extend_from_slice(bytes.get(line.span.clone()).unwrap_or_default());
        }
        // assert! instead of assert_eq! so fixture bytes are never printed.
        assert!(rebuilt == bytes, "{fixture}: lines must rebuild the file");
        let mut highest = 0;
        for row in rows.filter(|r| !r.is_empty()) {
            let mut fields = row.split('\t');
            let range = fields.nth(1).expect("range field");
            let terminator = fields.next().expect("terminator field");
            let last: usize = range
                .rsplit('-')
                .next()
                .and_then(|n| n.parse().ok())
                .expect("numeric line range");
            highest = highest.max(last);
            let line = lines
                .get(last - 1)
                .unwrap_or_else(|| panic!("{fixture}: no line {last}"));
            assert_eq!(tag(line.terminator), terminator, "{fixture}: line {last}");
        }
        assert_eq!(lines.len(), highest, "{fixture}: physical line count");
        checked += 1;
    }
    assert!(checked > 0);
}
