//! `kiln-ui`: terminal rendering, layout, buffer views, status lines, and themes.

pub mod buffer_view;
pub mod error;
pub mod gutter;
pub mod renderer;
pub mod statusline;
pub mod terminal;
pub mod theme;

pub use buffer_view::BufferView;
pub use crossterm;
pub use error::UiError;
pub use gutter::{Gutter, LineNumberMode};
pub use ratatui;
pub use renderer::{Color, Modifier, RatatuiRenderer, Renderer, Style, TestRenderer};
pub use statusline::{StatusLine, StatusLineState};
pub use terminal::{restore_terminal, TerminalGuard};
pub use theme::Theme;

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-ui"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-ui");
    }
}
