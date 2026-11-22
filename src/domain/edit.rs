//! Edit commands over an [`EnvDocument`] (TD §6.1).
//!
//! `SetValue` rewrites only the value bytes of one assignment (SYNTAX §6.2).
//! Everything else in the node (prefix, `export`, key, separator, trailing
//! comment, terminator) stays byte-identical. A quote-style conversion happens
//! only when the caller passes [`ConversionPolicy::ConvertToDouble`].
//!
//! Commands hold the node's bytes in memory, so they hold secrets: they must
//! never be written to logs or disk. Their Debug output is redacted.

use super::{
    codec::{NeedsConversion, decode, encode_in_node},
    document::{EnvDocument, InvariantViolation, NodeId, NodeKind},
    parser::parse,
    redact::Redacted,
    syntax::{AssignmentSyntax, QuoteStyle},
};

/// Whether `set_value` may change the quote style of the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionPolicy {
    /// A value the current style cannot hold is an error.
    Reject,
    /// Rewrite the quotes of the node as double quotes when needed.
    ConvertToDouble,
}

/// An applied edit, with enough data to undo and redo it (ENV-T019).
///
/// `before` and `after` are the node's full bytes, terminator included.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditCommand {
    SetValue {
        node_id: NodeId,
        before: Redacted<Vec<u8>>,
        after: Redacted<Vec<u8>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetValueOutcome {
    /// The value already displays as requested; nothing changed (SYN-41).
    Unchanged,
    Applied(EditCommand),
}

/// Carries no value data, so Debug and Display are safe to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EditError {
    #[error("the node does not exist")]
    NodeNotFound,
    #[error("the node was removed")]
    NodeRemoved,
    #[error("the node is not an assignment")]
    NotAnAssignment,
    #[error("the value contains a character that no quote style can hold")]
    Unrepresentable,
    #[error(transparent)]
    NeedsConversion(#[from] NeedsConversion),
    #[error("the edit would break a document invariant: {0}")]
    Invariant(InvariantViolation),
}

/// Sets the displayed value of an assignment, rewriting only its value bytes.
///
/// On any error the document is left unchanged.
pub fn set_value(
    doc: &mut EnvDocument,
    node_id: NodeId,
    value: &str,
    policy: ConversionPolicy,
) -> Result<SetValueOutcome, EditError> {
    let broken = || {
        EditError::Invariant(InvariantViolation::InvalidValueRange {
            node_id: node_id.get(),
        })
    };

    let node = doc.node(node_id).ok_or(EditError::NodeNotFound)?;
    if node.is_removed() {
        return Err(EditError::NodeRemoved);
    }
    let NodeKind::Assignment(syntax) = node.kind() else {
        return Err(EditError::NotAnAssignment);
    };
    let before = doc.current_bytes(node).ok_or_else(broken)?.to_vec();
    let text = std::str::from_utf8(&before).map_err(|_| broken())?;
    let raw = text.get(syntax.value_range.clone()).ok_or_else(broken)?;
    if decode(syntax.quote, raw) == value {
        return Ok(SetValueOutcome::Unchanged);
    }
    // Rejected before any conversion: the line would become `Raw`.
    if value.contains('\0') {
        return Err(EditError::Unrepresentable);
    }

    let dominant = doc.dominant_terminator();
    let new_text = match encode_in_node(text, syntax, value, dominant) {
        Ok(encoded) => encoded,
        Err(needs) => match policy {
            ConversionPolicy::Reject => return Err(needs.into()),
            ConversionPolicy::ConvertToDouble => {
                let template = double_quoted_template(text, syntax).ok_or_else(broken)?;
                let template_syntax = reparse(&template).ok_or_else(broken)?;
                encode_in_node(&template, &template_syntax, value, dominant)?
            }
        },
    };
    let new_syntax = reparse(&new_text).ok_or_else(broken)?;
    let after = new_text.into_bytes();

    let is_original = doc.node(node_id).is_some_and(|node| {
        matches!(node.origin(), super::document::NodeOrigin::Source(span)
            if doc.source().get(span.clone()) == Some(after.as_slice()))
    });
    let previous_kind = NodeKind::Assignment(syntax.clone());
    let previous_replacement = doc
        .node(node_id)
        .and_then(|node| node.replacement())
        .map(|bytes| bytes.expose().clone());

    let node = doc.node_mut(node_id).ok_or(EditError::NodeNotFound)?;
    node.set_replacement((!is_original).then(|| after.clone()));
    node.set_kind(NodeKind::Assignment(new_syntax));
    if let Err(violation) = doc.check_invariants() {
        if let Some(node) = doc.node_mut(node_id) {
            node.set_replacement(previous_replacement);
            node.set_kind(previous_kind);
        }
        return Err(EditError::Invariant(violation));
    }

    Ok(SetValueOutcome::Applied(EditCommand::SetValue {
        node_id,
        before: Redacted::new(before),
        after: Redacted::new(after),
    }))
}

/// Parses `text` as a single assignment node and returns its syntax.
fn reparse(text: &str) -> Option<AssignmentSyntax> {
    let document = parse(text.as_bytes().to_vec()).ok()?;
    let [node] = document.nodes() else {
        return None;
    };
    match node.kind() {
        NodeKind::Assignment(syntax) => Some(syntax.clone()),
        _ => None,
    }
}

/// Rewrites the node's value delimiters as double quotes around a placeholder.
///
/// An unquoted value gains quotes at both ends of its range; a single-quoted
/// one has its two quotes swapped. Prefix, trailing comment and terminator are
/// kept byte-identical. The old content is dropped (a real terminator stands in
/// for it when the value was multiline, so the node stays multiline); the
/// caller then encodes the new value into the template.
fn double_quoted_template(text: &str, syntax: &AssignmentSyntax) -> Option<String> {
    let range = &syntax.value_range;
    let (prefix, suffix) = match syntax.quote {
        QuoteStyle::Unquoted => (text.get(..range.start)?, text.get(range.end..)?),
        QuoteStyle::Single => {
            let open = range.start.checked_sub(1)?;
            let close = range.end;
            if text.get(open..range.start)? != "'" || text.get(close..close + 1)? != "'" {
                return None;
            }
            (text.get(..open)?, text.get(close + 1..)?)
        }
        QuoteStyle::Double => return None,
    };
    let content = if syntax.is_multiline {
        // Only the line break matters; the final suffix terminator is untouched.
        "\n"
    } else {
        ""
    };
    Some(format!("{prefix}\"{content}\"{suffix}"))
}

#[cfg(test)]
mod tests {
    // `assert!(a == b)` instead of `assert_eq!` on content: a failing
    // `assert_eq!` would print values, which is itself a leak.
    use super::*;
    use crate::test_support::SENTINEL;
    use ConversionPolicy::{ConvertToDouble, Reject};

    fn doc(source: &str) -> EnvDocument {
        parse(source.as_bytes().to_vec()).unwrap()
    }

    fn id(doc: &EnvDocument, index: usize) -> NodeId {
        doc.nodes()[index].id()
    }

    /// Edits node `index` and returns the serialized document.
    fn edit(source: &str, index: usize, value: &str, policy: ConversionPolicy) -> String {
        let mut d = doc(source);
        let node = id(&d, index);
        let outcome = set_value(&mut d, node, value, policy);
        assert!(outcome.is_ok(), "edit of {source:?} failed");
        assert!(d.check_invariants().is_ok());
        String::from_utf8(d.serialize()).unwrap()
    }

    fn displayed(d: &EnvDocument, index: usize) -> String {
        let node = &d.nodes()[index];
        let NodeKind::Assignment(syntax) = node.kind() else {
            panic!("not an assignment");
        };
        let text = std::str::from_utf8(d.current_bytes(node).unwrap()).unwrap();
        decode(syntax.quote, text.get(syntax.value_range.clone()).unwrap())
    }

    #[test]
    fn keeps_each_quote_style() {
        for (source, value, expected) in [
            ("A=1\n", "two", "A=two\n"),
            ("A='1'\n", "two words", "A='two words'\n"),
            ("A=\"1\"\n", "a\"b\\c", "A=\"a\\\"b\\\\c\"\n"),
            ("A=\"1\"\n", "x\ny", "A=\"x\\ny\"\n"),
            ("A=\"1\r\n2\"\r\n", "x\ny", "A=\"x\r\ny\"\r\n"),
            ("A='1\n2'\n", "x\ny\nz", "A='x\ny\nz'\n"),
            ("A=1\n", "é世😀", "A=é世😀\n"),
            ("A='é'\n", "世", "A='世'\n"),
            ("export A = 1 # c\n", "2", "export A = 2 # c\n"),
            ("A= # n\n", "x", "A=x # n\n"),
            ("A=1", "2", "A=2"),
            ("A=1\r\nB=2\r\n", "9", "A=9\r\nB=2\r\n"),
            ("A=1\rB=2\r", "9", "A=9\rB=2\r"),
        ] {
            assert!(
                edit(source, 0, value, Reject) == expected,
                "style case {source:?}"
            );
        }
    }

    #[test]
    fn reject_reports_the_reason_and_leaves_the_document_alone() {
        for (source, value) in [
            ("A=1\n", "a b "),
            ("A=1\n", " a"),
            ("A=1\n", "a\nb"),
            ("A=1 # c\n", "a\rb"),
            ("A='1'\n", "it's"),
            ("A='1'\n", "a\rb"),
        ] {
            let mut d = doc(source);
            let node = id(&d, 0);
            let result = set_value(&mut d, node, value, Reject);
            assert!(
                matches!(result, Err(EditError::NeedsConversion(_))),
                "reject case {source:?}"
            );
            assert!(d.serialize() == source.as_bytes());
            assert!(d.nodes()[0].replacement().is_none());
        }
    }

    #[test]
    fn unquoted_value_with_hash_roundtrips_without_conversion_or_converts() {
        // ` #` would start a comment, so the round-trip guard asks for conversion.
        let mut d = doc("A=1\n");
        let node = id(&d, 0);
        assert!(matches!(
            set_value(&mut d, node, "a #b", Reject),
            Err(EditError::NeedsConversion(_))
        ));
        assert!(edit("A=1\n", 0, "a #b", ConvertToDouble) == "A=\"a #b\"\n");
    }

    #[test]
    fn conversion_changes_only_the_delimiters() {
        for (source, value, expected) in [
            ("A=1\n", "a b ", "A=\"a b \"\n"),
            ("A=1 # note\r\n", "a\nb", "A=\"a\\nb\" # note\r\n"),
            ("export A = 1\t# c\n", " x", "export A = \" x\"\t# c\n"),
            ("A= # n\n", " x", "A=\" x\" # n\n"),
            ("A=\n", "\ty", "A=\"\ty\"\n"),
            ("A=1", "a\nb", "A=\"a\\nb\""),
            ("A='1' # c\n", "it's", "A=\"it's\" # c\n"),
            ("A='1'\r\n", "a\rb", "A=\"a\\rb\"\r\n"),
            ("export A='1'  # c", "q\"'", "export A=\"q\\\"'\"  # c"),
            // A multiline source keeps its multiline layout.
            ("A='1\n2' # c\n", "it's\nok", "A=\"it's\nok\" # c\n"),
            ("A='1\r\n2'\r\n", "it's\nok", "A=\"it's\r\nok\"\r\n"),
        ] {
            assert!(
                edit(source, 0, value, ConvertToDouble) == expected,
                "conversion case {source:?}"
            );
        }
    }

    #[test]
    fn conversion_updates_the_node_syntax() {
        let mut d = doc("A=1 # c\n");
        let node = id(&d, 0);
        set_value(&mut d, node, "a b ", ConvertToDouble).unwrap();
        let NodeKind::Assignment(syntax) = d.node(node).unwrap().kind() else {
            panic!("not an assignment");
        };
        assert!(syntax.quote == QuoteStyle::Double);
        assert!(displayed(&d, 0) == "a b ");
        // A second edit now keeps the double quotes without conversion.
        assert!(matches!(
            set_value(&mut d, node, "c d ", Reject),
            Ok(SetValueOutcome::Applied(_))
        ));
        assert!(d.serialize() == b"A=\"c d \" # c\n");
    }

    #[test]
    fn nul_is_unrepresentable_under_both_policies() {
        for source in ["A=1\n", "A='1'\n", "A=\"1\"\n", "A='1\n2'\n"] {
            for policy in [Reject, ConvertToDouble] {
                for value in ["\0", "a\0b", "it's \0"] {
                    let mut d = doc(source);
                    let node = id(&d, 0);
                    assert!(
                        set_value(&mut d, node, value, policy) == Err(EditError::Unrepresentable)
                    );
                    assert!(d.serialize() == source.as_bytes());
                    assert!(
                        d.nodes()[0].kind().tag() == crate::domain::error::NodeKindTag::Assignment
                    );
                }
            }
        }
    }

    #[test]
    fn same_displayed_value_is_unchanged_even_if_raw_differs() {
        for (source, value) in [
            ("A=1\n", "1"),
            ("A=\n", ""),
            ("A='x y'\n", "x y"),
            ("A=\"a\\qb\"\n", "a\\qb"),
            ("A=\"a\\$b\"\n", "a\\$b"),
            ("A=\"a\tb\"\n", "a\tb"),
            ("A=\"a\\tb\"\n", "a\tb"),
            ("A='1\r\n2'\r\n", "1\n2"),
            ("A=\"é\"\n", "é"),
        ] {
            let mut d = doc(source);
            let node = id(&d, 0);
            for policy in [Reject, ConvertToDouble] {
                assert!(
                    set_value(&mut d, node, value, policy) == Ok(SetValueOutcome::Unchanged),
                    "unchanged case {source:?}"
                );
            }
            assert!(d.nodes()[0].replacement().is_none());
            assert!(d.serialize() == source.as_bytes());
        }
    }

    #[test]
    fn setting_back_the_original_clears_the_replacement() {
        for (source, other) in [
            ("A=1\n", "2"),
            ("A=\"a\\qb\"\n", "z"),
            ("A='1' # c\r\n", "it's"),
        ] {
            let mut d = doc(source);
            let node = id(&d, 0);
            let original = displayed(&d, 0);
            set_value(&mut d, node, other, ConvertToDouble).unwrap();
            assert!(d.nodes()[0].replacement().is_some());
            // The canonical encoding of the old value may differ from the raw
            // bytes (`\q`), so only canonical sources return to None.
            set_value(&mut d, node, &original, ConvertToDouble).unwrap();
            assert!(displayed(&d, 0) == original);
        }
        let mut d = doc("A=1\nB=2\n");
        let node = id(&d, 0);
        set_value(&mut d, node, "9", Reject).unwrap();
        set_value(&mut d, node, "1", Reject).unwrap();
        assert!(d.nodes()[0].replacement().is_none());
        assert!(d.serialize() == b"A=1\nB=2\n");
    }

    #[test]
    fn only_the_chosen_duplicate_changes() {
        let source = "A=1\nB=2\nA=3\n# between\nA=\"4\"\n";
        for (index, expected) in [
            (0, "A=x\nB=2\nA=3\n# between\nA=\"4\"\n"),
            (2, "A=1\nB=2\nA=x\n# between\nA=\"4\"\n"),
            (4, "A=1\nB=2\nA=3\n# between\nA=\"x\"\n"),
        ] {
            assert!(edit(source, index, "x", Reject) == expected);
        }
    }

    #[test]
    fn synthetic_nodes_are_edited_in_place() {
        let mut d = doc("A=1\n");
        let synthetic = d.insert_synthetic_node(
            1,
            b"B='x'\n".to_vec(),
            reparse_kind("B='x'\n"),
            Some(crate::domain::syntax::LineTerminator::Lf),
        );
        let outcome = set_value(&mut d, synthetic, "it's", ConvertToDouble).unwrap();
        assert!(matches!(outcome, SetValueOutcome::Applied(_)));
        assert!(d.serialize() == b"A=1\nB=\"it's\"\n");
        assert!(d.check_invariants().is_ok());
        set_value(&mut d, synthetic, "y", Reject).unwrap();
        assert!(d.serialize() == b"A=1\nB=\"y\"\n");
    }

    fn reparse_kind(text: &str) -> NodeKind {
        NodeKind::Assignment(reparse(text).unwrap())
    }

    #[test]
    fn command_holds_the_node_bytes_before_and_after() {
        let mut d = doc("A=1 # c\r\nB=2\r\n");
        let node = id(&d, 0);
        let Ok(SetValueOutcome::Applied(EditCommand::SetValue {
            node_id,
            before,
            after,
        })) = set_value(&mut d, node, "9", Reject)
        else {
            panic!("expected an applied edit");
        };
        assert!(node_id == node);
        assert!(before.expose() == b"A=1 # c\r\n");
        assert!(after.expose() == b"A=9 # c\r\n");
    }

    #[test]
    fn conversion_regressions_from_qa() {
        for (source, value, expected) in [
            // CR-only document with a multiline single-quoted value.
            ("K='a\rb'\r", "it's\rok", "K=\"it's\\rok\"\r"),
            ("K='a\rb'\r", "it's\nok", "K=\"it's\rok\"\r"),
            // Q-12: whitespace and a trailing comment around an empty value.
            ("KEY=  # n\n", " x", "KEY=\" x\"  # n\n"),
            // `export` plus a multiline single-quoted value.
            (
                "export K='a\nb' # c\n",
                "it's\nok",
                "export K=\"it's\nok\" # c\n",
            ),
        ] {
            assert!(
                edit(source, 0, value, ConvertToDouble) == expected,
                "conversion case {source:?}"
            );
        }
    }

    #[test]
    fn splicing_the_command_bytes_undoes_and_redoes_the_edit() {
        let source = "A=1\nexport K='a\nb' # c\r\nZ=9";
        let mut d = doc(source);
        let node = id(&d, 1);
        let Ok(SetValueOutcome::Applied(EditCommand::SetValue { before, after, .. })) =
            set_value(&mut d, node, "it's", ConvertToDouble)
        else {
            panic!("expected an applied edit");
        };
        let edited = d.serialize();
        let start = source.find("export").unwrap();
        let end = start + before.expose().len();
        let mut undone = edited.clone();
        let after_end = start + after.expose().len();
        undone.splice(start..after_end, before.expose().iter().copied());
        assert!(undone == source.as_bytes());
        let mut redone = source.as_bytes().to_vec();
        redone.splice(start..end, after.expose().iter().copied());
        assert!(redone == edited);
        assert!(parse(undone).unwrap().check_invariants().is_ok());
    }

    #[test]
    fn conversion_is_sticky_across_sequential_edits() {
        let mut d = doc("A=1\n");
        let node = id(&d, 0);
        assert!(set_value(&mut d, node, "a b ", ConvertToDouble).is_ok());
        assert!(set_value(&mut d, node, "1", Reject).is_ok());
        assert!(d.serialize() == b"A=\"1\"\n");
        assert!(d.nodes()[0].replacement().is_some());
        assert!(displayed(&d, 0) == "1");
    }

    #[test]
    fn non_assignments_removed_and_unknown_nodes_are_errors() {
        let source = "# c\n\nraw line\nA=1\n";
        let mut d = doc(source);
        let (comment, blank, raw, assignment) = (id(&d, 0), id(&d, 1), id(&d, 2), id(&d, 3));
        for node in [comment, blank, raw] {
            assert!(
                set_value(&mut d, node, "x", ConvertToDouble) == Err(EditError::NotAnAssignment)
            );
        }
        d.node_mut(assignment).unwrap().set_removed(true);
        let after_removal = d.serialize();
        assert!(set_value(&mut d, assignment, "x", Reject) == Err(EditError::NodeRemoved));
        assert!(d.serialize() == after_removal);

        let other = doc("A=1\nB=2\nC=3\nD=4\n");
        let unknown = id(&other, 3);
        let mut small = doc("A=1\n");
        let before = small.serialize();
        assert!(set_value(&mut small, unknown, "x", Reject) == Err(EditError::NodeNotFound));
        assert!(small.serialize() == before);
    }

    #[test]
    fn unclosed_quote_region_is_not_an_assignment() {
        let source = "A=1\nB=\"open\nC=2\n";
        let mut d = doc(source);
        for index in 1..d.nodes().len() {
            let node = id(&d, index);
            assert!(
                set_value(&mut d, node, "x", ConvertToDouble) == Err(EditError::NotAnAssignment)
            );
        }
        assert!(d.serialize() == source.as_bytes());
    }

    #[test]
    fn bom_documents_keep_the_bom() {
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice(b"A=1\n");
        let mut d = parse(bytes).unwrap();
        let node = id(&d, 0);
        set_value(&mut d, node, "2", Reject).unwrap();
        assert!(d.serialize() == b"\xEF\xBB\xBFA=2\n");
    }

    #[test]
    fn formatting_never_leaks_the_value() {
        let mut d = doc("A=1\n");
        let node = id(&d, 0);
        let value = format!("{SENTINEL} 'x' #");
        let mut outputs = Vec::new();
        let applied = set_value(&mut d, node, &value, ConvertToDouble).unwrap();
        outputs.push(format!("{applied:?}"));
        if let SetValueOutcome::Applied(command) = &applied {
            outputs.push(format!("{command:?}"));
            outputs.push(format!("{command:#?}"));
        }
        outputs.push(format!("{:?}", SetValueOutcome::Unchanged));
        let mut other = doc("A=1\n");
        let node = id(&other, 0);
        let errors = [
            set_value(&mut other, node, &value, Reject).unwrap_err(),
            set_value(&mut other, node, &format!("{SENTINEL}\0"), Reject).unwrap_err(),
            EditError::NodeNotFound,
            EditError::NotAnAssignment,
            EditError::Invariant(InvariantViolation::InvalidValueRange { node_id: 0 }),
        ];
        for error in errors {
            outputs.push(format!("{error:?}"));
            outputs.push(format!("{error}"));
        }
        for output in outputs {
            assert!(!output.contains(SENTINEL), "formatting leaked the value");
        }
    }
}
