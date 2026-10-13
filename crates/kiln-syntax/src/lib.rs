//! `kiln-syntax` crate: tree-sitter parsing, highlight/indent/text-object queries.

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-syntax"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-syntax");
        assert_eq!(kiln_text::name(), "kiln-text");
    }
}
