use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Korean tile index range starts at 0x0100
const KR_INDEX_START: u16 = 0x0100;

/// Special tile mappings (display name → tile index)
fn special_tiles() -> HashMap<&'static str, u16> {
    let mut m = HashMap::new();
    m.insert("hp:1", 0x005F);
    m.insert("hp:2", 0x0060);
    m.insert("hp:3", 0x0061);
    m.insert("hp:4", 0x0062);
    m.insert("hp:5", 0x0063);
    m.insert("spell:0", 0x0050);
    m.insert("spell:1", 0x0051);
    m.insert("spell:2", 0x0052);
    m.insert("spell:3", 0x0053);
    m.insert("header", 0x0076);
    m.insert("list", 0x0075);
    m.insert("item", 0x0077);
    m.insert("item:73", 0x0073);
    m.insert("q-open", 0x0048);
    m.insert("q", 0x0049);
    m.insert("pad", 0x0078);
    m
}

/// Named control code aliases
fn named_ctrl() -> HashMap<&'static str, u16> {
    let mut m = HashMap::new();
    m.insert("NL", 0xFF30);
    m.insert("PAGE", 0xFF34);
    m.insert("END", 0xFF04);
    m
}

/// Token types from parsing display format text
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ctrl(u16),
    CtrlParam(u16, u16),
    Tile(u16),
    /// Source-backed JP `0x0000` blank used to reserve full-width layout space.
    ///
    /// The parser emits this only for the explicit `{source-pad}` semantic
    /// token. JP-native normalization also promotes matching source/KR padding
    /// runs so ordinary Korean spaces can keep using their dedicated
    /// half-width glyph.
    LayoutPad,
    /// Build-generated end-of-row operation for a source-backed in-place
    /// overwrite.
    ///
    /// The parser never emits this token. JP-native normalization emits
    /// exactly one operation for every non-empty source row. At runtime it
    /// clears `clear_half_cells` 8-pixel cells, then restores the standard
    /// dialogue-box right border without advancing into it.
    SourceRowFinalize {
        clear_half_cells: usize,
    },
    KrChar(char),
    EnChar(char),
    Raw(u16),
}

/// Check if character is Korean (Hangul syllable or Jamo)
fn is_korean(ch: char) -> bool {
    let cp = ch as u32;
    (0xAC00..=0xD7AF).contains(&cp) || (0x3131..=0x3163).contains(&cp)
}

/// Parse display format text into tokens.
///
/// Handles: {FFXX:YYYY}, {FFXX}, {NL}, {PAGE}, {END}, {hp:N}, {spell:N},
/// {header}, {list}, {item}, {item:73}, {q}, {pad}, {icon:XX}, [XXXX], single chars
pub fn parse_display_text(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '{' {
            chars.next(); // consume '{'
            let content: String = chars.by_ref().take_while(|&c| c != '}').collect();

            let specials = special_tiles();
            let named = named_ctrl();

            if content == "source-pad" {
                tokens.push(Token::LayoutPad);
            } else if let Some(&tile) = specials.get(content.as_str()) {
                tokens.push(Token::Tile(tile));
            } else if let Some(&code) = named.get(content.as_str()) {
                tokens.push(Token::Ctrl(code));
            } else if let Some(hex) = content.strip_prefix("icon:") {
                if let Ok(idx) = u16::from_str_radix(hex, 16) {
                    tokens.push(Token::Tile(idx));
                }
            } else if content.len() == 9 && content.as_bytes()[4] == b':' {
                // {FFXX:YYYY}
                if let (Ok(code), Ok(param)) = (
                    u16::from_str_radix(&content[..4], 16),
                    u16::from_str_radix(&content[5..], 16),
                ) {
                    tokens.push(Token::CtrlParam(code, param));
                }
            } else if content.len() == 4 {
                // {FFXX}
                if let Ok(code) = u16::from_str_radix(&content, 16) {
                    tokens.push(Token::Ctrl(code));
                }
            }
        } else if ch == '[' {
            chars.next(); // consume '['
            let content: String = chars.by_ref().take_while(|&c| c != ']').collect();
            if let Ok(val) = u16::from_str_radix(&content, 16) {
                tokens.push(Token::Raw(val));
            }
        } else {
            chars.next();
            if is_korean(ch) {
                tokens.push(Token::KrChar(ch));
            } else {
                tokens.push(Token::EnChar(ch));
            }
        }
    }
    tokens
}

/// Load EN charmap from charmap.json.
/// Returns {display_char → tile_index}.
pub fn load_en_charmap(path: &Path) -> Result<HashMap<char, u16>, String> {
    let data = fs::read_to_string(path).map_err(|e| format!("failed to read charmap: {e}"))?;
    let raw: HashMap<String, String> =
        serde_json::from_str(&data).map_err(|e| format!("failed to parse charmap: {e}"))?;

    let mut charmap = HashMap::new();
    for (hex_key, display_char) in &raw {
        let tile_idx = u16::from_str_radix(hex_key.trim_start_matches("0x"), 16)
            .map_err(|e| format!("bad hex key {hex_key}: {e}"))?;

        // Skip special tiles
        if display_char.starts_with('{') {
            continue;
        }

        let ch = match display_char.chars().next() {
            Some(c) if display_char.chars().count() == 1 => c,
            _ => continue,
        };

        // Space: prefer 0x004A
        if ch == ' ' {
            if tile_idx == 0x004A {
                charmap.insert(' ', 0x004A);
            }
            continue;
        }

        // Dash: prefer 0x004B
        if ch == '-' {
            if tile_idx == 0x004B {
                charmap.insert('-', 0x004B);
            }
            continue;
        }

        charmap.entry(ch).or_insert(tile_idx);
    }

    charmap.entry(' ').or_insert(0x004A);
    Ok(charmap)
}

/// Collect unique Korean characters from translation JSON files.
/// Also includes characters not in EN charmap (like ~).
pub fn collect_unique_kr_chars(
    translation_files: &[impl AsRef<Path>],
    en_charmap: &HashMap<char, u16>,
) -> Result<Vec<char>, String> {
    let mut seen = BTreeSet::new();

    for path in translation_files {
        let data = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("failed to read {}: {e}", path.as_ref().display()))?;
        let obj: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| format!("failed to parse {}: {e}", path.as_ref().display()))?;

        let texts = extract_kr_texts_from_json(&obj);
        for text in &texts {
            let tokens = parse_display_text(text);
            for token in tokens {
                match token {
                    Token::KrChar(ch) => {
                        seen.insert(ch);
                    }
                    Token::EnChar(ch) if !en_charmap.contains_key(&ch) => {
                        seen.insert(ch);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(seen.into_iter().collect())
}

/// Extract KR text strings from a script_*.json value.
fn extract_kr_texts_from_json(obj: &serde_json::Value) -> Vec<&str> {
    let mut texts = Vec::new();

    if let Some(entries) = obj.get("entries").and_then(|v| v.as_array()) {
        for entry in entries {
            if let Some(ko) = entry.get("ko").and_then(|v| v.as_str())
                && !ko.is_empty()
            {
                texts.push(ko);
            }
        }
    }

    texts
}

/// Build Korean charmap: char → tile index (starting at 0x0100).
pub fn build_kr_charmap(unique_chars: &[char]) -> HashMap<char, u16> {
    unique_chars
        .iter()
        .enumerate()
        .map(|(i, &ch)| (ch, KR_INDEX_START + i as u16))
        .collect()
}

/// Encode display format text to 16-bit word sequence.
pub fn encode_text(
    text: &str,
    kr_charmap: &HashMap<char, u16>,
    en_charmap: &HashMap<char, u16>,
) -> Result<Vec<u16>, String> {
    let tokens = parse_display_text(text);
    let mut words = Vec::new();

    for token in tokens {
        match token {
            Token::Ctrl(code) => words.push(code),
            Token::CtrlParam(code, param) => {
                words.push(code);
                words.push(param);
            }
            Token::Tile(idx) => words.push(idx),
            Token::LayoutPad | Token::SourceRowFinalize { .. } => {
                return Err("internal JP-native layout padding reached the legacy encoder".into());
            }
            Token::Raw(val) => words.push(val),
            Token::KrChar(ch) => {
                if let Some(&idx) = kr_charmap.get(&ch) {
                    words.push(idx);
                } else {
                    return Err(format!(
                        "unknown Korean char: '{}' (U+{:04X})",
                        ch, ch as u32
                    ));
                }
            }
            Token::EnChar(ch) => {
                if let Some(&idx) = en_charmap.get(&ch) {
                    words.push(idx);
                } else if let Some(&idx) = kr_charmap.get(&ch) {
                    words.push(idx);
                } else {
                    return Err(format!("unknown char: '{}' (U+{:04X})", ch, ch as u32));
                }
            }
        }
    }

    Ok(words)
}

/// Convert 16-bit word sequence to big-endian bytes.
pub fn words_to_bytes(words: &[u16]) -> Vec<u8> {
    let mut data = Vec::with_capacity(words.len() * 2);
    for &w in words {
        data.push((w >> 8) as u8);
        data.push((w & 0xFF) as u8);
    }
    data
}

// ============================================================
// Charmap generation & ROM text decoder
// ============================================================

/// Full CTRL_WITH_PARAM set (matching build/mod.rs and dump_en_reference.py).
pub fn ctrl_with_param() -> HashSet<u16> {
    [
        0xFF0C, 0xFF44, 0xFF48, 0xFF4C, 0xFF50, 0xFF54, 0xFF58, 0xFF78, 0xFF84, 0xFF94, 0xFF9C,
        0xFFA0, 0xFFC4, 0xFFF8,
    ]
    .into_iter()
    .collect()
}

/// Block terminators for raw EN text reading.
pub fn terminators() -> HashSet<u16> {
    [0xFF38, 0xFF04, 0xFFFF].into_iter().collect()
}

/// Smaller CTRL_WITH_PARAM set (matching extract_translation.py for text_en.json).
pub fn ctrl_with_param_text_en() -> HashSet<u16> {
    [
        0xFF0C, 0xFF44, 0xFF48, 0xFF4C, 0xFF50, 0xFF78, 0xFF84, 0xFF9C, 0xFFF8,
    ]
    .into_iter()
    .collect()
}

/// Named control code display aliases (code → display name).
fn named_ctrl_display() -> HashMap<u16, &'static str> {
    HashMap::from([(0xFF30, "NL"), (0xFF34, "PAGE"), (0xFF04, "END")])
}

/// Generate the canonical tile→display charmap (matching assets/charmap.json).
/// Returns BTreeMap with "0xXXXX" keys for sorted JSON output.
pub fn generate_charmap() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let mut ins = |tile: u16, s: &str| {
        m.insert(format!("0x{tile:04X}"), s.to_string());
    };

    // Space at 0x0000
    ins(0x0000, " ");

    // Digits: 0x01-0x0A → 0-9
    for i in 0u16..10 {
        ins(0x01 + i, &i.to_string());
    }

    // Uppercase A-Z: 0x0B-0x24
    for i in 0u16..26 {
        ins(0x0B + i, &String::from((b'A' + i as u8) as char));
    }

    // Lowercase a-z: 0x25-0x3E
    for i in 0u16..26 {
        ins(0x25 + i, &String::from((b'a' + i as u8) as char));
    }

    // Punctuation (corrected order: 0x40=comma, 0x41=!, 0x42=?)
    for &(tile, ch) in &[
        (0x3Fu16, "."),
        (0x40, ","),
        (0x41, "!"),
        (0x42, "?"),
        (0x43, "'"),
        (0x44, ";"),
        (0x45, "("),
        (0x46, ")"),
        (0x47, "&"),
        (0x48, "{q-open}"),
        (0x49, "\""),
        (0x4A, " "),
        (0x4B, "-"),
        (0x4C, ":"),
        (0x4D, "*"),
    ] {
        ins(tile, ch);
    }

    // Spell icons
    for i in 0u16..4 {
        ins(0x50 + i, &format!("{{spell:{i}}}"));
    }

    // Symbol characters
    for &(tile, sym) in &[
        (0x54u16, "♥"),
        (0x55, "♦"),
        (0x56, "♠"),
        (0x58, "★"),
        (0x59, "◆"),
        (0x5A, "♪"),
        (0x5E, "◇"),
    ] {
        ins(tile, sym);
    }

    // Icon tiles
    for &tile in &[
        0x5Bu16, 0x64, 0x65, 0x68, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x7B, 0x7C, 0x81, 0x84, 0x85,
        0x94, 0xA7, 0xB9,
    ] {
        ins(tile, &format!("{{icon:{tile:02X}}}"));
    }

    // HP level icons: 0x5F-0x63 → hp:1 .. hp:5
    for i in 1u16..=5 {
        ins(0x5E + i, &format!("{{hp:{i}}}"));
    }

    // UI formatting tiles
    ins(0x71, "▸");
    ins(0x72, " ");
    ins(0x73, "{item:73}");
    ins(0x75, "{list}");
    ins(0x76, "{header}");
    ins(0x77, "{item}");
    ins(0x78, " ");
    ins(0x7F, " ");

    m
}

/// Build tile_index → display_string reverse lookup from charmap.
pub fn build_tile_to_display(charmap: &BTreeMap<String, String>) -> HashMap<u16, String> {
    let mut m = HashMap::new();
    for (hex_key, display) in charmap {
        if let Ok(tile) = u16::from_str_radix(hex_key.trim_start_matches("0x"), 16) {
            m.insert(tile, display.clone());
        }
    }
    m
}

/// Read raw u16 words from ROM at `offset`, handling ctrl code params.
///
/// Stops when encountering a word in `block_end`. The terminator word IS included.
/// Returns the flat word list.
pub fn read_rom_words(
    rom: &[u8],
    offset: usize,
    ctrl_with_param: &HashSet<u16>,
    block_end: &HashSet<u16>,
) -> Vec<u16> {
    let mut words = Vec::new();
    let mut pos = offset;
    let max_pos = (offset + 8192).min(rom.len());

    while pos + 1 < max_pos {
        let w = u16::from_be_bytes([rom[pos], rom[pos + 1]]);
        words.push(w);
        pos += 2;
        if block_end.contains(&w) {
            break;
        }
        if ctrl_with_param.contains(&w) && pos + 1 < max_pos {
            let param = u16::from_be_bytes([rom[pos], rom[pos + 1]]);
            words.push(param);
            pos += 2;
        }
    }
    words
}

/// Convert a u16 word sequence to display format string.
///
/// Uses `ctrl_with_param` to identify parameterized control codes.
/// Named codes: FF30→{NL}, FF34→{PAGE}, FF04→{END}.
/// Parameterized: {FFXX:YYYY}. Other ctrl: {FFXX}.
/// Tiles: charmap lookup or [XXXX].
pub fn words_to_display_text(
    words: &[u16],
    tile_to_str: &HashMap<u16, String>,
    ctrl_with_param: &HashSet<u16>,
) -> String {
    let named = named_ctrl_display();
    let mut parts = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let w = words[i];
        if w >= 0xFF00 {
            if let Some(&name) = named.get(&w) {
                parts.push(format!("{{{name}}}"));
            } else if ctrl_with_param.contains(&w) && i + 1 < words.len() {
                parts.push(format!("{{{w:04X}:{:04X}}}", words[i + 1]));
                i += 1;
            } else {
                parts.push(format!("{{{w:04X}}}"));
            }
        } else if let Some(s) = tile_to_str.get(&w) {
            parts.push(s.clone());
        } else {
            parts.push(format!("[{w:04X}]"));
        }
        i += 1;
    }
    parts.join("")
}

/// Convert a u16 word sequence to human-readable text (stripping control codes).
pub fn words_to_readable_text(
    words: &[u16],
    tile_to_str: &HashMap<u16, String>,
    ctrl_with_param: &HashSet<u16>,
) -> String {
    let mut parts = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let w = words[i];
        if w >= 0xFF00 {
            if w == 0xFF30 {
                parts.push("\n".to_string());
            } else if w == 0xFF34 {
                parts.push("\n---\n".to_string());
            } else if w == 0xFF04 || w == 0xFF38 || w == 0xFFFF {
                break;
            }
            if ctrl_with_param.contains(&w) && i + 1 < words.len() {
                i += 1; // skip param
            }
        } else if let Some(s) = tile_to_str.get(&w) {
            parts.push(s.clone());
        }
        i += 1;
    }
    parts.join("").trim().to_string()
}

/// Helper: build char→tile encode map from generated charmap (for testing/init).
pub fn load_en_charmap_from_generated(charmap: &BTreeMap<String, String>) -> HashMap<char, u16> {
    let mut result = HashMap::new();
    for (hex_key, display) in charmap {
        let tile_idx = match u16::from_str_radix(hex_key.trim_start_matches("0x"), 16) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if display.starts_with('{') {
            continue;
        }
        let ch = match display.chars().next() {
            Some(c) if display.chars().count() == 1 => c,
            _ => continue,
        };
        if ch == ' ' {
            if tile_idx == 0x004A {
                result.insert(' ', 0x004A);
            }
            continue;
        }
        if ch == '-' {
            if tile_idx == 0x004B {
                result.insert('-', 0x004B);
            }
            continue;
        }
        result.entry(ch).or_insert(tile_idx);
    }
    result.entry(' ').or_insert(0x004A);
    result
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
