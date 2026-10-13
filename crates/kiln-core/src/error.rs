//! Errors for `kiln-core`.

use thiserror::Error;

/// Error type for editing model operations.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CoreError {
    /// The transaction's input length does not match the document length.
    #[error("Transaction input length {tx_len} does not match document length {doc_len}")]
    TransactionLengthMismatch {
        /// Input length declared by the transaction.
        tx_len: usize,
        /// Actual length of the target document in characters.
        doc_len: usize,
    },

    /// The buffer is marked read-only and cannot be modified.
    #[error("Buffer is read-only")]
    ReadOnly,

    /// A selections list cannot be empty.
    #[error("Selections list cannot be empty")]
    EmptySelections,

    /// The primary selection index is out of bounds.
    #[error("Primary selection index {primary} out of bounds (len {len})")]
    PrimarySelectionOutOfBounds {
        /// The given primary index.
        primary: usize,
        /// The number of ranges.
        len: usize,
    },

    /// A position was out of bounds for the document.
    #[error("Position {pos} out of bounds (document length {max})")]
    PositionOutOfBounds {
        /// The invalid character index.
        pos: usize,
        /// Maximum allowed character index.
        max: usize,
    },

    /// A character range was invalid (e.g. start > end).
    #[error("Invalid character range: start {start} > end {end}")]
    InvalidRange {
        /// Start index.
        start: usize,
        /// End index.
        end: usize,
    },

    /// Edits provided to builder are overlapping or not sorted in ascending order.
    #[error("Edits are unsorted or overlapping")]
    UnsortedOrOverlappingEdits,

    /// An invalid ChangeSet was provided (e.g. malformed or invalid lengths).
    #[error("Invalid ChangeSet: {message}")]
    InvalidChangeSet {
        /// Description of the error.
        message: &'static str,
    },

    /// An undo group was ended without a corresponding `begin_group`.
    #[error("Attempted to end undo group without matching begin_group")]
    UnbalancedGroupEnd,

    /// An operation (undo, redo, jump, mark_saved) was attempted while an undo group is open.
    #[error("Cannot perform operation while an undo group is open")]
    GroupOpenConflict,

    /// The requested history sequence number was not found.
    #[error("History sequence number {seq} not found")]
    HistorySeqNotFound {
        /// Requested sequence number.
        seq: u64,
    },

    /// The requested command was not found in the registry.
    #[error("Command not found: {name}")]
    CommandNotFound {
        /// Name of the missing command.
        name: String,
    },

    /// An invalid argument was passed to a command.
    #[error("Invalid command argument: {message}")]
    InvalidArgument {
        /// Description of the argument issue.
        message: &'static str,
    },

    /// An underlying text manipulation error occurred.
    #[error("Text error: {0}")]
    Text(#[from] kiln_text::TextError),
}
