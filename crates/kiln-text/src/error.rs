//! Error types for the `kiln-text` crate.

use thiserror::Error;

/// Errors returned by text operations when offsets or ranges are invalid.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum TextError {
    /// A character offset was out of bounds.
    #[error("Character offset out of bounds: index {char_idx}, len {len}")]
    CharIndexOutOfBounds {
        /// The invalid character index.
        char_idx: usize,
        /// The total character length of the text.
        len: usize,
    },

    /// A line index was out of bounds.
    #[error("Line index out of bounds: line {line_idx}, len {len}")]
    LineIndexOutOfBounds {
        /// The invalid line index.
        line_idx: usize,
        /// The total line count of the text.
        len: usize,
    },

    /// A byte offset was out of bounds or not on a UTF-8 character boundary.
    #[error("Byte offset out of bounds or not on a char boundary: index {byte_idx}, len {len}")]
    ByteIndexOutOfBounds {
        /// The invalid byte index.
        byte_idx: usize,
        /// The total byte length of the text.
        len: usize,
    },

    /// A UTF-16 code unit offset was out of bounds.
    #[error("UTF-16 code unit offset out of bounds: index {utf16_idx}, len {len}")]
    Utf16IndexOutOfBounds {
        /// The invalid UTF-16 code unit index.
        utf16_idx: usize,
        /// The total UTF-16 length of the text.
        len: usize,
    },

    /// An invalid range was specified where start > end.
    #[error("Invalid character range: start {start} > end {end}")]
    InvalidCharRange {
        /// The start character index.
        start: usize,
        /// The end character index.
        end: usize,
    },
}
