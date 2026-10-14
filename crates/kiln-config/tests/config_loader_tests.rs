use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use kiln_config::{config_schema_json, load_file, load_str, Config, EditorConfig, ThemeColor};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = std::env::temp_dir().join(format!("kiln-config-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path).expect("test dir created");
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_file_and_empty_file_produce_defaults() {
    let dir = TestDir::new();
    let missing = load_file(dir.0.join("missing.toml"));
    assert_eq!(missing.config, Config::default());
    assert!(missing.diagnostics.is_empty());

    let empty_path = dir.0.join("empty.toml");
    fs::write(&empty_path, "\n# comments only\n").expect("fixture write succeeds");
    let empty = load_file(empty_path);
    assert_eq!(empty.config, Config::default());
    assert!(empty.diagnostics.is_empty());
}

#[test]
fn valid_global_sections_override_defaults() {
    let loaded = load_str(
        "/virtual/config.toml",
        "[editor]\ntab_width = 2\nscrolloff = 7\nmouse = false\nline_numbers = false\n\n[keybindings.modes.normal]\nq = \"undo\"\n\n[theme]\nfile = \"theme.toml\"\n",
    );
    assert_eq!(loaded.config.editor.tab_width, 2);
    assert_eq!(loaded.config.editor.scrolloff, 7);
    assert!(!loaded.config.editor.mouse);
    assert!(!loaded.config.editor.line_numbers);
    assert_eq!(loaded.config.keybindings.modes["normal"]["q"], "undo");
    assert_eq!(loaded.config.theme.file.as_deref(), Some("theme.toml"));
    assert!(loaded.diagnostics.is_empty());
}

#[test]
fn invalid_and_unknown_section_values_fall_back_independently_with_lines() {
    let loaded = load_str(
        "/virtual/config.toml",
        "[editor]\ntab_width = \"large\"\n\n[theme]\nfile = \"custom.toml\"\n\n[other]\nvalue = 1\n",
    );
    assert_eq!(loaded.config.editor, EditorConfig::default());
    assert_eq!(loaded.config.theme.file.as_deref(), Some("custom.toml"));
    assert_eq!(loaded.diagnostics.len(), 2);
    assert_eq!(loaded.diagnostics[0].line, Some(2));
    assert_eq!(loaded.diagnostics[1].line, Some(7));
}

#[test]
fn syntax_errors_use_all_defaults_with_a_location() {
    let loaded = load_str("/virtual/config.toml", "[editor\ntab_width = 2\n");
    assert_eq!(loaded.config, Config::default());
    assert_eq!(loaded.diagnostics.len(), 1);
    assert_eq!(loaded.diagnostics[0].line, Some(1));
}

#[test]
fn schema_is_json_with_expected_top_level_sections() {
    let schema = config_schema_json().expect("schema serializes");
    let value: serde_json::Value = serde_json::from_str(&schema).expect("schema is JSON");
    assert!(value["properties"]["editor"].is_object());
    assert!(value["properties"]["keybindings"].is_object());
    assert!(value["properties"]["theme"].is_object());
}

#[test]
fn custom_theme_file_parses_named_and_rgb_colors() {
    let dir = TestDir::new();
    let path = dir.0.join("theme.toml");
    fs::write(
        &path,
        "name = \"Example\"\n[scopes.\"ui.text\"]\nforeground = [12, 34, 56]\nmodifiers = [\"bold\"]\n[scopes.\"ui.gutter\"]\nforeground = \"gray\"\n",
    )
    .expect("theme fixture written");
    let theme = kiln_config::load_theme_file(path).expect("theme parses");
    assert_eq!(theme.name, "Example");
    assert_eq!(
        theme.scopes["ui.text"].foreground,
        Some(ThemeColor::Rgb([12, 34, 56]))
    );
    assert_eq!(
        theme.scopes["ui.gutter"].foreground,
        Some(ThemeColor::Name("gray".into()))
    );
}

#[test]
fn malformed_theme_error_has_path_and_line() {
    let dir = TestDir::new();
    let path = dir.0.join("bad-theme.toml");
    fs::write(&path, "name = 1\n[scopes.ui.text]\nforeground = [1, 2]\n")
        .expect("theme fixture written");
    let diagnostic = kiln_config::load_theme_file(&path).expect_err("invalid theme rejected");
    assert_eq!(diagnostic.path, path);
    assert!(diagnostic.line.is_some());
}
