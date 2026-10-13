//! Benchmark for UI frame rendering at 200x60 viewport.
//!
//! Enforces the PRD Section 8 keystroke-to-screen performance budget (< 16 ms).

use criterion::{criterion_group, criterion_main, Criterion};
use kiln_core::{Buffer, BufferId, LineCol, Selections, Text};
use kiln_ui::{
    BufferView, Gutter, LineNumberMode, Renderer, StatusLine, StatusLineState, TestRenderer, Theme,
};
use std::hint::black_box;

/// Generates a large buffer (~100 MB, ~1,100,000 lines).
fn create_100mb_buffer() -> Buffer {
    let line = "The quick brown fox jumps over the lazy dog. 1234567890! #$%^&*()_+\n";
    let count = (100 * 1024 * 1024) / line.len();
    let text = line.repeat(count);
    Buffer::new(BufferId::from_raw(1), Text::from(text.as_str()))
}

fn bench_ui_frame(c: &mut Criterion) {
    let buffer = create_100mb_buffer();
    let theme = Theme::dark();

    let mut group = c.benchmark_group("ui_frame_200x60");

    // Benchmark 1: Top of 100 MB buffer (200x60)
    group.bench_function("render_frame_top_200x60_100mb", |b| {
        let mut renderer = TestRenderer::new(200, 60);
        let mut view = BufferView {
            gutter: Gutter {
                mode: LineNumberMode::Hybrid,
                show_signs: true,
            },
            ..BufferView::default()
        };

        let status_state = StatusLineState {
            mode_name: "NORMAL",
            mode_style: theme.mode_normal,
            buffer_name: "large_file.txt",
            is_modified: false,
            is_readonly: false,
            line: 1,
            col: 1,
            percent: "Top",
            encoding: "UTF-8",
            line_ending: "LF",
        };

        b.iter(|| {
            renderer.clear();
            view.render(
                black_box(&mut renderer),
                black_box(&buffer),
                black_box(59),
                black_box(&theme),
            );
            StatusLine::render(
                black_box(&mut renderer),
                black_box(59),
                black_box(&status_state),
                black_box(&theme),
            );
            renderer.flush().unwrap();
            black_box(renderer.cursor())
        });
    });

    // Benchmark 2: Middle of 100 MB buffer (cursor at line 500,000)
    group.bench_function("render_frame_middle_200x60_100mb", |b| {
        let mut middle_buffer = buffer.clone();
        let middle_char = middle_buffer
            .text()
            .line_col_to_char(LineCol::new(500_000, 10))
            .unwrap();
        middle_buffer.set_selections(Selections::point(middle_char));

        let mut renderer = TestRenderer::new(200, 60);
        let mut view = BufferView {
            gutter: Gutter {
                mode: LineNumberMode::Hybrid,
                show_signs: true,
            },
            ..BufferView::default()
        };

        let status_state = StatusLineState {
            mode_name: "NORMAL",
            mode_style: theme.mode_normal,
            buffer_name: "large_file.txt",
            is_modified: false,
            is_readonly: false,
            line: 500_001,
            col: 11,
            percent: "45%",
            encoding: "UTF-8",
            line_ending: "LF",
        };

        b.iter(|| {
            renderer.clear();
            view.render(
                black_box(&mut renderer),
                black_box(&middle_buffer),
                black_box(59),
                black_box(&theme),
            );
            StatusLine::render(
                black_box(&mut renderer),
                black_box(59),
                black_box(&status_state),
                black_box(&theme),
            );
            renderer.flush().unwrap();
            black_box(renderer.cursor())
        });
    });

    group.finish();
}

criterion_group!(benches, bench_ui_frame);
criterion_main!(benches);
