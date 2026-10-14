use kiln_input::{InputEngine, InputResult, KeyEvent, Mode};

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
