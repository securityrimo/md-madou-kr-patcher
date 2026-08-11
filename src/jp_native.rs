//! JP-ROM-native Korean build stages.
//!
//! M0 establishes an exact-source, fail-closed 4 MiB expansion baseline. M1
//! adds a typed-ISA fixed-font dispatcher and the full current Hangul glyph
//! set while retaining the original JP renderer for existing JP codes. M2
//! redirects stable JP script entries into JP-source KR text storage without
//! depending on the English charmap or executable patch code. M3 repacks the
//! original JP item-name table in place for its native dynamic-text consumer.
//! M4 promotes the next runtime-proven opening-prologue entry while preserving
//! the byte-identical M2/M3 build recipes. M5 repacks the adjacent quoted-item
//! text table for its original JP consumer. M6 repacks the separate item
//! description pointer population across only its source-owned string slots.
//! M7 repacks the ordinary item-use messages and preserves the one battle-use
//! reference that shares an item-description target. M8 repacks the battle-use
//! messages, retains that shared target, and closes the one source fallthrough
//! explicitly. M9 promotes the recovery-result population, M10 promotes the
//! health-status population, M11 promotes the remaining-MP population with
//! their native JP face icons, and M12 promotes the contiguous Puyo battle
//! action population. M13 through M16 finish the next enemy battle batches,
//! and M17 through M42 progressively promote JP-source dungeon consumers. M43
//! through M46 promote the native shop consumer batches, M47 through M49
//! promote the native 256-pixel-wide intro consumers, M50 through M52 continue
//! through the matching ending consumers, and M53 closes the EN-only long-item-name
//! exceptions in favor of the M34 native dynamic-name consumers. M54 closes the
//! trailing EN split-key ledger without promoting duplicate payloads or the
//! remaining ambiguous consumers. M55 promotes the first native system batch
//! and localizes dynamic save/floor suffix tiles through the generated KR
//! charmap. M56 promotes the following chest, item, note, and door system batch
//! while preserving the two source fallthrough tails and suppressing the JP
//! item-object suffix on particle-free KR lines. M57 promotes the native door,
//! stair, spell-tutorial, and dungeon-hazard system batch. M58 proves that the
//! two following blank-looking reward templates are unselected JP pointer-table
//! slots rather than live dynamic consumers. M59 promotes the remaining live
//! system-table messages for an exhausted wall, the floating-stone choice, and
//! a dark passage. M60 preserves the JP 32-entry monster-name pointer table and
//! writes 25 Korean names into its original six-word fixed records. M61 promotes
//! the complete native damage-voice population while preserving its JP-only
//! control prefixes and operands. M62 closes the early Puyo battle population
//! immediately before M12 while leaving its empty slot and unconsumed physical
//! fragments in JP form. M63 promotes the visible monster-encounter introduction
//! consumers while preserving their two control-only transition slots. M64
//! promotes the complete non-voiced damage population as the control-stripped
//! semantic counterpart of M61. M65 promotes the source-backed spell and
//! battle-effect messages while keeping the zero EN pointer at index 192 and
//! JP `FFC0` segment boundaries intact. M66 promotes the one following standalone
//! event while preserving two JP control-prefix parents that fall through into
//! already-promoted M65 tails. M67 promotes the native ten-entry spell-command
//! table without the EN-only header/list tiles. M68 promotes the two special
//! item-event consumers that sit outside the three native item pointer tables.
//! M69 promotes the contiguous sixteen-entry enemy-health status ladder while
//! restoring the exact JP face stage for every entry. M70 promotes the eight
//! enemy-damage responses that immediately follow it and restores the JP
//! intensity ladder without EN wording. M71 audits all 1,273 legacy FFF8
//! indices against their stable, native, EN-only, deferred, or absent owner
//! without changing the ROM. M72 audits the enemy status-table bindings, and
//! M74 audits the normal voiced and two alternate non-voiced player-damage
//! bindings without changing the ROM. The earlier glyph-slot PoC remains
//! available as a diagnostic-only proof.

mod audits;
mod dynamic_layout;
mod expected_writes;
mod item_labels;
mod item_messages;
mod map_overlay;
mod milestones;
mod monster_names;
mod native_font;
mod native_text;
mod text_catalog;

pub use audits::{
    EnemyStatusConsumerReport, Fff8Ownership, Fff8OwnershipReport, Fff8OwnershipRow,
    PlayerDamageConsumerReport, audit_enemy_status_consumers, audit_fff8_ownership,
    audit_player_damage_consumers,
};
pub use milestones::*;
use milestones::{m58_direct_jsr_offsets, m58_expect_instruction, m58_read_word, m70_text_specs};

use audits::{
    validate_m70_enemy_damage_boundaries, validate_m72_enemy_status_consumers,
    validate_m75_player_damage_path_classification,
};

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use fontdue::{Font, FontSettings};
use sha2::{Digest, Sha256};

use crate::build::text::{Token, words_to_bytes};
use crate::m68k::{
    AddressReg, BranchCondition, BranchWidth, DataReg, Inst, assemble as assemble_m68k,
    assemble_at as assemble_m68k_at,
};
use dynamic_layout::*;
use expected_writes::*;
use item_labels::*;
use item_messages::*;
use map_overlay::*;
use monster_names::*;
pub(crate) use native_font::render_native_glyph;
use native_font::{
    collect_jp_native_glyphs, collect_scoped_jp_native_glyphs, jp_font_offset, render_native_font,
};
use native_text::*;
use text_catalog::*;

const JP_ROM_SIZE: usize = 0x20_0000;
const JP_EXPANDED_ROM_SIZE: usize = 0x40_0000;
const HEADER_OFFSET: usize = 0x100;
const ROM_END_OFFSET: usize = 0x1A4;
const CHECKSUM_OFFSET: usize = 0x18E;
const CHECKSUM_START: usize = 0x200;
const JP_KANA_FONT_BASE: usize = 0x0B_F000;
const JP_KANJI_FONT_BASE: usize = 0x0B_4FC0;
const JP_HIGH_CODE_START: u16 = 0x0080;
const KR_CODE_START: u16 = 0x8000;
const BYTES_PER_GLYPH: usize = 32;

const JP_NATIVE_HOOK_POINT: usize = 0x049202;
const JP_NATIVE_HOOK_CODE: usize = 0x0B2514;
const JP_NATIVE_DISPLAY_ADVANCE_POINT: usize = 0x0492E4;
const JP_NATIVE_DISPLAY_ADVANCE_CODE: usize = 0x33E000;
const JP_NATIVE_BLANK_ADVANCE_POINT: usize = 0x04930A;
const JP_NATIVE_BLANK_ADVANCE_CODE: usize = 0x33E100;
const JP_NATIVE_FONT_BASE: usize = 0x340000;
const JP_TEXT_OPCODE_HANDLER_SLOT: usize = 0x048E8E;
const JP_TEXT_REDIRECT_HANDLER: usize = 0x0B2560;
const JP_TEXT_REDIRECT_RETURN: u32 = 0x0004_8D32;
const JP_ORIGINAL_FFF8_HANDLER: u32 = 0x0004_900A;
const JP_TEXT_REDIRECT_MAGIC: u16 = 0x4B52;
const JP_TEXT_ROW_FINALIZE_MAGIC: u16 = 0x4B48;
const JP_TEXT_POINTER_TABLE: usize = 0x350000;
const JP_NATIVE_FULL_WIDTH_LAYOUT_PAD: u16 = 0x0000;
const M2_KR_PUNCTUATION: [char; 6] = ['.', '?', '!', ',', '*', '…'];
const JP_NATIVE_HALF_WIDTH_CHARS: [char; 7] = [' ', '.', '?', '!', ',', '*', '~'];
const M2_MAX_FIXED_GLYPHS_PER_LINE: usize = 8;
const M2_MAX_LINES_PER_PAGE: usize = 4;
const M3_KR_EXTRA_GLYPHS: [char; 2] = ['A', '-'];
// Keep code points stable when later JP-source edits retire the final use of a glyph.
const JP_NATIVE_RETIRED_GLYPHS: [char; 28] = [
    '극', '급', '낮', '뜻', '률', '박', '봤', '섭', '쫄', '암', '잔', '풍', '즉', '탈', '뜩', '확',
    '씰', '쩡', '훨', '씬', '궁', '짜', '맣', '꼈', '땅', '욕', '컨', '션',
];
// Later milestones append newly introduced glyphs instead of letting the
// globally scanned asset set shift earlier stable code points.
const JP_NATIVE_DEFERRED_GLYPHS: [char; 7] = ['찔', '꼴', '훌', '륭', '웅', '빼', '쌌'];
const M51_KR_EXTRA_GLYPHS: [char; 6] = ['T', '찔', '꼴', '훌', '륭', '웅'];
const M65_KR_EXTRA_GLYPHS: [char; 8] = ['T', '찔', '꼴', '훌', '륭', '웅', '빼', '쌌'];
const JP_MONSTER_NAME_TABLE: usize = 0x09_ECDC;
const JP_MONSTER_NAME_TABLE_COUNT: usize = 32;
const JP_MONSTER_NAME_DATA_START: usize = 0x09_ED1C;
const JP_MONSTER_NAME_DATA_END: usize = 0x09_EE48;
const JP_MONSTER_NAME_RECORD_WORDS: usize = 6;
const JP_MONSTER_NAME_UNIQUE_COUNT: usize = 25;
const JP_MONSTER_NAME_LOADER: usize = 0x0000_8272;
const JP_MONSTER_NAME_PC_RELATIVE_CALLSITE: usize = 0x0000_81DA;
const JP_MONSTER_NAME_ABSOLUTE_CALLSITE: usize = 0x0004_BF10;
const JP_MONSTER_NAME_BUFFER: u16 = 0x86AA;
const M60_MONSTER_NAME_TARGET_RECORDS: [u8; JP_MONSTER_NAME_TABLE_COUNT] = [
    0,
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
    20,
    21,
    22,
    23,
    23,
    23,
    24,
    u8::MAX,
    u8::MAX,
    u8::MAX,
    u8::MAX,
];
const JP_ITEM_NAME_TABLE: usize = 0x0A_08C6;
const JP_ITEM_NAME_TABLE_COUNT: usize = 52;
const JP_ITEM_NAME_DATA_START: usize = 0x0A_092E;
const JP_ITEM_NAME_DATA_END: usize = 0x0A_0AEC;
const JP_ITEM_NAME_UNIQUE_COUNT: usize = 40;
const JP_ITEM_NAME_VISIBLE_WORDS: usize = 7;
const JP_ITEM_QUOTED_TABLE: usize = 0x0A_0AEC;
const JP_ITEM_QUOTED_TABLE_COUNT: usize = 52;
const JP_ITEM_QUOTED_DATA_START: usize = 0x0A_0B54;
const JP_ITEM_QUOTED_DATA_END: usize = 0x0A_0E12;
const JP_ITEM_QUOTED_UNIQUE_COUNT: usize = 40;
const JP_ITEM_QUOTED_OPEN: u16 = 0x007E;
const JP_ITEM_QUOTED_CLOSE: u16 = 0x007F;
const JP_ITEM_QUOTED_MAX_GLYPHS_PER_LINE: usize = 8;
const JP_ITEM_DESC_TABLE: usize = 0x0A_0E12;
const JP_ITEM_DESC_TABLE_COUNT: usize = 52;
const JP_ITEM_DESC_UNIQUE_COUNT: usize = 38;
const JP_ITEM_DESC_OWNED_UNIQUE_COUNT: usize = 39;
const JP_ITEM_DESC_TOTAL_UNIQUE_TARGET_COUNT: usize = 40;
const JP_ITEM_DESC_UNUSED_INDEX: usize = 39;
const JP_ITEM_DESC_UNUSED_TARGET: usize = 0x0A_26B6;
const JP_ITEM_DESC_SHARED_EVENT_ID: &str = "script_0325";
const JP_ITEM_DESC_SHARED_EVENT_OFFSET: usize = 0x0A_0F88;
const JP_ITEM_DESC_MAX_GLYPHS_PER_LINE: usize = 8;
const JP_ITEM_USE_TABLE: usize = 0x0A_0E7A;
const JP_ITEM_USE_TABLE_COUNT: usize = 51;
const JP_ITEM_USE_UNIQUE_COUNT: usize = 51;
const JP_ITEM_USE2_TABLE: usize = 0x0A_0EE0;
const JP_ITEM_USE2_TABLE_COUNT: usize = 45;
const JP_ITEM_USE2_UNIQUE_COUNT: usize = 45;
const JP_ITEM_USE2_ASSET_COUNT: usize = 44;
const JP_ITEM_USE2_FALLTHROUGH_ID: &str = "script_0400";
const JP_ITEM_USE2_FALLTHROUGH_TARGET_ID: &str = "script_0401";
const JP_ITEM_USE2_SHARED_SUFFIX_ID: &str = "script_0394";
const JP_ITEM_USE2_SHARED_SUFFIX_TARGET_ID: &str = "script_0395";
const JP_ITEM_DESC_USE2_SHARED_TARGET_COUNT: usize = 1;

const JP_ITEM_USE_MAX_GLYPHS_PER_LINE: usize = 8;
const JP_QUOTE_OPEN: u16 = 0x00D6;
const JP_QUOTE_CLOSE: u16 = 0x00D7;
const EN_FACE_TILE_START: u16 = 0x005F;
const EN_FACE_TILE_END: u16 = 0x0063;
const JP_FACE_TILE_START: u16 = 0x00EB;
const JP_FACE_TILE_END: u16 = 0x00EF;
const JP_HEART_TILE: u16 = 0x00DD;
const JP_SWEAT1_TILE: u16 = 0x00DE;
const JP_SWEAT2_TILE: u16 = 0x00DF;
const JP_SWEAT3_TILE: u16 = 0x00E0;
const JP_BANDAGE_TILE: u16 = 0x00E2;
const JP_SURPRISE_TILE: u16 = 0x00E4;
const JP_BLUSH_TILE: u16 = 0x00E5;
const JP_SMALL_STAR_TILE: u16 = 0x00E6;
const JP_ANGER_TILE: u16 = 0x00EA;
const JP_MUSIC_NOTE_TILE: u16 = 0x00F1;
const JP_WHITE_STAR_TILE: u16 = 0x00F3;
const JP_BLACK_STAR_TILE: u16 = 0x00F4;
const JP_PERCENT_TILE: u16 = 0x01C5;
const JP_SPELL_UP_TILE: u16 = 0x01C9;
const JP_SPELL_DOWN_TILE: u16 = 0x01CA;
const JP_SPELL_LEFT_TILE: u16 = 0x00FB;
const JP_SPELL_RIGHT_TILE: u16 = 0x00FC;
const JP_SPELL_COMMAND_TABLE: usize = 0x09_E4B2;
const JP_SPELL_COMMAND_TABLE_COUNT: usize = 10;
const JP_SPELL_COMMAND_CONSUMER: usize = 0x0000_3C5A;
const JP_ENEMY_HP_TABLE: usize = 0x0A_26B6;
const JP_ENEMY_DAMAGE_TABLE: usize = 0x0A_299E;
const JP_ENEMY_STATUS_POINTER_ARRAY: u16 = 0x8732;
const JP_ENEMY_STATUS_ARRAY_INIT: usize = 0x0000_4446;
const JP_ENEMY_STATUS_OBJECT_BINDING: usize = 0x0000_4760;
const JP_PLAYER_DAMAGE_POINTER_VECTOR: usize = 0x09_E170;
const JP_MONSTER_BATTLE_DIALOGUE_POINTER_VECTOR: u32 = 0x0009_E190;
const JP_DAMAGE_VOICE_TABLE: usize = 0x09_F6EC;
const JP_DAMAGE_NOVOICE_TABLE: usize = 0x09_F972;
const JP_PLAYER_MESSAGE_POINTER_ARRAY: u32 = 0x00FF_8700;
const JP_PLAYER_DAMAGE_NORMAL_INIT: usize = 0x0000_4426;
const JP_PLAYER_DAMAGE_CAMUS_CUTSCENE_INIT: usize = 0x0004_756C;
const JP_PLAYER_DAMAGE_AMIGO_BATTLE_INIT: usize = 0x0004_C25A;
const JP_PLAYER_MESSAGE_OBJECT_BINDING: usize = 0x0000_471E;
const JP_CAMUS_BATTLE_SAVE_ARLE: usize = 0x0004_750E;
const JP_CAMUS_BATTLE_STATS_SOURCE: u32 = 0x0006_4F38;
const JP_PLAYER_BATTLE_STATS: u16 = 0xF9C4;
const JP_CAMUS_BATTLE_DIALOGUE_SELECT: usize = 0x0004_755C;
const JP_CAMUS_BATTLE_ENEMY_SELECT: usize = 0x0004_7586;
const JP_CAMUS_BATTLE_RESTORE_ARLE: usize = 0x0004_7606;
const JP_AMIGO_CAPTURE_MONSTER: usize = 0x0000_B604;
const JP_AMIGO_CAPTURED_MONSTER_ID: u16 = 0xF9D5;
const JP_AMIGO_CAPTURED_STATS_POINTER: u16 = 0x88B8;
const JP_AMIGO_CAPTURED_STATS: u16 = 0xF9DA;
const JP_AMIGO_BATTLE_ENTRY: usize = 0x0004_C106;
const JP_AMIGO_BATTLE_STATS_SWAP: usize = 0x0004_C15E;
const JP_AMIGO_BATTLE_DIALOGUE_SELECT: usize = 0x0004_C242;

const EXPECTED_HOOK_POINT: [u8; 6] = [0x45, 0xF9, 0x00, 0x0B, 0xF0, 0x00];
const EXPECTED_DISPLAY_ADVANCE_POINT: [u8; 6] = [0x58, 0x68, 0x00, 0x4C, 0x4E, 0x75];
const EXPECTED_BLANK_ADVANCE_POINT: [u8; 10] =
    [0x23, 0xFC, 0x80, 0x01, 0x80, 0x01, 0x00, 0xC0, 0x00, 0x00];
const EXPECTED_FFF8_HANDLER_POINTER: [u8; 4] = [0x00, 0x04, 0x90, 0x0A];

const SRAM_WRAPPER_OFFSET: usize = 0x14FEB0;
const SRAM_REGISTER: u32 = 0x00A130F1;
const SRAM_LATEST_SLOT: u32 = 0x00201029;
const SRAM_PRESENT_UNLOCKED: u32 = 0x0020102A;

const SRAM_DISABLE_OFFSETS: [usize; 10] = [
    0x04B9D0, 0x04B9FC, 0x04BA4C, 0x04BA72, 0x04BA80, 0x04BAAC, 0x04BAD4, 0x04BAFC, 0x04BB2A,
    0x04BB58,
];
const EXPECTED_SRAM_ENABLE_WRITE: [u8; 8] = [0x13, 0xFC, 0x00, 0x03, 0x00, 0xA1, 0x30, 0xF1];

const JP_SHA256: [u8; 32] = [
    0x61, 0xD1, 0xDE, 0xC3, 0x19, 0xAF, 0xB1, 0x38, 0x0D, 0xFB, 0xEE, 0x0C, 0xDB, 0x42, 0xE6, 0xE6,
    0x4A, 0xB1, 0x80, 0x94, 0x1B, 0x0D, 0x53, 0x3D, 0x8D, 0xEE, 0xD9, 0xEF, 0xA5, 0x02, 0xF8, 0x3C,
];

pub(crate) fn supported_jp_sha256_hex() -> String {
    bytes_to_hex(&JP_SHA256)
}

const ENCOUNTER_SCRIPT_OFFSET: usize = 0x09_E3AE;
const EXPECTED_ENCOUNTER_SCRIPT: [u8; 14] = [
    0xFF, 0x10, 0x00, 0x2E, 0x00, 0x32, 0x00, 0x28, 0x00, 0xAA, 0x00, 0x80, 0xFF, 0x04,
];

const SHOWCASE_SCRIPT_OFFSET: usize = 0x0A_EC98;
const EXPECTED_SHOWCASE_SCRIPT: [u8; 16] = [
    0x00, 0x19, 0x00, 0x19, 0x00, 0x29, 0xFF, 0x30, 0x00, 0xAE, 0x00, 0x19, 0x00, 0xAA, 0x00, 0x3A,
];
const REPLACEMENT_SHOWCASE_SCRIPT: [u8; 16] = [
    0x00, 0x2E, 0x00, 0x32, 0x00, 0x28, 0x00, 0xAA, 0x00, 0x80, 0xFF, 0xB4, 0xFF, 0x64, 0xFF, 0x04,
];

const EXPECTED_GLYPH_002E: [u8; BYTES_PER_GLYPH] = [
    0x00, 0x00, 0x00, 0xC0, 0x00, 0xC0, 0x3F, 0xFC, 0x00, 0xC0, 0x00, 0xC0, 0x3F, 0xFC, 0x00, 0xC0,
    0x00, 0xC0, 0x1F, 0xC0, 0x39, 0xE0, 0x30, 0xF8, 0x30, 0xDC, 0x39, 0xCC, 0x1F, 0x80, 0x00, 0x00,
];
const EXPECTED_GLYPH_0032: [u8; BYTES_PER_GLYPH] = [
    0x00, 0x00, 0x06, 0x00, 0x06, 0x00, 0x06, 0x00, 0x3F, 0xE0, 0x06, 0x00, 0x06, 0x00, 0x36, 0x00,
    0x3F, 0xC0, 0x0C, 0x18, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x06, 0x18, 0x03, 0xF0, 0x00, 0x00,
];
const EXPECTED_GLYPH_0028: [u8; BYTES_PER_GLYPH] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xE0, 0x1F, 0xF8, 0x39, 0x9C, 0x31, 0x8C, 0x71, 0x86,
    0x61, 0x86, 0x61, 0x86, 0x63, 0x06, 0x63, 0x0C, 0x3E, 0x0C, 0x1C, 0x38, 0x00, 0xE0, 0x00, 0x00,
];
const EXPECTED_GLYPH_00AA: [u8; BYTES_PER_GLYPH] = [
    0x00, 0x00, 0x0C, 0x06, 0x0C, 0x12, 0x7F, 0xD8, 0x0C, 0x08, 0x0C, 0x00, 0x18, 0x00, 0x19, 0xF8,
    0x18, 0x00, 0x18, 0x00, 0x30, 0x00, 0x31, 0x80, 0x33, 0x00, 0x63, 0x00, 0x61, 0xFC, 0x00, 0x00,
];

const POC_GLYPHS: [(u16, char, &[u8; BYTES_PER_GLYPH]); 4] = [
    (0x002E, '몬', &EXPECTED_GLYPH_002E),
    (0x0032, '스', &EXPECTED_GLYPH_0032),
    (0x0028, '터', &EXPECTED_GLYPH_0028),
    (0x00AA, '다', &EXPECTED_GLYPH_00AA),
];

const M59_SYSTEM_POINTER_SPECS: [(&str, u16); 5] = [
    ("script_0045", 32),
    ("script_0046", 34),
    ("script_0047", 35),
    ("script_0048", 36),
    ("script_0049", 38),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UnconsumedSystemTextSpec {
    id: &'static str,
    legacy_fff8_idx: u64,
    pointer_index: u16,
    target_offset: usize,
}

const JP_SYSTEM_TEXT_POINTER_TABLE: usize = 0x09_E210;
const JP_SYSTEM_TEXT_POINTER_COUNT: usize = 40;
const JP_SYSTEM_TEXT_DISPLAY_ROUTINE: usize = 0x0000_459E;
const JP_SYSTEM_TEXT_DIRECT_READER: usize = 0x0003_9FE8;
const JP_SYSTEM_SELECTOR_ADDRESS: u16 = 0x84DE;
const JP_CODE_SCAN_END: usize = 0x09_E000;
const M58_EXPECTED_DISPLAY_CALL_COUNT: usize = 47;
const M58_EXPECTED_DIRECT_READER_CALL_COUNT: usize = 6;
const M58_DYNAMIC_SELECTOR_CALL: usize = 0x0000_42B6;
const M58_EN_UNCONSUMED_TAILS: [(u16, usize, usize); 2] =
    [(40, 0x09_E3BE, 0x09_E438), (41, 0x09_E438, 0x09_E4B2)];

const M58_UNCONSUMED_SYSTEM_TEXT_SPECS: [UnconsumedSystemTextSpec; 2] = [
    UnconsumedSystemTextSpec {
        id: "script_0043",
        legacy_fff8_idx: 33,
        pointer_index: 30,
        target_offset: 0x09_EB48,
    },
    UnconsumedSystemTextSpec {
        id: "script_0044",
        legacy_fff8_idx: 34,
        pointer_index: 31,
        target_offset: 0x09_EB74,
    },
];

const M58_SELECTOR_WRITES: [(usize, u16); 40] = [
    (0x0000_4230, 0x18),
    (0x0003_D2F6, 0x14),
    (0x0003_D368, 0x16),
    (0x0003_D74C, 0x0C),
    (0x0003_EE9E, 0x05),
    (0x0003_EFFE, 0x07),
    (0x0003_F074, 0x09),
    (0x0003_FB0E, 0x0B),
    (0x0003_FCFA, 0x0D),
    (0x0003_FFDA, 0x08),
    (0x0004_01C8, 0x13),
    (0x0004_0442, 0x0D),
    (0x0004_04B8, 0x0E),
    (0x0004_09E4, 0x0F),
    (0x0004_0A7C, 0x10),
    (0x0004_1672, 0x08),
    (0x0004_1714, 0x0A),
    (0x0004_2160, 0x08),
    (0x0004_2EDE, 0x0A),
    (0x0004_3088, 0x0B),
    (0x0004_34BC, 0x1A),
    (0x0004_352C, 0x0A),
    (0x0004_35A8, 0x0B),
    (0x0004_45FE, 0x18),
    (0x0004_4676, 0x15),
    (0x0004_46F2, 0x16),
    (0x0004_5038, 0x0C),
    (0x0004_50E4, 0x0D),
    (0x0004_5946, 0x08),
    (0x0004_5A94, 0x0F),
    (0x0004_5AFE, 0x09),
    (0x0004_5B7E, 0x0A),
    (0x0004_6332, 0x0F),
    (0x0004_6492, 0x0E),
    (0x0004_663E, 0x08),
    (0x0004_66BA, 0x09),
    (0x0004_6806, 0x0B),
    (0x0004_7E5A, 0x08),
    (0x0004_7ED0, 0x0A),
    (0x0004_8430, 0x09),
];

const M58_SELECTOR_READS: [usize; 7] = [
    0x0000_33B6,
    0x0000_42B2,
    0x0000_4B40,
    0x0000_7078,
    0x0000_812E,
    0x0000_8246,
    0x0003_F882,
];

#[derive(Clone, Copy)]
struct TextBuildScope {
    include_item_names: bool,
    include_item_quoted: bool,
    include_item_desc: bool,
    include_item_use: bool,
    include_item_use2: bool,
    extra_glyphs: &'static [char],
    milestone: &'static str,
}

/// Build the M0 JP-native baseline: expand the exact JP ROM to 4 MiB without
/// changing the text engine, scripts, font, or graphics.
pub fn build_raw(jp_rom_path: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;

    let mut baseline = expand_with_ff(&source)?;
    let mut writes = vec![ExpectedWrite {
        label: "Mega Drive ROM end address (4 MiB)".to_string(),
        offset: ROM_END_OFFSET,
        expected: vec![0x00, 0x1F, 0xFF, 0xFF],
        replacement: vec![0x00, 0x3F, 0xFF, 0xFF],
    }];

    validate_plan(&baseline, &writes)?;
    apply_plan(&mut baseline, &writes);
    let checksum = calculate_checksum(&baseline);
    writes.push(ExpectedWrite {
        label: format!("Mega Drive checksum -> 0x{checksum:04X}"),
        offset: CHECKSUM_OFFSET,
        expected: vec![0x91, 0xBF],
        replacement: checksum.to_be_bytes().to_vec(),
    });

    let expanded = expand_with_ff(&source)?;
    validate_plan(&expanded, &writes)?;
    let mut output = expanded.clone();
    apply_plan(&mut output, &writes);
    validate_result(&expanded, &output, &writes)?;
    validate_raw_expansion(&source, &output, &writes)?;

    eprintln!("JP-native M0 raw expansion Expected Writes:");
    eprintln!(
        "  source 0x{JP_ROM_SIZE:06X} bytes -> target 0x{JP_EXPANDED_ROM_SIZE:06X} bytes; appended region is 0xFF"
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

/// Build the M1 JP-native fixed-font consumer proof.
///
/// The output starts from the exact JP ROM, expands it to 4 MiB, installs all
/// Hangul glyphs used by the current Korean text assets as compact 1bpp glyphs,
/// and patches one mandatory opening line to exercise the new 0x8000 code
/// range. Full text relocation remains an M2 task.
pub fn build_font_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;

    let glyphs = collect_jp_native_glyphs(assets_dir)?;
    if glyphs.is_empty() {
        return Err("JP-native glyph set is empty".to_string());
    }
    if glyphs.len() > (0xFF00 - KR_CODE_START) as usize {
        return Err(format!(
            "JP-native glyph set is too large: {} entries",
            glyphs.len()
        ));
    }
    let charmap: BTreeMap<char, u16> = glyphs
        .iter()
        .enumerate()
        .map(|(index, &ch)| (ch, KR_CODE_START + index as u16))
        .collect();

    let ttf_data = fs::read(assets_dir.join("neodgm.ttf"))
        .map_err(|e| format!("failed to read Korean font: {e}"))?;
    let font = Font::from_bytes(ttf_data, FontSettings::default())
        .map_err(|e| format!("failed to parse Korean font: {e}"))?;
    let mut font_data = Vec::with_capacity(glyphs.len() * BYTES_PER_GLYPH);
    for &ch in &glyphs {
        font_data.extend_from_slice(&render_native_glyph(&font, ch));
    }

    let mut writes = vec![ExpectedWrite {
        label: "Mega Drive ROM end address (4 MiB)".to_string(),
        offset: ROM_END_OFFSET,
        expected: vec![0x00, 0x1F, 0xFF, 0xFF],
        replacement: vec![0x00, 0x3F, 0xFF, 0xFF],
    }];
    append_sram_safety_writes(&mut writes)?;
    let hook_point = assemble_m68k(&[Inst::JmpAbsoluteLong(JP_NATIVE_HOOK_CODE as u32)])?;
    let hook_code = assemble_jp_native_hook()?;
    writes.push(ExpectedWrite {
        label: "JP fixed-font renderer hook".to_string(),
        offset: JP_NATIVE_HOOK_POINT,
        expected: EXPECTED_HOOK_POINT.to_vec(),
        replacement: hook_point,
    });
    writes.push(ExpectedWrite {
        label: "JP-native Korean fixed-font dispatcher".to_string(),
        offset: JP_NATIVE_HOOK_CODE,
        expected: vec![0xFF; hook_code.len()],
        replacement: hook_code,
    });
    writes.push(ExpectedWrite {
        label: format!("JP-native 16x16 1bpp Korean font ({} glyphs)", glyphs.len()),
        offset: JP_NATIVE_FONT_BASE,
        expected: vec![0xFF; font_data.len()],
        replacement: font_data,
    });
    writes.push(ExpectedWrite {
        label: "mandatory opening-line M1 showcase: 한글직결!".to_string(),
        offset: SHOWCASE_SCRIPT_OFFSET,
        expected: EXPECTED_SHOWCASE_SCRIPT.to_vec(),
        replacement: encode_m1_showcase(&charmap)?.to_vec(),
    });

    let expanded = expand_with_ff(&source)?;
    validate_plan(&expanded, &writes)?;
    let mut checksum_stage = expanded.clone();
    apply_plan(&mut checksum_stage, &writes);
    let checksum = calculate_checksum(&checksum_stage);
    writes.push(ExpectedWrite {
        label: format!("Mega Drive checksum -> 0x{checksum:04X}"),
        offset: CHECKSUM_OFFSET,
        expected: vec![0x91, 0xBF],
        replacement: checksum.to_be_bytes().to_vec(),
    });

    validate_plan(&expanded, &writes)?;
    let mut output = expanded.clone();
    apply_plan(&mut output, &writes);
    validate_result(&expanded, &output, &writes)?;

    eprintln!("JP-native M1 fixed-font Expected Writes:");
    eprintln!(
        "  glyph codes 0x{KR_CODE_START:04X}..0x{:04X}: {} glyphs, {} bytes",
        KR_CODE_START + glyphs.len() as u16 - 1,
        glyphs.len(),
        glyphs.len() * BYTES_PER_GLYPH
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

fn build_text_poc_internal(
    jp_rom_path: &Path,
    assets_dir: &Path,
    text_specs: &[StableTextSpec],
    scope: TextBuildScope,
) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_dynamic_display_consumer_contract(&source)?;
    validate_dynamic_display_asset_ledger(&assets_dir.join("translation"))?;
    let redirect_signature = [
        0xFF,
        0xF8,
        (JP_TEXT_REDIRECT_MAGIC >> 8) as u8,
        JP_TEXT_REDIRECT_MAGIC as u8,
    ];
    if source
        .windows(redirect_signature.len())
        .any(|window| window == redirect_signature)
    {
        return Err("JP-native KR redirect signature already occurs in the JP source".to_string());
    }

    // Keep established glyph codes stable and append a dedicated blank glyph
    // last. The source renderer also uses 0x0000 for full-width page clearing,
    // so Korean half-width spaces must not reuse that ABI value.
    let glyphs =
        collect_scoped_jp_native_glyphs(assets_dir, scope.include_item_names, scope.extra_glyphs)?;
    if glyphs.is_empty() {
        return Err("JP-native glyph set is empty".to_string());
    }
    if glyphs.len() > (0xFF00 - KR_CODE_START) as usize {
        return Err(format!(
            "JP-native glyph set is too large: {} entries",
            glyphs.len()
        ));
    }
    let charmap: BTreeMap<char, u16> = glyphs
        .iter()
        .enumerate()
        .map(|(index, &ch)| (ch, KR_CODE_START + index as u16))
        .collect();
    let font_data = render_native_font(assets_dir, &glyphs)?;
    let item_name_layout = scope
        .include_item_names
        .then(|| build_m3_item_name_layout(&source, &assets_dir.join("translation"), &charmap))
        .transpose()?;
    let item_quoted_layout = scope
        .include_item_quoted
        .then(|| build_m5_item_quoted_layout(&source, &assets_dir.join("translation"), &charmap))
        .transpose()?;
    let item_desc_layout = scope
        .include_item_desc
        .then(|| build_m6_item_desc_layout(&source, &assets_dir.join("translation"), &charmap))
        .transpose()?;
    let item_use_layout = scope
        .include_item_use
        .then(|| build_m7_item_use_layout(&source, &assets_dir.join("translation"), &charmap))
        .transpose()?;
    let item_use2_layout = scope
        .include_item_use2
        .then(|| {
            let item_desc_layout = item_desc_layout
                .as_ref()
                .ok_or("M8 battle-use build requires the M6 item-description layout")?;
            build_m8_item_use2_layout(
                &source,
                &assets_dir.join("translation"),
                &charmap,
                item_desc_layout,
            )
        })
        .transpose()?;

    let entries = load_stable_text_entries(&assets_dir.join("translation"), text_specs)?;
    let text_payload_base = JP_TEXT_POINTER_TABLE + text_specs.len() * 4;
    let mut pointer_table = Vec::with_capacity(text_specs.len() * 4);
    let mut payload = Vec::new();
    let mut encoded_entries = Vec::with_capacity(entries.len());
    let mut layout_errors = Vec::new();
    let mut source_row_entries = 0usize;
    let mut source_rows_finalized = 0usize;
    let mut source_half_cells_cleared = 0usize;
    for (entry, spec) in entries.iter().zip(text_specs) {
        let (protected_jp, protected_ko) = compose_stable_fallthrough_text(entry, &entries)?;
        let source_words = validate_jp_text_source(
            &source,
            &entry.id,
            spec.offset,
            &protected_jp,
            scope.milestone,
        )?;
        let jp_tokens = crate::build::text::parse_display_text(&protected_jp);
        let ko_tokens = crate::build::text::parse_display_text(&protected_ko);
        let payload_tokens = match spec.mode {
            StableTextMode::PreserveLeadingPrelude
            | StableTextMode::PreserveLeadingPreludeProtected => {
                require_matching_control_sequence(&protected_jp, &protected_ko, &entry.id)?;
                let leading_controls = leading_control_count(&jp_tokens);
                if ko_tokens.get(..leading_controls) != jp_tokens.get(..leading_controls) {
                    return Err(format!(
                        "{}: JP/KR leading control prelude differs",
                        entry.id
                    ));
                }
                let leading_control_bytes = token_word_len(&jp_tokens[..leading_controls]) * 2;
                if spec.offset + leading_control_bytes != spec.redirect_offset {
                    return Err(format!(
                        "{}: control prelude ends at 0x{:06X}, not redirect offset 0x{:06X}",
                        entry.id,
                        spec.offset + leading_control_bytes,
                        spec.redirect_offset
                    ));
                }
                if spec.mode == StableTextMode::PreserveLeadingPreludeProtected {
                    normalize_m7_item_use_tokens_with_limit(
                        &entry.id,
                        &protected_jp,
                        &source_words[leading_control_bytes / 2..],
                        &ko_tokens[leading_controls..],
                        spec.max_glyphs_per_line,
                    )
                } else {
                    Ok(normalize_m2_ellipsis(&ko_tokens[leading_controls..]))
                }
            }
            StableTextMode::WholeEntryProtected => {
                if spec.redirect_offset != spec.offset {
                    return Err(format!(
                        "{}: whole-entry redirect does not start at its JP offset",
                        entry.id
                    ));
                }
                normalize_m7_item_use_tokens_with_limit(
                    &entry.id,
                    &protected_jp,
                    &source_words,
                    &ko_tokens,
                    spec.max_glyphs_per_line,
                )
            }
        };
        let mut payload_tokens = match payload_tokens {
            Ok(tokens) => tokens,
            Err(error) => {
                layout_errors.push(error);
                continue;
            }
        };
        let protected_prefix_len = match spec.mode {
            StableTextMode::PreserveLeadingPrelude
            | StableTextMode::PreserveLeadingPreludeProtected => leading_control_count(&jp_tokens),
            StableTextMode::WholeEntryProtected => 0,
        };
        localize_dynamic_trailing_tiles(&entry.id, &mut payload_tokens, &charmap)?;
        validate_dynamic_display_population(&entry.id, &payload_tokens)?;
        validate_map_label_source_footprint(&entry.id, &payload_tokens)?;
        if let Err(error) = validate_fixed_width_layout(
            &payload_tokens,
            &entry.id,
            spec.max_glyphs_per_line,
            spec.max_lines_per_page,
        ) {
            layout_errors.push(error);
            continue;
        }
        let mut footprint_tokens = jp_tokens[..protected_prefix_len].to_vec();
        footprint_tokens.extend(payload_tokens);
        let footprint_tokens = protect_source_overwrite_footprints(
            &entry.id,
            &source_words,
            &footprint_tokens,
            fixed_width_owned_row_half_cells(spec.max_glyphs_per_line),
        )?;
        payload_tokens = footprint_tokens[protected_prefix_len..].to_vec();
        let entry_finalizers = payload_tokens
            .iter()
            .filter_map(|token| match token {
                Token::SourceRowFinalize { clear_half_cells } => Some(*clear_half_cells),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !entry_finalizers.is_empty() {
            source_row_entries += 1;
            source_rows_finalized += entry_finalizers.len();
            source_half_cells_cleared += entry_finalizers.iter().sum::<usize>();
        }
        let words = encode_jp_native_tokens(&payload_tokens, &charmap)?;
        let terminator = *source_words
            .last()
            .ok_or_else(|| format!("{}: JP source text is empty", entry.id))?;
        if !matches!(terminator, 0xFF04 | 0xFF38 | 0xFFC0 | 0xFFFF)
            || words.last() != Some(&terminator)
        {
            return Err(format!(
                "{}: JP-native KR text does not preserve terminator 0x{terminator:04X}",
                entry.id
            ));
        }
        let encoded = words_to_bytes(&words);
        let address = text_payload_base
            .checked_add(payload.len())
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| format!("{}: JP-native payload address overflow", entry.id))?;
        pointer_table.extend_from_slice(&address.to_be_bytes());
        payload.extend_from_slice(&encoded);
        encoded_entries.push(encoded);
    }
    if !layout_errors.is_empty() {
        return Err(format!(
            "{} mixed-width layout audit found {} issue(s):\n{}",
            scope.milestone,
            layout_errors.len(),
            layout_errors.join("\n")
        ));
    }
    if pointer_table.len() != text_payload_base - JP_TEXT_POINTER_TABLE {
        return Err(format!(
            "{} pointer table layout no longer matches payload base",
            scope.milestone
        ));
    }

    let hook_point = assemble_m68k(&[Inst::JmpAbsoluteLong(JP_NATIVE_HOOK_CODE as u32)])?;
    let hook_code = assemble_jp_native_hook()?;
    let display_advance_point =
        assemble_m68k(&[Inst::JmpAbsoluteLong(JP_NATIVE_DISPLAY_ADVANCE_CODE as u32)])?;
    let display_advance_code = assemble_jp_native_display_advance(&charmap)?;
    let blank_advance_point = assemble_m68k(&[
        Inst::JmpAbsoluteLong(JP_NATIVE_BLANK_ADVANCE_CODE as u32),
        Inst::Nop,
        Inst::Nop,
    ])?;
    let blank_advance_code = assemble_jp_native_blank_advance()?;
    let half_space_code = *charmap
        .get(&' ')
        .ok_or("JP-native charmap is missing the dedicated half-space glyph")?;
    let ff78_fixed_slot_handler = assemble_ff78_fixed_slot_handler(half_space_code)?;
    let redirect_handler = assemble_jp_text_redirect_handler()?;
    let mut writes = vec![ExpectedWrite {
        label: "Mega Drive ROM end address (4 MiB)".to_string(),
        offset: ROM_END_OFFSET,
        expected: vec![0x00, 0x1F, 0xFF, 0xFF],
        replacement: vec![0x00, 0x3F, 0xFF, 0xFF],
    }];
    append_sram_safety_writes(&mut writes)?;
    writes.push(ExpectedWrite {
        label: "JP fixed-font renderer hook".to_string(),
        offset: JP_NATIVE_HOOK_POINT,
        expected: EXPECTED_HOOK_POINT.to_vec(),
        replacement: hook_point,
    });
    writes.push(ExpectedWrite {
        label: "JP-native Korean fixed-font dispatcher".to_string(),
        offset: JP_NATIVE_HOOK_CODE,
        expected: vec![0xFF; hook_code.len()],
        replacement: hook_code,
    });
    writes.push(ExpectedWrite {
        label: "JP fixed-font mixed-width display-advance hook".to_string(),
        offset: JP_NATIVE_DISPLAY_ADVANCE_POINT,
        expected: EXPECTED_DISPLAY_ADVANCE_POINT.to_vec(),
        replacement: display_advance_point,
    });
    writes.push(ExpectedWrite {
        label: "typed JP-native 8/16px display-advance dispatcher".to_string(),
        offset: JP_NATIVE_DISPLAY_ADVANCE_CODE,
        expected: vec![0xFF; display_advance_code.len()],
        replacement: display_advance_code,
    });
    writes.push(ExpectedWrite {
        label: "JP fixed-font zero-word compatibility hook".to_string(),
        offset: JP_NATIVE_BLANK_ADVANCE_POINT,
        expected: EXPECTED_BLANK_ADVANCE_POINT.to_vec(),
        replacement: blank_advance_point,
    });
    writes.push(ExpectedWrite {
        label: "typed JP-native zero-word full-width dispatcher".to_string(),
        offset: JP_NATIVE_BLANK_ADVANCE_CODE,
        expected: vec![0xFF; blank_advance_code.len()],
        replacement: blank_advance_code,
    });
    writes.push(ExpectedWrite {
        label: "JP FF78 fixed-buffer handler pointer".to_string(),
        offset: JP_FF78_HANDLER_SLOT,
        expected: (DynamicDisplayControl::SevenWords.handler() as u32)
            .to_be_bytes()
            .to_vec(),
        replacement: (JP_KR_FF78_FIXED_SLOT_HANDLER as u32)
            .to_be_bytes()
            .to_vec(),
    });
    writes.push(ExpectedWrite {
        label: "typed JP-native FF78 half-space slot compensation".to_string(),
        offset: JP_KR_FF78_FIXED_SLOT_HANDLER,
        expected: vec![0xFF; ff78_fixed_slot_handler.len()],
        replacement: ff78_fixed_slot_handler,
    });
    writes.push(ExpectedWrite {
        label: format!("JP-native 16x16 1bpp Korean font ({} glyphs)", glyphs.len()),
        offset: JP_NATIVE_FONT_BASE,
        expected: vec![0xFF; font_data.len()],
        replacement: font_data,
    });
    for (local_id, spec) in text_specs.iter().enumerate() {
        let local_id = u16::try_from(local_id)
            .map_err(|_| format!("{} local text ID exceeds 16-bit range", scope.milestone))?;
        let mut replacement = vec![
            0xFF,
            0xF8,
            (JP_TEXT_REDIRECT_MAGIC >> 8) as u8,
            JP_TEXT_REDIRECT_MAGIC as u8,
        ];
        replacement.extend_from_slice(&local_id.to_be_bytes());
        let expected = source
            .get(spec.redirect_offset..spec.redirect_offset + replacement.len())
            .ok_or_else(|| format!("{}: redirect source is truncated", spec.id))?;
        if let Some(expected_prefix) = spec.expected_prefix
            && expected != expected_prefix
        {
            return Err(format!("{}: redirect source prefix differs", spec.id));
        }
        writes.push(ExpectedWrite {
            label: format!(
                "{} JP entry -> {} local text {local_id}",
                spec.id, scope.milestone
            ),
            offset: spec.redirect_offset,
            expected: expected.to_vec(),
            replacement,
        });
    }
    writes.push(ExpectedWrite {
        label: "JP opcode FFF8 handler pointer".to_string(),
        offset: JP_TEXT_OPCODE_HANDLER_SLOT,
        expected: EXPECTED_FFF8_HANDLER_POINTER.to_vec(),
        replacement: (JP_TEXT_REDIRECT_HANDLER as u32).to_be_bytes().to_vec(),
    });
    writes.push(ExpectedWrite {
        label: "typed JP-native stable-text redirect handler".to_string(),
        offset: JP_TEXT_REDIRECT_HANDLER,
        expected: vec![0xFF; redirect_handler.len()],
        replacement: redirect_handler,
    });
    writes.push(ExpectedWrite {
        label: format!("{} stable-text pointer table", scope.milestone),
        offset: JP_TEXT_POINTER_TABLE,
        expected: vec![0xFF; pointer_table.len()],
        replacement: pointer_table,
    });
    writes.push(ExpectedWrite {
        label: format!("{} JP-source KR text payload", scope.milestone),
        offset: text_payload_base,
        expected: vec![0xFF; payload.len()],
        replacement: payload,
    });
    if let Some(layout) = &item_name_layout {
        writes.push(ExpectedWrite {
            label: "M3 JP item-name relative-pointer table".to_string(),
            offset: JP_ITEM_NAME_TABLE,
            expected: source[JP_ITEM_NAME_TABLE..JP_ITEM_NAME_TABLE + layout.table.len()].to_vec(),
            replacement: layout.table.clone(),
        });
        writes.push(ExpectedWrite {
            label: "M3 JP-source KR item-name payload".to_string(),
            offset: JP_ITEM_NAME_DATA_START,
            expected: source
                [JP_ITEM_NAME_DATA_START..JP_ITEM_NAME_DATA_START + layout.payload.len()]
                .to_vec(),
            replacement: layout.payload.clone(),
        });
    }
    if let Some(layout) = &item_quoted_layout {
        writes.push(ExpectedWrite {
            label: "M5 JP quoted-item relative-pointer table".to_string(),
            offset: JP_ITEM_QUOTED_TABLE,
            expected: source[JP_ITEM_QUOTED_TABLE..JP_ITEM_QUOTED_TABLE + layout.table.len()]
                .to_vec(),
            replacement: layout.table.clone(),
        });
        writes.push(ExpectedWrite {
            label: "M5 JP-source KR quoted-item payload".to_string(),
            offset: JP_ITEM_QUOTED_DATA_START,
            expected: source
                [JP_ITEM_QUOTED_DATA_START..JP_ITEM_QUOTED_DATA_START + layout.payload.len()]
                .to_vec(),
            replacement: layout.payload.clone(),
        });
    }
    if let Some(layout) = &item_desc_layout {
        writes.push(ExpectedWrite {
            label: "M6 JP item-description relative-pointer table".to_string(),
            offset: JP_ITEM_DESC_TABLE,
            expected: source[JP_ITEM_DESC_TABLE..JP_ITEM_DESC_TABLE + layout.table.len()].to_vec(),
            replacement: layout.table.clone(),
        });
        for write in &layout.writes {
            writes.push(ExpectedWrite {
                label: format!("M6 JP-source KR item description {}", write.id),
                offset: write.offset,
                expected: source[write.offset..write.offset + write.replacement.len()].to_vec(),
                replacement: write.replacement.clone(),
            });
        }
        if item_use2_layout.is_none() {
            writes.push(ExpectedWrite {
                label: "M6 item-description dependent battle-use pointer table".to_string(),
                offset: JP_ITEM_USE2_TABLE,
                expected: source[JP_ITEM_USE2_TABLE
                    ..JP_ITEM_USE2_TABLE + layout.dependent_item_use2_table.len()]
                    .to_vec(),
                replacement: layout.dependent_item_use2_table.clone(),
            });
        }
    }
    if let Some(layout) = &item_use_layout {
        writes.push(ExpectedWrite {
            label: "M7 JP ordinary item-use relative-pointer table".to_string(),
            offset: JP_ITEM_USE_TABLE,
            expected: source[JP_ITEM_USE_TABLE..JP_ITEM_USE_TABLE + layout.table.len()].to_vec(),
            replacement: layout.table.clone(),
        });
        for write in &layout.writes {
            writes.push(ExpectedWrite {
                label: format!("M7 JP-source KR ordinary item use {}", write.id),
                offset: write.offset,
                expected: source[write.offset..write.offset + write.replacement.len()].to_vec(),
                replacement: write.replacement.clone(),
            });
        }
    }
    if let Some(layout) = &item_use2_layout {
        writes.push(ExpectedWrite {
            label: "M8 JP battle item-use relative-pointer table".to_string(),
            offset: JP_ITEM_USE2_TABLE,
            expected: source[JP_ITEM_USE2_TABLE..JP_ITEM_USE2_TABLE + layout.table.len()].to_vec(),
            replacement: layout.table.clone(),
        });
        for write in &layout.writes {
            writes.push(ExpectedWrite {
                label: format!("M8 JP-source KR battle item use {}", write.id),
                offset: write.offset,
                expected: source[write.offset..write.offset + write.replacement.len()].to_vec(),
                replacement: write.replacement.clone(),
            });
        }
    }

    let expanded = expand_with_ff(&source)?;
    validate_plan(&expanded, &writes)?;
    let mut checksum_stage = expanded.clone();
    apply_plan(&mut checksum_stage, &writes);
    let checksum = calculate_checksum(&checksum_stage);
    writes.push(ExpectedWrite {
        label: format!("Mega Drive checksum -> 0x{checksum:04X}"),
        offset: CHECKSUM_OFFSET,
        expected: vec![0x91, 0xBF],
        replacement: checksum.to_be_bytes().to_vec(),
    });

    validate_plan(&expanded, &writes)?;
    let mut output = expanded.clone();
    apply_plan(&mut output, &writes);
    validate_result(&expanded, &output, &writes)?;
    validate_stable_text_layout(&output, text_specs, text_payload_base, &encoded_entries)?;
    validate_ff78_fixed_slot_handler(&output, half_space_code)?;
    if let Some(layout) = &item_name_layout {
        validate_m3_item_name_layout(&output, layout)?;
    }
    if let Some(layout) = &item_quoted_layout {
        validate_m5_item_quoted_layout(&output, layout)?;
    }
    if let Some(layout) = &item_desc_layout {
        validate_m6_item_desc_layout(&output, layout, item_use2_layout.is_none())?;
    }
    if let Some(layout) = &item_use_layout {
        validate_m7_item_use_layout(&output, layout)?;
    }
    if let Some(layout) = &item_use2_layout {
        validate_m8_item_use2_layout(&output, layout)?;
    }

    eprintln!("JP-native {} Expected Writes:", scope.milestone);
    eprintln!(
        "  stable entries: {}; glyph codes 0x{KR_CODE_START:04X}..0x{:04X}: {} glyphs",
        entries.len(),
        KR_CODE_START + glyphs.len() as u16 - 1,
        glyphs.len(),
    );
    eprintln!(
        "  source-row finalizers: {source_row_entries} entries / \
         {source_rows_finalized} rows; {source_half_cells_cleared} half cells cleared; \
         {source_rows_finalized} right borders restored",
    );
    if let Some(layout) = &item_name_layout {
        eprintln!(
            "  item names: {} unique / {} references; {} of {} source bytes used",
            layout.entries.len(),
            JP_ITEM_NAME_TABLE_COUNT,
            layout.payload.len(),
            JP_ITEM_NAME_DATA_END - JP_ITEM_NAME_DATA_START,
        );
    }
    if let Some(layout) = &item_quoted_layout {
        eprintln!(
            "  quoted item names: {} unique / {} references; {} of {} source bytes used",
            layout.entries.len(),
            JP_ITEM_QUOTED_TABLE_COUNT,
            layout.payload.len(),
            JP_ITEM_QUOTED_DATA_END - JP_ITEM_QUOTED_DATA_START,
        );
    }
    if let Some(layout) = &item_desc_layout {
        eprintln!(
            "  item descriptions: {} unique / {} references; {} of {} source-owned bytes used",
            layout.entries.len(),
            JP_ITEM_DESC_TABLE_COUNT,
            layout.payload_bytes,
            layout.source_bytes,
        );
    }
    if let Some(layout) = &item_use_layout {
        eprintln!(
            "  ordinary item uses: {} unique / {} references; {} of {} source-owned bytes used",
            layout.entries.len(),
            JP_ITEM_USE_TABLE_COUNT,
            layout.payload_bytes,
            layout.source_bytes,
        );
    }
    if let Some(layout) = &item_use2_layout {
        eprintln!(
            "  battle item uses: {} owned / {} references; {} of {} source-owned bytes used; shared target 0x{:06X}->0x{:06X}",
            layout.entries.len(),
            JP_ITEM_USE2_TABLE_COUNT,
            layout.payload_bytes,
            layout.source_bytes,
            layout.shared_old_target,
            layout.shared_new_target,
        );
    }
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

fn assemble_jp_native_hook() -> Result<Vec<u8>, String> {
    assemble_m68k(&[
        Inst::CmpiWordImmediate {
            immediate: KR_CODE_START,
            destination: DataReg::D0,
        },
        Inst::Branch {
            condition: BranchCondition::CarrySet,
            width: BranchWidth::Word,
            target: "old_font",
        },
        Inst::MoveByteImmediateToAbsoluteLong {
            immediate: 0,
            address: SRAM_REGISTER,
        },
        Inst::LeaAbsoluteLong {
            address: JP_NATIVE_FONT_BASE as u32,
            destination: AddressReg::A2,
        },
        Inst::SubiWordImmediate {
            immediate: KR_CODE_START,
            destination: DataReg::D0,
        },
        Inst::Branch {
            condition: BranchCondition::Always,
            width: BranchWidth::Word,
            target: "calculate_offset",
        },
        Inst::Label("old_font"),
        Inst::LeaAbsoluteLong {
            address: JP_KANA_FONT_BASE as u32,
            destination: AddressReg::A2,
        },
        Inst::CmpiWordImmediate {
            immediate: JP_HIGH_CODE_START,
            destination: DataReg::D0,
        },
        Inst::Branch {
            condition: BranchCondition::CarrySet,
            width: BranchWidth::Word,
            target: "calculate_offset",
        },
        Inst::LeaAbsoluteLong {
            address: JP_KANJI_FONT_BASE as u32,
            destination: AddressReg::A2,
        },
        Inst::SubiWordImmediate {
            immediate: JP_HIGH_CODE_START,
            destination: DataReg::D0,
        },
        Inst::Label("calculate_offset"),
        Inst::AslWordImmediate {
            count: 5,
            destination: DataReg::D0,
        },
        Inst::AddaWordData {
            source: DataReg::D0,
            destination: AddressReg::A2,
        },
        Inst::JmpAbsoluteLong(0x0004921C),
    ])
}

fn jp_native_half_width_code(charmap: &BTreeMap<char, u16>, ch: char) -> Result<u16, String> {
    let code = *charmap
        .get(&ch)
        .ok_or_else(|| format!("JP-native half-width glyph is missing: '{ch}'"))?;
    if code < KR_CODE_START {
        return Err(format!(
            "JP-native glyph '{ch}' has a non-KR code 0x{code:04X}"
        ));
    }
    Ok(code)
}

fn assemble_jp_native_display_advance(charmap: &BTreeMap<char, u16>) -> Result<Vec<u8>, String> {
    let space_code = jp_native_half_width_code(charmap, ' ')?;
    let tilde_code = jp_native_half_width_code(charmap, '~')?;
    let punctuation_codes = ['.', '?', '!', ',', '*']
        .map(|ch| jp_native_half_width_code(charmap, ch))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    if punctuation_codes
        .windows(2)
        .any(|pair| pair[1] != pair[0] + 1)
    {
        return Err("JP-native half-width punctuation codes are no longer contiguous".to_string());
    }
    let punctuation_start = punctuation_codes[0];
    let punctuation_end = punctuation_codes[punctuation_codes.len() - 1]
        .checked_add(1)
        .ok_or("JP-native punctuation code range overflow")?;

    assemble_m68k(&[
        // A1 is the glyph-code cache base loaded from $54(A0). D1 is the
        // pattern number after the renderer's net +3 byte-only tile writes.
        // Undo those byte operations before subtracting the pattern base so
        // low-byte wrap is inverted exactly.
        Inst::MoveLongDataToPredecrementAddress {
            source: DataReg::D0,
            destination: AddressReg::A7,
        },
        Inst::MoveWordDataToData {
            source: DataReg::D1,
            destination: DataReg::D0,
        },
        Inst::SubqByteImmediate {
            immediate: 3,
            destination: DataReg::D0,
        },
        Inst::SubWordDisplacementAddress {
            displacement: 0x0038,
            source: AddressReg::A0,
            destination: DataReg::D0,
        },
        Inst::LsrWordImmediate {
            count: 1,
            destination: DataReg::D0,
        },
        Inst::MoveWordIndexedAddressToData {
            displacement: 0,
            base: AddressReg::A1,
            index: DataReg::D0,
            destination: DataReg::D0,
        },
        Inst::CmpiWordImmediate {
            immediate: space_code,
            destination: DataReg::D0,
        },
        Inst::Branch {
            condition: BranchCondition::Equal,
            width: BranchWidth::Word,
            target: "durable_half_width_space",
        },
        Inst::CmpiWordImmediate {
            immediate: tilde_code,
            destination: DataReg::D0,
        },
        Inst::Branch {
            condition: BranchCondition::NotEqual,
            width: BranchWidth::Word,
            target: "check_punctuation",
        },
        Inst::Branch {
            condition: BranchCondition::Always,
            width: BranchWidth::Word,
            target: "half_width",
        },
        Inst::Label("check_punctuation"),
        Inst::CmpiWordImmediate {
            immediate: punctuation_start,
            destination: DataReg::D0,
        },
        Inst::Branch {
            condition: BranchCondition::CarrySet,
            width: BranchWidth::Word,
            target: "full_width",
        },
        Inst::CmpiWordImmediate {
            immediate: punctuation_end,
            destination: DataReg::D0,
        },
        Inst::Branch {
            condition: BranchCondition::CarrySet,
            width: BranchWidth::Word,
            target: "half_width",
        },
        Inst::Label("full_width"),
        Inst::MoveLongPostincrementAddressToData {
            source: AddressReg::A7,
            destination: DataReg::D0,
        },
        Inst::AddqWordImmediateToDisplacementAddress {
            immediate: 4,
            displacement: 0x004C,
            destination: AddressReg::A0,
        },
        Inst::Rts,
        Inst::Label("half_width"),
        Inst::MoveLongPostincrementAddressToData {
            source: AddressReg::A7,
            destination: DataReg::D0,
        },
        Inst::AddqWordImmediateToDisplacementAddress {
            immediate: 2,
            displacement: 0x004C,
            destination: AddressReg::A0,
        },
        Inst::Rts,
        Inst::Label("durable_half_width_space"),
        // A cached blank glyph is not a durable clear: when that cache slot is
        // reused, its old tilemap references display the replacement glyph.
        // Ordinary semantic spaces are followed by content, so replace both
        // cached columns on both rows with the source renderer's fixed blank
        // tile. The following glyph deliberately overlaps the second column.
        // A build-generated terminal half clear must preserve the following
        // border tile and therefore bypasses this glyph path through the
        // namespaced FFF8 half-clear operation.
        Inst::MoveLongDataToPredecrementAddress {
            source: DataReg::D5,
            destination: AddressReg::A7,
        },
        Inst::MoveWordDisplacementAddressToData {
            displacement: 0x004C,
            source: AddressReg::A0,
            destination: DataReg::D5,
        },
        Inst::OriWordImmediateToStatus { immediate: 0x0700 },
        Inst::JsrAbsoluteLong(0x0004_9992),
        Inst::MoveLongImmediateToAbsoluteLong {
            immediate: 0x8001_8001,
            address: 0x00C0_0000,
        },
        Inst::AndiWordImmediateToStatus { immediate: 0xF8FF },
        Inst::MoveWordAbsoluteLongToData {
            address: 0x00FF_8A64,
            destination: DataReg::D0,
        },
        Inst::AddWordData {
            source: DataReg::D0,
            destination: DataReg::D5,
        },
        Inst::OriWordImmediateToStatus { immediate: 0x0700 },
        Inst::JsrAbsoluteLong(0x0004_9992),
        Inst::MoveLongImmediateToAbsoluteLong {
            immediate: 0x8001_8001,
            address: 0x00C0_0000,
        },
        Inst::AndiWordImmediateToStatus { immediate: 0xF8FF },
        Inst::MoveLongPostincrementAddressToData {
            source: AddressReg::A7,
            destination: DataReg::D5,
        },
        Inst::MoveLongPostincrementAddressToData {
            source: AddressReg::A7,
            destination: DataReg::D0,
        },
        Inst::AddqWordImmediateToDisplacementAddress {
            immediate: 2,
            displacement: 0x004C,
            destination: AddressReg::A0,
        },
        Inst::Rts,
    ])
}

fn assemble_jp_native_blank_advance() -> Result<Vec<u8>, String> {
    assemble_m68k(&[
        // This replaces the second blank-tile write at 0x4930A, then performs
        // the original interrupt-mask restore. A zero word belongs to the
        // source renderer ABI and is also used by full-width page-clearing
        // scripts; Korean half-width spaces use a dedicated blank glyph.
        Inst::MoveLongImmediateToAbsoluteLong {
            immediate: 0x8001_8001,
            address: 0x00C0_0000,
        },
        Inst::AndiWordImmediateToStatus { immediate: 0xF8FF },
        Inst::AddqWordImmediateToDisplacementAddress {
            immediate: 4,
            displacement: 0x004C,
            destination: AddressReg::A0,
        },
        Inst::Rts,
    ])
}

fn assemble_jp_text_redirect_handler() -> Result<Vec<u8>, String> {
    assemble_m68k(&[
        Inst::MoveWordStatusToPredecrementAddress {
            destination: AddressReg::A7,
        },
        Inst::CmpiWordImmediateToAddressIndirect {
            immediate: JP_TEXT_ROW_FINALIZE_MAGIC,
            source: AddressReg::A1,
        },
        Inst::Branch {
            condition: BranchCondition::Equal,
            width: BranchWidth::Word,
            target: "row_finalize",
        },
        Inst::CmpiWordImmediateToAddressIndirect {
            immediate: JP_TEXT_REDIRECT_MAGIC,
            source: AddressReg::A1,
        },
        Inst::Branch {
            condition: BranchCondition::NotEqual,
            width: BranchWidth::Word,
            target: "original_fff8",
        },
        Inst::MoveWordPostincrementAddressToStatus {
            source: AddressReg::A7,
        },
        Inst::MoveWordPostincrementAddressToData {
            source: AddressReg::A1,
            destination: DataReg::D0,
        },
        Inst::Moveq {
            immediate: 0,
            destination: DataReg::D0,
        },
        Inst::MoveWordPostincrementAddressToData {
            source: AddressReg::A1,
            destination: DataReg::D0,
        },
        Inst::LslWordImmediate {
            count: 2,
            destination: DataReg::D0,
        },
        Inst::MoveAddressLongImmediate {
            address: JP_TEXT_POINTER_TABLE as u32,
            destination: AddressReg::A2,
        },
        Inst::MoveAddressLongIndexedWordToAddress {
            base: AddressReg::A2,
            index: DataReg::D0,
            destination: AddressReg::A1,
        },
        Inst::MoveAddressLongToDisplacementAddress {
            source: AddressReg::A1,
            displacement: 0x0040,
            destination: AddressReg::A0,
        },
        Inst::JmpAbsoluteLong(JP_TEXT_REDIRECT_RETURN),
        Inst::Label("row_finalize"),
        // Consume the private magic and half-cell count. Build-generated
        // row finalization bypasses the normal glyph/cache path entirely.
        // Each iteration clears one 8x8 cell on both text rows. The cursor
        // then points at the shared right-border cell, which is restored
        // explicitly without advancing into it.
        Inst::MoveLongDataToPredecrementAddress {
            source: DataReg::D0,
            destination: AddressReg::A7,
        },
        Inst::MoveLongDataToPredecrementAddress {
            source: DataReg::D5,
            destination: AddressReg::A7,
        },
        Inst::MoveLongDataToPredecrementAddress {
            source: DataReg::D6,
            destination: AddressReg::A7,
        },
        Inst::MoveWordPostincrementAddressToData {
            source: AddressReg::A1,
            destination: DataReg::D0,
        },
        Inst::MoveWordPostincrementAddressToData {
            source: AddressReg::A1,
            destination: DataReg::D6,
        },
        // The interpreter reloads its stream pointer from $40(A0) after the
        // FFF8 handler returns. Consuming only the local A1 would make the
        // private magic and count visible again as two ordinary 16px glyphs.
        Inst::MoveAddressLongToDisplacementAddress {
            source: AddressReg::A1,
            displacement: 0x0040,
            destination: AddressReg::A0,
        },
        Inst::OriWordImmediateToStatus { immediate: 0x0700 },
        Inst::CmpiWordImmediate {
            immediate: 0,
            destination: DataReg::D6,
        },
        Inst::Branch {
            condition: BranchCondition::Equal,
            width: BranchWidth::Word,
            target: "row_restore_border",
        },
        Inst::Label("row_clear_loop"),
        Inst::MoveWordDisplacementAddressToData {
            displacement: 0x004C,
            source: AddressReg::A0,
            destination: DataReg::D5,
        },
        Inst::JsrAbsoluteLong(0x0004_9992),
        Inst::MoveWordImmediateToAbsoluteLong {
            immediate: 0x8001,
            address: 0x00C0_0000,
        },
        Inst::MoveWordAbsoluteLongToData {
            address: 0x00FF_8A64,
            destination: DataReg::D0,
        },
        Inst::AddWordData {
            source: DataReg::D0,
            destination: DataReg::D5,
        },
        Inst::JsrAbsoluteLong(0x0004_9992),
        Inst::MoveWordImmediateToAbsoluteLong {
            immediate: 0x8001,
            address: 0x00C0_0000,
        },
        Inst::AddqWordImmediateToDisplacementAddress {
            immediate: 2,
            displacement: 0x004C,
            destination: AddressReg::A0,
        },
        Inst::SubiWordImmediate {
            immediate: 1,
            destination: DataReg::D6,
        },
        Inst::Branch {
            condition: BranchCondition::NotEqual,
            width: BranchWidth::Word,
            target: "row_clear_loop",
        },
        Inst::Label("row_restore_border"),
        Inst::MoveWordDisplacementAddressToData {
            displacement: 0x004C,
            source: AddressReg::A0,
            destination: DataReg::D5,
        },
        Inst::JsrAbsoluteLong(0x0004_9992),
        Inst::MoveWordImmediateToAbsoluteLong {
            immediate: 0x8833,
            address: 0x00C0_0000,
        },
        Inst::MoveWordAbsoluteLongToData {
            address: 0x00FF_8A64,
            destination: DataReg::D0,
        },
        Inst::AddWordData {
            source: DataReg::D0,
            destination: DataReg::D5,
        },
        Inst::JsrAbsoluteLong(0x0004_9992),
        Inst::MoveWordImmediateToAbsoluteLong {
            immediate: 0x8833,
            address: 0x00C0_0000,
        },
        Inst::Label("row_finalize_done"),
        Inst::MoveLongPostincrementAddressToData {
            source: AddressReg::A7,
            destination: DataReg::D6,
        },
        Inst::MoveLongPostincrementAddressToData {
            source: AddressReg::A7,
            destination: DataReg::D5,
        },
        Inst::MoveLongPostincrementAddressToData {
            source: AddressReg::A7,
            destination: DataReg::D0,
        },
        Inst::MoveWordPostincrementAddressToStatus {
            source: AddressReg::A7,
        },
        Inst::JmpAbsoluteLong(JP_TEXT_REDIRECT_RETURN),
        Inst::Label("original_fff8"),
        Inst::MoveWordPostincrementAddressToStatus {
            source: AddressReg::A7,
        },
        Inst::JmpAbsoluteLong(JP_ORIGINAL_FFF8_HANDLER),
    ])
}

fn assemble_sram_wrappers() -> Result<Vec<u8>, String> {
    assemble_m68k(&[
        Inst::MoveByteImmediateToAbsoluteLong {
            immediate: 1,
            address: SRAM_REGISTER,
        },
        Inst::MoveByteAbsoluteLongToData {
            address: SRAM_LATEST_SLOT,
            destination: DataReg::D0,
        },
        Inst::MoveByteImmediateToAbsoluteLong {
            immediate: 0,
            address: SRAM_REGISTER,
        },
        Inst::Rts,
        Inst::MoveByteImmediateToAbsoluteLong {
            immediate: 1,
            address: SRAM_REGISTER,
        },
        Inst::MoveWordAbsoluteLongToData {
            address: SRAM_PRESENT_UNLOCKED,
            destination: DataReg::D0,
        },
        Inst::MoveByteImmediateToAbsoluteLong {
            immediate: 0,
            address: SRAM_REGISTER,
        },
        Inst::Rts,
        Inst::MoveByteImmediateToAbsoluteLong {
            immediate: 1,
            address: SRAM_REGISTER,
        },
        Inst::MoveWordImmediateToAbsoluteLong {
            immediate: 1,
            address: SRAM_PRESENT_UNLOCKED,
        },
        Inst::MoveByteImmediateToAbsoluteLong {
            immediate: 0,
            address: SRAM_REGISTER,
        },
        Inst::Rts,
    ])
}

fn append_instruction_write(
    writes: &mut Vec<ExpectedWrite>,
    offset: usize,
    expected: &[u8],
    program: &[Inst],
    label: &str,
) -> Result<(), String> {
    writes.push(ExpectedWrite {
        label: label.to_string(),
        offset,
        expected: expected.to_vec(),
        replacement: assemble_m68k(program)?,
    });
    Ok(())
}

fn append_sram_safety_writes(writes: &mut Vec<ExpectedWrite>) -> Result<(), String> {
    let wrappers = assemble_sram_wrappers()?;
    writes.push(ExpectedWrite {
        label: "SRAM read/write wrappers below 2 MiB".to_string(),
        offset: SRAM_WRAPPER_OFFSET,
        expected: vec![0xFF; wrappers.len()],
        replacement: wrappers,
    });

    append_instruction_write(
        writes,
        0x00370C,
        &[0x10, 0x39, 0x00, 0x20, 0x10, 0x29],
        &[Inst::JsrAbsoluteLong(0x0014FEB0)],
        "save prompt latest-slot SRAM read",
    )?;
    append_instruction_write(
        writes,
        0x091F4C,
        &[0x30, 0x39, 0x00, 0x20, 0x10, 0x2A],
        &[Inst::JsrAbsoluteLong(0x0014FEC8)],
        "title present-unlock SRAM read",
    )?;
    append_instruction_write(
        writes,
        0x0920F0,
        &[0x10, 0x39, 0x00, 0x20, 0x10, 0x29],
        &[Inst::JsrAbsoluteLong(0x0014FEB0)],
        "title latest-slot SRAM read",
    )?;
    let unlock_program = [Inst::JsrAbsoluteLong(0x0014FEE0), Inst::Nop];
    append_instruction_write(
        writes,
        0x093E02,
        &[0x33, 0xFC, 0x00, 0x01, 0x00, 0x20, 0x10, 0x2A],
        &unlock_program,
        "perfect-ending SRAM unlock write A",
    )?;
    append_instruction_write(
        writes,
        0x093E86,
        &[0x33, 0xFC, 0x00, 0x01, 0x00, 0x20, 0x10, 0x2A],
        &unlock_program,
        "perfect-ending SRAM unlock write B",
    )?;

    for &offset in &SRAM_DISABLE_OFFSETS {
        append_instruction_write(
            writes,
            offset,
            &EXPECTED_SRAM_ENABLE_WRITE,
            &[Inst::MoveByteImmediateToAbsoluteLong {
                immediate: 0,
                address: SRAM_REGISTER,
            }],
            &format!("disable SRAM after save operation at 0x{offset:06X}"),
        )?;
    }
    Ok(())
}

/// Build the diagnostic JP-native PoC ROM.
pub fn build_poc(jp_rom_path: &Path, font_path: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;

    expect_bytes(
        &source,
        ENCOUNTER_SCRIPT_OFFSET,
        &EXPECTED_ENCOUNTER_SCRIPT,
        "encounter script",
    )?;

    let ttf_data = fs::read(font_path).map_err(|e| format!("failed to read Korean font: {e}"))?;
    let font = Font::from_bytes(ttf_data, FontSettings::default())
        .map_err(|e| format!("failed to parse Korean font: {e}"))?;

    let mut writes = Vec::with_capacity(6);
    for (code, ch, expected) in POC_GLYPHS {
        writes.push(ExpectedWrite {
            label: format!("JP glyph 0x{code:04X} -> {ch}"),
            offset: jp_font_offset(code)?,
            expected: expected.to_vec(),
            replacement: render_native_glyph(&font, ch).to_vec(),
        });
    }
    writes.push(ExpectedWrite {
        label: "mandatory opening-line runtime showcase".to_string(),
        offset: SHOWCASE_SCRIPT_OFFSET,
        expected: EXPECTED_SHOWCASE_SCRIPT.to_vec(),
        replacement: REPLACEMENT_SHOWCASE_SCRIPT.to_vec(),
    });

    // The checksum depends on the planned glyph writes, but is itself kept in
    // the same fail-closed Expected Writes plan.
    let mut checksum_stage = source.clone();
    validate_plan(&source, &writes)?;
    apply_plan(&mut checksum_stage, &writes);
    let checksum = calculate_checksum(&checksum_stage);
    writes.push(ExpectedWrite {
        label: format!("Mega Drive checksum -> 0x{checksum:04X}"),
        offset: CHECKSUM_OFFSET,
        expected: vec![0x91, 0xBF],
        replacement: checksum.to_be_bytes().to_vec(),
    });

    validate_plan(&source, &writes)?;
    let mut output = source.clone();
    apply_plan(&mut output, &writes);
    validate_result(&source, &output, &writes)?;

    eprintln!("JP-native diagnostic PoC Expected Writes:");
    for write in &writes {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {} ({} bytes)",
            write.offset,
            write.offset + write.replacement.len(),
            write.label,
            write.replacement.len(),
        );
    }
    eprintln!(
        "  encounter source script remains byte-identical at 0x{ENCOUNTER_SCRIPT_OFFSET:06X}"
    );

    Ok(output)
}

fn validate_jp_source(data: &[u8]) -> Result<(), String> {
    if data.len() != JP_ROM_SIZE {
        return Err(format!(
            "JP ROM size mismatch: expected 0x{JP_ROM_SIZE:X}, got 0x{:X}",
            data.len()
        ));
    }
    let header = data
        .get(HEADER_OFFSET..HEADER_OFFSET + 16)
        .ok_or("JP ROM header is missing")?;
    if !header.starts_with(b"SEGA MEGA DRIVE") {
        return Err("input is not the expected Mega Drive ROM".to_string());
    }

    let digest = Sha256::digest(data);
    if digest.as_slice() != JP_SHA256 {
        return Err(format!(
            "JP ROM SHA-256 mismatch: expected {}, got {}",
            bytes_to_hex(&JP_SHA256),
            bytes_to_hex(digest.as_slice()),
        ));
    }
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}
