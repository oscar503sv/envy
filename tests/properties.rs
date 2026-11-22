//! Property tests for the parser/serializer round trip (ENV-T013).
//!
//! Properties checked for every input:
//! - P1: `parse` never panics; invalid UTF-8 after the optional BOM yields
//!   `InvalidUtf8` at the absolute offset of the first bad byte, otherwise `Ok`.
//! - P2: `parse(x).serialize() == x`, invariants hold, and node source spans are
//!   contiguous and cover the whole body.
//! - P3: parsing is deterministic.
//!
//! Failure messages carry only lengths and offsets, never content. proptest
//! itself prints the minimized failing input; that is acceptable because every
//! generator draws from small synthetic alphabets and bounded lengths (keys
//! have at most 9 characters, the sentinel `ENVY_TEST_SENTINEL_9F2C` has 23),
//! so the generators cannot produce the sentinel string.
//!
//! Regressions: proptest persists minimized counterexamples to
//! `proptest-regressions/properties.txt` (committed to git, replayed on every
//! run). Every counterexample found must ALSO be added by hand to the list in
//! `regression_cases`. The files in `tests/fixtures` are an immutable oracle
//! corpus and are not extended by this task.
//!
//! Case count: `PROPTEST_CASES` if set, otherwise `DEFAULT_CASES`.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "integration tests may panic with context on broken expectations"
)]

use envy::domain::codec::{EncodeContext, decode, encode, encode_in_node};
use envy::domain::document::{EnvDocument, NodeKind, NodeOrigin};
use envy::domain::error::DomainError;
use envy::domain::parser::parse;
use envy::domain::syntax::{LineTerminator, QuoteStyle};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

const BOM: &[u8] = b"\xEF\xBB\xBF";
const DEFAULT_CASES: u32 = 4096;

fn config() -> ProptestConfig {
    // `ProptestConfig::default()` already honours `PROPTEST_CASES`, but an
    // explicit read keeps the local default in one visible place.
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|cases| *cases > 0)
        .unwrap_or(DEFAULT_CASES);
    ProptestConfig {
        // The default would write `tests/properties.proptest-regressions`
        // (no `src/` to anchor on); pin the documented location instead.
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/proptest-regressions/properties.txt"
        )))),
        ..ProptestConfig::with_cases(cases)
    }
}

fn first_difference(expected: &[u8], actual: &[u8]) -> usize {
    expected
        .iter()
        .zip(actual)
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| expected.len().min(actual.len()))
}

fn check_spans(document: &EnvDocument, input_len: usize) -> Result<(), String> {
    let mut covered_until = document.body_start();
    for (index, node) in document.nodes().iter().enumerate() {
        match node.origin() {
            NodeOrigin::Source(span) => {
                if span.start != covered_until || span.end <= span.start {
                    return Err(format!(
                        "node {index}: span {}..{} breaks contiguity at {covered_until}",
                        span.start, span.end
                    ));
                }
                covered_until = span.end;
            }
            NodeOrigin::Synthetic(_) => return Err(format!("node {index} is synthetic")),
        }
    }
    if covered_until != input_len {
        return Err(format!(
            "spans end at {covered_until}, input has {input_len}"
        ));
    }
    Ok(())
}

/// Checks P1 to P3 for one input, returning a content-free message on failure.
fn check_properties(input: &[u8]) -> Result<(), String> {
    let bom_len = if input.starts_with(BOM) { BOM.len() } else { 0 };
    let body = &input[bom_len..];
    let result = parse(input.to_vec());

    let document = match (std::str::from_utf8(body), result) {
        (Ok(_), Ok(document)) => document,
        (Ok(_), Err(_)) => return Err(format!("valid UTF-8 ({} bytes) was rejected", input.len())),
        (Err(error), Ok(_)) => {
            return Err(format!(
                "invalid UTF-8 at {} accepted ({} bytes)",
                bom_len + error.valid_up_to(),
                input.len()
            ));
        }
        (Err(error), Err(DomainError::InvalidUtf8 { byte_offset })) => {
            let expected = bom_len + error.valid_up_to();
            return if byte_offset == expected {
                Ok(())
            } else {
                Err(format!(
                    "invalid UTF-8 offset {byte_offset}, expected {expected}"
                ))
            };
        }
        (Err(_), Err(_)) => return Err("invalid UTF-8 produced a different error".to_owned()),
    };

    let output = document.serialize();
    if output != input {
        return Err(format!(
            "round trip mismatch: first difference at offset {} (input {} bytes, output {} bytes)",
            first_difference(input, &output),
            input.len(),
            output.len()
        ));
    }
    document
        .check_invariants()
        .map_err(|violation| format!("invariant violated: {violation}"))?;
    check_spans(&document, input.len())?;
    // Exercised for panics only; the value is a line number, not content.
    let _ = document.unclosed_quote_line();

    match parse(input.to_vec()) {
        Ok(second) if second == document => Ok(()),
        _ => Err(format!(
            "parse is not deterministic ({} bytes)",
            input.len()
        )),
    }
}

fn with_optional_bom(bytes: Vec<u8>, has_bom: bool) -> Vec<u8> {
    if has_bom {
        let mut out = BOM.to_vec();
        out.extend(bytes);
        out
    } else {
        bytes
    }
}

fn arbitrary_bytes() -> impl Strategy<Value = Vec<u8>> {
    (vec(any::<u8>(), 0..512), any::<bool>())
        .prop_map(|(bytes, has_bom)| with_optional_bom(bytes, has_bom))
}

fn value_fragment() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        3 => Just("a"),
        1 => Just("\\\\"),
        1 => Just("\\\""),
        1 => Just("\\n"),
        1 => Just("\\"),
        1 => Just("é"),
        1 => Just("世"),
        1 => Just("😀"),
        1 => Just(" # "),
        1 => Just("#"),
        1 => Just("'"),
        1 => Just("\""),
        1 => Just("\n"),
        1 => Just("\r\n"),
        1 => Just("\r"),
        1 => Just("\0"),
    ]
}

fn fragments() -> impl Strategy<Value = String> {
    vec(value_fragment(), 0..6).prop_map(|parts| parts.concat())
}

fn key() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => "[A-Za-z_][A-Za-z0-9_.-]{0,8}",
        1 => "[0-9][a-z]{0,3}",
        1 => Just(String::new()),
        1 => Just("a b".to_owned()),
        1 => Just("é世".to_owned()),
        1 => Just("a\0b".to_owned()),
        1 => Just("export".to_owned()),
    ]
}

fn value() -> impl Strategy<Value = String> {
    let tail = prop_oneof![
        3 => Just(""),
        1 => Just(" # c"),
        1 => Just("#c"),
        1 => Just(" x"),
        1 => Just("zz"),
    ];
    prop_oneof![
        3 => "[a-zA-Z0-9_/:. é世😀]{0,10}",
        1 => "[a-z]{1,4}[ \t]+#[a-z ]{0,4}",
        1 => "[a-z]{1,4}#[a-z]{0,4}",
        3 => (fragments(), tail.clone()).prop_map(|(body, tail)| format!("'{body}'{tail}")),
        4 => (fragments(), tail.clone()).prop_map(|(body, tail)| format!("\"{body}\"{tail}")),
        2 => fragments().prop_map(|body| format!("'{body}")),
        2 => fragments().prop_map(|body| format!("\"{body}")),
        1 => Just("\"\\\\\r\n".to_owned()),
    ]
}

fn line() -> impl Strategy<Value = String> {
    let export = prop_oneof![
        4 => Just(""),
        2 => Just("export "),
        1 => Just("export\t"),
        1 => Just("  "),
        1 => Just("exportX "),
    ];
    prop_oneof![
        2 => "[ \t]{0,4}",
        2 => "[ \t]{0,3}#[ -~é世😀]{0,12}",
        8 => (export, key(), "[ \t]{0,2}", "[ \t]{0,2}", value())
            .prop_map(|(export, key, before, after, value)| format!("{export}{key}{before}={after}{value}")),
        1 => Just("\0".to_owned()),
        1 => Just("K=\0v".to_owned()),
        1 => "[ -~é世😀]{0,10}",
    ]
}

fn terminator() -> impl Strategy<Value = &'static str> {
    prop_oneof![3 => Just("\n"), 1 => Just("\r\n"), 1 => Just("\r")]
}

fn dotenv_document() -> impl Strategy<Value = Vec<u8>> {
    (
        vec((line(), terminator()), 0..12),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(lines, is_last_terminated, has_bom)| {
            let count = lines.len();
            let mut out = String::new();
            for (index, (content, term)) in lines.into_iter().enumerate() {
                out.push_str(&content);
                if is_last_terminated || index + 1 < count {
                    out.push_str(term);
                }
            }
            with_optional_bom(out.into_bytes(), has_bom)
        })
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn arbitrary_bytes_hold_properties(input in arbitrary_bytes()) {
        if let Err(message) = check_properties(&input) {
            prop_assert!(false, "{}", message);
        }
    }

    #[test]
    fn dotenv_documents_hold_properties(input in dotenv_document()) {
        // The generator only emits valid UTF-8, so the Ok path must be taken.
        prop_assert!(std::str::from_utf8(&input[if input.starts_with(BOM) { 3 } else { 0 }..]).is_ok(), "generator emitted invalid UTF-8");
        if let Err(message) = check_properties(&input) {
            prop_assert!(false, "{}", message);
        }
    }
}

const STYLES: [QuoteStyle; 3] = [QuoteStyle::Unquoted, QuoteStyle::Single, QuoteStyle::Double];
const TERMINATORS: [LineTerminator; 3] =
    [LineTerminator::Lf, LineTerminator::CrLf, LineTerminator::Cr];

/// Node templates for `encode_in_node`: unquoted with and without a trailing
/// comment, single, double, multiline double and export.
const NODE_TEMPLATES: &[&str] = &[
    "K=a\n",
    "K=a # c\n",
    "K= # c\n",
    "K='a'\n",
    "K=\"a\"\n",
    "K=\"a\r\nb\" # c\r\n",
    "K='a\nb'",
    "export K=\"a\"\n",
    "export K = a\t# c\r\n",
];

fn codec_value() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => fragments(),
        // Arbitrary Unicode but at most 16 chars, so it cannot hold the
        // 23-char sentinel.
        1 => "[\\s\\S]{0,16}",
        1 => "[a-z ]{0,6}",
    ]
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn encode_then_decode_returns_the_value(v in codec_value()) {
        for style in STYLES {
            for dominant in TERMINATORS {
                for is_multiline in [false, true] {
                    let ctx = EncodeContext { dominant, is_multiline };
                    match encode(style, &v, ctx) {
                        Ok(encoded) => prop_assert!(
                            decode(style, &encoded) == v,
                            "decode(encode(v)) differs ({style:?}, value of {} bytes)",
                            v.len()
                        ),
                        Err(_) => prop_assert!(
                            style != QuoteStyle::Double,
                            "double must never need conversion"
                        ),
                    }
                }
            }
        }
    }

    #[test]
    fn encode_in_node_ok_reparses_to_the_value(
        v in codec_value(),
        template in 0..NODE_TEMPLATES.len(),
        dominant in 0..TERMINATORS.len(),
    ) {
        let node = NODE_TEMPLATES[template];
        let document = parse(node.as_bytes().to_vec()).unwrap();
        let NodeKind::Assignment(syntax) = document.nodes()[0].kind() else {
            return Err(TestCaseError::fail(format!("template {template} is not an assignment")));
        };
        let result = encode_in_node(node, syntax, &v, TERMINATORS[dominant]);
        // Double escapes everything; only NUL (which makes the line Raw) can fail.
        if syntax.quote == QuoteStyle::Double {
            prop_assert!(
                result.is_ok() || v.contains('\0'),
                "template {template}: double rejected a representable value ({} bytes)",
                v.len()
            );
        }
        if let Ok(edited) = result {
            let reparsed = parse(edited.clone().into_bytes()).unwrap();
            prop_assert!(reparsed.nodes().len() == 1, "template {template}: node count");
            let NodeKind::Assignment(new_syntax) = reparsed.nodes()[0].kind() else {
                return Err(TestCaseError::fail(format!("template {template}: not an assignment")));
            };
            prop_assert!(new_syntax.quote == syntax.quote, "template {template}: quote style");
            let raw = edited.get(new_syntax.value_range.clone()).unwrap();
            prop_assert!(
                decode(new_syntax.quote, raw) == v,
                "template {template}: value differs ({} bytes)",
                v.len()
            );
        }
    }
}

/// Explicit regression list. Add every counterexample found by proptest here
/// as well as in `proptest-regressions/properties.txt` (which is automatic).
#[test]
fn regression_cases() {
    let cases: &[&[u8]] = &[
        b"",
        b"\xEF\xBB\xBF",
        b"\xEF\xBB\xBF\xEF\xBB\xBF",
        b"\r",
        b"\r\r\n",
        b"\n\r\n\r",
        b"K=\"a\\\\\r\n\"",
        b"K=\"a\\\\\r\n",
        b"K=\"\\",
        b"K=\"\\\n",
        b"K=\"\\\r\n",
        b"K=\"a\\\r\nb\"\n",
        b"\0",
        b"\0\n\0\r\n\0",
        b"K=\0v\n",
        b"K=\"v\0\"\n",
        b"K=\"unclosed",
        b"K='unclosed\nL=1\n",
        b"K=\"a\"garbage\n",
        b"K='a' #c\r",
        b"export K=1\nexport\tL=2\r\nexport",
        b"export =1\n",
        b"  # comment\n \t\n",
        b"K=a#b\nL=a #b\n",
        "K=é世😀\n".as_bytes(),
        b"\xEF\xBB\xBFK=\xFF\n",
        b"K=\xC3",
        b"\xFF",
        b"\xEF\xBB",
    ];
    for (index, case) in cases.iter().enumerate() {
        if let Err(message) = check_properties(case) {
            panic!("regression case {index}: {message}");
        }
    }
}
