//! Byte-preserving dotenv document model.

use std::{collections::HashSet, ops::Range};

use super::{
    error::{LineNumber, NodeKindTag},
    redact::Redacted,
    syntax::{AssignmentSyntax, LineTerminator},
};

const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";

/// Identifies one occurrence within a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u64);

impl NodeId {
    pub fn get(self) -> u64 {
        self.0
    }
}

// Derived Debug is safe because AssignmentSyntax redacts its user data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Assignment(AssignmentSyntax),
    Comment,
    Blank,
    Raw,
}

impl NodeKind {
    pub fn tag(&self) -> NodeKindTag {
        match self {
            Self::Assignment(_) => NodeKindTag::Assignment,
            Self::Comment => NodeKindTag::Comment,
            Self::Blank => NodeKindTag::Blank,
            Self::Raw => NodeKindTag::Raw,
        }
    }
}

// Derived Debug is safe because synthetic bytes are wrapped in Redacted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeOrigin {
    /// Absolute byte offsets into the original source, counted from byte zero.
    Source(Range<usize>),
    Synthetic(Redacted<Vec<u8>>),
}

// Derived Debug is safe because all user data is wrapped in Redacted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvNode {
    id: NodeId,
    origin: NodeOrigin,
    kind: NodeKind,
    terminator: Option<LineTerminator>,
    replacement: Option<Redacted<Vec<u8>>>,
    removed: bool,
}

impl EnvNode {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn origin(&self) -> &NodeOrigin {
        &self.origin
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    /// Terminator of the node's last physical line, if present.
    pub fn terminator(&self) -> Option<LineTerminator> {
        self.terminator
    }

    pub fn replacement(&self) -> Option<&Redacted<Vec<u8>>> {
        self.replacement.as_ref()
    }

    pub fn is_removed(&self) -> bool {
        self.removed
    }

    pub fn is_synthetic(&self) -> bool {
        matches!(self.origin, NodeOrigin::Synthetic(_))
    }

    #[allow(dead_code, reason = "reserved for structured editing")]
    pub(crate) fn set_replacement(&mut self, bytes: Option<Vec<u8>>) {
        self.replacement = bytes.map(Redacted::new);
    }

    #[allow(dead_code, reason = "reserved for structured editing")]
    pub(crate) fn set_removed(&mut self, removed: bool) {
        self.removed = removed;
    }
}

// Derived Debug is safe because all user data is wrapped in Redacted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvDocument {
    source: Redacted<Vec<u8>>,
    bom: bool,
    nodes: Vec<EnvNode>,
    next_id: u64,
    unclosed_quote: Option<LineNumber>,
}

impl EnvDocument {
    pub fn new(source: Vec<u8>) -> Self {
        Self {
            bom: source.starts_with(UTF8_BOM),
            source: Redacted::new(source),
            nodes: Vec::new(),
            next_id: 0,
            unclosed_quote: None,
        }
    }

    pub fn source(&self) -> &[u8] {
        self.source.expose()
    }

    pub fn has_bom(&self) -> bool {
        self.bom
    }

    pub fn body_start(&self) -> usize {
        if self.bom { UTF8_BOM.len() } else { 0 }
    }

    /// 1-based physical line where the first unclosed quote opens, if any.
    ///
    /// Everything from that line to EOF is a single `Raw` node (SYNTAX §5.6).
    /// Only the line number is stored, never content; the UI builds the
    /// user-facing message from it.
    pub fn unclosed_quote_line(&self) -> Option<LineNumber> {
        self.unclosed_quote
    }

    pub(crate) fn set_unclosed_quote(&mut self, line: Option<LineNumber>) {
        self.unclosed_quote = line;
    }

    pub fn nodes(&self) -> &[EnvNode] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> Option<&EnvNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    #[allow(dead_code, reason = "reserved for structured editing")]
    pub(crate) fn node_mut(&mut self, id: NodeId) -> Option<&mut EnvNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// Appends an original node. Use check_invariants to validate the spans.
    pub fn push_source_node(
        &mut self,
        span: Range<usize>,
        kind: NodeKind,
        terminator: Option<LineTerminator>,
    ) -> NodeId {
        let node = self.new_node(NodeOrigin::Source(span), kind, terminator);
        let id = node.id;
        self.nodes.push(node);
        id
    }

    pub fn insert_synthetic_node(
        &mut self,
        index: usize,
        bytes: Vec<u8>,
        kind: NodeKind,
        terminator: Option<LineTerminator>,
    ) -> NodeId {
        let node = self.new_node(
            NodeOrigin::Synthetic(Redacted::new(bytes)),
            kind,
            terminator,
        );
        let id = node.id;
        self.nodes.insert(index.min(self.nodes.len()), node);
        id
    }

    fn new_node(
        &mut self,
        origin: NodeOrigin,
        kind: NodeKind,
        terminator: Option<LineTerminator>,
    ) -> EnvNode {
        let id = NodeId(self.next_id);
        // Every allocated ID remains in the Vec, whose capacity is bounded
        // by isize::MAX bytes. The u64 counter cannot reach its limit.
        self.next_id += 1;
        EnvNode {
            id,
            origin,
            kind,
            terminator,
            replacement: None,
            removed: false,
        }
    }

    pub fn current_bytes<'a>(&'a self, node: &'a EnvNode) -> Option<&'a [u8]> {
        if let Some(replacement) = node.replacement() {
            return Some(replacement.expose());
        }
        match node.origin() {
            NodeOrigin::Source(span) => self.source().get(span.clone()),
            NodeOrigin::Synthetic(bytes) => Some(bytes.expose()),
        }
    }

    /// Checks the BOM first, then validates nodes in document order and checks
    /// final source coverage.
    pub fn check_invariants(&self) -> Result<(), InvariantViolation> {
        use InvariantViolation::*;

        let source = self.source();
        if self.bom != source.starts_with(UTF8_BOM) {
            return Err(BomMismatch);
        }

        let mut ids = HashSet::new();
        let mut covered_until = self.body_start();
        // Only the last live node may lack a terminator (SYNTAX.md §4).
        let mut unterminated: Option<u64> = None;
        for node in &self.nodes {
            let node_id = node.id.get();
            if let NodeOrigin::Source(span) = node.origin() {
                if source.get(span.clone()).is_none() {
                    return Err(SpanOutOfBounds { node_id });
                }
                if span.is_empty() {
                    return Err(EmptySourceSpan { node_id });
                }
                if span.start != covered_until {
                    return Err(SourceCoverage {
                        expected_start: covered_until,
                        found_start: span.start,
                    });
                }
                // `\r\n` is always one CRLF terminator (SYNTAX.md §1), so a
                // node boundary between its two bytes means a broken lexer.
                let ends_with_cr = span
                    .end
                    .checked_sub(1)
                    .and_then(|last| source.get(last))
                    .is_some_and(|byte| *byte == b'\r');
                if ends_with_cr && source.get(span.end) == Some(&b'\n') {
                    return Err(SplitCrLf { node_id });
                }
                covered_until = span.end;
            }

            if !node.removed {
                if let Some(previous) = unterminated {
                    return Err(MissingTerminator { node_id: previous });
                }
                if node.terminator.is_none() {
                    unterminated = Some(node_id);
                }
            }

            if !ids.insert(node.id) {
                return Err(DuplicateId { node_id });
            }
            if node.kind == NodeKind::Raw && (node.replacement.is_some() || node.removed) {
                return Err(RawNodeEdited { node_id });
            }

            let bytes = self
                .current_bytes(node)
                .ok_or(SpanOutOfBounds { node_id })?;
            let actual_terminator = if bytes.ends_with(b"\r\n") {
                Some(LineTerminator::CrLf)
            } else if bytes.ends_with(b"\n") {
                Some(LineTerminator::Lf)
            } else if bytes.ends_with(b"\r") {
                Some(LineTerminator::Cr)
            } else {
                None
            };
            if node.terminator != actual_terminator {
                return Err(TerminatorMismatch { node_id });
            }

            if let NodeKind::Assignment(syntax) = node.kind() {
                let content_end = bytes.len() - actual_terminator.map_or(0, |t| t.as_bytes().len());
                let range = &syntax.value_range;
                let is_valid = range.start <= range.end
                    && range.end <= content_end
                    && std::str::from_utf8(bytes).is_ok_and(|text| {
                        text.is_char_boundary(range.start) && text.is_char_boundary(range.end)
                    });
                if !is_valid {
                    return Err(InvalidValueRange { node_id });
                }
            }
        }

        if covered_until != source.len() {
            return Err(SourceIncomplete {
                covered_until,
                source_len: source.len(),
            });
        }
        Ok(())
    }
}

// Copy ensures that diagnostics carry only value-free metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum InvariantViolation {
    #[error("bom flag disagrees with source prefix")]
    BomMismatch,
    #[error("expected source span to start at {expected_start}, found {found_start}")]
    SourceCoverage {
        expected_start: usize,
        found_start: usize,
    },
    #[error("source coverage ends at {covered_until} of {source_len} bytes")]
    SourceIncomplete {
        covered_until: usize,
        source_len: usize,
    },
    #[error("source span for node {node_id} is out of bounds")]
    SpanOutOfBounds { node_id: u64 },
    #[error("source span for node {node_id} is empty")]
    EmptySourceSpan { node_id: u64 },
    #[error("duplicate node id {node_id}")]
    DuplicateId { node_id: u64 },
    #[error("raw node {node_id} was edited")]
    RawNodeEdited { node_id: u64 },
    #[error("terminator does not match bytes for node {node_id}")]
    TerminatorMismatch { node_id: u64 },
    #[error("invalid value range for node {node_id}")]
    InvalidValueRange { node_id: u64 },
    #[error("crlf terminator is split at the end of node {node_id}")]
    SplitCrLf { node_id: u64 },
    #[error("node {node_id} has no terminator but is not the last node")]
    MissingTerminator { node_id: u64 },
}

#[cfg(test)]
mod tests {
    use super::{InvariantViolation::*, *};
    use crate::{domain::syntax::QuoteStyle, test_support::SENTINEL};

    fn assignment(key: &str, value_range: Range<usize>, quote: QuoteStyle) -> NodeKind {
        NodeKind::Assignment(AssignmentSyntax {
            key: Redacted::new(key.to_owned()),
            value_range,
            quote,
            is_multiline: false,
            is_export: false,
        })
    }

    fn example(with_bom: bool) -> EnvDocument {
        use LineTerminator::{CrLf, Lf};

        let mut source = Vec::new();
        if with_bom {
            source.extend_from_slice(UTF8_BOM);
        }
        source.extend_from_slice(b"A=1\nB='x'\r\n# c\n\nraw line");
        let mut doc = EnvDocument::new(source);
        let start = doc.body_start();
        for (span, kind, terminator) in [
            (0..4, assignment("A", 2..3, QuoteStyle::Unquoted), Some(Lf)),
            (4..11, assignment("B", 3..4, QuoteStyle::Single), Some(CrLf)),
            (11..15, NodeKind::Comment, Some(Lf)),
            (15..16, NodeKind::Blank, Some(Lf)),
            (16..24, NodeKind::Raw, None),
        ] {
            doc.push_source_node(start + span.start..start + span.end, kind, terminator);
        }
        doc
    }

    #[test]
    fn covers_source_with_or_without_bom() {
        for with_bom in [false, true] {
            let doc = example(with_bom);
            assert_eq!(doc.has_bom(), with_bom);
            let start = if with_bom { 3 } else { 0 };
            assert_eq!(
                doc.nodes()[0].origin(),
                &NodeOrigin::Source(start..start + 4)
            );
            assert_eq!(doc.check_invariants(), Ok(()));
        }
        for bytes in [b"".as_slice(), UTF8_BOM] {
            assert_eq!(EnvDocument::new(bytes.to_vec()).check_invariants(), Ok(()));
        }
    }

    #[test]
    fn edits_preserve_source_coverage() {
        let mut doc = example(true);
        let first = doc.nodes()[0].id();
        let second = doc.nodes()[1].id();
        let inserted = doc.insert_synthetic_node(
            1,
            b"C=3\r".to_vec(),
            assignment("C", 2..3, QuoteStyle::Unquoted),
            Some(LineTerminator::Cr),
        );
        doc.node_mut(first)
            .unwrap()
            .set_replacement(Some(b"A=9\n".to_vec()));
        doc.node_mut(second).unwrap().set_removed(true);

        let node = doc.node(inserted).unwrap();
        assert!(node.is_synthetic());
        assert_eq!(doc.nodes()[1].id(), inserted);
        assert_eq!(doc.current_bytes(node), Some(b"C=3\r".as_slice()));
        assert_eq!(
            doc.current_bytes(doc.node(first).unwrap()),
            Some(b"A=9\n".as_slice())
        );
        assert!(doc.node(second).unwrap().is_removed());
        assert_eq!(doc.check_invariants(), Ok(()));
    }

    #[test]
    fn ids_follow_allocation_order() {
        let mut doc = EnvDocument::new(b"#a\n#b\n".to_vec());
        let first = doc.push_source_node(0..3, NodeKind::Comment, Some(LineTerminator::Lf));
        let second =
            doc.insert_synthetic_node(0, b"\n".to_vec(), NodeKind::Blank, Some(LineTerminator::Lf));
        let third = doc.push_source_node(3..6, NodeKind::Comment, Some(LineTerminator::Lf));
        let fourth = doc.insert_synthetic_node(usize::MAX, b"#c".to_vec(), NodeKind::Comment, None);

        assert!(first < second && second < third && third < fourth);
        assert_eq!(doc.nodes().last().unwrap().id(), fourth);
        assert_eq!(doc.check_invariants(), Ok(()));
    }

    #[test]
    fn detects_crlf_split_across_nodes() {
        let mut doc = EnvDocument::new(b"A=1\r\nB=2\n".to_vec());
        let first = doc.push_source_node(0..4, NodeKind::Raw, Some(LineTerminator::Cr));
        doc.push_source_node(4..9, NodeKind::Raw, Some(LineTerminator::Lf));
        assert_eq!(
            doc.check_invariants(),
            Err(SplitCrLf {
                node_id: first.get()
            })
        );
    }

    #[test]
    fn only_the_last_live_node_may_lack_a_terminator() {
        let mut doc = EnvDocument::new(b"A=1B=2\n".to_vec());
        let first = doc.push_source_node(0..3, NodeKind::Raw, None);
        doc.push_source_node(3..7, NodeKind::Raw, Some(LineTerminator::Lf));
        assert_eq!(
            doc.check_invariants(),
            Err(MissingTerminator {
                node_id: first.get()
            })
        );

        // SYNTAX.md SYN-08: deleting the last line of a file without a final
        // newline drops the terminator of the new last line.
        let mut doc = EnvDocument::new(b"A=1\nB=2".to_vec());
        let a = doc.push_source_node(
            0..4,
            assignment("A", 2..3, QuoteStyle::Unquoted),
            Some(LineTerminator::Lf),
        );
        let b = doc.push_source_node(4..7, assignment("B", 2..3, QuoteStyle::Unquoted), None);
        doc.node_mut(b).unwrap().set_removed(true);
        let node = doc.node_mut(a).unwrap();
        node.set_replacement(Some(b"A=1".to_vec()));
        node.terminator = None;
        assert_eq!(doc.check_invariants(), Ok(()));
    }

    #[test]
    fn detects_bom_mismatch() {
        let mut doc = example(true);
        doc.bom = false;
        assert_eq!(doc.check_invariants(), Err(BomMismatch));
    }

    #[test]
    fn detects_gaps_and_overlaps() {
        for start in [3, 5] {
            let mut doc = example(false);
            doc.nodes[1].origin = NodeOrigin::Source(start..11);
            assert_eq!(
                doc.check_invariants(),
                Err(SourceCoverage {
                    expected_start: 4,
                    found_start: start
                })
            );
        }
    }

    #[test]
    fn detects_incomplete_coverage() {
        let mut doc = example(false);
        doc.nodes.pop();
        assert_eq!(
            doc.check_invariants(),
            Err(SourceIncomplete {
                covered_until: 16,
                source_len: 24
            })
        );
    }

    #[test]
    fn detects_out_of_bounds_span() {
        let mut doc = example(false);
        let node_id = doc.nodes[4].id().get();
        doc.nodes[4].origin = NodeOrigin::Source(16..25);
        assert!(doc.current_bytes(&doc.nodes[4]).is_none());
        assert_eq!(doc.check_invariants(), Err(SpanOutOfBounds { node_id }));
    }

    #[test]
    fn detects_empty_source_span() {
        let mut doc = example(false);
        let node_id = doc.nodes[4].id().get();
        doc.nodes[4].origin = NodeOrigin::Source(16..16);
        assert_eq!(doc.check_invariants(), Err(EmptySourceSpan { node_id }));
    }

    #[test]
    fn detects_duplicate_id() {
        let mut doc = example(false);
        doc.nodes[4].id = doc.nodes[0].id;
        assert_eq!(
            doc.check_invariants(),
            Err(DuplicateId {
                node_id: doc.nodes[0].id().get()
            })
        );
    }

    #[test]
    fn detects_raw_replacement_and_removal() {
        let mut doc = example(false);
        let node_id = doc.nodes[4].id().get();
        doc.nodes[4].set_replacement(Some(b"replacement".to_vec()));
        assert_eq!(doc.check_invariants(), Err(RawNodeEdited { node_id }));
        doc.nodes[4].set_replacement(None);
        doc.nodes[4].set_removed(true);
        assert_eq!(doc.check_invariants(), Err(RawNodeEdited { node_id }));
    }

    #[test]
    fn detects_terminator_mismatches() {
        use LineTerminator::{Cr, CrLf, Lf};

        for (bytes, terminator) in [
            (b"# c\r\n".as_slice(), Some(Lf)),
            (b"# c\n", None),
            (b"# c\r", None),
            (b"# c", Some(Cr)),
            (b"# c\n", Some(CrLf)),
        ] {
            let mut doc = EnvDocument::new(Vec::new());
            let id = doc.insert_synthetic_node(0, bytes.to_vec(), NodeKind::Comment, terminator);
            assert_eq!(
                doc.check_invariants(),
                Err(TerminatorMismatch { node_id: id.get() })
            );
        }
    }

    #[test]
    fn detects_invalid_value_ranges() {
        for (bytes, range) in [
            ("A=é\n".as_bytes(), 2..6),
            ("A=é\n".as_bytes(), 2..3),
            ("A=é\n".as_bytes(), 2..5),
            (b"A=\xff\n", 2..3),
        ] {
            let mut doc = EnvDocument::new(bytes.to_vec());
            let id = doc.push_source_node(
                0..bytes.len(),
                assignment("A", range, QuoteStyle::Unquoted),
                Some(LineTerminator::Lf),
            );
            assert_eq!(
                doc.check_invariants(),
                Err(InvalidValueRange { node_id: id.get() })
            );
        }
    }

    fn assert_redacted(value: &impl std::fmt::Debug) {
        assert!(!format!("{value:?}").contains(SENTINEL));
        assert!(!format!("{value:#?}").contains(SENTINEL));
    }

    #[test]
    fn document_debug_redacts_all_user_data() {
        let bytes = format!("{SENTINEL}={SENTINEL}\n").into_bytes();
        let kind = assignment(
            SENTINEL,
            SENTINEL.len() + 1..bytes.len() - 1,
            QuoteStyle::Unquoted,
        );
        let mut doc = EnvDocument::new(bytes.clone());
        let id = doc.push_source_node(0..bytes.len(), kind.clone(), Some(LineTerminator::Lf));
        doc.insert_synthetic_node(1, bytes.clone(), kind, Some(LineTerminator::Lf));
        doc.node_mut(id).unwrap().set_replacement(Some(bytes));

        assert_eq!(doc.check_invariants(), Ok(()));
        assert_redacted(&doc);
        for node in doc.nodes() {
            assert_redacted(node);
            if let NodeKind::Assignment(syntax) = node.kind() {
                assert_redacted(syntax);
            }
        }
    }

    #[test]
    fn invariant_diagnostics_are_value_free() {
        for error in [
            BomMismatch,
            SourceCoverage {
                expected_start: 0,
                found_start: 1,
            },
            SourceIncomplete {
                covered_until: 0,
                source_len: 1,
            },
            SpanOutOfBounds { node_id: 0 },
            EmptySourceSpan { node_id: 0 },
            DuplicateId { node_id: 0 },
            RawNodeEdited { node_id: 0 },
            TerminatorMismatch { node_id: 0 },
            InvalidValueRange { node_id: 0 },
        ] {
            assert_redacted(&error);
            assert!(!error.to_string().contains(SENTINEL));
        }
    }
}
