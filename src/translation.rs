//! Translation file loading for script_*.json format.
//!
//! Format (from `align` command):
//! ```json
//! { "entries": [{ "id": "script_0000", "old_dialog_id": "dialog_0000",
//!                 "fff8_idx": 0, "en": "...", "ko": "...", ... }] }
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn validate_kr_punctuation_style(file: &str, id: &str, ko: &str) -> Result<(), String> {
    if ko.chars().any(|ch| {
        matches!(
            ch,
            '\u{3040}'..='\u{309F}'
                | '\u{30A0}'..='\u{30FF}'
                | '\u{31F0}'..='\u{31FF}'
                | '\u{FF65}'..='\u{FF9F}'
        )
    }) {
        return Err(format!(
            "{file}:{id}: KR text contains Japanese kana; restore source operands through typed controls instead of visible KR prose"
        ));
    }
    if ko.contains("~.") {
        return Err(format!(
            "{file}:{id}: KR text joins '~' and '.'; use the drawn-out ending without a period"
        ));
    }
    if ko.contains("...") {
        return Err(format!(
            "{file}:{id}: KR text uses three ASCII periods; use the dedicated ellipsis glyph '…'"
        ));
    }
    let without_source_star_operand = ko.replace("{FFAC}*", "");
    if without_source_star_operand.contains('*') {
        return Err(format!(
            "{file}:{id}: KR text uses EN-style asterisk decoration; preserve only source-backed {{FFAC}}* operands"
        ));
    }
    Ok(())
}

/// Load translations as `dialog_XXXX → KR text` mapping (for build pipeline).
///
/// Maps `old_dialog_id → ko` for FFF8 entries.
/// For entries with `id` starting with `plural_` or `momomo_`, maps `id → ko`.
pub fn load_translations(translation_dir: &Path) -> Result<HashMap<String, String>, String> {
    let mut translations = HashMap::new();
    let filenames = list_script_files(translation_dir)?;

    for fname in &filenames {
        let path = translation_dir.join(fname);
        let data = fs::read_to_string(&path).map_err(|e| format!("failed to read {fname}: {e}"))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| format!("failed to parse {fname}: {e}"))?;

        let entries = match parsed.get("entries").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => continue,
        };

        let mut count = 0;
        for entry in entries {
            let ko = entry.get("ko").and_then(|v| v.as_str()).unwrap_or("");
            if ko.is_empty() {
                continue;
            }

            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("");
            validate_kr_punctuation_style(fname, id, ko)?;
            let old_dialog_id = entry
                .get("old_dialog_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // FFF8 entries: use old_dialog_id as key
            if !old_dialog_id.is_empty() && old_dialog_id.starts_with("dialog_") {
                translations.insert(old_dialog_id.to_string(), ko.to_string());
                count += 1;
            }
            // Hardcoded entries (plural, momomo, etc.): use id directly
            else if id.starts_with("plural_") || id.starts_with("momomo_") {
                translations.insert(id.to_string(), ko.to_string());
                count += 1;
            }
        }
        eprintln!("  {fname}: {count} entries");
    }

    eprintln!(
        "  total translations (script format): {}",
        translations.len()
    );
    Ok(translations)
}

/// Collect all KR display text strings from translation files.
/// Used by `collect_unique_kr_chars` for font generation.
pub fn collect_all_kr_texts(translation_dir: &Path) -> Result<Vec<String>, String> {
    let mut texts = Vec::new();
    let filenames = list_script_files(translation_dir)?;

    for fname in &filenames {
        let path = translation_dir.join(fname);
        let data = fs::read_to_string(&path).map_err(|e| format!("failed to read {fname}: {e}"))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| format!("failed to parse {fname}: {e}"))?;

        if let Some(entries) = parsed.get("entries").and_then(|v| v.as_array()) {
            for entry in entries {
                if let Some(ko) = entry.get("ko").and_then(|v| v.as_str())
                    && !ko.is_empty()
                {
                    let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    validate_kr_punctuation_style(fname, id, ko)?;
                    texts.push(ko.to_string());
                }
            }
        }
    }

    Ok(texts)
}

/// Source-KR entry for QA tools (check-ctrl, check-overflow).
pub struct PairedEntry {
    pub file: String,
    pub key: String,
    pub jp: String,
    pub en: String,
    pub ko: String,
}

/// Load paired EN-KR entries for QA checks.
pub fn load_paired_entries(assets_dir: &Path) -> Result<Vec<PairedEntry>, String> {
    let translation_dir = assets_dir.join("translation");
    let mut entries = Vec::new();
    let filenames = list_script_files(&translation_dir)?;

    for fname in &filenames {
        let path = translation_dir.join(fname);
        let data = fs::read_to_string(&path).map_err(|e| format!("failed to read {fname}: {e}"))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| format!("failed to parse {fname}: {e}"))?;

        if let Some(arr) = parsed.get("entries").and_then(|v| v.as_array()) {
            for entry in arr {
                let ko = entry.get("ko").and_then(|v| v.as_str()).unwrap_or("");
                if ko.is_empty() {
                    continue;
                }
                let en = entry.get("en").and_then(|v| v.as_str()).unwrap_or("");
                let jp = entry.get("jp").and_then(|v| v.as_str()).unwrap_or("");
                let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("");
                validate_kr_punctuation_style(fname, id, ko)?;

                // Use old_dialog_id as key for backward compat with QA reports
                let key = entry
                    .get("old_dialog_id")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty() && s.starts_with("dialog_"))
                    .unwrap_or(id);

                entries.push(PairedEntry {
                    file: fname.clone(),
                    key: key.to_string(),
                    jp: jp.to_string(),
                    en: en.to_string(),
                    ko: ko.to_string(),
                });
            }
        }
    }

    Ok(entries)
}

/// List translation file paths for font character collection.
pub fn list_translation_paths(translation_dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let filenames = list_script_files(translation_dir)?;
    Ok(filenames.iter().map(|f| translation_dir.join(f)).collect())
}

/// List sorted script_*.json filenames in a directory.
fn list_script_files(dir: &Path) -> Result<Vec<String>, String> {
    let mut filenames: Vec<String> = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("failed to read {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with("script_") && fname.ends_with(".json") {
            filenames.push(fname);
        }
    }
    filenames.sort();
    Ok(filenames)
}
