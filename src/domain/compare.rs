//! Key-level diagnostics over parsed documents: duplicate keys and key set
//! comparison, and conflict summaries.
//!
//! Comparisons consider live assignments. Conflict review compares decoded
//! values in memory and returns only redacted keys, statuses, and a line count.

use std::collections::HashMap;

use super::{
    codec::decode,
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

/// A key mismatch, relative to the disk snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyChangeStatus {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChange {
    pub key: Redacted<String>,
    pub status: KeyChangeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictSummary {
    Structured {
        changes: Vec<KeyChange>,
        differing_lines: usize,
    },
    LinesOnly {
        differing_lines: usize,
    },
}

/// Compares live assignments using case-sensitive keys and ordered lists of
/// decoded values. A different value or occurrence count marks a key changed.
/// Quoting changes with the same decoded value only affect the line count.
/// Changes are ordered lexicographically by key.
pub fn compare_conflict(local: &EnvDocument, disk: &EnvDocument) -> ConflictSummary {
    let differing_lines = count_differing_lines(&local.serialize(), &disk.serialize());
    // Keys inside raw nodes are invisible to the comparison, so a structured
    // result would misreport them as added or removed.
    let has_raw = |doc: &EnvDocument| {
        doc.nodes()
            .iter()
            .any(|node| !node.is_removed() && matches!(node.kind(), NodeKind::Raw))
    };
    if has_raw(local) || has_raw(disk) {
        return ConflictSummary::LinesOnly { differing_lines };
    }

    let local_values = assignment_values(local);
    let mut disk_values = assignment_values(disk);
    let mut changes = Vec::new();
    for (key, values) in local_values {
        let status = match disk_values.remove(key) {
            None => KeyChangeStatus::Added,
            Some(disk_values) if disk_values != values => KeyChangeStatus::Changed,
            Some(_) => continue,
        };
        changes.push(KeyChange {
            key: Redacted::new(key.to_owned()),
            status,
        });
    }
    changes.extend(disk_values.into_keys().map(|key| KeyChange {
        key: Redacted::new(key.to_owned()),
        status: KeyChangeStatus::Removed,
    }));
    changes.sort_by(|a, b| a.key.expose().cmp(b.key.expose()));
    ConflictSummary::Structured {
        changes,
        differing_lines,
    }
}

fn assignment_values(doc: &EnvDocument) -> HashMap<&str, Vec<String>> {
    let mut values = HashMap::<&str, Vec<String>>::new();
    for node in doc.nodes().iter().filter(|node| !node.is_removed()) {
        let NodeKind::Assignment(syntax) = node.kind() else {
            continue;
        };
        let raw = doc
            .current_bytes(node)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|text| text.get(syntax.value_range.clone()));
        if let Some(raw) = raw {
            values
                .entry(syntax.key.expose())
                .or_default()
                .push(decode(syntax.quote, raw));
        }
    }
    values
}

/// Counts differing physical lines as the larger of the lines present only
/// locally and the lines present only on disk, matching equal lines as a
/// multiset. Inserting or removing a block counts only that block, wherever it
/// is. When only the order differs, it counts the lines between the common
/// leading and trailing lines, so differing inputs never report zero.
/// Terminators (LF, CRLF or CR) are part of each line, so line-ending changes
/// count. A final terminator does not create a trailing empty line. Linear
/// expected time; no full diff is computed.
pub fn count_differing_lines(local: &[u8], disk: &[u8]) -> usize {
    let mut balance = HashMap::<&[u8], isize>::new();
    for line in physical_lines(local) {
        *balance.entry(line).or_default() += 1;
    }
    for line in physical_lines(disk) {
        *balance.entry(line).or_default() -= 1;
    }
    let only_local: isize = balance.values().filter(|count| **count > 0).sum();
    let only_disk: isize = -balance.values().filter(|count| **count < 0).sum::<isize>();
    let unmatched = only_local.max(only_disk).unsigned_abs();
    if unmatched > 0 || local == disk {
        return unmatched;
    }
    let local: Vec<&[u8]> = physical_lines(local).collect();
    let disk: Vec<&[u8]> = physical_lines(disk).collect();
    let prefix = local.iter().zip(&disk).take_while(|(a, b)| a == b).count();
    let suffix = local
        .iter()
        .rev()
        .zip(disk.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    local.len().saturating_sub(prefix + suffix)
}

fn physical_lines(mut bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    std::iter::from_fn(move || {
        if bytes.is_empty() {
            return None;
        }
        let end = match bytes.iter().position(|byte| matches!(byte, b'\r' | b'\n')) {
            Some(at) if bytes.get(at..at + 2) == Some(b"\r\n") => at + 2,
            Some(at) => at + 1,
            None => bytes.len(),
        };
        let (line, rest) = bytes.split_at(end);
        bytes = rest;
        Some(line)
    })
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
    fn conflict_reports_only_mismatched_keys_and_redacts_debug() {
        let local = doc(&format!(
            "{SENTINEL}={SENTINEL}\nCHANGED={SENTINEL}\nDUP=1\nDUP=2\nORDER=1\nORDER=2\nQUOTED='{SENTINEL}'\nMULTI=\"{SENTINEL}\nlocal\"\n"
        ));
        let disk = doc(&format!(
            "{SENTINEL}_REMOVED={SENTINEL}\nCHANGED=other\nDUP=1\nORDER=2\nORDER=1\nQUOTED={SENTINEL}\nMULTI=\"{SENTINEL}\ndisk\"\n"
        ));
        let summary = compare_conflict(&local, &disk);
        let ConflictSummary::Structured {
            changes,
            differing_lines,
        } = &summary
        else {
            panic!("expected a structured comparison");
        };
        use KeyChangeStatus::{Added, Changed, Removed};
        let actual: Vec<_> = changes
            .iter()
            .map(|change| (change.key.expose().as_str(), change.status))
            .collect();
        let removed = format!("{SENTINEL}_REMOVED");
        assert!(
            actual
                == [
                    ("CHANGED", Changed),
                    ("DUP", Changed),
                    (SENTINEL, Added),
                    (removed.as_str(), Removed),
                    ("MULTI", Changed),
                    ("ORDER", Changed),
                ]
        );
        assert!(*differing_lines == 5);
        assert!(!format!("{summary:?} {summary:#?}").contains(SENTINEL));
        for change in changes {
            assert!(!format!("{change:?} {:?}", change.status).contains(SENTINEL));
        }
    }

    #[test]
    fn conflict_counts_raw_lines_separately_from_decoded_values() {
        let local = doc(&format!("MULTI=\"{SENTINEL}\r\nnext\"\r\n"));
        let disk = doc(&format!("MULTI=\"{SENTINEL}\nnext\"\n"));
        assert!(
            compare_conflict(&local, &disk)
                == ConflictSummary::Structured {
                    changes: Vec::new(),
                    differing_lines: 2,
                }
        );
        assert!(
            compare_conflict(&local, &doc(SENTINEL))
                == ConflictSummary::LinesOnly { differing_lines: 2 }
        );
        assert!(
            compare_conflict(&doc("A='x\nB=1\n"), &doc("A=y\nB=1\n"))
                == ConflictSummary::LinesOnly { differing_lines: 1 }
        );
        assert!(count_differing_lines(b"A\rB\r\nC\n", b"A\rC\n") == 1);
        assert!(count_differing_lines(b"NEW\nA\nB\nC\n", b"A\nB\nC\n") == 1);
        assert!(count_differing_lines(b"A\nX\nY\nC\n", b"A\nB\nC\n") == 2);
        assert!(count_differing_lines(b"A\nB\nC\n", b"B\nA\nC\n") == 2);
        assert!(count_differing_lines(b"", b"\n") == 1);
        assert!(count_differing_lines(b"A", b"") == 1);
        assert!(count_differing_lines(b"A\n", b"A") == 1);
        assert!(count_differing_lines(b"A\rB\r\nC\n", b"A\rB\r\nC\n") == 0);
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
