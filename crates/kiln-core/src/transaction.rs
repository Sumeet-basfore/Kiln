//! Transactions and change sets.
//!
//! A transaction represents an atomic set of changes covering an entire document,
//! along with selection states before and after.

use std::cmp::min;
use std::ops::Range;

use kiln_text::Text;

use crate::error::CoreError;
use crate::selection::Selections;

/// Association preference when mapping positions through an insertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Assoc {
    /// Stay before text inserted at this position (new position = old position).
    Before,
    /// Move after text inserted at this position (new position = old position + insert length).
    After,
}

/// A primitive operation in a `ChangeSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Retain `n` characters from the source text.
    Retain(usize),
    /// Delete `n` characters from the source text.
    Delete(usize),
    /// Insert the given string.
    Insert(String),
}

/// A compact, ordered list of operations covering an entire document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangeSet {
    ops: Vec<Operation>,
}

impl ChangeSet {
    /// Creates an empty `ChangeSet`.
    #[must_use]
    pub const fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Creates a validated and normalized `ChangeSet` from a list of operations.
    #[must_use]
    pub fn from_ops(ops: Vec<Operation>) -> Self {
        Self {
            ops: normalize_ops(ops),
        }
    }

    /// Returns a slice of operations in this changeset.
    #[must_use]
    pub fn ops(&self) -> &[Operation] {
        &self.ops
    }

    /// Returns the length of the document before this changeset is applied.
    #[must_use]
    pub fn old_len(&self) -> usize {
        let mut len = 0;
        for op in &self.ops {
            match op {
                Operation::Retain(n) | Operation::Delete(n) => len += n,
                Operation::Insert(_) => {}
            }
        }
        len
    }

    /// Returns the length of the document after this changeset is applied.
    #[must_use]
    pub fn new_len(&self) -> usize {
        let mut len = 0;
        for op in &self.ops {
            match op {
                Operation::Retain(n) => len += n,
                Operation::Delete(_) => {}
                Operation::Insert(s) => len += s.chars().count(),
            }
        }
        len
    }

    /// Returns `true` if this changeset makes no modifications to the document.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ops.iter().all(|op| matches!(op, Operation::Retain(_)))
    }

    /// Applies this changeset to the given `Text`.
    ///
    /// # Errors
    /// Returns `CoreError::TransactionLengthMismatch` if `self.old_len() != text.len_chars()`.
    pub fn apply(&self, text: &Text) -> Result<Text, CoreError> {
        let doc_len = text.len_chars();
        let expected_len = self.old_len();
        if doc_len != expected_len {
            return Err(CoreError::TransactionLengthMismatch {
                tx_len: expected_len,
                doc_len,
            });
        }

        let mut result = Text::new();
        let mut cursor = 0;

        for op in &self.ops {
            match op {
                Operation::Retain(n) => {
                    if *n > 0 {
                        let slice = text.slice(cursor..(cursor + n)).map_err(|_| {
                            CoreError::InvalidChangeSet {
                                message: "Retain slice out of bounds",
                            }
                        })?;
                        result.insert(result.len_chars(), &slice).map_err(|_| {
                            CoreError::InvalidChangeSet {
                                message: "Insert retain slice failed",
                            }
                        })?;
                        cursor += n;
                    }
                }
                Operation::Delete(n) => {
                    cursor += n;
                }
                Operation::Insert(s) => {
                    result.insert(result.len_chars(), s).map_err(|_| {
                        CoreError::InvalidChangeSet {
                            message: "Insert failed",
                        }
                    })?;
                }
            }
        }

        Ok(result)
    }

    /// Computes the inverse of this changeset with respect to `old_text`.
    ///
    /// # Errors
    /// Returns `CoreError::TransactionLengthMismatch` if `self.old_len() != old_text.len_chars()`.
    pub fn invert(&self, old_text: &Text) -> Result<Self, CoreError> {
        let doc_len = old_text.len_chars();
        let expected_len = self.old_len();
        if doc_len != expected_len {
            return Err(CoreError::TransactionLengthMismatch {
                tx_len: expected_len,
                doc_len,
            });
        }

        let mut inv_ops = Vec::new();
        let mut cursor = 0;

        for op in &self.ops {
            match op {
                Operation::Retain(n) => {
                    inv_ops.push(Operation::Retain(*n));
                    cursor += n;
                }
                Operation::Delete(n) => {
                    let deleted_str = old_text.slice(cursor..(cursor + n)).map_err(|_| {
                        CoreError::InvalidChangeSet {
                            message: "Invert delete slice failed",
                        }
                    })?;
                    inv_ops.push(Operation::Insert(deleted_str));
                    cursor += n;
                }
                Operation::Insert(s) => {
                    inv_ops.push(Operation::Delete(s.chars().count()));
                }
            }
        }

        Ok(Self::from_ops(inv_ops))
    }

    /// Composes this changeset with `other`, producing a single equivalent changeset.
    ///
    /// `self` is applied first, then `other`.
    ///
    /// # Errors
    /// Returns `CoreError::TransactionLengthMismatch` if `self.new_len() != other.old_len()`.
    pub fn compose(&self, other: &Self) -> Result<Self, CoreError> {
        let intermediate_len = self.new_len();
        let other_in_len = other.old_len();
        if intermediate_len != other_in_len {
            return Err(CoreError::TransactionLengthMismatch {
                tx_len: other_in_len,
                doc_len: intermediate_len,
            });
        }

        let mut composed_ops = Vec::new();
        let mut a_idx = 0;
        let mut b_idx = 0;

        // Current sub-operations being matched
        let a_ops = &self.ops;
        let b_ops = &other.ops;

        let mut a_cur: Option<Operation> = a_ops.first().cloned();
        let mut b_cur: Option<Operation> = b_ops.first().cloned();

        while a_cur.is_some() || b_cur.is_some() {
            // Case 1: A has pure Delete; pass through to result immediately
            if let Some(Operation::Delete(n)) = a_cur {
                composed_ops.push(Operation::Delete(n));
                a_idx += 1;
                a_cur = a_ops.get(a_idx).cloned();
                continue;
            }

            // Case 2: B has pure Insert; pass through to result immediately
            if let Some(Operation::Insert(s)) = b_cur {
                composed_ops.push(Operation::Insert(s));
                b_idx += 1;
                b_cur = b_ops.get(b_idx).cloned();
                continue;
            }

            // Both A and B are touching the intermediate stream:
            // A has either Retain or Insert.
            // B has either Retain or Delete.
            match (&a_cur, &b_cur) {
                (Some(Operation::Retain(a_n)), Some(Operation::Retain(b_n))) => {
                    let take = min(*a_n, *b_n);
                    composed_ops.push(Operation::Retain(take));
                    if *a_n == take {
                        a_idx += 1;
                        a_cur = a_ops.get(a_idx).cloned();
                    } else {
                        a_cur = Some(Operation::Retain(*a_n - take));
                    }
                    if *b_n == take {
                        b_idx += 1;
                        b_cur = b_ops.get(b_idx).cloned();
                    } else {
                        b_cur = Some(Operation::Retain(*b_n - take));
                    }
                }
                (Some(Operation::Retain(a_n)), Some(Operation::Delete(b_n))) => {
                    let take = min(*a_n, *b_n);
                    composed_ops.push(Operation::Delete(take));
                    if *a_n == take {
                        a_idx += 1;
                        a_cur = a_ops.get(a_idx).cloned();
                    } else {
                        a_cur = Some(Operation::Retain(*a_n - take));
                    }
                    if *b_n == take {
                        b_idx += 1;
                        b_cur = b_ops.get(b_idx).cloned();
                    } else {
                        b_cur = Some(Operation::Delete(*b_n - take));
                    }
                }
                (Some(Operation::Insert(a_s)), Some(Operation::Retain(b_n))) => {
                    let a_char_count = a_s.chars().count();
                    let take = min(a_char_count, *b_n);
                    let prefix: String = a_s.chars().take(take).collect();
                    composed_ops.push(Operation::Insert(prefix));

                    if a_char_count == take {
                        a_idx += 1;
                        a_cur = a_ops.get(a_idx).cloned();
                    } else {
                        let rem: String = a_s.chars().skip(take).collect();
                        a_cur = Some(Operation::Insert(rem));
                    }
                    if *b_n == take {
                        b_idx += 1;
                        b_cur = b_ops.get(b_idx).cloned();
                    } else {
                        b_cur = Some(Operation::Retain(*b_n - take));
                    }
                }
                (Some(Operation::Insert(a_s)), Some(Operation::Delete(b_n))) => {
                    let a_char_count = a_s.chars().count();
                    let take = min(a_char_count, *b_n);
                    // Insert followed by Delete in intermediate stream cancels out!
                    if a_char_count == take {
                        a_idx += 1;
                        a_cur = a_ops.get(a_idx).cloned();
                    } else {
                        let rem: String = a_s.chars().skip(take).collect();
                        a_cur = Some(Operation::Insert(rem));
                    }
                    if *b_n == take {
                        b_idx += 1;
                        b_cur = b_ops.get(b_idx).cloned();
                    } else {
                        b_cur = Some(Operation::Delete(*b_n - take));
                    }
                }
                _ => break,
            }
        }

        Ok(Self::from_ops(composed_ops))
    }

    /// Maps a character position through this changeset.
    #[must_use]
    pub fn map_pos(&self, pos: usize, assoc: Assoc) -> usize {
        let mut old_offset = 0;
        let mut new_offset = 0;

        for op in &self.ops {
            if old_offset > pos {
                break;
            }

            match op {
                Operation::Retain(n) => {
                    let next_old = old_offset + n;
                    if pos < next_old {
                        return new_offset + (pos - old_offset);
                    }
                    if pos == next_old {
                        // At boundary of retain
                        new_offset += n;
                        old_offset = next_old;
                        continue;
                    }
                    old_offset = next_old;
                    new_offset += n;
                }
                Operation::Delete(n) => {
                    let next_old = old_offset + n;
                    if pos <= next_old {
                        // Position lands inside deleted range; collapses to deletion point
                        return new_offset;
                    }
                    old_offset = next_old;
                }
                Operation::Insert(s) => {
                    let insert_len = s.chars().count();
                    if old_offset == pos {
                        if assoc == Assoc::After {
                            new_offset += insert_len;
                        }
                    } else {
                        new_offset += insert_len;
                    }
                }
            }
        }

        new_offset
    }
}

/// Normalizes a sequence of operations:
/// - Strips zero-length Retain, Delete, and empty Inserts.
/// - Merges consecutive operations of the same kind.
fn normalize_ops(raw: Vec<Operation>) -> Vec<Operation> {
    let mut normalized = Vec::with_capacity(raw.len());

    for op in raw {
        match op {
            Operation::Retain(0) | Operation::Delete(0) => continue,
            Operation::Insert(ref s) if s.is_empty() => continue,
            _ => {}
        }

        if let Some(last) = normalized.last_mut() {
            match (last, &op) {
                (Operation::Retain(a), Operation::Retain(b)) => {
                    *a += b;
                    continue;
                }
                (Operation::Delete(a), Operation::Delete(b)) => {
                    *a += b;
                    continue;
                }
                (Operation::Insert(a), Operation::Insert(b)) => {
                    a.push_str(b);
                    continue;
                }
                _ => {}
            }
        }

        normalized.push(op);
    }

    normalized
}

/// The origin of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TransactionOrigin {
    /// Regular user editing (insert mode, normal mode edit command).
    #[default]
    UserEdit,
    /// Undo operation.
    Undo,
    /// Redo operation.
    Redo,
    /// Edit performed by an external AI agent.
    AgentEdit,
    /// External disk change reload.
    ExternalReload,
}

/// A transaction carrying a changeset and selection contexts before and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    /// The document changeset.
    pub changes: ChangeSet,
    /// Selections before the change.
    pub selections_before: Option<Selections>,
    /// Selections after the change.
    pub selections_after: Option<Selections>,
    /// Origin of this transaction.
    pub origin: TransactionOrigin,
}

impl Transaction {
    /// Creates a new `Transaction` from a `ChangeSet`.
    #[must_use]
    pub fn new(changes: ChangeSet) -> Self {
        Self {
            changes,
            selections_before: None,
            selections_after: None,
            origin: TransactionOrigin::UserEdit,
        }
    }

    /// Sets the selection states before and after.
    #[must_use]
    pub fn with_selections(
        mut self,
        before: Option<Selections>,
        after: Option<Selections>,
    ) -> Self {
        self.selections_before = before;
        self.selections_after = after;
        self
    }

    /// Sets the origin of this transaction.
    #[must_use]
    pub fn with_origin(mut self, origin: TransactionOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Inverts this transaction against `old_text`.
    ///
    /// Swaps `selections_before` and `selections_after`.
    ///
    /// # Errors
    /// Returns `CoreError::TransactionLengthMismatch` if changeset lengths do not match.
    pub fn invert(&self, old_text: &Text) -> Result<Self, CoreError> {
        let inv_changes = self.changes.invert(old_text)?;
        Ok(Self {
            changes: inv_changes,
            selections_before: self.selections_after.clone(),
            selections_after: self.selections_before.clone(),
            origin: match self.origin {
                TransactionOrigin::Undo => TransactionOrigin::Redo,
                _ => TransactionOrigin::Undo,
            },
        })
    }

    /// Composes this transaction with `other`.
    ///
    /// # Errors
    /// Returns `CoreError::TransactionLengthMismatch` if lengths do not align.
    pub fn compose(&self, other: &Self) -> Result<Self, CoreError> {
        let composed_changes = self.changes.compose(&other.changes)?;
        Ok(Self {
            changes: composed_changes,
            selections_before: self.selections_before.clone(),
            selections_after: other.selections_after.clone(),
            origin: other.origin,
        })
    }

    /// Maps a character offset through this transaction.
    #[must_use]
    pub fn map_pos(&self, pos: usize, assoc: Assoc) -> usize {
        self.changes.map_pos(pos, assoc)
    }

    // -------------------------------------------------------------------------
    // Builders
    // -------------------------------------------------------------------------

    /// Builds a transaction that inserts `text` at `pos`.
    ///
    /// # Errors
    /// Returns `CoreError::PositionOutOfBounds` if `pos > doc_len`.
    pub fn insert_at(doc_len: usize, pos: usize, text: &str) -> Result<Self, CoreError> {
        if pos > doc_len {
            return Err(CoreError::PositionOutOfBounds { pos, max: doc_len });
        }
        let mut ops = Vec::new();
        if pos > 0 {
            ops.push(Operation::Retain(pos));
        }
        if !text.is_empty() {
            ops.push(Operation::Insert(text.to_string()));
        }
        if doc_len > pos {
            ops.push(Operation::Retain(doc_len - pos));
        }
        Ok(Self::new(ChangeSet::from_ops(ops)))
    }

    /// Builds a transaction that deletes character `range`.
    ///
    /// # Errors
    /// Returns `CoreError::InvalidRange` if `range.start > range.end`, or
    /// `CoreError::PositionOutOfBounds` if `range.end > doc_len`.
    pub fn delete_range(doc_len: usize, range: Range<usize>) -> Result<Self, CoreError> {
        if range.start > range.end {
            return Err(CoreError::InvalidRange {
                start: range.start,
                end: range.end,
            });
        }
        if range.end > doc_len {
            return Err(CoreError::PositionOutOfBounds {
                pos: range.end,
                max: doc_len,
            });
        }
        let mut ops = Vec::new();
        if range.start > 0 {
            ops.push(Operation::Retain(range.start));
        }
        let del_len = range.end - range.start;
        if del_len > 0 {
            ops.push(Operation::Delete(del_len));
        }
        if doc_len > range.end {
            ops.push(Operation::Retain(doc_len - range.end));
        }
        Ok(Self::new(ChangeSet::from_ops(ops)))
    }

    /// Builds a transaction that replaces `range` with `text`.
    ///
    /// # Errors
    /// Returns `CoreError::InvalidRange` or `CoreError::PositionOutOfBounds`.
    pub fn replace_range(
        doc_len: usize,
        range: Range<usize>,
        text: &str,
    ) -> Result<Self, CoreError> {
        if range.start > range.end {
            return Err(CoreError::InvalidRange {
                start: range.start,
                end: range.end,
            });
        }
        if range.end > doc_len {
            return Err(CoreError::PositionOutOfBounds {
                pos: range.end,
                max: doc_len,
            });
        }
        let mut ops = Vec::new();
        if range.start > 0 {
            ops.push(Operation::Retain(range.start));
        }
        let del_len = range.end - range.start;
        if del_len > 0 {
            ops.push(Operation::Delete(del_len));
        }
        if !text.is_empty() {
            ops.push(Operation::Insert(text.to_string()));
        }
        if doc_len > range.end {
            ops.push(Operation::Retain(doc_len - range.end));
        }
        Ok(Self::new(ChangeSet::from_ops(ops)))
    }

    /// Builds a transaction from a list of sorted, non-overlapping `(range, text)` edits.
    ///
    /// # Errors
    /// Returns `CoreError::UnsortedOrOverlappingEdits` if edits are not strictly sorted
    /// and non-overlapping, or `CoreError::PositionOutOfBounds` if an edit exceeds `doc_len`.
    pub fn from_edits(doc_len: usize, edits: &[(Range<usize>, &str)]) -> Result<Self, CoreError> {
        let mut ops = Vec::new();
        let mut cursor = 0;

        for (range, text) in edits {
            if range.start > range.end || range.start < cursor {
                return Err(CoreError::UnsortedOrOverlappingEdits);
            }
            if range.end > doc_len {
                return Err(CoreError::PositionOutOfBounds {
                    pos: range.end,
                    max: doc_len,
                });
            }

            if range.start > cursor {
                ops.push(Operation::Retain(range.start - cursor));
            }

            let del_len = range.end - range.start;
            if del_len > 0 {
                ops.push(Operation::Delete(del_len));
            }
            if !text.is_empty() {
                ops.push(Operation::Insert((*text).to_string()));
            }

            cursor = range.end;
        }

        if cursor < doc_len {
            ops.push(Operation::Retain(doc_len - cursor));
        }

        Ok(Self::new(ChangeSet::from_ops(ops)))
    }
}
