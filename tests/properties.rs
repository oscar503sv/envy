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
use envy::domain::edit::{
    ConversionPolicy, EditError, SetValueOutcome, insert_assignment, remove_assignment, set_value,
};
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

proptest! {
    #![proptest_config(config())]

    #[test]
    fn set_value_applies_only_inside_the_node(
        v in codec_value(),
        template in 0..NODE_TEMPLATES.len(),
        should_convert in any::<bool>(),
    ) {
        const BEFORE: &str = "# lead\r\nX=1\n";
        const AFTER: &str = "\nY='2'\r\n# tail";
        let node = NODE_TEMPLATES[template];
        // Templates without a terminator are only valid as the last node.
        let (head, tail) = if node.ends_with(['\n', '\r']) {
            (BEFORE, AFTER)
        } else {
            (BEFORE, "")
        };
        let source = format!("{head}{node}{tail}");
        let mut doc = parse(source.clone().into_bytes()).unwrap();
        // `# lead` and `X=1` come first, so the template is node 2.
        let index = 2;
        let id = doc.nodes()[index].id();
        let policy = if should_convert { ConversionPolicy::ConvertToDouble } else { ConversionPolicy::Reject };
        let result = set_value(&mut doc, id, &v, policy);
        if v.contains('\0') {
            prop_assert!(result == Err(EditError::Unrepresentable), "NUL must be unrepresentable");
        }
        if should_convert && !v.contains('\0') {
            prop_assert!(result.is_ok(), "template {template}: conversion must succeed");
        }
        prop_assert!(doc.check_invariants().is_ok(), "template {template}: invariants");
        let out = doc.serialize();
        match result {
            Ok(SetValueOutcome::Applied(_)) => {
                let reparsed = parse(out.clone()).unwrap();
                prop_assert!(reparsed.nodes().len() == doc.nodes().len(), "node count");
                prop_assert!(out.starts_with(head.as_bytes()), "bytes before the node changed");
                prop_assert!(out.ends_with(tail.as_bytes()), "bytes after the node changed");
                let NodeKind::Assignment(syntax) = reparsed.nodes()[index].kind() else {
                    return Err(TestCaseError::fail("edited node is not an assignment"));
                };
                let bytes = reparsed.current_bytes(&reparsed.nodes()[index]).unwrap();
                let text = std::str::from_utf8(bytes).unwrap();
                let raw = text.get(syntax.value_range.clone()).unwrap();
                prop_assert!(decode(syntax.quote, raw) == v, "template {template}: value differs");
            }
            Ok(SetValueOutcome::Unchanged) | Err(_) => {
                prop_assert!(out == source.as_bytes(), "document changed without an edit");
            }
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

proptest! {
    #![proptest_config(config())]

    #[test]
    fn inserting_and_removing_an_assignment_restores_the_template(
        name in "[A-Za-z_][A-Za-z0-9_]{0,8}",
        value in codec_value(),
        template in prop::sample::select(vec!["A=1", "# comment", "A=1\r", "A=1\r\nB=2", "A=1\n"]),
    ) {
        prop_assume!(!value.contains('\0'));
        let mut doc = parse(template.as_bytes().to_vec()).expect("template parses");
        insert_assignment(&mut doc, &name, &value, ConversionPolicy::ConvertToDouble)
            .expect("valid assignment inserts");
        let reparsed = parse(doc.serialize()).expect("serialization parses");
        let last = reparsed.nodes().last().expect("appended node");
        let NodeKind::Assignment(syntax) = last.kind() else {
            panic!("append did not produce an assignment");
        };
        let bytes = reparsed.current_bytes(last).expect("appended bytes");
        let text = std::str::from_utf8(bytes).expect("UTF-8");
        prop_assert!(syntax.key.expose() == &name, "key mismatch");
        prop_assert!(decode(syntax.quote, text.get(syntax.value_range.clone()).expect("value range")) == value, "value mismatch");

        let created_id = doc.nodes().last().expect("appended node").id();
        remove_assignment(&mut doc, created_id).expect("remove inserted assignment");
        prop_assert!(doc.serialize() == template.as_bytes(), "template bytes changed");
        prop_assert!(doc.check_invariants().is_ok(), "invalid document after removal");
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn history_round_trips_random_edit_sequences(
        template_index in 0_usize..5,
        steps in vec((0_u8..8, any::<usize>(), 0_usize..5, 1_usize..=3), 1..32),
    ) {
        use envy::domain::history::{EditHistory, EditRequest};
        use envy::domain::redact::Redacted;

        let templates = [
            include_bytes!("fixtures/no-trailing-newline.env").as_slice(),
            include_bytes!("fixtures/plain.env").as_slice(),
            b"\xef\xbb\xbf# note\r\nA='a'\r\n\r\nB=2",
            b"opaque\r\nA=1",
            b"",
        ];
        let input = templates[template_index];
        let mut history = EditHistory::new(parse(input.to_vec()).unwrap()).unwrap();
        let values = ["", "plain", "it's quoted", "multi\nline", "é世"];
        let policy = ConversionPolicy::ConvertToDouble;
        let mut mirror = vec![input.to_vec()];
        let mut cursor = 0_usize;
        let mut saved = input.to_vec();
        let check = |history: &EditHistory, expected: &[u8], saved: &[u8]| {
            prop_assert!(history.document().check_invariants().is_ok());
            let actual = history.document().serialize();
            prop_assert!(actual == expected, "history bytes differ from mirror");
            prop_assert_eq!(history.is_dirty(), actual != saved);
            Ok(())
        };

        for (action, selector, value_index, batch_size) in steps {
            let document = history.document().clone();
            let ids: Vec<_> = document.nodes().iter()
                .filter(|node| !node.is_removed() && matches!(node.kind(), NodeKind::Assignment(_)))
                .map(|node| node.id())
                .collect();
            let value = values[value_index];
            let mut expected = document.clone();
            let edit = match action {
                0 if !ids.is_empty() => {
                    let node_id = ids[selector % ids.len()];
                    let changed = matches!(set_value(&mut expected, node_id, value, policy).unwrap(),
                        SetValueOutcome::Applied(_));
                    Some((history.apply(EditRequest::SetValue {
                        node_id, value: Redacted::new(value), policy,
                    }), changed))
                }
                1 => {
                    insert_assignment(&mut expected, "K", value, policy).unwrap();
                    Some((history.apply(EditRequest::InsertAssignment {
                        key: Redacted::new("K"), value: Redacted::new(value), policy,
                    }), true))
                }
                2 if !ids.is_empty() => {
                    let node_id = ids[selector % ids.len()];
                    remove_assignment(&mut expected, node_id).unwrap();
                    Some((history.apply(EditRequest::RemoveAssignment { node_id }), true))
                }
                3 => {
                    prop_assert_eq!(history.undo().unwrap(), cursor > 0);
                    cursor = cursor.saturating_sub(1);
                    None
                }
                4 => {
                    let can_redo = cursor + 1 < mirror.len();
                    prop_assert_eq!(history.redo().unwrap(), can_redo);
                    cursor += usize::from(can_redo);
                    None
                }
                5 => {
                    history.mark_saved();
                    saved.clone_from(&mirror[cursor]);
                    None
                }
                6 | 7 => {
                    let requests = (0..batch_size).map(|index| {
                        let key = if action == 7 && index + 1 == batch_size { "" } else { "K" };
                        EditRequest::InsertAssignment {
                            key: Redacted::new(key), value: Redacted::new(value), policy,
                        }
                    });
                    let result = history.apply_batch(requests);
                    if action == 7 {
                        prop_assert!(result.is_err());
                        prop_assert!(history.document() == &document, "failed batch changed document");
                        None
                    } else {
                        for _ in 0..batch_size {
                            insert_assignment(&mut expected, "K", value, policy).unwrap();
                        }
                        Some((result, true))
                    }
                }
                _ => None,
            };
            if let Some((result, changed)) = edit {
                prop_assert_eq!(result.expect("valid edit").is_some(), changed);
                if changed {
                    mirror.truncate(cursor + 1);
                    mirror.push(expected.serialize());
                    cursor += 1;
                }
            }
            check(&history, &mirror[cursor], &saved)?;
            prop_assert_eq!(history.can_undo(), cursor > 0);
            prop_assert_eq!(history.can_redo(), cursor + 1 < mirror.len());
        }

        for expected in &mirror[cursor + 1..] {
            prop_assert!(history.redo().unwrap());
            check(&history, expected, &saved)?;
        }
        prop_assert!(!history.redo().unwrap());
        for expected in mirror[..mirror.len() - 1].iter().rev() {
            prop_assert!(history.undo().unwrap());
            check(&history, expected, &saved)?;
        }
        prop_assert!(!history.undo().unwrap());
        check(&history, input, &saved)?;
        for expected in &mirror[1..] {
            prop_assert!(history.redo().unwrap());
            check(&history, expected, &saved)?;
        }
        prop_assert!(!history.redo().unwrap());
        check(&history, mirror.last().unwrap(), &saved)?;
    }
}
