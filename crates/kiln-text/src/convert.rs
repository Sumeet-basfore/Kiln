//! Position conversion types and structures.

/// A line and column position in character offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LineCol {
    /// 0-indexed line number.
    pub line: usize,
    /// 0-indexed column offset (in characters).
    pub col: usize,
}

impl LineCol {
    /// Creates a new `LineCol` position.
    #[must_use]
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}
