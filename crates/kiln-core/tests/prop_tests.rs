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
}
