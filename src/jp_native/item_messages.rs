//! Native M6 item descriptions and M7/M8 item-use message tables.

use super::*;

#[derive(Debug)]
pub(super) struct ItemDescEntry {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) jp: String,
    pub(super) ko: String,
}

#[derive(Debug)]
pub(super) struct EncodedItemDesc {
    pub(super) id: String,
    pub(super) old_offset: usize,
    pub(super) new_offset: usize,
    pub(super) words: Vec<u16>,
}

#[derive(Debug)]
pub(super) struct ItemDescWrite {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) replacement: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct ItemDescLayout {
    pub(super) table: Vec<u8>,
    pub(super) dependent_item_use2_table: Vec<u8>,
    pub(super) writes: Vec<ItemDescWrite>,
    pub(super) entries: Vec<EncodedItemDesc>,
    pub(super) source_bytes: usize,
    pub(super) payload_bytes: usize,
}

#[derive(Debug, Clone)]
pub(super) struct ItemUseEntry {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) jp: String,
    pub(super) ko: String,
}

#[derive(Debug)]
pub(super) struct EncodedItemUse {
    pub(super) id: String,
    pub(super) old_offset: usize,
    pub(super) new_offset: usize,
    pub(super) words: Vec<u16>,
}

#[derive(Debug)]
pub(super) struct ItemUseWrite {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) replacement: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct ItemUseLayout {
    pub(super) table: Vec<u8>,
    pub(super) writes: Vec<ItemUseWrite>,
    pub(super) entries: Vec<EncodedItemUse>,
    pub(super) source_bytes: usize,
    pub(super) payload_bytes: usize,
}

#[derive(Debug)]
pub(super) struct ItemUse2Layout {
    pub(super) table: Vec<u8>,
    pub(super) writes: Vec<ItemUseWrite>,
    pub(super) entries: Vec<EncodedItemUse>,
    pub(super) source_bytes: usize,
    pub(super) payload_bytes: usize,
    pub(super) shared_old_target: usize,
    pub(super) shared_new_target: usize,
    pub(super) shared_words: Vec<u16>,
}

pub(super) fn build_m6_item_desc_layout(
    source: &[u8],
    translation_dir: &Path,
    charmap: &BTreeMap<char, u16>,
) -> Result<ItemDescLayout, String> {
    let table_len = JP_ITEM_DESC_TABLE_COUNT * 2;
    let table_source = source
        .get(JP_ITEM_DESC_TABLE..JP_ITEM_DESC_TABLE + table_len)
        .ok_or("M6 item-description pointer table is outside the source ROM")?;
    let old_targets: Vec<usize> = table_source
        .chunks_exact(2)
        .map(|pair| JP_ITEM_DESC_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize)
        .collect();
    let unique_old_targets: BTreeSet<usize> = old_targets.iter().copied().collect();
    if unique_old_targets.len() != JP_ITEM_DESC_TOTAL_UNIQUE_TARGET_COUNT {
        return Err(format!(
            "M6 item-description source has {} unique targets, expected {JP_ITEM_DESC_TOTAL_UNIQUE_TARGET_COUNT}",
            unique_old_targets.len()
        ));
    }

    let unused_target = *old_targets
        .get(JP_ITEM_DESC_UNUSED_INDEX)
        .ok_or("M6 item-description unused index is outside the pointer table")?;
    if unused_target != JP_ITEM_DESC_UNUSED_TARGET {
        return Err(format!(
            "M6 item-description unused index {} targets 0x{unused_target:06X}, expected 0x{JP_ITEM_DESC_UNUSED_TARGET:06X}",
            JP_ITEM_DESC_UNUSED_INDEX
        ));
    }

    let mut asset_entries = load_m6_item_desc_entries(translation_dir)?;
    let shared_event = load_stable_text_entries(translation_dir, &M68_ITEM_EVENT_TEXT_SPECS[..1])?
        .into_iter()
        .next()
        .ok_or("M6 shared item-description event is missing")?;
    if shared_event.id != JP_ITEM_DESC_SHARED_EVENT_ID {
        return Err(format!(
            "M6 shared item-description event is {}, expected {} at 0x{:06X}",
            shared_event.id, JP_ITEM_DESC_SHARED_EVENT_ID, JP_ITEM_DESC_SHARED_EVENT_OFFSET
        ));
    }
    asset_entries.push(ItemDescEntry {
        id: shared_event.id,
        offset: JP_ITEM_DESC_SHARED_EVENT_OFFSET,
        jp: shared_event.jp,
        ko: shared_event.ko,
    });
    asset_entries.sort_by_key(|entry| entry.offset);
    let asset_offsets: BTreeSet<usize> = asset_entries.iter().map(|entry| entry.offset).collect();
    let owned_old_targets = unique_old_targets
        .iter()
        .copied()
        .filter(|target| *target != JP_ITEM_DESC_UNUSED_TARGET)
        .collect::<BTreeSet<_>>();
    if owned_old_targets.len() != JP_ITEM_DESC_OWNED_UNIQUE_COUNT {
        return Err(format!(
            "M6 item-description source has {} owned unique targets, expected {JP_ITEM_DESC_OWNED_UNIQUE_COUNT}",
            owned_old_targets.len()
        ));
    }
    if asset_offsets != owned_old_targets {
        return Err(
            "M6 item-description asset offsets do not match the owned JP pointer population"
                .to_string(),
        );
    }

    let mut source_slots = Vec::with_capacity(asset_entries.len());
    let mut encoded_entries = Vec::with_capacity(asset_entries.len());
    for entry in asset_entries {
        let source_words = validate_m6_jp_item_desc_source(source, &entry)?;
        source_slots.push((entry.offset, source_words.len()));

        let tokens = normalize_m6_item_desc_tokens(
            &entry.id,
            &crate::build::text::parse_display_text(&entry.ko),
        )?;
        validate_m2_fixed_width_lines(&tokens, &entry.id)?;
        let words = encode_jp_native_tokens(&tokens, charmap)?;
        validate_m6_item_desc_words(&entry.id, &words)?;
        encoded_entries.push(EncodedItemDesc {
            id: entry.id,
            old_offset: entry.offset,
            new_offset: 0,
            words,
        });
    }

    let shared_slot_index = source_slots
        .iter()
        .position(|(offset, _)| *offset == JP_ITEM_DESC_SHARED_EVENT_OFFSET)
        .ok_or("M6 shared item-description source slot is missing")?;
    let (shared_slot_offset, shared_slot_words) = source_slots.remove(shared_slot_index);
    let shared_entry_index = encoded_entries
        .iter()
        .position(|entry| entry.id == JP_ITEM_DESC_SHARED_EVENT_ID)
        .ok_or("M6 shared item-description encoded entry is missing")?;
    let mut shared_entry = encoded_entries.remove(shared_entry_index);
    if shared_entry.words.len() > shared_slot_words {
        return Err(format!(
            "{}: {} encoded words do not fit its shared M6 source slot of {} words",
            shared_entry.id,
            shared_entry.words.len(),
            shared_slot_words
        ));
    }
    shared_entry.new_offset = shared_slot_offset;

    source_slots.sort_by_key(|&(offset, words)| (std::cmp::Reverse(words), offset));
    encoded_entries.sort_by_key(|entry| {
        (
            std::cmp::Reverse(entry.words.len()),
            entry.old_offset,
            entry.id.clone(),
        )
    });
    for (entry, &(slot_offset, slot_words)) in encoded_entries.iter_mut().zip(source_slots.iter()) {
        if entry.words.len() > slot_words {
            return Err(format!(
                "{}: {} encoded words do not fit the next M6 source slot of {} words",
                entry.id,
                entry.words.len(),
                slot_words
            ));
        }
        entry.new_offset = slot_offset;
    }
    source_slots.push((shared_slot_offset, shared_slot_words));
    encoded_entries.push(shared_entry);
    encoded_entries.sort_by_key(|entry| entry.old_offset);

    let new_offsets: BTreeMap<usize, usize> = encoded_entries
        .iter()
        .map(|entry| (entry.old_offset, entry.new_offset))
        .collect();
    let mut table = Vec::with_capacity(table_len);
    for (index, old_target) in old_targets.into_iter().enumerate() {
        let new_target = if index == JP_ITEM_DESC_UNUSED_INDEX {
            if old_target != JP_ITEM_DESC_UNUSED_TARGET {
                return Err(format!(
                    "M6 item-description unused index {index} changed target to 0x{old_target:06X}"
                ));
            }
            old_target
        } else {
            *new_offsets.get(&old_target).ok_or_else(|| {
                format!("M6 item-description index {index} target 0x{old_target:06X} has no asset")
            })?
        };
        let relative = new_target
            .checked_sub(JP_ITEM_DESC_TABLE)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| format!("M6 item-description target 0x{new_target:06X} is not rel16"))?;
        table.extend_from_slice(&relative.to_be_bytes());
    }

    let dependent_table_len = JP_ITEM_USE2_TABLE_COUNT * 2;
    let dependent_table_source = source
        .get(JP_ITEM_USE2_TABLE..JP_ITEM_USE2_TABLE + dependent_table_len)
        .ok_or("M6 dependent battle-use pointer table is outside the source ROM")?;
    let mut dependent_item_use2_table = Vec::with_capacity(dependent_table_len);
    let mut shared_targets = BTreeSet::new();
    for pair in dependent_table_source.chunks_exact(2) {
        let old_target = JP_ITEM_USE2_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize;
        let new_target = new_offsets.get(&old_target).copied().unwrap_or(old_target);
        if new_offsets.contains_key(&old_target) {
            shared_targets.insert(old_target);
        }
        let relative = new_target
            .checked_sub(JP_ITEM_USE2_TABLE)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| {
                format!("M6 dependent battle-use target 0x{new_target:06X} is not rel16")
            })?;
        dependent_item_use2_table.extend_from_slice(&relative.to_be_bytes());
    }
    if shared_targets.len() != JP_ITEM_DESC_USE2_SHARED_TARGET_COUNT {
        return Err(format!(
            "M6 found {} item-description targets shared by battle use, expected {JP_ITEM_DESC_USE2_SHARED_TARGET_COUNT}",
            shared_targets.len()
        ));
    }

    let mut writes: Vec<ItemDescWrite> = encoded_entries
        .iter()
        .map(|entry| ItemDescWrite {
            id: entry.id.clone(),
            offset: entry.new_offset,
            replacement: words_to_bytes(&entry.words),
        })
        .collect();
    writes.sort_by_key(|write| write.offset);
    for pair in writes.windows(2) {
        let left_end = pair[0].offset + pair[0].replacement.len();
        if left_end > pair[1].offset {
            return Err(format!(
                "M6 item-description writes overlap at 0x{left_end:06X}"
            ));
        }
    }

    Ok(ItemDescLayout {
        table,
        dependent_item_use2_table,
        source_bytes: source_slots.iter().map(|(_, words)| words * 2).sum(),
        payload_bytes: encoded_entries
            .iter()
            .map(|entry| entry.words.len() * 2)
            .sum(),
        writes,
        entries: encoded_entries,
    })
}

pub(super) fn load_m6_item_desc_entries(
    translation_dir: &Path,
) -> Result<Vec<ItemDescEntry>, String> {
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
            if entry.get("section").and_then(|value| value.as_str()) != Some("item_desc") {
                continue;
            }
            let id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or("M6 item-description entry is missing id")?
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
                return Err(format!(
                    "{id}: item-description status is not done: {status}"
                ));
            }
            let jp = entry
                .get("jp")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: JP item description is empty"))?
                .to_string();
            let ko = entry
                .get("ko")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: KR item description is empty"))?
                .to_string();
            if !seen_ids.insert(id.clone()) {
                return Err(format!("{id}: duplicate item-description ID"));
            }
            if !seen_offsets.insert(offset) {
                return Err(format!("0x{offset:06X}: duplicate item-description offset"));
            }
            result.push(ItemDescEntry { id, offset, jp, ko });
        }
    }
    result.sort_by_key(|entry| entry.offset);
    if result.len() != JP_ITEM_DESC_UNIQUE_COUNT {
        return Err(format!(
            "M6 found {} item-description assets, expected {JP_ITEM_DESC_UNIQUE_COUNT}",
            result.len()
        ));
    }
    Ok(result)
}

pub(super) fn validate_m6_jp_item_desc_source(
    source: &[u8],
    entry: &ItemDescEntry,
) -> Result<Vec<u16>, String> {
    let ctrl_with_param = HashSet::new();
    let block_end = HashSet::from([0xFFFF]);
    let words =
        crate::build::text::read_rom_words(source, entry.offset, &ctrl_with_param, &block_end);
    if words.first() != Some(&0xFF10)
        || words.get(words.len().saturating_sub(2)) != Some(&0xFF34)
        || words.last() != Some(&0xFFFF)
    {
        return Err(format!(
            "{}: JP item-description wrapper differs from FF10/.../PAGE/FFFF",
            entry.id
        ));
    }
    if words
        .iter()
        .any(|word| *word >= 0xFF00 && !matches!(*word, 0xFF10 | 0xFF30 | 0xFF34 | 0xFFFF))
    {
        return Err(format!(
            "{}: JP item description contains an unsupported control",
            entry.id
        ));
    }
    let decoded = crate::build::text::words_to_display_text(
        &words,
        &crate::align::build_jp_charmap(),
        &ctrl_with_param,
    );
    if decoded != entry.jp {
        return Err(format!(
            "{}: JP asset differs from ROM: {:?} != {decoded:?}",
            entry.id, entry.jp
        ));
    }
    Ok(words)
}

pub(super) fn normalize_m6_item_desc_tokens(
    id: &str,
    tokens: &[Token],
) -> Result<Vec<Token>, String> {
    if tokens.len() < 4
        || tokens.first() != Some(&Token::Ctrl(0xFF10))
        || tokens.get(tokens.len() - 2) != Some(&Token::Ctrl(0xFF34))
        || tokens.last() != Some(&Token::Ctrl(0xFFFF))
    {
        return Err(format!(
            "{id}: KR item description must use FF10/.../PAGE/FFFF wrapper"
        ));
    }
    for token in tokens {
        match token {
            Token::Ctrl(0xFF10 | 0xFF30 | 0xFF34 | 0xFFFF)
            | Token::KrChar(_)
            | Token::EnChar(_) => {}
            Token::Ctrl(code) => {
                return Err(format!(
                    "{id}: unsupported item-description control 0x{code:04X}"
                ));
            }
            Token::CtrlParam(code, _) => {
                return Err(format!(
                    "{id}: parameterized control 0x{code:04X} is not valid in item descriptions"
                ));
            }
            Token::Tile(code) => {
                return Err(format!(
                    "{id}: named tile 0x{code:04X} is not valid in item descriptions"
                ));
            }
            Token::LayoutPad | Token::SourceRowFinalize { .. } => {
                return Err(format!(
                    "{id}: source-backed layout padding is not valid in item descriptions"
                ));
            }
            Token::Raw(code) => {
                return Err(format!(
                    "{id}: raw text word 0x{code:04X} is not valid in item descriptions"
                ));
            }
        }
    }

    let mut normalized = tokens.to_vec();
    loop {
        let mut line_start = 0usize;
        let mut wrapped = false;
        while line_start < normalized.len() {
            let line_end = normalized[line_start..]
                .iter()
                .position(|token| matches!(token, Token::Ctrl(0xFF30 | 0xFF34 | 0xFFFF)))
                .map(|relative| line_start + relative)
                .unwrap_or(normalized.len());
            let visible_half_cells = normalized[line_start..line_end]
                .iter()
                .map(fixed_width_token_half_cells)
                .sum::<usize>();
            if visible_half_cells > JP_ITEM_DESC_MAX_GLYPHS_PER_LINE * 2 {
                let split = normalized[line_start..line_end]
                    .iter()
                    .rposition(|token| *token == Token::EnChar(' '))
                    .map(|relative| line_start + relative)
                    .ok_or_else(|| {
                        format!(
                            "{id}: item-description line exceeds {JP_ITEM_DESC_MAX_GLYPHS_PER_LINE} cells and has no safe word boundary"
                        )
                    })?;
                normalized[split] = Token::Ctrl(0xFF30);
                wrapped = true;
                break;
            }
            if line_end == normalized.len() {
                break;
            }
            line_start = line_end + 1;
        }
        if !wrapped {
            break;
        }
    }
    Ok(normalized)
}

pub(super) fn validate_m6_item_desc_words(id: &str, words: &[u16]) -> Result<(), String> {
    if words.len() < 4
        || words.first() != Some(&0xFF10)
        || words.get(words.len() - 2) != Some(&0xFF34)
        || words.last() != Some(&0xFFFF)
    {
        return Err(format!("{id}: encoded item-description wrapper is invalid"));
    }
    if words
        .iter()
        .any(|word| *word >= 0xFF00 && !matches!(*word, 0xFF10 | 0xFF30 | 0xFF34 | 0xFFFF))
    {
        return Err(format!(
            "{id}: encoded item description contains an unsupported control"
        ));
    }
    Ok(())
}

pub(super) fn validate_m6_item_desc_layout(
    rom: &[u8],
    layout: &ItemDescLayout,
    validate_dependent_item_use2_table: bool,
) -> Result<(), String> {
    if rom.get(JP_ITEM_DESC_TABLE..JP_ITEM_DESC_TABLE + layout.table.len())
        != Some(layout.table.as_slice())
    {
        return Err("M6 item-description table re-extraction differs".to_string());
    }
    let entries_by_offset: BTreeMap<usize, &EncodedItemDesc> = layout
        .entries
        .iter()
        .map(|entry| (entry.new_offset, entry))
        .collect();
    let mut referenced_offsets = BTreeSet::new();
    for (index, pair) in layout.table.chunks_exact(2).enumerate() {
        let target = JP_ITEM_DESC_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize;
        if index == JP_ITEM_DESC_UNUSED_INDEX {
            if target != JP_ITEM_DESC_UNUSED_TARGET {
                return Err(format!(
                    "M6 item-description unused index {index} targets 0x{target:06X}, expected 0x{JP_ITEM_DESC_UNUSED_TARGET:06X}"
                ));
            }
            continue;
        }
        let entry = entries_by_offset
            .get(&target)
            .ok_or_else(|| format!("M6 item-description pointer targets unknown 0x{target:06X}"))?;
        referenced_offsets.insert(target);
        let expected = words_to_bytes(&entry.words);
        if rom.get(target..target + expected.len()) != Some(expected.as_slice()) {
            return Err(format!(
                "{}: item-description direct-consumer re-extraction differs",
                entry.id
            ));
        }
    }
    if referenced_offsets.len() != layout.entries.len() {
        return Err("M6 item-description table does not reference every packed entry".to_string());
    }
    if validate_dependent_item_use2_table
        && rom.get(JP_ITEM_USE2_TABLE..JP_ITEM_USE2_TABLE + layout.dependent_item_use2_table.len())
            != Some(layout.dependent_item_use2_table.as_slice())
    {
        return Err("M6 dependent battle-use table re-extraction differs".to_string());
    }
    Ok(())
}

pub(super) fn build_m7_item_use_layout(
    source: &[u8],
    translation_dir: &Path,
    charmap: &BTreeMap<char, u16>,
) -> Result<ItemUseLayout, String> {
    let table_len = JP_ITEM_USE_TABLE_COUNT * 2;
    let table_source = source
        .get(JP_ITEM_USE_TABLE..JP_ITEM_USE_TABLE + table_len)
        .ok_or("M7 ordinary item-use pointer table is outside the source ROM")?;
    let old_targets: Vec<usize> = table_source
        .chunks_exact(2)
        .map(|pair| JP_ITEM_USE_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize)
        .collect();
    let unique_old_targets: BTreeSet<usize> = old_targets.iter().copied().collect();
    if unique_old_targets.len() != JP_ITEM_USE_UNIQUE_COUNT {
        return Err(format!(
            "M7 ordinary item-use source has {} unique targets, expected {JP_ITEM_USE_UNIQUE_COUNT}",
            unique_old_targets.len()
        ));
    }

    let asset_entries = load_item_use_entries(
        translation_dir,
        "item_use",
        JP_ITEM_USE_UNIQUE_COUNT,
        "M7 ordinary item-use",
    )?;
    let asset_offsets: BTreeSet<usize> = asset_entries.iter().map(|entry| entry.offset).collect();
    if asset_offsets != unique_old_targets {
        return Err(
            "M7 ordinary item-use asset offsets do not match the JP pointer population".to_string(),
        );
    }

    let all_item_targets = collect_item_text_targets(source)?;
    let mut source_slots = Vec::with_capacity(asset_entries.len());
    let mut encoded_entries = Vec::with_capacity(asset_entries.len());
    for entry in asset_entries {
        let source_words = validate_jp_item_use_source(source, &entry, &entry.jp, "M7")?;
        let source_end = entry.offset + source_words.len() * 2;
        if let Some(interior_target) = all_item_targets.range(entry.offset + 2..source_end).next() {
            return Err(format!(
                "{}: item text target 0x{interior_target:06X} points inside its source slot",
                entry.id
            ));
        }
        source_slots.push((entry.offset, source_words.len()));

        let mut tokens = normalize_m7_item_use_tokens(
            &entry.id,
            &entry.jp,
            &source_words,
            &crate::build::text::parse_display_text(&entry.ko),
        )?;
        localize_dynamic_trailing_tiles(&entry.id, &mut tokens, charmap)?;
        validate_m2_fixed_width_lines(&tokens, &entry.id)?;
        let words = encode_m7_item_use_tokens(&entry.id, &tokens, charmap)?;
        encoded_entries.push(EncodedItemUse {
            id: entry.id,
            old_offset: entry.offset,
            new_offset: 0,
            words,
        });
    }

    source_slots.sort_by_key(|&(offset, words)| (std::cmp::Reverse(words), offset));
    encoded_entries.sort_by_key(|entry| {
        (
            std::cmp::Reverse(entry.words.len()),
            entry.old_offset,
            entry.id.clone(),
        )
    });
    let overflows: Vec<String> = encoded_entries
        .iter()
        .zip(source_slots.iter())
        .filter(|(entry, (_, slot_words))| entry.words.len() > *slot_words)
        .map(|(entry, (_, slot_words))| format!("{} {}>{slot_words}", entry.id, entry.words.len()))
        .collect();
    if !overflows.is_empty() {
        return Err(format!(
            "M7 ordinary item-use slot capacity failed: {}",
            overflows.join(", ")
        ));
    }
    for (entry, &(slot_offset, _)) in encoded_entries.iter_mut().zip(source_slots.iter()) {
        entry.new_offset = slot_offset;
    }
    encoded_entries.sort_by_key(|entry| entry.old_offset);

    let new_offsets: BTreeMap<usize, usize> = encoded_entries
        .iter()
        .map(|entry| (entry.old_offset, entry.new_offset))
        .collect();
    let mut table = Vec::with_capacity(table_len);
    for old_target in old_targets {
        let new_target = *new_offsets.get(&old_target).ok_or_else(|| {
            format!("M7 ordinary item-use target 0x{old_target:06X} has no asset")
        })?;
        let relative = new_target
            .checked_sub(JP_ITEM_USE_TABLE)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| {
                format!("M7 ordinary item-use target 0x{new_target:06X} is not rel16")
            })?;
        table.extend_from_slice(&relative.to_be_bytes());
    }

    let mut writes: Vec<ItemUseWrite> = encoded_entries
        .iter()
        .map(|entry| ItemUseWrite {
            id: entry.id.clone(),
            offset: entry.new_offset,
            replacement: words_to_bytes(&entry.words),
        })
        .collect();
    writes.sort_by_key(|write| write.offset);
    for pair in writes.windows(2) {
        let left_end = pair[0].offset + pair[0].replacement.len();
        if left_end > pair[1].offset {
            return Err(format!(
                "M7 ordinary item-use writes overlap at 0x{left_end:06X}"
            ));
        }
    }

    Ok(ItemUseLayout {
        table,
        source_bytes: source_slots.iter().map(|(_, words)| words * 2).sum(),
        payload_bytes: encoded_entries
            .iter()
            .map(|entry| entry.words.len() * 2)
            .sum(),
        writes,
        entries: encoded_entries,
    })
}

pub(super) fn load_item_use_entries(
    translation_dir: &Path,
    section: &str,
    expected_count: usize,
    context: &str,
) -> Result<Vec<ItemUseEntry>, String> {
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
            if entry.get("section").and_then(|value| value.as_str()) != Some(section) {
                continue;
            }
            let id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("{context} entry is missing id"))?
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
                return Err(format!("{id}: {context} status is not done: {status}"));
            }
            let jp = entry
                .get("jp")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: JP {context} text is empty"))?
                .to_string();
            let ko = entry
                .get("ko")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: KR {context} text is empty"))?
                .to_string();
            if !seen_ids.insert(id.clone()) {
                return Err(format!("{id}: duplicate {context} ID"));
            }
            if !seen_offsets.insert(offset) {
                return Err(format!("0x{offset:06X}: duplicate {context} offset"));
            }
            result.push(ItemUseEntry { id, offset, jp, ko });
        }
    }
    result.sort_by_key(|entry| entry.offset);
    if result.len() != expected_count {
        return Err(format!(
            "found {} {context} assets, expected {expected_count}",
            result.len(),
        ));
    }
    Ok(result)
}

pub(super) fn collect_item_text_targets(source: &[u8]) -> Result<BTreeSet<usize>, String> {
    let mut targets = BTreeSet::new();
    for &(table, count) in &[
        (JP_ITEM_DESC_TABLE, JP_ITEM_DESC_TABLE_COUNT),
        (JP_ITEM_USE_TABLE, JP_ITEM_USE_TABLE_COUNT),
        (JP_ITEM_USE2_TABLE, JP_ITEM_USE2_TABLE_COUNT),
    ] {
        let table_bytes = source
            .get(table..table + count * 2)
            .ok_or_else(|| format!("item text table 0x{table:06X} is outside the source ROM"))?;
        for pair in table_bytes.chunks_exact(2) {
            targets.insert(table + u16::from_be_bytes([pair[0], pair[1]]) as usize);
        }
    }
    Ok(targets)
}

pub(super) fn validate_jp_item_use_source(
    source: &[u8],
    entry: &ItemUseEntry,
    expected_jp: &str,
    milestone: &str,
) -> Result<Vec<u16>, String> {
    validate_jp_text_source(
        source,
        &entry.id,
        entry.offset,
        expected_jp,
        &format!("{milestone} item-use"),
    )
}

pub(super) fn normalize_m7_item_use_tokens(
    id: &str,
    jp: &str,
    source_words: &[u16],
    ko_tokens: &[Token],
) -> Result<Vec<Token>, String> {
    normalize_m7_item_use_tokens_with_limit(
        id,
        jp,
        source_words,
        ko_tokens,
        JP_ITEM_USE_MAX_GLYPHS_PER_LINE,
    )
}

pub(super) fn normalize_m7_item_use_tokens_with_limit(
    id: &str,
    jp: &str,
    source_words: &[u16],
    ko_tokens: &[Token],
    max_glyphs_per_line: usize,
) -> Result<Vec<Token>, String> {
    validate_dialog_quote_start_rows(id, source_words, ko_tokens)?;
    validate_terminal_page_wait(id, source_words, ko_tokens)?;
    let source_events = m7_source_control_events(id, source_words)?;
    let source_face_icons: Vec<u16> = source_words
        .iter()
        .copied()
        .filter(|code| (JP_FACE_TILE_START..=JP_FACE_TILE_END).contains(code))
        .collect();
    let source_symbol_icons: Vec<u16> = source_words
        .iter()
        .copied()
        .filter(|code| is_jp_protected_symbol_tile(*code))
        .collect();
    let source_digits = visible_jp_digits(jp);
    let source_percent_tiles: Vec<u16> = source_words
        .iter()
        .copied()
        .filter(|code| *code == JP_PERCENT_TILE)
        .collect();
    let has_choice = source_events.iter().any(is_jp_choice_event);
    let source_layout_pad_runs = source_full_width_layout_pad_runs(source_words);
    let kr_layout_pad_runs = kr_layout_pad_runs(ko_tokens);
    let has_named_layout_pad = ko_tokens.contains(&Token::Tile(0x0078));
    let layout_pad_indices = if has_choice || has_named_layout_pad {
        if kr_layout_pad_runs
            .iter()
            .map(|run| run.len())
            .collect::<Vec<_>>()
            != source_layout_pad_runs
        {
            return Err(format!(
                "{id}: KR layout padding runs {:?} do not match JP source full-width blank runs {:?}",
                kr_layout_pad_runs
                    .iter()
                    .map(|run| run.len())
                    .collect::<Vec<_>>(),
                source_layout_pad_runs
            ));
        }
        kr_layout_pad_runs
            .iter()
            .flat_map(|run| run.clone())
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let explicit_source_pad_count = ko_tokens
        .iter()
        .filter(|token| matches!(token, Token::LayoutPad))
        .count();
    let implicit_source_pad_count = layout_pad_indices.len();
    let available_source_pad_count = source_full_width_layout_pad_count(source_words);
    if explicit_source_pad_count + implicit_source_pad_count > available_source_pad_count {
        return Err(format!(
            "{id}: KR requests {} source-backed full-width pads, but JP owns only {available_source_pad_count}",
            explicit_source_pad_count + implicit_source_pad_count
        ));
    }
    let has_mixed_quote_pairs =
        (jp.contains('「') || jp.contains('」')) && (jp.contains('『') || jp.contains('』'));
    let source_quote_tiles = if has_mixed_quote_pairs {
        source_words
            .iter()
            .copied()
            .filter(|code| {
                matches!(
                    *code,
                    JP_QUOTE_OPEN | JP_QUOTE_CLOSE | JP_ITEM_QUOTED_OPEN | JP_ITEM_QUOTED_CLOSE
                )
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let quote_pair = if jp.contains('『') || jp.contains('』') {
        Some((JP_ITEM_QUOTED_OPEN, JP_ITEM_QUOTED_CLOSE))
    } else if jp.contains('「') || jp.contains('」') {
        Some((JP_QUOTE_OPEN, JP_QUOTE_CLOSE))
    } else {
        None
    };

    let mut normalized = Vec::with_capacity(ko_tokens.len());
    let mut event_index = 0usize;
    let mut face_icon_index = 0usize;
    let mut symbol_icon_index = 0usize;
    let mut digit_index = 0usize;
    let mut percent_index = 0usize;
    let mut quote_index = 0usize;
    let mut index = 0usize;
    while index < ko_tokens.len() {
        let token = &ko_tokens[index];
        if let Some(digit) = korean_source_digit_at(ko_tokens, index)
            && !source_digits.is_empty()
        {
            let expected = source_digits.get(digit_index).ok_or_else(|| {
                format!("{id}: KR native numeral {digit} has no JP source-backed digit")
            })?;
            if *expected != digit {
                return Err(format!(
                    "{id}: KR native numeral {digit} does not match JP source digit {expected}"
                ));
            }
            digit_index += 1;
        }
        if layout_pad_indices.contains(&index) {
            normalized.push(Token::LayoutPad);
            index += 1;
            continue;
        }
        if matches!(token, Token::Ctrl(0xFF30 | 0xFF34)) {
            normalized.push(token.clone());
            index += 1;
            continue;
        }
        if token == &Token::Ctrl(0xFF74) && ko_tokens.get(index + 1) == Some(&Token::Ctrl(0xFFFF)) {
            let expected = source_events
                .get(event_index)
                .ok_or_else(|| format!("{id}: KR choice terminator has no JP protected control"))?;
            if expected != &Token::CtrlParam(0xFFA0, 0xFFFF) {
                return Err(format!(
                    "{id}: KR choice terminator does not match JP protected control {expected:?}"
                ));
            }
            normalized.push(expected.clone());
            event_index += 1;
            index += 2;
            continue;
        }

        let actual_code = match token {
            Token::Ctrl(code) | Token::CtrlParam(code, _) => Some(*code),
            _ => None,
        };
        if let Some(actual_code) = actual_code {
            let expected = source_events.get(event_index).ok_or_else(|| {
                format!("{id}: KR control 0x{actual_code:04X} has no JP protected control")
            })?;
            let expected_code = match expected {
                Token::Ctrl(code) | Token::CtrlParam(code, _) => *code,
                _ => return Err(format!("{id}: invalid JP protected control event")),
            };
            if actual_code != expected_code {
                if matches!(actual_code, 0xFF0C | 0xFFB8) {
                    index += 1;
                    continue;
                }
                return Err(format!(
                    "{id}: KR control 0x{actual_code:04X} does not match JP protected control 0x{expected_code:04X}"
                ));
            }
            normalized.push(expected.clone());
            event_index += 1;
            index += 1;

            if expected_code == 0xFFAC {
                let placeholder = ko_tokens
                    .get(index)
                    .ok_or_else(|| format!("{id}: FFAC is missing its EN-derived placeholder"))?;
                if !matches!(
                    placeholder,
                    Token::EnChar(_) | Token::Tile(_) | Token::Raw(_)
                ) {
                    return Err(format!(
                        "{id}: FFAC placeholder has unexpected token {placeholder:?}"
                    ));
                }
                index += 1;
            } else if matches!(expected, Token::CtrlParam(_, _)) && matches!(token, Token::Ctrl(_))
            {
                let placeholder = ko_tokens.get(index).ok_or_else(|| {
                    format!("{id}: parameterized control 0x{expected_code:04X} is missing its legacy placeholder")
                })?;
                if !matches!(placeholder, Token::EnChar('0'..='9')) {
                    return Err(format!(
                        "{id}: parameterized control 0x{expected_code:04X} has unexpected legacy placeholder {placeholder:?}"
                    ));
                }
                index += 1;
            }
            continue;
        }

        if let Some(mapped) = jp_protected_symbol_tile(token) {
            let expected = source_symbol_icons.get(symbol_icon_index).ok_or_else(|| {
                format!("{id}: KR symbol {token:?} has no JP protected symbol tile")
            })?;
            if *expected != mapped {
                return Err(format!(
                    "{id}: KR symbol {token:?} maps to 0x{mapped:04X}, not JP protected 0x{expected:04X}"
                ));
            }
            normalized.push(Token::Raw(mapped));
            symbol_icon_index += 1;
            index += 1;
            continue;
        }

        match token {
            Token::EnChar(ch) if ch.is_ascii_digit() => {
                let digit = ch.to_digit(10).expect("ASCII digit must have a value");
                let expected = source_digits.get(digit_index).ok_or_else(|| {
                    format!("{id}: KR digit {digit} has no JP source-backed digit")
                })?;
                if *expected != digit {
                    return Err(format!(
                        "{id}: KR digit {digit} does not match JP source digit {expected}"
                    ));
                }
                normalized.push(token.clone());
                digit_index += 1;
            }
            Token::Tile(0x0048 | 0x006D) => {
                let open = if has_mixed_quote_pairs {
                    let expected = *source_quote_tiles
                        .get(quote_index)
                        .ok_or_else(|| format!("{id}: KR open quote has no JP mixed-quote tile"))?;
                    if !matches!(expected, JP_QUOTE_OPEN | JP_ITEM_QUOTED_OPEN) {
                        return Err(format!(
                            "{id}: KR open quote does not match JP close quote 0x{expected:04X}"
                        ));
                    }
                    quote_index += 1;
                    expected
                } else {
                    let (open, _) = quote_pair
                        .ok_or_else(|| format!("{id}: KR open quote has no JP quote pair"))?;
                    open
                };
                normalized.push(Token::Raw(open));
            }
            Token::Tile(0x0049 | 0x006E) | Token::EnChar('"') => {
                let close = if has_mixed_quote_pairs {
                    let expected = *source_quote_tiles.get(quote_index).ok_or_else(|| {
                        format!("{id}: KR close quote has no JP mixed-quote tile")
                    })?;
                    if !matches!(expected, JP_QUOTE_CLOSE | JP_ITEM_QUOTED_CLOSE) {
                        return Err(format!(
                            "{id}: KR close quote does not match JP open quote 0x{expected:04X}"
                        ));
                    }
                    quote_index += 1;
                    expected
                } else {
                    let (_, close) = quote_pair
                        .ok_or_else(|| format!("{id}: KR close quote has no JP quote pair"))?;
                    close
                };
                normalized.push(Token::Raw(close));
            }
            Token::Tile(0x0078) => {
                return Err(format!(
                    "{id}: layout pad is not backed by a matching JP full-width blank run"
                ));
            }
            Token::Tile(code) if (EN_FACE_TILE_START..=EN_FACE_TILE_END).contains(code) => {
                let mapped = JP_FACE_TILE_START + (*code - EN_FACE_TILE_START);
                let expected = source_face_icons
                    .get(face_icon_index)
                    .ok_or_else(|| format!("{id}: KR face tile has no JP protected face tile"))?;
                if *expected != mapped {
                    return Err(format!(
                        "{id}: KR face tile maps to 0x{mapped:04X}, not JP protected 0x{expected:04X}"
                    ));
                }
                normalized.push(Token::Raw(mapped));
                face_icon_index += 1;
            }
            Token::Tile(code) => {
                return Err(format!(
                    "{id}: EN-derived named tile 0x{code:04X} is not valid in ordinary item use"
                ));
            }
            Token::EnChar('▸') if has_choice => {}
            Token::Raw(0x004E) => {
                let expected = source_percent_tiles.get(percent_index).ok_or_else(|| {
                    format!("{id}: KR percent placeholder has no JP source percent tile")
                })?;
                normalized.push(Token::Raw(*expected));
                percent_index += 1;
            }
            Token::Raw(code) => {
                return Err(format!(
                    "{id}: raw text word 0x{code:04X} is not valid in ordinary item use"
                ));
            }
            _ => normalized.push(token.clone()),
        }
        index += 1;
    }
    if event_index != source_events.len() {
        return Err(format!(
            "{id}: KR ordinary item-use text consumed {event_index} of {} JP protected controls",
            source_events.len()
        ));
    }
    if face_icon_index != source_face_icons.len() {
        return Err(format!(
            "{id}: KR text consumed {face_icon_index} of {} JP protected face tiles",
            source_face_icons.len()
        ));
    }
    if symbol_icon_index != source_symbol_icons.len() {
        return Err(format!(
            "{id}: KR text consumed {symbol_icon_index} of {} JP protected symbol tiles",
            source_symbol_icons.len()
        ));
    }
    if digit_index != source_digits.len() {
        return Err(format!(
            "{id}: KR text consumed {digit_index} of {} JP source-backed digits",
            source_digits.len()
        ));
    }
    if percent_index != source_percent_tiles.len() {
        return Err(format!(
            "{id}: KR text consumed {percent_index} of {} JP source percent tiles",
            source_percent_tiles.len()
        ));
    }
    if quote_index != source_quote_tiles.len() {
        return Err(format!(
            "{id}: KR text consumed {quote_index} of {} JP mixed-quote tiles",
            source_quote_tiles.len()
        ));
    }

    let normalized = normalize_m2_ellipsis(&normalized);
    wrap_fixed_width_lines(id, &normalized, max_glyphs_per_line)
}

fn validate_terminal_page_wait(
    id: &str,
    source_words: &[u16],
    target_tokens: &[Token],
) -> Result<(), String> {
    let source_has_terminal_page = source_words.ends_with(&[0xFF34, 0xFFFF]);
    let target_has_terminal_page =
        target_tokens.ends_with(&[Token::Ctrl(0xFF34), Token::Ctrl(0xFFFF)]);
    if source_has_terminal_page && !target_has_terminal_page {
        return Err(format!(
            "{id}: KR drops the JP terminal PAGE wait immediately before FFFF"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceOverwriteFootprint {
    selector: u16,
    line_half_cells: Vec<usize>,
}

#[derive(Debug)]
struct ActiveSourceOverwrite {
    footprint_index: usize,
    line_index: usize,
    half_cells: usize,
}

fn is_source_overwrite_selector(code: u16) -> bool {
    matches!(code, 0xFF10 | 0xFF14 | 0xFF18 | 0xFF1C)
}

fn source_first_quote_rows(source_words: &[u16]) -> Vec<Option<usize>> {
    let controls_with_parameter = crate::build::text::ctrl_with_param();
    let mut rows = Vec::new();
    let mut active = None;
    let mut line = 0usize;
    let mut index = 0usize;
    while index < source_words.len() {
        let word = source_words[index];
        if is_source_overwrite_selector(word) {
            rows.push(None);
            active = Some(rows.len() - 1);
            line = 0;
        } else if word == 0xFF30 {
            line += 1;
        } else if word == JP_QUOTE_OPEN
            && let Some(active) = active
            && rows[active].is_none()
        {
            rows[active] = Some(line);
        }
        index += if word == 0xFFAC || controls_with_parameter.contains(&word) {
            2
        } else {
            1
        };
    }
    rows
}

fn target_first_quote_rows(tokens: &[Token]) -> Vec<Option<usize>> {
    let mut rows = Vec::new();
    let mut active = None;
    let mut line = 0usize;
    for token in tokens {
        match token {
            Token::Ctrl(code) if is_source_overwrite_selector(*code) => {
                rows.push(None);
                active = Some(rows.len() - 1);
                line = 0;
            }
            Token::Ctrl(0xFF30) => line += 1,
            Token::Tile(0x0048) if active.is_some_and(|active| rows[active].is_none()) => {
                rows[active.expect("active quote row disappeared")] = Some(line);
            }
            _ => {}
        }
    }
    rows
}

fn validate_dialog_quote_start_rows(
    id: &str,
    source_words: &[u16],
    target_tokens: &[Token],
) -> Result<(), String> {
    let source_rows = source_first_quote_rows(source_words);
    let target_rows = target_first_quote_rows(target_tokens);
    for (selector_index, source_row) in source_rows.iter().enumerate() {
        let Some(source_row) = source_row else {
            continue;
        };
        let target_row = target_rows.get(selector_index).copied().flatten();
        // Some battle surfaces add a monster-name row before JP dialogue that
        // opens immediately. All other source-leading rows are semantic text:
        // KR may not leave one of them outside the opening quote.
        let aligned = if *source_row == 0 {
            target_row.is_some_and(|row| row <= 1)
        } else {
            target_row == Some(*source_row)
        };
        if !aligned {
            return Err(format!(
                "{id}: selector {} JP dialogue opens on row {}, but KR opening quote is on row {}",
                selector_index + 1,
                source_row + 1,
                target_row.map_or_else(|| "missing".to_string(), |row| (row + 1).to_string())
            ));
        }
    }
    Ok(())
}

fn is_source_overwrite_boundary(code: u16) -> bool {
    is_source_overwrite_selector(code)
        || matches!(
            code,
            0xFF04
                | 0xFF08
                | 0xFF0C
                | 0xFF34
                | 0xFF38
                | 0xFF3C
                | 0xFF70
                | 0xFF74
                | 0xFF7C
                | 0xFF90
                | 0xFF94
                | 0xFF98
                | 0xFF9C
                | 0xFFA0
                | 0xFFA4
                | 0xFFA8
                | 0xFFB4
                | 0xFFB8
                | 0xFFC0
                | 0xFFCC
                | 0xFFFF
        )
}

fn source_word_span(
    id: &str,
    source_words: &[u16],
    index: usize,
    controls_with_parameter: &HashSet<u16>,
) -> Result<usize, String> {
    let word = source_words[index];
    if word == 0xFFAC || controls_with_parameter.contains(&word) {
        source_words.get(index + 1).ok_or_else(|| {
            format!("{id}: JP control 0x{word:04X} is missing its protected operand")
        })?;
        Ok(2)
    } else {
        Ok(1)
    }
}

fn source_overwrite_footprints(
    id: &str,
    source_words: &[u16],
) -> Result<Vec<SourceOverwriteFootprint>, String> {
    let controls_with_parameter = crate::build::text::ctrl_with_param();
    let mut footprints = Vec::new();
    let mut index = 0usize;
    while index < source_words.len() {
        let selector = source_words[index];
        if is_source_overwrite_selector(selector) {
            let mut line_half_cells = vec![0usize];
            let mut cursor = index + 1;
            while cursor < source_words.len() {
                let word = source_words[cursor];
                if is_source_overwrite_boundary(word) {
                    break;
                }
                if word == 0xFF30 {
                    line_half_cells.push(0);
                    cursor += 1;
                    continue;
                }
                if word == 0xFFAC {
                    cursor += source_word_span(id, source_words, cursor, &controls_with_parameter)?;
                    continue;
                }
                if controls_with_parameter.contains(&word) {
                    let parameter = *source_words.get(cursor + 1).ok_or_else(|| {
                        format!("{id}: JP control 0x{word:04X} is missing its protected operand")
                    })?;
                    if let Some(consumer) = DynamicDisplayControl::from_code(word) {
                        *line_half_cells
                            .last_mut()
                            .expect("source overwrite always has a current line") +=
                            consumer.visible_words() * 2;
                        if parameter < 0xFF00 {
                            *line_half_cells
                                .last_mut()
                                .expect("source overwrite always has a current line") += 2;
                        } else if parameter == 0xFF30 {
                            line_half_cells.push(0);
                        }
                    }
                    cursor += 2;
                    continue;
                }
                if word < 0xFF00 {
                    *line_half_cells
                        .last_mut()
                        .expect("source overwrite always has a current line") += 2;
                }
                cursor += 1;
            }
            footprints.push(SourceOverwriteFootprint {
                selector,
                line_half_cells,
            });
        }
        index += source_word_span(id, source_words, index, &controls_with_parameter)?;
    }
    Ok(footprints)
}

fn owned_source_line_half_cells(
    source_half_cells: usize,
    target_half_cells: usize,
    standard_row_half_cells: usize,
) -> usize {
    if source_half_cells == 0 && target_half_cells == 0 {
        0
    } else if standard_row_half_cells == 0 {
        source_half_cells.max(target_half_cells)
    } else {
        // The physical owner is the standard text row, not the longest JP
        // phrase ever drawn into it. A source line that reaches into the
        // border still finalizes at the same fixed right edge.
        standard_row_half_cells
    }
}

fn finalize_active_source_line(
    id: &str,
    output: &mut Vec<Token>,
    active: &ActiveSourceOverwrite,
    footprints: &[SourceOverwriteFootprint],
    standard_row_half_cells: usize,
) -> Result<(), String> {
    let source_half_cells = footprints[active.footprint_index]
        .line_half_cells
        .get(active.line_index)
        .copied()
        .unwrap_or_default();
    let owned_half_cells = owned_source_line_half_cells(
        source_half_cells,
        active.half_cells,
        standard_row_half_cells,
    );
    if owned_half_cells == 0 {
        if active.half_cells != 0 {
            return Err(format!(
                "{id}: KR writes {} half cells into an empty JP overwrite row",
                active.half_cells
            ));
        }
        return Ok(());
    }
    let clear_half_cells = owned_half_cells
        .checked_sub(active.half_cells)
        .ok_or_else(|| {
            format!(
                "{id}: KR overwrite row uses {} half cells but owns only {owned_half_cells}",
                active.half_cells
            )
        })?;
    output.push(Token::SourceRowFinalize { clear_half_cells });
    Ok(())
}

fn append_remaining_source_rows(
    output: &mut Vec<Token>,
    active: &ActiveSourceOverwrite,
    footprints: &[SourceOverwriteFootprint],
    standard_row_half_cells: usize,
) {
    let footprint = &footprints[active.footprint_index];
    for &source_half_cells in footprint.line_half_cells.iter().skip(active.line_index + 1) {
        output.push(Token::Ctrl(0xFF30));
        let owned_half_cells =
            owned_source_line_half_cells(source_half_cells, 0, standard_row_half_cells);
        if owned_half_cells != 0 {
            output.push(Token::SourceRowFinalize {
                clear_half_cells: owned_half_cells,
            });
        }
    }
}

fn finish_active_source_overwrite(
    id: &str,
    output: &mut Vec<Token>,
    active: ActiveSourceOverwrite,
    footprints: &[SourceOverwriteFootprint],
    standard_row_half_cells: usize,
) -> Result<(), String> {
    finalize_active_source_line(id, output, &active, footprints, standard_row_half_cells)?;
    append_remaining_source_rows(output, &active, footprints, standard_row_half_cells);
    Ok(())
}

fn target_dynamic_half_cells(token: &Token) -> Option<(usize, Option<u16>)> {
    let Token::CtrlParam(code, parameter) = token else {
        return None;
    };
    let consumer = DynamicDisplayControl::from_code(*code)?;
    let trailing_half_cells = dynamic_trailing_word_half_cells(*parameter);
    Some((
        consumer.visible_words() * 2 + trailing_half_cells,
        (*parameter >= 0xFF00).then_some(*parameter),
    ))
}

pub(super) fn protect_source_overwrite_footprints(
    id: &str,
    source_words: &[u16],
    tokens: &[Token],
    standard_row_half_cells: usize,
) -> Result<Vec<Token>, String> {
    let footprints = source_overwrite_footprints(id, source_words)?;
    if footprints.is_empty() {
        return Ok(tokens.to_vec());
    }

    let mut output = Vec::with_capacity(tokens.len());
    let mut next_footprint = 0usize;
    let mut active = None;
    for token in tokens {
        let selector = match token {
            Token::Ctrl(code) if is_source_overwrite_selector(*code) => Some(*code),
            _ => None,
        };
        if let Some(selector) = selector {
            if let Some(previous) = active.take() {
                finish_active_source_overwrite(
                    id,
                    &mut output,
                    previous,
                    &footprints,
                    standard_row_half_cells,
                )?;
            }
            let footprint = footprints.get(next_footprint).ok_or_else(|| {
                format!("{id}: KR has an extra in-place overwrite selector 0x{selector:04X}")
            })?;
            if footprint.selector != selector {
                return Err(format!(
                    "{id}: KR overwrite selector 0x{selector:04X} does not match JP selector 0x{:04X}",
                    footprint.selector
                ));
            }
            output.push(token.clone());
            active = Some(ActiveSourceOverwrite {
                footprint_index: next_footprint,
                line_index: 0,
                half_cells: 0,
            });
            next_footprint += 1;
            continue;
        }

        if let Some(current) = active.as_mut() {
            if token == &Token::Ctrl(0xFF30) {
                finalize_active_source_line(
                    id,
                    &mut output,
                    current,
                    &footprints,
                    standard_row_half_cells,
                )?;
                output.push(token.clone());
                current.line_index += 1;
                current.half_cells = 0;
                continue;
            }
            if matches!(
                token,
                Token::Ctrl(code) | Token::CtrlParam(code, _)
                    if is_source_overwrite_boundary(*code)
            ) {
                let finished = active
                    .take()
                    .expect("active source overwrite disappeared before its boundary");
                finish_active_source_overwrite(
                    id,
                    &mut output,
                    finished,
                    &footprints,
                    standard_row_half_cells,
                )?;
                output.push(token.clone());
                continue;
            }
            if let Some((half_cells, trailing_control)) = target_dynamic_half_cells(token) {
                if trailing_control
                    .is_some_and(|code| code == 0xFF30 || is_source_overwrite_boundary(code))
                {
                    let Token::CtrlParam(code, trailing_control) = token else {
                        unreachable!("dynamic display token shape changed");
                    };
                    current.half_cells += half_cells;
                    // The word after a dynamic-display opcode is redispatched
                    // only after its fixed buffer is drawn. Put the private
                    // finalizer in that slot, then place the original trailing
                    // control after it: buffer -> finalize row -> NL/boundary.
                    output.push(Token::Ctrl(*code));
                    finalize_active_source_line(
                        id,
                        &mut output,
                        current,
                        &footprints,
                        standard_row_half_cells,
                    )?;
                    output.push(Token::Ctrl(*trailing_control));
                    if *trailing_control == 0xFF30 {
                        current.line_index += 1;
                        current.half_cells = 0;
                    } else {
                        let finished = active
                            .take()
                            .expect("active source overwrite disappeared at a dynamic boundary");
                        append_remaining_source_rows(
                            &mut output,
                            &finished,
                            &footprints,
                            standard_row_half_cells,
                        );
                    }
                    continue;
                }
                current.half_cells += half_cells;
                output.push(token.clone());
                continue;
            }
            current.half_cells += fixed_width_token_half_cells(token);
        }
        output.push(token.clone());
    }

    if let Some(previous) = active.take() {
        finish_active_source_overwrite(
            id,
            &mut output,
            previous,
            &footprints,
            standard_row_half_cells,
        )?;
    }
    if next_footprint != footprints.len() {
        return Err(format!(
            "{id}: KR consumed {next_footprint} of {} JP in-place overwrite selectors",
            footprints.len()
        ));
    }
    Ok(output)
}

fn korean_source_digit_at(tokens: &[Token], index: usize) -> Option<u32> {
    let tail = tokens.get(index..)?;
    let starts_word = index == 0 || !matches!(tokens.get(index - 1), Some(Token::KrChar(_)));
    if matches!(
        tail,
        [
            Token::KrChar('한'),
            Token::EnChar(' '),
            Token::KrChar('번' | '층' | '명'),
            ..
        ] | [
            Token::KrChar('한'),
            Token::EnChar(' '),
            Token::KrChar('가'),
            Token::KrChar('지'),
            ..
        ]
    ) {
        return Some(1);
    }
    let standalone_one = matches!(tail, [Token::KrChar('하'), Token::KrChar('나'), ..])
        && matches!(
            tail.get(2),
            None | Some(
                Token::KrChar('가' | '는' | '도' | '만' | '를' | '씩' | '의' | '인')
                    | Token::EnChar(' ')
                    | Token::Ctrl(_)
                    | Token::CtrlParam(_, _)
            )
        );
    if starts_word && standalone_one {
        return Some(1);
    }
    let standalone_two = matches!(tail.first(), Some(Token::KrChar('둘')))
        && matches!(
            tail.get(1),
            None | Some(
                Token::KrChar('이' | '은' | '만' | '을' | '과' | '도' | '의')
                    | Token::EnChar(' ')
                    | Token::Ctrl(_)
                    | Token::CtrlParam(_, _)
            )
        );
    if matches!(
        tail,
        [
            Token::KrChar('두'),
            Token::EnChar(' '),
            Token::KrChar('명'),
            ..
        ] | [
            Token::KrChar('두'),
            Token::EnChar(' '),
            Token::KrChar('사'),
            Token::KrChar('람'),
            ..
        ]
    ) || (starts_word && standalone_two)
    {
        return Some(2);
    }
    None
}

fn is_jp_choice_event(token: &Token) -> bool {
    matches!(
        token,
        Token::Ctrl(0xFF70 | 0xFF74 | 0xFF7C | 0xFFA4 | 0xFFA8)
            | Token::CtrlParam(0xFF9C | 0xFFA0, _)
    )
}

fn source_full_width_layout_pad_runs(source_words: &[u16]) -> Vec<usize> {
    let controls_with_parameter = crate::build::text::ctrl_with_param();
    let mut runs = Vec::new();
    let mut run = 0usize;
    let mut index = 0usize;
    while index < source_words.len() {
        let word = source_words[index];
        if word == JP_NATIVE_FULL_WIDTH_LAYOUT_PAD {
            run += 1;
            index += 1;
            continue;
        }
        if run >= 2 {
            runs.push(run);
        }
        run = 0;
        index += if word == 0xFFAC || controls_with_parameter.contains(&word) {
            2
        } else {
            1
        };
    }
    if run >= 2 {
        runs.push(run);
    }
    runs
}

fn source_full_width_layout_pad_count(source_words: &[u16]) -> usize {
    let controls_with_parameter = crate::build::text::ctrl_with_param();
    let mut count = 0usize;
    let mut index = 0usize;
    while index < source_words.len() {
        let word = source_words[index];
        if word == JP_NATIVE_FULL_WIDTH_LAYOUT_PAD {
            count += 1;
        }
        index += if word == 0xFFAC || controls_with_parameter.contains(&word) {
            2
        } else {
            1
        };
    }
    count
}

fn kr_layout_pad_runs(tokens: &[Token]) -> Vec<Vec<usize>> {
    let mut runs = Vec::new();
    let mut run = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token, Token::EnChar(' ') | Token::Tile(0x0078)) {
            run.push(index);
            continue;
        }
        if run.len() >= 2 {
            runs.push(std::mem::take(&mut run));
        } else {
            run.clear();
        }
    }
    if run.len() >= 2 {
        runs.push(run);
    }
    runs
}

pub(super) fn m7_source_control_events(
    id: &str,
    source_words: &[u16],
) -> Result<Vec<Token>, String> {
    let mut events = Vec::new();
    let ctrl_with_param = crate::build::text::ctrl_with_param();
    let mut word_index = 0usize;
    while let Some(&word) = source_words.get(word_index) {
        match word {
            0xFF30 | 0xFF34 => {}
            0xFFAC => {
                let operand = *source_words.get(word_index + 1).ok_or_else(|| {
                    format!("{id}: JP FFAC operand is outside the protected source")
                })?;
                if operand >= 0xFF00 {
                    return Err(format!(
                        "{id}: JP FFAC operand 0x{operand:04X} is not a word"
                    ));
                }
                events.push(Token::CtrlParam(0xFFAC, operand));
                word_index += 1;
            }
            code if ctrl_with_param.contains(&code) => {
                let parameter = *source_words.get(word_index + 1).ok_or_else(|| {
                    format!("{id}: JP 0x{code:04X} parameter is outside the protected source")
                })?;
                events.push(Token::CtrlParam(code, parameter));
                word_index += 1;
            }
            code if code >= 0xFF00 => events.push(Token::Ctrl(code)),
            _ => {}
        }
        word_index += 1;
    }
    Ok(events)
}

pub(super) fn wrap_fixed_width_lines(
    id: &str,
    tokens: &[Token],
    max_glyphs_per_line: usize,
) -> Result<Vec<Token>, String> {
    let mut normalized = tokens.to_vec();
    loop {
        let mut line_start = 0usize;
        let mut wrapped = false;
        while line_start < normalized.len() {
            let line_end = normalized[line_start..]
                .iter()
                .position(|token| {
                    matches!(
                        token,
                        Token::Ctrl(
                            0xFF10
                                | 0xFF14
                                | 0xFF18
                                | 0xFF1C
                                | 0xFF30
                                | 0xFF34
                                | 0xFF04
                                | 0xFF38
                                | 0xFFB4
                                | 0xFFB8
                                | 0xFFFF
                        ) | Token::CtrlParam(0xFFA0, _)
                    )
                })
                .map(|relative| line_start + relative)
                .unwrap_or(normalized.len());
            let visible_half_cells = normalized[line_start..line_end]
                .iter()
                .map(fixed_width_token_half_cells)
                .sum::<usize>();
            if visible_half_cells > max_glyphs_per_line * 2 {
                let split = normalized[line_start..line_end]
                    .iter()
                    .rposition(|token| *token == Token::EnChar(' '))
                    .map(|relative| line_start + relative)
                    .ok_or_else(|| {
                        format!(
                            "{id}: fixed-width line exceeds {max_glyphs_per_line} cells and has no safe word boundary: {:?}",
                            &normalized[line_start..line_end]
                        )
                    })?;
                normalized[split] = Token::Ctrl(0xFF30);
                wrapped = true;
                break;
            }
            if line_end == normalized.len() {
                break;
            }
            line_start = line_end + 1;
        }
        if !wrapped {
            break;
        }
    }
    Ok(normalized)
}

pub(super) fn encode_m7_item_use_tokens(
    id: &str,
    tokens: &[Token],
    charmap: &BTreeMap<char, u16>,
) -> Result<Vec<u16>, String> {
    let mut words = Vec::new();
    for token in tokens {
        match token {
            Token::Ctrl(code) => words.push(*code),
            Token::CtrlParam(code, parameter) => {
                if *code == 0xFFF8 {
                    return Err(format!(
                        "{id}: ordinary item use cannot redirect through FFF8"
                    ));
                }
                let parameter = if *parameter == 0
                    && DynamicDisplayControl::from_code(*code).is_some()
                {
                    *charmap.get(&' ').ok_or_else(|| {
                        format!(
                            "{id}: dynamic blank suffix requires the dedicated JP-native space glyph"
                        )
                    })?
                } else {
                    *parameter
                };
                words.extend_from_slice(&[*code, parameter]);
            }
            Token::KrChar(ch) => words.push(*charmap.get(ch).ok_or_else(|| {
                format!(
                    "{id}: JP-native glyph is missing: '{ch}' (U+{:04X})",
                    *ch as u32
                )
            })?),
            Token::EnChar(ch) => {
                if let Some(&code) = charmap.get(ch) {
                    words.push(code);
                } else if *ch == ' ' {
                    return Err(format!(
                        "{id}: ordinary item-use space requires the dedicated JP-native space glyph"
                    ));
                } else if let Some(digit) = ch.to_digit(10) {
                    words.push(0x0001 + digit as u16);
                } else {
                    return Err(format!(
                        "{id}: unsupported JP-native ordinary item-use character '{ch}' (U+{:04X})",
                        *ch as u32
                    ));
                }
            }
            Token::LayoutPad => {
                words.push(JP_NATIVE_FULL_WIDTH_LAYOUT_PAD);
            }
            Token::SourceRowFinalize { .. } => {
                return Err(format!(
                    "{id}: source-row finalization is not valid in packed ordinary item use"
                ));
            }
            Token::Raw(code)
                if matches!(
                    *code,
                    JP_ITEM_QUOTED_OPEN | JP_ITEM_QUOTED_CLOSE | JP_QUOTE_OPEN | JP_QUOTE_CLOSE
                ) =>
            {
                words.push(*code);
            }
            Token::Raw(code) => {
                return Err(format!(
                    "{id}: unapproved raw word 0x{code:04X} in ordinary item use"
                ));
            }
            Token::Tile(code) => {
                return Err(format!(
                    "{id}: unnormalized named tile 0x{code:04X} in ordinary item use"
                ));
            }
        }
    }
    Ok(words)
}

pub(super) fn validate_m7_item_use_layout(
    rom: &[u8],
    layout: &ItemUseLayout,
) -> Result<(), String> {
    if rom.get(JP_ITEM_USE_TABLE..JP_ITEM_USE_TABLE + layout.table.len())
        != Some(layout.table.as_slice())
    {
        return Err("M7 ordinary item-use table re-extraction differs".to_string());
    }
    let entries_by_offset: BTreeMap<usize, &EncodedItemUse> = layout
        .entries
        .iter()
        .map(|entry| (entry.new_offset, entry))
        .collect();
    let mut referenced_offsets = BTreeSet::new();
    for pair in layout.table.chunks_exact(2) {
        let target = JP_ITEM_USE_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize;
        let entry = entries_by_offset.get(&target).ok_or_else(|| {
            format!("M7 ordinary item-use pointer targets unknown 0x{target:06X}")
        })?;
        referenced_offsets.insert(target);
        let expected = words_to_bytes(&entry.words);
        if rom.get(target..target + expected.len()) != Some(expected.as_slice()) {
            return Err(format!(
                "{}: ordinary item-use direct-consumer re-extraction differs",
                entry.id
            ));
        }
    }
    if referenced_offsets.len() != layout.entries.len() {
        return Err("M7 ordinary item-use table does not reference every packed entry".to_string());
    }
    Ok(())
}

pub(super) fn build_m8_item_use2_layout(
    source: &[u8],
    translation_dir: &Path,
    charmap: &BTreeMap<char, u16>,
    item_desc_layout: &ItemDescLayout,
) -> Result<ItemUse2Layout, String> {
    let table_len = JP_ITEM_USE2_TABLE_COUNT * 2;
    let table_source = source
        .get(JP_ITEM_USE2_TABLE..JP_ITEM_USE2_TABLE + table_len)
        .ok_or("M8 battle item-use pointer table is outside the source ROM")?;
    let old_targets: Vec<usize> = table_source
        .chunks_exact(2)
        .map(|pair| JP_ITEM_USE2_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize)
        .collect();
    let unique_old_targets: BTreeSet<usize> = old_targets.iter().copied().collect();
    if unique_old_targets.len() != JP_ITEM_USE2_UNIQUE_COUNT {
        return Err(format!(
            "M8 battle item-use source has {} unique targets, expected {JP_ITEM_USE2_UNIQUE_COUNT}",
            unique_old_targets.len()
        ));
    }

    let desc_old_targets: BTreeSet<usize> = item_desc_layout
        .entries
        .iter()
        .map(|entry| entry.old_offset)
        .collect();
    let shared_targets: Vec<usize> = unique_old_targets
        .intersection(&desc_old_targets)
        .copied()
        .collect();
    if shared_targets.len() != JP_ITEM_DESC_USE2_SHARED_TARGET_COUNT {
        return Err(format!(
            "M8 found {} description targets shared by battle use, expected {JP_ITEM_DESC_USE2_SHARED_TARGET_COUNT}",
            shared_targets.len()
        ));
    }
    let shared_old_target = shared_targets[0];
    let shared_desc = item_desc_layout
        .entries
        .iter()
        .find(|entry| entry.old_offset == shared_old_target)
        .ok_or("M8 shared battle-use description entry is missing")?;
    let shared_new_target = shared_desc.new_offset;

    let asset_entries = load_item_use_entries(
        translation_dir,
        "item_use2",
        JP_ITEM_USE2_ASSET_COUNT,
        "M8 battle item-use",
    )?;
    let asset_offsets: BTreeSet<usize> = asset_entries.iter().map(|entry| entry.offset).collect();
    let owned_old_targets: BTreeSet<usize> = unique_old_targets
        .iter()
        .copied()
        .filter(|target| *target != shared_old_target)
        .collect();
    if asset_offsets != owned_old_targets {
        return Err(
            "M8 battle item-use asset offsets do not match the non-description JP pointer population"
                .to_string(),
        );
    }

    let fallthrough_target = asset_entries
        .iter()
        .find(|entry| entry.id == JP_ITEM_USE2_FALLTHROUGH_TARGET_ID)
        .cloned()
        .ok_or("M8 fallthrough target script_0401 is missing")?;
    let shared_suffix_target = asset_entries
        .iter()
        .find(|entry| entry.id == JP_ITEM_USE2_SHARED_SUFFIX_TARGET_ID)
        .cloned()
        .ok_or("M8 shared-suffix target script_0395 is missing")?;
    let all_item_targets = collect_item_text_targets(source)?;
    let mut source_slots = Vec::with_capacity(asset_entries.len());
    let mut encoded_entries = Vec::with_capacity(asset_entries.len());
    for entry in asset_entries {
        let source_words = validate_jp_item_use_source(source, &entry, &entry.jp, "M8")?;
        let source_end = entry.offset + source_words.len() * 2;
        let slot_words = match all_item_targets
            .range(entry.offset + 2..source_end)
            .next()
            .copied()
        {
            Some(interior_target)
                if entry.id == JP_ITEM_USE2_SHARED_SUFFIX_ID
                    && interior_target == shared_suffix_target.offset =>
            {
                (interior_target - entry.offset) / 2
            }
            Some(interior_target) => {
                return Err(format!(
                    "{}: unexpected item text target 0x{interior_target:06X} points inside its M8 source slot",
                    entry.id
                ));
            }
            None => source_words.len(),
        };
        source_slots.push((entry.offset, slot_words));

        let (protected_jp, protected_words) = if entry.id == JP_ITEM_USE2_FALLTHROUGH_ID {
            if source_end != fallthrough_target.offset {
                return Err(format!(
                    "{}: source fallthrough ends at 0x{source_end:06X}, not {} at 0x{:06X}",
                    entry.id, fallthrough_target.id, fallthrough_target.offset
                ));
            }
            let combined_jp = format!("{}{}", entry.jp, fallthrough_target.jp);
            let combined_words =
                validate_jp_item_use_source(source, &entry, &combined_jp, "M8 fallthrough")?;
            (combined_jp, combined_words)
        } else {
            (entry.jp.clone(), source_words)
        };

        let mut tokens = normalize_m7_item_use_tokens(
            &entry.id,
            &protected_jp,
            &protected_words,
            &crate::build::text::parse_display_text(&entry.ko),
        )?;
        localize_dynamic_trailing_tiles(&entry.id, &mut tokens, charmap)?;
        validate_m2_fixed_width_lines(&tokens, &entry.id)?;
        let words = encode_m7_item_use_tokens(&entry.id, &tokens, charmap)?;
        if words.last() != Some(&0xFF38) {
            return Err(format!(
                "{}: M8 battle item-use message does not end in FF38",
                entry.id
            ));
        }
        encoded_entries.push(EncodedItemUse {
            id: entry.id,
            old_offset: entry.offset,
            new_offset: 0,
            words,
        });
    }

    source_slots.sort_by_key(|&(offset, words)| (std::cmp::Reverse(words), offset));
    encoded_entries.sort_by_key(|entry| {
        (
            std::cmp::Reverse(entry.words.len()),
            entry.old_offset,
            entry.id.clone(),
        )
    });
    let overflows: Vec<String> = encoded_entries
        .iter()
        .zip(source_slots.iter())
        .filter(|(entry, (_, slot_words))| entry.words.len() > *slot_words)
        .map(|(entry, (_, slot_words))| format!("{} {}>{slot_words}", entry.id, entry.words.len()))
        .collect();
    if !overflows.is_empty() {
        return Err(format!(
            "M8 battle item-use slot capacity failed: {}",
            overflows.join(", ")
        ));
    }
    for (entry, &(slot_offset, _)) in encoded_entries.iter_mut().zip(source_slots.iter()) {
        entry.new_offset = slot_offset;
    }
    encoded_entries.sort_by_key(|entry| entry.old_offset);

    let new_offsets: BTreeMap<usize, usize> = encoded_entries
        .iter()
        .map(|entry| (entry.old_offset, entry.new_offset))
        .collect();
    let mut table = Vec::with_capacity(table_len);
    for old_target in old_targets {
        let new_target = if old_target == shared_old_target {
            shared_new_target
        } else {
            *new_offsets.get(&old_target).ok_or_else(|| {
                format!("M8 battle item-use target 0x{old_target:06X} has no asset")
            })?
        };
        let relative = new_target
            .checked_sub(JP_ITEM_USE2_TABLE)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| format!("M8 battle item-use target 0x{new_target:06X} is not rel16"))?;
        table.extend_from_slice(&relative.to_be_bytes());
    }

    let mut writes: Vec<ItemUseWrite> = encoded_entries
        .iter()
        .map(|entry| ItemUseWrite {
            id: entry.id.clone(),
            offset: entry.new_offset,
            replacement: words_to_bytes(&entry.words),
        })
        .collect();
    writes.sort_by_key(|write| write.offset);
    for pair in writes.windows(2) {
        let left_end = pair[0].offset + pair[0].replacement.len();
        if left_end > pair[1].offset {
            return Err(format!(
                "M8 battle item-use writes overlap at 0x{left_end:06X}"
            ));
        }
    }

    Ok(ItemUse2Layout {
        table,
        source_bytes: source_slots.iter().map(|(_, words)| words * 2).sum(),
        payload_bytes: encoded_entries
            .iter()
            .map(|entry| entry.words.len() * 2)
            .sum(),
        writes,
        entries: encoded_entries,
        shared_old_target,
        shared_new_target,
        shared_words: shared_desc.words.clone(),
    })
}

pub(super) fn validate_m8_item_use2_layout(
    rom: &[u8],
    layout: &ItemUse2Layout,
) -> Result<(), String> {
    if rom.get(JP_ITEM_USE2_TABLE..JP_ITEM_USE2_TABLE + layout.table.len())
        != Some(layout.table.as_slice())
    {
        return Err("M8 battle item-use table re-extraction differs".to_string());
    }
    let entries_by_offset: BTreeMap<usize, &EncodedItemUse> = layout
        .entries
        .iter()
        .map(|entry| (entry.new_offset, entry))
        .collect();
    let mut referenced_offsets = BTreeSet::new();
    let mut referenced_shared = false;
    for pair in layout.table.chunks_exact(2) {
        let target = JP_ITEM_USE2_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize;
        if target == layout.shared_new_target {
            let expected = words_to_bytes(&layout.shared_words);
            if rom.get(target..target + expected.len()) != Some(expected.as_slice()) {
                return Err("M8 shared battle-use description re-extraction differs".to_string());
            }
            referenced_shared = true;
            continue;
        }
        let entry = entries_by_offset
            .get(&target)
            .ok_or_else(|| format!("M8 battle item-use pointer targets unknown 0x{target:06X}"))?;
        referenced_offsets.insert(target);
        let expected = words_to_bytes(&entry.words);
        if rom.get(target..target + expected.len()) != Some(expected.as_slice()) {
            return Err(format!(
                "{}: battle item-use direct-consumer re-extraction differs",
                entry.id
            ));
        }
    }
    if !referenced_shared {
        return Err(format!(
            "M8 battle item-use table lost shared description target 0x{:06X}",
            layout.shared_new_target
        ));
    }
    if referenced_offsets.len() != layout.entries.len() {
        return Err("M8 battle item-use table does not reference every packed entry".to_string());
    }
    Ok(())
}
