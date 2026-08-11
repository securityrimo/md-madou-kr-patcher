//! `align` 서브커맨드 — JP ROM 기반 완전 정렬 생성.
//!
//! JP ROM에서 모든 텍스트를 직접 추출하고, EN/KR과 ROM 오프셋 기반으로 매칭하여
//! 32개 단위 JSON 파일로 출력.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::build::text as en_text_mod;

// ============================================================
// Pointer table definitions
// ============================================================

#[derive(Clone, Copy)]
enum PtrFormat {
    /// 4-byte absolute address (u32be) — master tables
    Abs32,
    /// 2-byte relative offset from table base (u16be) — sub-tables
    Rel16,
}

struct PtrTableDef {
    name: &'static str,
    address: usize,
    count: usize,
    format: PtrFormat,
}

const PTR_TABLES: &[PtrTableDef] = &[
    // Sub-tables first (2-byte relative offsets) — so they get correct section labels
    // before master tables discover the same offsets via indirection.
    PtrTableDef {
        name: "system",
        address: 0x09E210,
        count: 40,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "spell_cmd",
        address: 0x09E4B2,
        count: 10,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "monster",
        address: 0x09ECDC,
        count: 32,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "health",
        address: 0x09EE48,
        count: 16,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "mp_restore",
        address: 0x09F148,
        count: 8,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "mp_remaining",
        address: 0x09F26E,
        count: 8,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "hp_restore",
        address: 0x09F424,
        count: 8,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "hpmp_restore",
        address: 0x09F570,
        count: 8,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "dmg_voice",
        address: 0x09F6EC,
        count: 23,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "dmg_novoice",
        address: 0x09F972,
        count: 23,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "spell_msg",
        address: 0x09FC80,
        count: 75,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "item_name",
        address: 0x0A08C6,
        count: 52,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "item_quoted",
        address: 0x0A0AEC,
        count: 52,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "item_desc",
        address: 0x0A0E12,
        count: 39,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "item_use",
        address: 0x0A0E7A,
        count: 51,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "item_use2",
        address: 0x0A0EE0,
        count: 45,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "enemy_hp",
        address: 0x0A26B6,
        count: 16,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "enemy_dmg",
        address: 0x0A299E,
        count: 8,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "enemy_26",
        address: 0x0A4F60,
        count: 8,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "enemy_27",
        address: 0x0A391A,
        count: 8,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "shop",
        address: 0x0B063C,
        count: 60,
        format: PtrFormat::Rel16,
    },
    PtrTableDef {
        name: "intro_ending",
        address: 0x0B1090,
        count: 91,
        format: PtrFormat::Rel16,
    },
    // Master tables (4-byte absolute pointers) — picks up remaining entries
    PtrTableDef {
        name: "dungeon",
        address: 0x09E0F0,
        count: 28,
        format: PtrFormat::Abs32,
    },
    PtrTableDef {
        name: "event",
        address: 0x09E170,
        count: 40,
        format: PtrFormat::Abs32,
    },
    PtrTableDef {
        name: "enemy_battle",
        address: 0x09E194,
        count: 24,
        format: PtrFormat::Abs32,
    },
    PtrTableDef {
        name: "chapter",
        address: 0x0A505C,
        count: 18,
        format: PtrFormat::Abs32,
    },
];

/// FFF8 pointer table in extended EN ROM area.
const EN_FFF8_PTR_TABLE: usize = 0x210000;

// ============================================================
// Helpers
// ============================================================

fn read_u16_be(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

// ============================================================
// JP Charmap (from madou1md.tbl)
// ============================================================

pub(crate) fn build_jp_charmap() -> HashMap<u16, String> {
    let mut m = HashMap::new();
    let mut ins = |tile: u16, s: &str| {
        m.insert(tile, s.to_string());
    };

    ins(0x0000, "　");

    for (i, ch) in "０１２３４５６７８９".chars().enumerate() {
        ins(0x01 + i as u16, &ch.to_string());
    }
    for (i, ch) in "゛゜、。．".chars().enumerate() {
        ins(0x0B + i as u16, &ch.to_string());
    }

    let hiragana = "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをん";
    for (i, ch) in hiragana.chars().enumerate() {
        ins(0x10 + i as u16, &ch.to_string());
    }
    for (i, ch) in "ぁぃぅぇぉゃゅょっ".chars().enumerate() {
        ins(0x3E + i as u16, &ch.to_string());
    }

    let katakana = "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン";
    for (i, ch) in katakana.chars().enumerate() {
        ins(0x47 + i as u16, &ch.to_string());
    }
    for (i, ch) in "ァィゥェォャュョッ".chars().enumerate() {
        ins(0x75 + i as u16, &ch.to_string());
    }

    ins(0x7E, "『");
    ins(0x7F, "』");
    ins(0x80, "！");
    ins(0x81, "？");
    ins(0x82, "・");
    ins(0x83, "ー");
    ins(0x84, "…");
    ins(0x85, "〜");

    for (i, ch) in "ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ"
        .chars()
        .enumerate()
    {
        ins(0x86 + i as u16, &ch.to_string());
    }
    for (i, ch) in "がぎぐげござじずぜぞだぢづでど".chars().enumerate() {
        ins(0xA0 + i as u16, &ch.to_string());
    }
    for (i, ch) in "ばびぶべぼ".chars().enumerate() {
        ins(0xAF + i as u16, &ch.to_string());
    }
    for (i, ch) in "ぱぴぷぺぽ".chars().enumerate() {
        ins(0xB4 + i as u16, &ch.to_string());
    }
    for (i, ch) in "ガギグゲゴザジズゼゾダヂヅデド".chars().enumerate() {
        ins(0xB9 + i as u16, &ch.to_string());
    }
    for (i, ch) in "バビブベボ".chars().enumerate() {
        ins(0xC8 + i as u16, &ch.to_string());
    }
    for (i, ch) in "パピプペポ".chars().enumerate() {
        ins(0xCD + i as u16, &ch.to_string());
    }

    ins(0xD2, "ヵ");
    ins(0xD3, "ヶ");
    ins(0xD4, "＆");
    ins(0xD5, "＋");
    ins(0xD6, "「");
    ins(0xD7, "」");
    ins(0xD8, "\u{201C}");
    ins(0xD9, "敵");
    ins(0xDA, "前");
    ins(0xDB, "\u{201D}");
    ins(0xDC, "ヴ");

    for &(tile, tag) in &[
        (0xDDu16, "[heart]"),
        (0xDE, "[sweat1]"),
        (0xDF, "[sweat2]"),
        (0xE0, "[sweat3]"),
        (0xE1, "！！"),
        (0xE2, "[bandage]"),
        (0xE3, "[2hearts]"),
        (0xE4, "[surprise]"),
        (0xE5, "[blush]"),
        (0xE6, "[smallstar]"),
        (0xE7, "[carbuncle]"),
        (0xE8, "フ"),
        (0xE9, "ブ"),
        (0xEA, "[anger]"),
        (0xEB, "[:D]"),
        (0xEC, "[:)]"),
        (0xED, "[:|]"),
        (0xEE, "[:(]"),
        (0xEF, "[>:(]"),
    ] {
        ins(tile, tag);
    }

    for &(tile, tag) in &[
        (0xF0u16, "立"),
        (0xF1, "[musicnote]"),
        (0xF2, "※"),
        (0xF3, "☆"),
        (0xF4, "★"),
        (0xF5, "（"),
        (0xF6, "）"),
        (0xF7, "[sigh]"),
        (0xF8, "[anxiety]"),
        (0xF9, "[sta]"),
        (0xFA, "[rt]"),
        (0xFB, "←"),
        (0xFC, "→"),
        (0xFD, "[a_button]"),
        (0xFE, "[b_button]"),
        (0xFF, "[c_button]"),
    ] {
        ins(tile, tag);
    }

    let kanji = "袋酒激辛仙人魔導竜牙尻尾角草毒消笛卵壺象失明玉黄金必殺酸素防火壁方向石魚福入体力回復食水声光中元気少苦肩息出死最大半分残高幸以上宝箱開手持他捨切拾日記塔階迫表情巨買多売客初来次混感触下段深言思用意左右流文字書地底湖危険決使押南北東西音聞願紙兄渡商品園長先顔名管理子試験説室点数合格問題正解噴忘者報告見悪始穴生利役恋自会筆休卒何歩私妖精話印店終了法陣年愛赤糸受転減好本当動部屋床色強師注返犯真目止％雷落宿↑↓男女行泣再伝足驚逃同闇貴公友怒滝呼込秒壺炎呪起不笑今井助弟白一幻剣脳天爆撃斬閉扉心配緑銀勝負道通氷知早貸借封洪";
    for (i, ch) in kanji.chars().enumerate() {
        ins(0x0100 + i as u16, &ch.to_string());
    }

    m
}

// ============================================================
// EN Charmap (from build/text.rs generate_charmap)
// ============================================================

fn build_en_charmap() -> HashMap<u16, String> {
    let charmap = en_text_mod::generate_charmap();
    en_text_mod::build_tile_to_display(&charmap)
}

// ============================================================
// Control codes (shared JP/EN)
// ============================================================

fn ctrl_with_param() -> HashSet<u16> {
    [
        0xFF0C, 0xFF44, 0xFF48, 0xFF4C, 0xFF50, 0xFF54, 0xFF58, 0xFF78, 0xFF84, 0xFF94, 0xFF9C,
        0xFFA0, 0xFFC4, 0xFFF8,
    ]
    .into_iter()
    .collect()
}

fn ctrl_with_long_param() -> HashSet<u16> {
    [0xFFD0].into_iter().collect()
}

fn named_ctrl() -> HashMap<u16, &'static str> {
    HashMap::from([
        (0xFF04, "END"),
        (0xFF10, "FF10"),
        (0xFF14, "FF14"),
        (0xFF18, "FF18"),
        (0xFF1C, "FF1C"),
        (0xFF2C, "FF2C"),
        (0xFF30, "NL"),
        (0xFF34, "PAGE"),
        (0xFF38, "FF38"),
        (0xFF3C, "FF3C"),
        (0xFF60, "FF60"),
        (0xFF64, "FF64"),
        (0xFF68, "FF68"),
        (0xFF6C, "FF6C"),
        (0xFF70, "FF70"),
        (0xFF74, "FF74"),
        (0xFF80, "FF80"),
        (0xFFAC, "FFAC"),
        (0xFFB0, "FFB0"),
        (0xFFB4, "FFB4"),
        (0xFFB8, "FFB8"),
        (0xFFC0, "FFC0"),
        (0xFFCC, "FFCC"),
        (0xFFF4, "FFF4"),
        (0xFFFF, "FFFF"),
    ])
}

/// Terminators that end a text entry.
fn is_terminator(word: u16) -> bool {
    matches!(word, 0xFF04 | 0xFF38 | 0xFFC0 | 0xFFFF)
}

// ============================================================
// Generic text decoder
// ============================================================

/// Decode text from ROM using given charmap.
/// Returns (display_text, end_position).
fn decode_text(
    rom: &[u8],
    start: usize,
    charmap: &HashMap<u16, String>,
    cp: &HashSet<u16>,
    clp: &HashSet<u16>,
    named: &HashMap<u16, &str>,
    max_bytes: usize,
) -> (String, usize) {
    let mut parts = Vec::new();
    let mut pos = start;
    let end = (start + max_bytes).min(rom.len());

    while pos + 1 < end {
        let word = read_u16_be(rom, pos);
        pos += 2;

        if word >= 0xFF00 {
            if let Some(&name) = named.get(&word) {
                parts.push(format!("{{{name}}}"));
            } else if cp.contains(&word) {
                if pos + 1 < end {
                    let param = read_u16_be(rom, pos);
                    pos += 2;
                    parts.push(format!("{{{word:04X}:{param:04X}}}"));
                } else {
                    parts.push(format!("{{{word:04X}}}"));
                }
            } else if clp.contains(&word) {
                if pos + 3 < end {
                    let p1 = read_u16_be(rom, pos);
                    let p2 = read_u16_be(rom, pos + 2);
                    pos += 4;
                    parts.push(format!("{{{word:04X}:{p1:04X}{p2:04X}}}"));
                } else {
                    parts.push(format!("{{{word:04X}}}"));
                }
            } else {
                parts.push(format!("{{{word:04X}}}"));
            }

            if is_terminator(word) {
                break;
            }
        } else if word == 0x0000 {
            parts.push(" ".to_string());
        } else if let Some(s) = charmap.get(&word) {
            parts.push(s.clone());
        } else {
            parts.push(format!("[{word:04X}]"));
        }
    }

    (parts.join(""), pos)
}

// ============================================================
// Entry extraction
// ============================================================

struct RawEntry {
    jp_offset: usize,
    section: String,
    jp_text: String,
    en_text: String,
    fff8_idx: i32,
    old_dialog_id: String,
}

/// Get EN text for an entry. Uses en_reference.json for FFF8 entries,
/// falls back to inline ROM decode for non-FFF8 entries.
#[allow(clippy::too_many_arguments)]
fn get_en_text(
    en_rom: &[u8],
    jp_rom: &[u8],
    offset: usize,
    en_charmap: &HashMap<u16, String>,
    en_ref: &HashMap<String, String>,
    cp: &HashSet<u16>,
    clp: &HashSet<u16>,
    named: &HashMap<u16, &str>,
) -> (String, i32) {
    // Check if EN ROM at this offset has FFF8 redirect
    if offset + 3 < en_rom.len() {
        let first_word = read_u16_be(en_rom, offset);
        if first_word == 0xFFF8 {
            let idx = read_u16_be(en_rom, offset + 2);
            // Use en_reference.json (already decoded, more reliable)
            let en_ref_key = format!("dialog_{idx:04}");
            if let Some(text) = en_ref.get(&en_ref_key) {
                return (text.clone(), idx as i32);
            }
            // Fallback: decode from FFF8 pointer table
            let ptr_loc = EN_FFF8_PTR_TABLE + idx as usize * 4;
            if ptr_loc + 3 < en_rom.len() {
                let text_addr = read_u32_be(en_rom, ptr_loc) as usize;
                if text_addr > 0 && text_addr < en_rom.len() {
                    let (text, _) =
                        decode_text(en_rom, text_addr, en_charmap, cp, clp, named, 8192);
                    return (text, idx as i32);
                }
            }
            return (String::new(), idx as i32);
        }
    }

    // Not FFF8: check if EN ROM differs from JP ROM at this offset (EN patch modified it)
    if offset + 4 <= jp_rom.len().min(en_rom.len()) {
        let jp_bytes = &jp_rom[offset..offset + 4];
        let en_bytes = &en_rom[offset..offset + 4];
        if jp_bytes != en_bytes {
            // EN patch modified this text — decode inline EN text
            let (text, _) = decode_text(en_rom, offset, en_charmap, cp, clp, named, 8192);
            if !text.is_empty() {
                return (text, -1);
            }
        }
    }

    (String::new(), -1)
}

/// Detect sub-pointer table at an address (heuristic).
/// Returns list of text addresses (direct or via sub-pointers).
fn resolve_sub_addresses(rom: &[u8], addr: usize) -> Vec<usize> {
    if addr + 1 >= rom.len() {
        return vec![];
    }

    let first_val = read_u16_be(rom, addr);

    // Heuristic: sub-pointer table starts with a small even offset
    if (0x0004..=0x2000).contains(&first_val) && first_val.is_multiple_of(2) {
        let mut offsets = Vec::new();
        let mut scan_pos = addr;
        while scan_pos + 1 < rom.len() {
            let off = read_u16_be(rom, scan_pos);
            if off == 0x0000 || off >= 0x4000 {
                break;
            }
            if !offsets.is_empty() && off < *offsets.last().unwrap() {
                break;
            }
            offsets.push(off);
            scan_pos += 2;
            if offsets.len() * 2 >= first_val as usize {
                break;
            }
        }

        if offsets.len() >= 2 {
            return offsets
                .iter()
                .map(|off| addr + *off as usize)
                .filter(|&a| a < rom.len())
                .collect();
        }
    }

    vec![addr]
}

/// Scan a text block in JP ROM, extracting all text entries.
/// For each entry, also gets corresponding EN text.
/// Continues scanning past block terminators to find subsequent entries
/// (EN patch splits large JP blocks into multiple FFF8 entries).
#[allow(clippy::too_many_arguments)]
fn scan_block(
    jp_rom: &[u8],
    en_rom: &[u8],
    block_start: usize,
    section: &str,
    jp_charmap: &HashMap<u16, String>,
    en_charmap: &HashMap<u16, String>,
    en_ref: &HashMap<String, String>,
    cp: &HashSet<u16>,
    clp: &HashSet<u16>,
    named: &HashMap<u16, &str>,
    protected_zones: &[(usize, usize)],
) -> Vec<RawEntry> {
    let mut entries = Vec::new();
    let mut pos = block_start;
    let max_pos = (block_start + 65536).min(jp_rom.len());
    let mut consecutive_fails = 0u32;

    while pos + 1 < max_pos {
        // Stop if we've entered a protected zone (Rel16 table header region)
        if protected_zones
            .iter()
            .any(|&(start, end)| pos >= start && pos < end)
        {
            break;
        }

        let word = read_u16_be(jp_rom, pos);

        // Skip standalone terminators
        if word == 0xFFFF || word == 0xFF38 {
            pos += 2;
            consecutive_fails += 1;
            if consecutive_fails > 4 {
                break;
            }
            continue;
        }

        let (jp_text, jp_end) = decode_text(jp_rom, pos, jp_charmap, cp, clp, named, 8192);
        if jp_end <= pos || jp_text.is_empty() {
            break;
        }

        let (en_text, fff8_idx) =
            get_en_text(en_rom, jp_rom, pos, en_charmap, en_ref, cp, clp, named);

        entries.push(RawEntry {
            jp_offset: pos,
            section: section.to_string(),
            jp_text: jp_text.clone(),
            en_text,
            fff8_idx,
            old_dialog_id: String::new(),
        });

        consecutive_fails = 0;
        pos = jp_end;
    }
    entries
}

/// Extract all text entries from all pointer tables.
#[allow(clippy::too_many_arguments)]
fn extract_all_entries(
    jp_rom: &[u8],
    en_rom: &[u8],
    jp_charmap: &HashMap<u16, String>,
    en_charmap: &HashMap<u16, String>,
    en_ref: &HashMap<String, String>,
    cp: &HashSet<u16>,
    clp: &HashSet<u16>,
    named: &HashMap<u16, &str>,
) -> BTreeMap<usize, RawEntry> {
    let mut entries: BTreeMap<usize, RawEntry> = BTreeMap::new();

    // Build protected zones: Rel16 table header regions that scan_block must not enter.
    // Each zone covers [table_addr, table_addr + header_bytes) where header_bytes = auto_count * 2.
    let mut protected_zones: Vec<(usize, usize)> = Vec::new();
    let mut rel16_bases: HashSet<usize> = HashSet::new();
    for table in PTR_TABLES {
        if matches!(table.format, PtrFormat::Rel16) {
            rel16_bases.insert(table.address);
            if table.address + 1 < jp_rom.len() {
                let first_off = read_u16_be(jp_rom, table.address) as usize;
                let auto_count =
                    if first_off > 0 && first_off.is_multiple_of(2) && first_off / 2 <= table.count
                    {
                        first_off / 2
                    } else {
                        table.count
                    };
                let header_bytes = auto_count * 2;
                protected_zones.push((table.address, table.address + header_bytes));
            }
        }
    }
    eprintln!(
        "  보호 구역 {}개 (Rel16 테이블 헤더)",
        protected_zones.len()
    );

    for table in PTR_TABLES {
        eprintln!(
            "  {} (0x{:06X}, {}개, {})...",
            table.name,
            table.address,
            table.count,
            match table.format {
                PtrFormat::Abs32 => "abs32",
                PtrFormat::Rel16 => "rel16",
            }
        );

        let mut section_count = 0usize;

        match table.format {
            PtrFormat::Abs32 => {
                let mut seen_ptrs = HashSet::new();
                for i in 0..table.count {
                    let ptr_addr_loc = table.address + i * 4;
                    if ptr_addr_loc + 3 >= jp_rom.len() {
                        break;
                    }
                    let ptr = read_u32_be(jp_rom, ptr_addr_loc) as usize;
                    if ptr < 0x200 || ptr >= jp_rom.len() {
                        continue;
                    }
                    if !seen_ptrs.insert(ptr) {
                        continue;
                    }
                    // Skip pointers to known Rel16 table bases (already processed)
                    if rel16_bases.contains(&ptr) {
                        continue;
                    }

                    let sub_addrs = resolve_sub_addresses(jp_rom, ptr);
                    for addr in sub_addrs {
                        if addr + 1 >= jp_rom.len() {
                            continue;
                        }
                        // Skip if address falls within a protected zone
                        if protected_zones.iter().any(|&(s, e)| addr >= s && addr < e) {
                            continue;
                        }
                        let block_entries = scan_block(
                            jp_rom,
                            en_rom,
                            addr,
                            table.name,
                            jp_charmap,
                            en_charmap,
                            en_ref,
                            cp,
                            clp,
                            named,
                            &protected_zones,
                        );
                        for entry in block_entries {
                            entries.entry(entry.jp_offset).or_insert_with(|| {
                                section_count += 1;
                                entry
                            });
                        }
                    }
                }
            }
            PtrFormat::Rel16 => {
                if table.address + 1 >= jp_rom.len() {
                    continue;
                }

                // Auto-detect entry count from first offset
                let first_off = read_u16_be(jp_rom, table.address) as usize;
                let auto_count =
                    if first_off > 0 && first_off.is_multiple_of(2) && first_off / 2 <= table.count
                    {
                        first_off / 2
                    } else {
                        table.count
                    };

                // Read all offsets
                let mut offsets = Vec::new();
                for i in 0..auto_count {
                    let pos = table.address + i * 2;
                    if pos + 1 >= jp_rom.len() {
                        break;
                    }
                    offsets.push(read_u16_be(jp_rom, pos) as usize);
                }

                for (i, &off) in offsets.iter().enumerate() {
                    let addr = table.address + off;
                    if entries.contains_key(&addr) {
                        continue;
                    }
                    if addr + 1 >= jp_rom.len() {
                        continue;
                    }

                    // Compute max bytes from next offset for bounded decode
                    let max_bytes = if i + 1 < offsets.len() {
                        let next_off = offsets[i + 1];
                        if next_off > off {
                            (next_off - off).min(8192)
                        } else {
                            8192
                        }
                    } else {
                        8192
                    };

                    let (jp_text, _) =
                        decode_text(jp_rom, addr, jp_charmap, cp, clp, named, max_bytes);
                    let (en_text, fff8_idx) =
                        get_en_text(en_rom, jp_rom, addr, en_charmap, en_ref, cp, clp, named);

                    entries.insert(
                        addr,
                        RawEntry {
                            jp_offset: addr,
                            section: table.name.to_string(),
                            jp_text,
                            en_text,
                            fff8_idx,
                            old_dialog_id: String::new(),
                        },
                    );
                    section_count += 1;
                }
            }
        }

        eprintln!("    → {section_count}개 엔트리");
    }

    // Direct-address entries (not in any pointer table)
    let direct_addrs: &[(&str, usize)] = &[
        ("enemy_24", 0x0A4DEE),
        ("enemy_25", 0x0A4E90),
        ("enemy_generic", 0x0A4F40),
    ];
    for &(section, addr) in direct_addrs {
        if entries.contains_key(&addr) || addr + 1 >= jp_rom.len() {
            continue;
        }
        let block = scan_block(
            jp_rom,
            en_rom,
            addr,
            section,
            jp_charmap,
            en_charmap,
            en_ref,
            cp,
            clp,
            named,
            &protected_zones,
        );
        let mut cnt = 0usize;
        for entry in block {
            entries.entry(entry.jp_offset).or_insert_with(|| {
                cnt += 1;
                entry
            });
        }
        if cnt > 0 {
            eprintln!("  {section} (0x{addr:06X}, direct) → {cnt}개 엔트리");
        }
    }

    entries
}

// ============================================================
// Data loaders
// ============================================================

/// Build FFF8 entry index → KR dialog ID mapping from text_en.json.
fn build_fff8_to_dialog_map(assets_dir: &Path) -> Result<HashMap<u16, String>, String> {
    let path = assets_dir.join("translation").join("text_en.json");
    let data =
        fs::read_to_string(&path).map_err(|e| format!("failed to read text_en.json: {e}"))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| format!("failed to parse text_en.json: {e}"))?;

    let entry_map = parsed
        .get("extended_entry_map")
        .and_then(|v| v.as_object())
        .ok_or("text_en.json missing extended_entry_map")?;

    let mut fff8_to_dialog: HashMap<u16, String> = HashMap::new();
    for (entry_idx_str, dialog_ids) in entry_map {
        let entry_idx: u16 = entry_idx_str
            .parse()
            .map_err(|e| format!("bad entry index {entry_idx_str}: {e}"))?;
        if let Some(arr) = dialog_ids.as_array()
            && let Some(first) = arr.first().and_then(|v| v.as_str())
        {
            fff8_to_dialog.insert(entry_idx, first.to_string());
        }
    }

    Ok(fff8_to_dialog)
}

/// Load EN reference text from en_reference.json.
fn load_en_reference(assets_dir: &Path) -> Result<HashMap<String, String>, String> {
    let path = assets_dir.join("en_reference.json");
    let data =
        fs::read_to_string(&path).map_err(|e| format!("failed to read en_reference.json: {e}"))?;
    let parsed: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| format!("failed to parse en_reference.json: {e}"))?;

    let entries = parsed
        .get("entries")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| {
            parsed
                .as_object()
                .expect("en_reference.json must be an object")
        });

    let mut result = HashMap::new();
    for (key, val) in entries {
        if let Some(text) = val.as_str() {
            result.insert(key.clone(), text.to_string());
        }
    }
    Ok(result)
}

/// Load all kr_*.json files → {key → kr_text}.
fn load_kr_translations(assets_dir: &Path) -> Result<BTreeMap<String, String>, String> {
    let tr_dir = assets_dir.join("translation");
    let mut entries = BTreeMap::new();

    let mut filenames: Vec<String> = Vec::new();
    for entry in
        fs::read_dir(&tr_dir).map_err(|e| format!("failed to read translation dir: {e}"))?
    {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with("kr_") && fname.ends_with(".json") && !fname.contains("raw") {
            filenames.push(fname);
        }
    }
    filenames.sort();

    for fname in &filenames {
        let path = tr_dir.join(fname);
        let data = fs::read_to_string(&path).map_err(|e| format!("failed to read {fname}: {e}"))?;
        let obj: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| format!("failed to parse {fname}: {e}"))?;

        if let Some(map) = obj.as_object() {
            for (key, val) in map {
                if key.starts_with('_') {
                    continue;
                }
                if let Some(text) = val.as_str()
                    && !text.is_empty()
                {
                    entries.insert(key.clone(), text.to_string());
                }
            }
        }
    }

    Ok(entries)
}

// ============================================================
// FFF8 supplemental scan
// ============================================================

/// Scan all FFF8 entries in EN ROM and add any not already captured
/// by JP pointer table scanning.
fn supplement_fff8_entries(
    entries: &mut BTreeMap<usize, RawEntry>,
    en_rom: &[u8],
    en_charmap: &HashMap<u16, String>,
    en_ref: &HashMap<String, String>,
    cp: &HashSet<u16>,
    clp: &HashSet<u16>,
    named: &HashMap<u16, &str>,
) {
    // Determine FFF8 table size from first pointer
    let first_ptr = read_u32_be(en_rom, EN_FFF8_PTR_TABLE) as usize;
    let max_entry = if EN_FFF8_PTR_TABLE < first_ptr && first_ptr <= 0x3FFFFF {
        (first_ptr - EN_FFF8_PTR_TABLE) / 4
    } else {
        1273
    };

    // Collect FFF8 indices already captured
    let captured_fff8: HashSet<i32> = entries
        .values()
        .filter(|e| e.fff8_idx >= 0)
        .map(|e| e.fff8_idx)
        .collect();

    let mut added = 0usize;

    for idx in 0..max_entry {
        if captured_fff8.contains(&(idx as i32)) {
            continue;
        }

        let ptr_addr = EN_FFF8_PTR_TABLE + idx * 4;
        if ptr_addr + 3 >= en_rom.len() {
            break;
        }
        let text_ptr = read_u32_be(en_rom, ptr_addr) as usize;
        if !(0x200000..=0x3FFFFF).contains(&text_ptr) {
            continue;
        }

        // Decode EN text from FFF8 pointer
        let en_ref_key = format!("dialog_{idx:04}");
        let en_text = if let Some(text) = en_ref.get(&en_ref_key) {
            text.clone()
        } else {
            let (text, _) = decode_text(en_rom, text_ptr, en_charmap, cp, clp, named, 8192);
            text
        };

        if en_text.is_empty() {
            continue;
        }

        // Use a synthetic offset beyond JP ROM (0x200000 + idx) to avoid collisions
        let synthetic_offset = 0x200000 + idx;

        entries.insert(
            synthetic_offset,
            RawEntry {
                jp_offset: synthetic_offset,
                section: "fff8_only".to_string(),
                jp_text: String::new(),
                en_text,
                fff8_idx: idx as i32,
                old_dialog_id: String::new(),
            },
        );
        added += 1;
    }

    eprintln!("  FFF8 보충 스캔: {added}개 추가 (총 FFF8: {max_entry}개)");
}

// ============================================================
// KR matching
// ============================================================

fn match_kr_translations(
    entries: &mut BTreeMap<usize, RawEntry>,
    fff8_to_dialog: &HashMap<u16, String>,
) {
    for entry in entries.values_mut() {
        // Skip entries that already have old_dialog_id set (e.g. plural entries)
        if !entry.old_dialog_id.is_empty() {
            continue;
        }
        if entry.fff8_idx >= 0 {
            let idx = entry.fff8_idx as u16;
            entry.old_dialog_id = fff8_to_dialog
                .get(&idx)
                .cloned()
                .unwrap_or_else(|| format!("dialog_{idx:04}"));
        }
    }
}

// ============================================================
// JSON output
// ============================================================

#[derive(serde::Serialize)]
struct OutputFile {
    source: String,
    range: String,
    count: usize,
    entries: Vec<OutputEntry>,
}

#[derive(serde::Serialize)]
struct OutputEntry {
    id: String,
    offset: String,
    old_dialog_id: String,
    fff8_idx: i32,
    section: String,
    jp: String,
    en: String,
    ko: String,
    status: String,
    notes: String,
}

fn output_chunked_json(
    sorted_entries: &[(usize, &RawEntry)],
    kr_trans: &BTreeMap<String, String>,
    output_dir: &Path,
    chunk_size: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    fs::create_dir_all(output_dir)?;

    // Clean old script_*.json files
    if let Ok(dir) = fs::read_dir(output_dir) {
        for entry in dir.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("script_") && fname.ends_with(".json") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    let mut file_count = 0;

    for (chunk_idx, chunk) in sorted_entries.chunks(chunk_size).enumerate() {
        let first_seq = chunk_idx * chunk_size;
        let last_seq = first_seq + chunk.len() - 1;

        let first_offset = chunk
            .first()
            .map(|(off, _)| format!("0x{off:06x}"))
            .unwrap_or_default();
        let last_offset = chunk
            .last()
            .map(|(off, _)| format!("0x{off:06x}"))
            .unwrap_or_default();

        let output_entries: Vec<OutputEntry> = chunk
            .iter()
            .enumerate()
            .map(|(i, (_, entry))| {
                let seq_id = first_seq + i;
                let kr_text = if !entry.old_dialog_id.is_empty() {
                    kr_trans
                        .get(&entry.old_dialog_id)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                let status = if !kr_text.is_empty() {
                    "done"
                } else if entry.jp_text.is_empty() {
                    "empty"
                } else {
                    "needs_translation"
                };

                // Use old_dialog_id as id for plural entries
                let id = if entry.old_dialog_id.starts_with("plural_") {
                    entry.old_dialog_id.clone()
                } else {
                    format!("script_{seq_id:04}")
                };

                OutputEntry {
                    id,
                    offset: format!("0x{:06x}", entry.jp_offset),
                    old_dialog_id: entry.old_dialog_id.clone(),
                    fff8_idx: entry.fff8_idx,
                    section: entry.section.clone(),
                    jp: entry.jp_text.clone(),
                    en: entry.en_text.clone(),
                    ko: kr_text,
                    status: status.to_string(),
                    notes: String::new(),
                }
            })
            .collect();

        let output_file = OutputFile {
            source: "madou1_md".to_string(),
            range: format!(
                "script_{first_seq:04} ({first_offset}) ~ script_{last_seq:04} ({last_offset})"
            ),
            count: output_entries.len(),
            entries: output_entries,
        };

        let filename = format!("script_{first_seq:04}.json");
        let json = serde_json::to_string_pretty(&output_file)?;
        fs::write(output_dir.join(&filename), json.as_bytes())?;
        file_count += 1;
    }

    Ok(file_count)
}

// ============================================================
// Main entry point
// ============================================================

pub fn run(
    jp_rom: &[u8],
    en_rom: &[u8],
    assets_dir: &Path,
    output_dir: &Path,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("============================================================");
    eprintln!("Madou Monogatari I - Align (JP-EN-KR 완전 정렬)");
    eprintln!("============================================================");

    eprintln!("\nROM 크기:");
    eprintln!("  JP: {} bytes (0x{:X})", jp_rom.len(), jp_rom.len());
    eprintln!("  EN: {} bytes (0x{:X})", en_rom.len(), en_rom.len());

    if jp_rom.len() < 0x100000 {
        return Err("JP ROM too small".into());
    }
    if en_rom.len() != 0x400000 {
        return Err(format!(
            "EN ROM size mismatch: expected 0x400000, got 0x{:X}",
            en_rom.len()
        )
        .into());
    }

    // Build charmaps
    eprintln!("\ncharmap 구성...");
    let jp_charmap = build_jp_charmap();
    let en_charmap = build_en_charmap();
    eprintln!("  JP: {}개, EN: {}개", jp_charmap.len(), en_charmap.len());

    let cp = ctrl_with_param();
    let clp = ctrl_with_long_param();
    let named = named_ctrl();

    // Load data files
    eprintln!("\n데이터 파일 로딩...");
    let en_ref = load_en_reference(assets_dir)?;
    eprintln!("  en_reference: {}개", en_ref.len());

    let fff8_to_dialog = build_fff8_to_dialog_map(assets_dir)?;
    eprintln!("  FFF8→dialog 매핑: {}개", fff8_to_dialog.len());

    let kr_trans = load_kr_translations(assets_dir)?;
    eprintln!("  KR 번역: {}개", kr_trans.len());

    // Extract all entries from pointer tables
    eprintln!("\n포인터 테이블 스캔...");
    let mut entries = extract_all_entries(
        jp_rom,
        en_rom,
        &jp_charmap,
        &en_charmap,
        &en_ref,
        &cp,
        &clp,
        &named,
    );

    // Filter out entries with broken JP text (pointer table data decoded as text).
    // These contain unknown tile references like [0314] that indicate non-text data.
    // Such entries are sub-pointer table headers that scan_block decoded as characters.
    let before_filter = entries.len();
    entries.retain(|_, entry| {
        if entry.jp_text.is_empty() {
            return true; // keep EN-only entries
        }
        // Count unknown tile references [XXXX] (4-digit hex in brackets)
        let mut unknown_count = 0u32;
        let bytes = entry.jp_text.as_bytes();
        for i in 0..bytes.len().saturating_sub(5) {
            if bytes[i] == b'['
                && bytes[i + 5] == b']'
                && bytes[i + 1..i + 5].iter().all(|c| c.is_ascii_hexdigit())
            {
                unknown_count += 1;
            }
        }
        // 3+ unknown tiles = pointer table data, not real text
        if unknown_count >= 3 {
            return false;
        }
        true
    });
    let filtered = before_filter - entries.len();
    if filtered > 0 {
        eprintln!("  포인터 테이블 데이터 {}개 필터링됨", filtered);
    }

    // Supplement with FFF8 entries not captured by JP pointer tables
    eprintln!("\nFFF8 보충 스캔...");
    supplement_fff8_entries(
        &mut entries,
        en_rom,
        &en_charmap,
        &en_ref,
        &cp,
        &clp,
        &named,
    );

    // Add plural entries (hardcoded EN patch, not in JP ROM)
    {
        let plural_defs: &[(&str, &str)] = &[
            (
                "plural_capsule_prompt",
                "{FF10}The {FF78:FF30}are sleeping{NL}inside the{NL}Capsule.{PAGE}Release the{NL}monsters,{NL}{pad}{pad}▸Yes{NL}{pad}{pad}▸No{FF74}{FFFF}",
            ),
            (
                "plural_capsule_released",
                "{FF10}The {FF78:FF30}leave the{NL}Capsule! eager to{NL}please.{PAGE}{FF10}{q-open}You can go{NL}now.{q}{PAGE}{FF10}The {FF78:FF30}look sad! but{NL}comply.{PAGE}{FFFF}",
            ),
            ("plural_monster_encounter", "{FF10}Monsters!{END}"),
            (
                "plural_amigo_defeated",
                "{FF10}The {FF78:FF30}are defeated.{FF38}",
            ),
        ];
        let base_offset = 0x400000; // synthetic offset beyond ROM
        for (i, (key, en_text)) in plural_defs.iter().enumerate() {
            entries.insert(
                base_offset + i,
                RawEntry {
                    jp_offset: base_offset + i,
                    section: "plural".to_string(),
                    jp_text: String::new(),
                    en_text: en_text.to_string(),
                    fff8_idx: -1,
                    old_dialog_id: key.to_string(),
                },
            );
        }
        eprintln!("  plural: {}개 추가", plural_defs.len());
    }

    // Match KR translations
    eprintln!("\nKR 번역 매칭...");
    match_kr_translations(&mut entries, &fff8_to_dialog);

    // Statistics
    let total = entries.len();
    let jp_has = entries.values().filter(|e| !e.jp_text.is_empty()).count();
    let en_has = entries.values().filter(|e| !e.en_text.is_empty()).count();
    let kr_matched = entries
        .values()
        .filter(|e| !e.old_dialog_id.is_empty() && kr_trans.contains_key(&e.old_dialog_id))
        .count();
    let all_three = entries
        .values()
        .filter(|e| {
            !e.jp_text.is_empty()
                && !e.en_text.is_empty()
                && !e.old_dialog_id.is_empty()
                && kr_trans.contains_key(&e.old_dialog_id)
        })
        .count();

    eprintln!("\n=== 정렬 통계 ===");
    eprintln!("총 엔트리: {total}");
    eprintln!("JP 텍스트: {jp_has}");
    eprintln!("EN 텍스트: {en_has}");
    eprintln!("KR 매칭: {kr_matched}");
    eprintln!("JP+EN+KR 모두: {all_three}");

    // Output chunked JSON
    eprintln!("\nJSON 출력: {}", output_dir.display());
    let sorted_entries: Vec<(usize, &RawEntry)> = entries.iter().map(|(&k, v)| (k, v)).collect();
    let file_count = output_chunked_json(&sorted_entries, &kr_trans, output_dir, chunk_size)?;
    eprintln!("  {file_count}개 파일 생성 ({chunk_size}개/파일)");

    eprintln!("\n완료!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_jp_charmap_basics() {
        let cm = build_jp_charmap();
        assert_eq!(cm[&0x0000], "\u{3000}");
        assert_eq!(cm[&0x01], "０");
        assert_eq!(cm[&0x10], "あ");
        assert_eq!(cm[&0x47], "ア");
        assert_eq!(cm[&0x83], "ー");
        assert_eq!(cm.len(), 514);
    }

    #[test]
    fn test_decode_text_simple() {
        let cm = build_jp_charmap();
        let cp = ctrl_with_param();
        let clp = ctrl_with_long_param();
        let named = named_ctrl();

        let rom: Vec<u8> = vec![0x00, 0x10, 0x00, 0x11, 0x00, 0x12, 0xFF, 0x04];
        let (text, _) = decode_text(&rom, 0, &cm, &cp, &clp, &named, 8192);
        assert_eq!(text, "あいう{END}");
    }

    #[test]
    fn test_decode_text_with_ctrl() {
        let cm = build_jp_charmap();
        let cp = ctrl_with_param();
        let clp = ctrl_with_long_param();
        let named = named_ctrl();

        let rom: Vec<u8> = vec![
            0xFF, 0x50, 0x00, 0x1D, 0xFF, 0x10, 0x00, 0x10, 0xFF, 0x30, 0xFF, 0x04,
        ];
        let (text, _) = decode_text(&rom, 0, &cm, &cp, &clp, &named, 8192);
        assert_eq!(text, "{FF50:001D}{FF10}あ{NL}{END}");
    }

    #[test]
    fn test_en_charmap() {
        let cm = build_en_charmap();
        assert_eq!(cm[&0x0B], "A");
        assert_eq!(cm[&0x25], "a");
        assert_eq!(cm[&0x01], "0");
        assert_eq!(cm[&0x3F], ".");
        assert_eq!(cm[&0x4A], " ");
    }
}
