//! M43-M59 shop, intro, ending, and system-text milestones.

use super::super::*;
use super::dungeon::{validate_m40_spicy_curry_shared_tail, validate_m42_nested_event_tails};

/// Build the M43 JP-native first shop batch.
pub fn build_shop1_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
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
    text_specs.extend_from_slice(&M43_SHOP1_TEXT_SPECS);
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
            milestone: "M43",
        },
    )
}

/// Build the M44 JP-native second shop batch.
pub fn build_shop2_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
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
    text_specs.extend_from_slice(&M43_SHOP1_TEXT_SPECS);
    text_specs.extend_from_slice(&M44_SHOP2_TEXT_SPECS);
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
            milestone: "M44",
        },
    )
}

/// Build the M45 JP-native third shop batch.
pub fn build_shop3_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
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
    text_specs.extend_from_slice(&M43_SHOP1_TEXT_SPECS);
    text_specs.extend_from_slice(&M44_SHOP2_TEXT_SPECS);
    text_specs.extend_from_slice(&M45_SHOP3_TEXT_SPECS);
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
            milestone: "M45",
        },
    )
}

/// Build the M46 JP-native fourth shop batch.
pub fn build_shop4_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    let text_specs = m46_text_specs();
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
            milestone: "M46",
        },
    )
}

pub(crate) fn m46_text_specs() -> Vec<StableTextSpec> {
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
    text_specs.extend_from_slice(&M43_SHOP1_TEXT_SPECS);
    text_specs.extend_from_slice(&M44_SHOP2_TEXT_SPECS);
    text_specs.extend_from_slice(&M45_SHOP3_TEXT_SPECS);
    text_specs.extend_from_slice(&M46_SHOP4_TEXT_SPECS);
    text_specs
}

pub(crate) fn validate_m47_intro_boundaries(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<(), String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    if source.get(M47_DYNAMIC_BUFFER_OFFSET..M47_DYNAMIC_BUFFER_OFFSET + 4)
        != Some(M47_EXPECTED_DYNAMIC_BUFFER.as_slice())
    {
        return Err(
            "M47 dynamic-buffer script is no longer the exact four-byte FF78 consumer".into(),
        );
    }
    let dynamic_leaf = [Token::CtrlParam(0xFF78, 0xFF04)];
    validate_dynamic_display_population("script_1237", &dynamic_leaf)?;
    validate_fixed_width_layout(
        &dynamic_leaf,
        "script_1237",
        M2_MAX_FIXED_GLYPHS_PER_LINE,
        None,
    )?;

    let parent_spec = M47_INTRO1_TEXT_SPECS[0];
    let specs = [parent_spec, M47_WAKE_UP_ALIAS_SPEC];
    let entries = load_stable_text_entries(&assets_dir.join("translation"), &specs)?;
    let parent_words = validate_jp_text_source(
        &source,
        &entries[0].id,
        parent_spec.offset,
        &entries[0].jp,
        "M47 wake-up parent",
    )?;
    let alias_words = validate_jp_text_source(
        &source,
        &entries[1].id,
        M47_WAKE_UP_ALIAS_SPEC.offset,
        &entries[1].jp,
        "M47 wake-up alias",
    )?;
    if parent_spec.offset + 4 != M47_WAKE_UP_ALIAS_SPEC.offset
        || parent_words.get(2..) != Some(alias_words.as_slice())
    {
        return Err("M47 wake-up entry points no longer share the FF28 text tail".into());
    }

    let parent_tokens = crate::build::text::parse_display_text(&entries[0].ko);
    let alias_tokens = crate::build::text::parse_display_text(&entries[1].ko);
    if !matches!(
        parent_tokens.first(),
        Some(Token::CtrlParam(0xFF84, 0xA29C))
    ) || parent_tokens.get(1..) != Some(alias_tokens.as_slice())
    {
        return Err("M47 KR wake-up aliases no longer share one translated body".into());
    }
    Ok(())
}

pub(crate) fn validate_m49_intro_boundaries(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<(), String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    let specs = [M49_BLANK_CLEAR_SPEC];
    let entries = load_stable_text_entries(&assets_dir.join("translation"), &specs)?;

    validate_jp_text_source(
        &source,
        &entries[0].id,
        M49_BLANK_CLEAR_SPEC.offset,
        &entries[0].jp,
        "M49 JP-native blank clear",
    )?;
    if entries[0].jp != "{FF2C}                {NL}                {NL}                {FF38}" {
        return Err("M49 blank clear no longer owns the exact JP 16-by-3 space fill".into());
    }
    Ok(())
}

pub(crate) fn validate_m50_ending_boundaries(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<(), String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    let parent_spec = M50_ENDING1_TEXT_SPECS[6];
    let specs = [parent_spec, M50_CAMUS_ALIAS_SPEC];
    let entries = load_stable_text_entries(&assets_dir.join("translation"), &specs)?;
    let parent_words = validate_jp_text_source(
        &source,
        &entries[0].id,
        parent_spec.offset,
        &entries[0].jp,
        "M50 Camus-tail parent",
    )?;
    let alias_words = validate_jp_text_source(
        &source,
        &entries[1].id,
        M50_CAMUS_ALIAS_SPEC.offset,
        &entries[1].jp,
        "M50 Camus-tail alias",
    )?;
    let suffix_word_index = parent_words
        .len()
        .checked_sub(alias_words.len())
        .ok_or_else(|| "M50 Camus alias is longer than its parent".to_string())?;
    if parent_spec.offset + suffix_word_index * 2 != M50_CAMUS_ALIAS_SPEC.offset
        || parent_words.get(suffix_word_index..) != Some(alias_words.as_slice())
    {
        return Err("M50 Camus entry no longer shares the parent JP text tail".into());
    }

    let parent_tokens = crate::build::text::parse_display_text(&entries[0].ko);
    let alias_tokens = crate::build::text::parse_display_text(&entries[1].ko);
    if !parent_tokens.ends_with(&alias_tokens) {
        return Err("M50 Camus KR alias no longer matches its translated parent suffix".into());
    }
    Ok(())
}

pub(crate) fn validate_m51_ending_boundaries(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<(), String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    let parent_spec = M51_ENDING2_TEXT_SPECS[22];
    let tail_spec = M51_ENDING2_TEXT_SPECS[23];
    if tail_spec.id != M51_SCORE_FEEDBACK_TAIL_SPEC.id
        || tail_spec.offset != M51_SCORE_FEEDBACK_TAIL_SPEC.offset
        || tail_spec.legacy_fff8_idx != M51_SCORE_FEEDBACK_TAIL_SPEC.legacy_fff8_idx
    {
        return Err("M51 score-feedback tail is missing from the stable catalog".into());
    }
    let pointer_bytes = source
        .get(M51_SCORE_FEEDBACK_NATIVE_POINTER_OFFSET..M51_SCORE_FEEDBACK_NATIVE_POINTER_OFFSET + 2)
        .ok_or("M51 score-feedback native pointer is outside the JP ROM")?;
    let pointer = u16::from_be_bytes([pointer_bytes[0], pointer_bytes[1]]);
    if pointer as usize != M51_SCORE_FEEDBACK_TAIL_SPEC.offset & 0xFFFF {
        return Err(format!(
            "M51 score-feedback native pointer is 0x{pointer:04X}, expected 0x{:04X}",
            M51_SCORE_FEEDBACK_TAIL_SPEC.offset & 0xFFFF
        ));
    }
    let specs = [parent_spec, tail_spec];
    let entries = load_stable_text_entries(&assets_dir.join("translation"), &specs)?;
    let parent_words = validate_jp_text_source(
        &source,
        &entries[0].id,
        parent_spec.offset,
        &entries[0].jp,
        "M51 score-result parent",
    )?;
    let alias_words = validate_jp_text_source(
        &source,
        &entries[1].id,
        tail_spec.offset,
        &entries[1].jp,
        "M51 score-feedback tail",
    )?;
    let suffix_word_index = parent_words
        .len()
        .checked_sub(alias_words.len())
        .ok_or_else(|| "M51 score-feedback tail is longer than its parent".to_string())?;
    if parent_spec.offset + suffix_word_index * 2 != tail_spec.offset
        || parent_words.get(suffix_word_index..) != Some(alias_words.as_slice())
    {
        return Err("M51 score entry no longer shares the JP feedback tail".into());
    }

    let parent_tokens = crate::build::text::parse_display_text(&entries[0].ko);
    let alias_tokens = crate::build::text::parse_display_text(&entries[1].ko);
    if !parent_tokens.ends_with(&alias_tokens) {
        return Err(
            "M51 score KR feedback tail no longer matches its translated parent suffix".into(),
        );
    }
    Ok(())
}

pub(crate) fn validate_m52_ending_duplicates(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<(), String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    let specs = [M52_ENDING3_TEXT_SPECS[3], M52_ENDING3_TEXT_SPECS[7]];
    let entries = load_stable_text_entries(&assets_dir.join("translation"), &specs)?;
    let first_words = validate_jp_text_source(
        &source,
        &entries[0].id,
        specs[0].offset,
        &entries[0].jp,
        "M52 first graduation narration",
    )?;
    let second_words = validate_jp_text_source(
        &source,
        &entries[1].id,
        specs[1].offset,
        &entries[1].jp,
        "M52 repeated graduation narration",
    )?;
    if first_words != second_words {
        return Err("M52 repeated JP graduation narrations no longer match".into());
    }
    if crate::build::text::parse_display_text(&entries[0].ko)
        != crate::build::text::parse_display_text(&entries[1].ko)
    {
        return Err("M52 repeated KR graduation narrations no longer match".into());
    }
    Ok(())
}

/// Build the M47 JP-native first automatic-prologue batch.
pub fn build_intro1_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    let text_specs = m47_text_specs();
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
            milestone: "M47",
        },
    )
}

pub(crate) fn m47_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m46_text_specs();
    text_specs.extend_from_slice(&M47_INTRO1_TEXT_SPECS);
    text_specs
}

/// Build the M48 JP-native second automatic-intro batch.
pub fn build_intro2_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    let text_specs = m48_text_specs();
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
            milestone: "M48",
        },
    )
}

pub(crate) fn m48_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m47_text_specs();
    text_specs.extend_from_slice(&M48_INTRO2_TEXT_SPECS);
    text_specs
}

/// Build the M49 JP-native automatic-intro final fall-effect batch.
pub fn build_intro3_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    let mut text_specs = m48_text_specs();
    text_specs.extend_from_slice(&M49_INTRO3_TEXT_SPECS);
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
            milestone: "M49",
        },
    )
}

pub(crate) fn m49_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m48_text_specs();
    text_specs.extend_from_slice(&M49_INTRO3_TEXT_SPECS);
    text_specs
}

/// Build the M50 JP-native first ending escape-and-rescue batch.
pub fn build_ending1_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    let text_specs = m50_text_specs();
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
            milestone: "M50",
        },
    )
}

pub(crate) fn m50_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m49_text_specs();
    text_specs.extend_from_slice(&M50_ENDING1_TEXT_SPECS);
    text_specs
}

/// Build the M51 JP-native ending aftermath-and-score batch.
pub fn build_ending2_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    let text_specs = m51_text_specs();
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
            extra_glyphs: &M51_KR_EXTRA_GLYPHS,
            milestone: "M51",
        },
    )
}

pub(crate) fn m51_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m50_text_specs();
    text_specs.extend_from_slice(&M51_ENDING2_TEXT_SPECS);
    text_specs
}

/// Build the M52 JP-native ending outcome-and-gift batch.
pub fn build_ending3_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    let text_specs = m52_text_specs();
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
            extra_glyphs: &M51_KR_EXTRA_GLYPHS,
            milestone: "M52",
        },
    )
}

pub(crate) fn m52_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m51_text_specs();
    text_specs.extend_from_slice(&M52_ENDING3_TEXT_SPECS);
    text_specs
}

/// Build the M55 JP-native save, floor-label, and encounter system batch.
pub fn build_system1_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    let text_specs = m55_text_specs();
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
            extra_glyphs: &M51_KR_EXTRA_GLYPHS,
            milestone: "M55",
        },
    )
}

pub(crate) fn m55_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m52_text_specs();
    text_specs.extend_from_slice(&M54_NESTED_TAIL_TEXT_SPECS);
    text_specs.extend_from_slice(&M55_SYSTEM1_TEXT_SPECS);
    text_specs
}

/// Build the M56 JP-native chest, item, note, and door system batch.
pub fn build_system2_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    let text_specs = m56_text_specs();
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
            extra_glyphs: &M51_KR_EXTRA_GLYPHS,
            milestone: "M56",
        },
    )
}

pub(crate) fn m56_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m55_text_specs();
    text_specs.extend_from_slice(&M56_SYSTEM2_TEXT_SPECS);
    text_specs
}

pub(crate) fn m57_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m56_text_specs();
    text_specs.extend_from_slice(&M57_SYSTEM3_TEXT_SPECS);
    text_specs
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum M58SelectorSource {
    Immediate(u16),
    Dynamic,
}

pub(crate) fn m58_read_word(source: &[u8], offset: usize, label: &str) -> Result<u16, String> {
    let bytes = source
        .get(offset..offset + 2)
        .ok_or_else(|| format!("{label}: word at 0x{offset:06X} is outside the JP ROM"))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

pub(crate) fn m58_expect_instruction(
    source: &[u8],
    offset: usize,
    instruction: Inst,
    label: &str,
) -> Result<(), String> {
    let expected = assemble_m68k_at(offset as u32, &[instruction])?;
    let actual = source
        .get(offset..offset + expected.len())
        .ok_or_else(|| format!("{label}: instruction at 0x{offset:06X} exceeds the JP ROM"))?;
    if actual != expected {
        return Err(format!(
            "{label}: typed 68000 instruction mismatch at 0x{offset:06X}"
        ));
    }
    Ok(())
}

pub(crate) fn m58_direct_jsr_offsets(source: &[u8], target: usize) -> Result<Vec<usize>, String> {
    let mut calls = Vec::new();
    for offset in (0..JP_CODE_SCAN_END - 5).step_by(2) {
        let opcode = m58_read_word(source, offset, "M58 JSR scan")?;
        let matches = match opcode {
            0x4EB9 => {
                let bytes = &source[offset + 2..offset + 6];
                u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize == target
            }
            0x4EBA => {
                let displacement =
                    m58_read_word(source, offset + 2, "M58 PC-relative JSR")? as i16 as isize;
                offset as isize + 2 + displacement == target as isize
            }
            _ => false,
        };
        if matches {
            calls.push(offset);
        }
    }
    Ok(calls)
}

pub(crate) fn m58_selector_source(
    source: &[u8],
    call_offset: usize,
) -> Result<M58SelectorSource, String> {
    let dynamic = assemble_m68k(&[Inst::MoveWordAbsoluteShortToData {
        address: JP_SYSTEM_SELECTOR_ADDRESS,
        destination: DataReg::D0,
    }])?;

    for distance in (4..=20).step_by(2) {
        let Some(offset) = call_offset.checked_sub(distance) else {
            break;
        };
        if source.get(offset..offset + dynamic.len()) == Some(dynamic.as_slice()) {
            return Ok(M58SelectorSource::Dynamic);
        }
        if m58_read_word(source, offset, "M58 selector source")? == 0x303C {
            let immediate = m58_read_word(source, offset + 2, "M58 immediate selector")?;
            let expected = assemble_m68k(&[Inst::MoveWordImmediateToData {
                immediate,
                destination: DataReg::D0,
            }])?;
            if source.get(offset..offset + expected.len()) == Some(expected.as_slice()) {
                return Ok(M58SelectorSource::Immediate(immediate));
            }
        }
    }

    Err(format!(
        "M58 could not resolve the D0 selector feeding JSR at 0x{call_offset:06X}"
    ))
}

pub(crate) fn validate_m58_unconsumed_system_rewards(jp_rom_path: &Path) -> Result<(), String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;

    for spec in M58_UNCONSUMED_SYSTEM_TEXT_SPECS {
        let pointer_offset = JP_SYSTEM_TEXT_POINTER_TABLE + usize::from(spec.pointer_index) * 2;
        let relative = usize::from(m58_read_word(
            &source,
            pointer_offset,
            "M58 system pointer",
        )?);
        if JP_SYSTEM_TEXT_POINTER_TABLE + relative != spec.target_offset {
            return Err(format!(
                "M58 {} pointer index {} no longer resolves to 0x{:06X}",
                spec.id, spec.pointer_index, spec.target_offset
            ));
        }
    }

    let system_targets = (0..JP_SYSTEM_TEXT_POINTER_COUNT)
        .map(|index| {
            let relative = usize::from(m58_read_word(
                &source,
                JP_SYSTEM_TEXT_POINTER_TABLE + index * 2,
                "M58 system pointer population",
            )?);
            Ok(JP_SYSTEM_TEXT_POINTER_TABLE + relative)
        })
        .collect::<Result<BTreeSet<_>, String>>()?;
    if m58_read_word(&source, 0x09_E3BC, "M58 pre-tail terminator")? != 0xFFFF
        || m58_read_word(&source, 0x09_E436, "M58 first tail terminator")? != 0xFFFF
        || m58_read_word(&source, 0x09_E4B0, "M58 second tail terminator")? != 0xFFFF
    {
        return Err("M58 EN-extracted system tail boundaries changed".into());
    }
    for (fff8_index, start, end) in M58_EN_UNCONSUMED_TAILS {
        if system_targets
            .iter()
            .any(|target| *target >= start && *target < end)
        {
            return Err(format!(
                "M58 FFF8 {fff8_index} unconsumed tail gained a JP system-table target"
            ));
        }
    }

    let mut expected_selector_operands = BTreeSet::new();
    for (offset, selector) in M58_SELECTOR_WRITES {
        m58_expect_instruction(
            &source,
            offset,
            Inst::MoveWordImmediateToAbsoluteShort {
                immediate: selector,
                address: JP_SYSTEM_SELECTOR_ADDRESS,
            },
            "M58 selector write",
        )?;
        expected_selector_operands.insert(offset + 4);
    }
    for offset in M58_SELECTOR_READS {
        m58_expect_instruction(
            &source,
            offset,
            Inst::MoveWordAbsoluteShortToData {
                address: JP_SYSTEM_SELECTOR_ADDRESS,
                destination: DataReg::D0,
            },
            "M58 selector read",
        )?;
        expected_selector_operands.insert(offset + 2);
    }
    let selector_address = JP_SYSTEM_SELECTOR_ADDRESS.to_be_bytes();
    let actual_selector_operands = source[..JP_CODE_SCAN_END]
        .windows(selector_address.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == selector_address).then_some(offset))
        .collect::<BTreeSet<_>>();
    if actual_selector_operands != expected_selector_operands {
        return Err("M58 system-selector RAM xrefs no longer match the closed ledger".into());
    }

    let table_address = (JP_SYSTEM_TEXT_POINTER_TABLE as u32).to_be_bytes();
    let actual_table_refs = source[..JP_CODE_SCAN_END]
        .windows(table_address.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == table_address).then_some(offset))
        .collect::<BTreeSet<_>>();
    let expected_table_refs = BTreeSet::from([0x0000_45A0, 0x0000_45E2, 0x0003_9FEA]);
    if actual_table_refs != expected_table_refs {
        return Err(
            "M58 system pointer-table xrefs no longer match the three known readers".into(),
        );
    }
    for offset in [
        JP_SYSTEM_TEXT_DISPLAY_ROUTINE,
        0x0000_45E0,
        JP_SYSTEM_TEXT_DIRECT_READER,
    ] {
        m58_expect_instruction(
            &source,
            offset,
            Inst::LeaAbsoluteLong {
                address: JP_SYSTEM_TEXT_POINTER_TABLE as u32,
                destination: AddressReg::A2,
            },
            "M58 system pointer-table reader",
        )?;
    }
    for (offset, selector) in [(0x0000_45C4, 20), (0x0000_45D4, 21)] {
        m58_expect_instruction(
            &source,
            offset,
            Inst::MoveWordImmediateToData {
                immediate: selector,
                destination: DataReg::D0,
            },
            "M58 fixed system-table wrapper selector",
        )?;
    }

    let orphan_pointer_indices = M58_UNCONSUMED_SYSTEM_TEXT_SPECS
        .iter()
        .map(|spec| spec.pointer_index)
        .collect::<BTreeSet<_>>();
    if M58_SELECTOR_WRITES
        .iter()
        .any(|(_, selector)| orphan_pointer_indices.contains(selector))
    {
        return Err("M58 dynamic selector storage can reach an orphan reward slot".into());
    }

    let display_calls = m58_direct_jsr_offsets(&source, JP_SYSTEM_TEXT_DISPLAY_ROUTINE)?;
    if display_calls.len() != M58_EXPECTED_DISPLAY_CALL_COUNT {
        return Err(format!(
            "M58 expected {} direct system-display calls, found {}",
            M58_EXPECTED_DISPLAY_CALL_COUNT,
            display_calls.len()
        ));
    }
    let mut dynamic_calls = Vec::new();
    for call in display_calls {
        match m58_selector_source(&source, call)? {
            M58SelectorSource::Immediate(selector) => {
                if orphan_pointer_indices.contains(&selector) {
                    return Err(format!(
                        "M58 direct system-display call at 0x{call:06X} selects orphan slot {selector}"
                    ));
                }
            }
            M58SelectorSource::Dynamic => dynamic_calls.push(call),
        }
    }
    if dynamic_calls != [M58_DYNAMIC_SELECTOR_CALL] {
        return Err("M58 dynamic system-display callsite no longer matches the ledger".into());
    }

    let direct_reader_calls = m58_direct_jsr_offsets(&source, JP_SYSTEM_TEXT_DIRECT_READER)?;
    if direct_reader_calls.len() != M58_EXPECTED_DIRECT_READER_CALL_COUNT {
        return Err(format!(
            "M58 expected {} direct-reader calls, found {}",
            M58_EXPECTED_DIRECT_READER_CALL_COUNT,
            direct_reader_calls.len()
        ));
    }
    let mut direct_reader_selectors = BTreeSet::new();
    for call in direct_reader_calls {
        match m58_selector_source(&source, call)? {
            M58SelectorSource::Immediate(selector) => {
                direct_reader_selectors.insert(selector);
            }
            M58SelectorSource::Dynamic => {
                return Err("M58 direct system-table reader gained a dynamic selector".into());
            }
        }
    }
    if direct_reader_selectors != BTreeSet::from([29, 32]) {
        return Err(format!(
            "M58 direct system-table reader selectors changed: {direct_reader_selectors:?}"
        ));
    }

    Ok(())
}

pub(crate) fn validate_m59_live_system_entries(jp_rom_path: &Path) -> Result<(), String> {
    validate_m58_unconsumed_system_rewards(jp_rom_path)?;
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;

    for (text_spec, (id, pointer_index)) in
        M59_SYSTEM4_TEXT_SPECS.iter().zip(M59_SYSTEM_POINTER_SPECS)
    {
        if text_spec.id != id {
            return Err(format!(
                "M59 text/pointer ledger ID mismatch: {} != {id}",
                text_spec.id
            ));
        }
        let pointer_offset = JP_SYSTEM_TEXT_POINTER_TABLE + usize::from(pointer_index) * 2;
        let relative = usize::from(m58_read_word(
            &source,
            pointer_offset,
            "M59 live system pointer",
        )?);
        if JP_SYSTEM_TEXT_POINTER_TABLE + relative != text_spec.offset {
            return Err(format!(
                "M59 {id} pointer index {pointer_index} no longer resolves to 0x{:06X}",
                text_spec.offset
            ));
        }
    }

    let mut fixed_selectors = BTreeSet::new();
    for target in [JP_SYSTEM_TEXT_DISPLAY_ROUTINE, JP_SYSTEM_TEXT_DIRECT_READER] {
        for call in m58_direct_jsr_offsets(&source, target)? {
            if let M58SelectorSource::Immediate(selector) = m58_selector_source(&source, call)? {
                fixed_selectors.insert(selector);
            }
        }
    }
    for (id, pointer_index) in M59_SYSTEM_POINTER_SPECS {
        if !fixed_selectors.contains(&pointer_index) {
            return Err(format!(
                "M59 {id} pointer index {pointer_index} has no fixed JP callsite"
            ));
        }
    }

    Ok(())
}

/// Build the M57 JP-native door, stair, spell-tutorial, and hazard system batch.
pub fn build_system3_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m58_unconsumed_system_rewards(jp_rom_path)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    let text_specs = m57_text_specs();
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
            extra_glyphs: &M51_KR_EXTRA_GLYPHS,
            milestone: "M57",
        },
    )
}

/// Build the M59 JP-native exhausted-wall, floating-stone, and dark-passage batch.
pub fn build_system4_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    validate_m59_live_system_entries(jp_rom_path)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    let text_specs = m59_text_specs();
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
            extra_glyphs: &M51_KR_EXTRA_GLYPHS,
            milestone: "M59",
        },
    )
}

pub(crate) fn m59_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m57_text_specs();
    text_specs.extend_from_slice(&M59_SYSTEM4_TEXT_SPECS);
    text_specs
}
