//! Editor mode definitions.

/// Sub-variant for Visual mode selections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualKind {
    /// Character-wise selection.
    Char,
    /// Line-wise selection.
    Line,
    /// Block-wise / rectangular selection.
    Block,
}

/// Editor modal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// Normal mode: navigation, operators, and commands.
    #[default]
    Normal,
    /// Insert mode: direct text input.
    Insert,
    /// Visual mode: range selection.
    Visual(VisualKind),
    /// Command-line mode: `:` ex-commands.
    Command,
    /// Search mode: `/` or `?` incremental search.
    Search,
}

impl Mode {
    /// Returns the uppercase display name of the mode.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Visual(VisualKind::Char) => "VISUAL",
            Self::Visual(VisualKind::Line) => "V-LINE",
            Self::Visual(VisualKind::Block) => "V-BLOCK",
            Self::Command => "COMMAND",
            Self::Search => "SEARCH",
        }
    }
}
