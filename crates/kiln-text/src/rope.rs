//! Rope wrapper providing a character-offset API over `ropey::Rope`.
//!
//! Note on line breaks:
//! In `kiln-text`, text is normalized to `\n` (LF) in memory.
//! Lone CR (`\r`), vertical tab (`\u{0B}`), form feed (`\u{0C}`), next line (`\u{85}`),
//! line separator (`\u{2028}`), and paragraph separator (`\u{2029}`) do NOT count as line
//! breaks and do not affect `len_lines` or line splitting.

use std::fmt;
use std::ops::Range;
use std::str::FromStr;

use ropey::{Rope, RopeSlice};

use crate::convert::LineCol;
use crate::error::TextError;

/// An immutable or mutable text buffer wrapping a rope data structure.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Text {
    rope: Rope,
}

impl Text {
    /// Creates a new empty `Text` buffer.
    #[must_use]
    pub fn new() -> Self {
        Self { rope: Rope::new() }
    }

    /// Returns the length of the text in Unicode scalar values (characters).
    #[must_use]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Returns the length of the text in UTF-8 bytes.
    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Returns the number of lines in the text.
    ///
    /// An empty text contains 1 line. Every newline `\n` introduces a new line.
    #[must_use]
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Returns `true` if the text contains no characters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Inserts a string slice at the given character offset.
    ///
    /// # Errors
    /// Returns `TextError::CharIndexOutOfBounds` if `char_idx > self.len_chars()`.
    pub fn insert(&mut self, char_idx: usize, text: &str) -> Result<(), TextError> {
        let len = self.len_chars();
        if char_idx > len {
            return Err(TextError::CharIndexOutOfBounds { char_idx, len });
        }
        self.rope.insert(char_idx, text);
        Ok(())
    }

    /// Removes characters in the specified range `[start..end)`.
    ///
    /// # Errors
    /// Returns `TextError::InvalidCharRange` if `start > end`, or
    /// `TextError::CharIndexOutOfBounds` if `end > self.len_chars()`.
    pub fn remove(&mut self, range: Range<usize>) -> Result<(), TextError> {
        let len = self.len_chars();
        if range.start > range.end {
            return Err(TextError::InvalidCharRange {
                start: range.start,
                end: range.end,
            });
        }
        if range.end > len {
            return Err(TextError::CharIndexOutOfBounds {
                char_idx: range.end,
                len,
            });
        }
        self.rope.remove(range);
        Ok(())
    }

    /// Returns a string slice of the text for the given character range.
    ///
    /// # Errors
    /// Returns `TextError::InvalidCharRange` if `start > end`, or
    /// `TextError::CharIndexOutOfBounds` if `end > self.len_chars()`.
    pub fn slice(&self, range: Range<usize>) -> Result<String, TextError> {
        let len = self.len_chars();
        if range.start > range.end {
            return Err(TextError::InvalidCharRange {
                start: range.start,
                end: range.end,
            });
        }
        if range.end > len {
            return Err(TextError::CharIndexOutOfBounds {
                char_idx: range.end,
                len,
            });
        }
        Ok(self.rope.slice(range).to_string())
    }

    /// Returns the contents of a line including its trailing newline (if any).
    ///
    /// # Errors
    /// Returns `TextError::LineIndexOutOfBounds` if `line_idx >= self.len_lines()`.
    pub fn line(&self, line_idx: usize) -> Result<String, TextError> {
        let len = self.len_lines();
        if line_idx >= len {
            return Err(TextError::LineIndexOutOfBounds { line_idx, len });
        }
        Ok(self.rope.line(line_idx).to_string())
    }

    /// Returns the contents of a line without its trailing newline character.
    ///
    /// # Errors
    /// Returns `TextError::LineIndexOutOfBounds` if `line_idx >= self.len_lines()`.
    pub fn line_without_newline(&self, line_idx: usize) -> Result<String, TextError> {
        let slice = self.line_slice(line_idx)?;
        let mut s = slice.to_string();
        if s.ends_with('\n') {
            s.pop();
            if s.ends_with('\r') {
                s.pop();
            }
        }
        Ok(s)
    }

    /// Returns a `RopeSlice` for the specified line index.
    ///
    /// # Errors
    /// Returns `TextError::LineIndexOutOfBounds` if `line_idx >= self.len_lines()`.
    pub fn line_slice(&self, line_idx: usize) -> Result<RopeSlice<'_>, TextError> {
        let len = self.len_lines();
        if line_idx >= len {
            return Err(TextError::LineIndexOutOfBounds { line_idx, len });
        }
        Ok(self.rope.line(line_idx))
    }

    /// Returns a non-allocating iterator over the characters of a single line.
    ///
    /// # Errors
    /// Returns `TextError::LineIndexOutOfBounds` if `line_idx >= self.len_lines()`.
    pub fn line_chars(
        &self,
        line_idx: usize,
    ) -> Result<impl Iterator<Item = char> + '_, TextError> {
        let slice = self.line_slice(line_idx)?;
        Ok(slice.chars())
    }

    /// Returns an iterator over all characters in the text.
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.rope.chars()
    }

    /// Returns an iterator over all lines in the text (each line as a `String`).
    pub fn lines(&self) -> impl Iterator<Item = String> + '_ {
        self.rope.lines().map(|l| l.to_string())
    }

    /// Provides access to the underlying `RopeSlice` of the entire text.
    #[must_use]
    pub fn as_slice(&self) -> RopeSlice<'_> {
        self.rope.slice(..)
    }

    // -------------------------------------------------------------------------
    // Position conversions
    // -------------------------------------------------------------------------

    /// Converts a character offset to a UTF-8 byte offset.
    ///
    /// # Errors
    /// Returns `TextError::CharIndexOutOfBounds` if `char_idx > self.len_chars()`.
    pub fn char_to_byte(&self, char_idx: usize) -> Result<usize, TextError> {
        let len = self.len_chars();
        if char_idx > len {
            return Err(TextError::CharIndexOutOfBounds { char_idx, len });
        }
        Ok(self.rope.char_to_byte(char_idx))
    }

    /// Converts a UTF-8 byte offset to a character offset.
    ///
    /// # Errors
    /// Returns `TextError::ByteIndexOutOfBounds` if `byte_idx > self.len_bytes()`.
    pub fn byte_to_char(&self, byte_idx: usize) -> Result<usize, TextError> {
        let len = self.len_bytes();
        if byte_idx > len {
            return Err(TextError::ByteIndexOutOfBounds { byte_idx, len });
        }
        // Verify that byte_idx lands on a UTF-8 code point boundary
        // ropey's byte_to_char handles slice conversion; we check char boundary
        let char_idx = self.rope.byte_to_char(byte_idx);
        let round_trip_byte = self.rope.char_to_byte(char_idx);
        if round_trip_byte != byte_idx {
            return Err(TextError::ByteIndexOutOfBounds { byte_idx, len });
        }
        Ok(char_idx)
    }

    /// Converts a character offset to a `(line, col)` position (0-indexed character units).
    ///
    /// # Errors
    /// Returns `TextError::CharIndexOutOfBounds` if `char_idx > self.len_chars()`.
    pub fn char_to_line_col(&self, char_idx: usize) -> Result<LineCol, TextError> {
        let len = self.len_chars();
        if char_idx > len {
            return Err(TextError::CharIndexOutOfBounds { char_idx, len });
        }
        let line = self.rope.char_to_line(char_idx);
        let line_start_char = self.rope.line_to_char(line);
        let col = char_idx - line_start_char;
        Ok(LineCol::new(line, col))
    }

    /// Converts a `(line, col)` position to a character offset.
    ///
    /// # Errors
    /// Returns `TextError::LineIndexOutOfBounds` if `pos.line >= self.len_lines()`, or
    /// `TextError::CharIndexOutOfBounds` if `pos.col` exceeds the length of the line.
    pub fn line_col_to_char(&self, pos: LineCol) -> Result<usize, TextError> {
        let num_lines = self.len_lines();
        if pos.line >= num_lines {
            return Err(TextError::LineIndexOutOfBounds {
                line_idx: pos.line,
                len: num_lines,
            });
        }
        let line_slice = self.rope.line(pos.line);
        let line_len_chars = line_slice.len_chars();
        if pos.col > line_len_chars {
            return Err(TextError::CharIndexOutOfBounds {
                char_idx: pos.col,
                len: line_len_chars,
            });
        }
        let line_start_char = self.rope.line_to_char(pos.line);
        Ok(line_start_char + pos.col)
    }

    /// Converts a character offset to a UTF-16 code unit offset.
    ///
    /// # Errors
    /// Returns `TextError::CharIndexOutOfBounds` if `char_idx > self.len_chars()`.
    pub fn char_to_utf16_cu(&self, char_idx: usize) -> Result<usize, TextError> {
        let len = self.len_chars();
        if char_idx > len {
            return Err(TextError::CharIndexOutOfBounds { char_idx, len });
        }
        let slice = self.rope.slice(0..char_idx);
        let utf16_cu = slice.chars().map(char::len_utf16).sum();
        Ok(utf16_cu)
    }

    /// Converts a UTF-16 code unit offset to a character offset.
    ///
    /// # Errors
    /// Returns `TextError::Utf16IndexOutOfBounds` if `utf16_idx` exceeds total UTF-16 length.
    pub fn utf16_cu_to_char(&self, utf16_idx: usize) -> Result<usize, TextError> {
        let mut current_utf16 = 0;
        let mut char_count = 0;

        for ch in self.rope.chars() {
            if current_utf16 == utf16_idx {
                return Ok(char_count);
            }
            if current_utf16 > utf16_idx {
                // Lands inside a surrogate pair
                let total_utf16: usize = self.rope.chars().map(char::len_utf16).sum();
                return Err(TextError::Utf16IndexOutOfBounds {
                    utf16_idx,
                    len: total_utf16,
                });
            }
            current_utf16 += ch.len_utf16();
            char_count += 1;
        }

        if current_utf16 == utf16_idx {
            return Ok(char_count);
        }

        Err(TextError::Utf16IndexOutOfBounds {
            utf16_idx,
            len: current_utf16,
        })
    }

    /// Converts a character offset to LSP UTF-16 `(line, col)` position.
    ///
    /// # Errors
    /// Returns `TextError::CharIndexOutOfBounds` if `char_idx > self.len_chars()`.
    pub fn char_to_utf16_line_col(&self, char_idx: usize) -> Result<LineCol, TextError> {
        let pos = self.char_to_line_col(char_idx)?;
        let line_start_char = self.rope.line_to_char(pos.line);
        let line_slice = self.rope.slice(line_start_char..char_idx);
        let col_utf16 = line_slice.chars().map(char::len_utf16).sum();
        Ok(LineCol::new(pos.line, col_utf16))
    }

    /// Converts an LSP UTF-16 `(line, col)` position to a character offset.
    ///
    /// # Errors
    /// Returns `TextError::LineIndexOutOfBounds` or `TextError::Utf16IndexOutOfBounds`.
    pub fn utf16_line_col_to_char(&self, pos: LineCol) -> Result<usize, TextError> {
        let num_lines = self.len_lines();
        if pos.line >= num_lines {
            return Err(TextError::LineIndexOutOfBounds {
                line_idx: pos.line,
                len: num_lines,
            });
        }
        let line_slice = self.rope.line(pos.line);
        let mut current_col_utf16 = 0;
        let mut col_chars = 0;

        for ch in line_slice.chars() {
            if current_col_utf16 == pos.col {
                let line_start_char = self.rope.line_to_char(pos.line);
                return Ok(line_start_char + col_chars);
            }
            if current_col_utf16 > pos.col {
                let total_line_utf16: usize = line_slice.chars().map(char::len_utf16).sum();
                return Err(TextError::Utf16IndexOutOfBounds {
                    utf16_idx: pos.col,
                    len: total_line_utf16,
                });
            }
            current_col_utf16 += ch.len_utf16();
            col_chars += 1;
        }

        if current_col_utf16 == pos.col {
            let line_start_char = self.rope.line_to_char(pos.line);
            return Ok(line_start_char + col_chars);
        }

        Err(TextError::Utf16IndexOutOfBounds {
            utf16_idx: pos.col,
            len: current_col_utf16,
        })
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
        }
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl FromStr for Text {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.rope.chunks() {
            f.write_str(chunk)?;
        }
        Ok(())
    }
}
