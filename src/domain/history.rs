//! Document-owned undo and redo for confirmed edits.

use std::collections::BTreeSet;

use super::{
    document::{EnvDocument, EnvNode, InvariantViolation, NodeId},
    edit::{
        ConversionPolicy, EditCommand, EditError, SetValueOutcome, insert_assignment,
        remove_assignment, set_value,
    },
    paste::StagedPaste,
    redact::Redacted,
};

/// An edit request. Applying it produces a command only if it changes the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditRequest<'a> {
    SetValue {
        node_id: NodeId,
        value: Redacted<&'a str>,
        policy: ConversionPolicy,
    },
    InsertAssignment {
        key: Redacted<&'a str>,
        value: Redacted<&'a str>,
        policy: ConversionPolicy,
    },
    RemoveAssignment {
        node_id: NodeId,
    },
}

impl EditRequest<'_> {
    fn apply(self, document: &mut EnvDocument) -> Result<Option<EditCommand>, EditError> {
        match self {
            Self::SetValue {
                node_id,
                value,
                policy,
            } => match set_value(document, node_id, value.into_inner(), policy)? {
                SetValueOutcome::Unchanged => Ok(None),
                SetValueOutcome::Applied(command) => Ok(Some(command)),
            },
            Self::InsertAssignment { key, value, policy } => {
                insert_assignment(document, key.into_inner(), value.into_inner(), policy).map(Some)
            }
            Self::RemoveAssignment { node_id } => remove_assignment(document, node_id).map(Some),
        }
    }
}

// Full node states also preserve parsed syntax, terminators and retired blanks.
#[derive(Debug)]
struct NodeChange {
    before: EnvNode,
    after: EnvNode,
}

#[derive(Debug)]
struct Entry {
    changes: Vec<NodeChange>,
}

impl Entry {
    fn replay(&self, document: &mut EnvDocument, is_redo: bool) -> Result<(), EditError> {
        let mut staged = document.clone();
        let positions = staged.node_positions();
        for change in &self.changes {
            let snapshot = if is_redo {
                &change.after
            } else {
                &change.before
            };
            let index = positions
                .get(&snapshot.id())
                .copied()
                .ok_or(EditError::NodeNotFound)?;
            *staged.node_at_mut(index).ok_or(EditError::NodeNotFound)? = snapshot.clone();
        }
        staged.check_invariants().map_err(EditError::Invariant)?;
        *document = staged;
        Ok(())
    }
}

/// Owns one document and exposes it through a shared reference so that every
/// mutation passes through its history and replay cannot target another document.
#[derive(Debug)]
pub struct EditHistory {
    document: EnvDocument,
    saved: Redacted<Vec<u8>>,
    undo: Vec<Entry>,
    redo: Vec<Entry>,
}

impl EditHistory {
    /// Starts with the document's current bytes as the confirmed snapshot.
    pub fn new(document: EnvDocument) -> Result<Self, InvariantViolation> {
        document.check_invariants()?;
        Ok(Self {
            saved: Redacted::new(document.serialize()),
            document,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    pub fn document(&self) -> &EnvDocument {
        &self.document
    }

    pub fn is_dirty(&self) -> bool {
        self.document.serialize().as_slice() != self.saved.expose()
    }

    /// Records the current bytes after the caller confirms a successful save.
    pub fn mark_saved(&mut self) {
        self.saved = Redacted::new(self.document.serialize());
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn apply(&mut self, request: EditRequest<'_>) -> Result<Option<EditCommand>, EditError> {
        self.apply_requests([request], false)
    }

    /// Applies requests in order on a staged document, committing one history
    /// entry only if all requests succeed and at least one changes a node.
    pub fn apply_batch<'a>(
        &mut self,
        requests: impl IntoIterator<Item = EditRequest<'a>>,
    ) -> Result<Option<EditCommand>, EditError> {
        self.apply_requests(requests, true)
    }

    pub fn commit_paste(&mut self, paste: StagedPaste) -> Result<Option<EditCommand>, EditError> {
        let (base, staged, commands) = paste.into_parts();
        // Include IDs and tombstones because byte-identical documents can
        // have different occurrence targets or reserved IDs.
        if self.document != base {
            return Err(EditError::StalePaste);
        }
        self.commit(staged, commands, true)
    }

    fn apply_requests<'a>(
        &mut self,
        requests: impl IntoIterator<Item = EditRequest<'a>>,
        is_batch: bool,
    ) -> Result<Option<EditCommand>, EditError> {
        let mut staged = self.document.clone();
        let mut commands = Vec::new();
        for request in requests {
            if let Some(command) = request.apply(&mut staged)? {
                commands.push(command);
            }
        }
        self.commit(staged, commands, is_batch)
    }

    fn commit(
        &mut self,
        staged: EnvDocument,
        mut commands: Vec<EditCommand>,
        is_batch: bool,
    ) -> Result<Option<EditCommand>, EditError> {
        if commands.is_empty() {
            return Ok(None);
        }
        staged.check_invariants().map_err(EditError::Invariant)?;
        let mut ids = BTreeSet::new();
        for command in &commands {
            record_affected_ids(command, &mut ids);
        }
        let staged_positions = staged.node_positions();
        let base_positions = self.document.node_positions();
        let changes = ids
            .into_iter()
            .map(|id| {
                let after = staged_positions
                    .get(&id)
                    .and_then(|&index| staged.nodes().get(index))
                    .ok_or(EditError::NodeNotFound)?
                    .clone();
                // Retain a tombstone for each undone insert. Its ID stays reserved
                // even when another edit discards the redo stack.
                let before = base_positions
                    .get(&id)
                    .and_then(|&index| self.document.nodes().get(index))
                    .cloned()
                    .unwrap_or_else(|| {
                        let mut inserted = after.clone();
                        inserted.set_removed(true);
                        inserted
                    });
                Ok(NodeChange { before, after })
            })
            .collect::<Result<_, EditError>>()?;
        self.document = staged;
        self.undo.push(Entry { changes });
        self.redo.clear();
        Ok(if is_batch {
            Some(EditCommand::Batch(commands))
        } else {
            commands.pop()
        })
    }

    /// Returns whether a history entry was undone.
    pub fn undo(&mut self) -> Result<bool, EditError> {
        replay(&mut self.document, &mut self.undo, &mut self.redo, false)
    }

    /// Returns whether a history entry was redone.
    pub fn redo(&mut self) -> Result<bool, EditError> {
        replay(&mut self.document, &mut self.redo, &mut self.undo, true)
    }
}

fn record_affected_ids(command: &EditCommand, ids: &mut BTreeSet<NodeId>) {
    let (id, neighbor) = match command {
        EditCommand::SetValue { node_id, .. } => (*node_id, None),
        EditCommand::InsertAssignment {
            created_id,
            neighbor,
            ..
        } => (*created_id, neighbor.as_ref()),
        EditCommand::RemoveAssignment {
            node_id, neighbor, ..
        } => (*node_id, neighbor.as_ref()),
        EditCommand::Batch(commands) => {
            for command in commands {
                record_affected_ids(command, ids);
            }
            return;
        }
    };
    ids.insert(id);
    if let Some(neighbor) = neighbor {
        ids.insert(neighbor.node_id);
    }
}

fn replay(
    document: &mut EnvDocument,
    from: &mut Vec<Entry>,
    to: &mut Vec<Entry>,
    is_redo: bool,
) -> Result<bool, EditError> {
    let Some(entry) = from.last() else {
        return Ok(false);
    };
    entry.replay(document, is_redo)?;
    to.extend(from.pop());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::parser::parse, test_support::SENTINEL};

    const POLICY: ConversionPolicy = ConversionPolicy::ConvertToDouble;

    fn history(bytes: &[u8]) -> EditHistory {
        EditHistory::new(parse(bytes.to_vec()).unwrap()).unwrap()
    }

    fn insert(key: &str) -> EditRequest<'_> {
        EditRequest::InsertAssignment {
            key: Redacted::new(key),
            value: Redacted::new("2"),
            policy: POLICY,
        }
    }

    #[test]
    fn set_value_restores_kind_and_tracks_saved_bytes() {
        let mut history = history(b"A='one'");
        let before = history.document().nodes().first().unwrap().clone();
        assert!(!history.is_dirty());
        history
            .apply(EditRequest::SetValue {
                node_id: before.id(),
                value: Redacted::new("it's changed"),
                policy: POLICY,
            })
            .unwrap();
        let after = history.document().nodes().first().unwrap().clone();
        assert!(before.kind() != after.kind());
        assert!(history.is_dirty());
        assert!(history.undo().unwrap());
        assert!(!history.is_dirty());
        assert!(history.document().node(before.id()) == Some(&before));
        assert!(history.redo().unwrap());
        history.mark_saved();
        assert!(!history.is_dirty());
        assert!(history.undo().unwrap());
        assert!(history.is_dirty());
        assert!(history.redo().unwrap());
        assert!(!history.is_dirty());
        assert!(history.document().node(after.id()) == Some(&after));
    }

    #[test]
    fn insert_reuses_id_and_restores_raw_neighbor() {
        let mut history = history(b"A=1\r\nraw line");
        let command = history.apply(insert("B")).unwrap().unwrap();
        let EditCommand::InsertAssignment {
            created_id,
            neighbor: Some(_),
            ..
        } = command
        else {
            panic!("expected insertion with a boundary change");
        };
        for _ in 0..3 {
            assert!(history.undo().unwrap());
            assert!(history.document().serialize() == b"A=1\r\nraw line");
            assert!(history.document().node(created_id).unwrap().is_removed());
            assert!(history.redo().unwrap());
            assert!(history.document().serialize() == b"A=1\r\nraw line\r\nB=2");
            assert!(!history.document().node(created_id).unwrap().is_removed());
        }
    }

    #[test]
    fn removal_restores_neighbors_including_retired_blank() {
        for (input, expected) in [
            (b"A=1\r\nB=2".as_slice(), b"A=1".as_slice()),
            (b"A=1\r\n\r\nB=2", b"A=1\r\n"),
            (b"raw line\r\nB=2", b"raw line"),
        ] {
            let mut history = history(input);
            let before = history.document().clone();
            let node_id = before.nodes().last().unwrap().id();
            history
                .apply(EditRequest::RemoveAssignment { node_id })
                .unwrap();
            assert!(history.document().serialize() == expected);
            assert!(history.undo().unwrap());
            assert!(history.document() == &before);
            assert!(history.redo().unwrap());
            assert!(history.document().serialize() == expected);
        }
    }

    #[test]
    fn batch_replays_once_and_new_apply_clears_redo() {
        let mut history = history(b"A=1");
        let node_id = history.document().nodes().first().unwrap().id();
        let command = history
            .apply_batch([
                insert("B"),
                EditRequest::RemoveAssignment { node_id },
                insert("C"),
            ])
            .unwrap();
        assert!(matches!(command, Some(EditCommand::Batch(commands)) if commands.len() == 3));
        assert!(history.document().serialize() == b"B=2\nC=2");
        let after = history.document().clone();
        history.apply(insert("D")).unwrap();
        assert!(history.undo().unwrap());
        assert!(history.undo().unwrap());
        assert!(history.document().serialize() == b"A=1");
        assert!(!history.undo().unwrap());
        assert!(history.redo().unwrap());
        assert!(history.document().serialize() == after.serialize());
        assert!(history.redo().unwrap());
        assert!(history.document().serialize() == b"B=2\nC=2\nD=2");
        assert!(history.undo().unwrap());
        history.apply(insert("E")).unwrap();
        assert!(!history.redo().unwrap());
    }

    #[test]
    fn failed_batch_preserves_document_and_both_stacks() {
        let mut history = history(b"A=1");
        history.apply(insert("B")).unwrap();
        history.apply(insert("C")).unwrap();
        assert!(history.undo().unwrap());
        let before = history.document().clone();
        assert!(matches!(
            history.apply_batch([insert("VALID"), insert("invalid-key")]),
            Err(EditError::InvalidKey)
        ));
        assert!(history.document() == &before);
        assert!(history.can_undo() && history.can_redo());
        assert!(history.undo().unwrap());
        assert!(history.document().serialize() == b"A=1");
        assert!(history.redo().unwrap());
        assert!(history.document() == &before);
        assert!(history.redo().unwrap());
        assert!(history.document().serialize() == b"A=1\nB=2\nC=2");
    }

    #[test]
    fn dirty_depends_on_bytes_and_noop_preserves_redo() {
        let mut history = history(b"A=1");
        let node_id = history.document().nodes().first().unwrap().id();
        let set = |value| EditRequest::SetValue {
            node_id,
            value: Redacted::new(value),
            policy: POLICY,
        };
        history.apply(set("2")).unwrap();
        history.apply(set("1")).unwrap();
        assert!(history.can_undo());
        assert!(!history.is_dirty());
        assert!(history.undo().unwrap());
        assert!(history.apply(set("2")).unwrap().is_none());
        assert!(history.can_redo());
        assert!(history.redo().unwrap());
        assert!(!history.is_dirty());
    }

    #[test]
    fn history_request_and_batch_debug_hide_secrets() {
        let mut history = history(SENTINEL.as_bytes());
        let request = EditRequest::InsertAssignment {
            key: Redacted::new(SENTINEL),
            value: Redacted::new(SENTINEL),
            policy: POLICY,
        };
        assert!(!format!("{request:#?}").contains(SENTINEL));
        let command = history.apply_batch([request]).unwrap();
        history.mark_saved();
        history.undo().unwrap();
        assert!(!format!("{history:#?} {command:#?}").contains(SENTINEL));
    }
}
