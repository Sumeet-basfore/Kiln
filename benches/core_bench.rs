use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kiln_core::{Buffer, BufferIdAllocator, Marks, Transaction};
use kiln_text::Text;

fn build_large_text(target_mb: usize) -> Text {
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

fn bench_core_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_100mb");
    group.sample_size(10);

    println!("Constructing 100 MB buffer for core benchmark...");
    let large_text = build_large_text(100);
    let mut allocator = BufferIdAllocator::new();
    let id = allocator.allocate();
    let mut buffer = Buffer::new(id, large_text);
    println!("100 MB buffer ready: {} chars", buffer.text().len_chars());

    let mid = buffer.text().len_chars() / 2;

    // 1. Applying a small insert transaction to a ~100 MB buffer (and undoing)
    group.bench_function("apply_and_undo_transaction_100mb", |b| {
        b.iter(|| {
            let tx = Transaction::insert_at(
                buffer.text().len_chars(),
                black_box(mid),
                black_box("TEST"),
            )
            .unwrap();
            buffer.apply(tx).unwrap();
            buffer.undo().unwrap().unwrap();
        });
    });

    // 2. Mapping 1,000 marks through a transaction
    let mut marks = Marks::new();
    for i in 0..1000usize {
        let mark_char = char::from_u32((33 + (i % 90)) as u32).unwrap();
        marks.set(mark_char, i * 1000);
    }
    let tx_for_marks = Transaction::insert_at(2_000_000, 500, "INSERTION").unwrap();

    group.bench_function("map_1000_marks", |b| {
        b.iter(|| {
            let mut m = marks.clone();
            m.map(black_box(&tx_for_marks));
            black_box(m);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_core_operations);
criterion_main!(benches);
