//! Grapheme cluster boundary navigation and terminal display width calculations.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::rope::Text;

/// Returns the next grapheme cluster boundary at or after `char_idx` in character units.
///
/// If `char_idx` is already at the end of the text, returns `text.len_chars()`.
/// If `char_idx > text.len_chars()`, it is clamped to `text.len_chars()`.
#[must_use]
pub fn next_grapheme_boundary(text: &Text, char_idx: usize) -> usize {
    let len = text.len_chars();
    if char_idx >= len {
        return len;
    }

    // Convert text to string or iterate graphemes over slice
    // To handle large buffers and chunk boundaries correctly, slice from char_idx
    let slice_str = match text.slice(char_idx..len) {
        Ok(s) => s,
        Err(_) => return len,
    };

    let mut indices = slice_str.grapheme_indices(true);
    // First grapheme starts at byte 0
    if indices.next().is_some() {
        if let Some((next_byte_offset, _)) = indices.next() {
            // Count characters in the first grapheme
            let first_grapheme_chars = slice_str[..next_byte_offset].chars().count();
            return char_idx + first_grapheme_chars;
        }
    }

    len
}

/// Returns the previous grapheme cluster boundary at or before `char_idx` in character units.
///
/// If `char_idx == 0`, returns `0`.
/// If `char_idx > text.len_chars()`, it starts search from `text.len_chars()`.
#[must_use]
pub fn prev_grapheme_boundary(text: &Text, char_idx: usize) -> usize {
    let len = text.len_chars();
    let char_idx = char_idx.min(len);
    if char_idx == 0 {
        return 0;
    }

    let slice_str = match text.slice(0..char_idx) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    // Find the last grapheme in slice_str[0..char_idx]
    if let Some((last_byte_offset, _)) = slice_str.grapheme_indices(true).next_back() {
        let prefix_chars = slice_str[..last_byte_offset].chars().count();
        return prefix_chars;
    }

    0
}

/// Computes the terminal display width of a single grapheme cluster string.
///
/// Returns:
/// - 0 for combining characters, zero-width joiners, and zero-width spaces
/// - 1 for standard half-width characters
/// - 2 for full-width and East Asian Wide (CJK) characters
///
/// Control characters (e.g. tabs, control codes) return safe fixed display widths.
#[must_use]
pub fn grapheme_width(grapheme: &str) -> usize {
    // Treat tabs as 1 width unit at the primitive level (renderer handles tabstops)
    if grapheme == "\t" {
        return 1;
    }
    UnicodeWidthStr::width(grapheme)
}

/// Computes the terminal display width of an entire string.
#[must_use]
pub fn str_display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}
