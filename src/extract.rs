//! `init` 명령: EN ROM에서 파생 파일 3개 생성.
//!
//! - `charmap.json` — 타일↔문자 매핑
//! - `en_reference.json` — 전체 FFF8 엔트리 디코딩 (check-ctrl용)
//! - `translation/text_en.json` — 확장 다이얼로그 + entry_map (build용)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::build::text;

/// EN FFF8 pointer table base address.
const EN_PTR_TABLE: usize = 0x210000;

/// Number of FFF8 entries to scan for text_en.json (matching extract_translation.py).
const TEXT_EN_SCAN_ENTRIES: usize = 512;

// ============================================================
// Helpers
// ============================================================

fn read_u32_be(rom: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        rom[offset],
        rom[offset + 1],
        rom[offset + 2],
        rom[offset + 3],
    ])
}

/// Determine FFF8 entry count from first pointer.
fn scan_entry_count(rom: &[u8]) -> usize {
    let first_ptr = read_u32_be(rom, EN_PTR_TABLE) as usize;
    if EN_PTR_TABLE < first_ptr && first_ptr <= 0x3FFFFF {
        (first_ptr - EN_PTR_TABLE) / 4 - 1
    } else {
        0
    }
}

/// Return one FFF8 pointer and its next greater packed-data pointer.
///
/// A later pointer may intentionally enter a shared tail, so this boundary is
/// not a general script terminator. It is used only to recognize exact
/// two-word parameter-only leaf entries such as `FF78 FF04`.
fn fff8_entry_bounds(rom: &[u8], idx: usize) -> Option<(usize, usize)> {
    let max_entry = scan_entry_count(rom);
    if idx > max_entry {
        return None;
    }
    let ptr_addr = EN_PTR_TABLE.checked_add(idx.checked_mul(4)?)?;
    if ptr_addr + 4 > rom.len() {
        return None;
    }
    let start = read_u32_be(rom, ptr_addr) as usize;
    if !(0x200000..rom.len()).contains(&start) {
        return None;
    }

    let end = (0..=max_entry)
        .filter_map(|entry_idx| {
            let address = EN_PTR_TABLE + entry_idx * 4;
            (address + 4 <= rom.len()).then(|| read_u32_be(rom, address) as usize)
        })
        .filter(|pointer| *pointer > start && *pointer <= rom.len())
        .min()
        .unwrap_or(rom.len());
    Some((start, end))
}

fn read_fff8_entry_words(
    rom: &[u8],
    idx: usize,
    ctrl_params: &HashSet<u16>,
    terminators: &HashSet<u16>,
) -> Option<Vec<u16>> {
    let (start, end) = fff8_entry_bounds(rom, idx)?;
    let read_end = if end == start + 4 {
        let code = u16::from_be_bytes([rom[start], rom[start + 1]]);
        ctrl_params.contains(&code).then_some(end)
    } else {
        None
    }
    .unwrap_or(rom.len());
    Some(text::read_rom_words(
        rom.get(..read_end)?,
        start,
        ctrl_params,
        terminators,
    ))
}

// ============================================================
// charmap.json
// ============================================================

fn generate_charmap_json(assets_dir: &Path) -> Result<(), String> {
    let charmap = text::generate_charmap();
    let json =
        serde_json::to_string_pretty(&charmap).map_err(|e| format!("JSON serialize error: {e}"))?;
    let out_path = assets_dir.join("charmap.json");
    fs::write(&out_path, json.as_bytes())
        .map_err(|e| format!("failed to write charmap.json: {e}"))?;
    eprintln!("  charmap.json: {} entries", charmap.len());
    Ok(())
}

// ============================================================
// en_reference.json
// ============================================================

fn generate_en_reference(rom: &[u8], assets_dir: &Path) -> Result<(), String> {
    let charmap = text::generate_charmap();
    let tile_to_str = text::build_tile_to_display(&charmap);
    let ctrl_params = text::ctrl_with_param();
    let terminators: HashSet<u16> = [0xFF38, 0xFF04, 0xFFFF].into_iter().collect();

    let max_entry = scan_entry_count(rom);
    let mut entries = BTreeMap::new();

    for idx in 0..=max_entry {
        let Some(words) = read_fff8_entry_words(rom, idx, &ctrl_params, &terminators) else {
            continue;
        };
        let display = text::words_to_display_text(&words, &tile_to_str, &ctrl_params);
        if !display.is_empty() {
            entries.insert(format!("dialog_{idx:04}"), display);
        }
    }

    // Scan KR translation files for raw-byte entries
    let tr_dir = assets_dir.join("translation");
    let mut raw_byte_entries: BTreeMap<String, String> = BTreeMap::new();
    if tr_dir.is_dir() {
        let re_raw = regex_lite::Regex::new(r"\[[0-9A-Fa-f]{2,4}\]").unwrap();
        let mut kr_files: Vec<_> = fs::read_dir(&tr_dir)
            .map_err(|e| format!("read translation dir: {e}"))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("kr_") && n.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect();
        kr_files.sort();

        for path in &kr_files {
            let data =
                fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let obj: serde_json::Value = serde_json::from_str(&data)
                .map_err(|e| format!("parse {}: {e}", path.display()))?;
            if let Some(map) = obj.as_object() {
                let fname = path.file_name().unwrap().to_str().unwrap().to_string();
                for (key, val) in map {
                    if key.starts_with('_') {
                        continue;
                    }
                    if let Some(s) = val.as_str()
                        && re_raw.is_match(s)
                    {
                        raw_byte_entries.insert(key.clone(), fname.clone());
                    }
                }
            }
        }
    }

    let output = serde_json::json!({
        "_meta": {
            "description": "EN ROM reference text for all FFF8 entries",
            "total_entries": entries.len(),
            "raw_byte_kr_entries": raw_byte_entries.len(),
        },
        "entries": entries,
        "raw_byte_entries": raw_byte_entries,
    });

    let json =
        serde_json::to_string_pretty(&output).map_err(|e| format!("JSON serialize error: {e}"))?;
    let out_path = assets_dir.join("en_reference.json");
    fs::write(&out_path, json.as_bytes())
        .map_err(|e| format!("failed to write en_reference.json: {e}"))?;
    eprintln!(
        "  en_reference.json: {} entries, {} raw-byte KR",
        entries.len(),
        raw_byte_entries.len()
    );
    Ok(())
}

// ============================================================
// text_en.json
// ============================================================

/// A unique dialog segment extracted from FFF8 entries.
#[derive(serde::Serialize)]
struct ExtendedDialog {
    en: String,
    en_text: String,
    kr: String,
    raw: Vec<u16>,
    first_seen: String,
    dedup_id: String,
}

fn generate_text_en(rom: &[u8], assets_dir: &Path) -> Result<(), String> {
    let charmap = text::generate_charmap();
    let tile_to_str = text::build_tile_to_display(&charmap);
    let ctrl_params = text::ctrl_with_param_text_en();
    let block_end: HashSet<u16> = [0xFF38, 0xFFFF].into_iter().collect();

    // Phase 1: Read all FFF8 entries (0-511), parse words, split at FF04
    struct RawEntry {
        entry_idx: usize,
        dialogs: Vec<(String, Vec<u16>)>, // (display_text, raw_words)
    }

    let mut raw_entries: Vec<RawEntry> = Vec::new();

    for idx in 0..TEXT_EN_SCAN_ENTRIES {
        let Some(all_words) = read_fff8_entry_words(rom, idx, &ctrl_params, &block_end) else {
            continue;
        };

        // Split at FF04 and block_end boundaries into individual dialog segments
        let mut dialogs = Vec::new();
        let mut seg_start = 0;
        let mut i = 0;
        while i < all_words.len() {
            let w = all_words[i];
            if w == 0xFF04 || w == 0xFF38 || w == 0xFFFF {
                let seg = &all_words[seg_start..=i];
                let display = text::words_to_display_text(seg, &tile_to_str, &ctrl_params);
                dialogs.push((display, seg.to_vec()));
                seg_start = i + 1;
            } else if ctrl_params.contains(&w) {
                i += 1; // skip param word
            }
            i += 1;
        }
        // Any remaining words (shouldn't happen normally)
        if seg_start < all_words.len() {
            let seg = &all_words[seg_start..];
            let display = text::words_to_display_text(seg, &tile_to_str, &ctrl_params);
            dialogs.push((display, seg.to_vec()));
        }

        if !dialogs.is_empty() {
            raw_entries.push(RawEntry {
                entry_idx: idx,
                dialogs,
            });
        }
    }

    // Phase 2: Deduplicate by display text
    let mut unique_texts: HashMap<String, usize> = HashMap::new(); // display → index in deduped
    let mut deduped: Vec<(String, Vec<u16>, String)> = Vec::new(); // (display, raw, first_seen)

    for entry in &raw_entries {
        for (display, raw) in &entry.dialogs {
            if !unique_texts.contains_key(display) {
                let idx = deduped.len();
                unique_texts.insert(display.clone(), idx);
                let first_seen = format!("ext_{:03}", entry.entry_idx);
                deduped.push((display.clone(), raw.clone(), first_seen));
            }
        }
    }

    // Phase 3: Build extended_dialogs and extended_entry_map
    let extended_dialogs: Vec<ExtendedDialog> = deduped
        .iter()
        .enumerate()
        .map(|(i, (display, raw, first_seen))| {
            let en_text = text::words_to_readable_text(raw, &tile_to_str, &ctrl_params);
            ExtendedDialog {
                en: display.clone(),
                en_text,
                kr: String::new(),
                raw: raw.clone(),
                first_seen: first_seen.clone(),
                dedup_id: format!("dialog_{i:04}"),
            }
        })
        .collect();

    // entry_map: entry_idx → [dedup_id, ...]
    let mut entry_map: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for entry in &raw_entries {
        let dialog_ids: Vec<String> = entry
            .dialogs
            .iter()
            .map(|(display, _)| {
                let idx = unique_texts[display];
                format!("dialog_{idx:04}")
            })
            .collect();
        entry_map.insert(entry.entry_idx, dialog_ids);
    }

    // Phase 4: Serialize
    let output = serde_json::json!({
        "_meta": {
            "game": "Madou Monogatari I (Mega Drive)",
            "source": "English v1.1 ROM",
            "format": "Each entry has 'en' (display with control codes), 'en_text' (readable text), 'kr' (Korean translation to fill in)",
            "control_codes": {
                "{NL}": "줄바꿈 (FF30)",
                "{PAGE}": "페이지 넘김 (FF34)",
                "{END}": "다이얼로그 종료 (FF04)",
                "{FF50:XXXX}": "화자 표시",
                "{FF78:XXXX}": "텍스트 참조 (아이템명 등)",
                "{FFAC}": "효과음/이벤트",
                "{FFB8}": "전투 텍스트 시작",
            },
            "translation_notes": [
                "kr 필드에 한국어 번역을 입력하세요",
                "제어 코드({FF...})는 반드시 보존하세요",
                "{NL}은 줄바꿈 위치를 조정할 수 있습니다",
                "한 줄에 약 14자(16x16 타일 기준)",
                "프로젝트 용어집을 참조하세요",
                "프로젝트 번역 지침을 참조하세요",
            ],
        },
        "extended_dialogs": extended_dialogs,
        "extended_entry_map": entry_map,
    });

    let json =
        serde_json::to_string_pretty(&output).map_err(|e| format!("JSON serialize error: {e}"))?;

    let tr_dir = assets_dir.join("translation");
    fs::create_dir_all(&tr_dir).map_err(|e| format!("create translation dir: {e}"))?;
    let out_path = tr_dir.join("text_en.json");
    fs::write(&out_path, json.as_bytes())
        .map_err(|e| format!("failed to write text_en.json: {e}"))?;
    eprintln!(
        "  text_en.json: {} unique dialogs, {} entry mappings",
        extended_dialogs.len(),
        entry_map.len()
    );
    Ok(())
}

// ============================================================
// Public API
// ============================================================

/// Run the `init` command: generate charmap.json, en_reference.json, text_en.json.
pub fn run(rom_path: &Path, assets_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("============================================================");
    eprintln!("Madou Monogatari I - Init (generate derived assets)");
    eprintln!("============================================================");

    // Load EN ROM
    eprintln!("\nLoading EN ROM: {}", rom_path.display());
    let rom = fs::read(rom_path)?;
    eprintln!("  size: {} bytes (0x{:X})", rom.len(), rom.len());
    if rom.len() != 0x400000 {
        return Err(format!(
            "EN ROM size mismatch: expected 0x400000, got 0x{:X}",
            rom.len()
        )
        .into());
    }

    // 1. charmap.json
    eprintln!("\n[1/3] Generating charmap.json");
    generate_charmap_json(assets_dir)?;

    // 2. en_reference.json
    eprintln!("\n[2/3] Generating en_reference.json");
    generate_en_reference(&rom, assets_dir)?;

    // 3. text_en.json
    eprintln!("\n[3/3] Generating text_en.json");
    generate_text_en(&rom, assets_dir)?;

    eprintln!("\nDone!");
    Ok(())
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod tests;
