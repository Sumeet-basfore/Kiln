//! `kiln-input` crate: key parsing, mode state machine, keymaps -> commands.

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-input"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-input");
        assert_eq!(kiln_core::name(), "kiln-core");
        assert_eq!(kiln_config::name(), "kiln-config");
    }
}
