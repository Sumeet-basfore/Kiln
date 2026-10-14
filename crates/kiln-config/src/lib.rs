//! Global TOML configuration, validation, theme loading, and JSON Schema.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

/// Typed global Kiln configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Editor behavior and layout options.
    pub editor: EditorConfig,
    /// Key sequence overrides, grouped by mode.
    pub keybindings: KeybindingsConfig,
    /// Optional custom theme file path.
    pub theme: ThemeConfig,
}

/// Editor settings with usable defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct EditorConfig {
    /// Number of spaces represented by a tab.
    pub tab_width: usize,
    /// Number of context lines to keep around the cursor.
    pub scrolloff: usize,
    /// Whether mouse input is enabled.
    pub mouse: bool,
    /// Whether hybrid line numbers are shown in the gutter.
    pub line_numbers: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            scrolloff: 3,
            mouse: true,
            line_numbers: true,
        }
    }
}

/// Keybinding overrides stored as mode -> key sequence -> command name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default, deny_unknown_fields)]
pub struct KeybindingsConfig {
    /// Mode-specific bindings, such as `normal` and `insert`.
    pub modes: BTreeMap<String, BTreeMap<String, String>>,
}

/// Custom theme-file selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeConfig {
    /// Optional path to a custom theme TOML file.
    pub file: Option<String>,
}

/// Color value supported by a basic theme file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ThemeColor {
    /// Named terminal color (for example `red` or `cyan`).
    Name(String),
    /// RGB triplet.
    Rgb([u8; 3]),
}

/// Basic style parsed from a theme file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeStyle {
    /// Foreground color.
    pub foreground: Option<ThemeColor>,
    /// Background color.
    pub background: Option<ThemeColor>,
    /// Style modifiers.
    pub modifiers: Vec<String>,
}

/// A parsed custom theme file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ThemeFile {
    /// Display name for the theme.
    pub name: String,
    /// Semantic UI scope names mapped to styles.
    pub scopes: BTreeMap<String, ThemeStyle>,
}

/// A user-facing configuration diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    /// Source file path.
    pub path: PathBuf,
    /// One-based source line, when available.
    pub line: Option<usize>,
    /// Diagnostic message.
    pub message: String,
}

impl std::fmt::Display for ConfigDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(
                formatter,
                "{}:{line}: {}",
                self.path.display(),
                self.message
            ),
            None => write!(formatter, "{}: {}", self.path.display(), self.message),
        }
    }
}

/// Loaded global configuration and any non-fatal diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    /// Effective settings after section fallback.
    pub config: Config,
    /// Warnings produced while loading settings.
    pub diagnostics: Vec<ConfigDiagnostic>,
}

/// Returns the platform's global Kiln config path.
#[must_use]
pub fn global_config_path() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("kiln/config.toml"))
    } else if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(config_home).join("kiln/config.toml"))
    } else {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".config/kiln/config.toml"))
    }
}

/// Loads configuration from the user's global config path.
pub fn load_global() -> LoadedConfig {
    match global_config_path() {
        Some(path) => load_file(path),
        None => LoadedConfig {
            config: Config::default(),
            diagnostics: Vec::new(),
        },
    }
}

/// Loads a global-format config document from a path.
pub fn load_file(path: impl AsRef<Path>) -> LoadedConfig {
    let path = path.as_ref().to_path_buf();
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return LoadedConfig {
                config: Config::default(),
                diagnostics: Vec::new(),
            }
        }
        Err(error) => {
            return LoadedConfig {
                config: Config::default(),
                diagnostics: vec![ConfigDiagnostic {
                    path,
                    line: None,
                    message: format!("could not read config; using defaults ({error})"),
                }],
            }
        }
    };
    load_str(path, &source)
}

/// Loads a TOML config string and falls back independently for invalid sections.
#[must_use]
pub fn load_str(path: impl Into<PathBuf>, source: &str) -> LoadedConfig {
    let path = path.into();
    if source.trim().is_empty() {
        return LoadedConfig {
            config: Config::default(),
            diagnostics: Vec::new(),
        };
    }

    let parsed = match source.parse::<toml::Table>() {
        Ok(parsed) => parsed,
        Err(error) => {
            return LoadedConfig {
                config: Config::default(),
                diagnostics: vec![ConfigDiagnostic {
                    path,
                    line: error.span().map(|span| line_number(source, span.start)),
                    message: format!("invalid TOML; using defaults ({error})"),
                }],
            }
        }
    };

    let known_sections = BTreeSet::from(["editor", "keybindings", "theme"]);
    let mut diagnostics = Vec::new();
    for key in parsed.keys() {
        if !known_sections.contains(key.as_str()) {
            diagnostics.push(ConfigDiagnostic {
                path: path.clone(),
                line: find_section_line(source, key),
                message: format!("unknown section `{key}`; ignoring it"),
            });
        }
    }

    let editor =
        deserialize_section::<EditorConfig>(&parsed, "editor", &path, source, &mut diagnostics);
    let keybindings = deserialize_section::<KeybindingsConfig>(
        &parsed,
        "keybindings",
        &path,
        source,
        &mut diagnostics,
    );
    let theme =
        deserialize_section::<ThemeConfig>(&parsed, "theme", &path, source, &mut diagnostics);

    diagnostics.sort_by_key(|diagnostic| diagnostic.line.unwrap_or(usize::MAX));
    LoadedConfig {
        config: Config {
            editor,
            keybindings,
            theme,
        },
        diagnostics,
    }
}

fn deserialize_section<T>(
    parsed: &toml::Table,
    name: &str,
    path: &Path,
    source: &str,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    let Some(value) = parsed.get(name) else {
        return T::default();
    };
    match value.clone().try_into::<T>() {
        Ok(section) => section,
        Err(error) => {
            let message = error.to_string();
            diagnostics.push(ConfigDiagnostic {
                path: path.to_path_buf(),
                line: find_error_line(source, &message).or_else(|| find_section_line(source, name)),
                message: format!("invalid `{name}` section; using defaults ({message})"),
            });
            T::default()
        }
    }
}

/// Returns a JSON Schema document for [`Config`].
pub fn config_schema_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&schema_for!(Config))
}

/// Parses a custom theme file and reports errors with path and line information.
pub fn load_theme_file(path: impl AsRef<Path>) -> Result<ThemeFile, ConfigDiagnostic> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|error| ConfigDiagnostic {
        path: path.to_path_buf(),
        line: None,
        message: format!("could not read theme file ({error})"),
    })?;
    toml::from_str(&source).map_err(|error: toml::de::Error| ConfigDiagnostic {
        path: path.to_path_buf(),
        line: error.span().map(|span| line_number(&source, span.start)),
        message: format!("invalid theme file ({error})"),
    })
}

fn line_number(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn find_section_line(source: &str, section: &str) -> Option<usize> {
    let header = format!("[{section}]");
    source
        .lines()
        .position(|line| line.trim() == header)
        .map(|line| line + 1)
}

fn find_error_line(source: &str, message: &str) -> Option<usize> {
    for marker in ['`', '\''] {
        let mut parts = message.split(marker);
        let _ = parts.next();
        if let Some(field) = parts.next() {
            if !field.is_empty() {
                if let Some(line) = source.lines().position(|line| line.contains(field)) {
                    return Some(line + 1);
                }
            }
        }
    }
    None
}

/// Returns the name of this crate.
#[must_use]
pub fn name() -> &'static str {
    "kiln-config"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path() -> PathBuf {
        PathBuf::from("/test/config.toml")
    }

    #[test]
    fn test_name() {
        assert_eq!(name(), "kiln-config");
    }

    #[test]
    fn defaults_and_empty_config_are_valid() {
        assert_eq!(Config::default().editor.tab_width, 4);
        let loaded = load_str(fixture_path(), "  # empty config\n");
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn valid_sections_load_over_defaults() {
        let loaded = load_str(
            fixture_path(),
            "[editor]\ntab_width = 2\nscrolloff = 8\nmouse = false\n\n[theme]\nfile = \"colors.toml\"\n",
        );
        assert_eq!(loaded.config.editor.tab_width, 2);
        assert_eq!(loaded.config.editor.scrolloff, 8);
        assert!(!loaded.config.editor.mouse);
        assert_eq!(loaded.config.theme.file.as_deref(), Some("colors.toml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn invalid_section_falls_back_without_discarding_valid_siblings() {
        let loaded = load_str(
            fixture_path(),
            "[editor]\ntab_width = \"wide\"\n\n[theme]\nfile = \"custom.toml\"\n",
        );
        assert_eq!(loaded.config.editor, EditorConfig::default());
        assert_eq!(loaded.config.theme.file.as_deref(), Some("custom.toml"));
        assert_eq!(loaded.diagnostics.len(), 1);
        assert!(loaded.diagnostics[0].line.is_some());
        assert!(loaded.diagnostics[0].message.contains("editor"));
    }

    #[test]
    fn unknown_keys_and_sections_warn_with_source_line() {
        let loaded = load_str(
            fixture_path(),
            "[editor]\ntab_width = 2\nunknown = true\n\n[other]\nvalue = 1\n",
        );
        assert_eq!(loaded.config.editor, EditorConfig::default());
        assert_eq!(loaded.diagnostics.len(), 2);
        assert!(loaded
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.line.is_some()));
    }

    #[test]
    fn syntax_error_falls_back_to_all_defaults_with_line() {
        let loaded = load_str(fixture_path(), "[editor\ntab_width = 2\n");
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.diagnostics.len(), 1);
        assert_eq!(loaded.diagnostics[0].line, Some(1));
    }

    #[test]
    fn schema_contains_config_sections() {
        let schema = config_schema_json().expect("schema serializes");
        let schema: serde_json::Value =
            serde_json::from_str(&schema).expect("schema is valid JSON");
        assert!(schema["properties"]["editor"].is_object());
        assert!(schema["properties"]["keybindings"].is_object());
        assert!(schema["properties"]["theme"].is_object());
    }

    #[test]
    fn theme_file_parses_typed_scopes() {
        let dir = std::env::temp_dir().join(format!("kiln-theme-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("test dir created");
        let path = dir.join("theme.toml");
        fs::write(
            &path,
            "name = \"Test\"\n[scopes.\"ui.text\"]\nforeground = [1, 2, 3]\nmodifiers = [\"bold\"]\n",
        )
        .expect("theme fixture written");
        let theme = load_theme_file(&path).expect("theme parsed");
        assert_eq!(theme.name, "Test");
        assert_eq!(theme.scopes["ui.text"].modifiers, vec!["bold"]);
        let _ = fs::remove_dir_all(dir);
    }
}
