//! `kiln-core`: editing model, buffers, selections, transactions, and undo tree.
//!
//! Features:
//! - Character-offset based range and selection lists with automatic normalization.
//! - Invertible and composable change sets and transactions (`ChangeSet`, `Transaction`).
//! - Branching undo tree with preferred redo navigation, common-ancestor jumps, and sequence numbers.
//! - Undo grouping support (`begin_group` / `end_group`).
//! - Buffer model with versioning, dirty flag tracking derived from undo tree, and named marks.

pub mod buffer;
pub mod command;
pub mod commands;
pub mod error;
pub mod event;
pub mod history;
pub mod marks;
pub mod selection;
pub mod transaction;

pub use buffer::{ApplyResult, Buffer, BufferId, BufferIdAllocator};
pub use command::{
    CommandArg, CommandContext, CommandFn, CommandMeta, CommandRegistry, CommandResult,
    MoveDirection,
};
pub use error::CoreError;
pub use event::{EditorEvent, EventBus, EventHandler, SubscriptionId};
pub use history::{HistoryNode, NodeId, UndoTree};
pub use marks::Marks;
pub use selection::{Range, Selections};
pub use transaction::{Assoc, ChangeSet, Operation, Transaction, TransactionOrigin};

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-core"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-core");
        assert_eq!(kiln_text::name(), "kiln-text");
    }
}
