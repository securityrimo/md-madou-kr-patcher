use super::*;

#[test]
fn test_parse_control_codes() {
    let tokens = parse_display_text("{FF50:001D}{FF10}");
    assert_eq!(
        tokens,
        vec![Token::CtrlParam(0xFF50, 0x001D), Token::Ctrl(0xFF10),]
    );
}

#[test]
fn test_parse_named_controls() {
    let tokens = parse_display_text("{NL}{PAGE}{END}");
    assert_eq!(
        tokens,
        vec![
            Token::Ctrl(0xFF30),
            Token::Ctrl(0xFF34),
            Token::Ctrl(0xFF04),
        ]
    );
}

#[test]
fn test_parse_special_tiles() {
    let tokens = parse_display_text("{hp:1}{spell:0}{header}");
    assert_eq!(
        tokens,
        vec![
            Token::Tile(0x005F),
            Token::Tile(0x0050),
            Token::Tile(0x0076),
        ]
    );
}

#[test]
fn test_parse_korean_and_en() {
    let tokens = parse_display_text("가A나B");
    assert_eq!(
        tokens,
        vec![
            Token::KrChar('가'),
            Token::EnChar('A'),
            Token::KrChar('나'),
            Token::EnChar('B'),
        ]
    );
}

#[test]
fn test_parse_raw_hex() {
    let tokens = parse_display_text("[004A]");
    assert_eq!(tokens, vec![Token::Raw(0x004A)]);
}

#[test]
fn test_parse_source_backed_full_width_pad() {
    assert_eq!(parse_display_text("{source-pad}"), vec![Token::LayoutPad]);
}

#[test]
fn test_parse_icon() {
    let tokens = parse_display_text("{icon:5B}");
    assert_eq!(tokens, vec![Token::Tile(0x005B)]);
}

#[test]
fn test_parse_quote() {
    let tokens = parse_display_text("{q}");
    assert_eq!(tokens, vec![Token::Tile(0x0049)]);
}

#[test]
fn test_parse_q_open() {
    let tokens = parse_display_text("{q-open}Hello{q}");
    assert_eq!(
        tokens,
        vec![
            Token::Tile(0x0048),
            Token::EnChar('H'),
            Token::EnChar('e'),
            Token::EnChar('l'),
            Token::EnChar('l'),
            Token::EnChar('o'),
            Token::Tile(0x0049),
        ]
    );
}

#[test]
fn test_encode_simple() {
    let kr_charmap: HashMap<char, u16> = [('가', 0x0100), ('나', 0x0101)].into();
    let en_charmap: HashMap<char, u16> = [(' ', 0x004A), ('!', 0x0040)].into();

    let words = encode_text("가 나!", &kr_charmap, &en_charmap).unwrap();
    assert_eq!(words, vec![0x0100, 0x004A, 0x0101, 0x0040]);
}

#[test]
fn test_encode_with_controls() {
    let kr_charmap: HashMap<char, u16> = [('가', 0x0100)].into();
    let en_charmap: HashMap<char, u16> = [].into();

    let words = encode_text("{FF10}가{NL}{END}", &kr_charmap, &en_charmap).unwrap();
    assert_eq!(words, vec![0xFF10, 0x0100, 0xFF30, 0xFF04]);
}

#[test]
fn test_encode_unknown_char_error() {
    let kr_charmap: HashMap<char, u16> = [].into();
    let en_charmap: HashMap<char, u16> = [].into();
    let result = encode_text("가", &kr_charmap, &en_charmap);
    assert!(result.is_err());
}

#[test]
fn test_words_to_bytes() {
    let bytes = words_to_bytes(&[0xFF50, 0x001D]);
    assert_eq!(bytes, vec![0xFF, 0x50, 0x00, 0x1D]);
}

#[test]
fn test_build_kr_charmap() {
    let chars = vec!['가', '나', '다'];
    let charmap = build_kr_charmap(&chars);
    assert_eq!(charmap[&'가'], 0x0100);
    assert_eq!(charmap[&'나'], 0x0101);
    assert_eq!(charmap[&'다'], 0x0102);
}

#[test]
fn test_is_korean() {
    assert!(is_korean('가'));
    assert!(is_korean('힣'));
    assert!(is_korean('ㄱ'));
    assert!(!is_korean('A'));
    assert!(!is_korean(' '));
}

// ====== New tests for charmap generation & ROM decoder ======

#[test]
fn test_generate_charmap_key_count() {
    let cm = generate_charmap();
    // Must match existing charmap.json: 119 entries
    assert_eq!(cm.len(), 119);
}

#[test]
fn test_generate_charmap_digits() {
    let cm = generate_charmap();
    assert_eq!(cm["0x0001"], "0");
    assert_eq!(cm["0x000A"], "9");
}

#[test]
fn test_generate_charmap_punctuation() {
    let cm = generate_charmap();
    // Corrected order (not Python's original)
    assert_eq!(cm["0x0040"], ",");
    assert_eq!(cm["0x0041"], "!");
    assert_eq!(cm["0x0042"], "?");
    assert_eq!(cm["0x004B"], "-");
    assert_eq!(cm["0x004C"], ":");
}

#[test]
fn test_build_tile_to_display() {
    let cm = generate_charmap();
    let t2d = build_tile_to_display(&cm);
    assert_eq!(t2d[&0x000B], "A");
    assert_eq!(t2d[&0x004A], " ");
    assert_eq!(t2d[&0x0050], "{spell:0}");
}

#[test]
fn test_read_rom_words_basic() {
    // Mock ROM: FF10 000B 000D FF30 000B FF04
    let rom: Vec<u8> = vec![
        0xFF, 0x10, 0x00, 0x0B, 0x00, 0x0D, 0xFF, 0x30, 0x00, 0x0B, 0xFF, 0x04,
    ];
    let cwp = ctrl_with_param();
    let block_end: HashSet<u16> = [0xFF38, 0xFF04, 0xFFFF].into();
    let words = read_rom_words(&rom, 0, &cwp, &block_end);
    assert_eq!(words, vec![0xFF10, 0x000B, 0x000D, 0xFF30, 0x000B, 0xFF04]);
}

#[test]
fn test_read_rom_words_with_param() {
    // FF50 001D FF10 000B FF04
    let rom: Vec<u8> = vec![0xFF, 0x50, 0x00, 0x1D, 0xFF, 0x10, 0x00, 0x0B, 0xFF, 0x04];
    let cwp = ctrl_with_param();
    let block_end: HashSet<u16> = [0xFF38, 0xFF04, 0xFFFF].into();
    let words = read_rom_words(&rom, 0, &cwp, &block_end);
    // FF50 takes param 001D
    assert_eq!(words, vec![0xFF50, 0x001D, 0xFF10, 0x000B, 0xFF04]);
}

#[test]
fn test_words_to_display_text_basic() {
    let cm = generate_charmap();
    let t2d = build_tile_to_display(&cm);
    let cwp = ctrl_with_param();

    let words = vec![0xFF50, 0x001D, 0xFF10, 0x000B, 0xFF30, 0xFF04];
    let display = words_to_display_text(&words, &t2d, &cwp);
    assert_eq!(display, "{FF50:001D}{FF10}A{NL}{END}");
}

#[test]
fn test_words_to_readable_text() {
    let cm = generate_charmap();
    let t2d = build_tile_to_display(&cm);
    let cwp = ctrl_with_param();

    let words = vec![
        0xFF50, 0x001D, 0xFF10, 0x000B, 0x000C, 0xFF30, 0x000D, 0xFF04,
    ];
    let readable = words_to_readable_text(&words, &t2d, &cwp);
    assert_eq!(readable, "AB\nC");
}

#[test]
fn test_roundtrip_display_encode() {
    // Verify: words → display → parse → encode → words
    let cm = generate_charmap();
    let t2d = build_tile_to_display(&cm);
    let cwp = ctrl_with_param();

    let original = vec![
        0xFF50, 0x001D, 0xFF10, 0x000B, 0x004A, 0x0025, 0xFF30, 0xFF04,
    ];
    let display = words_to_display_text(&original, &t2d, &cwp);

    let en_charmap = load_en_charmap_from_generated(&cm);
    let kr_charmap: HashMap<char, u16> = HashMap::new();
    let re_encoded = encode_text(&display, &kr_charmap, &en_charmap).unwrap();
    assert_eq!(re_encoded, original);
}

#[test]
fn test_all_charmap_special_tokens_parseable() {
    // Every {xxx} token in the charmap MUST be recognized by parse_display_text.
    // This prevents silent token drops like the {q-open} bug.
    let cm = generate_charmap();
    for (hex_key, display) in &cm {
        if !display.starts_with('{') {
            continue;
        }
        let tile_idx = u16::from_str_radix(hex_key.trim_start_matches("0x"), 16).unwrap();
        let tokens = parse_display_text(display);
        assert!(
            !tokens.is_empty(),
            "charmap token {display} (tile {hex_key}) produced no parse tokens — \
             add it to special_tiles() in text.rs"
        );
        match &tokens[0] {
            Token::Tile(t) => assert_eq!(
                *t, tile_idx,
                "charmap token {display} parsed to tile 0x{t:04X} but expected 0x{tile_idx:04X}"
            ),
            other => panic!(
                "charmap token {display} (tile {hex_key}) parsed as {other:?}, expected Token::Tile"
            ),
        }
    }
}
