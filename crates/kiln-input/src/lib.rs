//! `kiln-input` crate: key parsing, mode state machine, keymaps -> commands.

pub mod action;
pub mod engine;
pub mod key;
pub mod keymap;
pub mod mode;
pub mod vim;

pub use action::{Motion, Operator, Target, TextObject, VimAction};
pub use engine::{InputEngine, InputResult};
pub use key::{KeyCode, KeyEvent, KeyModifiers};
pub use keymap::{KeymapAction, KeymapLookup, KeymapTrie, Keymaps};
pub use mode::{Mode, VisualKind};
pub use vim::{StepResult, VimStateMachine};

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-input"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-input");
        assert_eq!(kiln_core::name(), "kiln-core");
        assert_eq!(kiln_config::name(), "kiln-config");
    }
}
