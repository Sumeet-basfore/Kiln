//! Keymap trie and mode-based key mapping tables.

use std::collections::BTreeMap;

use kiln_core::CommandArg;

use crate::key::KeyEvent;
use crate::mode::Mode;

/// Action triggered when a key sequence is matched in a keymap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapAction {
    /// Execute a named command with an optional argument.
    Command {
        /// Name of the command registered in `CommandRegistry`.
        name: &'static str,
        /// Argument to pass to the command.
        arg: CommandArg,
    },
    /// Switch editor mode.
    ChangeMode(Mode),
}

/// Result of looking up a sequence in a `KeymapTrie`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapLookup<'a> {
    /// Exact match found for the sequence.
    Exact(&'a KeymapAction),
    /// The sequence is a prefix of one or more longer bindings.
    Partial,
    /// No binding starts with this sequence.
    None,
}

/// Prefix tree for matching multi-key sequences with partial match support.
#[derive(Default)]
pub struct KeymapTrie {
    action: Option<KeymapAction>,
    children: BTreeMap<KeyEvent, KeymapTrie>,
}

impl KeymapTrie {
    /// Creates a new, empty keymap trie.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a keybinding sequence associated with an action.
    pub fn insert(&mut self, keys: &[KeyEvent], action: KeymapAction) {
        if keys.is_empty() {
            self.action = Some(action);
            return;
        }

        let mut node = self;
        for key in keys {
            node = node.children.entry(*key).or_default();
        }
        node.action = Some(action);
    }

    /// Looks up a key sequence in the trie.
    #[must_use]
    pub fn lookup(&self, keys: &[KeyEvent]) -> KeymapLookup<'_> {
        let mut node = self;
        for key in keys {
            match node.children.get(key) {
                Some(next_node) => node = next_node,
                None => return KeymapLookup::None,
            }
        }

        if let Some(ref action) = node.action {
            KeymapLookup::Exact(action)
        } else if !node.children.is_empty() {
            KeymapLookup::Partial
        } else {
            KeymapLookup::None
        }
    }
}

/// Collection of keymap tries indexed by mode.
#[derive(Default)]
pub struct Keymaps {
    normal: KeymapTrie,
    insert: KeymapTrie,
    visual: KeymapTrie,
    command: KeymapTrie,
    search: KeymapTrie,
}

impl Keymaps {
    /// Creates an empty keymap set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the trie for the specified mode.
    #[must_use]
    pub fn get_trie(&self, mode: Mode) -> &KeymapTrie {
        match mode {
            Mode::Normal => &self.normal,
            Mode::Insert => &self.insert,
            Mode::Visual(_) => &self.visual,
            Mode::Command => &self.command,
            Mode::Search => &self.search,
        }
    }

    /// Returns a mutable reference to the trie for the specified mode.
    pub fn get_trie_mut(&mut self, mode: Mode) -> &mut KeymapTrie {
        match mode {
            Mode::Normal => &mut self.normal,
            Mode::Insert => &mut self.insert,
            Mode::Visual(_) => &mut self.visual,
            Mode::Command => &mut self.command,
            Mode::Search => &mut self.search,
        }
    }

    /// Creates and populates default keymaps loaded from static tables.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut keymaps = Self::new();
        load_default_bindings(&mut keymaps);
        keymaps
    }
}

fn load_default_bindings(keymaps: &mut Keymaps) {
    // Normal mode default transitions and basic commands
    let normal_bindings = [
        ("i", KeymapAction::ChangeMode(Mode::Insert)),
        (
            "v",
            KeymapAction::ChangeMode(Mode::Visual(crate::mode::VisualKind::Char)),
        ),
        (
            "V",
            KeymapAction::ChangeMode(Mode::Visual(crate::mode::VisualKind::Line)),
        ),
        (":", KeymapAction::ChangeMode(Mode::Command)),
        ("/", KeymapAction::ChangeMode(Mode::Search)),
        (
            "u",
            KeymapAction::Command {
                name: "undo",
                arg: CommandArg::None,
            },
        ),
        (
            "<C-r>",
            KeymapAction::Command {
                name: "redo",
                arg: CommandArg::None,
            },
        ),
        (
            "x",
            KeymapAction::Command {
                name: "vim_x",
                arg: CommandArg::None,
            },
        ),
        (
            "p",
            KeymapAction::Command {
                name: "vim_paste_after",
                arg: CommandArg::None,
            },
        ),
        (
            "P",
            KeymapAction::Command {
                name: "vim_paste_before",
                arg: CommandArg::None,
            },
        ),
        (
            "o",
            KeymapAction::Command {
                name: "vim_open_below",
                arg: CommandArg::None,
            },
        ),
        (
            "O",
            KeymapAction::Command {
                name: "vim_open_above",
                arg: CommandArg::None,
            },
        ),
        (
            "a",
            KeymapAction::Command {
                name: "vim_append",
                arg: CommandArg::None,
            },
        ),
        (
            "A",
            KeymapAction::Command {
                name: "vim_append_line_end",
                arg: CommandArg::None,
            },
        ),
        (
            "I",
            KeymapAction::Command {
                name: "vim_insert_line_start",
                arg: CommandArg::None,
            },
        ),
        (
            "J",
            KeymapAction::Command {
                name: "vim_join_lines",
                arg: CommandArg::None,
            },
        ),
    ];

    for (k_str, action) in normal_bindings {
        if let Some(key) = KeyEvent::parse(k_str) {
            keymaps.get_trie_mut(Mode::Normal).insert(&[key], action);
        }
    }

    // Insert mode default bindings
    let insert_bindings = [
        ("<Esc>", KeymapAction::ChangeMode(Mode::Normal)),
        (
            "<Enter>",
            KeymapAction::Command {
                name: "newline",
                arg: CommandArg::None,
            },
        ),
        (
            "<Backspace>",
            KeymapAction::Command {
                name: "backspace",
                arg: CommandArg::None,
            },
        ),
        (
            "<Left>",
            KeymapAction::Command {
                name: "move_cursor",
                arg: CommandArg::Direction(kiln_core::MoveDirection::Left),
            },
        ),
        (
            "<Right>",
            KeymapAction::Command {
                name: "move_cursor",
                arg: CommandArg::Direction(kiln_core::MoveDirection::Right),
            },
        ),
        (
            "<Up>",
            KeymapAction::Command {
                name: "move_cursor",
                arg: CommandArg::Direction(kiln_core::MoveDirection::Up),
            },
        ),
        (
            "<Down>",
            KeymapAction::Command {
                name: "move_cursor",
                arg: CommandArg::Direction(kiln_core::MoveDirection::Down),
            },
        ),
    ];

    for (k_str, action) in insert_bindings {
        if let Some(key) = KeyEvent::parse(k_str) {
            keymaps.get_trie_mut(Mode::Insert).insert(&[key], action);
        }
    }

    // Visual mode escape back to Normal
    if let Some(esc) = KeyEvent::parse("<Esc>") {
        keymaps
            .get_trie_mut(Mode::Visual(crate::mode::VisualKind::Char))
            .insert(&[esc], KeymapAction::ChangeMode(Mode::Normal));
    }

    // Command mode escape back to Normal
    if let Some(esc) = KeyEvent::parse("<Esc>") {
        keymaps
            .get_trie_mut(Mode::Command)
            .insert(&[esc], KeymapAction::ChangeMode(Mode::Normal));
    }
}
