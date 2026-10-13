use kiln_text::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_char_byte_roundtrip(s in "\\PC*") {
        let text = Text::from(s.as_str());
        let len = text.len_chars();

        for i in 0..=len {
            let byte_idx = text.char_to_byte(i).unwrap();
            let char_idx = text.byte_to_char(byte_idx).unwrap();
            prop_assert_eq!(i, char_idx);
        }
    }

    #[test]
    fn prop_char_line_col_roundtrip(s in "([a-zA-Z0-9 \t\r\n]|[\u{1F300}-\u{1F5FF}])*") {
        let text = Text::from(s.as_str());
        let len = text.len_chars();

        for i in 0..=len {
            let pos = text.char_to_line_col(i).unwrap();
            let round_trip = text.line_col_to_char(pos).unwrap();
            prop_assert_eq!(i, round_trip);
        }
    }

    #[test]
    fn prop_char_utf16_roundtrip(s in "([a-zA-Z0-9 \t\r\n]|[\u{1F300}-\u{1F5FF}])*") {
        let text = Text::from(s.as_str());
        let len = text.len_chars();

        for i in 0..=len {
            let utf16_cu = text.char_to_utf16_cu(i).unwrap();
            let char_idx = text.utf16_cu_to_char(utf16_cu).unwrap();
            prop_assert_eq!(i, char_idx);

            let lsp_pos = text.char_to_utf16_line_col(i).unwrap();
            let round_trip_lsp = text.utf16_line_col_to_char(lsp_pos).unwrap();
            prop_assert_eq!(i, round_trip_lsp);
        }
    }

    #[test]
    fn prop_insert_remove_invariant(
        initial in "[a-zA-Z0-9 ]{0,50}",
        insert_str in "[a-zA-Z0-9 \n\u{1F300}-\u{1F3FF}]{1,20}",
        insert_pos in 0..=50usize
    ) {
        let mut text = Text::from(initial.as_str());
        let pos = insert_pos.min(text.len_chars());
        let orig_len = text.len_chars();

        text.insert(pos, &insert_str).unwrap();
        let inserted_len = insert_str.chars().count();
        prop_assert_eq!(text.len_chars(), orig_len + inserted_len);

        text.remove(pos..(pos + inserted_len)).unwrap();
        prop_assert_eq!(text.to_string(), initial);
    }

    #[test]
    fn prop_grapheme_boundaries_monotonic(s in "([a-zA-Z0-9 \n]|[\u{1F300}-\u{1F5FF}]|[\u{0300}-\u{036F}])*") {
        let text = Text::from(s.as_str());
        let len = text.len_chars();

        let mut curr = 0;
        let mut visited = vec![0];
        while curr < len {
            let next = next_grapheme_boundary(&text, curr);
            prop_assert!(next > curr, "Grapheme step must be strictly monotonic");
            prop_assert!(next <= len);
            curr = next;
            visited.push(curr);
        }

        // Verify reverse navigation step by step
        for window in visited.windows(2).rev() {
            let prev = prev_grapheme_boundary(&text, window[1]);
            prop_assert_eq!(prev, window[0]);
        }
    }
}
