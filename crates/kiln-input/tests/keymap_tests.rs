use kiln_core::CommandArg;
use kiln_input::*;

#[test]
fn test_key_event_parsing_and_display() {
    assert_eq!(
        KeyEvent::parse("<C-w>"),
        Some(KeyEvent {
            code: KeyCode::Char('w'),
            modifiers: KeyModifiers::CTRL,
        })
    );

    assert_eq!(KeyEvent::parse("<Esc>"), Some(KeyEvent::new(KeyCode::Esc)));

    assert_eq!(
        KeyEvent::parse("<A-x>"),
        Some(KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::ALT,
        })
    );

    assert_eq!(
        KeyEvent::parse("Enter"),
        Some(KeyEvent::new(KeyCode::Enter))
    );
    assert_eq!(KeyEvent::parse("j"), Some(KeyEvent::char('j')));
    assert_eq!(KeyEvent::parse("<F1>"), Some(KeyEvent::new(KeyCode::F(1))));

    assert_eq!(format!("{}", KeyEvent::char('k')), "k");
    assert_eq!(format!("{}", KeyEvent::ctrl(KeyCode::Char('c'))), "<C-c>");
    assert_eq!(format!("{}", KeyEvent::new(KeyCode::Esc)), "<Esc>");
}

#[test]
fn test_keymap_trie_exact_and_partial_matches() {
    let mut trie = KeymapTrie::new();
    let k_g = KeyEvent::char('g');
    let k_d = KeyEvent::char('d');

    trie.insert(
        &[k_g, k_d],
        KeymapAction::Command {
            name: "go_to_definition",
            arg: CommandArg::None,
        },
    );

    // Single key 'g' is partial match
    assert_eq!(trie.lookup(&[k_g]), KeymapLookup::Partial);

    // Sequence 'g', 'd' is exact match
    match trie.lookup(&[k_g, k_d]) {
        KeymapLookup::Exact(KeymapAction::Command { name, .. }) => {
            assert_eq!(*name, "go_to_definition");
        }
        _ => panic!("Expected exact match"),
    }

    // Sequence 'g', 'x' is None
    assert_eq!(trie.lookup(&[k_g, KeyEvent::char('x')]), KeymapLookup::None);
}

#[test]
fn test_input_engine_mode_transitions_and_defaults() {
    let mut engine = InputEngine::new();
    assert_eq!(engine.mode(), Mode::Normal);

    // Press 'i' in Normal mode -> transitions to Insert mode
    let res = engine.handle_key(KeyEvent::char('i'));
    assert_eq!(res, InputResult::SwitchMode(Mode::Insert));
    assert_eq!(engine.mode(), Mode::Insert);

    // Press 'a' in Insert mode -> InsertChar('a')
    let res = engine.handle_key(KeyEvent::char('a'));
    assert_eq!(res, InputResult::InsertChar('a'));

    // Press '<Esc>' in Insert mode -> transitions back to Normal mode
    let res = engine.handle_key(KeyEvent::new(KeyCode::Esc));
    assert_eq!(res, InputResult::SwitchMode(Mode::Normal));
    assert_eq!(engine.mode(), Mode::Normal);

    // Press 'u' in Normal mode -> ExecuteCommand("undo")
    let res = engine.handle_key(KeyEvent::char('u'));
    assert_eq!(
        res,
        InputResult::ExecuteCommand {
            name: "undo",
            arg: CommandArg::None,
        }
    );
}

#[test]
fn test_input_engine_partial_timeout_handling() {
    let mut engine = InputEngine::new();
    let k_g = KeyEvent::char('g');

    // Register a 2-key binding in Normal mode
    engine.keymaps_mut().get_trie_mut(Mode::Normal).insert(
        &[k_g, k_g],
        KeymapAction::Command {
            name: "goto_first_line",
            arg: CommandArg::None,
        },
    );

    // Press first 'g' -> Pending
    let res = engine.handle_key(k_g);
    assert_eq!(res, InputResult::Pending);

    // Timeout occurs -> drops pending keys
    engine.handle_timeout();

    // Next key is evaluated fresh
    let res = engine.handle_key(KeyEvent::char('i'));
    assert_eq!(res, InputResult::SwitchMode(Mode::Insert));
}
