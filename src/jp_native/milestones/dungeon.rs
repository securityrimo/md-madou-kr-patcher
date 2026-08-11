//! M17-M42 dungeon-event milestones and shared-tail admission checks.

use super::super::*;

/// Build the M17 JP-native first dungeon-event batch after the enemy texts.
pub fn build_dungeon_event1_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
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
            milestone: "M17",
        },
    )
}

/// Build the M18 JP-native first dungeon choice and button-event batch.
pub fn build_dungeon_choice1_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
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
            milestone: "M18",
        },
    )
}

/// Build the M19 JP-native second dungeon-event batch.
pub fn build_dungeon_event2_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
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
            milestone: "M19",
        },
    )
}

/// Build the M20 JP-native third dungeon-event batch.
pub fn build_dungeon_event3_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
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
            milestone: "M20",
        },
    )
}

/// Build the M21 JP-native fourth dungeon-event batch.
pub fn build_dungeon_event4_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
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
            milestone: "M21",
        },
    )
}

/// Build the M22 JP-native fifth dungeon-event batch.
pub fn build_dungeon_event5_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
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
            milestone: "M22",
        },
    )
}

/// Build the M23 JP-native sixth dungeon-event batch.
pub fn build_dungeon_event6_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
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
            milestone: "M23",
        },
    )
}

/// Build the M24 JP-native seventh dungeon-event batch.
pub fn build_dungeon_event7_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
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
            milestone: "M24",
        },
    )
}

/// Build the M25 JP-native eighth dungeon-event batch.
pub fn build_dungeon_event8_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
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
            milestone: "M25",
        },
    )
}

/// Build the M26 JP-native ninth dungeon-event batch.
pub fn build_dungeon_event9_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
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
            milestone: "M26",
        },
    )
}

/// Build the M27 JP-native tenth dungeon-event batch.
pub fn build_dungeon_event10_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
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
            milestone: "M27",
        },
    )
}

/// Build the M28 JP-native eleventh dungeon-event batch.
pub fn build_dungeon_event11_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
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
            milestone: "M28",
        },
    )
}

/// Build the M29 JP-native twelfth dungeon-event batch.
pub fn build_dungeon_event12_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
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
            milestone: "M29",
        },
    )
}

/// Build the M30 JP-native thirteenth dungeon-event batch.
pub fn build_dungeon_event13_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
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
            milestone: "M30",
        },
    )
}

/// Build the M31 JP-native fourteenth dungeon-event batch.
pub fn build_dungeon_event14_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
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
            milestone: "M31",
        },
    )
}

/// Build the M32 JP-native fifteenth dungeon-event batch.
pub fn build_dungeon_event15_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
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
            milestone: "M32",
        },
    )
}

/// Build the M33 JP-native sixteenth dungeon-event batch.
pub fn build_dungeon_event16_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
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
            milestone: "M33",
        },
    )
}

/// Build the M34 JP-native seventeenth dungeon-event batch.
pub fn build_dungeon_event17_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
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
            milestone: "M34",
        },
    )
}

/// Build the M35 JP-native eighteenth dungeon-event batch.
pub fn build_dungeon_event18_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
    text_specs.extend_from_slice(&M35_DUNGEON_EVENT18_TEXT_SPECS);
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
            milestone: "M35",
        },
    )
}

/// Build the M36 JP-native nineteenth dungeon-event batch.
pub fn build_dungeon_event19_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
    text_specs.extend_from_slice(&M35_DUNGEON_EVENT18_TEXT_SPECS);
    text_specs.extend_from_slice(&M36_DUNGEON_EVENT19_TEXT_SPECS);
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
            milestone: "M36",
        },
    )
}

/// Build the M37 JP-native twentieth dungeon-event batch.
pub fn build_dungeon_event20_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
    text_specs.extend_from_slice(&M35_DUNGEON_EVENT18_TEXT_SPECS);
    text_specs.extend_from_slice(&M36_DUNGEON_EVENT19_TEXT_SPECS);
    text_specs.extend_from_slice(&M37_DUNGEON_EVENT20_TEXT_SPECS);
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
            milestone: "M37",
        },
    )
}

/// Build the M38 JP-native twenty-first dungeon-event batch.
pub fn build_dungeon_event21_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
    text_specs.extend_from_slice(&M35_DUNGEON_EVENT18_TEXT_SPECS);
    text_specs.extend_from_slice(&M36_DUNGEON_EVENT19_TEXT_SPECS);
    text_specs.extend_from_slice(&M37_DUNGEON_EVENT20_TEXT_SPECS);
    text_specs.extend_from_slice(&M38_DUNGEON_EVENT21_TEXT_SPECS);
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
            milestone: "M38",
        },
    )
}

/// Build the M39 JP-native twenty-second dungeon-event batch.
pub fn build_dungeon_event22_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
    text_specs.extend_from_slice(&M35_DUNGEON_EVENT18_TEXT_SPECS);
    text_specs.extend_from_slice(&M36_DUNGEON_EVENT19_TEXT_SPECS);
    text_specs.extend_from_slice(&M37_DUNGEON_EVENT20_TEXT_SPECS);
    text_specs.extend_from_slice(&M38_DUNGEON_EVENT21_TEXT_SPECS);
    text_specs.extend_from_slice(&M39_DUNGEON_EVENT22_TEXT_SPECS);
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
            milestone: "M39",
        },
    )
}

/// Build the M40 JP-native twenty-third dungeon-event batch.
pub fn build_dungeon_event23_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
    text_specs.extend_from_slice(&M35_DUNGEON_EVENT18_TEXT_SPECS);
    text_specs.extend_from_slice(&M36_DUNGEON_EVENT19_TEXT_SPECS);
    text_specs.extend_from_slice(&M37_DUNGEON_EVENT20_TEXT_SPECS);
    text_specs.extend_from_slice(&M38_DUNGEON_EVENT21_TEXT_SPECS);
    text_specs.extend_from_slice(&M39_DUNGEON_EVENT22_TEXT_SPECS);
    text_specs.extend_from_slice(&M40_DUNGEON_EVENT23_TEXT_SPECS);
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
            milestone: "M40",
        },
    )
}

pub(crate) fn validate_m40_spicy_curry_shared_tail(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<(), String> {
    let shared_tail_spec = M40_DUNGEON_EVENT23_TEXT_SPECS[9];
    if M40_SPICY_CURRY_ALIAS_SPEC.offset + 4 != shared_tail_spec.offset {
        return Err(
            "M40 spicy-curry alias no longer reaches the shared tail after FFAC".to_string(),
        );
    }

    let specs = [M40_SPICY_CURRY_ALIAS_SPEC, shared_tail_spec];
    let entries = load_stable_text_entries(&assets_dir.join("translation"), &specs)?;
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    let alias_words = validate_jp_text_source(
        &source,
        entries[0].id.as_str(),
        specs[0].offset,
        &entries[0].jp,
        "M40 shared-prefix alias",
    )?;
    let tail_words = validate_jp_text_source(
        &source,
        entries[1].id.as_str(),
        specs[1].offset,
        &entries[1].jp,
        "M40 shared reward tail",
    )?;
    if alias_words.get(2..) != Some(tail_words.as_slice()) {
        return Err("M40 spicy-curry source alias does not share the script_1153 tail".to_string());
    }

    let alias_tokens = normalize_m7_item_use_tokens(
        &entries[0].id,
        &entries[0].jp,
        &alias_words,
        &crate::build::text::parse_display_text(&entries[0].ko),
    )?;
    let tail_tokens = normalize_m7_item_use_tokens(
        &entries[1].id,
        &entries[1].jp,
        &tail_words,
        &crate::build::text::parse_display_text(&entries[1].ko),
    )?;
    if !matches!(alias_tokens.first(), Some(Token::CtrlParam(0xFFAC, _)))
        || alias_tokens.get(1..) != Some(tail_tokens.as_slice())
    {
        return Err("M40 spicy-curry KR alias does not share the script_1153 tail".to_string());
    }
    Ok(())
}

/// Build the M41 JP-native twenty-fourth dungeon-event batch.
pub fn build_dungeon_event24_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
    text_specs.extend_from_slice(&M35_DUNGEON_EVENT18_TEXT_SPECS);
    text_specs.extend_from_slice(&M36_DUNGEON_EVENT19_TEXT_SPECS);
    text_specs.extend_from_slice(&M37_DUNGEON_EVENT20_TEXT_SPECS);
    text_specs.extend_from_slice(&M38_DUNGEON_EVENT21_TEXT_SPECS);
    text_specs.extend_from_slice(&M39_DUNGEON_EVENT22_TEXT_SPECS);
    text_specs.extend_from_slice(&M40_DUNGEON_EVENT23_TEXT_SPECS);
    text_specs.extend_from_slice(&M41_DUNGEON_EVENT24_TEXT_SPECS);
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
            milestone: "M41",
        },
    )
}

pub(crate) fn validate_m42_nested_event_tails(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<(), String> {
    let specs = &M42_DUNGEON_EVENT25_TEXT_SPECS[9..=11];
    let entries = load_stable_text_entries(&assets_dir.join("translation"), specs)?;
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    let mut source_words = Vec::with_capacity(specs.len());
    let mut kr_tokens = Vec::with_capacity(specs.len());
    for (entry, spec) in entries.iter().zip(specs) {
        let words = validate_jp_text_source(
            &source,
            &entry.id,
            spec.offset,
            &entry.jp,
            "M42 nested event tail",
        )?;
        let tokens = normalize_m7_item_use_tokens(
            &entry.id,
            &entry.jp,
            &words,
            &crate::build::text::parse_display_text(&entry.ko),
        )?;
        source_words.push(words);
        kr_tokens.push(tokens);
    }

    for parent_index in 0..2 {
        let parent_spec = specs[parent_index];
        let child_spec = specs[parent_index + 1];
        let source_delta = child_spec
            .offset
            .checked_sub(parent_spec.offset)
            .ok_or("M42 nested event offsets are no longer ascending")?;
        if !source_delta.is_multiple_of(2)
            || source_words[parent_index].get(source_delta / 2..)
                != Some(source_words[parent_index + 1].as_slice())
        {
            return Err(format!(
                "M42 JP {} no longer shares the {} tail",
                parent_spec.id, child_spec.id
            ));
        }
        if !kr_tokens[parent_index].ends_with(&kr_tokens[parent_index + 1]) {
            return Err(format!(
                "M42 KR {} no longer shares the {} tail",
                parent_spec.id, child_spec.id
            ));
        }
    }
    Ok(())
}

/// Build the M42 JP-native twenty-fifth dungeon-event batch.
pub fn build_dungeon_event25_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    let mut text_specs = M4_TEXT_SPECS.to_vec();
    text_specs.extend_from_slice(&M9_RECOVERY_TEXT_SPECS);
    text_specs.extend_from_slice(&M10_HEALTH_TEXT_SPECS);
    text_specs.extend_from_slice(&M11_MP_REMAINING_TEXT_SPECS);
    text_specs.extend_from_slice(&M12_ENEMY_ACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M13_ENEMY_REACTION_TEXT_SPECS);
    text_specs.extend_from_slice(&M14_ENEMY_BATTLE2_TEXT_SPECS);
    text_specs.extend_from_slice(&M15_ENEMY_BATTLE3_TEXT_SPECS);
    text_specs.extend_from_slice(&M16_ENEMY_BATTLE4_TEXT_SPECS);
    text_specs.extend_from_slice(&M17_DUNGEON_EVENT1_TEXT_SPECS);
    text_specs.extend_from_slice(&M18_DUNGEON_CHOICE1_TEXT_SPECS);
    text_specs.extend_from_slice(&M19_DUNGEON_EVENT2_TEXT_SPECS);
    text_specs.extend_from_slice(&M20_DUNGEON_EVENT3_TEXT_SPECS);
    text_specs.extend_from_slice(&M21_DUNGEON_EVENT4_TEXT_SPECS);
    text_specs.extend_from_slice(&M22_DUNGEON_EVENT5_TEXT_SPECS);
    text_specs.extend_from_slice(&M23_DUNGEON_EVENT6_TEXT_SPECS);
    text_specs.extend_from_slice(&M24_DUNGEON_EVENT7_TEXT_SPECS);
    text_specs.extend_from_slice(&M25_DUNGEON_EVENT8_TEXT_SPECS);
    text_specs.extend_from_slice(&M26_DUNGEON_EVENT9_TEXT_SPECS);
    text_specs.extend_from_slice(&M27_DUNGEON_EVENT10_TEXT_SPECS);
    text_specs.extend_from_slice(&M28_DUNGEON_EVENT11_TEXT_SPECS);
    text_specs.extend_from_slice(&M29_DUNGEON_EVENT12_TEXT_SPECS);
    text_specs.extend_from_slice(&M30_DUNGEON_EVENT13_TEXT_SPECS);
    text_specs.extend_from_slice(&M31_DUNGEON_EVENT14_TEXT_SPECS);
    text_specs.extend_from_slice(&M32_DUNGEON_EVENT15_TEXT_SPECS);
    text_specs.extend_from_slice(&M33_DUNGEON_EVENT16_TEXT_SPECS);
    text_specs.extend_from_slice(&M34_DUNGEON_EVENT17_TEXT_SPECS);
    text_specs.extend_from_slice(&M35_DUNGEON_EVENT18_TEXT_SPECS);
    text_specs.extend_from_slice(&M36_DUNGEON_EVENT19_TEXT_SPECS);
    text_specs.extend_from_slice(&M37_DUNGEON_EVENT20_TEXT_SPECS);
    text_specs.extend_from_slice(&M38_DUNGEON_EVENT21_TEXT_SPECS);
    text_specs.extend_from_slice(&M39_DUNGEON_EVENT22_TEXT_SPECS);
    text_specs.extend_from_slice(&M40_DUNGEON_EVENT23_TEXT_SPECS);
    text_specs.extend_from_slice(&M41_DUNGEON_EVENT24_TEXT_SPECS);
    text_specs.extend_from_slice(&M42_DUNGEON_EVENT25_TEXT_SPECS);
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
            milestone: "M42",
        },
    )
}
