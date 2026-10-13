//! Line ending detection, normalization, and preservation.

/// Represents line break conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineEnding {
    /// Unix line ending (`\n`, LF).
    #[default]
    Lf,
    /// Windows/DOS line ending (`\r\n`, CRLF).
    Crlf,
    /// Classic Mac line ending (`\r`, CR).
    Cr,
}

impl LineEnding {
    /// Returns the string representation of this line ending.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

/// Result of analyzing line endings in a byte sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineEndingDetection {
    /// The dominant line ending style found. If no line endings exist, defaults to `Lf`.
    pub dominant: LineEnding,
    /// True if more than one distinct line ending style was present.
    pub is_mixed: bool,
}

/// Detects line endings in a byte slice.
///
/// Rules:
/// - Counts occurrences of `\r\n` (CRLF), lone `\n` (LF), and lone `\r` (CR).
/// - If no line endings are found, defaults to `LineEnding::Lf` with `is_mixed = false`.
/// - If more than one style is present, `is_mixed = true`, and dominant is the style with highest count.
#[must_use]
pub fn detect_line_endings(bytes: &[u8]) -> LineEndingDetection {
    let mut crlf_count = 0usize;
    let mut lf_count = 0usize;
    let mut cr_count = 0usize;

    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\r' {
            if i + 1 < len && bytes[i + 1] == b'\n' {
                crlf_count += 1;
                i += 2;
                continue;
            }
            cr_count += 1;
        } else if bytes[i] == b'\n' {
            lf_count += 1;
        }
        i += 1;
    }

    let distinct_styles =
        usize::from(crlf_count > 0) + usize::from(lf_count > 0) + usize::from(cr_count > 0);

    let is_mixed = distinct_styles > 1;

    let dominant = if crlf_count == 0 && lf_count == 0 && cr_count == 0 {
        LineEnding::Lf
    } else if crlf_count >= lf_count && crlf_count >= cr_count {
        LineEnding::Crlf
    } else if lf_count >= cr_count {
        LineEnding::Lf
    } else {
        LineEnding::Cr
    };

    LineEndingDetection { dominant, is_mixed }
}

/// Normalizes line endings to `\n` (LF).
///
/// Rules:
/// - CRLF (`\r\n`) is always converted to `\n`.
/// - Lone CR (`\r`) is converted to `\n` ONLY when the input contains no LF (`\n`) at all
///   (classic Mac OS CR-style files).
/// - In files that contain any LF (or CRLF), lone CR is preserved as a normal character.
#[must_use]
pub fn normalize_line_endings(text: &str) -> String {
    let has_lf = text.contains('\n');

    if !has_lf {
        // Old CR-style files: convert lone CR to \n
        if text.contains('\r') {
            return text.replace('\r', "\n");
        }
        return text.to_string();
    }

    // Contains LF (or CRLF). Replace \r\n with \n, leaving stray \r intact.
    text.replace("\r\n", "\n")
}

/// Denormalizes normalized `\n` text back to the specified line ending style.
#[must_use]
pub fn denormalize_line_endings(text: &str, ending: LineEnding) -> String {
    match ending {
        LineEnding::Lf => text.to_string(),
        LineEnding::Crlf => text.replace('\n', "\r\n"),
        LineEnding::Cr => text.replace('\n', "\r"),
    }
}
