//! Port of check_overflow.py — measure pixel width of each text line and report overflows.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use fontdue::{Font, FontSettings};

use crate::build::text::{self, Token};
use crate::translation;

// ──────────────────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────────────────

/// EN advance/width table offset in EN ROM.
const EN_WIDTH_TABLE: usize = 0x320D46;
/// Number of EN character entries.
const MAX_EN_CHARS: usize = 128;
/// Maximum borderline entries to display.
const BORDERLINE_TOP_N: usize = 30;
/// Font rendering grid size (must match gen_hangul_font.py).
const GRID_SIZE: usize = 16;
/// Default box limit for unknown box types (conservative).
const DEFAULT_BOX_LIMIT: u32 = 133;

/// Per-box-type pixel width limits.
fn box_limit(box_type: Option<u16>) -> u32 {
    match box_type {
        Some(0xFF10) => 133, // player text box
        Some(0xFF14) => 133, // NPC text box
        Some(0xFF18) => 137, // left dual box
        Some(0xFF1C) => 133, // right dual box
        Some(0xFF2C) => 224, // cutscene scroll
        Some(0xFFB8) => 133, // auto text
        _ => DEFAULT_BOX_LIMIT,
    }
}

// ──────────────────────────────────────────────────────────
// EN advance table
// ──────────────────────────────────────────────────────────

/// Load EN font advance widths from ROM data.
/// Returns tile_index → advance_width mapping for indices 0..127.
fn load_en_advance_table(rom: &[u8]) -> Result<HashMap<u16, u32>, String> {
    if rom.len() < EN_WIDTH_TABLE + MAX_EN_CHARS * 2 {
        return Err("ROM too small for EN width table".into());
    }
    let mut advance = HashMap::new();
    for i in 0..MAX_EN_CHARS {
        let offset = EN_WIDTH_TABLE + i * 2;
        let adv = rom[offset + 1] as u32; // byte1 = advance width
        advance.insert(i as u16, adv);
    }
    Ok(advance)
}

// ──────────────────────────────────────────────────────────
// KR character width measurement via fontdue
// ──────────────────────────────────────────────────────────

/// Measure advance width of a single character rendered at GRID_SIZE px.
/// advance = rightmost_pixel + 1 + 1 (matching build_kr_rom.py: width + 1).
fn measure_kr_advance(font: &Font, ch: char) -> u32 {
    let (metrics, coverage) = font.rasterize(ch, GRID_SIZE as f32);

    // Center glyph (same as font.rs render_glyph)
    let x_off = ((GRID_SIZE as i32 - metrics.width as i32) / 2).max(0) as usize;
    let y_off = ((GRID_SIZE as i32 - metrics.height as i32) / 2).max(0) as usize;

    let mut max_x: u32 = 0;
    for row in 0..metrics.height {
        for col in (0..metrics.width).rev() {
            let dst_row = y_off + row;
            let dst_col = x_off + col;
            if dst_row < GRID_SIZE && dst_col < GRID_SIZE {
                let alpha = coverage[row * metrics.width + col];
                if alpha > 127 {
                    max_x = max_x.max(dst_col as u32 + 1);
                    break;
                }
            }
        }
    }

    // advance = pixel_width + 1 (same as font.rs)
    max_x + 1
}

/// Measure advances for all unique KR characters found in translation files.
/// Returns (tile_index → advance) map and (char → tile_index) charmap.
#[allow(clippy::type_complexity)]
fn measure_kr_advances(
    translation_files: &[impl AsRef<Path>],
    en_charmap: &HashMap<char, u16>,
    font_path: &Path,
) -> Result<(HashMap<u16, u32>, HashMap<char, u16>), String> {
    let unique_chars = text::collect_unique_kr_chars(translation_files, en_charmap)?;
    let kr_charmap = text::build_kr_charmap(&unique_chars);

    let ttf_data = fs::read(font_path)
        .map_err(|e| format!("failed to read font {}: {e}", font_path.display()))?;
    let font = Font::from_bytes(ttf_data, FontSettings::default())
        .map_err(|e| format!("failed to parse font: {e}"))?;

    let mut kr_advances = HashMap::new();
    for (&ch, &tile_idx) in &kr_charmap {
        let adv = measure_kr_advance(&font, ch);
        kr_advances.insert(tile_idx, adv);
    }

    Ok((kr_advances, kr_charmap))
}

// ──────────────────────────────────────────────────────────
// Tile advance map (EN + KR combined)
// ──────────────────────────────────────────────────────────

fn build_tile_advance_map(
    en_advance: &HashMap<u16, u32>,
    kr_advances: &HashMap<u16, u32>,
) -> HashMap<u16, u32> {
    let mut m = en_advance.clone();
    for (&tile_idx, &adv) in kr_advances {
        m.insert(tile_idx, adv);
    }
    m
}

// ──────────────────────────────────────────────────────────
// Line width calculation
// ──────────────────────────────────────────────────────────

/// Line measurement result.
struct LineMeasure {
    line_num: u32,
    width_px: u32,
    text: String,
    box_type: Option<u16>,
}

/// Control codes that define a box type (affects width limit).
fn is_box_type_code(code: u16) -> bool {
    matches!(code, 0xFF10 | 0xFF14 | 0xFF18 | 0xFF1C | 0xFF2C | 0xFFB8)
}

/// Control codes that reset the line (new dialog box / scene transition).
fn is_line_reset_ctrl(code: u16) -> bool {
    matches!(
        code,
        0xFF04 | 0xFF38 | 0xFFFF |         // END, block end, data end
        0xFF10 | 0xFF14 | 0xFF18 | 0xFF1C | // text/dialog start
        0xFF2C |                             // scroll text
        0xFF68 | 0xFF60 | 0xFF6C | 0xFF64 | // scene control
        0xFFC0 | 0xFFCC |                   // dialog continuation
        0xFFB0 | 0xFFB4 | 0xFFB8 |         // display control (FFB4 = screen transition)
        0xFFAC | 0xFFFC // misc control
    )
}

/// Calculate per-line pixel widths for a text entry.
fn calc_line_widths(
    display_text: &str,
    en_charmap: &HashMap<char, u16>,
    tile_advance: &HashMap<u16, u32>,
    kr_charmap: &HashMap<char, u16>,
) -> Vec<LineMeasure> {
    let tokens = text::parse_display_text(display_text);
    let mut lines = Vec::new();
    let mut width: u32 = 0;
    let mut line_text = String::new();
    let mut line_num: u32 = 1;
    let mut current_box: Option<u16> = None;

    for token in &tokens {
        match token {
            Token::Ctrl(code) => {
                // Track box type
                if is_box_type_code(*code) {
                    current_box = Some(*code);
                }
                if *code == 0xFF30 || *code == 0xFF34 {
                    // NL / PAGE
                    lines.push(LineMeasure {
                        line_num,
                        width_px: width,
                        text: line_text.clone(),
                        box_type: current_box,
                    });
                    width = 0;
                    line_text.clear();
                    line_num += 1;
                } else if is_line_reset_ctrl(*code) {
                    if !line_text.is_empty() {
                        lines.push(LineMeasure {
                            line_num,
                            width_px: width,
                            text: line_text.clone(),
                            box_type: current_box,
                        });
                    }
                    width = 0;
                    line_text.clear();
                    line_num += 1;
                }
            }
            Token::CtrlParam(code, param) => {
                if *code == 0xFF48 || *code == 0xFF44 || *code == 0xFF4C {
                    // Number display + trailing tile
                    let digit_adv = *tile_advance.get(&0x01).unwrap_or(&8);
                    width += digit_adv * 2; // avg 2-digit number
                    let trailing_adv = *tile_advance.get(param).unwrap_or(&8);
                    width += trailing_adv;
                    line_text.push_str("[NUM+tile]");
                } else if *code == 0xFF9C {
                    // Menu end → new dialog context
                    if !line_text.is_empty() {
                        lines.push(LineMeasure {
                            line_num,
                            width_px: width,
                            text: line_text.clone(),
                            box_type: current_box,
                        });
                    }
                    width = 0;
                    line_text.clear();
                    line_num += 1;
                }
                // FF0C, FF50, FF54, FF58, FF78, FF84, FF94, FFA0 etc. → no visible width
            }
            Token::Tile(tile_idx) => {
                let adv = *tile_advance.get(tile_idx).unwrap_or(&16);
                width += adv;
                line_text.push_str(&format!("[T{tile_idx:02X}]"));
            }
            Token::LayoutPad => {
                width += 16;
                line_text.push_str("[PAD]");
            }
            Token::SourceRowFinalize { clear_half_cells } => {
                line_text.push_str(&format!("[FINALIZE:{clear_half_cells}]"));
            }
            Token::Raw(val) => {
                let adv = *tile_advance.get(val).unwrap_or(&8);
                width += adv;
                line_text.push_str(&format!("[{val:04X}]"));
            }
            Token::KrChar(ch) => {
                let tile_idx = kr_charmap.get(ch);
                if let Some(&tidx) = tile_idx {
                    let adv = *tile_advance.get(&tidx).unwrap_or(&17);
                    width += adv;
                } else {
                    width += 17; // default Korean advance
                }
                line_text.push(*ch);
            }
            Token::EnChar(ch) => {
                if let Some(&tidx) = en_charmap.get(ch) {
                    let adv = *tile_advance.get(&tidx).unwrap_or(&8);
                    width += adv;
                } else if let Some(&tidx) = kr_charmap.get(ch) {
                    let adv = *tile_advance.get(&tidx).unwrap_or(&17);
                    width += adv;
                } else {
                    width += 8; // default
                }
                line_text.push(*ch);
            }
        }
    }

    // Last line
    if !line_text.is_empty() {
        lines.push(LineMeasure {
            line_num,
            width_px: width,
            text: line_text,
            box_type: current_box,
        });
    }

    lines
}

// ──────────────────────────────────────────────────────────
// Load translation files (delegates to shared translation module)
// ──────────────────────────────────────────────────────────

/// Load translation entries for overflow checking.
/// Returns Vec<(filename, key, text)> for dialog entries.
fn load_translation_entries(assets_dir: &Path) -> Result<Vec<(String, String, String)>, String> {
    let paired = translation::load_paired_entries(assets_dir)?;
    Ok(paired.into_iter().map(|e| (e.file, e.key, e.ko)).collect())
}

// ──────────────────────────────────────────────────────────
// Overflow report entry
// ──────────────────────────────────────────────────────────

struct OverflowEntry {
    file: String,
    key: String,
    line: u32,
    width: u32,
    limit: u32,
    excess: u32,
    box_type: Option<u16>,
    text: String,
}

struct BorderlineEntry {
    file: String,
    key: String,
    line: u32,
    width: u32,
    limit: u32,
    box_type: Option<u16>,
    text: String,
}

// ──────────────────────────────────────────────────────────
// Main entry point
// ──────────────────────────────────────────────────────────

pub fn run(rom_path: &Path, assets_dir: &Path) -> Result<(), String> {
    println!("{}", "=".repeat(60));
    println!("텍스트 오버플로우 탐지");
    println!("{}", "=".repeat(60));

    // 1. Load EN ROM and advance table
    let rom = fs::read(rom_path).map_err(|e| format!("failed to read ROM: {e}"))?;
    let en_advance = load_en_advance_table(&rom)?;

    println!("\nEN advance 테이블 로드 (0-127)");
    println!(
        "  space(0x4A) advance: {}px",
        en_advance.get(&0x4A).unwrap_or(&0)
    );
    println!(
        "  A(0x0B) advance: {}px",
        en_advance.get(&0x0B).unwrap_or(&0)
    );
    println!(
        "  W(0x21) advance: {}px",
        en_advance.get(&0x21).unwrap_or(&0)
    );
    println!(
        "  a(0x25) advance: {}px",
        en_advance.get(&0x25).unwrap_or(&0)
    );
    println!(
        "  m(0x31) advance: {}px",
        en_advance.get(&0x31).unwrap_or(&0)
    );

    // 2. Load EN charmap
    let charmap_path = assets_dir.join("charmap.json");
    let en_charmap = text::load_en_charmap(&charmap_path)?;

    // 3. Load KR translation files and measure KR advances
    let font_path = assets_dir.join("neodgm.ttf");
    let tr_dir = assets_dir.join("translation");
    let kr_json_paths = translation::list_translation_paths(&tr_dir)?;
    let (kr_advances, kr_charmap) = measure_kr_advances(&kr_json_paths, &en_charmap, &font_path)?;

    println!("KR 문자 수: {}", kr_charmap.len());
    let mut kr_sample: Vec<_> = kr_charmap.iter().collect();
    kr_sample.sort_by_key(|(_, idx)| **idx);
    for (ch, tidx) in kr_sample.iter().take(3) {
        println!(
            "  '{}' (0x{:04X}) advance: {}px",
            ch,
            tidx,
            kr_advances.get(tidx).unwrap_or(&0)
        );
    }

    // 4. Build combined tile advance map
    let tile_advance = build_tile_advance_map(&en_advance, &kr_advances);

    // 5. Load all translation entries
    let entries = load_translation_entries(assets_dir)?;

    // 6. Scan for overflows (per-box-type limits)
    println!("\n{}", "=".repeat(60));
    println!("오버플로우 스캔 (per-box-type limits)");
    println!("{}", "=".repeat(60));

    let mut overflows = Vec::new();
    let mut borderline = Vec::new();

    for (fname, key, display_text) in &entries {
        let lines = calc_line_widths(display_text, &en_charmap, &tile_advance, &kr_charmap);
        for lm in &lines {
            let limit = box_limit(lm.box_type);
            let borderline_threshold = limit * 85 / 100;
            if lm.width_px > limit {
                overflows.push(OverflowEntry {
                    file: fname.clone(),
                    key: key.clone(),
                    line: lm.line_num,
                    width: lm.width_px,
                    limit,
                    excess: lm.width_px - limit,
                    box_type: lm.box_type,
                    text: lm.text.clone(),
                });
            } else if lm.width_px > borderline_threshold {
                borderline.push(BorderlineEntry {
                    file: fname.clone(),
                    key: key.clone(),
                    line: lm.line_num,
                    width: lm.width_px,
                    limit,
                    box_type: lm.box_type,
                    text: lm.text.clone(),
                });
            }
        }
    }

    // Sort overflows by excess (worst first)
    overflows.sort_by_key(|entry| Reverse(entry.excess));

    if overflows.is_empty() {
        println!("\n  오버플로우 없음!");
    } else {
        println!("\n  오버플로우 {}건 발견:\n", overflows.len());
        for e in &overflows {
            let box_str = match e.box_type {
                Some(b) => format!("box:{:04X}", b),
                None => "box:????".to_string(),
            };
            println!(
                "  [{}] {} L{}: {}px/{}px (+{}px) [{}]",
                e.file, e.key, e.line, e.width, e.limit, e.excess, box_str
            );
            let preview: String = e.text.chars().take(100).collect();
            println!("    {}", preview);
            println!();
        }
    }

    println!("\n총 {}건 오버플로우", overflows.len());

    // 7. Borderline report
    println!("\n{}", "=".repeat(60));
    println!("경계선 줄 (>85% of box limit)");
    println!("{}", "=".repeat(60));

    borderline.sort_by_key(|entry| Reverse(entry.width));
    println!("\n  경계선 줄 {}건:\n", borderline.len());
    for e in borderline.iter().take(BORDERLINE_TOP_N) {
        let box_str = match e.box_type {
            Some(b) => format!("box:{:04X}", b),
            None => "box:????".to_string(),
        };
        println!(
            "  [{}] {} L{}: {}px/{}px [{}]",
            e.file, e.key, e.line, e.width, e.limit, box_str
        );
        let preview: String = e.text.chars().take(100).collect();
        println!("    {}", preview);
        println!();
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "overflow_tests.rs"]
mod tests;
