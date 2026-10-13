use kiln_core::*;
use kiln_text::Text;

#[test]
fn test_range_and_selection_normalization() {
    // Normalization merges overlapping/adjacent ranges and tracks primary
    let r1 = Range::new(0, 5);
    let r2 = Range::new(3, 8); // overlaps with r1 -> 0..8
    let r3 = Range::new(10, 15);

    let sel = Selections::new(vec![r1, r2, r3], 1).unwrap();
    assert_eq!(sel.ranges().len(), 2);
    assert_eq!(sel.ranges()[0], Range::new(0, 8));
    assert_eq!(sel.ranges()[1], Range::new(10, 15));
    // Since original index 1 (r2) was merged into first range, primary is 0
    assert_eq!(sel.primary_index(), 0);

    // Adjacent ranges merge: [0..5] and [5..10] -> [0..10]
    let sel_adj = Selections::new(vec![Range::new(0, 5), Range::new(5, 10)], 1).unwrap();
    assert_eq!(sel_adj.ranges().len(), 1);
    assert_eq!(sel_adj.ranges()[0], Range::new(0, 10));
}

#[test]
fn test_range_mapping_boundary_insertion_and_deletion() {
    // Non-empty range [5..10].
    // Insertion at 5 (start boundary) maps start with After -> moves to 5 + len.
    // Insertion at 10 (end boundary) maps end with Before -> stays at 10.
    // Insertion strictly inside (e.g. at 7) expands range.
    let range = Range::new(5, 10);

    // 1. Insertion at start boundary (pos 5)
    let tx_start = Transaction::insert_at(20, 5, "ABC").unwrap();
    let mapped_start = range.map(&tx_start.changes);
    // Boundary insertion stays OUTSIDE range: start moves from 5 to 8, end moves from 10 to 13
    assert_eq!(mapped_start, Range::new(8, 13));

    // 2. Insertion at end boundary (pos 10)
    let tx_end = Transaction::insert_at(20, 10, "XYZ").unwrap();
    let mapped_end = range.map(&tx_end.changes);
    // Boundary insertion stays OUTSIDE range: start is 5, end is 10 (Assoc::Before)
    assert_eq!(mapped_end, Range::new(5, 10));

    // 3. Insertion inside range (pos 7)
    let tx_inside = Transaction::insert_at(20, 7, "INS").unwrap();
    let mapped_inside = range.map(&tx_inside.changes);
    assert_eq!(mapped_inside, Range::new(5, 13));

    // 4. Deletion covering mapped position
    // Range [5..10], delete [3..12] -> collapses to deletion boundary (3..3)
    let tx_del = Transaction::delete_range(20, 3..12).unwrap();
    let mapped_del = range.map(&tx_del.changes);
    assert_eq!(mapped_del, Range::new(3, 3));

    // 5. Empty range (cursor point) maps with Assoc::After
    let point = Range::point(5);
    let mapped_point = point.map(&tx_start.changes);
    assert_eq!(mapped_point, Range::point(8));
}

#[test]
fn test_changeset_normalization() {
    // Normalization removes 0-length ops and merges adjacent same-kind ops
    let raw = vec![
        Operation::Retain(0),
        Operation::Retain(5),
        Operation::Retain(3), // merges to Retain(8)
        Operation::Delete(0),
        Operation::Delete(2),
        Operation::Delete(4), // merges to Delete(6)
        Operation::Insert(String::new()),
        Operation::Insert("Hello ".to_string()),
        Operation::Insert("World".to_string()), // merges to Insert("Hello World")
    ];
    let cs = ChangeSet::from_ops(raw);
    assert_eq!(cs.ops().len(), 3);
    assert_eq!(cs.ops()[0], Operation::Retain(8));
    assert_eq!(cs.ops()[1], Operation::Delete(6));
    assert_eq!(cs.ops()[2], Operation::Insert("Hello World".to_string()));
}

#[test]
fn test_changeset_compose_and_length_mismatch() {
    // Doc: "Hello World" (len 11)
    // tx1: delete "World" (5..11), insert "Rust" -> "Hello Rust" (len 10)
    // tx2: insert "!" at end (pos 10) -> "Hello Rust!" (len 11)
    let tx1 = Transaction::replace_range(11, 6..11, "Rust").unwrap();
    let tx2 = Transaction::insert_at(10, 10, "!").unwrap();

    let composed = tx1.compose(&tx2).unwrap();
    let text = Text::from("Hello World");
    let result = composed.changes.apply(&text).unwrap();
    assert_eq!(result.to_string(), "Hello Rust!");

    // Length mismatch: tx_a input 10, output 12. tx_b input 15, output 15.
    let tx_a = Transaction::insert_at(10, 5, "AB").unwrap();
    let tx_b = Transaction::insert_at(15, 10, "CD").unwrap();
    let mismatch = tx_a.compose(&tx_b).unwrap_err();
    assert!(matches!(
        mismatch,
        CoreError::TransactionLengthMismatch { .. }
    ));
}

#[test]
fn test_map_pos_monotonic_and_assoc() {
    let tx = Transaction::insert_at(10, 5, "ABC").unwrap();
    // pos 5: Assoc::Before -> 5, Assoc::After -> 8
    let before = tx.map_pos(5, Assoc::Before);
    let after = tx.map_pos(5, Assoc::After);
    assert_eq!(before, 5);
    assert_eq!(after, 8);
    assert!(after >= before);

    // For all positions 0..=10, map_pos(pos, After) >= map_pos(pos, Before)
    for p in 0..=10 {
        assert!(tx.map_pos(p, Assoc::After) >= tx.map_pos(p, Assoc::Before));
    }
}

#[test]
fn test_builder_from_edits() {
    let edits = vec![
        (0..2, "Hi"),  // replace "He" with "Hi"
        (5..6, " - "), // replace " " with " - "
        (11..11, "!"), // insert "!" at end
    ];
    let tx = Transaction::from_edits(11, &edits).unwrap();
    let text = Text::from("Hello World");
    let result = tx.changes.apply(&text).unwrap();
    assert_eq!(result.to_string(), "Hillo - World!");

    // Unsorted or overlapping edits fail
    let bad_edits = vec![(5..8, "a"), (3..6, "b")];
    assert_eq!(
        Transaction::from_edits(10, &bad_edits).unwrap_err(),
        CoreError::UnsortedOrOverlappingEdits
    );
}

#[test]
fn test_buffer_apply_undo_redo_version_and_dirty() {
    let mut allocator = BufferIdAllocator::new();
    let id = allocator.allocate();
    let mut buffer = Buffer::new(id, Text::from("Hello"));

    assert_eq!(buffer.version(), 1);
    assert!(!buffer.is_dirty()); // at saved state initially

    // Mark clean saved state
    buffer.mark_saved().unwrap();
    assert!(!buffer.is_dirty());

    // Apply edit
    let tx = Transaction::insert_at(5, 5, " World").unwrap();
    let res = buffer.apply(tx).unwrap();
    assert_eq!(res.old_version, 1);
    assert_eq!(res.new_version, 2);
    assert_eq!(buffer.version(), 2);
    assert_eq!(buffer.text().to_string(), "Hello World");
    assert!(buffer.is_dirty());

    // Undo edit
    let undo_res = buffer.undo().unwrap().unwrap();
    assert_eq!(undo_res.old_version, 2);
    assert_eq!(undo_res.new_version, 3);
    assert_eq!(buffer.version(), 3);
    assert_eq!(buffer.text().to_string(), "Hello");
    // Undoing back to saved state returns to clean!
    assert!(!buffer.is_dirty());

    // Redo edit
    let redo_res = buffer.redo().unwrap().unwrap();
    assert_eq!(redo_res.old_version, 3);
    assert_eq!(redo_res.new_version, 4);
    assert_eq!(buffer.version(), 4);
    assert_eq!(buffer.text().to_string(), "Hello World");
    assert!(buffer.is_dirty());
}

#[test]
fn test_branching_undo_tree_and_preferred_child() {
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Base"));

    // Step 1: Base -> Base A
    buffer
        .apply(Transaction::insert_at(4, 4, " A").unwrap())
        .unwrap();
    let seq_a = buffer
        .history()
        .get_seq(buffer.history().current().unwrap())
        .unwrap();

    // Undo -> Base
    buffer.undo().unwrap();
    assert_eq!(buffer.text().to_string(), "Base");

    // Step 2: Base -> Base B (creates branch!)
    buffer
        .apply(Transaction::insert_at(4, 4, " B").unwrap())
        .unwrap();
    assert_eq!(buffer.text().to_string(), "Base B");
    let seq_b = buffer
        .history()
        .get_seq(buffer.history().current().unwrap())
        .unwrap();

    // Check branch heads
    let heads = buffer.history().branch_heads();
    assert_eq!(heads.len(), 2);

    // Jump between branches via sequence numbers
    buffer.jump_to_seq(seq_a).unwrap();
    assert_eq!(buffer.text().to_string(), "Base A");

    buffer.jump_to_seq(seq_b).unwrap();
    assert_eq!(buffer.text().to_string(), "Base B");
}

#[test]
fn test_undo_groups_nested_empty_and_conflicts() {
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Start"));

    // 1. Grouped insert session: multiple keystroke transactions undo as ONE
    buffer.begin_group();
    buffer
        .apply(Transaction::insert_at(5, 5, " 1").unwrap())
        .unwrap();
    buffer.begin_group(); // nested group
    buffer
        .apply(Transaction::insert_at(7, 7, " 2").unwrap())
        .unwrap();
    buffer.end_group().unwrap(); // inner end
    buffer
        .apply(Transaction::insert_at(9, 9, " 3").unwrap())
        .unwrap();
    buffer.end_group().unwrap(); // outer end -> commits single node!

    assert_eq!(buffer.text().to_string(), "Start 1 2 3");
    // Undoing must undo the ENTIRE group back to "Start" in one step
    buffer.undo().unwrap();
    assert_eq!(buffer.text().to_string(), "Start");

    // Redo re-applies entire group
    buffer.redo().unwrap();
    assert_eq!(buffer.text().to_string(), "Start 1 2 3");

    // 2. Empty group creates no history node
    let cur_before = buffer.history().current();
    buffer.begin_group();
    buffer.end_group().unwrap();
    assert_eq!(buffer.history().current(), cur_before);

    // 3. Group error conditions
    // Ending without begin
    assert_eq!(
        buffer.end_group().unwrap_err(),
        CoreError::UnbalancedGroupEnd
    );

    // Calling undo/redo/jump or mark_saved during open group
    buffer.begin_group();
    assert_eq!(buffer.undo().unwrap_err(), CoreError::GroupOpenConflict);
    assert_eq!(buffer.redo().unwrap_err(), CoreError::GroupOpenConflict);
    assert_eq!(
        buffer.jump_to_seq(1).unwrap_err(),
        CoreError::GroupOpenConflict
    );
    assert_eq!(
        buffer.mark_saved().unwrap_err(),
        CoreError::GroupOpenConflict
    );
    buffer.end_group().unwrap();
}

#[test]
fn test_marks_mapping_and_retrieval() {
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Hello World"));

    buffer.marks_mut().set('a', 6); // at 'W'
    buffer.marks_mut().set('b', 11); // at end

    // Insert "Beautiful " at pos 6
    buffer
        .apply(Transaction::insert_at(11, 6, "Beautiful ").unwrap())
        .unwrap();
    // Mark 'a' maps with Assoc::After -> moves to 6 + 10 = 16
    assert_eq!(buffer.marks().get('a'), Some(16));
    assert_eq!(buffer.marks().get('b'), Some(21));

    // Delete [0..6] ("Hello ")
    buffer
        .apply(Transaction::delete_range(21, 0..6).unwrap())
        .unwrap();
    assert_eq!(buffer.marks().get('a'), Some(10));
    assert_eq!(buffer.marks().get('b'), Some(15));
}

#[test]
fn test_read_only_buffer_rejects_edits() {
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Protected"));
    buffer.set_read_only(true);
    assert!(buffer.is_read_only());

    let tx = Transaction::insert_at(9, 9, "!").unwrap();
    assert_eq!(buffer.apply(tx).unwrap_err(), CoreError::ReadOnly);
    assert_eq!(buffer.undo().unwrap_err(), CoreError::ReadOnly);
    assert_eq!(buffer.redo().unwrap_err(), CoreError::ReadOnly);
}

#[test]
fn test_unicode_and_approx_bytes() {
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Hello 🌍"));
    let len = buffer.text().len_chars();

    // Add ZWJ emoji family
    buffer
        .apply(Transaction::insert_at(len, len, " 👨‍👩‍👧‍👦").unwrap())
        .unwrap();
    assert_eq!(buffer.text().to_string(), "Hello 🌍 👨‍👩‍👧‍👦");

    // History memory estimate
    assert!(buffer.history().approx_bytes() > 0);
}
