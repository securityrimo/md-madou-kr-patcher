//! M60-M70 fixed names, late battle text, spell, event, and canonical build.

use super::super::*;
use super::dungeon::{validate_m40_spicy_curry_shared_tail, validate_m42_nested_event_tails};
use super::scenes_system::{
    build_system4_poc, m58_expect_instruction, m58_read_word, m59_text_specs,
    validate_m47_intro_boundaries, validate_m49_intro_boundaries, validate_m50_ending_boundaries,
    validate_m51_ending_boundaries, validate_m52_ending_duplicates,
};

/// Build the M60 JP-native six-word monster-name table on top of M59.
pub fn build_monster_names_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;

    let baseline = build_system4_poc(jp_rom_path, assets_dir)?;
    apply_m60_monster_names(&source, baseline, assets_dir, &M51_KR_EXTRA_GLYPHS)
}

/// Build the M61 JP-native voiced-damage batch on top of the M60 fixed names.
pub fn build_damage_voice_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;

    let text_specs = m61_text_specs();
    let baseline = build_text_poc_internal(
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
            milestone: "M61",
        },
    )?;
    apply_m60_monster_names(&source, baseline, assets_dir, &M51_KR_EXTRA_GLYPHS)
}

pub(crate) fn m61_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m59_text_specs();
    text_specs.extend_from_slice(&M61_DAMAGE_VOICE_TEXT_SPECS);
    text_specs
}

/// Build the M62 JP-native early Puyo battle batch on top of M61.
///
/// M62 promotes the visible consumers in the contiguous FFF8 population 474
/// through 495. The four-byte empty entry at 485 and the physical fragments
/// without legacy consumers remain in their JP form.
pub fn build_early_puyo_battle_poc(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;

    let text_specs = m62_text_specs();
    let baseline = build_text_poc_internal(
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
            milestone: "M62",
        },
    )?;
    apply_m60_monster_names(&source, baseline, assets_dir, &M51_KR_EXTRA_GLYPHS)
}

pub(crate) fn m62_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m61_text_specs();
    text_specs.extend_from_slice(&M62_EARLY_PUYO_BATTLE_TEXT_SPECS);
    text_specs
}

/// Build the M63 JP-native monster-encounter introduction batch on top of M62.
///
/// M63 promotes every visible consumer in FFF8 indices 447 through 473. The
/// two control-only transition slots 469 and 473 remain in their JP form.
pub fn build_encounter_intro_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;

    let text_specs = m63_text_specs();
    let baseline = build_text_poc_internal(
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
            milestone: "M63",
        },
    )?;
    apply_m60_monster_names(&source, baseline, assets_dir, &M51_KR_EXTRA_GLYPHS)
}

pub(crate) fn m63_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m62_text_specs();
    text_specs.extend_from_slice(&M63_ENCOUNTER_INTRO_TEXT_SPECS);
    text_specs
}

/// Build the M64 JP-native non-voiced damage batch on top of M63.
///
/// M64 promotes the complete FFF8 `dmg_novoice` selector range 123 through 138
/// and keeps it semantically paired with M61 after removing only voice controls.
pub fn build_damage_novoice_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    validate_m75_player_damage_path_classification(&source)?;

    let text_specs = m64_text_specs();
    let baseline = build_text_poc_internal(
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
            milestone: "M64",
        },
    )?;
    apply_m60_monster_names(&source, baseline, assets_dir, &M51_KR_EXTRA_GLYPHS)
}

pub(crate) fn m64_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m63_text_specs();
    text_specs.extend_from_slice(&M64_DAMAGE_NOVOICE_TEXT_SPECS);
    text_specs
}

/// Build the M65 JP-native spell-message batch on top of M64.
///
/// M65 promotes the source-backed FFF8 `spell_msg` consumers from 139 through
/// 204, excluding the zero EN pointer at index 192. JP `FFC0` boundaries stay
/// independent instead of inheriting EN-composed following messages.
pub fn build_spell_msg_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;

    let mut text_specs = m64_text_specs();
    text_specs.extend_from_slice(&M65_SPELL_MSG_TEXT_SPECS);
    let baseline = build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &M65_KR_EXTRA_GLYPHS,
            milestone: "M65",
        },
    )?;
    apply_m60_monster_names(&source, baseline, assets_dir, &M65_KR_EXTRA_GLYPHS)
}

/// Build the M66 JP-native event/prefix-alias boundary on top of M65.
///
/// Only FFF8 index 205 owns a new payload. Indices 206 and 207 retain their
/// original JP `FF50` prefixes and then fall through into the already promoted
/// M65 consumers 196 and 198 respectively.
pub fn build_event_alias_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    validate_m66_event_prefix_aliases(jp_rom_path, assets_dir)?;

    let text_specs = m66_text_specs();
    let baseline = build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &M65_KR_EXTRA_GLYPHS,
            milestone: "M66",
        },
    )?;
    let output = apply_m60_monster_names(&source, baseline, assets_dir, &M65_KR_EXTRA_GLYPHS)?;
    validate_m66_event_prefix_redirects(&source, &output, &text_specs)?;
    Ok(output)
}

pub(crate) fn m66_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m64_text_specs();
    text_specs.extend_from_slice(&M65_SPELL_MSG_TEXT_SPECS);
    text_specs.extend_from_slice(&M66_EVENT_TEXT_SPECS);
    text_specs
}

pub(crate) fn validate_m66_event_prefix_aliases(
    jp_rom_path: &Path,
    assets_dir: &Path,
) -> Result<(), String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    let translation_dir = assets_dir.join("translation");

    for (alias_index, child_index, expected_prefix) in M66_EVENT_PREFIX_ALIAS_PAIRS {
        let alias_spec = *M66_EVENT_PREFIX_ALIAS_SPECS
            .iter()
            .find(|spec| spec.legacy_fff8_idx == alias_index)
            .ok_or_else(|| format!("M66 alias index {alias_index} is missing"))?;
        let child_spec = *M65_SPELL_MSG_TEXT_SPECS
            .iter()
            .find(|spec| spec.legacy_fff8_idx == child_index)
            .ok_or_else(|| format!("M66 child index {child_index} is missing from M65"))?;
        if alias_spec.offset + 4 != child_spec.offset {
            return Err(format!(
                "M66 alias {alias_index} no longer falls through into child {child_index}"
            ));
        }

        let specs = [alias_spec, child_spec];
        let entries = load_stable_text_entries(&translation_dir, &specs)?;
        let alias_words = validate_jp_text_source(
            &source,
            &entries[0].id,
            alias_spec.offset,
            &entries[0].jp,
            "M66 event-prefix alias",
        )?;
        let child_words = validate_jp_text_source(
            &source,
            &entries[1].id,
            child_spec.offset,
            &entries[1].jp,
            "M66 shared M65 tail",
        )?;
        if alias_words.get(..2) != Some(expected_prefix.as_slice())
            || alias_words.get(2..) != Some(child_words.as_slice())
        {
            return Err(format!(
                "M66 alias {alias_index} no longer has its exact prefix plus child {child_index}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_m66_event_prefix_redirects(
    source: &[u8],
    output: &[u8],
    text_specs: &[StableTextSpec],
) -> Result<(), String> {
    for (alias_index, child_index, _) in M66_EVENT_PREFIX_ALIAS_PAIRS {
        let alias_spec = M66_EVENT_PREFIX_ALIAS_SPECS
            .iter()
            .find(|spec| spec.legacy_fff8_idx == alias_index)
            .ok_or_else(|| format!("M66 alias index {alias_index} is missing"))?;
        let child_spec = M65_SPELL_MSG_TEXT_SPECS
            .iter()
            .find(|spec| spec.legacy_fff8_idx == child_index)
            .ok_or_else(|| format!("M66 child index {child_index} is missing from M65"))?;
        if output.get(alias_spec.offset..child_spec.offset)
            != source.get(alias_spec.offset..child_spec.offset)
        {
            return Err(format!(
                "M66 alias {alias_index} did not preserve its JP control prefix"
            ));
        }
        let local_id = text_specs
            .iter()
            .position(|spec| spec.id == child_spec.id)
            .ok_or_else(|| {
                format!(
                    "M66 child {} is absent from the stable catalog",
                    child_spec.id
                )
            })?;
        let local_id = u16::try_from(local_id)
            .map_err(|_| format!("M66 child {} local ID exceeds 16 bits", child_spec.id))?;
        let expected = [
            0xFF,
            0xF8,
            (JP_TEXT_REDIRECT_MAGIC >> 8) as u8,
            JP_TEXT_REDIRECT_MAGIC as u8,
            (local_id >> 8) as u8,
            local_id as u8,
        ];
        if output.get(child_spec.offset..child_spec.offset + expected.len())
            != Some(expected.as_slice())
        {
            return Err(format!(
                "M66 alias {alias_index} does not reach the M65 child {child_index} redirect"
            ));
        }
    }
    Ok(())
}

/// Build the M67 JP-native spell-command table on top of M66.
pub fn build_spell_command_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    validate_m66_event_prefix_aliases(jp_rom_path, assets_dir)?;
    validate_m67_spell_command_table(&source)?;

    let text_specs = m67_text_specs();
    let baseline = build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &M65_KR_EXTRA_GLYPHS,
            milestone: "M67",
        },
    )?;
    let output = apply_m60_monster_names(&source, baseline, assets_dir, &M65_KR_EXTRA_GLYPHS)?;
    validate_m66_event_prefix_redirects(&source, &output, &text_specs)?;
    validate_m67_spell_command_result(&source, &output)?;
    Ok(output)
}

pub(crate) fn m67_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m66_text_specs();
    text_specs.extend_from_slice(&M67_SPELL_COMMAND_TEXT_SPECS);
    text_specs
}

pub(crate) fn validate_m67_spell_command_table(source: &[u8]) -> Result<(), String> {
    m58_expect_instruction(
        source,
        JP_SPELL_COMMAND_CONSUMER,
        Inst::LeaAbsoluteLong {
            address: JP_SPELL_COMMAND_TABLE as u32,
            destination: AddressReg::A2,
        },
        "M67 spell-command table consumer",
    )?;

    let table_address = (JP_SPELL_COMMAND_TABLE as u32).to_be_bytes();
    let actual_refs = source[..JP_CODE_SCAN_END]
        .windows(table_address.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == table_address).then_some(offset))
        .collect::<BTreeSet<_>>();
    if actual_refs != BTreeSet::from([JP_SPELL_COMMAND_CONSUMER + 2]) {
        return Err(format!(
            "M67 spell-command table xrefs changed: {actual_refs:?}"
        ));
    }

    let mut targets = BTreeSet::new();
    for (slot, legacy_index) in M67_SPELL_COMMAND_TABLE_TARGET_INDICES
        .iter()
        .copied()
        .enumerate()
    {
        let relative = usize::from(m58_read_word(
            source,
            JP_SPELL_COMMAND_TABLE + slot * 2,
            "M67 spell-command table pointer",
        )?);
        let target = JP_SPELL_COMMAND_TABLE + relative;
        let spec = M67_SPELL_COMMAND_TEXT_SPECS
            .iter()
            .find(|spec| spec.legacy_fff8_idx == legacy_index)
            .ok_or_else(|| format!("M67 spell-command index {legacy_index} is missing"))?;
        if target != spec.offset {
            return Err(format!(
                "M67 spell-command slot {slot} resolves to 0x{target:06X}, not {} at 0x{:06X}",
                spec.id, spec.offset
            ));
        }
        targets.insert(target);
    }
    if targets.len() != M67_SPELL_COMMAND_TEXT_SPECS.len() {
        return Err("M67 spell-command table no longer reaches ten unique entries".into());
    }
    Ok(())
}

pub(crate) fn validate_m67_spell_command_result(
    source: &[u8],
    output: &[u8],
) -> Result<(), String> {
    let table_end = JP_SPELL_COMMAND_TABLE + JP_SPELL_COMMAND_TABLE_COUNT * 2;
    if output.get(JP_SPELL_COMMAND_TABLE..table_end)
        != source.get(JP_SPELL_COMMAND_TABLE..table_end)
    {
        return Err("M67 modified the native spell-command pointer table".into());
    }
    Ok(())
}

/// Build the M68 JP-native special item-event texts on top of M67.
pub fn build_item_event_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    validate_m66_event_prefix_aliases(jp_rom_path, assets_dir)?;
    validate_m67_spell_command_table(&source)?;
    validate_m68_item_event_boundaries(&source, assets_dir)?;

    let text_specs = m68_text_specs();
    let baseline = build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &M65_KR_EXTRA_GLYPHS,
            milestone: "M68",
        },
    )?;
    let output = apply_m60_monster_names(&source, baseline, assets_dir, &M65_KR_EXTRA_GLYPHS)?;
    validate_m66_event_prefix_redirects(&source, &output, &text_specs)?;
    validate_m67_spell_command_result(&source, &output)?;
    Ok(output)
}

pub(crate) fn m68_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m67_text_specs();
    text_specs.extend_from_slice(&M68_ITEM_EVENT_TEXT_SPECS[1..]);
    text_specs
}

pub(crate) fn validate_m68_item_event_boundaries(
    source: &[u8],
    assets_dir: &Path,
) -> Result<(), String> {
    let entries =
        load_stable_text_entries(&assets_dir.join("translation"), &M68_ITEM_EVENT_TEXT_SPECS)?;
    let all_item_targets = collect_item_text_targets(source)?;

    for (entry, spec) in entries.iter().zip(M68_ITEM_EVENT_TEXT_SPECS) {
        let words = validate_jp_text_source(
            source,
            &entry.id,
            spec.offset,
            &entry.jp,
            "M68 special item event",
        )?;
        let expected_item_target = spec.id == JP_ITEM_DESC_SHARED_EVENT_ID;
        if all_item_targets.contains(&spec.offset) != expected_item_target {
            return Err(format!(
                "{} at 0x{:06X} item-table membership differs: expected {expected_item_target}",
                spec.id, spec.offset,
            ));
        }
        if words.last() == Some(&0xFFFF)
            && source.get(spec.offset.saturating_sub(2)..spec.offset) != Some(&[0xFF, 0xFF])
        {
            return Err(format!(
                "{} no longer follows a separately terminated JP item entry",
                spec.id
            ));
        }
    }

    let elephant = &entries[1];
    let elephant_words = validate_jp_text_source(
        source,
        &elephant.id,
        M68_ITEM_EVENT_TEXT_SPECS[1].offset,
        &elephant.jp,
        "M68 elephant throw boundary",
    )?;
    let next_offset = M68_ITEM_EVENT_TEXT_SPECS[1].offset + elephant_words.len() * 2;
    if next_offset != 0x0A_1B9A
        || source.get(next_offset..next_offset + 4) != Some(&[0xFF, 0x18, 0xFF, 0xB8])
    {
        return Err("M68 elephant throw no longer ends at the native item-use2 successor".into());
    }
    Ok(())
}

/// Build the M69 JP-native enemy-health status ladder on top of M68.
pub fn build_enemy_hp_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    validate_m66_event_prefix_aliases(jp_rom_path, assets_dir)?;
    validate_m67_spell_command_table(&source)?;
    validate_m68_item_event_boundaries(&source, assets_dir)?;
    validate_m69_enemy_hp_boundaries(&source, assets_dir)?;
    validate_m72_enemy_status_consumers(&source)?;

    let text_specs = m69_text_specs();
    let baseline = build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &M65_KR_EXTRA_GLYPHS,
            milestone: "M69",
        },
    )?;
    let output = apply_m60_monster_names(&source, baseline, assets_dir, &M65_KR_EXTRA_GLYPHS)?;
    validate_m66_event_prefix_redirects(&source, &output, &text_specs)?;
    validate_m67_spell_command_result(&source, &output)?;
    Ok(output)
}

pub(crate) fn m69_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m68_text_specs();
    text_specs.extend_from_slice(&M69_ENEMY_HP_TEXT_SPECS);
    text_specs
}

pub(crate) fn validate_m69_enemy_hp_boundaries(
    source: &[u8],
    assets_dir: &Path,
) -> Result<(), String> {
    let entries =
        load_stable_text_entries(&assets_dir.join("translation"), &M69_ENEMY_HP_TEXT_SPECS)?;
    let mut expected_offset = M69_ENEMY_HP_TEXT_SPECS[0].offset;

    for (entry, spec) in entries.iter().zip(M69_ENEMY_HP_TEXT_SPECS) {
        if spec.offset != expected_offset {
            return Err(format!(
                "M69 enemy-health entry {} starts at 0x{:06X}, expected 0x{expected_offset:06X}",
                spec.id, spec.offset
            ));
        }
        let words = validate_jp_text_source(
            source,
            &entry.id,
            spec.offset,
            &entry.jp,
            "M69 enemy-health status",
        )?;
        expected_offset += words.len() * 2;
    }

    if expected_offset != 0x0A_299E {
        return Err(format!(
            "M69 enemy-health batch ends at 0x{expected_offset:06X}, expected 0x0A299E"
        ));
    }
    Ok(())
}

/// Build the current cumulative JP-to-KR ROM from the exact JP source.
///
/// Milestone builders remain available for regression bisection, but this is
/// the stable library entry point used by the canonical source-only pipeline.
pub fn build_jp_kr(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    build_enemy_damage_poc(jp_rom_path, assets_dir)
}

/// Build the M70 JP-native enemy-damage response ladder on top of M69.
pub fn build_enemy_damage_poc(jp_rom_path: &Path, assets_dir: &Path) -> Result<Vec<u8>, String> {
    let source = fs::read(jp_rom_path).map_err(|e| format!("failed to read JP ROM: {e}"))?;
    validate_jp_source(&source)?;
    audit_fff8_ownership(assets_dir)?;
    validate_m58_unconsumed_system_rewards(jp_rom_path)?;
    validate_m60_monster_name_consumer(&source)?;
    validate_m40_spicy_curry_shared_tail(jp_rom_path, assets_dir)?;
    validate_m42_nested_event_tails(jp_rom_path, assets_dir)?;
    validate_m47_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m49_intro_boundaries(jp_rom_path, assets_dir)?;
    validate_m50_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m51_ending_boundaries(jp_rom_path, assets_dir)?;
    validate_m52_ending_duplicates(jp_rom_path, assets_dir)?;
    validate_m66_event_prefix_aliases(jp_rom_path, assets_dir)?;
    validate_m67_spell_command_table(&source)?;
    validate_m68_item_event_boundaries(&source, assets_dir)?;
    validate_m69_enemy_hp_boundaries(&source, assets_dir)?;
    validate_m70_enemy_damage_boundaries(&source, assets_dir)?;
    validate_m72_enemy_status_consumers(&source)?;
    validate_m75_player_damage_path_classification(&source)?;

    let text_specs = m70_text_specs();
    let baseline = build_text_poc_internal(
        jp_rom_path,
        assets_dir,
        &text_specs,
        TextBuildScope {
            include_item_names: true,
            include_item_quoted: true,
            include_item_desc: true,
            include_item_use: true,
            include_item_use2: true,
            extra_glyphs: &M65_KR_EXTRA_GLYPHS,
            milestone: "M70",
        },
    )?;
    let output = apply_m60_monster_names(&source, baseline, assets_dir, &M65_KR_EXTRA_GLYPHS)?;
    validate_m66_event_prefix_redirects(&source, &output, &text_specs)?;
    validate_m67_spell_command_result(&source, &output)?;
    Ok(output)
}

pub(crate) fn m70_text_specs() -> Vec<StableTextSpec> {
    let mut text_specs = m69_text_specs();
    text_specs.extend_from_slice(&M70_ENEMY_DAMAGE_TEXT_SPECS);
    text_specs
}
