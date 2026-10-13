//! Renderer abstraction and implementations.
//!
//! Provides the `Renderer` trait to decouple view logic from the terminal backend,
//! enabling both production crossterm/ratatui rendering and headless `TestRenderer`
//! snapshot tests.

use crate::error::UiError;
pub use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthChar;

/// A trait for drawing editor views into an abstract terminal or grid.
pub trait Renderer {
    /// Returns the viewport dimensions as `(columns, rows)`.
    fn size(&self) -> (u16, u16);

    /// Clears the viewport or resets cells to default style and empty content.
    fn clear(&mut self);

    /// Draws a single character at `(col, row)` with the given style.
    fn draw_char(&mut self, col: u16, row: u16, ch: char, style: Style);

    /// Draws a text string starting at `(col, row)` with the given style.
    /// Advances columns according to character/grapheme display width.
    fn draw_str(&mut self, col: u16, row: u16, text: &str, style: Style);

    /// Sets the cursor position on the screen, or hides it if `None`.
    fn set_cursor(&mut self, cursor: Option<(u16, u16)>);

    /// Flushes or presents the rendered frame to the terminal or back-buffer.
    ///
    /// # Errors
    /// Returns `UiError` if presenting to the terminal backend fails.
    fn flush(&mut self) -> Result<(), UiError>;
}

/// An in-memory headless renderer for deterministic snapshot and layout testing.
#[derive(Debug, Clone)]
pub struct TestRenderer {
    cols: u16,
    rows: u16,
    buffer: ratatui::buffer::Buffer,
    cursor: Option<(u16, u16)>,
}

impl TestRenderer {
    /// Creates a new `TestRenderer` with the given dimensions.
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        let area = ratatui::layout::Rect::new(0, 0, cols, rows);
        Self {
            cols,
            rows,
            buffer: ratatui::buffer::Buffer::empty(area),
            cursor: None,
        }
    }

    /// Resizes the test renderer grid.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        let area = ratatui::layout::Rect::new(0, 0, cols, rows);
        self.buffer = ratatui::buffer::Buffer::empty(area);
        if let Some((c, r)) = self.cursor {
            if c >= cols || r >= rows {
                self.cursor = None;
            }
        }
    }

    /// Returns the current cursor position, if set.
    #[must_use]
    pub fn cursor(&self) -> Option<(u16, u16)> {
        self.cursor
    }

    /// Returns the cell symbol at `(col, row)` if within bounds.
    #[must_use]
    pub fn symbol_at(&self, col: u16, row: u16) -> Option<&str> {
        if col < self.cols && row < self.rows {
            Some(self.buffer.cell((col, row)).map_or(" ", |c| c.symbol()))
        } else {
            None
        }
    }

    /// Returns the cell style at `(col, row)` if within bounds.
    #[must_use]
    pub fn style_at(&self, col: u16, row: u16) -> Option<Style> {
        if col < self.cols && row < self.rows {
            self.buffer.cell((col, row)).map(|c| c.style())
        } else {
            None
        }
    }

    /// Returns all rendered lines as strings, with trailing whitespace preserved or trimmed.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut result = Vec::with_capacity(self.rows as usize);
        for r in 0..self.rows {
            let mut line = String::with_capacity(self.cols as usize);
            for c in 0..self.cols {
                if let Some(cell) = self.buffer.cell((c, r)) {
                    line.push_str(cell.symbol());
                } else {
                    line.push(' ');
                }
            }
            result.push(line);
        }
        result
    }

    /// Returns a multi-line snapshot of the rendered screen with trailing spaces trimmed on each line.
    #[must_use]
    pub fn snapshot(&self) -> String {
        self.lines()
            .into_iter()
            .map(|l| l.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Renderer for TestRenderer {
    fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    fn clear(&mut self) {
        self.buffer.reset();
    }

    fn draw_char(&mut self, col: u16, row: u16, ch: char, style: Style) {
        if col < self.cols && row < self.rows {
            let width = UnicodeWidthChar::width(ch).unwrap_or(1);
            if let Some(cell) = self.buffer.cell_mut((col, row)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
            for w in 1..width {
                if col + (w as u16) < self.cols {
                    if let Some(cell) = self.buffer.cell_mut((col + w as u16, row)) {
                        cell.set_symbol("");
                        cell.set_style(style);
                    }
                }
            }
        }
    }

    fn draw_str(&mut self, col: u16, row: u16, text: &str, style: Style) {
        if row >= self.rows {
            return;
        }
        let mut current_col = col;
        for ch in text.chars() {
            if current_col >= self.cols {
                break;
            }
            let width = UnicodeWidthChar::width(ch).unwrap_or(1);
            if current_col + (width as u16) > self.cols {
                break;
            }
            if let Some(cell) = self.buffer.cell_mut((current_col, row)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
            for w in 1..width {
                if current_col + (w as u16) < self.cols {
                    if let Some(cell) = self.buffer.cell_mut((current_col + w as u16, row)) {
                        cell.set_symbol("");
                        cell.set_style(style);
                    }
                }
            }
            current_col += width as u16;
        }
    }

    fn set_cursor(&mut self, cursor: Option<(u16, u16)>) {
        self.cursor = cursor;
    }

    fn flush(&mut self) -> Result<(), UiError> {
        Ok(())
    }
}

/// A `Renderer` implementation backed by a `ratatui::Terminal`.
pub struct RatatuiRenderer<B: ratatui::backend::Backend> {
    terminal: ratatui::Terminal<B>,
    buffer: ratatui::buffer::Buffer,
    cursor: Option<(u16, u16)>,
}

impl<B: ratatui::backend::Backend> RatatuiRenderer<B> {
    /// Creates a new `RatatuiRenderer` around the given terminal backend.
    ///
    /// # Errors
    /// Returns `UiError` if querying initial terminal size fails.
    pub fn new(terminal: ratatui::Terminal<B>) -> Result<Self, UiError> {
        let size = terminal.size().map_err(|e| UiError::TerminalError {
            message: e.to_string(),
        })?;
        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
        Ok(Self {
            terminal,
            buffer: ratatui::buffer::Buffer::empty(area),
            cursor: None,
        })
    }

    /// Access the underlying `ratatui::Terminal`.
    pub fn terminal(&self) -> &ratatui::Terminal<B> {
        &self.terminal
    }

    /// Access the underlying `ratatui::Terminal` mutably.
    pub fn terminal_mut(&mut self) -> &mut ratatui::Terminal<B> {
        &mut self.terminal
    }
}

impl<B: ratatui::backend::Backend> Renderer for RatatuiRenderer<B> {
    fn size(&self) -> (u16, u16) {
        let size = self.terminal.size().unwrap_or(ratatui::layout::Size {
            width: 80,
            height: 24,
        });
        (size.width, size.height)
    }

    fn clear(&mut self) {
        let (cols, rows) = self.size();
        let area = ratatui::layout::Rect::new(0, 0, cols, rows);
        if self.buffer.area != area {
            self.buffer = ratatui::buffer::Buffer::empty(area);
        } else {
            self.buffer.reset();
        }
    }

    fn draw_char(&mut self, col: u16, row: u16, ch: char, style: Style) {
        let (cols, rows) = self.size();
        if col < cols && row < rows {
            let width = UnicodeWidthChar::width(ch).unwrap_or(1);
            if let Some(cell) = self.buffer.cell_mut((col, row)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
            for w in 1..width {
                if col + (w as u16) < cols {
                    if let Some(cell) = self.buffer.cell_mut((col + w as u16, row)) {
                        cell.set_symbol("");
                        cell.set_style(style);
                    }
                }
            }
        }
    }

    fn draw_str(&mut self, col: u16, row: u16, text: &str, style: Style) {
        let (cols, rows) = self.size();
        if row >= rows {
            return;
        }
        let mut current_col = col;
        for ch in text.chars() {
            if current_col >= cols {
                break;
            }
            let width = UnicodeWidthChar::width(ch).unwrap_or(1);
            if current_col + (width as u16) > cols {
                break;
            }
            if let Some(cell) = self.buffer.cell_mut((current_col, row)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
            for w in 1..width {
                if current_col + (w as u16) < cols {
                    if let Some(cell) = self.buffer.cell_mut((current_col + w as u16, row)) {
                        cell.set_symbol("");
                        cell.set_style(style);
                    }
                }
            }
            current_col += width as u16;
        }
    }

    fn set_cursor(&mut self, cursor: Option<(u16, u16)>) {
        self.cursor = cursor;
    }

    fn flush(&mut self) -> Result<(), UiError> {
        let buffer_to_draw = self.buffer.clone();
        let cursor_pos = self.cursor;

        self.terminal
            .draw(|frame| {
                let frame_buffer = frame.buffer_mut();
                let area = frame_buffer.area;
                for y in 0..area.height {
                    for x in 0..area.width {
                        if let Some(src_cell) = buffer_to_draw.cell((x, y)) {
                            if let Some(dst_cell) = frame_buffer.cell_mut((x, y)) {
                                *dst_cell = src_cell.clone();
                            }
                        }
                    }
                }
                if let Some((col, row)) = cursor_pos {
                    frame.set_cursor_position((col, row));
                }
            })
            .map_err(|e| UiError::RenderError {
                message: e.to_string(),
            })?;

        Ok(())
    }
}
