//! Native fixed-record monster-name table loading, encoding, and verification.

use super::*;

#[derive(Debug)]
pub(super) struct MonsterNameEntry {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) jp: String,
    pub(super) ko: String,
}

#[derive(Debug)]
pub(super) struct EncodedMonsterName {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) words: Vec<u16>,
}

#[derive(Debug)]
pub(super) struct MonsterNameLayout {
    pub(super) entries: Vec<EncodedMonsterName>,
}

pub(super) fn apply_m60_monster_names(
    source: &[u8],
    baseline: Vec<u8>,
    assets_dir: &Path,
    extra_glyphs: &[char],
) -> Result<Vec<u8>, String> {
    let glyphs = collect_scoped_jp_native_glyphs(assets_dir, true, extra_glyphs)?;
    let charmap: BTreeMap<char, u16> = glyphs
        .iter()
        .enumerate()
        .map(|(index, &ch)| (ch, KR_CODE_START + index as u16))
        .collect();
    validate_m60_charmap_matches_baseline(&baseline, assets_dir, &glyphs, &charmap)?;
    let layout = build_m60_monster_name_layout(source, &assets_dir.join("translation"), &charmap)?;

    let mut writes = Vec::with_capacity(layout.entries.len() + 1);
    for entry in &layout.entries {
        let replacement = words_to_bytes(&entry.words);
        writes.push(ExpectedWrite {
            label: format!("M60 JP fixed monster name {}", entry.id),
            offset: entry.offset,
            expected: source[entry.offset..entry.offset + replacement.len()].to_vec(),
            replacement,
        });
    }

    validate_plan(&baseline, &writes)?;
    let mut checksum_stage = baseline.clone();
    apply_plan(&mut checksum_stage, &writes);
    let checksum = calculate_checksum(&checksum_stage);
    writes.push(ExpectedWrite {
        label: format!("Mega Drive checksum -> 0x{checksum:04X}"),
        offset: CHECKSUM_OFFSET,
        expected: baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2].to_vec(),
        replacement: checksum.to_be_bytes().to_vec(),
    });

    validate_plan(&baseline, &writes)?;
    let mut output = baseline.clone();
    apply_plan(&mut output, &writes);
    validate_result(&baseline, &output, &writes)?;
    validate_m60_monster_name_layout(&output, &layout)?;

    eprintln!("JP-native M60 Expected Writes:");
    eprintln!(
        "  monster names: {} unique / 28 live references; {} fixed words each",
        layout.entries.len(),
        JP_MONSTER_NAME_RECORD_WORDS,
    );
    for write in &writes {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {} ({} bytes)",
            write.offset,
            write.offset + write.replacement.len(),
            write.label,
            write.replacement.len(),
        );
    }

    Ok(output)
}

fn validate_m60_charmap_matches_baseline(
    baseline: &[u8],
    assets_dir: &Path,
    glyphs: &[char],
    charmap: &BTreeMap<char, u16>,
) -> Result<(), String> {
    let font = render_native_font(assets_dir, glyphs)?;
    if baseline.get(JP_NATIVE_FONT_BASE..JP_NATIVE_FONT_BASE + font.len()) != Some(font.as_slice())
    {
        return Err(
            "M60 scoped charmap does not match the font installed in its baseline ROM".into(),
        );
    }

    let dispatcher = assemble_jp_native_display_advance(charmap)?;
    if baseline
        .get(JP_NATIVE_DISPLAY_ADVANCE_CODE..JP_NATIVE_DISPLAY_ADVANCE_CODE + dispatcher.len())
        != Some(dispatcher.as_slice())
    {
        return Err(
            "M60 scoped charmap does not match the display dispatcher in its baseline ROM".into(),
        );
    }
    Ok(())
}

pub(super) fn assemble_m60_monster_name_loader() -> Result<Vec<u8>, String> {
    assemble_m68k_at(
        JP_MONSTER_NAME_LOADER as u32,
        &[
            Inst::AndiWordImmediate {
                immediate: 0x001F,
                destination: DataReg::D0,
            },
            Inst::LeaAbsoluteLong {
                address: JP_MONSTER_NAME_TABLE as u32,
                destination: AddressReg::A2,
            },
            Inst::AddWordData {
                source: DataReg::D0,
                destination: DataReg::D0,
            },
            Inst::MoveWordIndexedAddressToData {
                displacement: 0,
                base: AddressReg::A2,
                index: DataReg::D0,
                destination: DataReg::D0,
            },
            Inst::LeaIndexedWord {
                displacement: 0,
                base: AddressReg::A2,
                index: DataReg::D0,
                destination: AddressReg::A2,
            },
            Inst::LeaAbsoluteShort {
                address: JP_MONSTER_NAME_BUFFER,
                destination: AddressReg::A3,
            },
            Inst::ClearWordPostincrementAddress {
                destination: AddressReg::A3,
            },
            Inst::Moveq {
                immediate: JP_MONSTER_NAME_RECORD_WORDS as i8,
                destination: DataReg::D7,
            },
            Inst::Label("copy_monster_name"),
            Inst::MoveWordPostincrementAddressToPostincrementAddress {
                source: AddressReg::A2,
                destination: AddressReg::A3,
            },
            Inst::Dbf {
                register: DataReg::D7,
                target: "copy_monster_name",
            },
            Inst::Rts,
        ],
    )
}

pub(super) fn validate_m60_monster_name_consumer(source: &[u8]) -> Result<(), String> {
    let loader = assemble_m60_monster_name_loader()?;
    if source.get(JP_MONSTER_NAME_LOADER..JP_MONSTER_NAME_LOADER + loader.len())
        != Some(loader.as_slice())
    {
        return Err("M60 typed monster-name loader no longer matches the JP ROM".to_string());
    }

    let calls = m58_direct_jsr_offsets(source, JP_MONSTER_NAME_LOADER)?;
    if calls
        != [
            JP_MONSTER_NAME_PC_RELATIVE_CALLSITE,
            JP_MONSTER_NAME_ABSOLUTE_CALLSITE,
        ]
    {
        return Err(format!(
            "M60 monster-name loader callsites changed: {calls:?}"
        ));
    }

    let table_address = (JP_MONSTER_NAME_TABLE as u32).to_be_bytes();
    let code_refs: Vec<usize> = source[..JP_CODE_SCAN_END]
        .windows(table_address.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == table_address).then_some(offset))
        .collect();
    if code_refs != [JP_MONSTER_NAME_LOADER + 6] {
        return Err(format!(
            "M60 monster-name table code references changed: {code_refs:?}"
        ));
    }

    for (index, &record) in M60_MONSTER_NAME_TARGET_RECORDS.iter().enumerate() {
        let relative = m58_read_word(
            source,
            JP_MONSTER_NAME_TABLE + index * 2,
            "M60 monster-name pointer",
        )?;
        let expected = if record == u8::MAX {
            (JP_MONSTER_NAME_DATA_END - JP_MONSTER_NAME_TABLE) as u16
        } else {
            (JP_MONSTER_NAME_DATA_START - JP_MONSTER_NAME_TABLE
                + usize::from(record) * JP_MONSTER_NAME_RECORD_WORDS * 2) as u16
        };
        if relative != expected {
            return Err(format!(
                "M60 monster pointer {index} changed: 0x{relative:04X} != 0x{expected:04X}"
            ));
        }
    }

    Ok(())
}

pub(super) fn load_m60_monster_name_entries(
    translation_dir: &Path,
) -> Result<Vec<MonsterNameEntry>, String> {
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

    let mut result = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut seen_offsets = BTreeSet::new();
    for path in paths {
        let data = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        let root: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;
        let Some(entries) = root.get("entries").and_then(|value| value.as_array()) else {
            continue;
        };
        for entry in entries {
            if entry.get("section").and_then(|value| value.as_str()) != Some("monster") {
                continue;
            }
            let id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or("M60 monster-name entry is missing id")?
                .to_string();
            let offset = entry
                .get("offset")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("{id}: missing offset"))?;
            let offset = usize::from_str_radix(offset.trim_start_matches("0x"), 16)
                .map_err(|e| format!("{id}: invalid offset: {e}"))?;
            let status = entry
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if status != "done" {
                return Err(format!("{id}: monster-name status is not done: {status}"));
            }
            let jp = entry
                .get("jp")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: JP monster name is empty"))?
                .to_string();
            let ko = entry
                .get("ko")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: KR monster name is empty"))?
                .to_string();
            let old_dialog_id = entry
                .get("old_dialog_id")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let fff8_idx = entry
                .get("fff8_idx")
                .and_then(|value| value.as_u64())
                .ok_or_else(|| format!("{id}: missing FFF8 index"))?;
            let expected_index = result.len();
            let expected_dialog = format!("dialog_{:04}", 1248 + expected_index);
            if old_dialog_id != expected_dialog || fff8_idx != (1248 + expected_index) as u64 {
                return Err(format!("{id}: monster-name EN alignment metadata drifted"));
            }
            if !seen_ids.insert(id.clone()) {
                return Err(format!("{id}: duplicate monster-name ID"));
            }
            if !seen_offsets.insert(offset) {
                return Err(format!("0x{offset:06X}: duplicate monster-name offset"));
            }
            result.push(MonsterNameEntry { id, offset, jp, ko });
        }
    }
    result.sort_by_key(|entry| entry.offset);
    if result.len() != JP_MONSTER_NAME_UNIQUE_COUNT {
        return Err(format!(
            "M60 found {} monster-name assets, expected {JP_MONSTER_NAME_UNIQUE_COUNT}",
            result.len()
        ));
    }
    for (index, entry) in result.iter().enumerate() {
        let expected_id = format!("script_{:04}", 50 + index);
        let expected_offset = JP_MONSTER_NAME_DATA_START + index * JP_MONSTER_NAME_RECORD_WORDS * 2;
        if entry.id != expected_id || entry.offset != expected_offset {
            return Err(format!(
                "M60 monster-name catalog drifted at {index}: {} @ 0x{:06X}",
                entry.id, entry.offset
            ));
        }
    }
    Ok(result)
}

pub(super) fn validate_m60_jp_monster_name_source(
    source: &[u8],
    entry: &MonsterNameEntry,
) -> Result<(), String> {
    if entry.jp.chars().count() != JP_MONSTER_NAME_RECORD_WORDS {
        return Err(format!(
            "{}: JP monster record has {} glyphs, expected {JP_MONSTER_NAME_RECORD_WORDS}",
            entry.id,
            entry.jp.chars().count()
        ));
    }
    if entry.jp.contains('{') || entry.jp.contains('}') {
        return Err(format!("{}: JP monster record contains controls", entry.id));
    }

    let charmap = crate::align::build_jp_charmap();
    let mut decoded = String::new();
    for index in 0..JP_MONSTER_NAME_RECORD_WORDS {
        let word = m58_read_word(
            source,
            entry.offset + index * 2,
            "M60 JP monster-name record",
        )?;
        if word == 0 {
            decoded.push(' ');
        } else {
            decoded.push_str(charmap.get(&word).ok_or_else(|| {
                format!(
                    "{}: JP monster record has unknown glyph 0x{word:04X}",
                    entry.id
                )
            })?);
        }
    }
    if decoded != entry.jp {
        return Err(format!(
            "{}: JP monster asset differs from ROM: {:?} != {:?}",
            entry.id, entry.jp, decoded
        ));
    }
    Ok(())
}

pub(super) fn build_m60_monster_name_layout(
    source: &[u8],
    translation_dir: &Path,
    charmap: &BTreeMap<char, u16>,
) -> Result<MonsterNameLayout, String> {
    validate_m60_monster_name_consumer(source)?;
    let assets = load_m60_monster_name_entries(translation_dir)?;
    let mut entries = Vec::with_capacity(assets.len());
    for entry in assets {
        validate_m60_jp_monster_name_source(source, &entry)?;
        let ko = entry
            .ko
            .strip_suffix("{FFFF}")
            .ok_or_else(|| format!("{}: KR monster name must end in FFFF", entry.id))?;
        let tokens = crate::build::text::parse_display_text(ko);
        if tokens.iter().any(|token| {
            !matches!(
                token,
                Token::KrChar(_) | Token::EnChar(' ') | Token::EnChar('T')
            )
        }) {
            return Err(format!(
                "{}: KR monster name contains a non-name token",
                entry.id
            ));
        }
        let mut words = encode_jp_native_tokens(&tokens, charmap)?;
        if words.is_empty() || words.len() > JP_MONSTER_NAME_RECORD_WORDS {
            return Err(format!(
                "{}: KR monster name uses {} of {JP_MONSTER_NAME_RECORD_WORDS} fixed glyphs",
                entry.id,
                words.len()
            ));
        }
        words.resize(JP_MONSTER_NAME_RECORD_WORDS, 0);
        entries.push(EncodedMonsterName {
            id: entry.id,
            offset: entry.offset,
            words,
        });
    }
    Ok(MonsterNameLayout { entries })
}

pub(super) fn validate_m60_monster_name_layout(
    rom: &[u8],
    layout: &MonsterNameLayout,
) -> Result<(), String> {
    let entries_by_record: BTreeMap<usize, &EncodedMonsterName> =
        layout.entries.iter().enumerate().collect();
    for (record, entry) in &entries_by_record {
        let bytes = words_to_bytes(&entry.words);
        if rom.get(entry.offset..entry.offset + bytes.len()) != Some(bytes.as_slice()) {
            return Err(format!("{}: M60 monster record differs", entry.id));
        }
        let expected_offset =
            JP_MONSTER_NAME_DATA_START + record * JP_MONSTER_NAME_RECORD_WORDS * 2;
        if entry.offset != expected_offset {
            return Err(format!("{}: M60 monster record moved", entry.id));
        }
    }

    for &record in M60_MONSTER_NAME_TARGET_RECORDS
        .iter()
        .filter(|&&record| record != u8::MAX)
    {
        let entry = entries_by_record
            .get(&usize::from(record))
            .ok_or_else(|| format!("M60 pointer targets missing record {record}"))?;
        let mut render_buffer = [0u16; JP_MONSTER_NAME_RECORD_WORDS + 1];
        for (index, slot) in render_buffer[1..].iter_mut().enumerate() {
            *slot = m58_read_word(
                rom,
                entry.offset + index * 2,
                "M60 monster-name render emulation",
            )?;
        }
        if render_buffer[0] != 0 || render_buffer[1..] != entry.words {
            return Err(format!(
                "{}: M60 loader/render buffer emulation differs",
                entry.id
            ));
        }
    }
    Ok(())
}
