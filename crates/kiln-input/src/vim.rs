//! Vim grammar state machine.
//!
//! Parses sequential key events into structured `VimAction` records:
//! `[register][count]operator[target]` or standalone motions.

use crate::action::{Motion, Operator, Target, TextObject, VimAction};
use crate::key::{KeyCode, KeyEvent};

/// Internal parsing state of the Vim grammar parser.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum ParseState {
    #[default]
    Initial,
    WaitingForRegister,
    OperatorPending(Operator),
    WaitingForFindChar {
        backward: bool,
        till: bool,
    },
    WaitingForTextObjectInner,
    WaitingForTextObjectAround,
    WaitingForGChar,
}

/// Output of feeding a key event into the `VimStateMachine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepResult {
    /// A complete `VimAction` was recognized.
    Complete(VimAction),
    /// More keys are needed to complete an action.
    Pending,
    /// An unrecognized or invalid sequence was encountered; state was reset.
    Invalid,
    /// Cancelled explicitly (e.g. `<Esc>`).
    Cancelled,
}

/// State machine for parsing Vim grammar actions.
#[derive(Default)]
pub struct VimStateMachine {
    register: Option<char>,
    count: Option<usize>,
    operator: Option<Operator>,
    state: ParseState,
}

impl VimStateMachine {
    /// Creates a new Vim grammar state machine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the parser state back to initial.
    pub fn reset(&mut self) {
        self.register = None;
        self.count = None;
        self.operator = None;
        self.state = ParseState::Initial;
    }

    /// Feeds a key event into the state machine.
    pub fn step(&mut self, event: KeyEvent) -> StepResult {
        // Esc cancels any pending command sequence and resets
        if event.code == KeyCode::Esc {
            self.reset();
            return StepResult::Cancelled;
        }

        // Only handle unmodified characters or specific navigation keys
        match self.state {
            ParseState::Initial => self.step_initial(event),
            ParseState::WaitingForRegister => self.step_waiting_for_register(event),
            ParseState::OperatorPending(op) => self.step_operator_pending(op, event),
            ParseState::WaitingForFindChar { backward, till } => {
                self.step_waiting_for_find(backward, till, event)
            }
            ParseState::WaitingForTextObjectInner => self.step_waiting_for_inner_object(event),
            ParseState::WaitingForTextObjectAround => self.step_waiting_for_around_object(event),
            ParseState::WaitingForGChar => self.step_waiting_for_g(event),
        }
    }

    fn step_initial(&mut self, event: KeyEvent) -> StepResult {
        if !event.modifiers.is_empty() {
            return StepResult::Invalid;
        }

        match event.code {
            // Register prefix: '"'
            KeyCode::Char('"') => {
                self.state = ParseState::WaitingForRegister;
                StepResult::Pending
            }
            // Count digits (1..=9 starts count; 0 in initial state is Motion::LineStart)
            KeyCode::Char(d @ '1'..='9') => {
                let digit = d.to_digit(10).unwrap_or(0) as usize;
                let cur = self.count.unwrap_or(0);
                self.count = Some(cur * 10 + digit);
                StepResult::Pending
            }
            KeyCode::Char('0') => {
                if let Some(cur) = self.count {
                    self.count = Some(cur * 10);
                    StepResult::Pending
                } else {
                    // LineStart motion
                    self.finish_motion(Motion::LineStart)
                }
            }
            // Operators: 'd', 'c', 'y'
            KeyCode::Char('d') => {
                self.operator = Some(Operator::Delete);
                self.state = ParseState::OperatorPending(Operator::Delete);
                StepResult::Pending
            }
            KeyCode::Char('c') => {
                self.operator = Some(Operator::Change);
                self.state = ParseState::OperatorPending(Operator::Change);
                StepResult::Pending
            }
            KeyCode::Char('y') => {
                self.operator = Some(Operator::Yank);
                self.state = ParseState::OperatorPending(Operator::Yank);
                StepResult::Pending
            }
            // 'g' prefix (e.g. 'gg')
            KeyCode::Char('g') => {
                self.state = ParseState::WaitingForGChar;
                StepResult::Pending
            }
            // Motions
            KeyCode::Char('h') | KeyCode::Left => self.finish_motion(Motion::Left),
            KeyCode::Char('l') | KeyCode::Right => self.finish_motion(Motion::Right),
            KeyCode::Char('k') | KeyCode::Up => self.finish_motion(Motion::Up),
            KeyCode::Char('j') | KeyCode::Down => self.finish_motion(Motion::Down),
            KeyCode::Char('w') => self.finish_motion(Motion::WordForward),
            KeyCode::Char('W') => self.finish_motion(Motion::WordForwardBig),
            KeyCode::Char('b') => self.finish_motion(Motion::WordBackward),
            KeyCode::Char('B') => self.finish_motion(Motion::WordBackwardBig),
            KeyCode::Char('e') => self.finish_motion(Motion::WordEnd),
            KeyCode::Char('E') => self.finish_motion(Motion::WordEndBig),
            KeyCode::Char('^') => self.finish_motion(Motion::FirstNonBlank),
            KeyCode::Char('$') => self.finish_motion(Motion::LineEnd),
            KeyCode::Char('G') => self.finish_motion(Motion::BufferEnd),
            KeyCode::Char('%') => self.finish_motion(Motion::MatchPair),
            KeyCode::Char(';') => self.finish_motion(Motion::RepeatFind),
            KeyCode::Char(',') => self.finish_motion(Motion::RepeatFindReverse),
            KeyCode::Char('f') => {
                self.state = ParseState::WaitingForFindChar {
                    backward: false,
                    till: false,
                };
                StepResult::Pending
            }
            KeyCode::Char('F') => {
                self.state = ParseState::WaitingForFindChar {
                    backward: true,
                    till: false,
                };
                StepResult::Pending
            }
            KeyCode::Char('t') => {
                self.state = ParseState::WaitingForFindChar {
                    backward: false,
                    till: true,
                };
                StepResult::Pending
            }
            KeyCode::Char('T') => {
                self.state = ParseState::WaitingForFindChar {
                    backward: true,
                    till: true,
                };
                StepResult::Pending
            }
            _ => {
                self.reset();
                StepResult::Invalid
            }
        }
    }

    fn step_waiting_for_register(&mut self, event: KeyEvent) -> StepResult {
        if let KeyCode::Char(c) = event.code {
            if c.is_ascii_alphanumeric() || c == '"' || c == '_' {
                self.register = Some(c);
                self.state = ParseState::Initial;
                return StepResult::Pending;
            }
        }
        self.reset();
        StepResult::Invalid
    }

    fn step_operator_pending(&mut self, op: Operator, event: KeyEvent) -> StepResult {
        // Repeated operator = operate on entire line ('dd', 'cc', 'yy')
        match (op, event.code) {
            (Operator::Delete, KeyCode::Char('d'))
            | (Operator::Change, KeyCode::Char('c'))
            | (Operator::Yank, KeyCode::Char('y')) => {
                return self.finish_action(Some(Target::Motion(Motion::CurrentLine)));
            }
            _ => {}
        }

        // Secondary count (e.g. 'd2w')
        if let KeyCode::Char(d @ '1'..='9') = event.code {
            let digit = d.to_digit(10).unwrap_or(0) as usize;
            let existing = self.count.unwrap_or(1);
            self.count = Some(existing * digit);
            return StepResult::Pending;
        }

        // Text object prefixes: 'i' (inner), 'a' (around)
        if event.code == KeyCode::Char('i') {
            self.state = ParseState::WaitingForTextObjectInner;
            return StepResult::Pending;
        }
        if event.code == KeyCode::Char('a') {
            self.state = ParseState::WaitingForTextObjectAround;
            return StepResult::Pending;
        }

        // 'g' prefix for motions after operator (e.g. 'dgg')
        if event.code == KeyCode::Char('g') {
            self.state = ParseState::WaitingForGChar;
            return StepResult::Pending;
        }

        // Regular motions after operator
        match event.code {
            KeyCode::Char('h') | KeyCode::Left => {
                self.finish_action(Some(Target::Motion(Motion::Left)))
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.finish_action(Some(Target::Motion(Motion::Right)))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.finish_action(Some(Target::Motion(Motion::Up)))
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.finish_action(Some(Target::Motion(Motion::Down)))
            }
            KeyCode::Char('w') => self.finish_action(Some(Target::Motion(Motion::WordForward))),
            KeyCode::Char('W') => self.finish_action(Some(Target::Motion(Motion::WordForwardBig))),
            KeyCode::Char('b') => self.finish_action(Some(Target::Motion(Motion::WordBackward))),
            KeyCode::Char('B') => self.finish_action(Some(Target::Motion(Motion::WordBackwardBig))),
            KeyCode::Char('e') => self.finish_action(Some(Target::Motion(Motion::WordEnd))),
            KeyCode::Char('E') => self.finish_action(Some(Target::Motion(Motion::WordEndBig))),
            KeyCode::Char('0') => self.finish_action(Some(Target::Motion(Motion::LineStart))),
            KeyCode::Char('^') => self.finish_action(Some(Target::Motion(Motion::FirstNonBlank))),
            KeyCode::Char('$') => self.finish_action(Some(Target::Motion(Motion::LineEnd))),
            KeyCode::Char('G') => self.finish_action(Some(Target::Motion(Motion::BufferEnd))),
            KeyCode::Char('%') => self.finish_action(Some(Target::Motion(Motion::MatchPair))),
            KeyCode::Char('f') => {
                self.state = ParseState::WaitingForFindChar {
                    backward: false,
                    till: false,
                };
                StepResult::Pending
            }
            KeyCode::Char('F') => {
                self.state = ParseState::WaitingForFindChar {
                    backward: true,
                    till: false,
                };
                StepResult::Pending
            }
            KeyCode::Char('t') => {
                self.state = ParseState::WaitingForFindChar {
                    backward: false,
                    till: true,
                };
                StepResult::Pending
            }
            KeyCode::Char('T') => {
                self.state = ParseState::WaitingForFindChar {
                    backward: true,
                    till: true,
                };
                StepResult::Pending
            }
            _ => {
                self.reset();
                StepResult::Invalid
            }
        }
    }

    fn step_waiting_for_find(&mut self, backward: bool, till: bool, event: KeyEvent) -> StepResult {
        if let KeyCode::Char(c) = event.code {
            let motion = match (backward, till) {
                (false, false) => Motion::FindForward(c),
                (true, false) => Motion::FindBackward(c),
                (false, true) => Motion::TillForward(c),
                (true, true) => Motion::TillBackward(c),
            };
            self.finish_action(Some(Target::Motion(motion)))
        } else {
            self.reset();
            StepResult::Invalid
        }
    }

    fn step_waiting_for_inner_object(&mut self, event: KeyEvent) -> StepResult {
        match event.code {
            KeyCode::Char('w') => {
                self.finish_action(Some(Target::TextObject(TextObject::InnerWord)))
            }
            KeyCode::Char(q @ ('"' | '\'' | '`')) => {
                self.finish_action(Some(Target::TextObject(TextObject::InnerQuote(q))))
            }
            KeyCode::Char('(' | ')' | 'b') => {
                self.finish_action(Some(Target::TextObject(TextObject::InnerPair('('))))
            }
            KeyCode::Char('[' | ']') => {
                self.finish_action(Some(Target::TextObject(TextObject::InnerPair('['))))
            }
            KeyCode::Char('{' | '}' | 'B') => {
                self.finish_action(Some(Target::TextObject(TextObject::InnerPair('{'))))
            }
            KeyCode::Char('<' | '>') => {
                self.finish_action(Some(Target::TextObject(TextObject::InnerPair('<'))))
            }
            _ => {
                self.reset();
                StepResult::Invalid
            }
        }
    }

    fn step_waiting_for_around_object(&mut self, event: KeyEvent) -> StepResult {
        match event.code {
            KeyCode::Char('w') => {
                self.finish_action(Some(Target::TextObject(TextObject::AroundWord)))
            }
            KeyCode::Char(q @ ('"' | '\'' | '`')) => {
                self.finish_action(Some(Target::TextObject(TextObject::AroundQuote(q))))
            }
            KeyCode::Char('(' | ')' | 'b') => {
                self.finish_action(Some(Target::TextObject(TextObject::AroundPair('('))))
            }
            KeyCode::Char('[' | ']') => {
                self.finish_action(Some(Target::TextObject(TextObject::AroundPair('['))))
            }
            KeyCode::Char('{' | '}' | 'B') => {
                self.finish_action(Some(Target::TextObject(TextObject::AroundPair('{'))))
            }
            KeyCode::Char('<' | '>') => {
                self.finish_action(Some(Target::TextObject(TextObject::AroundPair('<'))))
            }
            _ => {
                self.reset();
                StepResult::Invalid
            }
        }
    }

    fn step_waiting_for_g(&mut self, event: KeyEvent) -> StepResult {
        if event.code == KeyCode::Char('g') {
            self.finish_action(Some(Target::Motion(Motion::BufferStart)))
        } else {
            self.reset();
            StepResult::Invalid
        }
    }

    fn finish_motion(&mut self, motion: Motion) -> StepResult {
        self.finish_action(Some(Target::Motion(motion)))
    }

    fn finish_action(&mut self, target: Option<Target>) -> StepResult {
        let action = VimAction {
            register: self.register,
            count: self.count,
            operator: self.operator,
            target,
        };
        self.reset();
        StepResult::Complete(action)
    }
}
