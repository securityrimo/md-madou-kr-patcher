//! M2-M16 base, item, and early battle milestones.

use super::super::*;

/// Build the M2 JP-native stable-text redirect proof.
///
/// Runtime-proven script entries are selected by stable translation ID, their
/// original JP control structures are checked against the KR text, and the
/// first translatable word after each in-place control prelude is replaced with
/// a namespaced `FFF8 + magic + local ID` redirect. The redirect handler is
/// emitted only through the typed 68000 ISA builder.
pub fn build_text_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &M2_TEXT_SPECS,
        TextBuildScope {
            include_item_names: false,
            include_item_quoted: false,
            include_item_desc: false,
            include_item_use: false,
            include_item_use2: false,
            extra_glyphs: &[],
            milestone: "M2",
        },
    )
}

/// Build the M3 JP-native item-name and dynamic-buffer proof.
///
/// M3 keeps the original JP item-name loader and seven-word FF78 consumer. It
/// repacks the 40 translated item names inside the original relative-pointer
/// block and rewrites all 52 references while preserving duplicate targets.
pub fn build_menu_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &M2_TEXT_SPECS,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: false,
            include_item_desc: false,
            include_item_use: false,
            include_item_use2: false,
            extra_glyphs: &[],
            milestone: "M3",
        },
    )
}

/// Build the M4 JP-native opening-dialog proof.
///
/// M4 retains M3's item-name path and adds `script_1106`, whose chapter-table
/// caller and first body-word reads have both been observed at runtime.
pub fn build_dialog_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &M4_TEXT_SPECS,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: false,
            include_item_desc: false,
            include_item_use: false,
            include_item_use2: false,
            extra_glyphs: &[],
            milestone: "M4",
        },
    )
}

/// Build the M5 JP-native quoted-item text proof.
///
/// M5 retains M4 and repacks the 40 translated quoted item strings inside the
/// original JP relative-pointer block. Quote and layout tokens are translated
/// only for this proven consumer surface.
pub fn build_item_quoted_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &M4_TEXT_SPECS,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: false,
            include_item_use: false,
            include_item_use2: false,
            extra_glyphs: &[],
            milestone: "M5",
        },
    )
}

/// Build the M6 JP-native item-description proof.
///
/// M6 retains M5 and repacks the 38 translated item descriptions across only
/// their original source-owned slots, then rebuilds the 39-reference rel16
/// table. The interleaved use and battle-use strings are not touched.
pub fn build_item_desc_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &M4_TEXT_SPECS,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: false,
            include_item_use2: false,
            extra_glyphs: &[],
            milestone: "M6",
        },
    )
}

/// Build the M7 JP-native ordinary item-use proof.
///
/// M7 retains M6, repacks all 51 ordinary item-use messages across only their
/// original source-owned slots, and rebuilds their rel16 table. EN-specific
/// control operands are restored from the protected JP source before encoding.
pub fn build_item_use_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &M4_TEXT_SPECS,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: false,
            extra_glyphs: &[],
            milestone: "M7",
        },
    )
}

/// Build the M8 JP-native battle item-use proof.
///
/// M8 retains M7, repacks the 44 battle-use-owned messages, preserves the one
/// description target shared by the 45-entry table, and makes the source
/// `script_0400` -> `script_0401` fallthrough explicit in the encoded message.
pub fn build_item_use2_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &M4_TEXT_SPECS,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &[],
            milestone: "M8",
        },
    )
}

/// Build the M9 JP-native recovery-result text proof.
///
/// M9 retains M8 and promotes the 24 HP, MP, and combined recovery messages
/// that the JP-to-EN patch redirected at their original JP entry starts.
pub fn build_recovery_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &[],
            milestone: "M9",
        },
    )
}

/// Build the M10 JP-native health-status text proof.
///
/// M10 retains M9 and promotes all 16 health-status messages. The EN `{hp:N}`
/// presentation tokens are mapped back to the exact JP face tiles protected by
/// each source entry.
pub fn build_health_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &[],
            milestone: "M10",
        },
    )
}

/// Build the M11 JP-native remaining-MP status proof.
///
/// M11 retains M10 and promotes all eight remaining-MP messages, using the
/// same exact-source face-tile protection established for health status.
pub fn build_mp_remaining_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &[],
            milestone: "M11",
        },
    )
}

/// Build the M12 JP-native Puyo battle-action proof.
///
/// M12 retains M11 and promotes the visible messages in the legacy FFF8
/// population 496 through 525. Auto-matched physical fragments without a
/// legacy consumer and the two four-byte empty messages are deliberately
/// excluded.
pub fn build_enemy_action_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &[],
            milestone: "M12",
        },
    )
}

/// Build the M13 JP-native enemy action and reaction proof.
pub fn build_enemy_reaction_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &['T'],
            milestone: "M13",
        },
    )
}

/// Build the M14 JP-native second enemy battle-text batch.
pub fn build_enemy_battle2_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &['T'],
            milestone: "M14",
        },
    )
}

/// Build the M15 JP-native third enemy battle-text batch.
pub fn build_enemy_battle3_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &['T'],
            milestone: "M15",
        },
    )
}

/// Build the M16 JP-native fourth and final contiguous enemy battle-text batch.
pub fn build_enemy_battle4_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &['T'],
            milestone: "M16",
        },
    )
}
