use proptest::prelude::*;

use kiln_core::*;
use kiln_text::Text;

proptest! {
    #[test]
    fn prop_apply_invert_identity(s in "[a-zA-Z0-9 \n\u{1F300}-\u{1F350}]{0,50}") {
        let text = Text::from(s.as_str());
        let doc_len = text.len_chars();

        // Generate a valid changeset
        let cs = ChangeSet::from_ops(vec![
            Operation::Retain(doc_len / 2),
            Operation::Insert("ABC".to_string()),
            Operation::Retain(doc_len - (doc_len / 2)),
        ]);

        let new_text = cs.apply(&text).unwrap();
        let inv = cs.invert(&text).unwrap();
        let restored = inv.apply(&new_text).unwrap();

        prop_assert_eq!(text.to_string(), restored.to_string());
    }

    #[test]
    fn prop_compose_equals_sequential(s in "[a-zA-Z0-9 \n]{5,30}") {
        let text = Text::from(s.as_str());
        let doc_len = text.len_chars();

        // tx1: insert at 2
        let tx1 = Transaction::insert_at(doc_len, 2, "X").unwrap();
        let mid_text = tx1.changes.apply(&text).unwrap();

        // tx2: delete 1 char at 4
        let tx2 = Transaction::delete_range(mid_text.len_chars(), 4..5).unwrap();
        let end_text = tx2.changes.apply(&mid_text).unwrap();

        // Composed tx
        let composed = tx1.compose(&tx2).unwrap();
        let direct_text = composed.changes.apply(&text).unwrap();

        prop_assert_eq!(end_text.to_string(), direct_text.to_string());
    }

    #[test]
    fn prop_mapped_pos_monotonic_and_within_bounds(
        doc_len in 5..=30usize,
        ins_pos in 0..=5usize,
        del_start in 2..=5usize,
        del_len in 0..=3usize
    ) {
        let mut ops = Vec::new();
        ops.push(Operation::Retain(ins_pos));
        ops.push(Operation::Insert("Hello".to_string()));
        if del_start > ins_pos {
            ops.push(Operation::Retain(del_start - ins_pos));
        }
        ops.push(Operation::Delete(del_len));
        let consumed = del_start + del_len;
        if doc_len > consumed {
            ops.push(Operation::Retain(doc_len - consumed));
        }
        let cs = ChangeSet::from_ops(ops);
        let new_len = cs.new_len();

        let mut prev_after = 0;
        let mut prev_before = 0;

        for p in 0..=doc_len {
            let mapped_after = cs.map_pos(p, Assoc::After);
            let mapped_before = cs.map_pos(p, Assoc::Before);

            prop_assert!(mapped_after <= new_len);
            prop_assert!(mapped_before <= new_len);
            prop_assert!(mapped_after >= mapped_before);

            // Monotonicity: mapping preserves order for increasing positions
            prop_assert!(mapped_after >= prev_after);
            prop_assert!(mapped_before >= prev_before);

            prev_after = mapped_after;
            prev_before = mapped_before;
        }
    }

    #[test]
    fn prop_undo_redo_roundtrip(s in "[a-zA-Z0-9 ]{5,20}") {
        let mut allocator = BufferIdAllocator::new();
        let mut buffer = Buffer::new(allocator.allocate(), Text::from(s.as_str()));
        let initial_text = buffer.text().to_string();

        let tx = Transaction::insert_at(buffer.text().len_chars(), 2, "NEW").unwrap();
        buffer.apply(tx).unwrap();

        let modified_text = buffer.text().to_string();
        prop_assert_ne!(&initial_text, &modified_text);

        buffer.undo().unwrap();
        prop_assert_eq!(buffer.text().to_string(), initial_text);

        buffer.redo().unwrap();
        prop_assert_eq!(buffer.text().to_string(), modified_text);
    }

    #[test]
    fn prop_compose_associative(s in "[a-zA-Z0-9 ]{15,40}") {
        let text = Text::from(s.as_str());
        let len0 = text.len_chars();

        // tx_a: insert at 2
        let tx_a = Transaction::insert_at(len0, 2, "A").unwrap();
        let text_a = tx_a.changes.apply(&text).unwrap();
        let len_a = text_a.len_chars();

        // tx_b: delete 1 char at 5
        let tx_b = Transaction::delete_range(len_a, 5..6).unwrap();
        let text_b = tx_b.changes.apply(&text_a).unwrap();
        let len_b = text_b.len_chars();

        // tx_c: insert at 7
        let tx_c = Transaction::insert_at(len_b, 7, "C").unwrap();

        // (a compose b) compose c
        let ab = tx_a.compose(&tx_b).unwrap();
        let ab_c = ab.compose(&tx_c).unwrap();

        // a compose (b compose c)
        let bc = tx_b.compose(&tx_c).unwrap();
        let a_bc = tx_a.compose(&bc).unwrap();

        // 1. Assert normalized ChangeSets are equal
        prop_assert_eq!(&ab_c.changes, &a_bc.changes);

        // 2. Assert applied results are equal
        let res1 = ab_c.changes.apply(&text).unwrap();
        let res2 = a_bc.changes.apply(&text).unwrap();
        prop_assert_eq!(res1.to_string(), res2.to_string());
    }

    #[test]
    fn prop_map_pos_assoc_ordering(
        doc_len in 5..=30usize,
        ins_pos in 0..=5usize,
        del_start in 2..=5usize,
        del_len in 0..=3usize
    ) {
        let mut ops = Vec::new();
        ops.push(Operation::Retain(ins_pos));
        ops.push(Operation::Insert("Hello".to_string()));
        if del_start > ins_pos {
            ops.push(Operation::Retain(del_start - ins_pos));
        }
        ops.push(Operation::Delete(del_len));
        let consumed = del_start + del_len;
        if doc_len > consumed {
            ops.push(Operation::Retain(doc_len - consumed));
        }
        let cs = ChangeSet::from_ops(ops);

        for p in 0..=doc_len {
            prop_assert!(cs.map_pos(p, Assoc::After) >= cs.map_pos(p, Assoc::Before));
        }
    }

    #[test]
    fn prop_random_edits_model_based(
        initial in "[a-zA-Z0-9 ]{5,15}",
        steps in prop::collection::vec(0..=4u8, 1..=20)
    ) {
        let mut allocator = BufferIdAllocator::new();
        let mut buffer = Buffer::new(allocator.allocate(), Text::from(initial.as_str()));

        // Model buffer maintaining history of plain strings for undo/redo
        let mut model_history: Vec<String> = vec![initial.clone()];
        let mut model_cursor = 0usize;

        buffer.marks_mut().set('a', 0);
        let init_len = buffer.text().len_chars();
        buffer.marks_mut().set('b', init_len);

        for step in steps {
            match step {
                0 => {
                    // Random small insert
                    let len = buffer.text().len_chars();
                    let pos = if len == 0 { 0 } else { (len / 2).min(len) };
                    let tx = Transaction::insert_at(len, pos, "X").unwrap();
                    let _ = buffer.apply(tx);

                    // Update model: truncate redo history and push new state
                    model_history.truncate(model_cursor + 1);
                    let mut s = model_history[model_cursor].clone();
                    s.insert(pos, 'X');
                    model_history.push(s);
                    model_cursor += 1;
                }
                1 => {
                    // Random small delete
                    let len = buffer.text().len_chars();
                    if len > 1 {
                        let pos = (len / 3).min(len - 1);
                        let tx = Transaction::delete_range(len, pos..(pos + 1)).unwrap();
                        let _ = buffer.apply(tx);

                        model_history.truncate(model_cursor + 1);
                        let mut s = model_history[model_cursor].clone();
                        s.remove(pos);
                        model_history.push(s);
                        model_cursor += 1;
                    }
                }
                2 => {
                    // Undo
                    if model_cursor > 0 {
                        let _ = buffer.undo();
                        model_cursor -= 1;
                    }
                }
                3 => {
                    // Redo
                    if model_cursor + 1 < model_history.len() {
                        let _ = buffer.redo();
                        model_cursor += 1;
                    }
                }
                4 => {
                    // Group insert
                    let len = buffer.text().len_chars();
                    buffer.begin_group();
                    let tx1 = Transaction::insert_at(len, 0, "[").unwrap();
                    let _ = buffer.apply(tx1);
                    let len2 = buffer.text().len_chars();
                    let tx2 = Transaction::insert_at(len2, len2, "]").unwrap();
                    let _ = buffer.apply(tx2);
                    let _ = buffer.end_group();

                    model_history.truncate(model_cursor + 1);
                    let mut s = model_history[model_cursor].clone();
                    s.insert(0, '[');
                    s.push(']');
                    model_history.push(s);
                    model_cursor += 1;
                }
                _ => {}
            }

            // Invariant 1: Buffer text equals model string after EVERY step
            prop_assert_eq!(buffer.text().to_string(), model_history[model_cursor].clone());

            // Invariant 2: Selections stay strictly within 0..=len
            let len = buffer.text().len_chars();
            for r in buffer.selections().ranges() {
                prop_assert!(r.anchor <= len);
                prop_assert!(r.head <= len);
            }

            // Invariant 3: Marks stay strictly within 0..=len
            if let Some(pos) = buffer.marks().get('a') {
                prop_assert!(pos <= len);
            }
            if let Some(pos) = buffer.marks().get('b') {
                prop_assert!(pos <= len);
            }
        }
    }
}
