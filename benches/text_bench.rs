use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kiln_text::Text;

fn build_large_text(target_mb: usize) -> Text {
    // Repeated 100-character line including newline
    let line = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+~`-=[]{};':,./<>?|\n";
    let target_bytes = target_mb * 1024 * 1024;
    let repeat_count = target_bytes / line.len();

    let mut chunk = String::with_capacity(line.len() * 1000);
    for _ in 0..1000 {
        chunk.push_str(line);
    }

    let mut text = Text::new();
    let num_chunks = repeat_count / 1000;
    for _ in 0..num_chunks {
        text.insert(text.len_chars(), &chunk)
            .expect("chunk insert failed");
    }
    text
}

fn bench_text_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("100mb_rope");
    // Reduce sample size for 100 MB text to complete reasonably in benchmark runs
    group.sample_size(10);

    println!("Constructing 100 MB rope for benchmark...");
    let mut large_text = build_large_text(100);
    println!(
        "100 MB rope ready: {} chars, {} lines",
        large_text.len_chars(),
        large_text.len_lines()
    );

    // 1. Reading: slice a 1000-character range from the middle
    let mid_char = large_text.len_chars() / 2;
    group.bench_function("read_slice_middle", |b| {
        b.iter(|| {
            let slice = large_text
                .slice(black_box(mid_char..mid_char + 1000))
                .unwrap();
            black_box(slice);
        });
    });

    // 2. Conversion: char_to_line_col
    group.bench_function("char_to_line_col", |b| {
        b.iter(|| {
            let pos = large_text.char_to_line_col(black_box(mid_char)).unwrap();
            black_box(pos);
        });
    });

    // 3. Inserting: insert 100 chars at middle and remove
    group.bench_function("insert_remove_middle", |b| {
        b.iter(|| {
            large_text.insert(black_box(mid_char), black_box("0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789")).unwrap();
            large_text.remove(black_box(mid_char..mid_char + 100)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_text_operations);
criterion_main!(benches);
