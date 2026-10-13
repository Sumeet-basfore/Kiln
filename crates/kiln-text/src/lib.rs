//! `kiln-text`: text primitives and rope wrapper for Kiln.
//!
//! Features:
//! - Character-offset based rope wrapper (`Text`) over `ropey`.
//! - Bidirectional conversions between character offsets, `(line, col)`, UTF-8 byte offsets,
//!   and LSP UTF-16 code unit offsets.
//! - Unicode grapheme cluster boundary navigation and terminal display widths.
//! - Line ending detection, normalization to `\n` in memory, and loss-free denormalization.
//! - Encoding detection (UTF-8, UTF-8 BOM, UTF-16 LE/BE, and Latin-1 fallback).
//! - Fast binary file detection with UTF-16 BOM exception handling.

pub mod binary;
pub mod convert;
pub mod encoding;
pub mod error;
pub mod grapheme;
pub mod line_ending;
pub mod rope;

pub use binary::{is_binary, BINARY_SNIFF_LEN};
pub use convert::LineCol;
pub use encoding::{detect_and_decode, encode, DecodeResult, Encoding};
pub use error::TextError;
pub use grapheme::{
    grapheme_width, next_grapheme_boundary, prev_grapheme_boundary, str_display_width,
};
pub use line_ending::{
    denormalize_line_endings, detect_line_endings, normalize_line_endings, LineEnding,
    LineEndingDetection,
};
pub use rope::Text;

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-text"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_name() {
        let text = Text::from("kiln-text");
        assert_eq!(text.len_chars(), 9);
    }
}
