pub mod font;
pub mod hook;
pub mod items;
pub mod plural;
pub mod text;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::translation;

/// ROM layout constants
const EN_PTR_TABLE: usize = 0x210000;
const KR_TEXT_BASE: usize = 0x280000;
const ROM_SIZE: usize = 0x400000;
const CHECKSUM_OFFSET: usize = 0x18E;

/// Build configuration
///
/// Assets directory layout:
///   assets/
///   ├── translation/           ← script_*.json
///   ├── charmap.json           ← EN tile mapping
///   └── neodgm.ttf             ← Korean font (TTF)
pub struct BuildConfig<'a> {
    pub en_rom_path: &'a Path,
    pub assets_dir: &'a Path,
    pub output_path: &'a Path,
}

impl<'a> BuildConfig<'a> {
    fn translation_dir(&self) -> std::path::PathBuf {
        self.assets_dir.join("translation")
    }
    fn charmap_path(&self) -> std::path::PathBuf {
        self.assets_dir.join("charmap.json")
    }
    fn font_path(&self) -> std::path::PathBuf {
        self.assets_dir.join("neodgm.ttf")
    }
    fn text_en_path(&self) -> std::path::PathBuf {
        self.assets_dir.join("translation").join("text_en.json")
    }
}

/// Load translations from translation directory (script_*.json format).
fn load_translations(translation_dir: &Path) -> Result<HashMap<String, String>, String> {
    eprintln!("  format: Script");
    translation::load_translations(translation_dir)
}

/// Scan FFF8 pointer table to determine entry count.
fn scan_fff8_table(rom: &[u8]) -> usize {
    let first_ptr = u32::from_be_bytes([
        rom[EN_PTR_TABLE],
        rom[EN_PTR_TABLE + 1],
        rom[EN_PTR_TABLE + 2],
        rom[EN_PTR_TABLE + 3],
    ]) as usize;

    if EN_PTR_TABLE < first_ptr && first_ptr <= 0x3FFFFF {
        let table_entries = (first_ptr - EN_PTR_TABLE) / 4;
        table_entries - 1 // 0-based index
    } else {
        // Fallback: scan for valid pointers
        let mut max_valid = 0;
        for i in 0..4096 {
            let addr = EN_PTR_TABLE + i * 4;
            if addr + 4 > rom.len() {
                break;
            }
            let ptr = u32::from_be_bytes([rom[addr], rom[addr + 1], rom[addr + 2], rom[addr + 3]])
                as usize;
            if (0x200000..=0x3FFFFF).contains(&ptr) {
                max_valid = i;
            }
        }
        max_valid
    }
}

/// Read raw EN text bytes from ROM at pointer (until terminator).
fn read_en_text_raw(rom: &[u8], ptr: usize) -> Vec<u8> {
    let ctrl_params = text::ctrl_with_param();
    let terms = text::terminators();
    let max_pos = std::cmp::min(ptr + 4096, rom.len());

    let mut pos = ptr;
    while pos + 1 < max_pos {
        let word = u16::from_be_bytes([rom[pos], rom[pos + 1]]);
        pos += 2;
        if terms.contains(&word) {
            break;
        }
        if ctrl_params.contains(&word) && pos < max_pos - 1 {
            pos += 2; // skip parameter
        }
    }
    rom[ptr..pos].to_vec()
}

/// Update ROM checksum at offset 0x18E.
fn update_checksum(rom: &mut [u8]) {
    let mut checksum: u16 = 0;
    let mut i = 0x200;
    while i + 1 < rom.len() {
        let word = u16::from_be_bytes([rom[i], rom[i + 1]]);
        checksum = checksum.wrapping_add(word);
        i += 2;
    }
    rom[CHECKSUM_OFFSET] = (checksum >> 8) as u8;
    rom[CHECKSUM_OFFSET + 1] = (checksum & 0xFF) as u8;
    eprintln!("  checksum: 0x{checksum:04X}");
}

/// Run the full KR ROM build pipeline.
///
/// Returns the built KR ROM data.
pub fn build_kr_rom(config: &BuildConfig) -> Result<Vec<u8>, String> {
    eprintln!("============================================================");
    eprintln!("Madou Monogatari I - KR Patch ROM Build (Rust)");
    eprintln!("============================================================");

    // Step 1: Load EN ROM
    eprintln!("\n[1/7] Loading EN ROM");
    let mut rom =
        fs::read(config.en_rom_path).map_err(|e| format!("failed to read EN ROM: {e}"))?;
    eprintln!("  size: {} bytes (0x{:X})", rom.len(), rom.len());
    if rom.len() != ROM_SIZE {
        return Err(format!(
            "EN ROM size mismatch: expected 0x{ROM_SIZE:X}, got 0x{:X}",
            rom.len()
        ));
    }

    // Step 2: Load translations
    eprintln!("\n[2/7] Loading translations");
    let translation_dir = config.translation_dir();
    let translations = load_translations(&translation_dir)?;

    // Step 3: Font insertion
    eprintln!("\n[3/7] Font insertion");
    let en_charmap = text::load_en_charmap(&config.charmap_path())?;
    let translation_paths = translation::list_translation_paths(&translation_dir)?;
    let unique_chars = text::collect_unique_kr_chars(&translation_paths, &en_charmap)?;
    let kr_charmap = text::build_kr_charmap(&unique_chars);
    eprintln!("  unique KR chars: {}", unique_chars.len());

    let char_count = font::render_and_insert_font(&mut rom, &config.font_path(), &unique_chars)?;
    eprintln!("  inserted {char_count} font characters");

    // Step 4: 68K hook code
    eprintln!("\n[4/7] 68K hook insertion");
    hook::apply_hook(&mut rom)?;
    let hook_code = hook::assemble_hook()?;
    eprintln!(
        "  hook code: {} bytes at 0x{:06X}",
        hook_code.len(),
        hook::HOOK_ADDR
    );

    // Step 4b: FFD0 item patches
    eprintln!("\n[4b] FFD0 item name patches");
    items::patch_ffd0_items(&mut rom)?;
    eprintln!("  3 patches applied");

    // Step 5: Text encoding & insertion
    eprintln!("\n[5/7] Text encoding & insertion");
    let (translated, fallback, text_offset) = insert_extended_text(
        &mut rom,
        &translations,
        &kr_charmap,
        &en_charmap,
        &config.text_en_path(),
        config.en_rom_path,
    )?;

    // Step 5b: Cait Sith plural-form hardcoded text patches
    eprintln!("\n[5b] Cait Sith plural text patches");
    let _text_offset = plural::patch_plural_texts(
        &mut rom,
        text_offset,
        &kr_charmap,
        &en_charmap,
        &translations,
    )?;

    // Step 6: Checksum
    eprintln!("\n[6/7] Checksum update");
    update_checksum(&mut rom);

    // Step 7: Save
    eprintln!("\n[7/7] Build complete");
    eprintln!(
        "  KR chars: {} (0x0100 - 0x{:04X})",
        unique_chars.len(),
        0x0100 + unique_chars.len() - 1
    );
    eprintln!("  translated: {translated}, fallback: {fallback}");

    Ok(rom)
}

/// Encode and insert all FFF8 extended text into ROM.
///
/// Phase A: Entries mapped via text_en.json extended_entry_map
/// Phase B: Remaining entries (direct EN ROM reference)
fn insert_extended_text(
    rom: &mut [u8],
    translations: &HashMap<String, String>,
    kr_charmap: &HashMap<char, u16>,
    en_charmap: &HashMap<char, u16>,
    text_en_path: &Path,
    en_rom_path: &Path,
) -> Result<(usize, usize, usize), String> {
    // Load text_en.json
    let text_en_data = fs::read_to_string(text_en_path)
        .map_err(|e| format!("failed to read text_en.json: {e}"))?;
    let text_en: serde_json::Value = serde_json::from_str(&text_en_data)
        .map_err(|e| format!("failed to parse text_en.json: {e}"))?;

    // Extract extended_dialogs → dedup_id → raw words
    let extended_dialogs = text_en
        .get("extended_dialogs")
        .and_then(|v| v.as_array())
        .ok_or("missing extended_dialogs in text_en.json")?;

    let mut dialog_raw: HashMap<String, Vec<u16>> = HashMap::new();
    let mut all_dialog_ids: Vec<String> = Vec::new();

    for entry in extended_dialogs {
        let dedup_id = entry
            .get("dedup_id")
            .and_then(|v| v.as_str())
            .ok_or("missing dedup_id")?
            .to_string();

        let raw = entry
            .get("raw")
            .and_then(|v| v.as_array())
            .ok_or_else(|| format!("missing raw for {dedup_id}"))?;

        let words: Vec<u16> = raw
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as u16))
            .collect();

        all_dialog_ids.push(dedup_id.clone());
        dialog_raw.insert(dedup_id, words);
    }

    // Extract extended_entry_map
    let entry_map = text_en
        .get("extended_entry_map")
        .and_then(|v| v.as_object())
        .ok_or("missing extended_entry_map")?;

    let mapped_max: usize = entry_map
        .keys()
        .filter_map(|k| k.parse::<usize>().ok())
        .max()
        .unwrap_or(0);

    // Load EN ROM for Phase B
    let en_rom_data =
        fs::read(en_rom_path).map_err(|e| format!("failed to read EN ROM for text ref: {e}"))?;
    let max_entry = scan_fff8_table(&en_rom_data);

    eprintln!(
        "  text_en.json dialogs: {} (entries 0-{mapped_max})",
        all_dialog_ids.len()
    );
    eprintln!("  EN ROM FFF8 entries: {} (0-{max_entry})", max_entry + 1);

    // Phase A: Encode dialogs
    let mut dialog_encoded: HashMap<String, Vec<u8>> = HashMap::new();
    let mut translated_count = 0usize;
    let mut fallback_count = 0usize;

    for dialog_id in &all_dialog_ids {
        if let Some(kr_text) = translations.get(dialog_id) {
            match text::encode_text(kr_text, kr_charmap, en_charmap) {
                Ok(words) => {
                    dialog_encoded.insert(dialog_id.clone(), text::words_to_bytes(&words));
                    translated_count += 1;
                    continue;
                }
                Err(e) => {
                    eprintln!("  [warning] {dialog_id} encode failed: {e}");
                }
            }
        }
        // Fallback: use raw EN data
        if let Some(raw_words) = dialog_raw.get(dialog_id) {
            dialog_encoded.insert(dialog_id.clone(), text::words_to_bytes(raw_words));
        }
        fallback_count += 1;
    }

    // Write text data and track offsets
    let mut text_offset = KR_TEXT_BASE;
    let mut dialog_offsets: HashMap<String, usize> = HashMap::new();

    for dialog_id in &all_dialog_ids {
        if let Some(encoded) = dialog_encoded.get(dialog_id) {
            dialog_offsets.insert(dialog_id.clone(), text_offset);
            if text_offset + encoded.len() > rom.len() {
                return Err(format!("ROM overflow at 0x{:06X}", text_offset));
            }
            rom[text_offset..text_offset + encoded.len()].copy_from_slice(encoded);
            text_offset += encoded.len();
        }
    }

    eprintln!(
        "  Phase A (0-{mapped_max}): translated {translated_count}, fallback {fallback_count}"
    );

    // Update pointers for Phase A entries
    let mut ptr_updated_a = 0usize;
    for (entry_idx_str, dialog_ids_val) in entry_map {
        let entry_idx: usize = match entry_idx_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let dialog_ids = match dialog_ids_val.as_array() {
            Some(arr) => arr,
            None => continue,
        };
        if dialog_ids.is_empty() {
            continue;
        }

        let first_dialog = match dialog_ids[0].as_str() {
            Some(s) => s,
            None => continue,
        };
        if let Some(&offset) = dialog_offsets.get(first_dialog) {
            let ptr_addr = EN_PTR_TABLE + entry_idx * 4;
            let offset_bytes = (offset as u32).to_be_bytes();
            rom[ptr_addr..ptr_addr + 4].copy_from_slice(&offset_bytes);
            ptr_updated_a += 1;
        }
    }

    // Phase B: Entries beyond mapped_max (direct EN ROM references)
    let mut ext_translated = 0usize;
    let mut ext_fallback_copy = 0usize;
    let mut ext_dedup = 0usize;
    let mut en_ptr_to_kr_offset: HashMap<usize, usize> = HashMap::new();

    for entry_idx in (mapped_max + 1)..=max_entry {
        let en_ptr_addr = EN_PTR_TABLE + entry_idx * 4;
        if en_ptr_addr + 4 > en_rom_data.len() {
            break;
        }

        let en_ptr = u32::from_be_bytes([
            en_rom_data[en_ptr_addr],
            en_rom_data[en_ptr_addr + 1],
            en_rom_data[en_ptr_addr + 2],
            en_rom_data[en_ptr_addr + 3],
        ]) as usize;

        if !(0x200000..=0x3FFFFF).contains(&en_ptr) {
            continue;
        }

        let dialog_id = format!("dialog_{entry_idx:04}");

        if let Some(kr_text) = translations.get(&dialog_id) {
            match text::encode_text(kr_text, kr_charmap, en_charmap) {
                Ok(words) => {
                    let encoded = text::words_to_bytes(&words);
                    if text_offset + encoded.len() > rom.len() {
                        return Err(format!("ROM overflow at 0x{:06X}", text_offset));
                    }
                    rom[text_offset..text_offset + encoded.len()].copy_from_slice(&encoded);
                    let offset_bytes = (text_offset as u32).to_be_bytes();
                    let kr_ptr_addr = EN_PTR_TABLE + entry_idx * 4;
                    rom[kr_ptr_addr..kr_ptr_addr + 4].copy_from_slice(&offset_bytes);
                    text_offset += encoded.len();
                    ext_translated += 1;
                    continue;
                }
                Err(e) => {
                    eprintln!("  [warning] {dialog_id} encode failed: {e}");
                }
            }
        }

        // Fallback: copy EN raw data (with dedup)
        if let Some(&kr_offset) = en_ptr_to_kr_offset.get(&en_ptr) {
            let offset_bytes = (kr_offset as u32).to_be_bytes();
            let kr_ptr_addr = EN_PTR_TABLE + entry_idx * 4;
            rom[kr_ptr_addr..kr_ptr_addr + 4].copy_from_slice(&offset_bytes);
            ext_dedup += 1;
        } else {
            let raw_data = read_en_text_raw(&en_rom_data, en_ptr);
            if text_offset + raw_data.len() > rom.len() {
                return Err(format!("ROM overflow at 0x{:06X}", text_offset));
            }
            rom[text_offset..text_offset + raw_data.len()].copy_from_slice(&raw_data);
            let offset_bytes = (text_offset as u32).to_be_bytes();
            let kr_ptr_addr = EN_PTR_TABLE + entry_idx * 4;
            rom[kr_ptr_addr..kr_ptr_addr + 4].copy_from_slice(&offset_bytes);
            en_ptr_to_kr_offset.insert(en_ptr, text_offset);
            text_offset += raw_data.len();
            ext_fallback_copy += 1;
        }
    }

    let ptr_updated_b = ext_translated + ext_fallback_copy + ext_dedup;

    eprintln!(
        "  Phase B ({}-{max_entry}): translated {ext_translated}, \
               EN copy {ext_fallback_copy}, dedup {ext_dedup}",
        mapped_max + 1
    );

    let text_total = text_offset - KR_TEXT_BASE;
    eprintln!(
        "  text area: 0x{KR_TEXT_BASE:06X} - 0x{text_offset:06X} ({text_total} bytes, {:.1} KB)",
        text_total as f64 / 1024.0
    );
    eprintln!(
        "  pointers updated: {} (A:{ptr_updated_a} + B:{ptr_updated_b})",
        ptr_updated_a + ptr_updated_b
    );

    let total_translated = translated_count + ext_translated;
    let total_fallback = fallback_count + ext_fallback_copy + ext_dedup;
    Ok((total_translated, total_fallback, text_offset))
}
