use kiln_input::*;

fn feed_keys(machine: &mut VimStateMachine, s: &str) -> StepResult {
    let mut last = StepResult::Pending;
    for c in s.chars() {
        last = machine.step(KeyEvent::char(c));
    }
    last
}

#[test]
fn test_vim_grammar_simple_motions() {
    let mut machine = VimStateMachine::new();

    // 'w' -> Motion::WordForward
    assert_eq!(
        machine.step(KeyEvent::char('w')),
        StepResult::Complete(VimAction {
            register: None,
            count: None,
            operator: None,
            target: Some(Target::Motion(Motion::WordForward)),
        })
    );

    // '3j' -> Motion::Down with count 3
    assert_eq!(machine.step(KeyEvent::char('3')), StepResult::Pending);
    assert_eq!(
        machine.step(KeyEvent::char('j')),
        StepResult::Complete(VimAction {
            register: None,
            count: Some(3),
            operator: None,
            target: Some(Target::Motion(Motion::Down)),
        })
    );

    // 'gg' -> Motion::BufferStart
    assert_eq!(machine.step(KeyEvent::char('g')), StepResult::Pending);
    assert_eq!(
        machine.step(KeyEvent::char('g')),
        StepResult::Complete(VimAction {
            register: None,
            count: None,
            operator: None,
            target: Some(Target::Motion(Motion::BufferStart)),
        })
    );
}

#[test]
fn test_vim_grammar_operator_motion_sequences() {
    let mut machine = VimStateMachine::new();

    // 1. "3dw" -> count 3, operator Delete, target WordForward
    assert_eq!(
        feed_keys(&mut machine, "3dw"),
        StepResult::Complete(VimAction {
            register: None,
            count: Some(3),
            operator: Some(Operator::Delete),
            target: Some(Target::Motion(Motion::WordForward)),
        })
    );

    // 2. "\"ayy" -> register 'a', operator Yank, target CurrentLine
    let mut machine = VimStateMachine::new();
    assert_eq!(machine.step(KeyEvent::char('"')), StepResult::Pending);
    assert_eq!(machine.step(KeyEvent::char('a')), StepResult::Pending);
    assert_eq!(machine.step(KeyEvent::char('y')), StepResult::Pending);
    assert_eq!(
        machine.step(KeyEvent::char('y')),
        StepResult::Complete(VimAction {
            register: Some('a'),
            count: None,
            operator: Some(Operator::Yank),
            target: Some(Target::Motion(Motion::CurrentLine)),
        })
    );

    // 3. "d2j" -> operator Delete, count 2, target Down
    let mut machine = VimStateMachine::new();
    assert_eq!(
        feed_keys(&mut machine, "d2j"),
        StepResult::Complete(VimAction {
            register: None,
            count: Some(2),
            operator: Some(Operator::Delete),
            target: Some(Target::Motion(Motion::Down)),
        })
    );

    // 4. "ciw" -> operator Change, target InnerWord
    let mut machine = VimStateMachine::new();
    assert_eq!(
        feed_keys(&mut machine, "ciw"),
        StepResult::Complete(VimAction {
            register: None,
            count: None,
            operator: Some(Operator::Change),
            target: Some(Target::TextObject(TextObject::InnerWord)),
        })
    );

    // 5. "da\"" -> operator Delete, target AroundQuote('"')
    let mut machine = VimStateMachine::new();
    assert_eq!(
        feed_keys(&mut machine, "da\""),
        StepResult::Complete(VimAction {
            register: None,
            count: None,
            operator: Some(Operator::Delete),
            target: Some(Target::TextObject(TextObject::AroundQuote('"'))),
        })
    );

    // 6. "f;" -> Motion::FindForward(';')
    let mut machine = VimStateMachine::new();
    assert_eq!(
        feed_keys(&mut machine, "f;"),
        StepResult::Complete(VimAction {
            register: None,
            count: None,
            operator: None,
            target: Some(Target::Motion(Motion::FindForward(';'))),
        })
    );
}

#[test]
fn test_vim_grammar_cancel_and_invalid_reset() {
    let mut machine = VimStateMachine::new();

    // Partial operator sequence: 'd'
    assert_eq!(machine.step(KeyEvent::char('d')), StepResult::Pending);

    // Pressing <Esc> cancels and resets cleanly
    assert_eq!(
        machine.step(KeyEvent::new(KeyCode::Esc)),
        StepResult::Cancelled
    );

    // After cancellation, a new motion is accepted cleanly without pending operator
    assert_eq!(
        machine.step(KeyEvent::char('l')),
        StepResult::Complete(VimAction {
            register: None,
            count: None,
            operator: None,
            target: Some(Target::Motion(Motion::Right)),
        })
    );

    // Invalid sequence: "d" followed by unknown character key resets state
    assert_eq!(machine.step(KeyEvent::char('d')), StepResult::Pending);
    assert_eq!(machine.step(KeyEvent::char('&')), StepResult::Invalid);

    // State is reset after invalid sequence
    assert_eq!(
        machine.step(KeyEvent::char('j')),
        StepResult::Complete(VimAction {
            register: None,
            count: None,
            operator: None,
            target: Some(Target::Motion(Motion::Down)),
        })
    );
}
