//! Theme specification and default dark theme.
//!
//! Maps semantic UI scopes to styles and colors in accordance with
//! `UI-UX-SPEC.md` Section 10.

use crate::renderer::{Color, Modifier, Style};

/// A color theme defining styles for UI components and editor text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Name of the theme.
    pub name: String,
    /// Default background and text style for the editor view.
    pub ui_text: Style,
    /// Style for unselected/inactive background.
    pub ui_background: Style,
    /// Cursor style.
    pub ui_cursor: Style,
    /// Text selection style.
    pub ui_selection: Style,
    /// Inactive line number / gutter style.
    pub ui_gutter: Style,
    /// Current line number gutter style.
    pub ui_gutter_current: Style,
    /// Status line base style.
    pub ui_statusline: Style,
    /// Inactive status line base style.
    pub ui_statusline_nc: Style,
    /// Status line badge for Normal mode.
    pub mode_normal: Style,
    /// Status line badge for Insert mode.
    pub mode_insert: Style,
    /// Status line badge for Visual mode.
    pub mode_visual: Style,
    /// Status line badge for Command mode.
    pub mode_command: Style,
    /// Modified file marker style (`[+]`).
    pub status_modified: Style,
    /// Read-only marker style (`[RO]`).
    pub status_readonly: Style,
}

impl Theme {
    /// Creates the default Kiln dark theme.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            name: "Kiln Dark".to_string(),
            ui_text: Style::default()
                .fg(Color::Rgb(205, 214, 244))
                .bg(Color::Rgb(30, 30, 46)),
            ui_background: Style::default().bg(Color::Rgb(30, 30, 46)),
            ui_cursor: Style::default()
                .fg(Color::Rgb(17, 17, 27))
                .bg(Color::Rgb(245, 224, 220)),
            ui_selection: Style::default().bg(Color::Rgb(69, 71, 90)),
            ui_gutter: Style::default()
                .fg(Color::Rgb(108, 112, 134))
                .bg(Color::Rgb(24, 24, 37)),
            ui_gutter_current: Style::default()
                .fg(Color::Rgb(249, 226, 175))
                .bg(Color::Rgb(24, 24, 37))
                .add_modifier(Modifier::BOLD),
            ui_statusline: Style::default()
                .fg(Color::Rgb(205, 214, 244))
                .bg(Color::Rgb(49, 50, 68)),
            ui_statusline_nc: Style::default()
                .fg(Color::Rgb(147, 153, 178))
                .bg(Color::Rgb(30, 30, 46)),
            mode_normal: Style::default()
                .fg(Color::Rgb(17, 17, 27))
                .bg(Color::Rgb(137, 180, 250))
                .add_modifier(Modifier::BOLD),
            mode_insert: Style::default()
                .fg(Color::Rgb(17, 17, 27))
                .bg(Color::Rgb(166, 227, 161))
                .add_modifier(Modifier::BOLD),
            mode_visual: Style::default()
                .fg(Color::Rgb(17, 17, 27))
                .bg(Color::Rgb(203, 166, 247))
                .add_modifier(Modifier::BOLD),
            mode_command: Style::default()
                .fg(Color::Rgb(17, 17, 27))
                .bg(Color::Rgb(249, 226, 175))
                .add_modifier(Modifier::BOLD),
            status_modified: Style::default()
                .fg(Color::Rgb(250, 179, 135))
                .add_modifier(Modifier::BOLD),
            status_readonly: Style::default()
                .fg(Color::Rgb(243, 139, 168))
                .add_modifier(Modifier::BOLD),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
