use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
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
    let mut group = c.benchmark_group("core_bench");
    group.sample_size(10);

    println!("Constructing texts for core benchmarks (1 MB, 10 MB, 100 MB)...");
    let text_1mb = build_large_text(1);
    let text_10mb = build_large_text(10);
    let text_100mb = build_large_text(100);

    let mut allocator = BufferIdAllocator::new();
    let proto_buf_1mb = Buffer::new(allocator.allocate(), text_1mb);
    let proto_buf_10mb = Buffer::new(allocator.allocate(), text_10mb);
    let proto_buf_100mb = Buffer::new(allocator.allocate(), text_100mb);

    // (a) Apply a one-character insert in the middle across 1 MB, 10 MB, 100 MB
    let mid_1mb = proto_buf_1mb.text().len_chars() / 2;
    let tx_1char_1mb =
        Transaction::insert_at(proto_buf_1mb.text().len_chars(), mid_1mb, "X").unwrap();

    group.bench_function("apply_1char_insert_1mb", |b| {
        b.iter_batched(
            || proto_buf_1mb.clone(),
            |mut buf| {
                let res = buf.apply(black_box(tx_1char_1mb.clone())).unwrap();
                black_box(res);
            },
            BatchSize::SmallInput,
        );
    });

    let mid_10mb = proto_buf_10mb.text().len_chars() / 2;
    let tx_1char_10mb =
        Transaction::insert_at(proto_buf_10mb.text().len_chars(), mid_10mb, "X").unwrap();

    group.bench_function("apply_1char_insert_10mb", |b| {
        b.iter_batched(
            || proto_buf_10mb.clone(),
            |mut buf| {
                let res = buf.apply(black_box(tx_1char_10mb.clone())).unwrap();
                black_box(res);
            },
            BatchSize::SmallInput,
        );
    });

    let mid_100mb = proto_buf_100mb.text().len_chars() / 2;
    let tx_1char_100mb =
        Transaction::insert_at(proto_buf_100mb.text().len_chars(), mid_100mb, "X").unwrap();

    group.bench_function("apply_1char_insert_100mb", |b| {
        b.iter_batched(
            || proto_buf_100mb.clone(),
            |mut buf| {
                let res = buf.apply(black_box(tx_1char_100mb.clone())).unwrap();
                black_box(res);
            },
            BatchSize::SmallInput,
        );
    });

    // (b) Apply a 100-character replacement in the middle (100 MB)
    let replacement = "0123456789".repeat(10);
    let tx_100char_100mb = Transaction::replace_range(
        proto_buf_100mb.text().len_chars(),
        mid_100mb..(mid_100mb + 100),
        &replacement,
    )
    .unwrap();

    group.bench_function("apply_100char_replace_100mb", |b| {
        b.iter_batched(
            || proto_buf_100mb.clone(),
            |mut buf| {
                let res = buf.apply(black_box(tx_100char_100mb.clone())).unwrap();
                black_box(res);
            },
            BatchSize::SmallInput,
        );
    });

    // (c) Apply a transaction with 100 scattered small edits (100 MB)
    let len_100mb = proto_buf_100mb.text().len_chars();
    let step = (len_100mb - 200) / 100;
    let mut edits_100 = Vec::with_capacity(100);
    for i in 0..100 {
        let start = i * step + 10;
        edits_100.push((start..(start + 1), "Z"));
    }
    let tx_100_scattered_100mb = Transaction::from_edits(len_100mb, &edits_100).unwrap();

    group.bench_function("apply_100_scattered_edits_100mb", |b| {
        b.iter_batched(
            || proto_buf_100mb.clone(),
            |mut buf| {
                let res = buf
                    .apply(black_box(tx_100_scattered_100mb.clone()))
                    .unwrap();
                black_box(res);
            },
            BatchSize::SmallInput,
        );
    });

    // (d) Undo of (a) on 100 MB buffer
    group.bench_function("undo_1char_insert_100mb", |b| {
        b.iter_batched(
            || {
                let mut buf = proto_buf_100mb.clone();
                buf.apply(tx_1char_100mb.clone()).unwrap();
                buf
            },
            |mut buf| {
                let res = buf.undo().unwrap().unwrap();
                black_box(res);
            },
            BatchSize::SmallInput,
        );
    });

    // (e) Invert only: inverting 100-character replacement changeset against 100 MB text
    let cs_100char = tx_100char_100mb.changes.clone();
    let text_ref = proto_buf_100mb.text();

    group.bench_function("invert_only_100char_replace_100mb", |b| {
        b.iter(|| {
            let inv = cs_100char.invert(black_box(text_ref)).unwrap();
            black_box(inv);
        });
    });

    // Mapping 1,000 marks through a transaction with black_box
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
