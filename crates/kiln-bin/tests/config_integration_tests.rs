use std::collections::BTreeMap;

use kiln_bin::{apply_keybinding_overrides, apply_theme_file, EditorSession};
use kiln_config::{load_str, KeybindingsConfig, ThemeColor, ThemeFile, ThemeStyle};
use kiln_core::CommandRegistry;
use kiln_input::{InputEngine, KeyEvent, KeymapAction, Mode};
use kiln_ui::{Color, Theme};

#[test]
fn config_applies_valid_keybinding_override() {
    let loaded = load_str(
        "/virtual/config.toml",
        "[keybindings.modes.normal]\nu = \"redo\"\n",
    );
    let mut session = EditorSession::empty(None);
    let warnings = session.apply_config(loaded);
    assert!(warnings.is_empty());
    assert!(matches!(
        session.keybinding(Mode::Normal, KeyEvent::char('u')),
        Some(KeymapAction::Command { name: "redo", .. })
    ));
}

#[test]
fn multikey_keybinding_override_is_installed() {
    let mut input = InputEngine::new();
    let registry = CommandRegistry::with_builtins();
    let config = KeybindingsConfig {
        modes: BTreeMap::from([(
            "normal".to_string(),
            BTreeMap::from([("g d".to_string(), "undo".to_string())]),
        )]),
    };
    assert!(apply_keybinding_overrides(&mut input, &config, &registry).is_empty());
    assert!(matches!(
        input
            .keymaps()
            .get_trie(Mode::Normal)
            .lookup(&[KeyEvent::char('g'), KeyEvent::char('d')]),
        kiln_input::KeymapLookup::Exact(KeymapAction::Command { name: "undo", .. })
    ));
}

#[test]
fn invalid_keybinding_override_is_warned_and_skipped() {
    let mut input = InputEngine::new();
    let registry = CommandRegistry::with_builtins();
    let config = KeybindingsConfig {
        modes: BTreeMap::from([(
            "normal".to_string(),
            BTreeMap::from([
                ("<NotAKey>".to_string(), "undo".to_string()),
                ("z".to_string(), "unknown_command".to_string()),
            ]),
        )]),
    };
    let warnings = apply_keybinding_overrides(&mut input, &config, &registry);
    assert_eq!(warnings.len(), 2);
    assert!(matches!(
        input
            .keymaps()
            .get_trie(Mode::Normal)
            .lookup(&[KeyEvent::char('z')]),
        kiln_input::KeymapLookup::None
    ));
}

#[test]
fn editor_settings_are_available_to_ui_adapter() {
    let loaded = load_str("/virtual/config.toml", "[editor]\nscrolloff = 9\n");
    let mut session = EditorSession::empty(None);
    assert!(session.apply_config(loaded).is_empty());
    assert_eq!(session.config().editor.scrolloff, 9);
    assert_eq!(session.mode(), Mode::Normal);
}

#[test]
fn custom_theme_scopes_override_matching_dark_theme_styles() {
    let mut theme = Theme::dark();
    let file = ThemeFile {
        name: "Custom".to_string(),
        scopes: BTreeMap::from([(
            "ui.text".to_string(),
            ThemeStyle {
                foreground: Some(ThemeColor::Rgb([1, 2, 3])),
                background: None,
                modifiers: vec!["bold".to_string()],
            },
        )]),
    };
    apply_theme_file(&mut theme, &file);
    assert_eq!(theme.name, "Custom");
    assert_eq!(theme.ui_text.fg, Some(Color::Rgb(1, 2, 3)));
}
