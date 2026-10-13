//! Character encoding detection, decoding, and encoding.
//!
//! Supported encodings:
//! - UTF-8 (default standard)
//! - UTF-8 with BOM (`[0xEF, 0xBB, 0xBF]`)
//! - UTF-16 LE with BOM (`[0xFF, 0xFE]`)
//! - UTF-16 BE with BOM (`[0xFE, 0xFF]`)
//! - Latin-1 / ISO-8859-1 (fallback heuristic for invalid UTF-8; byte value = Unicode code point)

use crate::line_ending::{detect_line_endings, normalize_line_endings, LineEndingDetection};

/// Character encoding formats recognized by Kiln.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Encoding {
    /// UTF-8 without BOM.
    #[default]
    Utf8,
    /// UTF-8 with Byte Order Mark (`0xEF, 0xBB, 0xBF`).
    Utf8Bom,
    /// UTF-16 Little Endian (requires BOM for automatic detection).
    Utf16Le,
    /// UTF-16 Big Endian (requires BOM for automatic detection).
    Utf16Be,
    /// Latin-1 / ISO-8859-1 fallback heuristic for invalid UTF-8.
    Latin1,
}

/// The result of detecting encoding and decoding byte content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeResult {
    /// The decoded, line-normalized text.
    pub text: String,
    /// The detected encoding.
    pub encoding: Encoding,
    /// Information about the original line endings.
    pub line_ending: LineEndingDetection,
    /// Whether any bytes/characters could not be losslessly preserved.
    pub is_lossy: bool,
}

/// Detects the character encoding, decodes the bytes into a `String`, and normalizes line endings.
///
/// Detection order:
/// 1. UTF-8 BOM (`[0xEF, 0xBB, 0xBF]`)
/// 2. UTF-16 LE BOM (`[0xFF, 0xFE]`)
/// 3. UTF-16 BE BOM (`[0xFE, 0xFF]`)
/// 4. Valid UTF-8 (no BOM)
/// 5. Latin-1 / ISO-8859-1 heuristic fallback
#[must_use]
pub fn detect_and_decode(bytes: &[u8]) -> DecodeResult {
    let line_ending = detect_line_endings(bytes);

    // 1. Check UTF-8 BOM
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let content = &bytes[3..];
        match std::str::from_utf8(content) {
            Ok(s) => {
                return DecodeResult {
                    text: normalize_line_endings(s),
                    encoding: Encoding::Utf8Bom,
                    line_ending,
                    is_lossy: false,
                };
            }
            Err(_) => {
                let (lossy_s, _) = decode_utf8_lossy(content);
                return DecodeResult {
                    text: normalize_line_endings(&lossy_s),
                    encoding: Encoding::Utf8Bom,
                    line_ending,
                    is_lossy: true,
                };
            }
        }
    }

    // 2. Check UTF-16 LE BOM
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let content = &bytes[2..];
        let (text, is_lossy) = decode_utf16_le(content);
        return DecodeResult {
            text: normalize_line_endings(&text),
            encoding: Encoding::Utf16Le,
            line_ending,
            is_lossy,
        };
    }

    // 3. Check UTF-16 BE BOM
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let content = &bytes[2..];
        let (text, is_lossy) = decode_utf16_be(content);
        return DecodeResult {
            text: normalize_line_endings(&text),
            encoding: Encoding::Utf16Be,
            line_ending,
            is_lossy,
        };
    }

    // 4. Try UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return DecodeResult {
            text: normalize_line_endings(s),
            encoding: Encoding::Utf8,
            line_ending,
            is_lossy: false,
        };
    }

    // 5. Fallback to Latin-1 (ISO-8859-1)
    // Every byte maps directly to U+0000..=U+00FF
    let text: String = bytes.iter().map(|&b| b as char).collect();
    DecodeResult {
        text: normalize_line_endings(&text),
        encoding: Encoding::Latin1,
        line_ending,
        is_lossy: false,
    }
}

/// Decodes UTF-16 LE byte slice.
fn decode_utf16_le(bytes: &[u8]) -> (String, bool) {
    let mut u16_units = Vec::with_capacity(bytes.len() / 2);
    let (chunks, remainder) = bytes.as_chunks::<2>();
    let mut is_lossy = !remainder.is_empty();

    for chunk in chunks {
        u16_units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    let mut decoded = String::with_capacity(u16_units.len());
    for res in char::decode_utf16(u16_units) {
        match res {
            Ok(c) => decoded.push(c),
            Err(_) => {
                decoded.push(char::REPLACEMENT_CHARACTER);
                is_lossy = true;
            }
        }
    }

    (decoded, is_lossy)
}

/// Decodes UTF-16 BE byte slice.
fn decode_utf16_be(bytes: &[u8]) -> (String, bool) {
    let mut u16_units = Vec::with_capacity(bytes.len() / 2);
    let (chunks, remainder) = bytes.as_chunks::<2>();
    let mut is_lossy = !remainder.is_empty();

    for chunk in chunks {
        u16_units.push(u16::from_be_bytes([chunk[0], chunk[1]]));
    }

    let mut decoded = String::with_capacity(u16_units.len());
    for res in char::decode_utf16(u16_units) {
        match res {
            Ok(c) => decoded.push(c),
            Err(_) => {
                decoded.push(char::REPLACEMENT_CHARACTER);
                is_lossy = true;
            }
        }
    }

    (decoded, is_lossy)
}

fn decode_utf8_lossy(bytes: &[u8]) -> (String, bool) {
    let lossy = String::from_utf8_lossy(bytes);
    let is_lossy = lossy.contains(char::REPLACEMENT_CHARACTER);
    (lossy.into_owned(), is_lossy)
}

/// Encodes text into bytes using the given encoding.
///
/// Re-attaches BOM for `Utf8Bom`, `Utf16Le`, and `Utf16Be`.
/// Returns `(encoded_bytes, is_lossy)`.
#[must_use]
pub fn encode(text: &str, encoding: Encoding) -> (Vec<u8>, bool) {
    match encoding {
        Encoding::Utf8 => (text.as_bytes().to_vec(), false),
        Encoding::Utf8Bom => {
            let mut bytes = Vec::with_capacity(3 + text.len());
            bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
            bytes.extend_from_slice(text.as_bytes());
            (bytes, false)
        }
        Encoding::Utf16Le => {
            let mut bytes = Vec::with_capacity(2 + text.len() * 2);
            // Write BOM
            bytes.extend_from_slice(&[0xFF, 0xFE]);
            for c in text.chars() {
                let mut buf = [0u16; 2];
                let encoded = c.encode_utf16(&mut buf);
                for &u in &*encoded {
                    bytes.extend_from_slice(&u.to_le_bytes());
                }
            }
            (bytes, false)
        }
        Encoding::Utf16Be => {
            let mut bytes = Vec::with_capacity(2 + text.len() * 2);
            // Write BOM
            bytes.extend_from_slice(&[0xFE, 0xFF]);
            for c in text.chars() {
                let mut buf = [0u16; 2];
                let encoded = c.encode_utf16(&mut buf);
                for &u in &*encoded {
                    bytes.extend_from_slice(&u.to_be_bytes());
                }
            }
            (bytes, false)
        }
        Encoding::Latin1 => {
            let mut bytes = Vec::with_capacity(text.len());
            let mut is_lossy = false;
            for c in text.chars() {
                let code = c as u32;
                if code <= 0xFF {
                    bytes.push(code as u8);
                } else {
                    // Cannot be represented in Latin-1
                    bytes.push(b'?');
                    is_lossy = true;
                }
            }
            (bytes, is_lossy)
        }
    }
}
