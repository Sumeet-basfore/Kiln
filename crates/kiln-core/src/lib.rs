//! `kiln-core` crate: buffers, selections, transactions, undo tree, registers, commands, and events.

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-core"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-core");
        assert_eq!(kiln_text::name(), "kiln-text");
    }
}
