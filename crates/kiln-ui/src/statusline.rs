//! Status line widget and rendering.
//!
//! Formats the one-line status bar at the bottom of the screen following
//! `UI-UX-SPEC.md` Section 4.

use crate::renderer::{Renderer, Style};
use crate::theme::Theme;

/// Information required to render the status line.
#[derive(Debug, Clone)]
pub struct StatusLineState<'a> {
    /// Mode label (e.g. "NORMAL", "INSERT", "VISUAL", "COMMAND").
    pub mode_name: &'a str,
    /// Style for the mode badge.
    pub mode_style: Style,
    /// File / buffer name (e.g. "src/main.rs" or "[No Name]").
    pub buffer_name: &'a str,
    /// Whether the buffer has unsaved modifications.
    pub is_modified: bool,
    /// Whether the buffer is read-only.
    pub is_readonly: bool,
    /// Cursor line number (1-indexed).
    pub line: usize,
    /// Cursor column number (1-indexed).
    pub col: usize,
    /// Document position percentage (e.g. "Top", "Bot", "50%", "All").
    pub percent: &'a str,
    /// Encoding label (e.g. "UTF-8").
    pub encoding: &'a str,
    /// Line ending label (e.g. "LF" or "CRLF").
    pub line_ending: &'a str,
}

/// Status line renderer.
pub struct StatusLine;

impl StatusLine {
    /// Renders the status line onto the given renderer at row `row`.
    pub fn render<R: Renderer + ?Sized>(
        renderer: &mut R,
        row: u16,
        state: &StatusLineState<'_>,
        theme: &Theme,
    ) {
        let (width, _) = renderer.size();
        if width == 0 {
            return;
        }

        // Fill status line row with statusline background
        let blank_str = " ".repeat(width as usize);
        renderer.draw_str(0, row, &blank_str, theme.ui_statusline);

        // 1. Mode badge (e.g. " NORMAL ")
        let mode_text = format!(" {} ", state.mode_name);
        renderer.draw_str(0, row, &mode_text, state.mode_style);
        let left_pos = mode_text.chars().count() as u16;

        // 2. Right segments, dropping gracefully from the right on narrow screens
        let pos_str = format!("{}:{}", state.line, state.col);
        let right_text = if width >= 80 {
            format!(
                "{} · {}   {}   {} ",
                state.encoding, state.line_ending, pos_str, state.percent
            )
        } else if width >= 50 {
            format!("{}   {} ", pos_str, state.percent)
        } else if width >= 30 {
            format!("{} ", pos_str)
        } else {
            String::new()
        };
        let right_len = right_text.chars().count() as u16;

        // 3. Buffer name and flags
        let mut file_info = format!(" {}", state.buffer_name);
        if state.is_modified {
            file_info.push_str(" [+]");
        }
        if state.is_readonly {
            file_info.push_str(" [RO]");
        }
        file_info.push(' ');

        // Truncate file info if width is very narrow
        let max_file_len = (width.saturating_sub(left_pos + right_len + 1)) as usize;
        let display_file_info = if file_info.len() > max_file_len && max_file_len > 0 {
            format!("{}… ", &file_info[..max_file_len.saturating_sub(2)])
        } else {
            file_info
        };

        renderer.draw_str(left_pos, row, &display_file_info, theme.ui_statusline);

        if !right_text.is_empty() && width > right_len {
            let start_col = width - right_len;
            if start_col >= left_pos + display_file_info.chars().count() as u16 {
                renderer.draw_str(start_col, row, &right_text, theme.ui_statusline);
            }
        }
    }
}
