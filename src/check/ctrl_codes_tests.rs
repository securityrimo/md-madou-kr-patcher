use super::*;

#[test]
fn test_extract_ctrl_codes_basic() {
    let codes = extract_ctrl_codes("{FF50:001D}{FF10}Hello{NL}{END}");
    assert_eq!(
        codes,
        vec![
            (0xFF50, Some(0x001D)),
            (0xFF10, None),
            (0xFF30, None), // NL
            (0xFF04, None), // END
        ]
    );
}

#[test]
fn test_extract_ctrl_codes_ffcc() {
    let codes = extract_ctrl_codes("{FF60}{FFAC}A{FF10}test{PAGE}{FFCC}{END}");
    assert_eq!(code_count(&codes, 0xFFCC), 1);
    assert_eq!(code_count(&codes, 0xFF60), 1);
    assert_eq!(code_count(&codes, 0xFFAC), 1);
}

#[test]
fn test_extract_ctrl_codes_empty() {
    let codes = extract_ctrl_codes("Hello world");
    assert!(codes.is_empty());
}

#[test]
fn test_code_count() {
    let codes = vec![
        (0xFF30, None),
        (0xFF30, None),
        (0xFF34, None),
        (0xFF30, None),
    ];
    assert_eq!(code_count(&codes, 0xFF30), 3);
    assert_eq!(code_count(&codes, 0xFF34), 1);
    assert_eq!(code_count(&codes, 0xFFCC), 0);
}

#[test]
fn test_code_set() {
    let codes = vec![
        (0xFF84, Some(0x91A4)),
        (0xFF84, Some(0x93BB)),
        (0xFF10, None),
        (0xFF84, Some(0x91A4)),
    ];
    let set = code_set(&codes, 0xFF84);
    assert_eq!(set.len(), 2);
    assert!(set.contains(&(0xFF84, Some(0x91A4))));
    assert!(set.contains(&(0xFF84, Some(0x93BB))));
}

#[test]
fn test_extract_ending_sequence_with_page() {
    let text = "{FF10}Hello{PAGE}{FF64}{FFCC}{END}";
    let ending = extract_ending_sequence(text);
    assert_eq!(ending, "{FF64}{FFCC}{END}");
}

#[test]
fn test_extract_ending_sequence_no_page() {
    let text = "{FFAC}W{FF10}한글은 UTF-8 바이트 길이가 다르다{FF50:000B}{FFFF}";
    let ending = extract_ending_sequence(text);
    assert_eq!(ending, text);
}

#[test]
fn test_missing_ffcc_detection() {
    let en = "{FF60}{FFAC}A{FF10}test{PAGE}{FFCC}{END}";
    let kr = "{FF60}{FFAC}A{FF10}테스트{PAGE}{END}";

    let en_codes = extract_ctrl_codes(en);
    let kr_codes = extract_ctrl_codes(kr);

    let en_ffcc = code_count(&en_codes, 0xFFCC);
    let kr_ffcc = code_count(&kr_codes, 0xFFCC);
    assert!(en_ffcc > kr_ffcc, "should detect missing FFCC");
}

#[test]
fn test_missing_ff84_detection() {
    let en = "{FF84:91A4}{FF84:93BB}{FF10}test{END}";
    let kr = "{FF84:91A4}{FF10}테스트{END}";

    let en_codes = extract_ctrl_codes(en);
    let kr_codes = extract_ctrl_codes(kr);

    let en_set = code_set(&en_codes, 0xFF84);
    let kr_set = code_set(&kr_codes, 0xFF84);
    let missing: HashSet<_> = en_set.difference(&kr_set).copied().collect();

    assert_eq!(missing.len(), 1);
    assert!(missing.contains(&(0xFF84, Some(0x93BB))));
}

#[test]
fn test_no_false_positive_on_equal() {
    let text = "{FF50:001D}{FF10}Hello{NL}{FF84:91A4}{PAGE}{FFCC}{END}";
    let en_codes = extract_ctrl_codes(text);
    let kr_codes = extract_ctrl_codes(text);

    assert_eq!(code_count(&en_codes, 0xFFCC), code_count(&kr_codes, 0xFFCC));
    assert_eq!(code_set(&en_codes, 0xFF84), code_set(&kr_codes, 0xFF84));
}

#[test]
fn control_reference_prefers_jp_and_uses_en_only_as_fallback() {
    let mut entry = translation::PairedEntry {
        file: "script_test.json".to_string(),
        key: "dialog_test".to_string(),
        jp: "{FFAC}JP".to_string(),
        en: "{FF84:1234}EN".to_string(),
        ko: "KR".to_string(),
    };
    assert_eq!(control_reference(&entry), ("JP", "{FFAC}JP"));

    entry.jp.clear();
    assert_eq!(control_reference(&entry), ("EN fallback", "{FF84:1234}EN"));
}
