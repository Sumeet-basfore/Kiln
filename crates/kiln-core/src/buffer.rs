//! Buffer model.
//!
//! A `Buffer` encapsulates the text rope, selections, marks, undo tree,
//! versioning, encoding, line ending format, and dirty state.

use std::path::{Path, PathBuf};

use kiln_text::{Encoding, LineEnding, Text};

use crate::error::CoreError;
use crate::history::{NodeId, UndoTree};
use crate::marks::Marks;
use crate::selection::Selections;
use crate::transaction::{Transaction, TransactionOrigin};

/// An opaque identifier for a `Buffer`, unique within an editor session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId(u64);

impl BufferId {
    /// Creates a new `BufferId` from a raw number.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw integer value of this buffer ID.
    #[must_use]
    pub const fn as_raw(&self) -> u64 {
        self.0
    }
}

/// Allocates process-unique `BufferId` values without global static state.
#[derive(Debug, Default)]
pub struct BufferIdAllocator {
    next: u64,
}

impl BufferIdAllocator {
    /// Creates a new `BufferIdAllocator` starting at ID 1.
    #[must_use]
    pub const fn new() -> Self {
        Self { next: 1 }
    }

    /// Allocates and returns the next unique `BufferId`.
    pub fn allocate(&mut self) -> BufferId {
        let id = self.next;
        self.next += 1;
        BufferId(id)
    }
}

/// The result returned when a transaction is applied to a `Buffer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    /// Version before the change.
    pub old_version: u64,
    /// Version after the change.
    pub new_version: u64,
    /// The transaction that was applied.
    pub transaction: Transaction,
}

/// Pending state for an active undo group.
#[derive(Debug, Clone)]
struct GroupState {
    start_text: Text,
    selections_before: Option<Selections>,
    transactions: Vec<Transaction>,
}

/// An editor buffer.
#[derive(Debug, Clone)]
pub struct Buffer {
    id: BufferId,
    text: Text,
    path: Option<PathBuf>,
    encoding: Encoding,
    line_ending: LineEnding,
    version: u64,
    selections: Selections,
    marks: Marks,
    history: UndoTree,
    read_only: bool,
    saved_node: Option<NodeId>,
    group_depth: usize,
    active_group: Option<GroupState>,
}

impl Buffer {
    /// Creates a new `Buffer` with initial text.
    #[must_use]
    pub fn new(id: BufferId, text: Text) -> Self {
        Self {
            id,
            text,
            path: None,
            encoding: Encoding::Utf8,
            line_ending: LineEnding::Lf,
            version: 1,
            selections: Selections::point(0),
            marks: Marks::new(),
            history: UndoTree::new(),
            read_only: false,
            saved_node: None,
            group_depth: 0,
            active_group: None,
        }
    }

    /// Sets the file path associated with this buffer.
    #[must_use]
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Sets the text encoding of this buffer.
    #[must_use]
    pub fn with_encoding(mut self, encoding: Encoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// Sets the line ending format of this buffer.
    #[must_use]
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Sets whether the buffer is read-only.
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    /// Returns the unique ID of this buffer.
    #[must_use]
    pub fn id(&self) -> BufferId {
        self.id
    }

    /// Returns a reference to the buffer's text rope.
    #[must_use]
    pub fn text(&self) -> &Text {
        &self.text
    }

    /// Returns a reference to the buffer's file path, if set.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns the detected or configured character encoding.
    #[must_use]
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Returns the detected or configured line ending format.
    #[must_use]
    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Returns the monotonically increasing version number of the buffer.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns a reference to the active selections.
    #[must_use]
    pub fn selections(&self) -> &Selections {
        &self.selections
    }

    /// Sets the active selections directly.
    pub fn set_selections(&mut self, selections: Selections) {
        self.selections = selections;
    }

    /// Returns a reference to the buffer's marks.
    #[must_use]
    pub fn marks(&self) -> &Marks {
        &self.marks
    }

    /// Returns a mutable reference to the buffer's marks.
    pub fn marks_mut(&mut self) -> &mut Marks {
        &mut self.marks
    }

    /// Returns a reference to the undo history tree.
    #[must_use]
    pub fn history(&self) -> &UndoTree {
        &self.history
    }

    /// Returns `true` if the buffer is marked read-only.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Returns `true` if the buffer has unsaved changes.
    ///
    /// The buffer is clean only when:
    /// - No undo group is currently open with pending changes.
    /// - The current undo history node matches `saved_node`.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        if self.group_depth > 0 {
            if let Some(ref group) = self.active_group {
                if !group.transactions.is_empty() {
                    return true;
                }
            }
        }
        self.history.current() != self.saved_node
    }

    /// Marks the current buffer state as saved.
    ///
    /// # Errors
    /// Returns `CoreError::GroupOpenConflict` if called while an undo group is open.
    pub fn mark_saved(&mut self) -> Result<(), CoreError> {
        if self.group_depth > 0 {
            return Err(CoreError::GroupOpenConflict);
        }
        self.saved_node = self.history.current();
        Ok(())
    }

    /// Begins an undo group.
    ///
    /// Multiple transactions applied during an undo group will be composed into
    /// a single node in the undo history when `end_group` is called.
    pub fn begin_group(&mut self) {
        if self.group_depth == 0 {
            self.active_group = Some(GroupState {
                start_text: self.text.clone(),
                selections_before: Some(self.selections.clone()),
                transactions: Vec::new(),
            });
        }
        self.group_depth += 1;
    }

    /// Ends an undo group.
    ///
    /// Composes accumulated transactions into a single node in the history.
    ///
    /// # Errors
    /// Returns `CoreError::UnbalancedGroupEnd` if no group was open.
    pub fn end_group(&mut self) -> Result<(), CoreError> {
        if self.group_depth == 0 {
            return Err(CoreError::UnbalancedGroupEnd);
        }

        self.group_depth -= 1;
        if self.group_depth == 0 {
            if let Some(group) = self.active_group.take() {
                if !group.transactions.is_empty() {
                    // Compose all transactions in order
                    let mut combined_tx = group.transactions[0].clone();
                    for next_tx in &group.transactions[1..] {
                        combined_tx = combined_tx.compose(next_tx)?;
                    }

                    // Selections before = first transaction's selections_before or group start
                    combined_tx.selections_before = group.transactions[0]
                        .selections_before
                        .clone()
                        .or(group.selections_before);

                    // Selections after = last transaction's selections_after
                    combined_tx.selections_after = group
                        .transactions
                        .last()
                        .and_then(|t| t.selections_after.clone())
                        .or_else(|| Some(self.selections.clone()));

                    // Compute inverse against start_text
                    let inverse = combined_tx.invert(&group.start_text)?;
                    self.history.commit(combined_tx, inverse);
                }
            }
        }

        Ok(())
    }

    /// Applies a transaction to the buffer.
    ///
    /// Validates document length, updates text, maps selections and marks,
    /// bumps version, and records history (unless in an active group).
    ///
    /// # Errors
    /// Returns `CoreError::ReadOnly` if read-only, or `CoreError::TransactionLengthMismatch`.
    pub fn apply(&mut self, mut tx: Transaction) -> Result<ApplyResult, CoreError> {
        if self.read_only {
            return Err(CoreError::ReadOnly);
        }

        let old_text = self.text.clone();
        let new_text = tx.changes.apply(&self.text)?;

        // Update selections before/after on transaction if absent
        if tx.selections_before.is_none() {
            tx.selections_before = Some(self.selections.clone());
        }

        // Apply text update
        self.text = new_text;

        // Update selections: use tx.selections_after if provided, else map existing
        let new_selections = match tx.selections_after.clone() {
            Some(sel) => sel,
            None => self.selections.map(&tx.changes),
        };
        self.selections = new_selections.clone();
        if tx.selections_after.is_none() {
            tx.selections_after = Some(new_selections);
        }

        // Map marks through the transaction
        self.marks.map(&tx);

        let old_version = self.version;
        self.version += 1;

        // Record history
        if self.group_depth > 0 {
            if let Some(ref mut group) = self.active_group {
                group.transactions.push(tx.clone());
            }
        } else {
            let inverse = tx.invert(&old_text)?;
            self.history.commit(tx.clone(), inverse);
        }

        Ok(ApplyResult {
            old_version,
            new_version: self.version,
            transaction: tx,
        })
    }

    /// Undoes the last committed edit in history.
    ///
    /// # Errors
    /// Returns `CoreError::GroupOpenConflict` if an undo group is currently open.
    pub fn undo(&mut self) -> Result<Option<ApplyResult>, CoreError> {
        if self.group_depth > 0 {
            return Err(CoreError::GroupOpenConflict);
        }
        if self.read_only {
            return Err(CoreError::ReadOnly);
        }

        let inverse_tx = match self.history.undo() {
            Some(tx) => tx.clone(),
            None => return Ok(None),
        };

        let result = self.apply_internal(inverse_tx, TransactionOrigin::Undo)?;
        Ok(Some(result))
    }

    /// Redoes the next edit along the preferred child branch in history.
    ///
    /// # Errors
    /// Returns `CoreError::GroupOpenConflict` if an undo group is currently open.
    pub fn redo(&mut self) -> Result<Option<ApplyResult>, CoreError> {
        if self.group_depth > 0 {
            return Err(CoreError::GroupOpenConflict);
        }
        if self.read_only {
            return Err(CoreError::ReadOnly);
        }

        let forward_tx = match self.history.redo() {
            Some(tx) => tx.clone(),
            None => return Ok(None),
        };

        let result = self.apply_internal(forward_tx, TransactionOrigin::Redo)?;
        Ok(Some(result))
    }

    /// Navigates the undo tree to the specified sequence number.
    ///
    /// # Errors
    /// Returns `CoreError::GroupOpenConflict` if an undo group is open, or
    /// `CoreError::HistorySeqNotFound` if sequence number is invalid.
    pub fn jump_to_seq(&mut self, seq: u64) -> Result<Vec<ApplyResult>, CoreError> {
        if self.group_depth > 0 {
            return Err(CoreError::GroupOpenConflict);
        }
        if self.read_only {
            return Err(CoreError::ReadOnly);
        }

        let txs = self.history.jump_to_seq(seq)?;
        let mut results = Vec::with_capacity(txs.len());

        for tx in txs {
            let origin = tx.origin;
            let res = self.apply_internal(tx, origin)?;
            results.push(res);
        }

        Ok(results)
    }

    /// Internal apply path used exclusively for undo/redo and jump navigation.
    ///
    /// Does NOT record new history nodes.
    fn apply_internal(
        &mut self,
        tx: Transaction,
        origin: TransactionOrigin,
    ) -> Result<ApplyResult, CoreError> {
        let new_text = tx.changes.apply(&self.text)?;
        self.text = new_text;

        // Restore selections if present, or map
        if let Some(ref sel) = tx.selections_after {
            self.selections = sel.clone();
        } else {
            self.selections = self.selections.map(&tx.changes);
        }

        self.marks.map(&tx);

        let old_version = self.version;
        self.version += 1;

        let mut applied_tx = tx;
        applied_tx.origin = origin;

        Ok(ApplyResult {
            old_version,
            new_version: self.version,
            transaction: applied_tx,
        })
    }
}
