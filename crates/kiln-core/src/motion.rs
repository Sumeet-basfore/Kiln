//! Pure motion and text-object calculations over document text.
//!
//! All calculations are character-offset based, respecting grapheme clusters and wide characters.

use kiln_text::{next_grapheme_boundary, prev_grapheme_boundary, LineCol, Text};

use crate::selection::Range;

/// Pure motion calculations.
pub struct MotionEvaluator;

impl MotionEvaluator {
    /// Moves cursor left by `count` graphemes, clamped to the beginning of the current line.
    #[must_use]
    pub fn move_left(text: &Text, pos: usize, count: usize) -> usize {
        let mut cur = pos;
        let line_idx = text.char_to_line_col(pos).map(|lc| lc.line).unwrap_or(0);
        let line_start = text
            .line_col_to_char(LineCol::new(line_idx, 0))
            .unwrap_or(0);

        for _ in 0..count {
            if cur <= line_start {
                break;
            }
            cur = prev_grapheme_boundary(text, cur).max(line_start);
        }
        cur
    }

    /// Moves cursor right by `count` graphemes, clamped to the end of the current line (excluding `\n`).
    #[must_use]
    pub fn move_right(text: &Text, pos: usize, count: usize) -> usize {
        let mut cur = pos;
        let line_idx = text.char_to_line_col(pos).map(|lc| lc.line).unwrap_or(0);
        let line_end = Self::line_end_pos(text, line_idx);

        for _ in 0..count {
            if cur >= line_end {
                break;
            }
            cur = next_grapheme_boundary(text, cur).min(line_end);
        }
        cur
    }

    /// Moves cursor up by `count` lines, maintaining column if possible.
    #[must_use]
    pub fn move_up(text: &Text, pos: usize, count: usize) -> usize {
        if let Ok(lc) = text.char_to_line_col(pos) {
            let target_line = lc.line.saturating_sub(count);
            Self::goto_line_col(text, target_line, lc.col)
        } else {
            pos
        }
    }

    /// Moves cursor down by `count` lines, maintaining column if possible.
    #[must_use]
    pub fn move_down(text: &Text, pos: usize, count: usize) -> usize {
        if let Ok(lc) = text.char_to_line_col(pos) {
            let max_line = text.len_lines().saturating_sub(1);
            let target_line = (lc.line + count).min(max_line);
            Self::goto_line_col(text, target_line, lc.col)
        } else {
            pos
        }
    }

    /// Moves cursor to the beginning of the line (`0`).
    #[must_use]
    pub fn line_start(text: &Text, pos: usize) -> usize {
        let line_idx = text.char_to_line_col(pos).map(|lc| lc.line).unwrap_or(0);
        text.line_col_to_char(LineCol::new(line_idx, 0))
            .unwrap_or(0)
    }

    /// Moves cursor to the first non-whitespace character on the line (`^`).
    #[must_use]
    pub fn first_non_blank(text: &Text, pos: usize) -> usize {
        let line_idx = text.char_to_line_col(pos).map(|lc| lc.line).unwrap_or(0);
        let start = text
            .line_col_to_char(LineCol::new(line_idx, 0))
            .unwrap_or(0);
        if let Ok(line_str) = text.line(line_idx) {
            for (idx, ch) in line_str.char_indices() {
                if ch == '\n' {
                    break;
                }
                if !ch.is_whitespace() {
                    return start + idx;
                }
            }
        }
        start
    }

    /// Moves cursor to the end of the line (`$`).
    #[must_use]
    pub fn line_end(text: &Text, pos: usize) -> usize {
        let line_idx = text.char_to_line_col(pos).map(|lc| lc.line).unwrap_or(0);
        Self::line_end_pos(text, line_idx)
    }

    /// Moves cursor to the start of the buffer (`gg`).
    #[must_use]
    pub fn buffer_start(_text: &Text) -> usize {
        0
    }

    /// Moves cursor to the last line of the buffer (`G`).
    #[must_use]
    pub fn buffer_end(text: &Text) -> usize {
        let last_line = text.len_lines().saturating_sub(1);
        Self::line_start(
            text,
            text.line_col_to_char(LineCol::new(last_line, 0))
                .unwrap_or(0),
        )
    }

    /// Word forward (`w`): start of next word.
    #[must_use]
    pub fn word_forward(text: &Text, pos: usize, count: usize) -> usize {
        let mut cur = pos;
        let len = text.len_chars();
        for _ in 0..count {
            cur = Self::single_word_forward(text, cur, false);
            if cur >= len {
                break;
            }
        }
        cur
    }

    /// WORDS forward (`W`): start of next whitespace-delimited word.
    #[must_use]
    pub fn words_forward(text: &Text, pos: usize, count: usize) -> usize {
        let mut cur = pos;
        let len = text.len_chars();
        for _ in 0..count {
            cur = Self::single_word_forward(text, cur, true);
            if cur >= len {
                break;
            }
        }
        cur
    }

    /// Word backward (`b`): start of previous word.
    #[must_use]
    pub fn word_backward(text: &Text, pos: usize, count: usize) -> usize {
        let mut cur = pos;
        for _ in 0..count {
            cur = Self::single_word_backward(text, cur, false);
            if cur == 0 {
                break;
            }
        }
        cur
    }

    /// WORDS backward (`B`): start of previous whitespace-delimited word.
    #[must_use]
    pub fn words_backward(text: &Text, pos: usize, count: usize) -> usize {
        let mut cur = pos;
        for _ in 0..count {
            cur = Self::single_word_backward(text, cur, true);
            if cur == 0 {
                break;
            }
        }
        cur
    }

    /// Word end (`e`): end of current or next word.
    #[must_use]
    pub fn word_end(text: &Text, pos: usize, count: usize) -> usize {
        let mut cur = pos;
        let len = text.len_chars();
        for _ in 0..count {
            cur = Self::single_word_end(text, cur, false);
            if cur >= len {
                break;
            }
        }
        cur
    }

    /// WORDS end (`E`): end of current or next whitespace-delimited word.
    #[must_use]
    pub fn words_end(text: &Text, pos: usize, count: usize) -> usize {
        let mut cur = pos;
        let len = text.len_chars();
        for _ in 0..count {
            cur = Self::single_word_end(text, cur, true);
            if cur >= len {
                break;
            }
        }
        cur
    }

    /// Find character forward on current line (`f{c}`).
    #[must_use]
    pub fn find_forward(text: &Text, pos: usize, target: char, count: usize) -> Option<usize> {
        let line_idx = text.char_to_line_col(pos).map(|lc| lc.line).ok()?;
        let line_slice = text.line(line_idx).ok()?;
        let line_start = text.line_col_to_char(LineCol::new(line_idx, 0)).ok()?;

        let mut matches = 0;
        for (idx, ch) in line_slice.char_indices() {
            let char_pos = line_start + idx;
            if char_pos > pos && ch == target {
                matches += 1;
                if matches == count {
                    return Some(char_pos);
                }
            }
        }
        None
    }

    /// Find character backward on current line (`F{c}`).
    #[must_use]
    pub fn find_backward(text: &Text, pos: usize, target: char, count: usize) -> Option<usize> {
        let line_idx = text.char_to_line_col(pos).map(|lc| lc.line).ok()?;
        let line_slice = text.line(line_idx).ok()?;
        let line_start = text.line_col_to_char(LineCol::new(line_idx, 0)).ok()?;

        let mut matches = 0;
        let chars_rev: Vec<(usize, char)> = line_slice.char_indices().collect();
        for &(idx, ch) in chars_rev.iter().rev() {
            let char_pos = line_start + idx;
            if char_pos < pos && ch == target {
                matches += 1;
                if matches == count {
                    return Some(char_pos);
                }
            }
        }
        None
    }

    /// Till character forward on current line (`t{c}`).
    #[must_use]
    pub fn till_forward(text: &Text, pos: usize, target: char, count: usize) -> Option<usize> {
        let found = Self::find_forward(text, pos, target, count)?;
        Some(prev_grapheme_boundary(text, found))
    }

    /// Till character backward on current line (`T{c}`).
    #[must_use]
    pub fn till_backward(text: &Text, pos: usize, target: char, count: usize) -> Option<usize> {
        let found = Self::find_backward(text, pos, target, count)?;
        Some(next_grapheme_boundary(text, found))
    }

    /// Jump to matching bracket or parenthesis (`%`).
    #[must_use]
    pub fn match_pair(text: &Text, pos: usize) -> Option<usize> {
        let ch = text.slice(pos..(pos + 1)).ok()?.chars().next()?;
        let (open, close, forward) = match ch {
            '(' => ('(', ')', true),
            ')' => ('(', ')', false),
            '[' => ('[', ']', true),
            ']' => ('[', ']', false),
            '{' => ('{', '}', true),
            '}' => ('{', '}', false),
            _ => return None,
        };

        let mut depth = 0;
        let len = text.len_chars();
        if forward {
            for idx in pos..len {
                if let Ok(slice) = text.slice(idx..(idx + 1)) {
                    if let Some(c) = slice.chars().next() {
                        if c == open {
                            depth += 1;
                        } else if c == close {
                            depth -= 1;
                            if depth == 0 {
                                return Some(idx);
                            }
                        }
                    }
                }
            }
        } else {
            for idx in (0..=pos).rev() {
                if let Ok(slice) = text.slice(idx..(idx + 1)) {
                    if let Some(c) = slice.chars().next() {
                        if c == close {
                            depth += 1;
                        } else if c == open {
                            depth -= 1;
                            if depth == 0 {
                                return Some(idx);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    // --- Helpers ---

    fn line_end_pos(text: &Text, line_idx: usize) -> usize {
        let start = text
            .line_col_to_char(LineCol::new(line_idx, 0))
            .unwrap_or(0);
        if let Ok(line_slice) = text.line(line_idx) {
            let mut line_len = line_slice.chars().count();
            if line_slice.ends_with('\n') {
                line_len = line_len.saturating_sub(1);
            }
            start + line_len
        } else {
            start
        }
    }

    fn goto_line_col(text: &Text, line_idx: usize, col: usize) -> usize {
        let line_end = Self::line_end_pos(text, line_idx);
        let line_start = text
            .line_col_to_char(LineCol::new(line_idx, 0))
            .unwrap_or(0);
        (line_start + col).min(line_end)
    }

    fn char_class(c: char, big_word: bool) -> u8 {
        if c.is_whitespace() {
            0
        } else if big_word || c.is_alphanumeric() || c == '_' {
            1
        } else {
            2
        }
    }

    fn single_word_forward(text: &Text, pos: usize, big: bool) -> usize {
        let len = text.len_chars();
        if pos >= len {
            return len;
        }

        let cur_char = text
            .slice(pos..(pos + 1))
            .ok()
            .and_then(|s| s.chars().next())
            .unwrap_or(' ');
        let start_class = Self::char_class(cur_char, big);

        let mut idx = pos;
        // Step 1: consume same class if non-whitespace
        if start_class != 0 {
            while idx < len {
                let c = text
                    .slice(idx..(idx + 1))
                    .ok()
                    .and_then(|s| s.chars().next())
                    .unwrap_or(' ');
                if Self::char_class(c, big) != start_class {
                    break;
                }
                idx += 1;
            }
        }

        // Step 2: consume whitespace
        while idx < len {
            let c = text
                .slice(idx..(idx + 1))
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or('a');
            if !c.is_whitespace() {
                break;
            }
            idx += 1;
        }

        idx.min(len)
    }

    fn single_word_backward(text: &Text, pos: usize, big: bool) -> usize {
        if pos == 0 {
            return 0;
        }

        let mut idx = pos.saturating_sub(1);

        // Skip whitespace backward
        while idx > 0 {
            let c = text
                .slice(idx..(idx + 1))
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or(' ');
            if !c.is_whitespace() {
                break;
            }
            idx -= 1;
        }

        let cur_char = text
            .slice(idx..(idx + 1))
            .ok()
            .and_then(|s| s.chars().next())
            .unwrap_or(' ');
        let target_class = Self::char_class(cur_char, big);

        // Move to start of this class
        while idx > 0 {
            let prev_c = text
                .slice((idx - 1)..idx)
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or(' ');
            if Self::char_class(prev_c, big) != target_class {
                break;
            }
            idx -= 1;
        }

        idx
    }

    fn single_word_end(text: &Text, pos: usize, big: bool) -> usize {
        let len = text.len_chars();
        if pos + 1 >= len {
            return len.saturating_sub(1);
        }

        let mut idx = pos + 1;

        // Skip whitespace forward
        while idx < len {
            let c = text
                .slice(idx..(idx + 1))
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or(' ');
            if !c.is_whitespace() {
                break;
            }
            idx += 1;
        }

        if idx >= len {
            return len.saturating_sub(1);
        }

        let target_char = text
            .slice(idx..(idx + 1))
            .ok()
            .and_then(|s| s.chars().next())
            .unwrap_or(' ');
        let target_class = Self::char_class(target_char, big);

        while idx + 1 < len {
            let next_c = text
                .slice((idx + 1)..(idx + 2))
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or(' ');
            if Self::char_class(next_c, big) != target_class {
                break;
            }
            idx += 1;
        }

        idx
    }
}

/// Pure text-object range calculations.
pub struct TextObjectEvaluator;

impl TextObjectEvaluator {
    /// Evaluates `iw` (inner word) around `pos`.
    #[must_use]
    pub fn inner_word(text: &Text, pos: usize) -> Option<Range> {
        let len = text.len_chars();
        if pos >= len {
            return None;
        }

        let cur_char = text.slice(pos..(pos + 1)).ok()?.chars().next()?;
        let class = MotionEvaluator::char_class(cur_char, false);

        let mut start = pos;
        while start > 0 {
            let prev = text.slice((start - 1)..start).ok()?.chars().next()?;
            if MotionEvaluator::char_class(prev, false) != class {
                break;
            }
            start -= 1;
        }

        let mut end = pos + 1;
        while end < len {
            let next = text.slice(end..(end + 1)).ok()?.chars().next()?;
            if MotionEvaluator::char_class(next, false) != class {
                break;
            }
            end += 1;
        }

        Some(Range::new(start, end))
    }

    /// Evaluates `aw` (a word): inner word plus trailing whitespace (or leading if trailing is absent).
    #[must_use]
    pub fn around_word(text: &Text, pos: usize) -> Option<Range> {
        let inner = Self::inner_word(text, pos)?;
        let mut end = inner.to();
        let len = text.len_chars();

        // Try consuming trailing whitespace
        let mut consumed_trailing = false;
        while end < len {
            let next = text.slice(end..(end + 1)).ok()?.chars().next()?;
            if next.is_whitespace() && next != '\n' {
                end += 1;
                consumed_trailing = true;
            } else {
                break;
            }
        }

        let mut start = inner.from();
        if !consumed_trailing {
            // Consume leading whitespace
            while start > 0 {
                let prev = text.slice((start - 1)..start).ok()?.chars().next()?;
                if prev.is_whitespace() && prev != '\n' {
                    start -= 1;
                } else {
                    break;
                }
            }
        }

        Some(Range::new(start, end))
    }

    /// Evaluates `i"` / `a"` (inner/around quote).
    #[must_use]
    pub fn quote(text: &Text, pos: usize, quote_char: char, around: bool) -> Option<Range> {
        let line_idx = text.char_to_line_col(pos).map(|lc| lc.line).ok()?;
        let line_slice = text.line(line_idx).ok()?;
        let line_start = text.line_col_to_char(LineCol::new(line_idx, 0)).ok()?;

        let mut quotes = Vec::new();
        for (idx, ch) in line_slice.char_indices() {
            if ch == quote_char {
                quotes.push(line_start + idx);
            }
        }

        // Find pair enclosing pos or closest pair
        for i in 0..quotes.len().saturating_sub(1) {
            let q1 = quotes[i];
            let q2 = quotes[i + 1];
            if q1 <= pos && pos <= q2 {
                return if around {
                    Some(Range::new(q1, q2 + 1))
                } else {
                    Some(Range::new(q1 + 1, q2))
                };
            }
        }

        None
    }

    /// Evaluates `i(` / `a(` (inner/around pair: `()`, `[]`, `{}`).
    #[must_use]
    pub fn pair(text: &Text, pos: usize, open: char, around: bool) -> Option<Range> {
        let close = match open {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            '<' => '>',
            _ => return None,
        };

        // Scan backwards for open
        let mut depth = 0;
        let mut open_idx = None;
        for idx in (0..=pos).rev() {
            let c = text.slice(idx..(idx + 1)).ok()?.chars().next()?;
            if c == close && idx != pos {
                depth += 1;
            } else if c == open {
                if depth == 0 {
                    open_idx = Some(idx);
                    break;
                }
                depth -= 1;
            }
        }

        let start = open_idx?;
        let match_end = MotionEvaluator::match_pair(text, start)?;

        if around {
            Some(Range::new(start, match_end + 1))
        } else {
            Some(Range::new(start + 1, match_end))
        }
    }
}
