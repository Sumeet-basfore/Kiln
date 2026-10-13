//! Buffer view widget and text rendering.
//!
//! Handles viewport scrolling, scrolloff margins, selections, cursor positioning,
//! and line formatting with wide character support.

use crate::gutter::Gutter;
use crate::renderer::Renderer;
use crate::theme::Theme;
use kiln_core::{Buffer, LineCol, Range};
use unicode_width::UnicodeWidthChar;

/// Manages viewport positioning and rendering of a text buffer.
#[derive(Debug, Clone)]
pub struct BufferView {
    /// 0-indexed line at the top of the viewport.
    pub scroll_row: usize,
    /// 0-indexed column at the left of the viewport.
    pub scroll_col: usize,
    /// Vertical scrolloff margin (lines kept above/below cursor).
    pub scrolloff_rows: usize,
    /// Horizontal scrolloff margin.
    pub scrolloff_cols: usize,
    /// Gutter layout options.
    pub gutter: Gutter,
}

impl Default for BufferView {
    fn default() -> Self {
        Self {
            scroll_row: 0,
            scroll_col: 0,
            scrolloff_rows: 3,
            scrolloff_cols: 5,
            gutter: Gutter::default(),
        }
    }
}

impl BufferView {
    /// Adjusts scrolling so that the cursor stays within the viewport and scrolloff margins.
    pub fn ensure_cursor_visible(
        &mut self,
        cursor_line: usize,
        cursor_col: usize,
        view_width: u16,
        view_height: u16,
        total_lines: usize,
    ) {
        if view_height == 0 || view_width == 0 {
            return;
        }

        let height = view_height as usize;
        let gutter_w = self.gutter.width(total_lines) as usize;
        let text_width = (view_width as usize).saturating_sub(gutter_w).max(1);

        // Vertical scrolling with scrolloff margin
        let margin_y = self.scrolloff_rows.min(height.saturating_sub(1) / 2);

        if cursor_line < self.scroll_row + margin_y {
            self.scroll_row = cursor_line.saturating_sub(margin_y);
        } else if cursor_line >= self.scroll_row + height.saturating_sub(margin_y) {
            self.scroll_row = (cursor_line + margin_y + 1).saturating_sub(height);
        }

        if total_lines > 0 && self.scroll_row >= total_lines {
            self.scroll_row = total_lines.saturating_sub(1);
        }

        // Horizontal scrolling with scrolloff margin
        let margin_x = self.scrolloff_cols.min(text_width.saturating_sub(1) / 2);

        if cursor_col < self.scroll_col + margin_x {
            self.scroll_col = cursor_col.saturating_sub(margin_x);
        } else if cursor_col >= self.scroll_col + text_width.saturating_sub(margin_x) {
            self.scroll_col = (cursor_col + margin_x + 1).saturating_sub(text_width);
        }
    }

    /// Renders the buffer onto the given `Renderer` in the area `0..width` and `0..height`.
    pub fn render<R: Renderer + ?Sized>(
        &mut self,
        renderer: &mut R,
        buffer: &Buffer,
        height: u16,
        theme: &Theme,
    ) {
        let (view_width, _) = renderer.size();
        if view_width == 0 || height == 0 {
            return;
        }

        let text = buffer.text();
        let total_lines = text.len_lines();
        let primary_sel = buffer.selections().primary();
        let cursor_char_idx = primary_sel.head;
        let cursor_line_col = text
            .char_to_line_col(cursor_char_idx)
            .unwrap_or(LineCol::new(0, 0));

        let cursor_line = cursor_line_col.line;
        let cursor_col = cursor_line_col.col;

        // Ensure scrolling keeps the cursor in view
        self.ensure_cursor_visible(cursor_line, cursor_col, view_width, height, total_lines);

        let gutter_w = self.gutter.width(total_lines);
        let text_width = view_width.saturating_sub(gutter_w);

        // Collect all active selection intervals for highlighting
        let selection_ranges: Vec<Range> = buffer
            .selections()
            .ranges()
            .iter()
            .filter(|r| !r.is_empty())
            .copied()
            .collect();

        let mut screen_cursor: Option<(u16, u16)> = None;

        for screen_row in 0..height {
            let doc_line_idx = self.scroll_row + screen_row as usize;

            if doc_line_idx < total_lines {
                // 1. Draw Gutter
                let gutter_str = self
                    .gutter
                    .format_line(doc_line_idx, cursor_line, total_lines);
                let gutter_style = if doc_line_idx == cursor_line {
                    theme.ui_gutter_current
                } else {
                    theme.ui_gutter
                };
                renderer.draw_str(0, screen_row, &gutter_str, gutter_style);

                // 2. Draw Text Characters
                let line_content = text.line_without_newline(doc_line_idx).unwrap_or_default();
                let line_start_char = text
                    .line_col_to_char(LineCol::new(doc_line_idx, 0))
                    .unwrap_or(0);

                let mut current_col: u16 = gutter_w;
                let line_chars: Vec<char> = line_content.chars().collect();

                let mut char_idx_on_line = self.scroll_col;
                while char_idx_on_line < line_chars.len() {
                    let ch = line_chars[char_idx_on_line];
                    let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;

                    if current_col + ch_width > view_width {
                        break;
                    }

                    let global_char_idx = line_start_char + char_idx_on_line;

                    // Check if this character is selected
                    let is_selected = selection_ranges
                        .iter()
                        .any(|r| r.from() <= global_char_idx && global_char_idx < r.to());

                    let char_style = if is_selected {
                        theme.ui_selection
                    } else {
                        theme.ui_text
                    };

                    renderer.draw_char(current_col, screen_row, ch, char_style);

                    // Track screen cursor if this character is the primary cursor
                    if doc_line_idx == cursor_line && char_idx_on_line == cursor_col {
                        screen_cursor = Some((current_col, screen_row));
                    }

                    current_col += ch_width;
                    char_idx_on_line += 1;
                }

                // If cursor is at the end of the line (or on empty line)
                if doc_line_idx == cursor_line && cursor_col >= line_chars.len() {
                    let offset_from_end = cursor_col.saturating_sub(self.scroll_col);
                    let target_col = gutter_w + offset_from_end as u16;
                    if target_col < view_width {
                        screen_cursor = Some((target_col, screen_row));
                    }
                }

                // Fill rest of the line with background
                if current_col < view_width {
                    let blank_len = (view_width - current_col) as usize;
                    let blank_str = " ".repeat(blank_len);
                    renderer.draw_str(current_col, screen_row, &blank_str, theme.ui_background);
                }
            } else {
                // Beyond EOF: draw '~' in gutter, blank for remainder
                let tilde_str = if gutter_w > 0 {
                    format!("~{:>width$}", "", width = gutter_w as usize - 1)
                } else {
                    String::new()
                };
                renderer.draw_str(0, screen_row, &tilde_str, theme.ui_gutter);

                if text_width > 0 {
                    let blank_str = " ".repeat(text_width as usize);
                    renderer.draw_str(gutter_w, screen_row, &blank_str, theme.ui_background);
                }
            }
        }

        renderer.set_cursor(screen_cursor);
    }
}
