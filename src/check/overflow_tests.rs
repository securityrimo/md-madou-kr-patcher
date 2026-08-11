use super::*;

fn dummy_en_advance() -> HashMap<u16, u32> {
    let mut m = HashMap::new();
    // Space
    m.insert(0x4A, 4);
    // A=0x0B
    m.insert(0x0B, 8);
    // B=0x0C
    m.insert(0x0C, 8);
    // a=0x25
    m.insert(0x25, 7);
    // '.'=0x3F
    m.insert(0x3F, 4);
    // '!'=0x40
    m.insert(0x40, 5);
    // digit '1'=0x02
    m.insert(0x01, 7);
    m.insert(0x02, 7);
    m
}

fn dummy_en_charmap() -> HashMap<char, u16> {
    let mut m = HashMap::new();
    m.insert(' ', 0x4A);
    m.insert('A', 0x0B);
    m.insert('B', 0x0C);
    m.insert('a', 0x25);
    m.insert('.', 0x3F);
    m.insert('!', 0x40);
    m
}

#[test]
fn test_calc_line_widths_simple_en() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    // "AB" → A(8) + B(8) = 16px, single line
    let lines = calc_line_widths("AB", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].width_px, 16);
    assert_eq!(lines[0].line_num, 1);
}

#[test]
fn test_calc_line_widths_newline() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    // "A{NL}B" → line1 A(8), line2 B(8)
    let lines = calc_line_widths("A{NL}B", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].width_px, 8);
    assert_eq!(lines[0].text, "A");
    assert_eq!(lines[1].width_px, 8);
    assert_eq!(lines[1].text, "B");
}

#[test]
fn test_calc_line_widths_page() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    let lines = calc_line_widths("A{PAGE}B", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, "A");
    assert_eq!(lines[1].text, "B");
}

#[test]
fn test_calc_line_widths_dialog_reset() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    // FF10 resets line
    let lines = calc_line_widths("A{FF10}B", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].width_px, 8);
    assert_eq!(lines[1].width_px, 8);
}

#[test]
fn test_calc_line_widths_korean() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let mut kr_cm: HashMap<char, u16> = HashMap::new();
    kr_cm.insert('\u{AC00}', 0x0100); // 가

    let mut kr_advances: HashMap<u16, u32> = HashMap::new();
    kr_advances.insert(0x0100, 15);

    let tile_adv = build_tile_advance_map(&en_adv, &kr_advances);

    let lines = calc_line_widths("\u{AC00}A", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 1);
    // 가(15) + A(8) = 23
    assert_eq!(lines[0].width_px, 23);
}

#[test]
fn test_calc_line_widths_empty() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    let lines = calc_line_widths("", &en_cm, &tile_adv, &kr_cm);
    assert!(lines.is_empty());
}

#[test]
fn test_calc_line_widths_ctrl_param_number() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    // {FF48:003F} → digit_adv(7)*2 + trailing_tile_3F_adv(4) = 18
    let lines = calc_line_widths("{FF48:003F}", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].width_px, 18);
    assert!(lines[0].text.contains("[NUM+tile]"));
}

#[test]
fn test_calc_line_widths_menu_end_reset() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    // FF9C resets line context
    let lines = calc_line_widths("A{FF9C:0000}B", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_is_line_reset_ctrl_codes() {
    assert!(is_line_reset_ctrl(0xFF04)); // END
    assert!(is_line_reset_ctrl(0xFF10)); // player textbox
    assert!(is_line_reset_ctrl(0xFF14)); // NPC textbox
    assert!(is_line_reset_ctrl(0xFFCC)); // cutscene end
    assert!(is_line_reset_ctrl(0xFFB4)); // screen transition
    assert!(is_line_reset_ctrl(0xFFB8)); // text display start
    assert!(!is_line_reset_ctrl(0xFF30)); // NL — handled separately
    assert!(!is_line_reset_ctrl(0xFF34)); // PAGE — handled separately
    assert!(!is_line_reset_ctrl(0xFF50)); // speaker — no reset
}

#[test]
fn test_is_box_type_code() {
    assert!(is_box_type_code(0xFF10));
    assert!(is_box_type_code(0xFF14));
    assert!(is_box_type_code(0xFF18));
    assert!(is_box_type_code(0xFF1C));
    assert!(is_box_type_code(0xFF2C));
    assert!(is_box_type_code(0xFFB8));
    assert!(!is_box_type_code(0xFF04));
    assert!(!is_box_type_code(0xFF30));
}

#[test]
fn test_box_limit_values() {
    assert_eq!(box_limit(Some(0xFF10)), 133);
    assert_eq!(box_limit(Some(0xFF14)), 133);
    assert_eq!(box_limit(Some(0xFF18)), 137);
    assert_eq!(box_limit(Some(0xFF1C)), 133);
    assert_eq!(box_limit(Some(0xFF2C)), 224);
    assert_eq!(box_limit(Some(0xFFB8)), 133);
    assert_eq!(box_limit(None), DEFAULT_BOX_LIMIT);
}

#[test]
fn test_calc_line_widths_box_type_tracking() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    // FF10 sets box type, then text on new line gets that box type
    let lines = calc_line_widths("{FF10}A{NL}B", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].box_type, Some(0xFF10));
    assert_eq!(lines[1].box_type, Some(0xFF10));

    // FF14 changes box type
    let lines = calc_line_widths("{FF14}A", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].box_type, Some(0xFF14));

    // No box code → None
    let lines = calc_line_widths("AB", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].box_type, None);
}

#[test]
fn test_load_en_advance_table() {
    // Create a minimal fake ROM with just enough data
    let mut rom = vec![0u8; EN_WIDTH_TABLE + MAX_EN_CHARS * 2 + 10];
    // Set space (0x4A) advance to 4
    let space_off = EN_WIDTH_TABLE + 0x4A * 2;
    rom[space_off] = 5; // width
    rom[space_off + 1] = 4; // advance

    // Set A (0x0B) advance to 8
    let a_off = EN_WIDTH_TABLE + 0x0B * 2;
    rom[a_off] = 7;
    rom[a_off + 1] = 8;

    let advance = load_en_advance_table(&rom).unwrap();
    assert_eq!(advance[&0x4A], 4);
    assert_eq!(advance[&0x0B], 8);
}

#[test]
fn test_load_en_advance_table_rom_too_small() {
    let rom = vec![0u8; 100];
    assert!(load_en_advance_table(&rom).is_err());
}

#[test]
fn test_build_tile_advance_map_merge() {
    let mut en = HashMap::new();
    en.insert(0x0B_u16, 8_u32);
    en.insert(0x4A_u16, 4_u32);

    let mut kr = HashMap::new();
    kr.insert(0x0100_u16, 15_u32);
    kr.insert(0x0101_u16, 14_u32);

    let combined = build_tile_advance_map(&en, &kr);
    assert_eq!(combined[&0x0B], 8);
    assert_eq!(combined[&0x4A], 4);
    assert_eq!(combined[&0x0100], 15);
    assert_eq!(combined[&0x0101], 14);
}

#[test]
fn test_calc_line_widths_tile_token() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let mut tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());
    tile_adv.insert(0x76, 16); // {header}

    let lines = calc_line_widths("{header}A", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 1);
    // header(16) + A(8) = 24
    assert_eq!(lines[0].width_px, 24);
}

#[test]
fn test_calc_line_widths_raw_token() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    let lines = calc_line_widths("[004A]", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 1);
    // 0x4A is space → 4px
    assert_eq!(lines[0].width_px, 4);
}

#[test]
fn test_calc_line_widths_end_code() {
    let en_adv = dummy_en_advance();
    let en_cm = dummy_en_charmap();
    let kr_cm: HashMap<char, u16> = HashMap::new();
    let tile_adv = build_tile_advance_map(&en_adv, &HashMap::new());

    // "A{END}" → line recorded then reset
    let lines = calc_line_widths("A{END}", &en_cm, &tile_adv, &kr_cm);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].width_px, 8);
    assert_eq!(lines[0].text, "A");
}
