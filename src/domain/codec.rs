//! Value codec: raw bytes of an assignment value <-> displayed text (SYNTAX §6).
//!
//! Pure functions, no I/O. `decode` shows what the file means; `encode` writes
//! a displayed value back in the node's current quote style or says the style
//! cannot hold it. Nothing here converts styles: [`NeedsConversion`] only
//! reports that a conversion would be required (applied elsewhere, with an
//! explicit user confirmation).

use super::{
    document::NodeKind,
    parser::parse,
    syntax::{AssignmentSyntax, LineTerminator, QuoteStyle},
};

/// Why a value cannot be written in the current quote style.
///
/// Carries no value data, so Debug and Display are safe to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConversionReason {
    #[error("the value contains a single quote")]
    ContainsQuote,
    #[error("the value contains a carriage return")]
    ContainsCarriageReturn,
    #[error("the value contains a line break")]
    ContainsNewline,
    #[error("the value starts or ends with whitespace")]
    EdgeWhitespace,
    #[error("the encoded value would not be read back as the same assignment")]
    RoundTripMismatch,
}

/// The value needs another quote style; it must never be applied without
/// explicit confirmation (SYNTAX §6.2).
///
/// There is no `Unrepresentable` outcome: `Double` encodes every string, so
/// conversion to `Double` is always a way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("value needs conversion to double quotes: {reason}")]
pub struct NeedsConversion {
    pub to: QuoteStyle,
    pub reason: ConversionReason,
}

impl NeedsConversion {
    fn to_double(reason: ConversionReason) -> Self {
        Self {
            to: QuoteStyle::Double,
            reason,
        }
    }
}

/// Context that `encode` needs from the document and the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeContext {
    pub dominant: LineTerminator,
    pub is_multiline: bool,
}

fn terminator_str(terminator: LineTerminator) -> &'static str {
    match terminator {
        LineTerminator::Lf => "\n",
        LineTerminator::CrLf => "\r\n",
        LineTerminator::Cr => "\r",
    }
}

/// Decodes the raw bytes of a value into the displayed text (SYNTAX §6.1).
pub fn decode(style: QuoteStyle, raw: &str) -> String {
    match style {
        QuoteStyle::Unquoted => raw.to_owned(),
        QuoteStyle::Single => normalize_terminators(raw),
        QuoteStyle::Double => decode_double(raw),
    }
}

fn normalize_terminators(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            chars.next_if_eq(&'\n');
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

fn decode_double(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                None => out.push('\\'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                // `\` + CRLF is one pair: shows `\` and a single LF.
                Some('\r') => {
                    chars.next_if_eq(&'\n');
                    out.push_str("\\\n");
                }
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
            },
            '\r' => {
                chars.next_if_eq(&'\n');
                out.push('\n');
            }
            other => out.push(other),
        }
    }
    out
}

/// Encodes a displayed value for the given quote style (SYNTAX §6.2).
///
/// This does not apply the round-trip guard; use [`encode_in_node`] when the
/// surrounding bytes are known.
pub fn encode(
    style: QuoteStyle,
    value: &str,
    ctx: EncodeContext,
) -> Result<String, NeedsConversion> {
    match style {
        QuoteStyle::Double => Ok(encode_double(value, ctx)),
        QuoteStyle::Single => encode_single(value, ctx),
        QuoteStyle::Unquoted => encode_unquoted(value),
    }
}

fn encode_double(value: &str, ctx: EncodeContext) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\r' => out.push_str("\\r"),
            '\n' if ctx.is_multiline => out.push_str(terminator_str(ctx.dominant)),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

fn encode_single(value: &str, ctx: EncodeContext) -> Result<String, NeedsConversion> {
    if value.contains('\'') {
        return Err(NeedsConversion::to_double(ConversionReason::ContainsQuote));
    }
    if value.contains('\r') {
        return Err(NeedsConversion::to_double(
            ConversionReason::ContainsCarriageReturn,
        ));
    }
    Ok(value.replace('\n', terminator_str(ctx.dominant)))
}

fn encode_unquoted(value: &str) -> Result<String, NeedsConversion> {
    if value.contains('\n') {
        return Err(NeedsConversion::to_double(
            ConversionReason::ContainsNewline,
        ));
    }
    if value.contains('\r') {
        return Err(NeedsConversion::to_double(
            ConversionReason::ContainsCarriageReturn,
        ));
    }
    let is_blank = |c: char| c == ' ' || c == '\t';
    if value.starts_with(is_blank) || value.ends_with(is_blank) {
        return Err(NeedsConversion::to_double(ConversionReason::EdgeWhitespace));
    }
    Ok(value.to_owned())
}

/// Encodes `value` into an assignment node and applies the round-trip guard
/// (SYNTAX §6.2, SYN-40, Q-07).
///
/// `node_bytes` are the node's current bytes (prefix, value, suffix and
/// terminator) with the value at `syntax.value_range`. On success returns the
/// new node bytes; only the value range changes. The result is re-parsed and
/// must be the same assignment (key, quote style, `export`) showing `value`;
/// otherwise the style cannot hold it.
pub fn encode_in_node(
    node_bytes: &str,
    syntax: &AssignmentSyntax,
    value: &str,
    dominant: LineTerminator,
) -> Result<String, NeedsConversion> {
    let mismatch = || NeedsConversion::to_double(ConversionReason::RoundTripMismatch);
    let ctx = EncodeContext {
        dominant,
        is_multiline: syntax.is_multiline,
    };
    let encoded = encode(syntax.quote, value, ctx)?;

    let range = &syntax.value_range;
    let (Some(prefix), Some(suffix)) = (node_bytes.get(..range.start), node_bytes.get(range.end..))
    else {
        return Err(mismatch());
    };
    let mut composed = String::with_capacity(prefix.len() + encoded.len() + suffix.len());
    composed.push_str(prefix);
    composed.push_str(&encoded);
    composed.push_str(suffix);

    let document = parse(composed.clone().into_bytes()).map_err(|_| mismatch())?;
    let [node] = document.nodes() else {
        return Err(mismatch());
    };
    let NodeKind::Assignment(reparsed) = node.kind() else {
        return Err(mismatch());
    };
    let raw = document
        .current_bytes(node)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|text| text.get(reparsed.value_range.clone()));
    let is_same = reparsed.key == syntax.key
        && reparsed.quote == syntax.quote
        && reparsed.is_export == syntax.is_export
        && raw.is_some_and(|raw| decode(reparsed.quote, raw) == value);
    if is_same {
        Ok(composed)
    } else {
        Err(mismatch())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::SENTINEL;
    use LineTerminator::{Cr, CrLf, Lf};
    use QuoteStyle::{Double, Single, Unquoted};

    fn ctx(dominant: LineTerminator, is_multiline: bool) -> EncodeContext {
        EncodeContext {
            dominant,
            is_multiline,
        }
    }

    fn enc(style: QuoteStyle, value: &str) -> Result<String, NeedsConversion> {
        encode(style, value, ctx(Lf, false))
    }

    fn conversion(reason: ConversionReason) -> Result<String, NeedsConversion> {
        Err(NeedsConversion { to: Double, reason })
    }

    /// Parses one node, edits its value and returns the new node bytes.
    fn edit(node: &str, value: &str, dominant: LineTerminator) -> Result<String, NeedsConversion> {
        let doc = parse(node.as_bytes().to_vec()).unwrap();
        let [n] = doc.nodes() else {
            panic!("expected one node");
        };
        let NodeKind::Assignment(syntax) = n.kind() else {
            panic!("expected an assignment");
        };
        encode_in_node(node, syntax, value, dominant)
    }

    #[test]
    fn decode_unquoted_is_identity() {
        for raw in [
            "",
            "abc",
            "a b",
            "a\\nb",
            "\\\\",
            "${VAR}",
            "é世😀",
            "a\"b'c",
        ] {
            assert_eq!(decode(Unquoted, raw), raw);
        }
    }

    #[test]
    fn decode_single_is_identity_except_terminators() {
        for (raw, expected) in [
            ("", ""),
            ("a\\nb\\\\", "a\\nb\\\\"),
            ("a\nb", "a\nb"),
            ("a\r\nb", "a\nb"),
            ("a\rb", "a\nb"),
            ("a\r\n\r\nb", "a\n\nb"),
            ("a\n\rb", "a\n\nb"),
            ("\r", "\n"),
            ("${VAR} $(x) $Y", "${VAR} $(x) $Y"),
        ] {
            assert_eq!(decode(Single, raw), expected);
        }
    }

    #[test]
    fn decode_double_escape_table() {
        for (raw, expected) in [
            ("", ""),
            ("a\\\\b", "a\\b"),
            ("a\\\"b", "a\"b"),
            ("a\\nb", "a\nb"),
            ("a\\rb", "a\rb"),
            ("a\\tb", "a\tb"),
            ("a\tb", "a\tb"),
            ("\\\\n", "\\n"),
            ("\\\\\\n", "\\\n"),
        ] {
            assert_eq!(decode(Double, raw), expected);
        }
    }

    #[test]
    fn decode_double_unknown_escapes_stay_literal() {
        for (raw, expected) in [
            ("\\$", "\\$"),
            ("\\u00e9", "\\u00e9"),
            ("\\0", "\\0"),
            ("\\'", "\\'"),
            ("\\q", "\\q"),
            ("\\é", "\\é"),
            ("\\世x", "\\世x"),
            ("\\😀", "\\😀"),
            ("${VAR}", "${VAR}"),
            ("\\${VAR}", "\\${VAR}"),
        ] {
            assert_eq!(decode(Double, raw), expected);
        }
    }

    #[test]
    fn decode_double_trailing_backslash_is_literal() {
        assert_eq!(decode(Double, "abc\\"), "abc\\");
        assert_eq!(decode(Double, "\\"), "\\");
    }

    #[test]
    fn decode_double_backslash_before_terminator_is_literal_pair() {
        assert_eq!(decode(Double, "a\\\nb"), "a\\\nb");
        assert_eq!(decode(Double, "a\\\rb"), "a\\\nb");
        // `\` + CRLF is one pair: a single LF follows the backslash.
        assert_eq!(decode(Double, "a\\\r\nb"), "a\\\nb");
        assert_eq!(decode(Double, "a\\\r\n\nb"), "a\\\n\nb");
        assert_eq!(decode(Double, "a\\\r\r\nb"), "a\\\n\nb");
    }

    #[test]
    fn decode_double_normalizes_real_terminators() {
        assert_eq!(decode(Double, "a\nb\r\nc\rd"), "a\nb\nc\nd");
        assert_eq!(decode(Double, "\r\n\r\n"), "\n\n");
        assert_eq!(decode(Double, "\n\r"), "\n\n");
    }

    #[test]
    fn encode_double_escapes_and_never_fails() {
        for (value, expected) in [
            ("", ""),
            ("a\"b\\c", "a\\\"b\\\\c"),
            ("a\rb", "a\\rb"),
            ("a\nb", "a\\nb"),
            ("a\tb", "a\tb"),
            ("\\n", "\\\\n"),
            ("${VAR}", "${VAR}"),
            ("é世😀", "é世😀"),
            ("\\q", "\\\\q"),
            ("a #b 'c'", "a #b 'c'"),
        ] {
            assert_eq!(enc(Double, value).as_deref(), Ok(expected));
        }
    }

    #[test]
    fn encode_double_multiline_uses_dominant_terminator() {
        for (dominant, term) in [(Lf, "\n"), (CrLf, "\r\n"), (Cr, "\r")] {
            let encoded = encode(Double, "a\nb\r", ctx(dominant, true)).unwrap();
            assert_eq!(encoded, format!("a{term}b\\r"));
        }
        let single_line = encode(Double, "a\nb", ctx(CrLf, false)).unwrap();
        assert_eq!(single_line, "a\\nb");
    }

    #[test]
    fn encode_single_rules() {
        assert_eq!(enc(Single, "").as_deref(), Ok(""));
        assert_eq!(enc(Single, "a\\n\"b #c $D").as_deref(), Ok("a\\n\"b #c $D"));
        assert_eq!(
            enc(Single, "it's"),
            conversion(ConversionReason::ContainsQuote)
        );
        assert_eq!(
            enc(Single, "a\rb"),
            conversion(ConversionReason::ContainsCarriageReturn)
        );
        assert_eq!(
            enc(Single, "a\r\nb"),
            conversion(ConversionReason::ContainsCarriageReturn)
        );
    }

    #[test]
    fn encode_single_lf_becomes_dominant_terminator() {
        for (dominant, term) in [(Lf, "\n"), (CrLf, "\r\n"), (Cr, "\r")] {
            let encoded = encode(Single, "a\nb\n", ctx(dominant, true)).unwrap();
            assert_eq!(encoded, format!("a{term}b{term}"));
        }
    }

    #[test]
    fn encode_unquoted_rules() {
        assert_eq!(enc(Unquoted, "").as_deref(), Ok(""));
        assert_eq!(enc(Unquoted, "a  b\\n").as_deref(), Ok("a  b\\n"));
        assert_eq!(
            enc(Unquoted, "a\nb"),
            conversion(ConversionReason::ContainsNewline)
        );
        assert_eq!(
            enc(Unquoted, "a\rb"),
            conversion(ConversionReason::ContainsCarriageReturn)
        );
        for value in [" a", "a ", "\ta", "a\t", " "] {
            assert_eq!(
                enc(Unquoted, value),
                conversion(ConversionReason::EdgeWhitespace)
            );
        }
    }

    #[test]
    fn guard_unquoted_whitespace_then_hash() {
        assert_eq!(
            edit("K=a\n", "a #b", Lf),
            conversion(ConversionReason::RoundTripMismatch)
        );
        assert_eq!(
            edit("K=a\n", "a\t#b", Lf),
            conversion(ConversionReason::RoundTripMismatch)
        );
        assert_eq!(edit("K=a\n", "a#b", Lf).as_deref(), Ok("K=a#b\n"));
    }

    #[test]
    fn guard_unquoted_leading_quote_or_hash() {
        for node in ["K=a\n", "K = a\n", "  K=a\n", "export K=a\n"] {
            for value in ["\"x\"", "'x'", "\"x", "'x"] {
                assert_eq!(
                    edit(node, value, Lf),
                    conversion(ConversionReason::RoundTripMismatch)
                );
            }
        }
        // Prefix ends in WS, so a leading `#` would start a comment.
        assert_eq!(
            edit("K= a\n", "#x", Lf),
            conversion(ConversionReason::RoundTripMismatch)
        );
        // Without WS before the value, `#` stays content.
        assert_eq!(edit("K=a\n", "#x", Lf).as_deref(), Ok("K=#x\n"));
    }

    #[test]
    fn guard_unquoted_newline_is_a_conversion() {
        assert_eq!(
            edit("K=a\n", "a\nb", Lf),
            conversion(ConversionReason::ContainsNewline)
        );
    }

    #[test]
    fn guard_single_with_quote() {
        assert_eq!(
            edit("K='a'\n", "it's", Lf),
            conversion(ConversionReason::ContainsQuote)
        );
        assert_eq!(edit("K='a'\n", "b\\n", Lf).as_deref(), Ok("K='b\\n'\n"));
    }

    #[test]
    fn guard_rejects_nul_that_turns_the_line_raw() {
        assert_eq!(
            edit("K=\"a\"\n", "a\0b", Lf),
            conversion(ConversionReason::RoundTripMismatch)
        );
    }

    #[test]
    fn empty_value_with_comment_keeps_the_gap() {
        assert_eq!(edit("KEY= # n\n", "x", Lf).as_deref(), Ok("KEY=x # n\n"));
        assert_eq!(edit("KEY=\n", "x", Lf).as_deref(), Ok("KEY=x\n"));
        assert_eq!(edit("KEY=  \n", "x", Lf).as_deref(), Ok("KEY=  x\n"));
        assert_eq!(edit("KEY=x # n\n", "", Lf).as_deref(), Ok("KEY= # n\n"));
    }

    #[test]
    fn double_edit_keeps_suffix_comment_and_terminator() {
        assert_eq!(
            edit("K = \"a\" # n\r\n", "b\"c", CrLf).as_deref(),
            Ok("K = \"b\\\"c\" # n\r\n")
        );
        assert_eq!(
            edit("export K=\"a\"", "x", Lf).as_deref(),
            Ok("export K=\"x\"")
        );
    }

    #[test]
    fn multiline_double_keeps_suffix_and_uses_real_terminators() {
        assert_eq!(
            edit("K=\"a\nb\" # c\n", "x\ny", Lf).as_deref(),
            Ok("K=\"x\ny\" # c\n")
        );
        assert_eq!(
            edit("K=\"a\nb\" # c\n", "x\ny", CrLf).as_deref(),
            Ok("K=\"x\r\ny\" # c\n")
        );
        // A single-line node writes `\n` instead (Q-06).
        assert_eq!(
            edit("K=\"a\" # c\n", "x\ny", CrLf).as_deref(),
            Ok("K=\"x\\ny\" # c\n")
        );
    }

    #[test]
    fn multiline_single_lf_becomes_dominant_crlf() {
        assert_eq!(
            edit("K='a\nb'\n", "x\ny", CrLf).as_deref(),
            Ok("K='x\r\ny'\n")
        );
        assert_eq!(
            edit("K='a\r\nb'\r\n", "x\ny\nz", Lf).as_deref(),
            Ok("K='x\ny\nz'\r\n")
        );
    }

    #[test]
    fn literal_expansions_are_plain_text() {
        for node in ["K=a\n", "K='a'\n", "K=\"a\"\n"] {
            let value = "${VAR} $(cmd) $X";
            let out = edit(node, value, Lf).unwrap();
            assert!(out.contains(value));
        }
    }

    #[test]
    fn needs_conversion_never_carries_the_value() {
        let value = format!("{SENTINEL} it's");
        let Err(error) = encode(Single, &value, ctx(Lf, false)) else {
            panic!("expected a conversion");
        };
        assert!(!error.to_string().contains(SENTINEL));
        assert!(!format!("{error:?}").contains(SENTINEL));
        assert_eq!(error.to, Double);

        let node = "K=a\n";
        let doc = parse(node.as_bytes().to_vec()).unwrap();
        let NodeKind::Assignment(syntax) = doc.nodes()[0].kind() else {
            panic!("expected an assignment");
        };
        let tainted = format!("{SENTINEL} #x");
        let Err(error) = encode_in_node(node, syntax, &tainted, Lf) else {
            panic!("expected a conversion");
        };
        assert!(!error.to_string().contains(SENTINEL));
        assert!(!format!("{error:?}").contains(SENTINEL));
    }

    #[test]
    fn display_messages_are_static() {
        let error = NeedsConversion {
            to: Double,
            reason: ConversionReason::ContainsQuote,
        };
        assert_eq!(
            error.to_string(),
            "value needs conversion to double quotes: the value contains a single quote"
        );
    }
}
