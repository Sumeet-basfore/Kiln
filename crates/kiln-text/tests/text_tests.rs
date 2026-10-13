use kiln_text::*;

#[test]
fn test_line_breaks_only_newline() {
    // Lone CR, VT (\u{0B}), FF (\u{0C}), NEL (\u{85}), U+2028, U+2029
    let special = "Hello\rWorld\u{0B}Foo\u{0C}Bar\u{85}Baz\u{2028}Qux\u{2029}End";
    let text = Text::from(special);

    // None of these should count as a line break in the rope
    assert_eq!(text.len_lines(), 1);
    assert_eq!(text.line(0).unwrap(), special);
    assert_eq!(text.line_without_newline(0).unwrap(), special);

    // Only \n creates new lines
    let with_newline = "Hello\rWorld\nFoo\u{0B}Bar\nBaz";
    let text_nl = Text::from(with_newline);
    assert_eq!(text_nl.len_lines(), 3);
    assert_eq!(text_nl.line(0).unwrap(), "Hello\rWorld\n");
    assert_eq!(text_nl.line(1).unwrap(), "Foo\u{0B}Bar\n");
    assert_eq!(text_nl.line(2).unwrap(), "Baz");
}

#[test]
#[allow(clippy::reversed_empty_ranges)]
fn test_basic_crud_and_errors() {
    let mut text = Text::new();
    assert!(text.is_empty());
    assert_eq!(text.len_chars(), 0);
    assert_eq!(text.len_lines(), 1);

    // Insert
    text.insert(0, "Hello World").unwrap();
    assert_eq!(text.len_chars(), 11);
    assert_eq!(text.slice(0..5).unwrap(), "Hello");

    // Out of bounds insert
    let err = text.insert(100, "!").unwrap_err();
    assert_eq!(
        err,
        TextError::CharIndexOutOfBounds {
            char_idx: 100,
            len: 11
        }
    );

    // Invalid range remove
    let err = text.remove(8..4).unwrap_err();
    assert_eq!(err, TextError::InvalidCharRange { start: 8, end: 4 });

    // Remove
    text.remove(5..11).unwrap();
    assert_eq!(text.to_string(), "Hello");

    // Line indexing
    assert_eq!(text.line(0).unwrap(), "Hello");
    assert_eq!(text.line_without_newline(0).unwrap(), "Hello");
    let err = text.line(1).unwrap_err();
    assert_eq!(
        err,
        TextError::LineIndexOutOfBounds {
            line_idx: 1,
            len: 1
        }
    );
}

#[test]
fn test_conversions_ascii_and_multibyte() {
    // "a\nβ\n" ->
    // line 0: 'a', '\n' (chars 0, 1)
    // line 1: 'β', '\n' (chars 2, 3; 'β' is 2 bytes in UTF-8, 1 in UTF-16)
    let text = Text::from("a\nβ\n");
    assert_eq!(text.len_chars(), 4);
    assert_eq!(text.len_bytes(), 5);

    // char <-> byte
    assert_eq!(text.char_to_byte(0).unwrap(), 0);
    assert_eq!(text.char_to_byte(2).unwrap(), 2); // 'β' starts at byte 2
    assert_eq!(text.char_to_byte(3).unwrap(), 4); // '\n' starts at byte 4
    assert_eq!(text.byte_to_char(0).unwrap(), 0);
    assert_eq!(text.byte_to_char(2).unwrap(), 2);
    assert_eq!(text.byte_to_char(4).unwrap(), 3);

    // byte not on char boundary
    assert_eq!(
        text.byte_to_char(3).unwrap_err(),
        TextError::ByteIndexOutOfBounds {
            byte_idx: 3,
            len: 5
        }
    );

    // char <-> line_col
    assert_eq!(text.char_to_line_col(0).unwrap(), LineCol::new(0, 0));
    assert_eq!(text.char_to_line_col(1).unwrap(), LineCol::new(0, 1));
    assert_eq!(text.char_to_line_col(2).unwrap(), LineCol::new(1, 0));
    assert_eq!(text.line_col_to_char(LineCol::new(1, 0)).unwrap(), 2);
    assert_eq!(text.line_col_to_char(LineCol::new(1, 1)).unwrap(), 3);
}

#[test]
fn test_conversions_emoji_surrogate_pairs() {
    // "👋" (U+1F44B Wave): 1 char, 4 UTF-8 bytes, 2 UTF-16 code units
    // Text: "Hi 👋\n"
    // Chars: 'H'(0), 'i'(1), ' '(2), '👋'(3), '\n'(4)
    // UTF-16 code units: H(0), i(1), ' '(2), 👋(3, 4), \n(5)
    let text = Text::from("Hi 👋\n");
    assert_eq!(text.len_chars(), 5);
    assert_eq!(text.char_to_utf16_cu(3).unwrap(), 3);
    assert_eq!(text.char_to_utf16_cu(4).unwrap(), 5); // after emoji is 5 UTF-16 code units

    assert_eq!(text.utf16_cu_to_char(3).unwrap(), 3);
    assert_eq!(text.utf16_cu_to_char(5).unwrap(), 4);

    // Offset inside surrogate pair should return error
    let err = text.utf16_cu_to_char(4).unwrap_err();
    assert_eq!(
        err,
        TextError::Utf16IndexOutOfBounds {
            utf16_idx: 4,
            len: 6
        }
    );

    // UTF-16 line_col (LSP position)
    assert_eq!(text.char_to_utf16_line_col(3).unwrap(), LineCol::new(0, 3));
    assert_eq!(text.char_to_utf16_line_col(4).unwrap(), LineCol::new(0, 5));
    assert_eq!(text.utf16_line_col_to_char(LineCol::new(0, 3)).unwrap(), 3);
    assert_eq!(text.utf16_line_col_to_char(LineCol::new(0, 5)).unwrap(), 4);
}

#[test]
fn test_line_chars_non_allocating() {
    let text = Text::from("Line 1\nLine 2");
    let chars: Vec<char> = text.line_chars(0).unwrap().collect();
    assert_eq!(chars, vec!['L', 'i', 'n', 'e', ' ', '1', '\n']);

    let chars2: Vec<char> = text.line_chars(1).unwrap().collect();
    assert_eq!(chars2, vec!['L', 'i', 'n', 'e', ' ', '2']);
}

#[test]
fn test_grapheme_navigation_and_widths() {
    // Family emoji: 👨‍👩‍👧‍👦 (composed with ZWJ U+200D)
    // Chars: Man(1), ZWJ(1), Woman(1), ZWJ(1), Girl(1), ZWJ(1), Boy(1) = 7 chars total
    let family = "👨‍👩‍👧‍👦";
    let s = format!("A{}B", family);
    let text = Text::from(s.as_str());

    assert_eq!(next_grapheme_boundary(&text, 0), 1); // 'A'
    assert_eq!(next_grapheme_boundary(&text, 1), 8); // entire family emoji cluster
    assert_eq!(next_grapheme_boundary(&text, 8), 9); // 'B'

    assert_eq!(prev_grapheme_boundary(&text, 9), 8);
    assert_eq!(prev_grapheme_boundary(&text, 8), 1);
    assert_eq!(prev_grapheme_boundary(&text, 1), 0);

    // Widths
    assert_eq!(grapheme_width("A"), 1);
    assert_eq!(grapheme_width("中"), 2);
    assert_eq!(grapheme_width("\t"), 1);
    assert_eq!(str_display_width("Hello 世界"), 10); // 5 + 1 + 2*2 = 10
}

#[test]
fn test_graphemes_across_chunk_boundaries() {
    // Ropey chunks are typically 512-1024 bytes. Construct a ~4 KB string with multi-byte and ZWJ emoji
    let unit = "Text 👨‍👩‍👧‍👦 combining e\u{0301} CJK 中文\n";
    let large_s: String = unit.repeat(100);
    let text = Text::from(large_s.as_str());

    let mut idx = 0;
    let len = text.len_chars();
    let mut boundaries = vec![0];
    while idx < len {
        idx = next_grapheme_boundary(&text, idx);
        boundaries.push(idx);
    }
    assert_eq!(*boundaries.last().unwrap(), len);

    // Verify reverse navigation matches exactly
    for window in boundaries.windows(2).rev() {
        let prev = prev_grapheme_boundary(&text, window[1]);
        assert_eq!(prev, window[0]);
    }
}

#[test]
fn test_line_endings_detection_and_normalization() {
    // 1. No newlines -> defaults to Lf, is_mixed = false
    let det = detect_line_endings(b"Single line no ending");
    assert_eq!(det.dominant, LineEnding::Lf);
    assert!(!det.is_mixed);

    // 2. Pure LF
    let det = detect_line_endings(b"a\nb\nc\n");
    assert_eq!(det.dominant, LineEnding::Lf);
    assert!(!det.is_mixed);

    // 3. Pure CRLF
    let det = detect_line_endings(b"a\r\nb\r\nc\r\n");
    assert_eq!(det.dominant, LineEnding::Crlf);
    assert!(!det.is_mixed);

    // 4. Pure CR (classic Mac)
    let det = detect_line_endings(b"a\rb\rc\r");
    assert_eq!(det.dominant, LineEnding::Cr);
    assert!(!det.is_mixed);

    // 5. Mixed: CRLF and LF
    let det = detect_line_endings(b"a\r\nb\nc\r\n");
    assert_eq!(det.dominant, LineEnding::Crlf);
    assert!(det.is_mixed);

    // 6. Normalization: pure CR file converted to LF
    assert_eq!(normalize_line_endings("a\rb\rc"), "a\nb\nc");

    // 7. Normalization: CRLF file with stray CR
    // \r\n should become \n, stray \r should stay
    let mixed_cr = "a\r\nb\rc\r\n";
    let normalized = normalize_line_endings(mixed_cr);
    assert_eq!(normalized, "a\nb\rc\n");

    // 8. Denormalize back to CRLF
    let denorm = denormalize_line_endings(&normalized, LineEnding::Crlf);
    assert_eq!(denorm, "a\r\nb\rc\r\n");
}

#[test]
fn test_encoding_detection_and_roundtrips() {
    // UTF-8
    let res = detect_and_decode("Hello World".as_bytes());
    assert_eq!(res.encoding, Encoding::Utf8);
    assert_eq!(res.text, "Hello World");
    assert!(!res.is_lossy);

    // UTF-8 BOM
    let mut bom_bytes = vec![0xEF, 0xBB, 0xBF];
    bom_bytes.extend_from_slice("Hello BOM".as_bytes());
    let res = detect_and_decode(&bom_bytes);
    assert_eq!(res.encoding, Encoding::Utf8Bom);
    assert_eq!(res.text, "Hello BOM");
    assert!(!res.is_lossy);
    let (encoded, lossy) = encode(&res.text, Encoding::Utf8Bom);
    assert!(!lossy);
    assert_eq!(encoded, bom_bytes);

    // UTF-16 LE with BOM
    let (utf16_le_bytes, _) = encode("Hello 🌍", Encoding::Utf16Le);
    let res = detect_and_decode(&utf16_le_bytes);
    assert_eq!(res.encoding, Encoding::Utf16Le);
    assert_eq!(res.text, "Hello 🌍");
    assert!(!res.is_lossy);

    // UTF-16 BE with BOM
    let (utf16_be_bytes, _) = encode("Hello 🌍", Encoding::Utf16Be);
    let res = detect_and_decode(&utf16_be_bytes);
    assert_eq!(res.encoding, Encoding::Utf16Be);
    assert_eq!(res.text, "Hello 🌍");
    assert!(!res.is_lossy);

    // Latin-1 fallback for invalid UTF-8
    let latin1_bytes = vec![0xE9, 0xE0, 0xE8]; // é, à, è in ISO-8859-1 (invalid UTF-8)
    let res = detect_and_decode(&latin1_bytes);
    assert_eq!(res.encoding, Encoding::Latin1);
    assert_eq!(res.text, "éàè");
    assert!(!res.is_lossy);
    let (encoded_latin1, lossy) = encode(&res.text, Encoding::Latin1);
    assert!(!lossy);
    assert_eq!(encoded_latin1, latin1_bytes);

    // Latin-1 lossy encoding when text has non-Latin-1 chars
    let (lossy_bytes, is_lossy) = encode("Hello 🌍", Encoding::Latin1);
    assert!(is_lossy);
    assert_eq!(lossy_bytes, b"Hello ?");
}

#[test]
fn test_binary_detection() {
    // Normal text
    assert!(!is_binary(b"Hello world\nThis is plain text"));

    // Binary with NUL byte
    assert!(is_binary(b"ELF\x7F\x00\x00\x00"));

    // UTF-16 LE text starts with FF FE, contains \0 bytes but is NOT binary
    let (utf16_bytes, _) = encode("Hello UTF-16", Encoding::Utf16Le);
    assert!(!is_binary(&utf16_bytes));

    // UTF-16 BE text starts with FE FF, contains \0 bytes but is NOT binary
    let (utf16_be, _) = encode("Hello UTF-16 BE", Encoding::Utf16Be);
    assert!(!is_binary(&utf16_be));
}
