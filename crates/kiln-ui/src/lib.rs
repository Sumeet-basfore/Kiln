//! `kiln-ui` crate: terminal rendering, layout, widgets, themes.

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-ui"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-ui");
        assert_eq!(kiln_core::name(), "kiln-core");
        assert_eq!(kiln_config::name(), "kiln-config");
    }
}
