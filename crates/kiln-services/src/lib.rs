//! `kiln-services` crate: lsp, search, git, file watcher, agent/pty (async).

pub mod file_io;

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-services"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-services");
        assert_eq!(kiln_core::name(), "kiln-core");
        assert_eq!(kiln_syntax::name(), "kiln-syntax");
        assert_eq!(kiln_config::name(), "kiln-config");
    }
}
