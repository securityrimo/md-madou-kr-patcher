//! Source-owned graphics-text inventory and authored-master validation.
//!
//! This module validates only repository inputs. The ignored JP-to-EN tool
//! checkout and its extracted PNGs are discovery references, never canonical
//! build dependencies.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::jp_native;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSummary {
    pub target_blocks: usize,
    pub excluded_or_separate_blocks: usize,
    pub unknown_blocks: usize,
    pub method_counts: BTreeMap<String, usize>,
    pub state_counts: BTreeMap<String, usize>,
    pub imagegen_targets: usize,
    pub imagegen_masters: usize,
}

#[derive(Debug, Deserialize)]
struct Catalog {
    scope: Scope,
    families: Vec<Family>,
    excluded_or_separate: Vec<ExcludedBlock>,
}

#[derive(Debug, Deserialize)]
struct Scope {
    source_rom_sha256: String,
    target_blocks: usize,
    excluded_or_separate_blocks: usize,
    unknown_blocks: usize,
}

#[derive(Debug, Deserialize)]
struct Family {
    blocks: Vec<Block>,
}

#[derive(Debug, Deserialize)]
struct Block {
    id: String,
    method: String,
    translation_state: String,
    #[serde(default)]
    jp_text: Option<String>,
    #[serde(default)]
    kr_text: Option<String>,
    #[serde(default)]
    master_asset: Option<String>,
    #[serde(default)]
    pixel_fit_asset: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExcludedBlock {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ImagegenManifest {
    masters: Vec<ImagegenMaster>,
}

#[derive(Debug, Deserialize)]
struct ImagegenMaster {
    master_id: String,
    target_ids: Vec<String>,
    master_asset: String,
    master_sha256: String,
    #[serde(default)]
    pixel_fit_asset: Option<String>,
    #[serde(default)]
    pixel_fit_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TitleMenuTextManifest {
    entries: Vec<TitleMenuTextEntry>,
}

#[derive(Debug, Deserialize)]
struct TitleMenuTextEntry {
    id: String,
    ko: String,
}

#[derive(Debug, Deserialize)]
struct TitlePromptTextManifest {
    logical_prompts: Vec<TitleMenuTextEntry>,
}

#[derive(Debug, Deserialize)]
struct ExamCardTextManifest {
    blocks: Vec<ExamCardTextEntry>,
}

#[derive(Debug, Deserialize)]
struct ExamCardTextEntry {
    id: String,
    ko: String,
}

#[derive(Debug, Deserialize)]
struct PanottyWahTextManifest {
    fueen: PanottyFueenTextEntry,
}

#[derive(Debug, Deserialize)]
struct PanottyFueenTextEntry {
    id: String,
    ko: String,
}

#[derive(Debug, Deserialize)]
struct SingleCodeLayoutTextManifest {
    asset_group_id: String,
    ko: String,
}

#[derive(Debug, Deserialize)]
struct MrFleaTextManifest {
    here: CodeLayoutTextEntry,
    defending: CodeLayoutTextEntry,
    batankyu: CodeLayoutTextEntry,
}

#[derive(Debug, Deserialize)]
struct CodeLayoutTextEntry {
    id: String,
    ko: String,
}

#[derive(Debug, Deserialize)]
struct SpellAnimationTextManifest {
    animations: Vec<CodeLayoutTextEntry>,
}

#[derive(Debug, Deserialize)]
struct KaraokeTextManifest {
    lines: Vec<CodeLayoutTextEntry>,
}

#[derive(Debug, Deserialize)]
struct CreditsTopTextManifest {
    pages: Vec<CodeLayoutTextEntry>,
}

#[derive(Debug, Deserialize)]
struct CreditsTimedTextManifest {
    standalone_ascii_policy: String,
    preserved_complete_latin_names: Vec<String>,
    lines: Vec<CodeLayoutTextEntry>,
}

#[derive(Debug, Deserialize)]
struct CreditsRemainingTextManifest {
    consumers: Vec<CreditsRemainingTextConsumer>,
}

#[derive(Debug, Deserialize)]
struct CreditsRemainingTextConsumer {
    lines: Vec<SourceOwnedTextEntry>,
}

#[derive(Debug, Deserialize)]
struct SourceOwnedTextEntry {
    id: String,
    jp: String,
    ko: String,
}

#[derive(Debug, Deserialize)]
struct TimerRemainingTextManifest {
    asset_group_id: String,
    jp: String,
    ko: String,
}

#[derive(Debug, Deserialize)]
struct EscapeDoorTextManifest {
    marks: Vec<SourceOwnedTextEntry>,
}

/// Validate the closed graphics denominator and every committed imagegen
/// master before a canonical JP-to-KR build starts.
pub fn validate(assets_dir: &Path) -> Result<CatalogSummary, String> {
    let catalog_path = assets_dir.join("graphics_text/catalog.json");
    let manifest_path = assets_dir.join("graphics_text/imagegen/manifest.json");
    let catalog: Catalog = read_json(&catalog_path)?;
    let manifest: ImagegenManifest = read_json(&manifest_path)?;

    let supported_jp_sha256 = jp_native::supported_jp_sha256_hex();
    if catalog.scope.source_rom_sha256 != supported_jp_sha256 {
        return Err(format!(
            "{}: graphics catalog JP SHA-256 drifted: expected {supported_jp_sha256}, got {}",
            catalog_path.display(),
            catalog.scope.source_rom_sha256
        ));
    }
    if catalog.scope.unknown_blocks != 0 {
        return Err(format!(
            "{}: graphics catalog is not closed: {} unknown blocks remain",
            catalog_path.display(),
            catalog.scope.unknown_blocks
        ));
    }

    let mut target_ids = BTreeSet::new();
    let mut method_counts = BTreeMap::new();
    let mut state_counts = BTreeMap::new();
    let mut catalog_jp_text_by_target = BTreeMap::new();
    let mut catalog_kr_text_by_target = BTreeMap::new();
    let mut catalog_master_by_target = BTreeMap::new();
    let mut catalog_pixel_fit_by_target = BTreeMap::new();
    for block in catalog.families.iter().flat_map(|family| &family.blocks) {
        if !target_ids.insert(block.id.clone()) {
            return Err(format!(
                "{}: duplicate graphics target ID {}",
                catalog_path.display(),
                block.id
            ));
        }
        if !matches!(
            block.method.as_str(),
            "code_layout" | "preserve_edit" | "imagegen_edit_then_pixel"
        ) {
            return Err(format!(
                "{}: unsupported production method {} on {}",
                catalog_path.display(),
                block.method,
                block.id
            ));
        }
        *method_counts.entry(block.method.clone()).or_insert(0) += 1;
        if !matches!(
            block.translation_state.as_str(),
            "draft"
                | "draft_timing_pending"
                | "master_generated_pending_pixel_fit"
                | "needs_jp_transcription"
                | "needs_jp_transcription_check"
                | "rom_integrated_controlled_runtime_passed"
                | "rom_integrated_pending_conditioned_runtime"
                | "rom_integrated_pending_natural_runtime"
                | "rom_integrated_runtime_unverified"
                | "rom_integrated_runtime_passed"
        ) {
            return Err(format!(
                "{}: unsupported translation state {} on {}",
                catalog_path.display(),
                block.translation_state,
                block.id
            ));
        }
        *state_counts
            .entry(block.translation_state.clone())
            .or_insert(0) += 1;
        if let Some(jp_text) = &block.jp_text {
            catalog_jp_text_by_target.insert(block.id.clone(), jp_text.clone());
        }
        if let Some(kr_text) = &block.kr_text {
            catalog_kr_text_by_target.insert(block.id.clone(), kr_text.clone());
        }
        match (&*block.method, &block.master_asset) {
            ("imagegen_edit_then_pixel", Some(master_asset)) => {
                catalog_master_by_target.insert(block.id.clone(), master_asset.clone());
                if let Some(pixel_fit_asset) = &block.pixel_fit_asset {
                    catalog_pixel_fit_by_target.insert(block.id.clone(), pixel_fit_asset.clone());
                }
            }
            ("imagegen_edit_then_pixel", None) => {
                return Err(format!(
                    "{}: imagegen target {} has no committed master_asset",
                    catalog_path.display(),
                    block.id
                ));
            }
            (_, Some(_)) => {
                return Err(format!(
                    "{}: non-imagegen target {} unexpectedly declares master_asset",
                    catalog_path.display(),
                    block.id
                ));
            }
            (_, None) => {}
        }
        if block.method != "imagegen_edit_then_pixel" && block.pixel_fit_asset.is_some() {
            return Err(format!(
                "{}: non-imagegen target {} unexpectedly declares pixel_fit_asset",
                catalog_path.display(),
                block.id
            ));
        }
    }

    let mut excluded_ids = BTreeSet::new();
    for block in &catalog.excluded_or_separate {
        if !excluded_ids.insert(block.id.clone()) {
            return Err(format!(
                "{}: duplicate excluded/separate graphics ID {}",
                catalog_path.display(),
                block.id
            ));
        }
        if target_ids.contains(&block.id) {
            return Err(format!(
                "{}: graphics ID {} is both a target and excluded/separate",
                catalog_path.display(),
                block.id
            ));
        }
    }

    if target_ids.len() != catalog.scope.target_blocks {
        return Err(format!(
            "{}: declared {} target blocks but found {}",
            catalog_path.display(),
            catalog.scope.target_blocks,
            target_ids.len()
        ));
    }
    if excluded_ids.len() != catalog.scope.excluded_or_separate_blocks {
        return Err(format!(
            "{}: declared {} excluded/separate blocks but found {}",
            catalog_path.display(),
            catalog.scope.excluded_or_separate_blocks,
            excluded_ids.len()
        ));
    }

    validate_title_text_manifests(assets_dir, &catalog_kr_text_by_target)?;
    validate_exam_card_text_manifest(assets_dir, &catalog_kr_text_by_target)?;
    validate_panotty_fueen_text_manifest(assets_dir, &catalog_kr_text_by_target)?;
    validate_fixed_effect_text_manifests(assets_dir, &catalog_kr_text_by_target)?;
    validate_spell_animation_text_manifest(assets_dir, &catalog_kr_text_by_target)?;
    validate_karaoke_text_manifest(assets_dir, &catalog_kr_text_by_target)?;
    validate_credits_top_text_manifest(assets_dir, &catalog_kr_text_by_target)?;
    validate_credits_timed_text_manifest(assets_dir, &catalog_kr_text_by_target)?;
    validate_credits_generic_text_manifest(assets_dir, &catalog_kr_text_by_target)?;
    validate_credits_native_frames_text_manifest(assets_dir, &catalog_kr_text_by_target)?;
    validate_credits_remaining_text_manifest(
        assets_dir,
        &catalog_jp_text_by_target,
        &catalog_kr_text_by_target,
    )?;
    validate_timer_remaining_text_manifest(
        assets_dir,
        &catalog_jp_text_by_target,
        &catalog_kr_text_by_target,
    )?;
    validate_escape_door_text_manifest(
        assets_dir,
        &catalog_jp_text_by_target,
        &catalog_kr_text_by_target,
    )?;

    let mut manifest_master_ids = BTreeSet::new();
    let mut manifest_targets = BTreeSet::new();
    for master in &manifest.masters {
        if !manifest_master_ids.insert(master.master_id.clone()) {
            return Err(format!(
                "{}: duplicate imagegen master ID {}",
                manifest_path.display(),
                master.master_id
            ));
        }
        if master.target_ids.is_empty() {
            return Err(format!(
                "{}: imagegen master {} has no target IDs",
                manifest_path.display(),
                master.master_id
            ));
        }
        let master_path = assets_dir.join(&master.master_asset);
        validate_master_hash(&master_path, &master.master_sha256)?;
        match (&master.pixel_fit_asset, &master.pixel_fit_sha256) {
            (Some(pixel_fit_asset), Some(pixel_fit_sha256)) => {
                validate_master_hash(&assets_dir.join(pixel_fit_asset), pixel_fit_sha256)?;
            }
            (None, None) => {}
            _ => {
                return Err(format!(
                    "{}: imagegen master {} must declare pixel-fit asset and SHA-256 together",
                    manifest_path.display(),
                    master.master_id
                ));
            }
        }
        for target_id in &master.target_ids {
            if !manifest_targets.insert(target_id.clone()) {
                return Err(format!(
                    "{}: imagegen target {} is assigned to multiple masters",
                    manifest_path.display(),
                    target_id
                ));
            }
            let catalog_asset = catalog_master_by_target.get(target_id).ok_or_else(|| {
                format!(
                    "{}: imagegen manifest target {} is absent from the catalog imagegen set",
                    manifest_path.display(),
                    target_id
                )
            })?;
            if catalog_asset != &master.master_asset {
                return Err(format!(
                    "{}: target {} maps to {} in catalog but {} in manifest",
                    manifest_path.display(),
                    target_id,
                    catalog_asset,
                    master.master_asset
                ));
            }
            let catalog_pixel_fit = catalog_pixel_fit_by_target.get(target_id);
            if catalog_pixel_fit != master.pixel_fit_asset.as_ref() {
                return Err(format!(
                    "{}: target {} pixel fit maps to {:?} in catalog but {:?} in manifest",
                    manifest_path.display(),
                    target_id,
                    catalog_pixel_fit,
                    master.pixel_fit_asset
                ));
            }
        }
    }

    let expected_imagegen_targets: BTreeSet<_> = catalog_master_by_target.keys().cloned().collect();
    if manifest_targets != expected_imagegen_targets {
        let missing: Vec<_> = expected_imagegen_targets
            .difference(&manifest_targets)
            .cloned()
            .collect();
        let extra: Vec<_> = manifest_targets
            .difference(&expected_imagegen_targets)
            .cloned()
            .collect();
        return Err(format!(
            "{}: imagegen target coverage drifted; missing={missing:?}, extra={extra:?}",
            manifest_path.display()
        ));
    }

    Ok(CatalogSummary {
        target_blocks: target_ids.len(),
        excluded_or_separate_blocks: excluded_ids.len(),
        unknown_blocks: catalog.scope.unknown_blocks,
        method_counts,
        state_counts,
        imagegen_targets: expected_imagegen_targets.len(),
        imagegen_masters: manifest_master_ids.len(),
    })
}

fn validate_title_text_manifests(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let menu_path = assets_dir.join("graphics_text/title_menu.json");
    let prompt_path = assets_dir.join("graphics_text/title_prompt.json");
    let menu: TitleMenuTextManifest = read_json(&menu_path)?;
    let prompt: TitlePromptTextManifest = read_json(&prompt_path)?;
    let mut manifest_ids = BTreeSet::new();
    for (path, entry) in menu.entries.iter().map(|entry| (&menu_path, entry)).chain(
        prompt
            .logical_prompts
            .iter()
            .map(|entry| (&prompt_path, entry)),
    ) {
        if !manifest_ids.insert(entry.id.clone()) {
            return Err(format!(
                "{}: duplicate source-owned title text ID {}",
                path.display(),
                entry.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
            format!(
                "{}: source-owned title text {} has no catalog kr_text",
                path.display(),
                entry.id
            )
        })?;
        if catalog_text != &entry.ko {
            return Err(format!(
                "{}: source-owned title text {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                entry.id,
                entry.ko,
                catalog_text
            ));
        }
    }
    if manifest_ids.len() != 12 {
        return Err(format!(
            "source-owned title text manifests contain {} logical entries, expected 12",
            manifest_ids.len()
        ));
    }
    Ok(())
}

fn validate_exam_card_text_manifest(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/exam_card.json");
    let manifest: ExamCardTextManifest = read_json(&path)?;
    let expected_ids = BTreeSet::from([
        "GFX-EXAM-CARD-HEADING",
        "GFX-EXAM-CARD-RESULT",
        "GFX-EXAM-CARD-SCORE-STEM",
        "GFX-EXAM-CARD-COPULA",
    ]);
    let mut actual_ids = BTreeSet::new();
    for block in &manifest.blocks {
        if !actual_ids.insert(block.id.as_str()) {
            return Err(format!(
                "{}: duplicate source-owned exam card text ID {}",
                path.display(),
                block.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&block.id).ok_or_else(|| {
            format!(
                "{}: source-owned exam card text {} has no catalog kr_text",
                path.display(),
                block.id
            )
        })?;
        if catalog_text != &block.ko {
            return Err(format!(
                "{}: source-owned exam card text {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                block.id,
                block.ko,
                catalog_text
            ));
        }
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned exam card text coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_panotty_fueen_text_manifest(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/panotty_wah.json");
    let manifest: PanottyWahTextManifest = read_json(&path)?;
    if manifest.fueen.id != "GFX-BATTLE-PANOTTY-FUEEN" {
        return Err(format!(
            "{}: source-owned Panotty fueen ID drifted to {}",
            path.display(),
            manifest.fueen.id
        ));
    }
    let catalog_text = catalog_kr_text_by_target
        .get(&manifest.fueen.id)
        .ok_or_else(|| {
            format!(
                "{}: source-owned Panotty fueen {} has no catalog kr_text",
                path.display(),
                manifest.fueen.id
            )
        })?;
    if catalog_text != &manifest.fueen.ko {
        return Err(format!(
            "{}: source-owned Panotty fueen text is {:?}, but catalog kr_text is {:?}",
            path.display(),
            manifest.fueen.ko,
            catalog_text
        ));
    }
    Ok(())
}

fn validate_fixed_effect_text_manifests(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let jin_path = assets_dir.join("graphics_text/bayoen_jin.json");
    let demon_path = assets_dir.join("graphics_text/demon_byun.json");
    let bechi_path = assets_dir.join("graphics_text/intro_bechi.json");
    let flea_path = assets_dir.join("graphics_text/mr_flea.json");
    let jin: SingleCodeLayoutTextManifest = read_json(&jin_path)?;
    let demon: SingleCodeLayoutTextManifest = read_json(&demon_path)?;
    let bechi: SingleCodeLayoutTextManifest = read_json(&bechi_path)?;
    let flea: MrFleaTextManifest = read_json(&flea_path)?;
    let entries = [
        (
            &bechi_path,
            CodeLayoutTextEntry {
                id: bechi.asset_group_id,
                ko: bechi.ko,
            },
        ),
        (
            &jin_path,
            CodeLayoutTextEntry {
                id: jin.asset_group_id,
                ko: jin.ko,
            },
        ),
        (
            &flea_path,
            CodeLayoutTextEntry {
                id: flea.here.id,
                ko: flea.here.ko,
            },
        ),
        (
            &flea_path,
            CodeLayoutTextEntry {
                id: flea.defending.id,
                ko: flea.defending.ko,
            },
        ),
        (
            &flea_path,
            CodeLayoutTextEntry {
                id: flea.batankyu.id,
                ko: flea.batankyu.ko,
            },
        ),
        (
            &demon_path,
            CodeLayoutTextEntry {
                id: demon.asset_group_id,
                ko: demon.ko,
            },
        ),
    ];
    let mut ids = BTreeSet::new();
    for (path, entry) in entries {
        if !ids.insert(entry.id.clone()) {
            return Err(format!(
                "{}: duplicate source-owned fixed-effect ID {}",
                path.display(),
                entry.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
            format!(
                "{}: source-owned fixed effect {} has no catalog kr_text",
                path.display(),
                entry.id
            )
        })?;
        if catalog_text != &entry.ko {
            return Err(format!(
                "{}: source-owned fixed effect {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                entry.id,
                entry.ko,
                catalog_text
            ));
        }
    }
    Ok(())
}

fn validate_spell_animation_text_manifest(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/spell_animations.json");
    let manifest: SpellAnimationTextManifest = read_json(&path)?;
    let expected_ids = BTreeSet::from(["GFX-SPELL-BAYOEN", "GFX-SPELL-COCKADOODLE"]);
    let mut actual_ids = BTreeSet::new();
    for entry in &manifest.animations {
        if !actual_ids.insert(entry.id.as_str()) {
            return Err(format!(
                "{}: duplicate source-owned spell-animation ID {}",
                path.display(),
                entry.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
            format!(
                "{}: source-owned spell animation {} has no catalog kr_text",
                path.display(),
                entry.id
            )
        })?;
        if catalog_text != &entry.ko {
            return Err(format!(
                "{}: source-owned spell animation {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                entry.id,
                entry.ko,
                catalog_text
            ));
        }
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned spell-animation coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_karaoke_text_manifest(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/karaoke.json");
    let manifest: KaraokeTextManifest = read_json(&path)?;
    let expected_ids = (0..9)
        .map(|index| format!("GFX-KARAOKE-LINE-{index:02}"))
        .collect::<BTreeSet<_>>();
    let mut actual_ids = BTreeSet::new();
    for entry in &manifest.lines {
        if !actual_ids.insert(entry.id.clone()) {
            return Err(format!(
                "{}: duplicate source-owned karaoke ID {}",
                path.display(),
                entry.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
            format!(
                "{}: source-owned karaoke {} has no catalog kr_text",
                path.display(),
                entry.id
            )
        })?;
        if catalog_text != &entry.ko {
            return Err(format!(
                "{}: source-owned karaoke {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                entry.id,
                entry.ko,
                catalog_text
            ));
        }
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned karaoke coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_credits_top_text_manifest(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/credits_top.json");
    let manifest: CreditsTopTextManifest = read_json(&path)?;
    let expected_ids = (0..10)
        .map(|index| format!("GFX-CREDITS-P{index:02}-TOP"))
        .collect::<BTreeSet<_>>();
    let mut actual_ids = BTreeSet::new();
    for entry in &manifest.pages {
        if !actual_ids.insert(entry.id.clone()) {
            return Err(format!(
                "{}: duplicate source-owned credit-heading ID {}",
                path.display(),
                entry.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
            format!(
                "{}: source-owned credit heading {} has no catalog kr_text",
                path.display(),
                entry.id
            )
        })?;
        if catalog_text != &entry.ko {
            return Err(format!(
                "{}: source-owned credit heading {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                entry.id,
                entry.ko,
                catalog_text
            ));
        }
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned credit-heading coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_credits_timed_text_manifest(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/credits_timed_cells.json");
    let manifest: CreditsTimedTextManifest = read_json(&path)?;
    let expected_ids = BTreeSet::from([
        "GFX-CREDITS-P01-L01",
        "GFX-CREDITS-P02-L01",
        "GFX-CREDITS-P02-L02",
        "GFX-CREDITS-P03-L01",
        "GFX-CREDITS-P03-L02",
        "GFX-CREDITS-P05-L01",
        "GFX-CREDITS-P05-L02",
    ]);
    if manifest.standalone_ascii_policy
        != "preserve_unmapped_standalone_ascii_escape_doors_owned_by_escape_doors_manifest"
        || manifest.preserved_complete_latin_names != ["Jemini", "RIU", "TAKIN"]
    {
        return Err(format!(
            "{}: timed-credit ASCII preservation policy drifted",
            path.display()
        ));
    }
    let mut actual_ids = BTreeSet::new();
    for entry in &manifest.lines {
        if !actual_ids.insert(entry.id.as_str()) {
            return Err(format!(
                "{}: duplicate source-owned timed-credit ID {}",
                path.display(),
                entry.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
            format!(
                "{}: source-owned timed credit {} has no catalog kr_text",
                path.display(),
                entry.id
            )
        })?;
        if catalog_text != &entry.ko {
            return Err(format!(
                "{}: source-owned timed credit {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                entry.id,
                entry.ko,
                catalog_text
            ));
        }
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned timed-credit coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_credits_generic_text_manifest(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/credits_generic_frames.json");
    let manifest: CreditsTimedTextManifest = read_json(&path)?;
    let expected_ids = BTreeSet::from(["GFX-CREDITS-P04-L01", "GFX-CREDITS-P08-L01"]);
    if manifest.standalone_ascii_policy
        != "preserve_unmapped_standalone_ascii_escape_doors_owned_by_escape_doors_manifest"
        || manifest.preserved_complete_latin_names != ["Jemini", "RIU", "TAKIN"]
    {
        return Err(format!(
            "{}: generic-credit ASCII preservation policy drifted",
            path.display()
        ));
    }
    let mut actual_ids = BTreeSet::new();
    for entry in &manifest.lines {
        if !actual_ids.insert(entry.id.as_str()) {
            return Err(format!(
                "{}: duplicate source-owned generic-credit ID {}",
                path.display(),
                entry.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
            format!(
                "{}: source-owned generic credit {} has no catalog kr_text",
                path.display(),
                entry.id
            )
        })?;
        if catalog_text != &entry.ko {
            return Err(format!(
                "{}: source-owned generic credit {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                entry.id,
                entry.ko,
                catalog_text
            ));
        }
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned generic-credit coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_credits_native_frames_text_manifest(
    assets_dir: &Path,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/credits_native_frames.json");
    let manifest: CreditsTimedTextManifest = read_json(&path)?;
    let expected_ids = BTreeSet::from([
        "GFX-CREDITS-P07-L01",
        "GFX-CREDITS-P07-L05",
        "GFX-CREDITS-P09-L07",
    ]);
    if manifest.standalone_ascii_policy
        != "preserve_unmapped_standalone_ascii_escape_doors_owned_by_escape_doors_manifest"
        || manifest.preserved_complete_latin_names != ["Jemini", "RIU", "TAKIN"]
    {
        return Err(format!(
            "{}: native-frame credit ASCII preservation policy drifted",
            path.display()
        ));
    }
    let mut actual_ids = BTreeSet::new();
    for entry in &manifest.lines {
        if !actual_ids.insert(entry.id.as_str()) {
            return Err(format!(
                "{}: duplicate source-owned native-frame credit ID {}",
                path.display(),
                entry.id
            ));
        }
        let catalog_text = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
            format!(
                "{}: source-owned native-frame credit {} has no catalog kr_text",
                path.display(),
                entry.id
            )
        })?;
        if catalog_text != &entry.ko {
            return Err(format!(
                "{}: source-owned native-frame credit {} is {:?}, but catalog kr_text is {:?}",
                path.display(),
                entry.id,
                entry.ko,
                catalog_text
            ));
        }
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned native-frame credit coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_credits_remaining_text_manifest(
    assets_dir: &Path,
    catalog_jp_text_by_target: &BTreeMap<String, String>,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/credits_remaining.json");
    let manifest: CreditsRemainingTextManifest = read_json(&path)?;
    let expected_ids = BTreeSet::from([
        "GFX-CREDITS-P00-L01",
        "GFX-CREDITS-P07-L02",
        "GFX-CREDITS-P07-L03",
        "GFX-CREDITS-P07-L04",
        "GFX-CREDITS-P09-L01",
        "GFX-CREDITS-P09-L02",
        "GFX-CREDITS-P09-L03",
        "GFX-CREDITS-P09-L04",
        "GFX-CREDITS-P09-L05",
        "GFX-CREDITS-P09-L06",
    ]);
    let mut actual_ids = BTreeSet::new();
    for entry in manifest
        .consumers
        .iter()
        .flat_map(|consumer| &consumer.lines)
    {
        if !actual_ids.insert(entry.id.as_str()) {
            return Err(format!(
                "{}: duplicate source-owned remaining-credit ID {}",
                path.display(),
                entry.id
            ));
        }
        validate_catalog_text(
            &path,
            entry,
            catalog_jp_text_by_target,
            catalog_kr_text_by_target,
        )?;
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned remaining-credit coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_timer_remaining_text_manifest(
    assets_dir: &Path,
    catalog_jp_text_by_target: &BTreeMap<String, String>,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/timer_remaining.json");
    let manifest: TimerRemainingTextManifest = read_json(&path)?;
    let entry = SourceOwnedTextEntry {
        id: manifest.asset_group_id,
        jp: manifest.jp,
        ko: manifest.ko,
    };
    if entry.id != "GFX-TIMER-REMAINING" {
        return Err(format!(
            "{}: remaining-time source ID drifted to {}",
            path.display(),
            entry.id
        ));
    }
    validate_catalog_text(
        &path,
        &entry,
        catalog_jp_text_by_target,
        catalog_kr_text_by_target,
    )
}

fn validate_escape_door_text_manifest(
    assets_dir: &Path,
    catalog_jp_text_by_target: &BTreeMap<String, String>,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = assets_dir.join("graphics_text/escape_doors.json");
    let manifest: EscapeDoorTextManifest = read_json(&path)?;
    let expected_ids = BTreeSet::from([
        "GFX-ESCAPE-DOOR-HIDON",
        "GFX-ESCAPE-DOOR-JUGEM",
        "GFX-ESCAPE-DOOR-BAYOEN",
    ]);
    let mut actual_ids = BTreeSet::new();
    for mark in &manifest.marks {
        if !actual_ids.insert(mark.id.as_str()) {
            return Err(format!(
                "{}: duplicate source-owned escape-door ID {}",
                path.display(),
                mark.id
            ));
        }
        validate_catalog_text(
            &path,
            mark,
            catalog_jp_text_by_target,
            catalog_kr_text_by_target,
        )?;
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "{}: source-owned escape-door coverage drifted",
            path.display()
        ));
    }
    Ok(())
}

fn validate_catalog_text(
    path: &Path,
    entry: &SourceOwnedTextEntry,
    catalog_jp_text_by_target: &BTreeMap<String, String>,
    catalog_kr_text_by_target: &BTreeMap<String, String>,
) -> Result<(), String> {
    let catalog_jp = catalog_jp_text_by_target.get(&entry.id).ok_or_else(|| {
        format!(
            "{}: source-owned text {} has no catalog jp_text",
            path.display(),
            entry.id
        )
    })?;
    let catalog_kr = catalog_kr_text_by_target.get(&entry.id).ok_or_else(|| {
        format!(
            "{}: source-owned text {} has no catalog kr_text",
            path.display(),
            entry.id
        )
    })?;
    if catalog_jp != &entry.jp || catalog_kr != &entry.ko {
        return Err(format!(
            "{}: source-owned text {} is {:?} -> {:?}, but catalog is {:?} -> {:?}",
            path.display(),
            entry.id,
            entry.jp,
            entry.ko,
            catalog_jp,
            catalog_kr
        ));
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read graphics source {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid graphics source {}: {error}", path.display()))
}

fn validate_master_hash(path: &PathBuf, expected: &str) -> Result<(), String> {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{}: invalid lowercase SHA-256 declaration {expected}",
            path.display()
        ));
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read imagegen master {}: {error}", path.display()))?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(format!("{}: imagegen master is not a PNG", path.display()));
    }
    let actual = sha256_hex(&bytes);
    if actual != expected {
        return Err(format!(
            "{}: imagegen master SHA-256 mismatch: expected {expected}, got {actual}",
            path.display()
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}
