//! Native M3 item-name and M5 quoted-item label tables.

use super::*;

#[derive(Debug)]
pub(super) struct ItemNameEntry {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) jp: String,
    pub(super) ko: String,
}

#[derive(Debug)]
pub(super) struct EncodedItemName {
    pub(super) id: String,
    pub(super) old_offset: usize,
    pub(super) new_offset: usize,
    pub(super) words: Vec<u16>,
}

#[derive(Debug)]
pub(super) struct ItemNameLayout {
    pub(super) table: Vec<u8>,
    pub(super) payload: Vec<u8>,
    pub(super) entries: Vec<EncodedItemName>,
}

#[derive(Debug)]
pub(super) struct ItemQuotedEntry {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) jp: String,
    pub(super) ko: String,
}

#[derive(Debug)]
pub(super) struct EncodedItemQuoted {
    pub(super) id: String,
    pub(super) old_offset: usize,
    pub(super) new_offset: usize,
    pub(super) words: Vec<u16>,
}

#[derive(Debug)]
pub(super) struct ItemQuotedLayout {
    pub(super) table: Vec<u8>,
    pub(super) payload: Vec<u8>,
    pub(super) entries: Vec<EncodedItemQuoted>,
}

pub(super) fn build_m3_item_name_layout(
    source: &[u8],
    translation_dir: &Path,
    charmap: &BTreeMap<char, u16>,
) -> Result<ItemNameLayout, String> {
    let table_len = JP_ITEM_NAME_TABLE_COUNT * 2;
    let table_source = source
        .get(JP_ITEM_NAME_TABLE..JP_ITEM_NAME_TABLE + table_len)
        .ok_or("M3 item-name pointer table is outside the source ROM")?;
    let first_offset = u16::from_be_bytes([table_source[0], table_source[1]]) as usize;
    if first_offset != table_len || JP_ITEM_NAME_TABLE + table_len != JP_ITEM_NAME_DATA_START {
        return Err(format!(
            "M3 item-name table boundary mismatch: first offset 0x{first_offset:04X}"
        ));
    }

    let old_targets: Vec<usize> = table_source
        .chunks_exact(2)
        .map(|pair| JP_ITEM_NAME_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize)
        .collect();
    if old_targets.iter().any(|&target| {
        !(JP_ITEM_NAME_DATA_START..JP_ITEM_NAME_DATA_END).contains(&target)
            || !target.is_multiple_of(2)
    }) {
        return Err("M3 item-name pointer targets escape the original data block".to_string());
    }
    let unique_old_targets: BTreeSet<usize> = old_targets.iter().copied().collect();
    if unique_old_targets.len() != JP_ITEM_NAME_UNIQUE_COUNT {
        return Err(format!(
            "M3 item-name source has {} unique targets, expected {JP_ITEM_NAME_UNIQUE_COUNT}",
            unique_old_targets.len()
        ));
    }

    let asset_entries = load_m3_item_name_entries(translation_dir)?;
    let asset_offsets: BTreeSet<usize> = asset_entries.iter().map(|entry| entry.offset).collect();
    if asset_offsets != unique_old_targets {
        return Err(
            "M3 item-name asset offsets do not match the JP pointer population".to_string(),
        );
    }

    let mut payload = Vec::new();
    let mut entries = Vec::with_capacity(asset_entries.len());
    let mut new_offsets = BTreeMap::new();
    for (index, entry) in asset_entries.into_iter().enumerate() {
        let next_old_offset = unique_old_targets
            .iter()
            .nth(index + 1)
            .copied()
            .unwrap_or(JP_ITEM_NAME_DATA_END);
        validate_m3_jp_item_source(source, &entry, next_old_offset)?;

        let words =
            encode_jp_native_tokens(&crate::build::text::parse_display_text(&entry.ko), charmap)?;
        validate_m3_item_name_words(&entry.id, &words)?;
        let new_offset = JP_ITEM_NAME_DATA_START + payload.len();
        new_offsets.insert(entry.offset, new_offset);
        payload.extend_from_slice(&words_to_bytes(&words));
        entries.push(EncodedItemName {
            id: entry.id,
            old_offset: entry.offset,
            new_offset,
            words,
        });
    }
    if payload.len() > JP_ITEM_NAME_DATA_END - JP_ITEM_NAME_DATA_START {
        return Err(format!(
            "M3 item-name payload uses {} bytes, but the JP block holds {}",
            payload.len(),
            JP_ITEM_NAME_DATA_END - JP_ITEM_NAME_DATA_START
        ));
    }

    let mut table = Vec::with_capacity(table_len);
    for old_target in old_targets {
        let new_target = *new_offsets
            .get(&old_target)
            .ok_or_else(|| format!("M3 item-name target 0x{old_target:06X} has no asset"))?;
        let relative = new_target
            .checked_sub(JP_ITEM_NAME_TABLE)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| format!("M3 item-name target 0x{new_target:06X} is not rel16"))?;
        table.extend_from_slice(&relative.to_be_bytes());
    }

    Ok(ItemNameLayout {
        table,
        payload,
        entries,
    })
}

pub(super) fn load_m3_item_name_entries(
    translation_dir: &Path,
) -> Result<Vec<ItemNameEntry>, String> {
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
            if entry.get("section").and_then(|value| value.as_str()) != Some("item_name") {
                continue;
            }
            let id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or("M3 item-name entry is missing id")?
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
                return Err(format!("{id}: item-name status is not done: {status}"));
            }
            let jp = entry
                .get("jp")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: JP item name is empty"))?
                .to_string();
            let ko = entry
                .get("ko")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: KR item name is empty"))?
                .to_string();
            if !seen_ids.insert(id.clone()) {
                return Err(format!("{id}: duplicate item-name ID"));
            }
            if !seen_offsets.insert(offset) {
                return Err(format!("0x{offset:06X}: duplicate item-name offset"));
            }
            result.push(ItemNameEntry { id, offset, jp, ko });
        }
    }
    result.sort_by_key(|entry| entry.offset);
    if result.len() != JP_ITEM_NAME_UNIQUE_COUNT {
        return Err(format!(
            "M3 found {} item-name assets, expected {JP_ITEM_NAME_UNIQUE_COUNT}",
            result.len()
        ));
    }
    for (index, entry) in result.iter().enumerate() {
        let expected_id = format!("script_{:04}", 240 + index);
        if entry.id != expected_id {
            return Err(format!(
                "M3 item-name catalog drifted at index {index}: expected {expected_id}, got {}",
                entry.id
            ));
        }
    }
    Ok(result)
}

pub(super) fn validate_m3_jp_item_source(
    source: &[u8],
    entry: &ItemNameEntry,
    next_offset: usize,
) -> Result<(), String> {
    let jp = entry
        .jp
        .strip_suffix("{FFFF}")
        .ok_or_else(|| format!("{}: JP item name must end in FFFF", entry.id))?;
    if jp.contains('{') || jp.contains('}') {
        return Err(format!(
            "{}: JP item name has unexpected controls",
            entry.id
        ));
    }
    let expected_end = entry.offset + (jp.chars().count() + 1) * 2;
    if expected_end != next_offset {
        return Err(format!(
            "{}: JP source boundary ends at 0x{expected_end:06X}, expected 0x{next_offset:06X}",
            entry.id
        ));
    }
    let terminator = source
        .get(next_offset - 2..next_offset)
        .ok_or_else(|| format!("{}: JP source terminator is outside the ROM", entry.id))?;
    if terminator != [0xFF, 0xFF] {
        return Err(format!("{}: JP source does not end in FFFF", entry.id));
    }
    let charmap = crate::align::build_jp_charmap();
    let mut decoded = String::new();
    for offset in (entry.offset..next_offset - 2).step_by(2) {
        let word = u16::from_be_bytes([
            *source
                .get(offset)
                .ok_or_else(|| format!("{}: JP source is truncated", entry.id))?,
            *source
                .get(offset + 1)
                .ok_or_else(|| format!("{}: JP source is truncated", entry.id))?,
        ]);
        let glyph = charmap.get(&word).ok_or_else(|| {
            format!(
                "{}: JP source has unknown glyph 0x{word:04X} at 0x{offset:06X}",
                entry.id
            )
        })?;
        decoded.push_str(glyph);
    }
    if decoded != jp {
        return Err(format!(
            "{}: JP asset differs from ROM: {jp:?} != {decoded:?}",
            entry.id
        ));
    }
    Ok(())
}

pub(super) fn validate_m3_item_name_words(id: &str, words: &[u16]) -> Result<(), String> {
    let Some((&terminator, visible)) = words.split_last() else {
        return Err(format!("{id}: encoded item name is empty"));
    };
    if terminator != 0xFFFF {
        return Err(format!("{id}: encoded item name is missing FFFF"));
    }
    if visible.is_empty() {
        return Err(format!("{id}: encoded item name has no visible glyph"));
    }
    if visible.len() > JP_ITEM_NAME_VISIBLE_WORDS {
        return Err(format!(
            "{id}: item name uses {} glyphs, but JP FF78 displays at most {JP_ITEM_NAME_VISIBLE_WORDS}",
            visible.len()
        ));
    }
    if visible.iter().any(|&word| word >= 0xFF00) {
        return Err(format!(
            "{id}: item name contains a control word before FFFF"
        ));
    }
    Ok(())
}

pub(super) fn validate_m3_item_name_layout(
    rom: &[u8],
    layout: &ItemNameLayout,
) -> Result<(), String> {
    if rom.get(JP_ITEM_NAME_TABLE..JP_ITEM_NAME_TABLE + layout.table.len())
        != Some(layout.table.as_slice())
    {
        return Err("M3 item-name table re-extraction differs".to_string());
    }
    if rom.get(JP_ITEM_NAME_DATA_START..JP_ITEM_NAME_DATA_START + layout.payload.len())
        != Some(layout.payload.as_slice())
    {
        return Err("M3 item-name payload re-extraction differs".to_string());
    }

    let entries_by_offset: BTreeMap<usize, &EncodedItemName> = layout
        .entries
        .iter()
        .map(|entry| (entry.new_offset, entry))
        .collect();
    let mut referenced_offsets = BTreeSet::new();
    for pair in layout.table.chunks_exact(2) {
        let target = JP_ITEM_NAME_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize;
        let entry = entries_by_offset
            .get(&target)
            .ok_or_else(|| format!("M3 item-name pointer targets unknown 0x{target:06X}"))?;
        referenced_offsets.insert(target);

        let mut loader_buffer = [0u16; 8];
        for (index, slot) in loader_buffer.iter_mut().enumerate() {
            let offset = target + index * 2;
            let word = u16::from_be_bytes([
                *rom.get(offset)
                    .ok_or_else(|| format!("{}: item-name read is truncated", entry.id))?,
                *rom.get(offset + 1)
                    .ok_or_else(|| format!("{}: item-name read is truncated", entry.id))?,
            ]);
            if word == 0xFFFF {
                break;
            }
            *slot = word;
        }
        let expected_visible = &entry.words[..entry.words.len() - 1];
        for (index, &word) in loader_buffer[..JP_ITEM_NAME_VISIBLE_WORDS]
            .iter()
            .enumerate()
        {
            let expected = expected_visible.get(index).copied().unwrap_or(0);
            if word != expected {
                return Err(format!(
                    "{}: JP loader/FF78 emulation differs at glyph {index}",
                    entry.id
                ));
            }
        }
    }
    if referenced_offsets.len() != layout.entries.len() {
        return Err("M3 item-name table does not reference every packed entry".to_string());
    }
    if layout
        .entries
        .iter()
        .any(|entry| entry.old_offset < JP_ITEM_NAME_DATA_START)
    {
        return Err("M3 item-name layout retained an invalid source offset".to_string());
    }
    Ok(())
}

pub(super) fn build_m5_item_quoted_layout(
    source: &[u8],
    translation_dir: &Path,
    charmap: &BTreeMap<char, u16>,
) -> Result<ItemQuotedLayout, String> {
    let table_len = JP_ITEM_QUOTED_TABLE_COUNT * 2;
    let table_source = source
        .get(JP_ITEM_QUOTED_TABLE..JP_ITEM_QUOTED_TABLE + table_len)
        .ok_or("M5 quoted-item pointer table is outside the source ROM")?;
    let first_offset = u16::from_be_bytes([table_source[0], table_source[1]]) as usize;
    if first_offset != table_len || JP_ITEM_QUOTED_TABLE + table_len != JP_ITEM_QUOTED_DATA_START {
        return Err(format!(
            "M5 quoted-item table boundary mismatch: first offset 0x{first_offset:04X}"
        ));
    }

    let old_targets: Vec<usize> = table_source
        .chunks_exact(2)
        .map(|pair| JP_ITEM_QUOTED_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize)
        .collect();
    if old_targets.iter().any(|&target| {
        !(JP_ITEM_QUOTED_DATA_START..JP_ITEM_QUOTED_DATA_END).contains(&target)
            || !target.is_multiple_of(2)
    }) {
        return Err("M5 quoted-item pointer targets escape the original data block".to_string());
    }
    let unique_old_targets: BTreeSet<usize> = old_targets.iter().copied().collect();
    if unique_old_targets.len() != JP_ITEM_QUOTED_UNIQUE_COUNT {
        return Err(format!(
            "M5 quoted-item source has {} unique targets, expected {JP_ITEM_QUOTED_UNIQUE_COUNT}",
            unique_old_targets.len()
        ));
    }

    let asset_entries = load_m5_item_quoted_entries(translation_dir)?;
    let asset_offsets: BTreeSet<usize> = asset_entries.iter().map(|entry| entry.offset).collect();
    if asset_offsets != unique_old_targets {
        return Err(
            "M5 quoted-item asset offsets do not match the JP pointer population".to_string(),
        );
    }

    let mut payload = Vec::new();
    let mut entries = Vec::with_capacity(asset_entries.len());
    let mut new_offsets = BTreeMap::new();
    for (index, entry) in asset_entries.into_iter().enumerate() {
        let next_old_offset = unique_old_targets
            .iter()
            .nth(index + 1)
            .copied()
            .unwrap_or(JP_ITEM_QUOTED_DATA_END);
        validate_m5_jp_item_quoted_source(source, &entry, next_old_offset)?;

        let tokens = normalize_m5_item_quoted_tokens(
            &entry.id,
            &crate::build::text::parse_display_text(&entry.ko),
        )?;
        let words = encode_m5_item_quoted_tokens(&entry.id, &tokens, charmap)?;
        validate_m5_item_quoted_words(&entry.id, &words, charmap)?;
        let new_offset = JP_ITEM_QUOTED_DATA_START + payload.len();
        new_offsets.insert(entry.offset, new_offset);
        payload.extend_from_slice(&words_to_bytes(&words));
        entries.push(EncodedItemQuoted {
            id: entry.id,
            old_offset: entry.offset,
            new_offset,
            words,
        });
    }
    if payload.len() > JP_ITEM_QUOTED_DATA_END - JP_ITEM_QUOTED_DATA_START {
        return Err(format!(
            "M5 quoted-item payload uses {} bytes, but the JP block holds {}",
            payload.len(),
            JP_ITEM_QUOTED_DATA_END - JP_ITEM_QUOTED_DATA_START
        ));
    }

    let mut table = Vec::with_capacity(table_len);
    for old_target in old_targets {
        let new_target = *new_offsets
            .get(&old_target)
            .ok_or_else(|| format!("M5 quoted-item target 0x{old_target:06X} has no asset"))?;
        let relative = new_target
            .checked_sub(JP_ITEM_QUOTED_TABLE)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| format!("M5 quoted-item target 0x{new_target:06X} is not rel16"))?;
        table.extend_from_slice(&relative.to_be_bytes());
    }

    Ok(ItemQuotedLayout {
        table,
        payload,
        entries,
    })
}

pub(super) fn load_m5_item_quoted_entries(
    translation_dir: &Path,
) -> Result<Vec<ItemQuotedEntry>, String> {
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
            if entry.get("section").and_then(|value| value.as_str()) != Some("item_quoted") {
                continue;
            }
            let id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or("M5 quoted-item entry is missing id")?
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
                return Err(format!("{id}: quoted-item status is not done: {status}"));
            }
            let jp = entry
                .get("jp")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: JP quoted item is empty"))?
                .to_string();
            let ko = entry
                .get("ko")
                .and_then(|value| value.as_str())
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{id}: KR quoted item is empty"))?
                .to_string();
            if !seen_ids.insert(id.clone()) {
                return Err(format!("{id}: duplicate quoted-item ID"));
            }
            if !seen_offsets.insert(offset) {
                return Err(format!("0x{offset:06X}: duplicate quoted-item offset"));
            }
            result.push(ItemQuotedEntry { id, offset, jp, ko });
        }
    }
    result.sort_by_key(|entry| entry.offset);
    if result.len() != JP_ITEM_QUOTED_UNIQUE_COUNT {
        return Err(format!(
            "M5 found {} quoted-item assets, expected {JP_ITEM_QUOTED_UNIQUE_COUNT}",
            result.len()
        ));
    }
    for (index, entry) in result.iter().enumerate() {
        let expected_id = format!("script_{:04}", 281 + index);
        if entry.id != expected_id {
            return Err(format!(
                "M5 quoted-item catalog drifted at index {index}: expected {expected_id}, got {}",
                entry.id
            ));
        }
    }
    Ok(result)
}

pub(super) fn validate_m5_jp_item_quoted_source(
    source: &[u8],
    entry: &ItemQuotedEntry,
    next_offset: usize,
) -> Result<(), String> {
    let source_bytes = source
        .get(entry.offset..next_offset)
        .ok_or_else(|| format!("{}: JP quoted-item source is outside the ROM", entry.id))?;
    if source_bytes.len() < 8 || !source_bytes.len().is_multiple_of(2) {
        return Err(format!(
            "{}: JP quoted-item source has an invalid size",
            entry.id
        ));
    }
    let words: Vec<u16> = source_bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    if words.first() != Some(&0xFF10)
        || words.get(1) != Some(&JP_ITEM_QUOTED_OPEN)
        || words.get(words.len() - 2) != Some(&JP_ITEM_QUOTED_CLOSE)
        || words.last() != Some(&0xFFFF)
    {
        return Err(format!(
            "{}: JP quoted-item wrapper differs from FF10/007E/007F/FFFF",
            entry.id
        ));
    }

    let jp_charmap = crate::align::build_jp_charmap();
    let mut decoded = String::from("{FF10}『");
    for (index, &word) in words[2..words.len() - 2].iter().enumerate() {
        match word {
            0x0000 => decoded.push(' '),
            0xFF30 => decoded.push_str("{NL}"),
            code if code >= 0xFF00 => {
                return Err(format!(
                    "{}: unexpected JP control 0x{code:04X} at word {}",
                    entry.id,
                    index + 2
                ));
            }
            code => decoded.push_str(jp_charmap.get(&code).ok_or_else(|| {
                format!(
                    "{}: JP source has unknown glyph 0x{code:04X} at word {}",
                    entry.id,
                    index + 2
                )
            })?),
        }
    }
    decoded.push_str("』{FFFF}");
    if decoded != entry.jp {
        return Err(format!(
            "{}: JP asset differs from ROM: {:?} != {decoded:?}",
            entry.id, entry.jp
        ));
    }
    Ok(())
}

pub(super) fn normalize_m5_item_quoted_tokens(
    id: &str,
    tokens: &[Token],
) -> Result<Vec<Token>, String> {
    if tokens.len() < 4
        || tokens.first() != Some(&Token::Ctrl(0xFF10))
        || tokens.get(1) != Some(&Token::Tile(0x0048))
        || tokens.get(tokens.len() - 2) != Some(&Token::Tile(0x0049))
        || tokens.last() != Some(&Token::Ctrl(0xFFFF))
    {
        return Err(format!(
            "{id}: KR quoted item must use FF10/q-open/q/FFFF wrapper"
        ));
    }

    let mut normalized = Vec::with_capacity(tokens.len());
    let mut index = 0usize;
    while index < tokens.len() {
        let en_layout_tail = tokens.get(index..index + 6);
        if matches!(
            en_layout_tail,
            Some([
                Token::Ctrl(0xFF30),
                Token::EnChar(' '),
                Token::EnChar(' '),
                Token::EnChar(' '),
                Token::Tile(0x0077),
                Token::Tile(0x0073) | Token::Tile(0x0076),
            ])
        ) {
            normalized.extend_from_slice(&[Token::Ctrl(0xFF30), Token::EnChar(' ')]);
            index += 6;
        } else {
            normalized.push(tokens[index].clone());
            index += 1;
        }
    }

    loop {
        let mut line_start = 0usize;
        let mut wrapped = false;
        while line_start < normalized.len() {
            let line_end = normalized[line_start..]
                .iter()
                .position(|token| *token == Token::Ctrl(0xFF30))
                .map(|relative| line_start + relative)
                .unwrap_or(normalized.len());
            let visible_half_cells = normalized[line_start..line_end]
                .iter()
                .map(fixed_width_token_half_cells)
                .sum::<usize>();
            if visible_half_cells > JP_ITEM_QUOTED_MAX_GLYPHS_PER_LINE * 2 {
                let split = normalized[line_start..line_end]
                    .iter()
                    .rposition(|token| *token == Token::EnChar(' '))
                    .map(|relative| line_start + relative)
                    .ok_or_else(|| {
                        format!(
                            "{id}: quoted-item line exceeds {JP_ITEM_QUOTED_MAX_GLYPHS_PER_LINE} cells and has no safe word boundary"
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

pub(super) fn encode_m5_item_quoted_tokens(
    id: &str,
    tokens: &[Token],
    charmap: &BTreeMap<char, u16>,
) -> Result<Vec<u16>, String> {
    let mut words = Vec::with_capacity(tokens.len());
    for token in tokens {
        match token {
            Token::Ctrl(code @ (0xFF10 | 0xFF30 | 0xFFFF)) => words.push(*code),
            Token::Ctrl(code) => {
                return Err(format!(
                    "{id}: unsupported quoted-item control 0x{code:04X}"
                ));
            }
            Token::CtrlParam(code, _) => {
                return Err(format!(
                    "{id}: parameterized control 0x{code:04X} is not valid in quoted items"
                ));
            }
            Token::Tile(0x0048) => words.push(JP_ITEM_QUOTED_OPEN),
            Token::Tile(0x0049) => words.push(JP_ITEM_QUOTED_CLOSE),
            Token::Tile(code) => {
                return Err(format!(
                    "{id}: EN-derived named tile 0x{code:04X} has no M5 surface mapping"
                ));
            }
            Token::LayoutPad | Token::SourceRowFinalize { .. } => {
                return Err(format!(
                    "{id}: source-backed layout padding is not valid in quoted items"
                ));
            }
            Token::KrChar(ch) => words.push(*charmap.get(ch).ok_or_else(|| {
                format!(
                    "{id}: JP-native glyph is missing: '{ch}' (U+{:04X})",
                    *ch as u32
                )
            })?),
            Token::EnChar(' ') => words.push(*charmap.get(&' ').ok_or_else(|| {
                format!("{id}: quoted-item space glyph is missing from the JP-native charmap")
            })?),
            Token::EnChar(ch) => words.push(*charmap.get(ch).ok_or_else(|| {
                format!(
                    "{id}: unsupported quoted-item character: '{ch}' (U+{:04X})",
                    *ch as u32
                )
            })?),
            Token::Raw(code) => {
                return Err(format!(
                    "{id}: raw text word 0x{code:04X} is not valid in quoted items"
                ));
            }
        }
    }
    Ok(words)
}

pub(super) fn validate_m5_item_quoted_words(
    id: &str,
    words: &[u16],
    charmap: &BTreeMap<char, u16>,
) -> Result<(), String> {
    if words.len() < 4
        || words.first() != Some(&0xFF10)
        || words.get(1) != Some(&JP_ITEM_QUOTED_OPEN)
        || words.get(words.len() - 2) != Some(&JP_ITEM_QUOTED_CLOSE)
        || words.last() != Some(&0xFFFF)
    {
        return Err(format!("{id}: encoded quoted-item wrapper is invalid"));
    }
    let mut line = 1usize;
    let mut half_cells = 0usize;
    let half_width_codes = JP_NATIVE_HALF_WIDTH_CHARS
        .iter()
        .filter_map(|ch| charmap.get(ch).copied())
        .collect::<BTreeSet<_>>();
    for &word in &words[1..words.len() - 1] {
        if word == 0xFF30 {
            line += 1;
            half_cells = 0;
            continue;
        }
        if word >= 0xFF00 {
            return Err(format!(
                "{id}: quoted-item body contains unsupported control 0x{word:04X}"
            ));
        }
        half_cells += if half_width_codes.contains(&word) {
            1
        } else {
            2
        };
        if half_cells > JP_ITEM_QUOTED_MAX_GLYPHS_PER_LINE * 2 {
            return Err(format!(
                "{id}: quoted-item line {line} exceeds {JP_ITEM_QUOTED_MAX_GLYPHS_PER_LINE} cells"
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_m5_item_quoted_layout(
    rom: &[u8],
    layout: &ItemQuotedLayout,
) -> Result<(), String> {
    if rom.get(JP_ITEM_QUOTED_TABLE..JP_ITEM_QUOTED_TABLE + layout.table.len())
        != Some(layout.table.as_slice())
    {
        return Err("M5 quoted-item table re-extraction differs".to_string());
    }
    if rom.get(JP_ITEM_QUOTED_DATA_START..JP_ITEM_QUOTED_DATA_START + layout.payload.len())
        != Some(layout.payload.as_slice())
    {
        return Err("M5 quoted-item payload re-extraction differs".to_string());
    }

    let entries_by_offset: BTreeMap<usize, &EncodedItemQuoted> = layout
        .entries
        .iter()
        .map(|entry| (entry.new_offset, entry))
        .collect();
    let mut referenced_offsets = BTreeSet::new();
    for pair in layout.table.chunks_exact(2) {
        let target = JP_ITEM_QUOTED_TABLE + u16::from_be_bytes([pair[0], pair[1]]) as usize;
        let entry = entries_by_offset
            .get(&target)
            .ok_or_else(|| format!("M5 quoted-item pointer targets unknown 0x{target:06X}"))?;
        referenced_offsets.insert(target);
        let expected = words_to_bytes(&entry.words);
        if rom.get(target..target + expected.len()) != Some(expected.as_slice()) {
            return Err(format!(
                "{}: quoted-item direct-consumer re-extraction differs",
                entry.id
            ));
        }
    }
    if referenced_offsets.len() != layout.entries.len() {
        return Err("M5 quoted-item table does not reference every packed entry".to_string());
    }
    if layout
        .entries
        .iter()
        .any(|entry| entry.old_offset < JP_ITEM_QUOTED_DATA_START)
    {
        return Err("M5 quoted-item layout retained an invalid source offset".to_string());
    }
    Ok(())
}
