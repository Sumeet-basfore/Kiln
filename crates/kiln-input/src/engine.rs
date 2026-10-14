//! Input engine coordinating mode, keymap trie, and Vim state machine.

use crate::action::VimAction;
use crate::key::{KeyCode, KeyEvent};
use crate::keymap::{KeymapAction, KeymapLookup, Keymaps};
use crate::mode::Mode;
use crate::vim::{StepResult, VimStateMachine};

/// High-level event produced by the input engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputResult {
    /// Execute a keymapped command.
    ExecuteCommand {
        /// Command name.
        name: &'static str,
        /// Command argument.
        arg: kiln_core::CommandArg,
    },
    /// Switch to a new editor mode.
    SwitchMode(Mode),
    /// A parsed Vim grammar action (for Normal/Visual execution).
    VimAction(VimAction),
    /// In Insert mode: a raw character to insert.
    InsertChar(char),
    /// The command line changed.
    CommandLineChanged(String),
    /// A command-line command was submitted.
    SubmitCommand(String),
    /// Key sequence is a partial match or waiting for more input.
    Pending,
    /// Unhandled or invalid key event (discarded cleanly).
    Ignored,
}

/// Central input engine managing key buffering, mode state, and parsing.
pub struct InputEngine {
    mode: Mode,
    keymaps: Keymaps,
    vim_machine: VimStateMachine,
    pending_keys: Vec<KeyEvent>,
    command_line: String,
}

impl Default for InputEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl InputEngine {
    /// Creates a new `InputEngine` with default keymaps and Normal mode.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            keymaps: Keymaps::with_defaults(),
            vim_machine: VimStateMachine::new(),
            pending_keys: Vec::new(),
            command_line: String::new(),
        }
    }

    /// Returns the current active mode.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Explicitly switches the current mode and resets any pending input buffers.
    pub fn set_mode(&mut self, mode: Mode) {
        let leaving_command = self.mode == Mode::Command && mode != Mode::Command;
        self.mode = mode;
        self.reset_pending();
        if mode == Mode::Command || leaving_command {
            self.command_line.clear();
        }
    }

    /// Returns the current command-line contents.
    #[must_use]
    pub fn command_line(&self) -> &str {
        &self.command_line
    }

    /// Returns a reference to the active keymaps.
    #[must_use]
    pub fn keymaps(&self) -> &Keymaps {
        &self.keymaps
    }

    /// Returns a mutable reference to the keymaps for customizing bindings.
    pub fn keymaps_mut(&mut self) -> &mut Keymaps {
        &mut self.keymaps
    }

    /// Resets pending key sequences and state machine.
    pub fn reset_pending(&mut self) {
        self.pending_keys.clear();
        self.vim_machine.reset();
    }

    /// Handles a partial match timeout by dropping pending key sequences.
    pub fn handle_timeout(&mut self) {
        self.reset_pending();
    }

    /// Processes a single `KeyEvent`, returning the resolved `InputResult`.
    pub fn handle_key(&mut self, event: KeyEvent) -> InputResult {
        // 1. Try keymap lookup in the active mode
        self.pending_keys.push(event);
        let trie = self.keymaps.get_trie(self.mode);

        match trie.lookup(&self.pending_keys) {
            KeymapLookup::Exact(action) => {
                let act = action.clone();
                self.pending_keys.clear();
                return self.apply_keymap_action(act);
            }
            KeymapLookup::Partial => {
                return InputResult::Pending;
            }
            KeymapLookup::None => {
                // Not in keymap trie; pop from pending_keys and fall through
                self.pending_keys.clear();
            }
        }

        // 2. Mode-specific dispatch
        match self.mode {
            Mode::Insert => {
                // Any unmodified character inserts directly; Shift is accepted
                // because the character value is already transformed (e.g., 'a' + Shift → 'A')
                if let KeyCode::Char(_) = event.code {
                    return InputResult::InsertChar(match event.code {
                        KeyCode::Char(c) => c,
                        _ => '\0',
                    });
                }
                InputResult::Ignored
            }
            Mode::Normal | Mode::Visual(_) => {
                // Route through Vim grammar state machine
                match self.vim_machine.step(event) {
                    StepResult::Complete(action) => InputResult::VimAction(action),
                    StepResult::Pending => InputResult::Pending,
                    StepResult::Invalid | StepResult::Cancelled => InputResult::Ignored,
                }
            }
            Mode::Command => match event.code {
                KeyCode::Enter => {
                    let command = std::mem::take(&mut self.command_line);
                    self.set_mode(Mode::Normal);
                    InputResult::SubmitCommand(command)
                }
                KeyCode::Backspace => {
                    self.command_line.pop();
                    InputResult::CommandLineChanged(self.command_line.clone())
                }
                KeyCode::Char(c) => {
                    self.command_line.push(c);
                    InputResult::CommandLineChanged(self.command_line.clone())
                }
                _ => InputResult::Ignored,
            },
            Mode::Search => {
                // Accept any character key including Shift-modified
                if let KeyCode::Char(c) = event.code {
                    return InputResult::InsertChar(c);
                }
                InputResult::Ignored
            }
        }
    }

    fn apply_keymap_action(&mut self, action: KeymapAction) -> InputResult {
        match action {
            KeymapAction::ChangeMode(new_mode) => {
                self.set_mode(new_mode);
                InputResult::SwitchMode(new_mode)
            }
            KeymapAction::Command { name, arg } => InputResult::ExecuteCommand { name, arg },
        }
    }
}
