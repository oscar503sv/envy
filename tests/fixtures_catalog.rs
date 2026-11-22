//! Guards the fidelity fixture corpus (ENV-T007) against docs/SYNTAX.md.

#![allow(
    clippy::expect_used,
    reason = "test helpers may panic with context when the corpus is broken"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const SENTINEL: &str = "ENVY_TEST_SENTINEL_9F2C";
const SENTINEL_FIXTURES: &[&str] = &[
    "plain.env",
    "quoted-double.env",
    "quoted-multiline.env",
    "inline-comments.env",
    "unicode-values.env",
];
const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const NODE_KINDS: &[&str] = &["assignment", "comment", "blank", "raw"];
const TERMINATORS: &[&str] = &["lf", "crlf", "cr", "none"];
const QUOTE_STYLES: &[&str] = &["unquoted", "single", "double"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    root().join("tests").join("fixtures")
}

fn read_syntax() -> String {
    fs::read_to_string(root().join("docs").join("SYNTAX.md")).expect("docs/SYNTAX.md is readable")
}

/// Lines of the section that starts at the heading with `start` and ends before the one with `end`.
fn section<'a>(text: &'a str, start: &str, end: &str) -> Vec<&'a str> {
    text.lines()
        .skip_while(|l| !l.starts_with(start))
        .skip(1)
        .take_while(|l| !l.starts_with(end))
        .collect()
}

fn cells(row: &str) -> Vec<&str> {
    row.split('|').map(str::trim).collect()
}

fn syn_ids(text: &str) -> Vec<String> {
    text.match_indices("SYN-")
        .filter_map(|(i, _)| text.get(i..i + 6))
        .filter(|id| {
            id.get(4..)
                .is_some_and(|d| d.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(str::to_owned)
        .collect()
}

/// Fixture name -> SYN ids, from the §9 table.
fn fixture_table(syntax: &str) -> Vec<(String, Vec<String>)> {
    section(syntax, "## 9.", "## 10.")
        .into_iter()
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            let c = cells(l);
            let name = c
                .get(1)
                .copied()
                .unwrap_or_default()
                .trim_matches('`')
                .to_owned();
            let rules = syn_ids(c.get(3).copied().unwrap_or_default());
            (name, rules)
        })
        .collect()
}

fn rule_ids(syntax: &str) -> BTreeSet<String> {
    section(syntax, "## 7.", "## 8.")
        .into_iter()
        .filter(|l| l.starts_with("| SYN-"))
        .flat_map(|l| syn_ids(cells(l).get(1).copied().unwrap_or_default()))
        .collect()
}

fn fixture_names_on_disk() -> BTreeSet<String> {
    fs::read_dir(fixtures_dir())
        .expect("fixtures dir exists")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != "SHA256SUMS")
        .collect()
}

/// Physical lines per SYNTAX §1: the BOM is not a line, CRLF is one terminator.
fn physical_line_count(bytes: &[u8]) -> usize {
    let body = bytes.strip_prefix(BOM).unwrap_or(bytes);
    let mut count = 0;
    let mut i = 0;
    while i < body.len() {
        match body.get(i) {
            Some(b'\n') => count += 1,
            Some(b'\r') => {
                count += 1;
                if body.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if !matches!(body.last(), None | Some(b'\n' | b'\r')) {
        count += 1;
    }
    count
}

fn parse_range(field: &str) -> Result<(usize, usize), String> {
    let (a, b) = field.split_once('-').unwrap_or((field, field));
    let a: usize = a.parse().map_err(|_| format!("bad range {field}"))?;
    let b: usize = b.parse().map_err(|_| format!("bad range {field}"))?;
    if a == 0 || b < a {
        return Err(format!("invalid range {field}"));
    }
    Ok((a, b))
}

fn check_node_line(line: &str) -> Result<(usize, usize), String> {
    let f: Vec<&str> = line.split('\t').collect();
    let kind = f.first().copied().unwrap_or_default();
    if !NODE_KINDS.contains(&kind) {
        return Err("unknown kind".to_owned());
    }
    let expected_fields = if kind == "assignment" { 8 } else { 3 };
    if f.len() != expected_fields {
        return Err(format!("{kind} needs {expected_fields} fields"));
    }
    let range = parse_range(f.get(1).copied().unwrap_or_default())?;
    if !TERMINATORS.contains(&f.get(2).copied().unwrap_or_default()) {
        return Err("bad terminator".to_owned());
    }
    if kind == "assignment" {
        let key = f.get(3).copied().unwrap_or_default();
        let valid_key = key
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid_key {
            return Err("bad key".to_owned());
        }
        if !QUOTE_STYLES.contains(&f.get(4).copied().unwrap_or_default()) {
            return Err("bad quote style".to_owned());
        }
        for idx in [5, 6] {
            if !matches!(f.get(idx).copied(), Some("true" | "false")) {
                return Err("bad boolean".to_owned());
            }
        }
    }
    Ok(range)
}

#[test]
fn fixtures_match_syntax_table() {
    let table = fixture_table(&read_syntax());
    let documented: BTreeSet<String> = table.iter().map(|(n, _)| n.clone()).collect();
    assert_eq!(table.len(), 22, "SYNTAX.md §9 lists 22 fixtures");
    assert_eq!(documented, fixture_names_on_disk());
}

#[test]
fn every_fixture_name_appears_in_syntax() {
    let syntax = read_syntax();
    for name in fixture_names_on_disk() {
        assert!(
            syntax.contains(&name),
            "{name} is not mentioned in SYNTAX.md"
        );
    }
}

#[test]
fn every_fixture_has_a_valid_descriptor() {
    let expected_dir = fixtures_dir().join("expected");
    let descriptors: BTreeSet<String> = fs::read_dir(&expected_dir)
        .expect("expected dir exists")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let wanted: BTreeSet<String> = fixture_names_on_disk()
        .into_iter()
        .map(|n| format!("{n}.nodes"))
        .collect();
    assert_eq!(
        descriptors, wanted,
        "one descriptor per fixture and vice versa"
    );

    for name in fixture_names_on_disk() {
        let bytes = fs::read(fixtures_dir().join(&name)).expect("fixture readable");
        let raw =
            fs::read(expected_dir.join(format!("{name}.nodes"))).expect("descriptor readable");
        let text =
            String::from_utf8(raw).unwrap_or_else(|_| panic!("{name}: descriptor is not UTF-8"));
        assert!(!text.contains('\r'), "{name}: descriptor must use LF only");
        let mut lines = text.lines();
        let head = lines.next().unwrap_or_default();

        if head == "protected" {
            assert_eq!(
                text, "protected\n",
                "{name}: protected descriptor is a single line"
            );
            assert!(
                std::str::from_utf8(&bytes).is_err(),
                "{name}: protected fixture must be invalid UTF-8"
            );
            continue;
        }

        let has_bom = bytes.starts_with(BOM);
        assert_eq!(head, format!("bom={has_bom}"), "{name}: bom flag");
        assert!(
            std::str::from_utf8(bytes.strip_prefix(BOM).unwrap_or(&bytes)).is_ok(),
            "{name}: non-protected fixture must be valid UTF-8"
        );

        let mut next_line = 1;
        // Failure messages cite the descriptor row number, never its text,
        // because rows of secret fixtures carry the sentinel (SYNTAX §10).
        for (row, node) in (2..).zip(lines) {
            let (a, b) =
                check_node_line(node).unwrap_or_else(|e| panic!("{name}.nodes row {row}: {e}"));
            assert_eq!(
                a, next_line,
                "{name}.nodes row {row}: ranges must be contiguous"
            );
            next_line = b + 1;
        }
        assert_eq!(
            next_line - 1,
            physical_line_count(&bytes),
            "{name}: descriptor must cover every physical line"
        );
    }
}

#[test]
fn every_syn_rule_is_covered_by_a_fixture() {
    let syntax = read_syntax();
    let defined = rule_ids(&syntax);
    assert_eq!(defined.len(), 42, "SYNTAX.md §7 defines 42 rules");

    let cited: BTreeSet<String> = fixture_table(&syntax)
        .into_iter()
        .flat_map(|(_, rules)| rules)
        .collect();
    let uncovered: Vec<_> = defined.difference(&cited).collect();
    assert!(uncovered.is_empty(), "rules without fixture: {uncovered:?}");
    let unknown: Vec<_> = cited.difference(&defined).collect();
    assert!(unknown.is_empty(), "§9 cites undefined rules: {unknown:?}");
}

#[test]
fn sentinel_is_present_in_secret_fixtures() {
    for name in SENTINEL_FIXTURES {
        let text = fs::read_to_string(fixtures_dir().join(name)).expect("fixture is UTF-8");
        assert!(text.contains(SENTINEL), "{name} must contain the sentinel");
    }
}

#[test]
fn secret_looking_values_use_only_the_sentinel() {
    const MARKERS: &[&str] = &["TOKEN", "SECRET", "PASSWORD", "API"];
    for name in fixture_names_on_disk() {
        let Ok(text) = fs::read_to_string(fixtures_dir().join(&name)) else {
            continue;
        };
        for (line_number, line) in (1..).zip(text.lines()) {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if !MARKERS.iter().any(|m| key.to_ascii_uppercase().contains(m)) {
                continue;
            }
            let value = value
                .split(" #")
                .next()
                .unwrap_or_default()
                .trim_matches(['"', '\'']);
            // assert! instead of assert_eq! so a real secret is never echoed.
            assert!(
                value == SENTINEL,
                "{name}: line {line_number} has a secret-looking value that is not the sentinel"
            );
        }
    }
}
