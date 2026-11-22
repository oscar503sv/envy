//! Key-level diagnostics over parsed documents: duplicate keys and key set
//! comparison.
//!
//! Everything here looks at live (non-removed) assignment nodes only and at
//! their keys only. Values are never read, compared or exposed, and keys are
//! held in [`Redacted`] so that Debug output of the results stays safe
//! (PRD §8.2). Duplicates are a diagnostic: which occurrence "wins" is a
//! consumer decision and is never computed here (SYNTAX §5.9).

use std::collections::HashMap;

use super::{
    document::{EnvDocument, NodeId, NodeKind},
    redact::Redacted,
};

/// A key that appears in more than one live assignment of a document.
// Derived Debug is safe because the key is wrapped in Redacted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateKey {
    key: Redacted<String>,
    ids: Vec<NodeId>,
}

impl DuplicateKey {
    pub fn key(&self) -> &Redacted<String> {
        &self.key
    }

    /// Number of live occurrences; always at least 2.
    pub fn count(&self) -> usize {
        self.ids.len()
    }

    /// Node IDs of the occurrences, in document order.
    pub fn ids(&self) -> &[NodeId] {
        &self.ids
    }
}

/// Live assignment keys grouped by key, ordered by first occurrence.
fn group_keys(doc: &EnvDocument) -> Vec<(&str, Vec<NodeId>)> {
    let mut groups: Vec<(&str, Vec<NodeId>)> = Vec::new();
    let mut positions: HashMap<&str, usize> = HashMap::new();
    for node in doc.nodes().iter().filter(|node| !node.is_removed()) {
        let NodeKind::Assignment(syntax) = node.kind() else {
            continue;
        };
        let key = syntax.key.expose().as_str();
        match positions.get(key).and_then(|&at| groups.get_mut(at)) {
            Some((_, ids)) => ids.push(node.id()),
            None => {
                positions.insert(key, groups.len());
                groups.push((key, vec![node.id()]));
            }
        }
    }
    groups
}

/// Returns the keys with two or more live occurrences.
///
/// Groups are ordered by the first occurrence of each key and the IDs inside a
/// group follow document order. Synthetic nodes count; removed nodes do not.
/// Key comparison is exact and case-sensitive.
pub fn duplicate_keys(doc: &EnvDocument) -> Vec<DuplicateKey> {
    group_keys(doc)
        .into_iter()
        .filter(|(_, ids)| ids.len() >= 2)
        .map(|(key, ids)| DuplicateKey {
            key: Redacted::new(key.to_owned()),
            ids,
        })
        .collect()
}

/// A key and how many live occurrences it has on each side of a comparison.
// Derived Debug is safe because the key is wrapped in Redacted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPresence {
    pub key: Redacted<String>,
    pub current_count: usize,
    pub reference_count: usize,
}

/// Result of [`compare_keys`].
// Derived Debug is safe because every entry redacts its key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyComparison {
    /// In the reference but not in the current document ("Faltan").
    pub missing: Vec<KeyPresence>,
    /// In the current document but not in the reference ("Solo en actual").
    pub only_in_current: Vec<KeyPresence>,
    /// In both documents ("En ambos").
    pub in_both: Vec<KeyPresence>,
}

/// Compares the key sets of `current` and `reference` (for example `.env`
/// against `.env.example`).
///
/// Order is deterministic: `missing` follows the reference's order of first
/// occurrence, while `only_in_current` and `in_both` follow the current
/// document's. Counts are live occurrences per side, so duplicates are visible
/// but never collapse the grouping. Only keys are compared; values are not
/// read.
pub fn compare_keys(current: &EnvDocument, reference: &EnvDocument) -> KeyComparison {
    let current_groups = group_keys(current);
    let reference_groups = group_keys(reference);
    let current_counts: HashMap<&str, usize> = current_groups
        .iter()
        .map(|(key, ids)| (*key, ids.len()))
        .collect();
    let reference_counts: HashMap<&str, usize> = reference_groups
        .iter()
        .map(|(key, ids)| (*key, ids.len()))
        .collect();

    let presence = |key: &str, current_count: usize, reference_count: usize| KeyPresence {
        key: Redacted::new(key.to_owned()),
        current_count,
        reference_count,
    };

    let mut result = KeyComparison::default();
    for (key, ids) in &current_groups {
        match reference_counts.get(key) {
            Some(&reference_count) => {
                result
                    .in_both
                    .push(presence(key, ids.len(), reference_count))
            }
            None => result.only_in_current.push(presence(key, ids.len(), 0)),
        }
    }
    for (key, ids) in &reference_groups {
        if !current_counts.contains_key(key) {
            result.missing.push(presence(key, 0, ids.len()));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    // `assert!(a == b)` on keys instead of `assert_eq!`: a failing `assert_eq!`
    // would print the key into the test output.
    use super::*;
    use crate::{
        domain::{
            parser::parse,
            syntax::{AssignmentSyntax, QuoteStyle},
        },
        test_support::SENTINEL,
    };

    fn doc(source: &str) -> EnvDocument {
        parse(source.as_bytes().to_vec()).unwrap()
    }

    fn assignment_ids(doc: &EnvDocument) -> Vec<NodeId> {
        doc.nodes()
            .iter()
            .filter(|node| matches!(node.kind(), NodeKind::Assignment(_)))
            .map(|node| node.id())
            .collect()
    }

    fn keys(entries: &[KeyPresence]) -> Vec<&str> {
        entries.iter().map(|e| e.key.expose().as_str()).collect()
    }

    fn counts(entries: &[KeyPresence]) -> Vec<(usize, usize)> {
        entries
            .iter()
            .map(|e| (e.current_count, e.reference_count))
            .collect()
    }

    fn synthetic_assignment(doc: &mut EnvDocument, key: &str) -> NodeId {
        let bytes = format!("{key}=x\n").into_bytes();
        let kind = NodeKind::Assignment(AssignmentSyntax {
            key: Redacted::new(key.to_owned()),
            value_range: key.len() + 1..key.len() + 2,
            quote: QuoteStyle::Unquoted,
            is_multiline: false,
            is_export: false,
        });
        doc.insert_synthetic_node(usize::MAX, bytes, kind, None)
    }

    #[test]
    fn duplicate_fixture_reports_only_repeated_keys() {
        let bytes = std::fs::read("tests/fixtures/duplicate-keys.env").unwrap();
        let document = parse(bytes).unwrap();
        let ids = assignment_ids(&document);

        let duplicates = duplicate_keys(&document);

        assert!(duplicates.len() == 1);
        let only = &duplicates[0];
        assert!(only.key().expose() == "A");
        assert!(only.count() == 3);
        assert!(only.ids() == [ids[0], ids[2], ids[3]]);
    }

    #[test]
    fn groups_follow_first_occurrence_and_keep_document_order() {
        let document = doc("B=1\nA=1\nB=2\nC=1\nA=2\nB=3\n");
        let ids = assignment_ids(&document);

        let duplicates = duplicate_keys(&document);

        let names: Vec<&str> = duplicates
            .iter()
            .map(|d| d.key().expose().as_str())
            .collect();
        assert!(names == ["B", "A"]);
        assert!(duplicates[0].ids() == [ids[0], ids[2], ids[5]]);
        assert!(duplicates[1].ids() == [ids[1], ids[4]]);
    }

    #[test]
    fn no_duplicates_and_empty_documents_return_nothing() {
        assert!(duplicate_keys(&doc("")).is_empty());
        assert!(duplicate_keys(&doc("A=1\nB=2\n# A=3\n")).is_empty());
    }

    #[test]
    fn keys_are_case_sensitive() {
        assert!(duplicate_keys(&doc("a=1\nA=2\n")).is_empty());
    }

    #[test]
    fn removed_nodes_do_not_count() {
        let mut document = doc("A=1\nA=2\nA=3\n");
        let ids = assignment_ids(&document);
        document.node_mut(ids[1]).unwrap().set_removed(true);

        let duplicates = duplicate_keys(&document);
        assert!(duplicates.len() == 1);
        assert!(duplicates[0].ids() == [ids[0], ids[2]]);

        document.node_mut(ids[2]).unwrap().set_removed(true);
        assert!(duplicate_keys(&document).is_empty());
    }

    #[test]
    fn synthetic_nodes_count() {
        let mut document = doc("A=1\n");
        let added = synthetic_assignment(&mut document, "A");
        let duplicates = duplicate_keys(&document);
        assert!(duplicates.len() == 1);
        assert!(duplicates[0].count() == 2);
        assert!(duplicates[0].ids().last() == Some(&added));
    }

    #[test]
    fn duplicate_debug_never_shows_keys() {
        let source = format!("{SENTINEL}_K=1\n{SENTINEL}_K=2\n");
        let output = format!("{:#?}", duplicate_keys(&doc(&source)));
        assert!(!output.contains(SENTINEL));
    }

    #[test]
    fn compare_splits_into_three_groups_with_counts() {
        let current = doc("A=1\nB=2\nB=3\nC=4\n");
        let reference = doc("D=1\nC=1\nA=1\nA=2\nE=1\n");

        let result = compare_keys(&current, &reference);

        assert!(keys(&result.missing) == ["D", "E"]);
        assert!(counts(&result.missing) == [(0, 1), (0, 1)]);
        assert!(keys(&result.only_in_current) == ["B"]);
        assert!(counts(&result.only_in_current) == [(2, 0)]);
        assert!(keys(&result.in_both) == ["A", "C"]);
        assert!(counts(&result.in_both) == [(1, 2), (1, 1)]);
    }

    #[test]
    fn compare_empty_documents() {
        let empty = doc("");
        let full = doc("A=1\nB=2\n");

        assert!(compare_keys(&empty, &empty) == KeyComparison::default());

        let only_reference = compare_keys(&empty, &full);
        assert!(keys(&only_reference.missing) == ["A", "B"]);
        assert!(only_reference.only_in_current.is_empty() && only_reference.in_both.is_empty());

        let only_current = compare_keys(&full, &empty);
        assert!(keys(&only_current.only_in_current) == ["A", "B"]);
        assert!(only_current.missing.is_empty() && only_current.in_both.is_empty());
    }

    #[test]
    fn compare_identical_and_disjoint() {
        let a = doc("A=1\nB=2\n");
        let identical = compare_keys(&a, &doc("B=9\nA=9\n"));
        assert!(identical.missing.is_empty() && identical.only_in_current.is_empty());
        assert!(keys(&identical.in_both) == ["A", "B"]);

        let disjoint = compare_keys(&a, &doc("X=1\nY=2\n"));
        assert!(disjoint.in_both.is_empty());
        assert!(keys(&disjoint.only_in_current) == ["A", "B"]);
        assert!(keys(&disjoint.missing) == ["X", "Y"]);
    }

    #[test]
    fn compare_ignores_raw_comment_and_blank_nodes() {
        let current = doc("# X=1\n\nA=1\nnot an assignment\n");
        let reference = doc("A=1\n# Y=2\n");
        let result = compare_keys(&current, &reference);
        assert!(keys(&result.in_both) == ["A"]);
        assert!(result.missing.is_empty() && result.only_in_current.is_empty());
    }

    #[test]
    fn compare_skips_removed_nodes_on_either_side() {
        let mut current = doc("A=1\nB=2\n");
        let b = assignment_ids(&current)[1];
        current.node_mut(b).unwrap().set_removed(true);
        let result = compare_keys(&current, &doc("A=1\nB=2\n"));
        assert!(keys(&result.missing) == ["B"]);

        let mut reference = doc("A=1\nB=2\n");
        let b = assignment_ids(&reference)[1];
        reference.node_mut(b).unwrap().set_removed(true);
        let result = compare_keys(&doc("A=1\nB=2\n"), &reference);
        assert!(keys(&result.only_in_current) == ["B"]);
        assert!(result.only_in_current[0].reference_count == 0);
        assert!(result.missing.is_empty());
    }

    #[test]
    fn compare_ignores_values() {
        let current = doc("A=one\n");
        let reference = doc("A=two\n");
        let result = compare_keys(&current, &reference);
        assert!(keys(&result.in_both) == ["A"]);
    }

    #[test]
    fn compare_debug_never_shows_values_or_keys() {
        let current = doc(&format!("SHARED={SENTINEL}\nONLY_CURRENT={SENTINEL}\n"));
        let reference = doc(&format!("SHARED={SENTINEL}_2\nMISSING={SENTINEL}\n"));
        let result = compare_keys(&current, &reference);

        let output = format!("{result:?} {result:#?}");
        assert!(!output.contains(SENTINEL));
        assert!(!output.contains("SHARED") && !output.contains("MISSING"));

        let key_sentinel = KeyPresence {
            key: Redacted::new(SENTINEL.to_owned()),
            current_count: 1,
            reference_count: 0,
        };
        assert!(!format!("{key_sentinel:?}").contains(SENTINEL));
    }
}
