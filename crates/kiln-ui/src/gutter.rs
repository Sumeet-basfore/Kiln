//! Gutter layout and line number formatting.
//!
//! Handles line number display (hybrid, absolute, relative) and reserved
//! columns for signs according to `UI-UX-SPEC.md` Section 3.

/// Mode for formatting line numbers in the gutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineNumberMode {
    /// Hybrid mode (default): current line displays absolute number,
    /// other lines display relative distance.
    #[default]
    Hybrid,
    /// Absolute 1-indexed line numbers.
    Absolute,
    /// Relative line numbers (distance from cursor).
    Relative,
    /// Line numbers disabled.
    None,
}

/// Gutter layout calculator and line formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gutter {
    /// The line numbering mode.
    pub mode: LineNumberMode,
    /// Whether git / diagnostic sign column is enabled.
    pub show_signs: bool,
}

impl Default for Gutter {
    fn default() -> Self {
        Self {
            mode: LineNumberMode::Hybrid,
            show_signs: true,
        }
    }
}

impl Gutter {
    /// Calculates the stable width (in columns) for the gutter given the total line count.
    #[must_use]
    pub fn width(&self, total_lines: usize) -> u16 {
        if self.mode == LineNumberMode::None {
            return if self.show_signs { 2 } else { 0 };
        }

        let digits = total_lines.max(1).to_string().len();
        // Number width + 1 trailing space + optional 2 sign columns
        let sign_width = if self.show_signs { 1 } else { 0 };
        (digits.max(2) + 1 + sign_width) as u16
    }

    /// Formats the gutter string for a specific line index (0-indexed).
    ///
    /// `line_idx`: 0-indexed line in the document.
    /// `cursor_line_idx`: 0-indexed line where the primary cursor currently sits.
    /// `total_lines`: total line count in the document.
    #[must_use]
    pub fn format_line(
        &self,
        line_idx: usize,
        cursor_line_idx: usize,
        total_lines: usize,
    ) -> String {
        let gutter_w = self.width(total_lines) as usize;
        let sign_col = if self.show_signs { " " } else { "" };

        if self.mode == LineNumberMode::None {
            return sign_col.to_string();
        }

        let num_str = match self.mode {
            LineNumberMode::Absolute => (line_idx + 1).to_string(),
            LineNumberMode::Relative => {
                let diff = (line_idx as isize - cursor_line_idx as isize).unsigned_abs();
                diff.to_string()
            }
            LineNumberMode::Hybrid => {
                if line_idx == cursor_line_idx {
                    (line_idx + 1).to_string()
                } else {
                    let diff = (line_idx as isize - cursor_line_idx as isize).unsigned_abs();
                    diff.to_string()
                }
            }
            LineNumberMode::None => String::new(),
        };

        // Right-align the line number inside the allocated space
        let num_width = gutter_w - if self.show_signs { 1 } else { 0 } - 1;
        format!("{}{:>width$} ", sign_col, num_str, width = num_width)
    }
}
