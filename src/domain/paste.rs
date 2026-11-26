//! Preview and stage a confirmed clipboard paste without changing the target.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use super::{
    document::{EnvDocument, NodeId, NodeKind},
    edit::{EditCommand, EditError, TerminatorChange},
    error::DomainError,
    parser::parse,
    redact::Redacted,
    syntax::{LineTerminator, split_lines},
};

pub const MAX_PASTE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteDecision {
    Add,
    ReplaceOccurrence(NodeId),
}

#[derive(Debug)]
pub struct PasteEntry {
    pub line: usize,
    pub key: Redacted<String>,
    /// Original assignment body, including internal line endings.
    pub assignment: Redacted<Vec<u8>>,
    pub occurrences: Arc<[NodeId]>,
    /// Every row sharing a key within the clipboard carries this warning.
    pub is_duplicate_in_paste: bool,
    pub decision: Option<PasteDecision>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PasteError {
    #[error("clipboard text exceeds the paste size limit")]
    TooLarge,
    #[error("the paste was cancelled")]
    Cancelled,
    #[error("confirm insertion of only the valid assignments")]
    PartialConfirmationRequired,
    #[error("the assignment at line {line} still needs a decision")]
    Unresolved { line: usize },
    #[error("there is no assignment at paste line {line}")]
    UnknownEntry { line: usize },
    #[error("the selected occurrence does not belong to the assignment at line {line}")]
    InvalidOccurrence { line: usize },
    #[error("the selected occurrence is already targeted by another pasted assignment")]
    RepeatedReplacement,
    #[error(transparent)]
    Parse(#[from] DomainError),
    #[error(transparent)]
    Edit(#[from] EditError),
}

/// A confirmed paste that can be committed after releasing the preview borrow.
#[derive(Debug)]
pub struct StagedPaste {
    base: EnvDocument,
    document: EnvDocument,
    commands: Vec<EditCommand>,
}

impl StagedPaste {
    pub(super) fn into_parts(self) -> (EnvDocument, EnvDocument, Vec<EditCommand>) {
        (self.base, self.document, self.commands)
    }
}

#[derive(Debug)]
pub struct PastePlan<'a> {
    target: &'a EnvDocument,
    entries: Vec<PasteEntry>,
    invalid_lines: Vec<usize>,
    ignored_lines: usize,
    is_partial_confirmed: bool,
    is_cancelled: bool,
}

impl<'a> PastePlan<'a> {
    pub fn new(document: &'a EnvDocument, pasted: &str) -> Result<Self, PasteError> {
        if pasted.len() > MAX_PASTE_BYTES {
            return Err(PasteError::TooLarge);
        }

        let clipboard = parse(pasted.as_bytes().to_vec())?;
        let mut occurrences = BTreeMap::<&str, Vec<NodeId>>::new();
        for node in document.nodes().iter().filter(|node| !node.is_removed()) {
            if let NodeKind::Assignment(syntax) = node.kind() {
                occurrences
                    .entry(syntax.key.expose())
                    .or_default()
                    .push(node.id());
            }
        }
        let occurrences: BTreeMap<&str, Arc<[NodeId]>> = occurrences
            .into_iter()
            .map(|(key, ids)| (key, ids.into()))
            .collect();
        let mut plan = Self {
            target: document,
            entries: Vec::new(),
            invalid_lines: Vec::new(),
            ignored_lines: 0,
            is_partial_confirmed: false,
            is_cancelled: false,
        };
        let mut line = 1;
        for node in clipboard.nodes() {
            let bytes = clipboard
                .current_bytes(node)
                .ok_or(EditError::NodeNotFound)?;
            let count = split_lines(bytes, 0).len();
            match node.kind() {
                NodeKind::Assignment(syntax) => {
                    let body = node
                        .terminator()
                        .and_then(|end| bytes.strip_suffix(end.as_bytes()))
                        .unwrap_or(bytes);
                    let matches = occurrences
                        .get(syntax.key.expose().as_str())
                        .cloned()
                        .unwrap_or_default();
                    plan.entries.push(PasteEntry {
                        line,
                        key: syntax.key.clone(),
                        assignment: Redacted::new(body.to_vec()),
                        decision: matches.is_empty().then_some(PasteDecision::Add),
                        occurrences: matches,
                        is_duplicate_in_paste: false,
                    });
                }
                NodeKind::Raw => plan.invalid_lines.extend(line..line + count),
                NodeKind::Comment | NodeKind::Blank => plan.ignored_lines += count,
            }
            line += count;
        }

        let mut counts = BTreeMap::<String, usize>::new();
        for entry in &plan.entries {
            *counts.entry(entry.key.expose().clone()).or_default() += 1;
        }
        for entry in &mut plan.entries {
            entry.is_duplicate_in_paste = counts.get(entry.key.expose()).is_some_and(|&n| n > 1);
        }
        Ok(plan)
    }

    pub fn entries(&self) -> &[PasteEntry] {
        &self.entries
    }

    pub fn invalid_lines(&self) -> &[usize] {
        &self.invalid_lines
    }

    pub fn ignored_lines(&self) -> usize {
        self.ignored_lines
    }

    pub fn decide(&mut self, line: usize, decision: PasteDecision) -> Result<(), PasteError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.line == line)
            .ok_or(PasteError::UnknownEntry { line })?;
        if let PasteDecision::ReplaceOccurrence(id) = decision
            && !entry.occurrences.contains(&id)
        {
            return Err(PasteError::InvalidOccurrence { line });
        }
        entry.decision = Some(decision);
        Ok(())
    }

    pub fn confirm_partial(&mut self) {
        self.is_partial_confirmed = true;
    }

    pub fn cancel(&mut self) {
        self.is_cancelled = true;
    }

    pub fn to_batch(&self) -> Result<EditCommand, PasteError> {
        self.stage().map(|(_, batch)| batch)
    }

    pub fn finish(self) -> Result<StagedPaste, PasteError> {
        let (document, commands) = self.stage_commands()?;
        Ok(StagedPaste {
            base: self.target.clone(),
            document,
            commands,
        })
    }

    /// Returns the resulting document and its batch for preview or inspection.
    /// A leading clipboard BOM is discarded by the parser. Internal pasted EOLs
    /// use the target's initial dominant style (SYN-06).
    pub fn stage(&self) -> Result<(EnvDocument, EditCommand), PasteError> {
        self.stage_commands()
            .map(|(document, commands)| (document, EditCommand::Batch(commands)))
    }

    fn stage_commands(&self) -> Result<(EnvDocument, Vec<EditCommand>), PasteError> {
        if self.is_cancelled {
            return Err(PasteError::Cancelled);
        }
        if !self.invalid_lines.is_empty() && !self.is_partial_confirmed {
            return Err(PasteError::PartialConfirmationRequired);
        }
        let mut replaced = BTreeSet::new();
        let decisions = self
            .entries
            .iter()
            .map(|entry| {
                let decision = entry
                    .decision
                    .ok_or(PasteError::Unresolved { line: entry.line })?;
                if let PasteDecision::ReplaceOccurrence(id) = decision
                    && !replaced.insert(id)
                {
                    return Err(PasteError::RepeatedReplacement);
                }
                Ok(decision)
            })
            .collect::<Result<Vec<_>, PasteError>>()?;

        let mut remaining_additions = decisions
            .iter()
            .filter(|&&d| d == PasteDecision::Add)
            .count();
        let dominant = self.target.dominant_terminator();
        let is_unterminated = self
            .target
            .nodes()
            .iter()
            .rfind(|node| !node.is_removed())
            .is_some_and(|node| node.terminator().is_none());
        let mut staged = self.target.clone();
        let mut commands = Vec::new();
        for (entry, decision) in self.entries.iter().zip(decisions) {
            let mut body = normalize(entry.assignment.expose(), dominant);
            match decision {
                PasteDecision::Add => {
                    remaining_additions -= 1;
                    // Only the final addition needs to inherit the target EOF state.
                    let terminator =
                        (!is_unterminated || remaining_additions > 0).then_some(dominant);
                    commands.push(append(&mut staged, body, dominant, terminator)?);
                }
                PasteDecision::ReplaceOccurrence(id) => {
                    // Replacing the full body keeps clipboard syntax and the selected
                    // node's ID, position and outer terminator.
                    let node = staged.node(id).ok_or(EditError::NodeNotFound)?;
                    if let Some(end) = node.terminator() {
                        body.extend_from_slice(end.as_bytes());
                    }
                    let before = staged.current_bytes(node).ok_or(EditError::NodeNotFound)?;
                    if before != body {
                        let before = Redacted::new(before.to_vec());
                        let kind = assignment_kind(&body)?;
                        let node = staged.node_mut(id).ok_or(EditError::NodeNotFound)?;
                        node.set_kind(kind);
                        node.set_replacement(Some(body.clone()));
                        commands.push(EditCommand::SetValue {
                            node_id: id,
                            before,
                            after: Redacted::new(body),
                        });
                    }
                }
            }
        }
        staged.check_invariants().map_err(EditError::Invariant)?;
        Ok((staged, commands))
    }
}

fn normalize(bytes: &[u8], dominant: LineTerminator) -> Vec<u8> {
    let mut body = Vec::with_capacity(bytes.len());
    for line in split_lines(bytes, 0) {
        if let Some(content) = bytes.get(line.span.start..line.content_end) {
            body.extend_from_slice(content);
        }
        if line.terminator.is_some() {
            body.extend_from_slice(dominant.as_bytes());
        }
    }
    body
}

fn assignment_kind(bytes: &[u8]) -> Result<NodeKind, PasteError> {
    let parsed = parse(bytes.to_vec())?;
    match parsed.nodes() {
        [node] if matches!(node.kind(), NodeKind::Assignment(_)) => Ok(node.kind().clone()),
        _ => Err(EditError::Unrepresentable.into()),
    }
}

fn append(
    doc: &mut EnvDocument,
    mut body: Vec<u8>,
    dominant: LineTerminator,
    terminator: Option<LineTerminator>,
) -> Result<EditCommand, PasteError> {
    if doc.unclosed_quote_line().is_some() {
        return Err(EditError::UnclosedQuote.into());
    }
    let last = doc.nodes().iter().rfind(|node| !node.is_removed());
    let neighbor = if let Some(last) = last.filter(|node| node.terminator().is_none()) {
        let id = last.id();
        let before = doc
            .current_bytes(last)
            .ok_or(EditError::NodeNotFound)?
            .to_vec();
        let mut after = before.clone();
        after.extend_from_slice(dominant.as_bytes());
        let last = doc.node_mut(id).ok_or(EditError::NodeNotFound)?;
        last.set_terminator(Some(dominant));
        last.set_replacement(Some(after.clone()));
        Some(TerminatorChange {
            node_id: id,
            before: Redacted::new(before),
            after: Redacted::new(after),
            before_terminator: None,
            after_terminator: Some(dominant),
        })
    } else {
        None
    };
    if let Some(end) = terminator {
        body.extend_from_slice(end.as_bytes());
    }
    let kind = assignment_kind(&body)?;
    let position = doc.nodes().len();
    let created_id = doc.insert_synthetic_node(position, body.clone(), kind, terminator);
    Ok(EditCommand::InsertAssignment {
        position,
        created_id,
        content: Redacted::new(body),
        neighbor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::SENTINEL;

    fn parsed(text: &str) -> EnvDocument {
        let result = parse(text.as_bytes().to_vec());
        assert!(result.is_ok());
        let Ok(doc) = result else { unreachable!() };
        doc
    }

    fn plan<'a>(doc: &'a EnvDocument, text: &str) -> PastePlan<'a> {
        let result = PastePlan::new(doc, text);
        assert!(result.is_ok());
        let Ok(plan) = result else { unreachable!() };
        plan
    }

    fn staged(plan: &PastePlan<'_>) -> (EnvDocument, Vec<EditCommand>) {
        let result = plan.stage();
        assert!(result.is_ok());
        let Ok((doc, EditCommand::Batch(commands))) = result else {
            unreachable!()
        };
        (doc, commands)
    }

    // Regression for quadratic history cost: a clipboard below the size limit
    // can produce hundreds of thousands of inserted nodes.
    #[test]
    fn large_paste_commits_and_undoes_in_bounded_time() {
        let doc = parsed("Z=1\n");
        let text = "A=\n".repeat(150_000);
        let result = plan(&doc, &text).finish();
        assert!(result.is_ok());
        let Ok(staged) = result else { unreachable!() };
        let history = crate::domain::history::EditHistory::new(doc.clone());
        assert!(history.is_ok());
        let Ok(mut history) = history else {
            unreachable!()
        };
        let started = std::time::Instant::now();
        assert!(history.commit_paste(staged).is_ok());
        assert!(history.undo().is_ok_and(|undone| undone));
        assert!(history.redo().is_ok_and(|redone| redone));
        assert!(history.undo().is_ok_and(|undone| undone));
        assert!(history.document().serialize() == doc.serialize());
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
    }

    #[test]
    fn each_collision_needs_a_choice_and_replacement_stays_in_place() {
        let doc = parsed("A=one\nB=other\nA=two");
        let mut plan = plan(&doc, "A=new\nB=more");
        let [a, b] = plan.entries() else {
            panic!("expected two entries")
        };
        let [_, chosen] = *a.occurrences else {
            panic!("expected two occurrences")
        };
        let [other] = *b.occurrences else {
            panic!("expected one occurrence")
        };
        assert!(matches!(
            plan.to_batch(),
            Err(PasteError::Unresolved { line: 1 })
        ));
        assert!(matches!(
            plan.decide(1, PasteDecision::ReplaceOccurrence(other)),
            Err(PasteError::InvalidOccurrence { line: 1 })
        ));
        assert!(
            plan.decide(1, PasteDecision::ReplaceOccurrence(chosen))
                .is_ok()
        );
        assert!(matches!(
            plan.to_batch(),
            Err(PasteError::Unresolved { line: 2 })
        ));
        assert!(plan.decide(2, PasteDecision::Add).is_ok());
        let (after, commands) = staged(&plan);
        assert!(after.serialize() == b"A=one\nB=other\nA=new\nB=more");
        assert!(matches!(commands.as_slice(), [
            EditCommand::SetValue { node_id, .. },
            EditCommand::InsertAssignment { .. }
        ] if *node_id == chosen));
        assert!(doc.serialize() == b"A=one\nB=other\nA=two");
        plan.cancel();
        assert!(matches!(plan.to_batch(), Err(PasteError::Cancelled)));
    }

    #[test]
    fn invalid_physical_lines_require_explicit_partial_confirmation() {
        let doc = parsed("");
        let mut plan = plan(&doc, "# note\n\nA=valid\nB='bad\nstill bad");
        assert!(plan.invalid_lines() == [4, 5]);
        assert!(plan.ignored_lines() == 2);
        assert!(matches!(
            plan.to_batch(),
            Err(PasteError::PartialConfirmationRequired)
        ));
        plan.confirm_partial();
        let (after, commands) = staged(&plan);
        assert!(commands.len() == 1);
        assert!(after.serialize() == b"A=valid\n");
    }

    #[test]
    fn clipboard_syntax_and_literals_survive_eol_normalization() {
        let doc = parsed("A=one\r\n");
        let plan = plan(&doc, "\u{feff} export B = '$VAR\r${VAR}\n$(cmd)' # note\n");
        let [entry] = plan.entries() else {
            panic!("expected one entry")
        };
        assert!(entry.assignment.expose() == b" export B = '$VAR\r${VAR}\n$(cmd)' # note");
        let (after, commands) = staged(&plan);
        let expected = b" export B = '$VAR\r\n${VAR}\r\n$(cmd)' # note\r\n";
        assert!(after.serialize() == [b"A=one\r\n".as_slice(), expected].concat());
        assert!(matches!(commands.as_slice(),
            [EditCommand::InsertAssignment { content, .. }] if content.expose() == expected));
    }

    #[test]
    fn same_style_multiline_body_is_preserved_and_duplicates_are_flagged() {
        let doc = parsed("START=x");
        let plan = plan(&doc, "export A=\"one\ntwo\"\nA=again");
        assert!(
            plan.entries()
                .iter()
                .all(|entry| entry.is_duplicate_in_paste)
        );
        let (after, _) = staged(&plan);
        assert!(after.serialize() == b"START=x\nexport A=\"one\ntwo\"\nA=again");
    }

    #[test]
    fn replacement_keeps_outer_eol_and_rejects_two_writes_to_one_occurrence() {
        let doc = parsed("A=1\nB=2\r\nC=3\r\n");
        let mut plan = plan(&doc, "export A='new\nvalue'\nA=other");
        let [first, _] = plan.entries() else {
            panic!("expected two entries")
        };
        let [id] = *first.occurrences else {
            panic!("expected one occurrence")
        };
        assert!(plan.decide(1, PasteDecision::ReplaceOccurrence(id)).is_ok());
        assert!(plan.decide(3, PasteDecision::ReplaceOccurrence(id)).is_ok());
        assert!(matches!(
            plan.to_batch(),
            Err(PasteError::RepeatedReplacement)
        ));
        assert!(plan.decide(3, PasteDecision::Add).is_ok());
        let (after, _) = staged(&plan);
        assert!(after.serialize() == b"export A='new\r\nvalue'\nB=2\r\nC=3\r\nA=other\r\n");
    }

    #[test]
    fn empty_paste_has_no_commands() {
        let doc = parsed("A=x");
        let (after, commands) = staged(&plan(&doc, ""));
        assert!(commands.is_empty());
        assert!(after == doc);
    }

    #[test]
    fn errors_and_debug_hide_sensitive_data_and_oversize_is_rejected() {
        let text = format!("{SENTINEL}={SENTINEL}\n?{SENTINEL}");
        let doc = parsed(&format!("TARGET={SENTINEL}\n"));
        let mut plan = plan(&doc, &text);
        assert!(!format!("{plan:#?}").contains(SENTINEL));
        let result = plan.to_batch();
        assert!(matches!(
            result,
            Err(PasteError::PartialConfirmationRequired)
        ));
        if let Err(error) = result {
            assert!(!error.to_string().contains(SENTINEL));
            assert!(!format!("{error:?}").contains(SENTINEL));
        }
        plan.confirm_partial();
        let result = plan.to_batch();
        assert!(result.is_ok());
        assert!(!format!("{result:#?}").contains(SENTINEL));
        let oversized = SENTINEL.repeat(MAX_PASTE_BYTES / SENTINEL.len() + 1);
        assert!(matches!(
            PastePlan::new(&doc, &oversized),
            Err(PasteError::TooLarge)
        ));
    }
}
