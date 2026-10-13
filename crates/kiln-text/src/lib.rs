//! `kiln-text` crate: rope wrapper, graphemes, encoding, and line endings.

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-text"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-text");
    }
}
