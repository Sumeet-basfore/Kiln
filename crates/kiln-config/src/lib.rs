//! `kiln-config` crate: TOML loading, validation, defaults, themes, keymaps.

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-config"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-config");
    }
}
