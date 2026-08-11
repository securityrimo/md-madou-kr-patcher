//! JP source guards, protected-token normalization, and KR native text encoding.

use super::*;

pub(super) struct StableTextEntry {
    pub(super) id: String,
    pub(super) jp: String,
    pub(super) ko: String,
}

pub(super) fn compose_stable_fallthrough_text(
    entry: &StableTextEntry,
    entries: &[StableTextEntry],
) -> Result<(String, String), String> {
    let Some(fallthrough) = M56_STABLE_TEXT_FALLTHROUGH_SPECS
        .iter()
        .chain(M59_STABLE_TEXT_FALLTHROUGH_SPECS.iter())
        .find(|fallthrough| fallthrough.parent_id == entry.id)
    else {
        return Ok((entry.jp.clone(), entry.ko.clone()));
    };
    let target = entries
        .iter()
        .find(|target| target.id == fallthrough.target_id)
        .ok_or_else(|| {
            format!(
                "{}: stable fallthrough target {} is absent",
                entry.id, fallthrough.target_id
            )
        })?;
    Ok((format!("{}{}", entry.jp, target.jp), entry.ko.clone()))
}

pub(super) fn load_stable_text_entries(
    translation_dir: &Path,
    specs: &[StableTextSpec],
) -> Result<Vec<StableTextEntry>, String> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(translation_dir)
        .map_err(|e| format!("failed to read {}: {e}", translation_dir.display()))?
    {
        let path = entry
            .map_err(|e| format!("failed to read translation directory entry: {e}"))?
            .path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with("script_") && name.ends_with(".json") {
            paths.push(path);
        }
    }
    paths.sort();

    let mut matches: Vec<Option<StableTextEntry>> = (0..specs.len()).map(|_| None).collect();
    for path in paths {
        let data = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        let root: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;
        let Some(entries) = root.get("entries").and_then(|value| value.as_array()) else {
            continue;
        };
        for entry in entries {
            let Some(id) = entry.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some((index, spec)) = specs.iter().enumerate().find(|(_, spec)| spec.id == id)
            else {
                continue;
            };
            if matches[index].is_some() {
                return Err(format!("{}: duplicate stable translation ID", spec.id));
            }

            let offset = entry
                .get("offset")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("{}: missing offset", spec.id))?;
            let offset = usize::from_str_radix(offset.trim_start_matches("0x"), 16)
                .map_err(|e| format!("{}: invalid offset: {e}", spec.id))?;
            if offset != spec.offset {
                return Err(format!(
                    "{}: stable offset drifted from 0x{:06X} to 0x{offset:06X}",
                    spec.id, spec.offset
                ));
            }
            let old_dialog_id = entry
                .get("old_dialog_id")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if old_dialog_id != spec.old_dialog_id {
                return Err(format!(
                    "{}: expected old_dialog_id {}, got {old_dialog_id}",
                    spec.id, spec.old_dialog_id
                ));
            }
            let legacy_fff8_idx = entry
                .get("fff8_idx")
                .and_then(|value| value.as_u64())
                .ok_or_else(|| format!("{}: missing legacy fff8_idx", spec.id))?;
            if legacy_fff8_idx != spec.legacy_fff8_idx {
                return Err(format!(
                    "{}: expected legacy fff8_idx {}, got {legacy_fff8_idx}",
                    spec.id, spec.legacy_fff8_idx
                ));
            }
            let status = entry
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if status != "done" {
                return Err(format!("{}: status is not done: {status}", spec.id));
            }
            if let Some(expected_section) = spec.section {
                let section = entry
                    .get("section")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                if section != expected_section {
                    return Err(format!(
                        "{}: expected section {expected_section}, got {section}",
                        spec.id
                    ));
                }
            }
            let jp = entry
                .get("jp")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{}: JP text is empty", spec.id))?;
            let ko = entry
                .get("ko")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{}: KR text is empty", spec.id))?;
            matches[index] = Some(StableTextEntry {
                id: id.to_string(),
                jp: jp.to_string(),
                ko: ko.to_string(),
            });
        }
    }

    matches
        .into_iter()
        .zip(specs)
        .map(|(entry, spec)| {
            entry.ok_or_else(|| format!("{}: stable translation ID was not found", spec.id))
        })
        .collect()
}

pub(super) fn validate_jp_text_source(
    source: &[u8],
    id: &str,
    offset: usize,
    expected_jp: &str,
    context: &str,
) -> Result<Vec<u16>, String> {
    let max_word_count = token_word_len(&parse_jp_protected_text(expected_jp));
    let max_end = offset
        .checked_add(max_word_count * 2)
        .ok_or_else(|| format!("{id}: {context} JP source range overflows"))?;
    let bytes = source
        .get(offset..max_end.min(source.len()))
        .ok_or_else(|| format!("{id}: {context} JP source is truncated"))?;
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    let charmap = crate::align::build_jp_charmap();
    let ctrl_with_param = crate::build::text::ctrl_with_param();
    for word_count in 1..=words.len() {
        let decoded = crate::build::text::words_to_display_text(
            &words[..word_count],
            &charmap,
            &ctrl_with_param,
        )
        .replace('\u{3000}', " ");
        if decoded == expected_jp {
            return Ok(words[..word_count].to_vec());
        }
    }
    let decoded = crate::build::text::words_to_display_text(&words, &charmap, &ctrl_with_param)
        .replace('\u{3000}', " ");
    Err(format!(
        "{id}: {context} JP asset differs from ROM: {expected_jp:?} != {decoded:?}",
    ))
}

fn jp_decimal_digit(ch: char) -> Option<u32> {
    match ch {
        '0'..='9' => ch.to_digit(10),
        '０'..='９' => Some(ch as u32 - '０' as u32),
        _ => None,
    }
}

pub(super) fn visible_jp_digits(text: &str) -> Vec<u32> {
    let mut digits = Vec::new();
    let mut chars = text.chars();
    let mut skip_ffac_placeholder = false;
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let tag = chars
                .by_ref()
                .take_while(|next| *next != '}')
                .collect::<String>();
            if skip_ffac_placeholder {
                skip_ffac_placeholder = false;
            }
            if tag == "FFAC" {
                skip_ffac_placeholder = true;
            }
            continue;
        }
        if ch == '[' {
            chars
                .by_ref()
                .take_while(|next| *next != ']')
                .for_each(drop);
            skip_ffac_placeholder = false;
            continue;
        }
        if skip_ffac_placeholder {
            skip_ffac_placeholder = false;
            continue;
        }
        if let Some(digit) = jp_decimal_digit(ch) {
            digits.push(digit);
        }
    }
    digits
}

pub(super) fn jp_protected_symbol_tile(token: &Token) -> Option<u16> {
    match token {
        Token::EnChar('♥') => Some(JP_HEART_TILE),
        Token::EnChar('♦') => Some(JP_SWEAT1_TILE),
        Token::EnChar('♠') => Some(JP_SWEAT2_TILE),
        Token::Raw(0x0057) => Some(JP_SWEAT3_TILE),
        Token::EnChar('★') => Some(JP_BANDAGE_TILE),
        Token::Tile(0x00E4) => Some(JP_SURPRISE_TILE),
        Token::Tile(0x005B) => Some(JP_BLUSH_TILE),
        Token::Raw(0x005C) => Some(JP_SMALL_STAR_TILE),
        Token::EnChar('◇') => Some(JP_ANGER_TILE),
        Token::Tile(0x0064) => Some(JP_MUSIC_NOTE_TILE),
        Token::Tile(0x006C) => Some(JP_WHITE_STAR_TILE),
        Token::Tile(0x006F) => Some(JP_BLACK_STAR_TILE),
        Token::Tile(0x0050) => Some(JP_SPELL_UP_TILE),
        Token::Tile(0x0051) => Some(JP_SPELL_DOWN_TILE),
        Token::Tile(0x0052) => Some(JP_SPELL_LEFT_TILE),
        Token::Tile(0x0053) => Some(JP_SPELL_RIGHT_TILE),
        _ => None,
    }
}

pub(super) fn is_jp_protected_symbol_tile(code: u16) -> bool {
    matches!(
        code,
        JP_HEART_TILE
            | JP_SWEAT1_TILE
            | JP_SWEAT2_TILE
            | JP_SWEAT3_TILE
            | JP_BANDAGE_TILE
            | JP_SURPRISE_TILE
            | JP_BLUSH_TILE
            | JP_SMALL_STAR_TILE
            | JP_ANGER_TILE
            | JP_MUSIC_NOTE_TILE
            | JP_WHITE_STAR_TILE
            | JP_BLACK_STAR_TILE
            | JP_SPELL_UP_TILE
            | JP_SPELL_DOWN_TILE
            | JP_SPELL_LEFT_TILE
            | JP_SPELL_RIGHT_TILE
    )
}

pub(super) fn parse_jp_protected_text(text: &str) -> Vec<Token> {
    let mut canonical = text.to_string();
    for (symbol, code) in [
        ("[heart]", 0x00DDu16),
        ("[sweat1]", 0x00DE),
        ("[sweat2]", 0x00DF),
        ("[sweat3]", 0x00E0),
        ("[bandage]", 0x00E2),
        ("[2hearts]", 0x00E3),
        ("[surprise]", 0x00E4),
        ("[blush]", 0x00E5),
        ("[smallstar]", 0x00E6),
        ("[carbuncle]", 0x00E7),
        ("[anger]", 0x00EA),
        ("[musicnote]", 0x00F1),
        ("[:D]", 0x00EBu16),
        ("[:)]", 0x00EC),
        ("[:|]", 0x00ED),
        ("[:(]", 0x00EE),
        ("[>:(]", 0x00EF),
    ] {
        canonical = canonical.replace(symbol, &format!("[{code:04X}]"));
    }
    crate::build::text::parse_display_text(&canonical)
}

fn control_sequence(text: &str) -> Vec<(u16, Option<u16>)> {
    crate::build::text::parse_display_text(text)
        .into_iter()
        .filter_map(|token| match token {
            Token::Ctrl(code) => Some((code, None)),
            Token::CtrlParam(code, parameter) => Some((code, Some(parameter))),
            _ => None,
        })
        .collect()
}

pub(super) fn require_matching_control_sequence(
    jp: &str,
    ko: &str,
    id: &str,
) -> Result<(), String> {
    let jp_controls = control_sequence(jp);
    let ko_controls = control_sequence(ko);
    if jp_controls != ko_controls {
        return Err(format!(
            "{id}: JP/KR control sequence mismatch: {jp_controls:?} != {ko_controls:?}"
        ));
    }
    Ok(())
}

pub(super) fn leading_control_count(tokens: &[Token]) -> usize {
    tokens
        .iter()
        .take_while(|token| matches!(token, Token::Ctrl(_) | Token::CtrlParam(_, _)))
        .count()
}

pub(super) fn token_word_len(tokens: &[Token]) -> usize {
    tokens
        .iter()
        .map(|token| match token {
            Token::CtrlParam(_, _) => 2,
            Token::SourceRowFinalize { .. } => 3,
            _ => 1,
        })
        .sum()
}

pub(super) fn normalize_m2_ellipsis(tokens: &[Token]) -> Vec<Token> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens.get(index..index + 3)
            == Some(&[Token::EnChar('.'), Token::EnChar('.'), Token::EnChar('.')])
        {
            normalized.push(Token::EnChar('…'));
            index += 3;
        } else {
            normalized.push(tokens[index].clone());
            index += 1;
        }
    }
    normalized
}

pub(super) fn validate_m2_fixed_width_lines(tokens: &[Token], id: &str) -> Result<(), String> {
    validate_dynamic_display_population(id, tokens)?;
    validate_fixed_width_layout(
        tokens,
        id,
        M2_MAX_FIXED_GLYPHS_PER_LINE,
        Some(M2_MAX_LINES_PER_PAGE),
    )
}

pub(super) fn encode_jp_native_tokens(
    tokens: &[Token],
    charmap: &BTreeMap<char, u16>,
) -> Result<Vec<u16>, String> {
    let mut words = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let token = &tokens[index];
        match token {
            Token::Ctrl(code) => words.push(*code),
            Token::CtrlParam(code, parameter) => {
                if *code == 0xFFF8 {
                    return Err("JP-native payload cannot recursively redirect through FFF8".into());
                }
                let parameter =
                    if *parameter == 0 && DynamicDisplayControl::from_code(*code).is_some() {
                        *charmap.get(&' ').ok_or(
                            "JP-native dynamic blank suffix requires the dedicated space glyph",
                        )?
                    } else {
                        *parameter
                    };
                words.extend_from_slice(&[*code, parameter]);
            }
            Token::KrChar(ch) => words.push(*charmap.get(ch).ok_or_else(|| {
                format!("JP-native glyph is missing: '{ch}' (U+{:04X})", *ch as u32)
            })?),
            Token::EnChar(ch) => {
                if let Some(&code) = charmap.get(ch) {
                    words.push(code);
                } else if *ch == ' ' {
                    return Err(
                        "JP-native ordinary space requires the dedicated space glyph".into(),
                    );
                } else if let Some(digit) = ch.to_digit(10) {
                    words.push(0x0001 + digit as u16);
                } else {
                    return Err(format!(
                        "unsupported JP-native non-Hangul character: '{ch}' (U+{:04X})",
                        *ch as u32
                    ));
                }
            }
            Token::LayoutPad => {
                words.push(JP_NATIVE_FULL_WIDTH_LAYOUT_PAD);
            }
            Token::SourceRowFinalize { clear_half_cells } => {
                let clear_half_cells = u16::try_from(*clear_half_cells)
                    .map_err(|_| "JP-native source-row clear exceeds 65535 half cells")?;
                words.extend_from_slice(&[0xFFF8, JP_TEXT_ROW_FINALIZE_MAGIC, clear_half_cells]);
            }
            Token::Tile(code) => {
                return Err(format!(
                    "EN-derived named tile 0x{code:04X} is not accepted by the JP-native encoder"
                ));
            }
            Token::Raw(code)
                if (JP_FACE_TILE_START..=JP_FACE_TILE_END).contains(code)
                    || is_jp_protected_symbol_tile(*code)
                    || *code == JP_PERCENT_TILE
                    || matches!(
                        *code,
                        JP_ITEM_QUOTED_OPEN | JP_ITEM_QUOTED_CLOSE | JP_QUOTE_OPEN | JP_QUOTE_CLOSE
                    ) =>
            {
                words.push(*code);
            }
            Token::Raw(code) => {
                return Err(format!(
                    "raw text word 0x{code:04X} is not accepted by the JP-native encoder"
                ));
            }
        }
        index += 1;
    }
    Ok(words)
}

pub(super) fn validate_stable_text_layout(
    rom: &[u8],
    text_specs: &[StableTextSpec],
    text_payload_base: usize,
    encoded_entries: &[Vec<u8>],
) -> Result<(), String> {
    let handler_pointer = u32::from_be_bytes(
        rom[JP_TEXT_OPCODE_HANDLER_SLOT..JP_TEXT_OPCODE_HANDLER_SLOT + 4]
            .try_into()
            .map_err(|_| "M2 handler pointer is truncated")?,
    );
    if handler_pointer as usize != JP_TEXT_REDIRECT_HANDLER {
        return Err("stable-text handler pointer does not target the typed handler".to_string());
    }

    let mut expected_address = text_payload_base;
    for (local_id, (spec, encoded)) in text_specs.iter().zip(encoded_entries).enumerate() {
        let redirect = &rom[spec.redirect_offset..spec.redirect_offset + 6];
        let expected_redirect = [
            0xFF,
            0xF8,
            (JP_TEXT_REDIRECT_MAGIC >> 8) as u8,
            JP_TEXT_REDIRECT_MAGIC as u8,
            (local_id >> 8) as u8,
            local_id as u8,
        ];
        if redirect != expected_redirect {
            return Err(format!("{}: redirect verification failed", spec.id));
        }
        let pointer_offset = JP_TEXT_POINTER_TABLE + local_id * 4;
        let pointer = u32::from_be_bytes(
            rom[pointer_offset..pointer_offset + 4]
                .try_into()
                .map_err(|_| format!("{}: pointer is truncated", spec.id))?,
        ) as usize;
        if pointer != expected_address {
            return Err(format!(
                "{}: pointer expected 0x{expected_address:06X}, got 0x{pointer:06X}",
                spec.id
            ));
        }
        if rom.get(pointer..pointer + encoded.len()) != Some(encoded.as_slice()) {
            return Err(format!("{}: re-extracted payload differs", spec.id));
        }
        expected_address += encoded.len();
    }
    Ok(())
}

pub(super) fn encode_m1_showcase(charmap: &BTreeMap<char, u16>) -> Result<[u8; 16], String> {
    let mut words = Vec::with_capacity(8);
    for ch in "한글직결".chars() {
        let code = charmap
            .get(&ch)
            .ok_or_else(|| format!("M1 showcase glyph is missing from translations: {ch}"))?;
        words.push(*code);
    }
    words.extend_from_slice(&[0x0080, 0xFFB4, 0xFF64, 0xFF04]);

    let mut bytes = [0u8; 16];
    for (index, word) in words.into_iter().enumerate() {
        bytes[index * 2..index * 2 + 2].copy_from_slice(&word.to_be_bytes());
    }
    Ok(bytes)
}
