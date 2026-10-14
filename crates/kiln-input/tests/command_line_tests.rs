use kiln_input::{InputEngine, InputResult, KeyCode, KeyEvent, KeyModifiers, Mode};

#[test]
fn command_line_collects_backspaces_and_submits_on_enter() {
    let mut input = InputEngine::new();
    input.handle_key(KeyEvent::char(':'));
    assert_eq!(input.mode(), Mode::Command);
    input.handle_key(KeyEvent::char('w'));
    input.handle_key(KeyEvent::char('q'));
    input.handle_key(KeyEvent::new(kiln_input::KeyCode::Backspace));
    assert_eq!(input.command_line(), "w");

    assert_eq!(
        input.handle_key(KeyEvent::new(kiln_input::KeyCode::Enter)),
        InputResult::SubmitCommand("w".to_string())
    );
    assert_eq!(input.mode(), Mode::Normal);
    assert_eq!(input.command_line(), "");
}

#[test]
fn escape_cancels_command_line() {
    let mut input = InputEngine::new();
    input.handle_key(KeyEvent::char(':'));
    input.handle_key(KeyEvent::char('q'));
    input.handle_key(KeyEvent::new(kiln_input::KeyCode::Esc));
    assert_eq!(input.mode(), Mode::Normal);
    assert_eq!(input.command_line(), "");
}

#[test]
fn insert_mode_accepts_shift_uppercase() {
    let mut engine = InputEngine::new();
    engine.set_mode(Mode::Insert);
    // Simulate typing 'A' with Shift (as crossterm delivers it)
    let result = engine.handle_key(KeyEvent {
        code: KeyCode::Char('A'),
        modifiers: KeyModifiers::SHIFT,
    });
    assert!(matches!(result, InputResult::InsertChar(c) if c == 'A'));
}

#[test]
fn insert_mode_accepts_lowercase_without_shift() {
    let mut engine = InputEngine::new();
    engine.set_mode(Mode::Insert);
    let result = engine.handle_key(KeyEvent::char('a'));
    assert!(matches!(result, InputResult::InsertChar(c) if c == 'a'));
}

#[test]
fn command_mode_accepts_shift_letters() {
    let mut engine = InputEngine::new();
    engine.set_mode(Mode::Command);
    // Simulate pressing Shift+e (delivers 'E')
    let result = engine.handle_key(KeyEvent {
        code: KeyCode::Char('E'),
        modifiers: KeyModifiers::SHIFT,
    });
    assert!(matches!(result, InputResult::CommandLineChanged(s) if s == "E"));
}
