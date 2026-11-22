//! Line classifier that turns dotenv bytes into an [`EnvDocument`].
//!
//! Implements the ordered algorithm C0-C3 of `docs/SYNTAX.md` §3 and the value
//! anatomy of §4/§5, including multi-line quoted values and unclosed quotes.
//! Nothing is expanded or
//! executed: `$VAR`, `${VAR}` and `$(cmd)` are ordinary bytes.

use std::ops::Range;

use super::{
    document::{EnvDocument, NodeKind},
    error::{DomainError, LineNumber},
    redact::Redacted,
    syntax::{AssignmentSyntax, LineTerminator, QuoteStyle, check_utf8, split_lines},
};

const EXPORT_KEYWORD: &[u8] = b"export";

/// Parses `source` into a byte-preserving document.
///
/// Each node covers one physical line, except quoted values that continue on
/// later lines (SYNTAX §5.4) and unclosed quotes, which turn the rest of the
/// file into one `Raw` node (§5.6). Node spans partition the body (everything
/// after the BOM) without gaps.
///
/// # Errors
///
/// Returns [`DomainError::InvalidUtf8`] when the content after the BOM is not
/// valid UTF-8. The caller is expected to open the file in protected mode.
pub fn parse(source: Vec<u8>) -> Result<EnvDocument, DomainError> {
    let mut document = EnvDocument::new(source);
    check_utf8(document.source(), document.body_start())?;

    let (nodes, unclosed_quote) = build_nodes(document.source(), document.body_start());
    for (span, kind, terminator) in nodes {
        document.push_source_node(span, kind, terminator);
    }
    document.set_unclosed_quote(unclosed_quote);
    Ok(document)
}

type PendingNode = (Range<usize>, NodeKind, Option<LineTerminator>);

/// Walks the physical lines once; every byte is scanned a bounded number of
/// times because a node consumes all the lines it covers and an unclosed quote
/// ends the walk.
fn build_nodes(source: &[u8], body_start: usize) -> (Vec<PendingNode>, Option<LineNumber>) {
    let lines = split_lines(source, body_start);
    // Upper bound: every node covers at least one line.
    let mut nodes = Vec::with_capacity(lines.len());
    let mut index = 0;
    while let Some(line) = lines.get(index) {
        let content = source
            .get(line.span.start..line.content_end)
            .unwrap_or_default();
        let start = match classify_line(content) {
            Classified::Kind(kind) => {
                nodes.push((line.span.clone(), kind, line.terminator));
                index = index.saturating_add(1);
                continue;
            }
            Classified::Quoted(start) => start,
        };

        let open = line.span.start.saturating_add(start.open);
        let closing = find_closing_quote(source, open, start.style).and_then(|close| {
            lines
                .iter()
                .enumerate()
                .skip(index)
                .find(|(_, candidate)| close < candidate.span.end)
                .map(|(end_index, end_line)| (close, end_index, end_line))
        });
        let Some((close, end_index, end_line)) = closing else {
            let end = lines.last().map_or(line.span.end, |last| last.span.end);
            let terminator = lines.last().and_then(|last| last.terminator);
            nodes.push((line.span.start..end, NodeKind::Raw, terminator));
            return (nodes, LineNumber::new(index.saturating_add(1)));
        };

        let end_content = source
            .get(end_line.span.start..end_line.content_end)
            .unwrap_or_default();
        let after_close = close.saturating_sub(end_line.span.start).saturating_add(1);
        // C0 covers every physical line of the node, continuation lines
        // included: a NUL anywhere keeps the whole span out of editing.
        let has_nul = source
            .get(line.span.start..end_line.content_end)
            .is_none_or(|bytes| bytes.contains(&0));
        let is_valid = !has_nul && is_valid_after_close(end_content, after_close);
        let kind = if is_valid {
            let value = open.saturating_add(1)..close;
            let is_multiline = source
                .get(value)
                .is_some_and(|raw| raw.iter().any(|byte| matches!(byte, b'\n' | b'\r')));
            NodeKind::Assignment(AssignmentSyntax {
                key: start.key,
                value_range: start.open.saturating_add(1)..close.saturating_sub(line.span.start),
                quote: start.style,
                is_multiline,
                is_export: start.is_export,
            })
        } else {
            NodeKind::Raw
        };
        nodes.push((
            line.span.start..end_line.span.end,
            kind,
            end_line.terminator,
        ));
        index = end_index.saturating_add(1);
    }
    (nodes, None)
}

fn is_ws(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

fn at(content: &[u8], index: usize) -> Option<u8> {
    content.get(index).copied()
}

fn skip_ws(content: &[u8], from: usize) -> usize {
    let mut index = from;
    while at(content, index).is_some_and(is_ws) {
        index = index.saturating_add(1);
    }
    index
}

/// Moves `end` back over trailing blanks without crossing `start`.
fn trim_end_ws(content: &[u8], start: usize, end: usize) -> usize {
    let mut end = end;
    while end > start
        && end
            .checked_sub(1)
            .and_then(|last| at(content, last))
            .is_some_and(is_ws)
    {
        end = end.saturating_sub(1);
    }
    end
}

/// Outcome of classifying the first physical line of a node.
enum Classified {
    Kind(NodeKind),
    /// A quoted assignment whose close is searched across the whole source.
    Quoted(QuotedStart),
}

struct QuotedStart {
    key: Redacted<String>,
    is_export: bool,
    style: QuoteStyle,
    /// Offset of the opening quote relative to the line start.
    open: usize,
}

/// Classifies one physical line (terminator excluded): SYNTAX §3, C0-C3.
fn classify_line(content: &[u8]) -> Classified {
    if content.contains(&0) {
        return Classified::Kind(NodeKind::Raw);
    }
    let first = skip_ws(content, 0);
    match at(content, first) {
        None => Classified::Kind(NodeKind::Blank),
        Some(b'#') => Classified::Kind(NodeKind::Comment),
        Some(_) => parse_assignment(content).unwrap_or(Classified::Kind(NodeKind::Raw)),
    }
}

struct Header {
    key: Range<usize>,
    is_export: bool,
    equals: usize,
}

/// Matches `KEY WS* '='` at `from`; ASCII-only key per SYNTAX §1.
fn match_key(content: &[u8], from: usize) -> Option<(Range<usize>, usize)> {
    let first = at(content, from)?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let length = content
        .get(from..)?
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'_')
        .count();
    let key_end = from.saturating_add(length);
    let equals = skip_ws(content, key_end);
    (at(content, equals) == Some(b'=')).then_some((from..key_end, equals))
}

/// `WS* ( 'export' WS+ )? KEY WS* '='`, retrying without `export` so that
/// `export=1` is the key `export` (SYNTAX §3).
fn match_header(content: &[u8]) -> Option<Header> {
    let start = skip_ws(content, 0);
    let after_keyword = start.saturating_add(EXPORT_KEYWORD.len());
    let has_export_prefix = content
        .get(start..)
        .is_some_and(|rest| rest.starts_with(EXPORT_KEYWORD))
        && at(content, after_keyword).is_some_and(is_ws);
    if has_export_prefix {
        let key_start = skip_ws(content, after_keyword);
        if let Some((key, equals)) = match_key(content, key_start) {
            return Some(Header {
                key,
                is_export: true,
                equals,
            });
        }
    }
    match_key(content, start).map(|(key, equals)| Header {
        key,
        is_export: false,
        equals,
    })
}

fn parse_assignment(content: &[u8]) -> Option<Classified> {
    let header = match_header(content)?;
    let after_equals = header.equals.saturating_add(1);
    let value_start = skip_ws(content, after_equals);
    let key = Redacted::new(
        std::str::from_utf8(content.get(header.key)?)
            .ok()?
            .to_owned(),
    );

    let style = match at(content, value_start) {
        Some(b'"') => QuoteStyle::Double,
        Some(b'\'') => QuoteStyle::Single,
        _ => {
            return Some(Classified::Kind(NodeKind::Assignment(AssignmentSyntax {
                key,
                value_range: unquoted_value_range(content, after_equals, value_start),
                quote: QuoteStyle::Unquoted,
                is_multiline: false,
                is_export: header.is_export,
            })));
        }
    };
    Some(Classified::Quoted(QuotedStart {
        key,
        is_export: header.is_export,
        style,
        open: value_start,
    }))
}

/// Value of an unquoted assignment (SYNTAX §5.1, Q-12).
fn unquoted_value_range(content: &[u8], after_equals: usize, value_start: usize) -> Range<usize> {
    // A '#' starts the trailing comment only when a blank precedes it, and that
    // blank may be the last one of the run after '='.
    let comment_start = (value_start..content.len()).find(|&index| {
        at(content, index) == Some(b'#')
            && index > after_equals
            && index
                .checked_sub(1)
                .and_then(|previous| at(content, previous))
                .is_some_and(is_ws)
    });
    match comment_start {
        Some(hash) if hash == value_start => after_equals..after_equals,
        Some(hash) => value_start..trim_end_ws(content, value_start, hash),
        None => value_start..trim_end_ws(content, value_start, content.len()),
    }
}

/// Absolute index of the closing quote after the opening one at `open`.
///
/// Single closes at the first `'`. Double lets a backslash consume the next
/// character, where CRLF counts as one, so `\"` and `\` + terminator never
/// close (SYNTAX §5.2, §5.3). Terminators are ordinary bytes here.
fn find_closing_quote(source: &[u8], open: usize, style: QuoteStyle) -> Option<usize> {
    let mut index = open.saturating_add(1);
    while let Some(byte) = at(source, index) {
        match (style, byte) {
            (QuoteStyle::Single, b'\'') | (QuoteStyle::Double, b'"') => return Some(index),
            (QuoteStyle::Double, b'\\') => {
                let is_crlf = at(source, index.saturating_add(1)) == Some(b'\r')
                    && at(source, index.saturating_add(2)) == Some(b'\n');
                index = index.saturating_add(if is_crlf { 3 } else { 2 });
            }
            _ => index = index.saturating_add(1),
        }
    }
    None
}

/// SYNTAX §5.5: nothing, blanks, or blanks followed by `#` comment.
fn is_valid_after_close(content: &[u8], from: usize) -> bool {
    let first_non_ws = skip_ws(content, from);
    match at(content, first_non_ws) {
        None => true,
        Some(b'#') => first_non_ws > from,
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::domain::{
        document::{EnvDocument, NodeKind, NodeOrigin},
        error::DomainError,
        syntax::{LineTerminator, QuoteStyle},
    };
    use crate::test_support::SENTINEL;

    fn parse_ok(input: &[u8]) -> EnvDocument {
        let doc = parse(input.to_vec()).expect("valid UTF-8");
        assert!(doc.check_invariants().is_ok(), "invariants must hold");
        doc
    }

    fn node_bytes(doc: &EnvDocument, index: usize) -> &[u8] {
        let node = &doc.nodes()[index];
        doc.current_bytes(node).expect("node bytes")
    }

    fn kinds(input: &[u8]) -> Vec<&'static str> {
        parse_ok(input)
            .nodes()
            .iter()
            .map(|node| match node.kind() {
                NodeKind::Assignment(_) => "assignment",
                NodeKind::Comment => "comment",
                NodeKind::Blank => "blank",
                NodeKind::Raw => "raw",
            })
            .collect()
    }

    /// Parses one line and checks key, quote, export flag and exact value bytes.
    fn expect_assignment(
        input: &[u8],
        key: &str,
        quote: QuoteStyle,
        is_export: bool,
        value: &[u8],
    ) {
        let doc = parse_ok(input);
        assert!(doc.nodes().len() == 1, "exactly one node expected");
        let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
            panic!("expected an assignment");
        };
        assert!(syntax.key.expose() == key, "key mismatch");
        assert!(syntax.quote == quote, "quote mismatch");
        assert!(syntax.is_export == is_export, "export mismatch");
        assert!(!syntax.is_multiline);
        assert!(
            &node_bytes(&doc, 0)[syntax.value_range.clone()] == value,
            "value bytes mismatch"
        );
    }

    fn unq(input: &[u8], key: &str, value: &[u8]) {
        expect_assignment(input, key, QuoteStyle::Unquoted, false, value);
    }

    #[test]
    fn plain_assignment() {
        unq(b"A=1\n", "A", b"1");
        unq(b"A=b=c\n", "A", b"b=c");
        unq(b"_k9=v\n", "_k9", b"v");
    }

    #[test]
    fn spaces_and_tabs_around_equals() {
        unq(b"  KEY \t=  v\n", "KEY", b"v");
        unq(b"KEY\t=\tv", "KEY", b"v");
    }

    #[test]
    fn export_variants() {
        expect_assignment(b"export KEY=v\n", "KEY", QuoteStyle::Unquoted, true, b"v");
        expect_assignment(b"export\tKEY=v\n", "KEY", QuoteStyle::Unquoted, true, b"v");
        expect_assignment(
            b"  export   KEY = v\n",
            "KEY",
            QuoteStyle::Unquoted,
            true,
            b"v",
        );
        expect_assignment(
            b"export export=1\n",
            "export",
            QuoteStyle::Unquoted,
            true,
            b"1",
        );
        expect_assignment(b"export=1\n", "export", QuoteStyle::Unquoted, false, b"1");
        // `export WS+` followed by `=` only matches through the retry without
        // the keyword, so these pin that retry (SYNTAX §3).
        expect_assignment(b"export =1\n", "export", QuoteStyle::Unquoted, false, b"1");
        expect_assignment(b"export\t=1\n", "export", QuoteStyle::Unquoted, false, b"1");
        expect_assignment(
            b"  export =1\n",
            "export",
            QuoteStyle::Unquoted,
            false,
            b"1",
        );
        expect_assignment(
            b"exportKEY=1\n",
            "exportKEY",
            QuoteStyle::Unquoted,
            false,
            b"1",
        );
        expect_assignment(b"export K=\"a\"\n", "K", QuoteStyle::Double, true, b"a");
    }

    #[test]
    fn malformed_export_is_raw() {
        for input in [
            &b"EXPORT KEY=1\n"[..],
            b"export KEY\n",
            b"export\n",
            b"export  \n",
        ] {
            assert!(kinds(input) == ["raw"]);
        }
    }

    #[test]
    fn unquoted_internal_and_trailing_whitespace() {
        unq(b"K=a  b  \n", "K", b"a  b");
        unq(b"K=a\t\t\n", "K", b"a");
        unq(b"K=ab\"cd\n", "K", b"ab\"cd");
        unq(b"K=it's\n", "K", b"it's");
    }

    #[test]
    fn hash_without_preceding_blank_belongs_to_value() {
        unq(b"PASS=ab#cd\n", "PASS", b"ab#cd");
        unq(b"URL=https://x/#frag\n", "URL", b"https://x/#frag");
        unq(b"KEY=#x\n", "KEY", b"#x");
    }

    #[test]
    fn trailing_comment_after_space_or_tab() {
        unq(b"K=value # n\n", "K", b"value");
        unq(b"K=a\t# c\n", "K", b"a");
        unq(b"K=a #b #c\n", "K", b"a");
        unq(b"K=a  \t # c\n", "K", b"a");
    }

    #[test]
    fn empty_values() {
        unq(b"KEY=\n", "KEY", b"");
        unq(b"KEY=", "KEY", b"");
        unq(b"KEY=   \n", "KEY", b"");
    }

    #[test]
    fn empty_value_with_comment_puts_blanks_in_suffix() {
        for (input, empty_at) in [
            (&b"KEY= # n\n"[..], 4),
            (b"KEY=\t# n\n", 4),
            (b"KEY=  #n\n", 4),
        ] {
            let doc = parse_ok(input);
            let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
                panic!("expected an assignment");
            };
            assert!(syntax.value_range == (empty_at..empty_at));
        }
    }

    #[test]
    fn hash_right_after_equals_with_blank_before_value() {
        unq(b"K= #x\n", "K", b"");
    }

    #[test]
    fn single_quoted() {
        expect_assignment(b"K='a b'\n", "K", QuoteStyle::Single, false, b"a b");
        expect_assignment(b"K=''\n", "K", QuoteStyle::Single, false, b"");
        expect_assignment(b"K= 'x' \n", "K", QuoteStyle::Single, false, b"x");
        expect_assignment(
            b"K='#x $HOME'\n",
            "K",
            QuoteStyle::Single,
            false,
            b"#x $HOME",
        );
    }

    #[test]
    fn backslash_is_literal_in_single() {
        expect_assignment(
            b"K='C:\\dir\\x'\n",
            "K",
            QuoteStyle::Single,
            false,
            b"C:\\dir\\x",
        );
        expect_assignment(b"K='a\\'\n", "K", QuoteStyle::Single, false, b"a\\");
    }

    #[test]
    fn double_quoted() {
        expect_assignment(b"K=\"a b\"\n", "K", QuoteStyle::Double, false, b"a b");
        expect_assignment(b"K=\"\"\n", "K", QuoteStyle::Double, false, b"");
        expect_assignment(b"K=\"a # b\"\n", "K", QuoteStyle::Double, false, b"a # b");
    }

    #[test]
    fn quoted_with_trailing_comment() {
        expect_assignment(b"K=\"a\" # n\n", "K", QuoteStyle::Double, false, b"a");
        expect_assignment(b"K='a'\t# n\n", "K", QuoteStyle::Single, false, b"a");
        expect_assignment(b"K=\"a\"   \n", "K", QuoteStyle::Double, false, b"a");
    }

    #[test]
    fn escaped_quote_and_backslash_in_double() {
        expect_assignment(b"K=\"a\\\"b\"\n", "K", QuoteStyle::Double, false, b"a\\\"b");
        expect_assignment(b"K=\"a\\\\\"\n", "K", QuoteStyle::Double, false, b"a\\\\");
        expect_assignment(
            b"K=\"a\\qb\\$\"\n",
            "K",
            QuoteStyle::Double,
            false,
            b"a\\qb\\$",
        );
        expect_assignment(
            b"K=\"\\\\\\\"\"\n",
            "K",
            QuoteStyle::Double,
            false,
            b"\\\\\\\"",
        );
    }

    #[test]
    fn garbage_after_closing_quote_is_raw() {
        for input in [
            &b"KEY=\"a\"b\n"[..],
            b"KEY=\"a\" b\n",
            b"KEY=\"a\"#c\n",
            b"KEY='a''b'\n",
            b"KEY='a'b\n",
        ] {
            assert!(kinds(input) == ["raw"]);
        }
    }

    #[test]
    fn invalid_keys_are_raw() {
        for input in [
            &b"1A=x\n"[..],
            b"A.B=x\n",
            b"A-B=x\n",
            b"A B=x\n",
            b"=x\n",
            " =x\n".as_bytes(),
            "claveñ=1\n".as_bytes(),
            "ñ=1\n".as_bytes(),
            b"KEY\n",
            b"KEY: v\n",
            b"cat <<EOF\n",
        ] {
            assert!(kinds(input) == ["raw"]);
        }
    }

    #[test]
    fn comment_and_blank_lines() {
        assert!(kinds(b"# c\n  # c\n\t#c\n") == ["comment", "comment", "comment"]);
        assert!(kinds(b"\n \t \n   ") == ["blank", "blank", "blank"]);
        // Other Unicode blanks are not WS (Q-02).
        assert!(kinds("\u{a0}\n".as_bytes()) == ["raw"]);
        assert!(kinds(b"\x0b\n") == ["raw"]);
    }

    #[test]
    fn nul_line_is_raw() {
        assert!(kinds(b"A=1\0\n") == ["raw"]);
        assert!(kinds(b"# c\0\n") == ["raw"]);
        assert!(kinds(b"\0") == ["raw"]);
        assert!(kinds(b"A=1\0\nB=2\n") == ["raw", "assignment"]);
    }

    #[test]
    fn expansions_are_literal() {
        unq(b"K=${HOME}/$USER\n", "K", b"${HOME}/$USER");
        unq(b"K=$(id)\n", "K", b"$(id)");
        expect_assignment(b"K=\"${A}\"\n", "K", QuoteStyle::Double, false, b"${A}");
        expect_assignment(b"K='$(id)'\n", "K", QuoteStyle::Single, false, b"$(id)");
    }

    #[test]
    fn unicode_values_are_preserved() {
        unq("K=café 世界\n".as_bytes(), "K", "café 世界".as_bytes());
        expect_assignment(
            "K=\"é # ü\" # ñ\n".as_bytes(),
            "K",
            QuoteStyle::Double,
            false,
            "é # ü".as_bytes(),
        );
    }

    #[test]
    fn terminators_are_preserved_per_node() {
        let doc = parse_ok(b"A=1\r\nB=2\nC=3\rD=4");
        let terminators: Vec<_> = doc.nodes().iter().map(|n| n.terminator()).collect();
        assert!(
            terminators
                == [
                    Some(LineTerminator::CrLf),
                    Some(LineTerminator::Lf),
                    Some(LineTerminator::Cr),
                    None
                ]
        );
        assert!(node_bytes(&doc, 0) == b"A=1\r\n");
        let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
            panic!("expected an assignment");
        };
        assert!(syntax.value_range == (2..3));
    }

    #[test]
    fn bom_document_spans_start_after_bom() {
        let doc = parse_ok(b"\xEF\xBB\xBFA=1\n");
        assert!(doc.has_bom());
        assert!(doc.nodes().len() == 1);
        assert!(doc.nodes()[0].origin() == &NodeOrigin::Source(3..7));
        let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
            panic!("expected an assignment");
        };
        assert!(syntax.key.expose() == "A");
        assert!(syntax.value_range == (2..3));
        assert!(parse_ok(b"\xEF\xBB\xBF").nodes().is_empty());
    }

    #[test]
    fn second_bom_is_ordinary_content() {
        assert!(kinds(b"\xEF\xBB\xBF\xEF\xBB\xBFA=1\n") == ["raw"]);
    }

    #[test]
    fn empty_document_has_no_nodes() {
        let doc = parse_ok(b"");
        assert!(doc.nodes().is_empty());
    }

    #[test]
    fn invalid_utf8_reports_offset() {
        assert!(matches!(
            parse(b"A=1\nB=\xFF\n".to_vec()),
            Err(DomainError::InvalidUtf8 { byte_offset: 6 })
        ));
        assert!(matches!(
            parse(b"\xEF\xBB\xBFA=\x80".to_vec()),
            Err(DomainError::InvalidUtf8 { byte_offset: 5 })
        ));
    }

    #[test]
    fn invalid_utf8_error_carries_no_content() {
        let mut input = SENTINEL.as_bytes().to_vec();
        input.push(0xFF);
        let Err(error) = parse(input) else {
            panic!("expected an error");
        };
        assert!(!error.to_string().contains(SENTINEL));
        assert!(!format!("{error:?}").contains(SENTINEL));
    }

    #[test]
    fn debug_of_parsed_document_hides_content() {
        let doc = parse_ok(format!("KEY_{SENTINEL}={SENTINEL}\n").as_bytes());
        assert!(!format!("{doc:?}").contains(SENTINEL));
    }

    /// Parses input that must be one assignment node and checks its value,
    /// terminator and multi-line flag.
    fn expect_one(
        input: &[u8],
        quote: QuoteStyle,
        value: &[u8],
        is_multiline: bool,
        terminator: Option<LineTerminator>,
    ) {
        let doc = parse_ok(input);
        assert!(doc.nodes().len() == 1, "exactly one node expected");
        let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
            panic!("expected an assignment");
        };
        assert!(syntax.quote == quote, "quote mismatch");
        assert!(syntax.is_multiline == is_multiline, "multiline mismatch");
        assert!(doc.nodes()[0].terminator() == terminator, "terminator");
        assert!(
            &node_bytes(&doc, 0)[syntax.value_range.clone()] == value,
            "value bytes mismatch"
        );
        assert!(doc.unclosed_quote_line().is_none());
    }

    fn unclosed_line(input: &[u8]) -> Option<usize> {
        parse_ok(input).unclosed_quote_line().map(|line| line.get())
    }

    const LF: Option<LineTerminator> = Some(LineTerminator::Lf);

    #[test]
    fn double_multiline_with_each_terminator() {
        expect_one(b"K=\"a\nb\"\n", QuoteStyle::Double, b"a\nb", true, LF);
        expect_one(
            b"K=\"a\r\nb\"\r\n",
            QuoteStyle::Double,
            b"a\r\nb",
            true,
            Some(LineTerminator::CrLf),
        );
        expect_one(
            b"K=\"a\rb\"\r",
            QuoteStyle::Double,
            b"a\rb",
            true,
            Some(LineTerminator::Cr),
        );
        expect_one(
            b"K=\"a\nb\r\nc\rd\"\n",
            QuoteStyle::Double,
            b"a\nb\r\nc\rd",
            true,
            LF,
        );
    }

    #[test]
    fn terminator_belongs_to_the_closing_line() {
        let doc = parse_ok(b"K=\"a\r\nb\"\n");
        assert!(doc.nodes()[0].terminator() == LF);
        let doc = parse_ok(b"K=\"a\nb\"\r\n");
        assert!(doc.nodes()[0].terminator() == Some(LineTerminator::CrLf));
    }

    #[test]
    fn single_multiline() {
        expect_one(b"K='a\nb\\\n'\n", QuoteStyle::Single, b"a\nb\\\n", true, LF);
        expect_one(b"K='\n'", QuoteStyle::Single, b"\n", true, None);
    }

    #[test]
    fn escaped_quote_inside_multiline_double() {
        expect_one(
            b"K=\"a\n\\\"b\nc\"\n",
            QuoteStyle::Double,
            b"a\n\\\"b\nc",
            true,
            LF,
        );
    }

    #[test]
    fn backslash_consumes_terminator_as_one_character() {
        // `\` + LF does not close and keeps going on the next line.
        expect_one(b"K=\"a\\\n\"\n", QuoteStyle::Double, b"a\\\n", true, LF);
        // `\` + CRLF is consumed whole, so the quote on the next line closes.
        expect_one(
            b"K=\"a\\\r\n\"\r\n",
            QuoteStyle::Double,
            b"a\\\r\n",
            true,
            Some(LineTerminator::CrLf),
        );
        // `\` + lone CR consumes only that CR; the quote right after it closes.
        expect_one(b"K=\"a\\\r\"\n", QuoteStyle::Double, b"a\\\r", true, LF);
        // The consumed terminator is not a closing opportunity: no later quote.
        assert!(unclosed_line(b"K=\"a\\\n") == Some(1));
        assert!(unclosed_line(b"K=\"a\\\r\n") == Some(1));
    }

    #[test]
    fn backslash_at_eof_leaves_quote_unclosed() {
        assert!(unclosed_line(b"K=\"a\\") == Some(1));
        assert!(unclosed_line(b"A=1\nK=\"\\") == Some(2));
    }

    #[test]
    fn escape_before_closing_quote() {
        expect_one(b"K=\"a\\\\\"\n", QuoteStyle::Double, b"a\\\\", false, LF);
        assert!(unclosed_line(b"K=\"a\\\"\n") == Some(1));
        expect_one(
            b"K=\"a\nb\\\\\"\n",
            QuoteStyle::Double,
            b"a\nb\\\\",
            true,
            LF,
        );
    }

    #[test]
    fn closing_quote_at_eof_without_terminator() {
        expect_one(b"K=\"a\nb\"", QuoteStyle::Double, b"a\nb", true, None);
        expect_one(b"K='a\nb'", QuoteStyle::Single, b"a\nb", true, None);
    }

    #[test]
    fn valid_comment_after_multiline_close() {
        expect_one(b"K=\"a\nb\" # n\n", QuoteStyle::Double, b"a\nb", true, LF);
        expect_one(b"K='a\nb'   \n", QuoteStyle::Single, b"a\nb", true, LF);
    }

    #[test]
    fn garbage_after_multiline_close_is_raw_over_both_lines() {
        for input in [
            &b"K=\"a\nb\"x\nZ=1\n"[..],
            b"K=\"a\nb\" x\nZ=1\n",
            b"K=\"a\nb\"#c\nZ=1\n",
            b"K='a\nb''c'\nZ=1\n",
        ] {
            let doc = parse_ok(input);
            assert!(doc.nodes().len() == 2);
            assert!(matches!(doc.nodes()[0].kind(), NodeKind::Raw));
            assert!(node_bytes(&doc, 0).ends_with(b"\n"));
            assert!(node_bytes(&doc, 0).len() > 8);
            assert!(matches!(doc.nodes()[1].kind(), NodeKind::Assignment(_)));
            assert!(doc.unclosed_quote_line().is_none());
        }
        let doc = parse_ok(b"K=\"a\nb\"x\nZ=1\n");
        assert!(node_bytes(&doc, 0) == b"K=\"a\nb\"x\n");
    }

    #[test]
    fn nul_after_multiline_close_is_raw() {
        assert!(kinds(b"K=\"a\nb\" # \0\n") == ["raw"]);
        assert!(kinds(b"K=\"a\0\nb\"\n") == ["raw", "raw"]);
    }

    #[test]
    fn nul_on_a_continuation_line_makes_the_node_raw() {
        for input in [&b"K=\"a\n\0\nb\"\n"[..], b"K=\"a\nb\0\"\n", b"K='a\n\0'\n"] {
            let doc = parse_ok(input);
            assert!(doc.nodes().len() == 1);
            assert!(matches!(doc.nodes()[0].kind(), NodeKind::Raw));
        }
    }

    #[test]
    fn lines_inside_a_value_are_not_classified() {
        let doc = parse_ok(b"S='x\nKEY=1\n# y\n\nz'\nT=2\n");
        assert!(doc.nodes().len() == 2);
        assert!(node_bytes(&doc, 0) == b"S='x\nKEY=1\n# y\n\nz'\n");
        assert!(node_bytes(&doc, 1) == b"T=2\n");
    }

    #[test]
    fn unclosed_quote_at_first_middle_and_last_line() {
        let doc = parse_ok(b"K=\"a\nB=1\n");
        assert!(doc.nodes().len() == 1);
        assert!(matches!(doc.nodes()[0].kind(), NodeKind::Raw));
        assert!(doc.unclosed_quote_line().map(|l| l.get()) == Some(1));
        assert!(node_bytes(&doc, 0) == b"K=\"a\nB=1\n");

        let doc = parse_ok(b"A=1\n# c\nK='x\nB=1\r\nC");
        assert!(doc.unclosed_quote_line().map(|l| l.get()) == Some(3));
        assert!(doc.nodes().len() == 3);
        assert!(matches!(doc.nodes()[0].kind(), NodeKind::Assignment(_)));
        assert!(matches!(doc.nodes()[1].kind(), NodeKind::Comment));
        assert!(matches!(doc.nodes()[2].kind(), NodeKind::Raw));
        assert!(node_bytes(&doc, 2) == b"K='x\nB=1\r\nC");
        assert!(doc.nodes()[2].terminator().is_none());

        let doc = parse_ok(b"A=1\nK=\"");
        assert!(doc.unclosed_quote_line().map(|l| l.get()) == Some(2));
        assert!(doc.nodes().len() == 2);
        assert!(node_bytes(&doc, 1) == b"K=\"");

        let doc = parse_ok(b"K=\"x\r\n");
        assert!(doc.nodes()[0].terminator() == Some(LineTerminator::CrLf));
    }

    #[test]
    fn orphan_quote_closing_later_absorbs_lines() {
        // Q-08: the orphan opens at line 1 and closes at line 3.
        let doc = parse_ok(b"A=\"x\nB=1\nC=\"y\"\nD=2\n");
        assert!(doc.nodes().len() == 2);
        assert!(doc.unclosed_quote_line().is_none());
        // The close at `C="` leaves `y"` as garbage, so the node is Raw.
        assert!(matches!(doc.nodes()[0].kind(), NodeKind::Raw));
        assert!(node_bytes(&doc, 0) == b"A=\"x\nB=1\nC=\"y\"\n");
        let doc = parse_ok(b"A='x\nB=1\nC='y'\nD=2\n");
        assert!(doc.nodes().len() == 2);
        assert!(matches!(doc.nodes()[0].kind(), NodeKind::Raw));
        let doc = parse_ok(b"A=\"x\nB=1\nC=\"\nD=2\n");
        let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
            panic!("expected an assignment");
        };
        assert!(syntax.is_multiline);
        assert!(doc.nodes().len() == 2);
    }

    #[test]
    fn cr_only_file_with_multiline_value() {
        let doc = parse_ok(b"A=1\rB=\"x\ry\"\rC=2\r");
        assert!(doc.nodes().len() == 3);
        let NodeKind::Assignment(syntax) = doc.nodes()[1].kind() else {
            panic!("expected an assignment");
        };
        assert!(syntax.is_multiline);
        assert!(node_bytes(&doc, 1) == b"B=\"x\ry\"\r");
        assert!(doc.nodes()[1].terminator() == Some(LineTerminator::Cr));
    }

    #[test]
    fn single_line_values_are_not_multiline() {
        for input in [&b"K=\"a\"\n"[..], b"K='a'\n", b"K=a\n", b"K=\"a\" # c\n"] {
            let doc = parse_ok(input);
            let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
                panic!("expected an assignment");
            };
            assert!(!syntax.is_multiline);
        }
    }

    #[test]
    fn bom_document_with_multiline_value() {
        let doc = parse_ok(b"\xEF\xBB\xBFK=\"a\nb\"\nZ=1\n");
        assert!(doc.nodes().len() == 2);
        assert!(doc.nodes()[0].origin() == &NodeOrigin::Source(3..11));
        let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
            panic!("expected an assignment");
        };
        assert!(syntax.is_multiline);
        assert!(syntax.value_range == (3..6));
        assert!(unclosed_line(b"\xEF\xBB\xBFA=1\nK=\"x") == Some(2));
    }

    #[test]
    fn well_formed_files_have_no_unclosed_quote() {
        assert!(unclosed_line(b"A=1\nB=\"x\"\nC='y\nz'\n").is_none());
        assert!(unclosed_line(b"").is_none());
    }

    #[test]
    fn unclosed_diagnostic_hides_content() {
        let doc = parse_ok(format!("K=\"{SENTINEL}\n").as_bytes());
        assert!(!format!("{doc:?}").contains(SENTINEL));
    }

    #[test]
    fn unclosed_quote_at_top_is_linear() {
        let mut input = b"K=\"".to_vec();
        input.extend(std::iter::repeat_n(b"line of filler text\n", 50_000).flatten());
        assert!(input.len() > 900_000);
        let doc = parse_ok(&input);
        assert!(doc.nodes().len() == 1);
        assert!(doc.unclosed_quote_line().map(|l| l.get()) == Some(1));
    }

    #[test]
    fn parsed_nodes_are_independent_per_line() {
        let doc = parse_ok(b"A=1\nA=2\n");
        assert!(doc.nodes().len() == 2);
        assert!(doc.nodes()[0].id() != doc.nodes()[1].id());
    }

    #[test]
    fn exhaustive_small_inputs_hold_invariants_and_cover_body() {
        const ALPHABET: [u8; 9] = [b'A', b'=', b'"', b'\'', b'#', b' ', b'\\', b'\n', b'\r'];
        const MAX_LEN: u32 = 5;
        let base = ALPHABET.len();
        let mut checked = 0_usize;
        for len in 0..=MAX_LEN {
            for n in 0..base.pow(len) {
                let mut rest = n;
                let input: Vec<u8> = (0..len)
                    .map(|_| {
                        let byte = ALPHABET[rest % base];
                        rest /= base;
                        byte
                    })
                    .collect();
                let doc = parse(input.clone()).expect("ASCII is valid UTF-8");
                assert!(
                    doc.check_invariants().is_ok(),
                    "invariants: len {len} case {n}"
                );
                let mut cursor = 0;
                for node in doc.nodes() {
                    let NodeOrigin::Source(span) = node.origin() else {
                        panic!("parser emits source nodes only");
                    };
                    assert!(span.start == cursor, "contiguous spans: len {len} case {n}");
                    cursor = span.end;
                }
                assert!(cursor == input.len(), "full coverage: len {len} case {n}");
                assert!(parse(input.clone()).expect("valid") == doc, "deterministic");
                checked += 1;
            }
        }
        assert!(checked == 66_430);
    }
}
