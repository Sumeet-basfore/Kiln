//! Snapshot tests of rendered frames for fixed editor states.

use kiln_core::{Buffer, BufferId, LineCol, Range, Selections, Text};
use kiln_ui::{
    BufferView, Gutter, LineNumberMode, StatusLine, StatusLineState, TestRenderer, Theme,
};

#[test]
fn test_snapshot_normal_mode_hybrid_gutter() {
    let text = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
    let buffer = Buffer::new(BufferId::from_raw(1), Text::from(text));

    let mut renderer = TestRenderer::new(40, 6);
    let mut view = BufferView {
        gutter: Gutter {
            mode: LineNumberMode::Hybrid,
            show_signs: true,
        },
        ..BufferView::default()
    };
    let theme = Theme::dark();

    view.render(&mut renderer, &buffer, 5, &theme);

    let status_state = StatusLineState {
        mode_name: "NORMAL",
        mode_style: theme.mode_normal,
        buffer_name: "main.rs",
        is_modified: false,
        is_readonly: false,
        line: 1,
        col: 1,
        percent: "Top",
        encoding: "UTF-8",
        line_ending: "LF",
    };
    StatusLine::render(&mut renderer, 5, &status_state, &theme);

    let snapshot = renderer.snapshot();
    let lines = renderer.lines();

    // Verify gutter hybrid numbers: line 0 is cursor (shows '1'), line 1 shows relative distance '1', line 2 shows '2'
    assert!(lines[0].contains(" 1 fn main() {"));
    assert!(lines[1].contains(" 1     println!(\"Hello, world!\");"));
    assert!(lines[2].contains(" 2 }"));
    // Past EOF shows tilde
    assert!(lines[4].starts_with("~"));
    // Statusline at row 5
    assert!(lines[5].contains("NORMAL"));
    assert!(lines[5].contains("main.rs"));

    // Cursor position should be at column 4 (gutter width), row 0
    assert_eq!(renderer.cursor(), Some((4, 0)));
    assert!(!snapshot.is_empty());
}

#[test]
fn test_snapshot_visual_selection() {
    let text = "hello beautiful world";
    let mut buffer = Buffer::new(BufferId::from_raw(1), Text::from(text));

    // Select "beautiful" (indices 6..15)
    buffer.set_selections(Selections::new(vec![Range::new(6, 15)], 0).unwrap());

    let mut renderer = TestRenderer::new(35, 3);
    let mut view = BufferView {
        gutter: Gutter {
            mode: LineNumberMode::None,
            show_signs: false,
        },
        ..BufferView::default()
    };
    let theme = Theme::dark();

    view.render(&mut renderer, &buffer, 2, &theme);

    // Verify cell styles for selected characters vs unselected
    // "hello " is unselected (cols 0..6)
    for c in 0..6 {
        assert_eq!(renderer.style_at(c, 0).unwrap().bg, theme.ui_text.bg);
    }
    // "beautiful" is selected (cols 6..15)
    for c in 6..15 {
        assert_eq!(renderer.style_at(c, 0).unwrap().bg, theme.ui_selection.bg);
    }
    // " world" is unselected (cols 15..21)
    for c in 15..21 {
        assert_eq!(renderer.style_at(c, 0).unwrap().bg, theme.ui_text.bg);
    }
}

#[test]
fn test_snapshot_scrolling_and_scrolloff() {
    // 20 lines
    let text: String = (0..20).map(|i| format!("Line {:02}\n", i)).collect();
    let mut buffer = Buffer::new(BufferId::from_raw(1), Text::from(text.as_str()));

    // Move cursor to line 12
    let char_pos = buffer.text().line_col_to_char(LineCol::new(12, 0)).unwrap();
    buffer.set_selections(Selections::point(char_pos));

    let mut renderer = TestRenderer::new(30, 8); // 7 text rows + 1 status line
    let mut view = BufferView {
        scrolloff_rows: 2,
        ..BufferView::default()
    };
    let theme = Theme::dark();

    view.render(&mut renderer, &buffer, 7, &theme);

    // Cursor is on line 12. With view height 7 and scrolloff 2,
    // the top visible line must scroll down so cursor is visible with scrolloff margin.
    assert!(view.scroll_row > 5);
    assert!(view.scroll_row <= 12);

    let lines = renderer.lines();
    // One of the rendered rows must display "Line 12"
    let contains_line_12 = lines[0..7].iter().any(|l| l.contains("Line 12"));
    assert!(contains_line_12, "Line 12 should be visible in viewport");

    // Cursor must be on screen within 0..7 rows
    let cursor = renderer.cursor();
    assert!(cursor.is_some());
    let (_, cursor_row) = cursor.unwrap();
    assert!(cursor_row < 7);
}

#[test]
fn test_statusline_responsive_narrow_terminal() {
    let theme = Theme::dark();
    let state = StatusLineState {
        mode_name: "NORMAL",
        mode_style: theme.mode_normal,
        buffer_name: "very_long_file_name_in_project.rs",
        is_modified: true,
        is_readonly: false,
        line: 42,
        col: 10,
        percent: "68%",
        encoding: "UTF-8",
        line_ending: "LF",
    };

    // 1. Full width: 90 columns
    let mut renderer_wide = TestRenderer::new(90, 1);
    StatusLine::render(&mut renderer_wide, 0, &state, &theme);
    let line_wide = &renderer_wide.lines()[0];
    assert!(line_wide.contains("NORMAL"));
    assert!(line_wide.contains("very_long_file_name"));
    assert!(line_wide.contains("[+]"));
    assert!(line_wide.contains("UTF-8 · LF"));
    assert!(line_wide.contains("42:10"));
    assert!(line_wide.contains("68%"));

    // 2. Medium width: 60 columns (drops encoding)
    let mut renderer_med = TestRenderer::new(60, 1);
    StatusLine::render(&mut renderer_med, 0, &state, &theme);
    let line_med = &renderer_med.lines()[0];
    assert!(line_med.contains("NORMAL"));
    assert!(line_med.contains("[+]"));
    assert!(!line_med.contains("UTF-8 · LF"));
    assert!(line_med.contains("42:10"));

    // 3. Narrow width: 35 columns
    let mut renderer_narrow = TestRenderer::new(35, 1);
    StatusLine::render(&mut renderer_narrow, 0, &state, &theme);
    let line_narrow = &renderer_narrow.lines()[0];
    assert!(line_narrow.contains("NORMAL"));
    assert!(line_narrow.contains("42:10"));
}

#[test]
fn test_unicode_and_wide_characters() {
    let text = "Emoji 🌍 and CJK 你好世界!";
    let buffer = Buffer::new(BufferId::from_raw(1), Text::from(text));

    let mut renderer = TestRenderer::new(40, 2);
    let mut view = BufferView {
        gutter: Gutter {
            mode: LineNumberMode::None,
            show_signs: false,
        },
        ..BufferView::default()
    };
    let theme = Theme::dark();

    view.render(&mut renderer, &buffer, 1, &theme);

    let line = &renderer.lines()[0];
    assert!(line.contains("Emoji 🌍 and CJK 你好世界!"));
}
