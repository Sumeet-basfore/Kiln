//! Table-driven tests for Vim motions, operators, text objects, and commands.

use kiln_core::*;
use kiln_text::Text;

/// Executes a Vim operator or motion on a test buffer and returns the resulting text and primary cursor position.
fn run_vim_action(
    initial_text: &str,
    initial_pos: usize,
    op: Option<char>, // 'd', 'c', 'y', or None (motion only)
    count: usize,
    motion_or_target: &str, // e.g. "w", "b", "e", "W", "B", "E", "0", "^", "$", "gg", "G", "f;", "F;", "t;", "T;", "%", "iw", "aw", "i\"", "a\"", "i(", "a(", "dd", "cc", "yy"
    reg: Option<char>,
    registers: &mut Registers,
) -> (String, usize) {
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from(initial_text));
    buffer.set_selections(Selections::point(initial_pos));

    let doc_len = buffer.text().len_chars();
    let head = buffer.selections().primary().head;

    // Check linewise operation like "dd", "cc", "yy"
    let is_linewise_op = matches!(motion_or_target, "dd" | "cc" | "yy" | "line");

    if is_linewise_op {
        let line_idx = buffer
            .text()
            .char_to_line_col(head)
            .map(|lc| lc.line)
            .unwrap_or(0);
        let num_lines = buffer.text().len_lines();
        let end_line = (line_idx + count).min(num_lines);

        let start_pos = buffer
            .text()
            .line_col_to_char(kiln_text::LineCol::new(line_idx, 0))
            .unwrap_or(0);
        let end_pos = if end_line < num_lines {
            buffer
                .text()
                .line_col_to_char(kiln_text::LineCol::new(end_line, 0))
                .unwrap_or(doc_len)
        } else {
            doc_len
        };

        let yanked = buffer.text().slice(start_pos..end_pos).unwrap().to_string();
        registers.set(reg, RegisterValue::linewise(yanked));

        match op {
            Some('d') | Some('c') => {
                let tx = Transaction::delete_range(doc_len, start_pos..end_pos).unwrap();
                buffer.apply(tx).unwrap();
                let new_len = buffer.text().len_chars();
                let new_head = start_pos.min(new_len);
                buffer.set_selections(Selections::point(new_head));
            }
            Some('y') | None => {}
            _ => panic!("unknown op"),
        }

        return (
            buffer.text().to_string(),
            buffer.selections().primary().head,
        );
    }

    // Resolve target range or motion endpoint
    let target_range = match motion_or_target {
        "h" => Range::new(head, MotionEvaluator::move_left(buffer.text(), head, count)),
        "l" => Range::new(
            head,
            MotionEvaluator::move_right(buffer.text(), head, count),
        ),
        "k" => Range::new(head, MotionEvaluator::move_up(buffer.text(), head, count)),
        "j" => Range::new(head, MotionEvaluator::move_down(buffer.text(), head, count)),
        "0" => Range::new(head, MotionEvaluator::line_start(buffer.text(), head)),
        "^" => Range::new(head, MotionEvaluator::first_non_blank(buffer.text(), head)),
        "$" => Range::new(head, MotionEvaluator::line_end(buffer.text(), head)),
        "gg" => Range::new(head, MotionEvaluator::buffer_start(buffer.text())),
        "G" => Range::new(head, MotionEvaluator::buffer_end(buffer.text())),
        "w" => Range::new(
            head,
            MotionEvaluator::word_forward(buffer.text(), head, count),
        ),
        "W" => Range::new(
            head,
            MotionEvaluator::words_forward(buffer.text(), head, count),
        ),
        "b" => Range::new(
            head,
            MotionEvaluator::word_backward(buffer.text(), head, count),
        ),
        "B" => Range::new(
            head,
            MotionEvaluator::words_backward(buffer.text(), head, count),
        ),
        "e" => Range::new(head, MotionEvaluator::word_end(buffer.text(), head, count)),
        "E" => Range::new(head, MotionEvaluator::words_end(buffer.text(), head, count)),
        "%" => Range::new(
            head,
            MotionEvaluator::match_pair(buffer.text(), head).unwrap_or(head),
        ),
        "iw" => TextObjectEvaluator::inner_word(buffer.text(), head).unwrap_or(Range::point(head)),
        "aw" => TextObjectEvaluator::around_word(buffer.text(), head).unwrap_or(Range::point(head)),
        "i\"" => TextObjectEvaluator::quote(buffer.text(), head, '"', false)
            .unwrap_or(Range::point(head)),
        "a\"" => {
            TextObjectEvaluator::quote(buffer.text(), head, '"', true).unwrap_or(Range::point(head))
        }
        "i(" => {
            TextObjectEvaluator::pair(buffer.text(), head, '(', false).unwrap_or(Range::point(head))
        }
        "a(" => {
            TextObjectEvaluator::pair(buffer.text(), head, '(', true).unwrap_or(Range::point(head))
        }
        "i{" => {
            TextObjectEvaluator::pair(buffer.text(), head, '{', false).unwrap_or(Range::point(head))
        }
        "a{" => {
            TextObjectEvaluator::pair(buffer.text(), head, '{', true).unwrap_or(Range::point(head))
        }
        find if find.starts_with('f') => {
            let ch = find.chars().nth(1).unwrap();
            let dest =
                MotionEvaluator::find_forward(buffer.text(), head, ch, count).unwrap_or(head);
            Range::new(head, dest)
        }
        find if find.starts_with('F') => {
            let ch = find.chars().nth(1).unwrap();
            let dest =
                MotionEvaluator::find_backward(buffer.text(), head, ch, count).unwrap_or(head);
            Range::new(head, dest)
        }
        till if till.starts_with('t') => {
            let ch = till.chars().nth(1).unwrap();
            let dest =
                MotionEvaluator::till_forward(buffer.text(), head, ch, count).unwrap_or(head);
            Range::new(head, dest)
        }
        till if till.starts_with('T') => {
            let ch = till.chars().nth(1).unwrap();
            let dest =
                MotionEvaluator::till_backward(buffer.text(), head, ch, count).unwrap_or(head);
            Range::new(head, dest)
        }
        _ => panic!("unsupported motion in test harness: {}", motion_or_target),
    };

    match op {
        None => {
            // Pure motion: move cursor to target endpoint
            buffer.set_selections(Selections::point(target_range.head));
        }
        Some('d') | Some('c') => {
            // Delete / Change operator: delete range from..to
            let from = target_range.from();
            let to = target_range.to();
            if from < to {
                let yanked = buffer.text().slice(from..to).unwrap().to_string();
                registers.set(reg, RegisterValue::character(yanked));

                let tx = Transaction::delete_range(doc_len, from..to).unwrap();
                buffer.apply(tx).unwrap();
                buffer.set_selections(Selections::point(from));
            }
        }
        Some('y') => {
            // Yank operator: copy range
            let from = target_range.from();
            let to = target_range.to();
            if from < to {
                let yanked = buffer.text().slice(from..to).unwrap().to_string();
                registers.set(reg, RegisterValue::character(yanked));
            }
        }
        _ => panic!("unknown op"),
    }

    (
        buffer.text().to_string(),
        buffer.selections().primary().head,
    )
}

#[test]
fn test_table_motions_character_and_line() {
    let cases = vec![
        // (initial_text, start_pos, count, motion, expected_pos)
        ("Hello World", 5, 2, "h", 3),
        ("Hello World", 0, 5, "h", 0), // clamp to line start
        ("Hello World", 5, 3, "l", 8),
        ("Hello World", 10, 5, "l", 11), // clamp to line end
        ("Line 1\nLine 2\nLine 3", 0, 1, "j", 7),
        ("Line 1\nLine 2\nLine 3", 14, 2, "k", 0),
        ("   Hello", 6, 1, "0", 0),
        ("   Hello", 1, 1, "^", 3),
        ("Hello World\nNext", 0, 1, "$", 11),
        ("First\nSecond\nThird", 15, 1, "gg", 0),
        ("First\nSecond\nThird", 0, 1, "G", 13),
    ];

    let mut registers = Registers::new();
    for (text, pos, count, motion, expected_pos) in cases {
        let (out_text, out_pos) =
            run_vim_action(text, pos, None, count, motion, None, &mut registers);
        assert_eq!(out_text, text);
        assert_eq!(
            out_pos, expected_pos,
            "motion '{}' failed on text '{}'",
            motion, text
        );
    }
}

#[test]
fn test_table_motions_words_and_find() {
    let cases = vec![
        // (initial_text, start_pos, count, motion, expected_pos)
        ("foo bar baz", 0, 1, "w", 4),
        ("foo bar baz", 0, 2, "w", 8),
        ("foo.bar baz", 0, 1, "w", 3), // punctuation is separate word
        ("foo.bar baz", 0, 1, "W", 8), // WORDS skips punctuation
        ("foo bar baz", 8, 1, "b", 4),
        ("foo.bar baz", 8, 1, "b", 4),
        ("foo.bar baz", 8, 1, "B", 0),
        ("foo bar", 0, 1, "e", 2),
        ("foo.bar", 0, 1, "E", 6),
        ("hello world", 0, 1, "fo", 4),
        ("hello world", 10, 1, "Fo", 7),
        ("hello world", 0, 1, "to", 3),
        ("hello world", 10, 1, "To", 8),
        ("fn (a + b)", 3, 1, "%", 9), // match pair
    ];

    let mut registers = Registers::new();
    for (text, pos, count, motion, expected_pos) in cases {
        let (out_text, out_pos) =
            run_vim_action(text, pos, None, count, motion, None, &mut registers);
        assert_eq!(out_text, text);
        assert_eq!(
            out_pos, expected_pos,
            "motion '{}' failed on text '{}'",
            motion, text
        );
    }
}

#[test]
fn test_table_operators_delete_change_yank() {
    let mut registers = Registers::new();

    // 1. "dw" on "hello world" -> "world"
    let (t, p) = run_vim_action("hello world", 0, Some('d'), 1, "w", None, &mut registers);
    assert_eq!(t, "world");
    assert_eq!(p, 0);
    assert_eq!(registers.get(None).unwrap().text, "hello ");

    // 2. "dd" on multi-line text
    let (t, p) = run_vim_action(
        "Line 1\nLine 2\nLine 3",
        0,
        Some('d'),
        1,
        "dd",
        None,
        &mut registers,
    );
    assert_eq!(t, "Line 2\nLine 3");
    assert_eq!(p, 0);
    assert!(registers.get(None).unwrap().is_linewise);
    assert_eq!(registers.get(None).unwrap().text, "Line 1\n");

    // 3. "cw" on "foo bar" -> deletes "foo ", ready to change
    let (t, p) = run_vim_action("foo bar", 0, Some('c'), 1, "w", None, &mut registers);
    assert_eq!(t, "bar");
    assert_eq!(p, 0);

    // 4. "yy" on "hello\nworld" -> text unchanged, register populated
    let (t, p) = run_vim_action("hello\nworld", 0, Some('y'), 1, "yy", None, &mut registers);
    assert_eq!(t, "hello\nworld");
    assert_eq!(p, 0);
    assert_eq!(registers.get(None).unwrap().text, "hello\n");
    assert!(registers.get(None).unwrap().is_linewise);
}

#[test]
fn test_table_text_objects() {
    let mut registers = Registers::new();

    // 1. "diw" on "hello (world)!" at 'o' in world (pos 9)
    let (t, p) = run_vim_action(
        "hello (world)!",
        9,
        Some('d'),
        1,
        "iw",
        None,
        &mut registers,
    );
    assert_eq!(t, "hello ()!");
    assert_eq!(p, 7);

    // 2. "di\"" on "let s = \"hello\";" at pos 10
    let (t, p) = run_vim_action(
        "let s = \"hello\";",
        10,
        Some('d'),
        1,
        "i\"",
        None,
        &mut registers,
    );
    assert_eq!(t, "let s = \"\";");
    assert_eq!(p, 9);

    // 3. "da\"" on "let s = \"hello\";" at pos 10
    let (t, p) = run_vim_action(
        "let s = \"hello\";",
        10,
        Some('d'),
        1,
        "a\"",
        None,
        &mut registers,
    );
    assert_eq!(t, "let s = ;");
    assert_eq!(p, 8);

    // 4. "di(" on "foo(bar + baz)" at pos 5
    let (t, p) = run_vim_action(
        "foo(bar + baz)",
        5,
        Some('d'),
        1,
        "i(",
        None,
        &mut registers,
    );
    assert_eq!(t, "foo()");
    assert_eq!(p, 4);
}

#[test]
fn test_commands_x_p_o_join_unicode() {
    let registry = CommandRegistry::with_builtins();
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Hello 🌍\nLine 2"));
    let mut event_bus = EventBus::new();
    let mut registers = Registers::new();

    // 1. 'x' at pos 0 -> deletes 'H'
    buffer.set_selections(Selections::point(0));
    {
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("vim_x", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "ello 🌍\nLine 2");
    assert_eq!(registers.get(None).unwrap().text, "H");

    // 2. 'p' (paste after) at pos 0 -> inserts 'H' at pos 1
    {
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("vim_paste_after", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "eHllo 🌍\nLine 2");

    // 3. 'J' (join lines)
    buffer.set_selections(Selections::point(0));
    {
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("vim_join_lines", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "eHllo 🌍 Line 2");

    // 4. 'o' (open below)
    {
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("vim_open_below", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "eHllo 🌍 Line 2\n");
    assert_eq!(buffer.selections().primary().head, 15);
}
