//! Viewport resize tests for kiln-ui.

use kiln_core::{Buffer, BufferId, LineCol, Selections, Text};
use kiln_ui::{BufferView, Gutter, LineNumberMode, TestRenderer, Theme};

#[test]
fn test_viewport_resizing_adapts_layout_and_scrolling() {
    // 50 lines of text
    let text: String = (1..=50)
        .map(|i| format!("Document Line #{:03}\n", i))
        .collect();
    let mut buffer = Buffer::new(BufferId::from_raw(1), Text::from(text.as_str()));

    // Place cursor at line 25
    let char_pos = buffer.text().line_col_to_char(LineCol::new(25, 5)).unwrap();
    buffer.set_selections(Selections::point(char_pos));

    let mut view = BufferView {
        gutter: Gutter {
            mode: LineNumberMode::Hybrid,
            show_signs: true,
        },
        scrolloff_rows: 3,
        ..BufferView::default()
    };
    let theme = Theme::dark();

    // 1. Initial size: 80x24 (23 view rows)
    let mut renderer = TestRenderer::new(80, 24);
    view.render(&mut renderer, &buffer, 23, &theme);
    assert!(renderer.cursor().is_some());
    let (_, r1) = renderer.cursor().unwrap();
    assert!(r1 < 23);

    // 2. Expand to 120x40 (39 view rows)
    renderer.resize(120, 40);
    view.render(&mut renderer, &buffer, 39, &theme);
    assert!(renderer.cursor().is_some());
    let (c2, r2) = renderer.cursor().unwrap();
    assert!(r2 < 39);
    assert!(c2 < 120);

    // 3. Shrink to narrow and short 40x10 (9 view rows)
    renderer.resize(40, 10);
    view.render(&mut renderer, &buffer, 9, &theme);
    assert!(renderer.cursor().is_some());
    let (c3, r3) = renderer.cursor().unwrap();
    assert!(r3 < 9);
    assert!(c3 < 40);

    // Cursor line (line 25) must be visible in the rendered screen
    let visible_lines = renderer.lines();
    let found_cursor_line = visible_lines.iter().any(|l| l.contains("Line #026"));
    assert!(
        found_cursor_line,
        "Cursor line 25 (Line #026) must remain visible after shrinking viewport"
    );
}
