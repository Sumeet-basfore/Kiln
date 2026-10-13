//! Binary file content sniffing.

/// Number of initial bytes inspected for binary sniffing.
pub const BINARY_SNIFF_LEN: usize = 8192;

/// Inspects the beginning of a byte sequence to determine if it appears to be a binary file.
///
/// Rules:
/// - If the bytes start with a UTF-16 BOM (`[0xFF, 0xFE]` or `[0xFE, 0xFF]`), it is considered
///   text (UTF-16) despite alternating NUL (`\0`) bytes.
/// - Otherwise, if any NUL (`0x00`) byte is present in the first `BINARY_SNIFF_LEN` bytes,
///   it is detected as binary.
#[must_use]
pub fn is_binary(bytes: &[u8]) -> bool {
    // UTF-16 BOM exception: text files in UTF-16 contain legitimate NUL bytes
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return false;
    }

    let sniff_len = bytes.len().min(BINARY_SNIFF_LEN);
    bytes[..sniff_len].contains(&0)
}
