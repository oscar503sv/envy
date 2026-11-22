//! Syntax metadata for dotenv nodes.

use std::ops::Range;

use super::{error::DomainError, redact::Redacted};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    Unquoted,
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineTerminator {
    Lf,
    CrLf,
    Cr,
}

impl LineTerminator {
    pub fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Lf => b"\n",
            Self::CrLf => b"\r\n",
            Self::Cr => b"\r",
        }
    }
}

// Derived Debug is safe because the key is wrapped in Redacted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentSyntax {
    pub key: Redacted<String>,
    /// Byte range relative to the node's current bytes, including for synthetic
    /// nodes and nodes with a replacement.
    pub value_range: Range<usize>,
    pub quote: QuoteStyle,
    pub is_multiline: bool,
    pub is_export: bool,
}

/// A physical line as absolute byte offsets into the source.
///
/// Holds only offsets, so derived Debug cannot leak content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalLine {
    /// Content plus terminator, BOM bytes included in the offsets.
    pub span: Range<usize>,
    /// Where the terminator starts; equals `span.end` without a terminator.
    pub content_end: usize,
    pub terminator: Option<LineTerminator>,
}

/// Splits `source[body_start..]` into physical lines (SYNTAX §1).
///
/// Byte-level: works on any input, valid UTF-8 or not. A final segment without
/// terminator is a line; no empty line follows a final terminator.
pub fn split_lines(source: &[u8], body_start: usize) -> Vec<PhysicalLine> {
    let mut lines = Vec::new();
    let Some(body) = source.get(body_start..) else {
        return lines;
    };
    let mut line_start = body_start;
    let mut iter = body.iter().enumerate().peekable();
    while let Some((offset, byte)) = iter.next() {
        let pos = body_start.saturating_add(offset);
        let (terminator, len) = match byte {
            b'\n' => (LineTerminator::Lf, 1),
            b'\r' => {
                if iter.next_if(|&(_, next)| *next == b'\n').is_some() {
                    (LineTerminator::CrLf, 2)
                } else {
                    (LineTerminator::Cr, 1)
                }
            }
            _ => continue,
        };
        let end = pos.saturating_add(len);
        lines.push(PhysicalLine {
            span: line_start..end,
            content_end: pos,
            terminator: Some(terminator),
        });
        line_start = end;
    }
    if line_start < source.len() {
        lines.push(PhysicalLine {
            span: line_start..source.len(),
            content_end: source.len(),
            terminator: None,
        });
    }
    lines
}

/// Validates `source[body_start..]` as UTF-8, reporting an absolute offset.
pub fn check_utf8(source: &[u8], body_start: usize) -> Result<(), DomainError> {
    let body = source.get(body_start..).unwrap_or_default();
    match std::str::from_utf8(body) {
        Ok(_) => Ok(()),
        Err(error) => Err(DomainError::InvalidUtf8 {
            byte_offset: body_start.saturating_add(error.valid_up_to()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{LineTerminator, PhysicalLine, check_utf8, split_lines};
    use crate::domain::error::DomainError;
    use crate::test_support::SENTINEL;

    const LF: Option<LineTerminator> = Some(LineTerminator::Lf);
    const CRLF: Option<LineTerminator> = Some(LineTerminator::CrLf);
    const CR: Option<LineTerminator> = Some(LineTerminator::Cr);

    fn shape(
        source: &[u8],
        body_start: usize,
    ) -> Vec<(usize, usize, usize, Option<LineTerminator>)> {
        split_lines(source, body_start)
            .into_iter()
            .map(|l| (l.span.start, l.content_end, l.span.end, l.terminator))
            .collect()
    }

    fn terminators(source: &[u8]) -> Vec<Option<LineTerminator>> {
        split_lines(source, 0)
            .into_iter()
            .map(|l| l.terminator)
            .collect()
    }

    #[test]
    fn terminators_preserve_exact_bytes() {
        assert_eq!(LineTerminator::Lf.as_bytes(), b"\n");
        assert_eq!(LineTerminator::CrLf.as_bytes(), b"\r\n");
        assert_eq!(LineTerminator::Cr.as_bytes(), b"\r");
    }

    #[test]
    fn lf_line() {
        assert_eq!(shape(b"A\n", 0), [(0, 1, 2, LF)]);
    }

    #[test]
    fn crlf_line() {
        assert_eq!(shape(b"A\r\n", 0), [(0, 1, 3, CRLF)]);
    }

    #[test]
    fn cr_line() {
        assert_eq!(shape(b"A\r", 0), [(0, 1, 2, CR)]);
    }

    #[test]
    fn mixed_terminators() {
        assert_eq!(
            shape(b"A\r\nB\nC\rD", 0),
            [
                (0, 1, 3, CRLF),
                (3, 4, 5, LF),
                (5, 6, 7, CR),
                (7, 8, 8, None)
            ]
        );
    }

    #[test]
    fn cr_only_file() {
        assert_eq!(terminators(b"A\rB\rC\r"), [CR, CR, CR]);
    }

    #[test]
    fn lf_then_cr_are_two_lines() {
        assert_eq!(shape(b"\n\r", 0), [(0, 0, 1, LF), (1, 1, 2, CR)]);
    }

    #[test]
    fn cr_then_crlf() {
        assert_eq!(shape(b"\r\r\n", 0), [(0, 0, 1, CR), (1, 1, 3, CRLF)]);
    }

    #[test]
    fn no_final_newline() {
        assert_eq!(shape(b"A\nB", 0), [(0, 1, 2, LF), (2, 3, 3, None)]);
    }

    #[test]
    fn only_terminators() {
        assert_eq!(shape(b"\n\n", 0), [(0, 0, 1, LF), (1, 1, 2, LF)]);
    }

    #[test]
    fn empty_input_has_no_lines() {
        assert!(split_lines(b"", 0).is_empty());
    }

    #[test]
    fn bom_only_input_has_no_lines() {
        assert!(split_lines(b"\xEF\xBB\xBF", 3).is_empty());
    }

    #[test]
    fn body_start_beyond_len_has_no_lines() {
        assert!(split_lines(b"A\n", 10).is_empty());
    }

    #[test]
    fn bom_plus_content_starts_after_bom() {
        let lines = split_lines(b"\xEF\xBB\xBFA=1\n", 3);
        assert_eq!(
            lines,
            [PhysicalLine {
                span: 3..7,
                content_end: 6,
                terminator: LF
            }]
        );
    }

    #[test]
    fn bom_after_offset_zero_is_content() {
        let source = b"\xEF\xBB\xBF\xEF\xBB\xBFA\n";
        assert_eq!(shape(source, 3), [(3, 7, 8, LF)]);
    }

    #[test]
    fn invalid_utf8_still_splits() {
        assert_eq!(shape(b"\xFF\n\xFE", 0), [(0, 1, 2, LF), (2, 3, 3, None)]);
    }

    /// Stands in for proptest: every input of length 0..=6 over a 4-byte
    /// alphabet (4^0 + ... + 4^6 = 5461 inputs).
    #[test]
    fn exhaustive_small_inputs_are_partitioned_losslessly() {
        const ALPHABET: [u8; 4] = [b'a', b'\r', b'\n', 0xEF];
        const MAX_LEN: u32 = 6;
        let mut checked = 0_usize;
        for len in 0..=MAX_LEN {
            for n in 0..4_usize.pow(len) {
                let mut rest = n;
                let input: Vec<u8> = (0..len)
                    .map(|_| {
                        let byte = ALPHABET.get(rest % 4).copied().unwrap_or(b'a');
                        rest /= 4;
                        byte
                    })
                    .collect();
                check_partition(&input);
                checked += 1;
            }
        }
        assert_eq!(checked, 5461);
    }

    fn check_partition(input: &[u8]) {
        let lines = split_lines(input, 0);
        let mut rebuilt = Vec::new();
        let mut cursor = 0;
        for (index, line) in lines.iter().enumerate() {
            assert_eq!(line.span.start, cursor, "contiguous: {input:?}");
            assert!(line.span.start <= line.content_end && line.content_end <= line.span.end);
            cursor = line.span.end;
            rebuilt.extend_from_slice(&input[line.span.clone()]);
            let tail = &input[line.content_end..line.span.end];
            match line.terminator {
                Some(t) => assert_eq!(tail, t.as_bytes(), "terminator bytes: {input:?}"),
                None => {
                    assert!(tail.is_empty());
                    assert_eq!(index + 1, lines.len(), "only last may lack terminator");
                }
            }
            if let Some(next) = lines.get(index + 1) {
                let ends_with_cr =
                    line.content_end > line.span.start && input[line.content_end - 1] == b'\r';
                let next_is_lf = input.get(next.span.start) == Some(&b'\n');
                assert!(!(ends_with_cr && next_is_lf), "CRLF split: {input:?}");
            }
        }
        assert_eq!(cursor, input.len());
        assert_eq!(rebuilt, input);
    }

    #[test]
    fn utf8_valid_ascii_and_multibyte() {
        assert_eq!(check_utf8(b"KEY=value\n", 0), Ok(()));
        assert_eq!(check_utf8("K=\u{e9}\u{1f600}".as_bytes(), 0), Ok(()));
    }

    #[test]
    fn utf8_bom_is_skipped_via_body_start() {
        assert_eq!(check_utf8(b"\xEF\xBB\xBFA=1", 3), Ok(()));
    }

    #[test]
    fn utf8_invalid_offset_is_absolute() {
        assert_eq!(
            check_utf8(b"AB\xFFC", 0),
            Err(DomainError::InvalidUtf8 { byte_offset: 2 })
        );
        assert_eq!(
            check_utf8(b"\xEF\xBB\xBFAB\xFFC", 3),
            Err(DomainError::InvalidUtf8 { byte_offset: 5 })
        );
    }

    #[test]
    fn utf8_truncated_multibyte_at_eof() {
        assert_eq!(
            check_utf8(b"A\xE2\x82", 0),
            Err(DomainError::InvalidUtf8 { byte_offset: 1 })
        );
    }

    #[test]
    fn utf8_rejects_overlong_surrogate_and_out_of_range() {
        for (input, offset) in [
            (&b"A\xC0\x80"[..], 1),
            (b"A\xE0\x80\x80", 1),
            (b"A\xF0\x80\x80\x80", 1),
            (b"A\xED\xA0\x80", 1),
            (b"A\xF4\x90\x80\x80", 1),
        ] {
            assert_eq!(
                check_utf8(input, 0),
                Err(DomainError::InvalidUtf8 {
                    byte_offset: offset
                })
            );
        }
    }

    #[test]
    fn utf8_body_start_beyond_len_is_ok() {
        assert_eq!(check_utf8(b"A", 5), Ok(()));
    }

    #[test]
    fn utf8_error_never_contains_input_bytes() {
        let mut input = SENTINEL.as_bytes().to_vec();
        input.push(0xFF);
        let Err(error) = check_utf8(&input, 0) else {
            panic!("expected an invalid UTF-8 error");
        };
        assert!(!error.to_string().contains(SENTINEL));
        assert!(!format!("{error:?}").contains(SENTINEL));
    }
}
