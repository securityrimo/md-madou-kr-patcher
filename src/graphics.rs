//! Deterministic JP-source graphics localization.
//!
//! The first promoted consumer is the title menu. It decodes the original
//! mode-1 pack, preserves every original tile, appends repository-rendered
//! Korean glyphs below the next VRAM transfer, and changes only declared
//! sprite tile indices plus a new source-owned pack in expanded ROM space.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use fontdue::{Font, FontSettings};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::jp_native;

mod bayoen_jin;
pub use bayoen_jin::{BayoenJinSummary, apply_bayoen_jin, write_bayoen_jin_preview};
mod bad_end_gaan;
mod bdf;
pub use bad_end_gaan::{BadEndGaanSummary, apply_bad_end_gaan, write_bad_end_gaan_preview};
mod compile_slogan;
pub use compile_slogan::{
    CompileSloganSummary, apply_compile_slogan, write_compile_slogan_preview,
};
mod credits_generic;
mod credits_native_frames;
mod credits_remaining;
mod credits_timed;
mod credits_top;
pub use credits_top::{
    CreditsTopSummary, apply_credits_top, write_credits_timed_preview, write_credits_top_preview,
};
mod demon_byun;
pub use demon_byun::{DemonByunSummary, apply_demon_byun, write_demon_byun_preview};
mod exam_card;
pub use exam_card::{ExamCardSummary, apply_exam_card, write_exam_card_preview};
mod exam_seals;
pub use exam_seals::{ExamSealsSummary, apply_exam_seals, write_exam_seals_preview};
mod escape_doors;
pub use escape_doors::{EscapeDoorsSummary, apply_escape_doors, write_escape_doors_preview};
mod font_effect;
mod intro_doki;
pub use intro_doki::{IntroDokiSummary, apply_intro_doki, write_intro_doki_preview};
mod intro_bechi;
pub use intro_bechi::{IntroBechiSummary, apply_intro_bechi, write_intro_bechi_preview};
mod intro_pokan;
pub use intro_pokan::{IntroPokanSummary, apply_intro_pokan, write_intro_pokan_preview};
mod karaoke;
pub use karaoke::{KaraokeSummary, apply_karaoke, write_karaoke_preview};
mod mr_flea;
pub use mr_flea::{MrFleaSummary, apply_mr_flea, write_mr_flea_preview};
mod panotty_poka;
pub use panotty_poka::{PanottyPokaSummary, apply_panotty_poka, write_panotty_poka_preview};
mod panotty_wah;
pub use panotty_wah::{
    PanottyWahSummary, apply_panotty_wah, write_panotty_fueen_preview, write_panotty_wah_preview,
};
mod pixel;
mod spell_animations;
mod sprite_map;
pub use spell_animations::{
    SpellAnimationMasterSummary, SpellAnimationsSummary, apply_spell_animations,
    write_spell_animation_font_master, write_spell_animations_preview,
};
mod title_logo;
pub use title_logo::{TitleLogoSummary, apply_title_logo, write_title_logo_preview};
mod title_prompt;
use title_prompt::{CodePatch, build_title_prompt, read_title_prompt_plan};
mod timer_remaining;
pub use timer_remaining::{
    TimerRemainingSummary, apply_timer_remaining, write_timer_remaining_preview,
};

const TITLE_MENU_HEADER_OFFSET: usize = 0x09_2BD4;
const TITLE_MENU_BANK_OFFSET: usize = 0x24_0000;
const TITLE_MENU_VRAM_DESTINATION: u16 = 0x4000;
const TITLE_TILE_ORIGIN: u16 = 0x0200;
const TITLE_ORIGINAL_END_TILE: u16 = 0x0370;
const TITLE_NEXT_TRANSFER_TILE: u16 = 0x0400;
const MD_TILE_BYTES: usize = 32;
const MODE1_SUBHEADER_BYTES: usize = 6;
const MODE1_CHAIN_TERMINATOR: [u8; 2] = [0xFF, 0xFF];
const GLYPH_TILES: u16 = 4;
const GLYPH_BYTES: usize = MD_TILE_BYTES * GLYPH_TILES as usize;
const ORIGINAL_TITLE_PAYLOAD_BYTES: usize =
    (TITLE_ORIGINAL_END_TILE - TITLE_TILE_ORIGIN) as usize * MD_TILE_BYTES;
const ORIGINAL_TITLE_PAYLOAD_SHA256: &str =
    "9d4a3f296b6a728be5e8bcda7830b9cd4ca44db611033a032c8677d379651b91";
const CHECKSUM_OFFSET: usize = 0x018E;
const CHECKSUM_START: usize = 0x0200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleMenuSummary {
    pub entries: usize,
    pub unique_glyphs: usize,
    pub decoded_bytes: usize,
    pub pack_bytes: usize,
    pub first_new_tile: u16,
    pub next_free_tile: u16,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct TitleMenuManifest {
    jp_pack_entry_header_offset: String,
    vram_destination: String,
    numeric_glyphs: String,
    numeric_pointer_table_offset: String,
    numeric_template_structure_offset: String,
    mutable_tile_ranges: Vec<MutableTileRange>,
    source_glyph_reuse: Vec<SourceGlyphReuse>,
    entries: Vec<TitleMenuEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct TitleMenuEntry {
    id: String,
    ko: String,
    structure_offset: String,
    consumer_pointer_offset: String,
    structure_kind: StructureKind,
    slots: usize,
    leading_blank_slots: usize,
    #[serde(default)]
    spaces_consume_slots: bool,
    #[serde(default)]
    target_slot_origin_x: Option<String>,
    #[serde(default)]
    target_visible_right_x: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MutableTileRange {
    start: String,
    end_exclusive: String,
}

#[derive(Debug, Deserialize)]
struct SourceGlyphReuse {
    character: String,
    source_tile: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StructureKind {
    PerSprite,
    CountedSprite,
}

#[derive(Debug)]
struct StructurePatch {
    id: String,
    offset: usize,
    expected: Vec<u8>,
    replacement: Vec<u8>,
}

#[derive(Debug)]
struct TitleMenuBuild {
    preview_entries: Vec<(String, String)>,
    logical_entries: usize,
    payload: Vec<u8>,
    tile_by_char: BTreeMap<char, u16>,
    blank_tile: u16,
    next_free_tile: u16,
    mutable_tile_ranges: Vec<(u16, u16)>,
    header: [u8; 6],
    bank: Vec<u8>,
    structures: Vec<StructurePatch>,
    data_patches: Vec<StructurePatch>,
    code_patches: Vec<CodePatch>,
    prompt_structure_bank_offset: usize,
    prompt_structure_bank: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecodedPack {
    vram_destination: u16,
    data: Vec<u8>,
}

#[derive(Debug)]
struct EncodedPack {
    header: [u8; 6],
    bank: Vec<u8>,
}

/// Apply the first source-owned graphics block to a cumulative JP-to-KR ROM.
pub fn apply_title_menu(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<TitleMenuSummary, String> {
    if output.len() < TITLE_MENU_BANK_OFFSET {
        return Err("cumulative JP-to-KR ROM is too small for graphics bank".to_string());
    }
    let build = build_title_menu(source, assets_dir)?;
    if TITLE_MENU_BANK_OFFSET + build.bank.len() > output.len() {
        return Err(format!(
            "title-menu pack ends outside output at 0x{:06X}",
            TITLE_MENU_BANK_OFFSET + build.bank.len()
        ));
    }
    if build.prompt_structure_bank_offset + build.prompt_structure_bank.len() > output.len() {
        return Err(format!(
            "title-prompt structures end outside output at 0x{:06X}",
            build.prompt_structure_bank_offset + build.prompt_structure_bank.len()
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::new();

    apply_expected_write(
        output,
        TITLE_MENU_HEADER_OFFSET,
        source_range(
            source,
            TITLE_MENU_HEADER_OFFSET,
            build.header.len(),
            "title-menu header",
        )?,
        &build.header,
        "title-menu pack header",
    )?;
    changed_ranges.push((
        TITLE_MENU_HEADER_OFFSET,
        TITLE_MENU_HEADER_OFFSET + build.header.len(),
    ));

    for patch in &build.structures {
        apply_expected_write(
            output,
            patch.offset,
            &patch.expected,
            &patch.replacement,
            &format!("{} sprite structure", patch.id),
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.replacement.len()));
    }

    for patch in &build.data_patches {
        apply_expected_write(
            output,
            patch.offset,
            &patch.expected,
            &patch.replacement,
            &patch.id,
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.replacement.len()));
    }

    for patch in &build.code_patches {
        apply_expected_write(
            output,
            patch.offset,
            &patch.expected,
            &patch.replacement,
            &patch.id,
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.replacement.len()));
    }

    let expected_prompt_structures = vec![0xFF; build.prompt_structure_bank.len()];
    apply_expected_write(
        output,
        build.prompt_structure_bank_offset,
        &expected_prompt_structures,
        &build.prompt_structure_bank,
        "title-prompt expanded structures",
    )?;
    changed_ranges.push((
        build.prompt_structure_bank_offset,
        build.prompt_structure_bank_offset + build.prompt_structure_bank.len(),
    ));

    let expected_pack = vec![0xFF; build.bank.len()];
    apply_expected_write(
        output,
        TITLE_MENU_BANK_OFFSET,
        &expected_pack,
        &build.bank,
        "title-menu expanded graphics pack",
    )?;
    changed_ranges.push((
        TITLE_MENU_BANK_OFFSET,
        TITLE_MENU_BANK_OFFSET + build.bank.len(),
    ));

    let checksum = calculate_checksum(output);
    let checksum_bytes = checksum.to_be_bytes();
    let expected_checksum = baseline
        .get(CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2)
        .ok_or_else(|| "cumulative ROM is too small for checksum".to_string())?;
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        expected_checksum,
        &checksum_bytes,
        "Mega Drive checksum after title-menu graphics",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));

    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let decoded = decode_mode1_pack_entry(output, TITLE_MENU_HEADER_OFFSET)?;
    if decoded.vram_destination != TITLE_MENU_VRAM_DESTINATION || decoded.data != build.payload {
        return Err("inserted title-menu pack does not decode to the planned payload".to_string());
    }
    validate_protected_title_tiles(
        &source_title_payload(source)?,
        &decoded.data,
        &build.mutable_tile_ranges,
    )?;
    let inserted_structures = source_range(
        output,
        build.prompt_structure_bank_offset,
        build.prompt_structure_bank.len(),
        "inserted title-prompt structures",
    )?;
    if inserted_structures != build.prompt_structure_bank {
        return Err("inserted title-prompt structures differ from the plan".to_string());
    }
    for patch in &build.code_patches {
        if source_range(output, patch.offset, patch.replacement.len(), &patch.id)?
            != patch.replacement
        {
            return Err(format!("inserted {} differs from typed output", patch.id));
        }
    }

    eprintln!("JP graphics GFX-TITLE-MENU Expected Writes:");
    eprintln!(
        "  0x{TITLE_MENU_HEADER_OFFSET:06X}..0x{:06X}  title pack header ({} bytes)",
        TITLE_MENU_HEADER_OFFSET + build.header.len(),
        build.header.len()
    );
    for patch in &build.structures {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {} sprite structure ({} bytes)",
            patch.offset,
            patch.offset + patch.replacement.len(),
            patch.id,
            patch.replacement.len()
        );
    }
    for patch in &build.data_patches {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {} ({} bytes, semantic data)",
            patch.offset,
            patch.offset + patch.replacement.len(),
            patch.id,
            patch.replacement.len()
        );
    }
    for patch in &build.code_patches {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {} ({} bytes, typed 68000)",
            patch.offset,
            patch.offset + patch.replacement.len(),
            patch.id,
            patch.replacement.len()
        );
    }
    eprintln!(
        "  0x{:06X}..0x{:06X}  title-prompt structures ({} bytes)",
        build.prompt_structure_bank_offset,
        build.prompt_structure_bank_offset + build.prompt_structure_bank.len(),
        build.prompt_structure_bank.len()
    );
    eprintln!(
        "  0x{TITLE_MENU_BANK_OFFSET:06X}..0x{:06X}  title-menu pack ({} bytes)",
        TITLE_MENU_BANK_OFFSET + build.bank.len(),
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(TitleMenuSummary {
        entries: build.logical_entries,
        unique_glyphs: build.tile_by_char.len(),
        decoded_bytes: build.payload.len(),
        pack_bytes: build.bank.len(),
        first_new_tile: build.blank_tile,
        next_free_tile: build.next_free_tile,
        checksum,
    })
}

/// Write a static, source-derived QA preview. This is not runtime proof.
pub fn write_title_menu_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<TitleMenuSummary, String> {
    let build = build_title_menu(source, assets_dir)?;
    let scale = 3usize;
    let width = build
        .preview_entries
        .iter()
        .map(|(_, text)| 40 + text.chars().count() * 16 * scale)
        .max()
        .unwrap_or(720)
        .max(720);
    let row_height = 48usize;
    let height = 16 + build.preview_entries.len() * row_height;
    let mut rgba = vec![0u8; width * height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[52, 0, 28, 255]);
    }

    for (row, (id, text)) in build.preview_entries.iter().enumerate() {
        let mut cursor_x = 20usize;
        let cursor_y = 12 + row * row_height;
        for ch in text.chars() {
            if ch.is_whitespace() {
                cursor_x += 16 * scale;
                continue;
            }
            let tile = *build
                .tile_by_char
                .get(&ch)
                .ok_or_else(|| format!("{id}: preview glyph {ch:?} is not allocated"))?;
            for glyph_y in 0..16usize {
                for glyph_x in 0..16usize {
                    if md_glyph_pixel(&build.payload, tile, glyph_x, glyph_y)? == 0 {
                        continue;
                    }
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = cursor_x + glyph_x * scale + dx;
                            let y = cursor_y + glyph_y * scale + dy;
                            if x < width && y < height {
                                let offset = (y * width + x) * 4;
                                rgba[offset..offset + 4].copy_from_slice(&[247, 225, 154, 255]);
                            }
                        }
                    }
                }
            }
            cursor_x += 16 * scale;
        }
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create preview directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let file = fs::File::create(output_path).map_err(|error| {
        format!(
            "failed to create title-menu preview {}: {error}",
            output_path.display()
        )
    })?;
    let mut encoder = png::Encoder::new(file, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to write PNG header: {error}"))?;
    writer
        .write_image_data(&rgba)
        .map_err(|error| format!("failed to write PNG pixels: {error}"))?;

    Ok(TitleMenuSummary {
        entries: build.logical_entries,
        unique_glyphs: build.tile_by_char.len(),
        decoded_bytes: build.payload.len(),
        pack_bytes: build.bank.len(),
        first_new_tile: build.blank_tile,
        next_free_tile: build.next_free_tile,
        checksum: 0,
    })
}

fn build_title_menu(source: &[u8], assets_dir: &Path) -> Result<TitleMenuBuild, String> {
    let manifest = read_title_manifest(assets_dir)?;
    let prompt_plan = read_title_prompt_plan(assets_dir)?;
    let header_offset = parse_hex(&manifest.jp_pack_entry_header_offset)?;
    if header_offset != TITLE_MENU_HEADER_OFFSET {
        return Err(format!(
            "title-menu header drifted: expected 0x{TITLE_MENU_HEADER_OFFSET:06X}, got 0x{header_offset:06X}"
        ));
    }
    let vram_destination = parse_hex(&manifest.vram_destination)?;
    if vram_destination != TITLE_MENU_VRAM_DESTINATION as usize {
        return Err(format!(
            "title-menu VRAM destination drifted: expected 0x{TITLE_MENU_VRAM_DESTINATION:04X}, got 0x{vram_destination:04X}"
        ));
    }

    let original_payload = source_title_payload(source)?;
    validate_noop_roundtrip(&original_payload)?;

    let mutable_tile_ranges = parse_mutable_tile_ranges(&manifest.mutable_tile_ranges)?;
    let (tile_by_char, blank_tile, next_free_tile) = allocate_title_tiles(
        &manifest.entries,
        &prompt_plan,
        &manifest.numeric_glyphs,
        &mutable_tile_ranges,
    )?;
    if next_free_tile > TITLE_NEXT_TRANSFER_TILE {
        return Err(format!(
            "title-menu Korean tiles end at 0x{next_free_tile:03X}, overlapping next transfer 0x{TITLE_NEXT_TRANSFER_TILE:03X}"
        ));
    }

    let source_glyphs = read_source_glyphs(&original_payload, &manifest.source_glyph_reuse)?;
    let mut payload = original_payload;
    let planned_len = ORIGINAL_TITLE_PAYLOAD_BYTES
        .max((next_free_tile - TITLE_TILE_ORIGIN) as usize * MD_TILE_BYTES);
    payload.resize(planned_len, 0);
    let blank_offset = (blank_tile - TITLE_TILE_ORIGIN) as usize * MD_TILE_BYTES;
    payload[blank_offset..blank_offset + GLYPH_BYTES].fill(0);

    let font_path = assets_dir.join("neodgm.ttf");
    let ttf_data = fs::read(&font_path).map_err(|error| {
        format!(
            "failed to read title-menu font {}: {error}",
            font_path.display()
        )
    })?;
    let font = Font::from_bytes(ttf_data, FontSettings::default())
        .map_err(|error| format!("failed to parse title-menu font: {error}"))?;
    for (&ch, &tile) in &tile_by_char {
        let md_glyph = if let Some(source_glyph) = source_glyphs.get(&ch) {
            *source_glyph
        } else {
            let glyph = jp_native::render_native_glyph(&font, ch);
            native_glyph_to_md_tiles(&glyph)
        };
        let offset = (tile - TITLE_TILE_ORIGIN) as usize * MD_TILE_BYTES;
        payload[offset..offset + GLYPH_BYTES].copy_from_slice(&md_glyph);
    }

    validate_protected_title_tiles(
        &source_title_payload(source)?,
        &payload,
        &mutable_tile_ranges,
    )?;

    let structures = build_structure_patches(source, &manifest.entries, &tile_by_char, blank_tile)?;
    let prompt = build_title_prompt(source, &prompt_plan, &tile_by_char, blank_tile)?;
    let mut prompt_structure_bank = prompt.structure_bank;
    let numeric_structure_offset = prompt.structure_bank_offset + prompt_structure_bank.len();
    let (numeric_structures, numeric_pointer_patch) =
        build_numeric_structures(source, &manifest, &tile_by_char, numeric_structure_offset)?;
    prompt_structure_bank.extend_from_slice(&numeric_structures);
    let encoded = encode_locked_mode1_pack(
        TITLE_MENU_BANK_OFFSET,
        TITLE_MENU_VRAM_DESTINATION,
        &payload,
    )?;

    let mut preview_entries: Vec<_> = manifest
        .entries
        .iter()
        .map(|entry| (entry.id.clone(), entry.ko.clone()))
        .collect();
    preview_entries.extend(
        prompt_plan
            .logical_prompts
            .iter()
            .map(|prompt| (prompt.id.clone(), prompt.ko.clone())),
    );
    let logical_entries = preview_entries.len();
    preview_entries.push((
        "GFX-TITLE-SOUND-NUMBER-ATLAS".to_string(),
        manifest.numeric_glyphs.clone(),
    ));

    Ok(TitleMenuBuild {
        preview_entries,
        logical_entries,
        payload,
        tile_by_char,
        blank_tile,
        next_free_tile,
        mutable_tile_ranges,
        header: encoded.header,
        bank: encoded.bank,
        structures,
        data_patches: vec![numeric_pointer_patch],
        code_patches: prompt.code_patches,
        prompt_structure_bank_offset: prompt.structure_bank_offset,
        prompt_structure_bank,
    })
}

fn read_title_manifest(assets_dir: &Path) -> Result<TitleMenuManifest, String> {
    let path = assets_dir.join("graphics_text/title_menu.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read title-menu source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid title-menu source {}: {error}", path.display()))
}

fn source_title_payload(source: &[u8]) -> Result<Vec<u8>, String> {
    let decoded = decode_mode1_pack_entry(source, TITLE_MENU_HEADER_OFFSET)?;
    if decoded.vram_destination != TITLE_MENU_VRAM_DESTINATION {
        return Err(format!(
            "JP title-menu pack targets VRAM 0x{:04X}, expected 0x{TITLE_MENU_VRAM_DESTINATION:04X}",
            decoded.vram_destination
        ));
    }
    if decoded.data.len() != ORIGINAL_TITLE_PAYLOAD_BYTES {
        return Err(format!(
            "JP title-menu payload is {} bytes, expected {ORIGINAL_TITLE_PAYLOAD_BYTES}",
            decoded.data.len()
        ));
    }
    let actual_hash = sha256_hex(&decoded.data);
    if actual_hash != ORIGINAL_TITLE_PAYLOAD_SHA256 {
        return Err(format!(
            "JP title-menu payload SHA-256 mismatch: expected {ORIGINAL_TITLE_PAYLOAD_SHA256}, got {actual_hash}"
        ));
    }
    Ok(decoded.data)
}

fn parse_mutable_tile_ranges(sources: &[MutableTileRange]) -> Result<Vec<(u16, u16)>, String> {
    if sources.is_empty() {
        return Err("title-menu manifest has no mutable tile ranges".to_string());
    }
    let mut ranges = Vec::with_capacity(sources.len());
    for source in sources {
        let start = u16::try_from(parse_hex(&source.start)?)
            .map_err(|_| format!("mutable tile start {} exceeds u16", source.start))?;
        let end = u16::try_from(parse_hex(&source.end_exclusive)?)
            .map_err(|_| format!("mutable tile end {} exceeds u16", source.end_exclusive))?;
        if start < TITLE_TILE_ORIGIN
            || end > TITLE_NEXT_TRANSFER_TILE
            || start >= end
            || !start.is_multiple_of(GLYPH_TILES)
            || !end.is_multiple_of(GLYPH_TILES)
        {
            return Err(format!(
                "invalid mutable title tile range 0x{start:03X}..0x{end:03X}"
            ));
        }
        ranges.push((start, end));
    }
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(format!(
                "mutable title tile ranges 0x{:03X}..0x{:03X} and 0x{:03X}..0x{:03X} overlap",
                pair[0].0, pair[0].1, pair[1].0, pair[1].1
            ));
        }
    }
    Ok(ranges)
}

fn read_source_glyphs(
    original_payload: &[u8],
    sources: &[SourceGlyphReuse],
) -> Result<BTreeMap<char, [u8; GLYPH_BYTES]>, String> {
    let mut glyphs = BTreeMap::new();
    for source in sources {
        let mut characters = source.character.chars();
        let character = characters
            .next()
            .ok_or_else(|| "source title glyph declares an empty character".to_string())?;
        if characters.next().is_some() || character.is_whitespace() {
            return Err(format!(
                "source title glyph {:?} is not one visible character",
                source.character
            ));
        }
        let tile = u16::try_from(parse_hex(&source.source_tile)?)
            .map_err(|_| format!("source title tile {} exceeds u16", source.source_tile))?;
        if tile < TITLE_TILE_ORIGIN
            || tile + GLYPH_TILES > TITLE_ORIGINAL_END_TILE
            || !tile.is_multiple_of(GLYPH_TILES)
        {
            return Err(format!(
                "source title glyph {character:?} tile 0x{tile:03X} is outside the JP payload"
            ));
        }
        let offset = (tile - TITLE_TILE_ORIGIN) as usize * MD_TILE_BYTES;
        let bytes: [u8; GLYPH_BYTES] = source_range(
            original_payload,
            offset,
            GLYPH_BYTES,
            &format!("source title glyph {character:?}"),
        )?
        .try_into()
        .expect("source_range returned the requested glyph size");
        if glyphs.insert(character, bytes).is_some() {
            return Err(format!(
                "source title glyph {character:?} is declared more than once"
            ));
        }
    }
    Ok(glyphs)
}

fn validate_protected_title_tiles(
    original: &[u8],
    replacement: &[u8],
    mutable_tile_ranges: &[(u16, u16)],
) -> Result<(), String> {
    if original.len() != ORIGINAL_TITLE_PAYLOAD_BYTES
        || replacement.len() < ORIGINAL_TITLE_PAYLOAD_BYTES
    {
        return Err("title payload length cannot prove protected JP tiles".to_string());
    }
    for tile in TITLE_TILE_ORIGIN..TITLE_ORIGINAL_END_TILE {
        if mutable_tile_ranges
            .iter()
            .any(|&(start, end)| (start..end).contains(&tile))
        {
            continue;
        }
        let offset = (tile - TITLE_TILE_ORIGIN) as usize * MD_TILE_BYTES;
        if original[offset..offset + MD_TILE_BYTES] != replacement[offset..offset + MD_TILE_BYTES] {
            return Err(format!(
                "title graphics changed protected JP tile 0x{tile:03X}"
            ));
        }
    }
    Ok(())
}

fn menu_layout(entry: &TitleMenuEntry) -> Vec<Option<char>> {
    entry
        .ko
        .chars()
        .filter_map(|ch| {
            if ch.is_whitespace() {
                entry.spaces_consume_slots.then_some(None)
            } else {
                Some(Some(ch))
            }
        })
        .collect()
}

struct TileBlockAllocator {
    ranges: Vec<(u16, u16)>,
    range_index: usize,
    cursor: u16,
    high_water_mark: u16,
}

impl TileBlockAllocator {
    fn new(ranges: &[(u16, u16)]) -> Result<Self, String> {
        let &(start, _) = ranges
            .first()
            .ok_or_else(|| "no mutable title tile ranges are available".to_string())?;
        Ok(Self {
            ranges: ranges.to_vec(),
            range_index: 0,
            cursor: start,
            high_water_mark: start,
        })
    }

    fn allocate(&mut self, label: &str) -> Result<u16, String> {
        loop {
            let Some(&(start, end)) = self.ranges.get(self.range_index) else {
                return Err(format!(
                    "{label} does not fit in declared mutable title tile ranges"
                ));
            };
            if self.cursor < start {
                self.cursor = start;
            }
            if self.cursor + GLYPH_TILES <= end {
                let allocated = self.cursor;
                self.cursor += GLYPH_TILES;
                self.high_water_mark = self.high_water_mark.max(self.cursor);
                return Ok(allocated);
            }
            self.range_index += 1;
            if let Some(&(next_start, _)) = self.ranges.get(self.range_index) {
                self.cursor = next_start;
            }
        }
    }

    fn high_water_mark(&self) -> u16 {
        self.high_water_mark
    }
}

fn allocate_title_tiles(
    entries: &[TitleMenuEntry],
    prompt_plan: &title_prompt::TitlePromptPlan,
    numeric_glyphs: &str,
    mutable_tile_ranges: &[(u16, u16)],
) -> Result<(BTreeMap<char, u16>, u16, u16), String> {
    let mut allocator = TileBlockAllocator::new(mutable_tile_ranges)?;
    let blank_tile = allocator.allocate("blank title glyph")?;
    let mut characters = BTreeSet::new();
    for entry in entries {
        let layout = menu_layout(entry);
        if entry.leading_blank_slots + layout.len() > entry.slots {
            return Err(format!(
                "{} needs {} slots but only {} are declared",
                entry.id,
                entry.leading_blank_slots + layout.len(),
                entry.slots
            ));
        }
        characters.extend(layout.into_iter().flatten());
    }
    for fragment in &prompt_plan.fragments {
        characters.extend(fragment.ko_layout.chars().filter(|ch| !ch.is_whitespace()));
    }
    let numeric = numeric_glyphs.chars().collect::<Vec<_>>();
    if numeric != "0123456789ABCDEF".chars().collect::<Vec<_>>() {
        return Err("title numeric glyph order must be exactly 0123456789ABCDEF".to_string());
    }
    characters.extend(numeric);

    let mut tile_by_char = BTreeMap::new();
    for ch in characters {
        tile_by_char.insert(ch, allocator.allocate(&format!("title glyph {ch:?}"))?);
    }
    Ok((tile_by_char, blank_tile, allocator.high_water_mark()))
}

fn build_structure_patches(
    source: &[u8],
    entries: &[TitleMenuEntry],
    tile_by_char: &BTreeMap<char, u16>,
    blank_tile: u16,
) -> Result<Vec<StructurePatch>, String> {
    let mut patches = Vec::with_capacity(entries.len());
    for entry in entries {
        let offset = parse_hex(&entry.structure_offset)?;
        let consumer_pointer_offset = parse_hex(&entry.consumer_pointer_offset)?;
        let consumer_pointer = source_range(
            source,
            consumer_pointer_offset,
            4,
            &format!("{} consumer pointer", entry.id),
        )?;
        let actual_structure_offset = u32::from_be_bytes([
            consumer_pointer[0],
            consumer_pointer[1],
            consumer_pointer[2],
            consumer_pointer[3],
        ]) as usize;
        if actual_structure_offset != offset {
            return Err(format!(
                "{} consumer at 0x{consumer_pointer_offset:06X} points to 0x{actual_structure_offset:06X}, not declared structure 0x{offset:06X}",
                entry.id
            ));
        }
        let length = match entry.structure_kind {
            StructureKind::PerSprite => entry.slots * 10,
            StructureKind::CountedSprite => 2 + entry.slots * 8,
        };
        let expected = source_range(source, offset, length, &entry.id)?.to_vec();
        let mut replacement = expected.clone();

        match entry.structure_kind {
            StructureKind::PerSprite => {
                for slot in 0..entry.slots {
                    let count_offset = slot * 10;
                    if read_u16(&replacement, count_offset, &entry.id)? != 1 {
                        return Err(format!(
                            "{} slot {} is not a one-subsprite JP structure",
                            entry.id, slot
                        ));
                    }
                }
            }
            StructureKind::CountedSprite => {
                if read_u16(&replacement, 0, &entry.id)? as usize != entry.slots {
                    return Err(format!(
                        "{} counted structure does not declare {} JP slots",
                        entry.id, entry.slots
                    ));
                }
            }
        }

        let visible = menu_layout(entry);
        let mut slots = vec![None; entry.slots];
        for (index, ch) in visible.into_iter().enumerate() {
            slots[entry.leading_blank_slots + index] = ch;
        }
        let last_visible_slot = slots
            .iter()
            .rposition(Option::is_some)
            .ok_or_else(|| format!("{} has no visible glyph", entry.id))?;
        for (slot, ch) in slots.iter().copied().enumerate() {
            let tile = match ch {
                Some(ch) => *tile_by_char
                    .get(&ch)
                    .ok_or_else(|| format!("{} glyph {ch:?} was not allocated", entry.id))?,
                None => blank_tile,
            };
            let tile_word_offset = match entry.structure_kind {
                StructureKind::PerSprite => slot * 10 + 6,
                StructureKind::CountedSprite => 2 + slot * 8 + 4,
            };
            let original_word = read_u16(&replacement, tile_word_offset, &entry.id)?;
            let replacement_word = (original_word & 0xF800) | tile;
            replacement[tile_word_offset..tile_word_offset + 2]
                .copy_from_slice(&replacement_word.to_be_bytes());
        }
        if let Some(origin) = &entry.target_slot_origin_x {
            let origin = u16::try_from(parse_hex(origin)?)
                .map_err(|_| format!("{} target X exceeds u16", entry.id))?;
            for slot in 0..entry.slots {
                let x = match entry.structure_kind {
                    // Each record belongs to an independently positioned parent
                    // sprite.  Advancing the relative X here would apply the
                    // 16-pixel slot stride twice at runtime.
                    StructureKind::PerSprite => origin,
                    StructureKind::CountedSprite => origin
                        .checked_add(u16::try_from(slot * 16).expect("slot advance fits u16"))
                        .ok_or_else(|| format!("{} target X overflowed", entry.id))?,
                };
                let x_offset = match entry.structure_kind {
                    StructureKind::PerSprite => slot * 10 + 8,
                    StructureKind::CountedSprite => 2 + slot * 8 + 6,
                };
                replacement[x_offset..x_offset + 2].copy_from_slice(&x.to_be_bytes());
            }
        }
        if let Some(expected_right) = &entry.target_visible_right_x {
            let expected_right = u16::try_from(parse_hex(expected_right)?)
                .map_err(|_| format!("{} target visible right edge exceeds u16", entry.id))?;
            let x_offset = match entry.structure_kind {
                StructureKind::PerSprite => last_visible_slot * 10 + 8,
                StructureKind::CountedSprite => 2 + last_visible_slot * 8 + 6,
            };
            let slot_x = read_u16(&replacement, x_offset, &entry.id)?;
            let actual_right = match entry.structure_kind {
                // The parent objects already contribute one 16-pixel stride
                // per physical slot; include the final glyph's width once.
                StructureKind::PerSprite => slot_x
                    .checked_add(
                        u16::try_from((last_visible_slot + 1) * 16).expect("slot advance fits u16"),
                    )
                    .ok_or_else(|| format!("{} visible right edge overflowed", entry.id))?,
                StructureKind::CountedSprite => slot_x
                    .checked_add(16)
                    .ok_or_else(|| format!("{} visible right edge overflowed", entry.id))?,
            };
            if actual_right != expected_right {
                return Err(format!(
                    "{} visible right edge is 0x{actual_right:04X}, expected 0x{expected_right:04X}",
                    entry.id
                ));
            }
        }

        patches.push(StructurePatch {
            id: entry.id.clone(),
            offset,
            expected,
            replacement,
        });
    }

    patches.sort_by_key(|patch| patch.offset);
    for pair in patches.windows(2) {
        if pair[0].offset + pair[0].replacement.len() > pair[1].offset {
            return Err(format!(
                "title-menu structures {} and {} overlap",
                pair[0].id, pair[1].id
            ));
        }
    }
    Ok(patches)
}

fn build_numeric_structures(
    source: &[u8],
    manifest: &TitleMenuManifest,
    tile_by_char: &BTreeMap<char, u16>,
    target_offset: usize,
) -> Result<(Vec<u8>, StructurePatch), String> {
    let glyphs = manifest.numeric_glyphs.chars().collect::<Vec<_>>();
    if glyphs != "0123456789ABCDEF".chars().collect::<Vec<_>>() {
        return Err("title numeric glyph order drifted".to_string());
    }
    let pointer_offset = parse_hex(&manifest.numeric_pointer_table_offset)?;
    let template_offset = parse_hex(&manifest.numeric_template_structure_offset)?;
    let expected_pointers = source_range(
        source,
        pointer_offset,
        glyphs.len() * 4,
        "JP sound-number pointer table",
    )?
    .to_vec();
    let template = source_range(
        source,
        template_offset,
        10,
        "JP sound-number zero structure",
    )?;
    if read_u16(template, 0, "JP sound-number zero structure")? != 1 {
        return Err("JP sound-number template is not one sprite".to_string());
    }

    let mut structures = Vec::with_capacity(glyphs.len() * template.len());
    let mut replacement_pointers = Vec::with_capacity(glyphs.len() * 4);
    for (index, ch) in glyphs.into_iter().enumerate() {
        let mut structure = template.to_vec();
        let tile = *tile_by_char
            .get(&ch)
            .ok_or_else(|| format!("sound-number glyph {ch:?} was not allocated"))?;
        let original_word = read_u16(&structure, 6, "JP sound-number template tile")?;
        structure[6..8].copy_from_slice(&((original_word & 0xF800) | tile).to_be_bytes());
        structures.extend_from_slice(&structure);
        let pointer = u32::try_from(target_offset + index * template.len())
            .map_err(|_| "sound-number structure pointer exceeds u32".to_string())?;
        replacement_pointers.extend_from_slice(&pointer.to_be_bytes());
    }
    Ok((
        structures,
        StructurePatch {
            id: "GFX-TITLE-SOUND-NUMBER pointer table".to_string(),
            offset: pointer_offset,
            expected: expected_pointers,
            replacement: replacement_pointers,
        },
    ))
}

fn native_glyph_to_md_tiles(glyph: &[u8; 32]) -> [u8; GLYPH_BYTES] {
    let mut output = [0u8; GLYPH_BYTES];
    let quadrants = [(0usize, 0usize), (0, 8), (8, 0), (8, 8)];
    for (tile_index, (x_origin, y_origin)) in quadrants.into_iter().enumerate() {
        let tile_offset = tile_index * MD_TILE_BYTES;
        for y in 0..8usize {
            let row =
                u16::from_be_bytes([glyph[(y_origin + y) * 2], glyph[(y_origin + y) * 2 + 1]]);
            for pair in 0..4usize {
                let x0 = x_origin + pair * 2;
                let x1 = x0 + 1;
                let mut byte = 0u8;
                if row & (1 << (15 - x0)) != 0 {
                    byte |= 0xF0;
                }
                if row & (1 << (15 - x1)) != 0 {
                    byte |= 0x0F;
                }
                output[tile_offset + y * 4 + pair] = byte;
            }
        }
    }
    output
}

fn md_glyph_pixel(payload: &[u8], glyph_tile: u16, x: usize, y: usize) -> Result<u8, String> {
    if x >= 16 || y >= 16 || glyph_tile < TITLE_TILE_ORIGIN {
        return Err("invalid title-menu glyph pixel coordinate".to_string());
    }
    let quadrant = match (x >= 8, y >= 8) {
        (false, false) => 0u16,
        (false, true) => 1,
        (true, false) => 2,
        (true, true) => 3,
    };
    let tile = glyph_tile + quadrant;
    let local_x = x % 8;
    let local_y = y % 8;
    let byte_offset =
        (tile - TITLE_TILE_ORIGIN) as usize * MD_TILE_BYTES + local_y * 4 + local_x / 2;
    let byte = *payload
        .get(byte_offset)
        .ok_or_else(|| "title-menu glyph pixel is outside payload".to_string())?;
    Ok(if local_x.is_multiple_of(2) {
        byte >> 4
    } else {
        byte & 0x0F
    })
}

fn validate_noop_roundtrip(data: &[u8]) -> Result<(), String> {
    let encoded =
        encode_locked_mode1_pack(TITLE_MENU_BANK_OFFSET, TITLE_MENU_VRAM_DESTINATION, data)?;
    let probe_len = (TITLE_MENU_BANK_OFFSET + encoded.bank.len())
        .max(TITLE_MENU_HEADER_OFFSET + encoded.header.len());
    let mut probe = vec![0u8; probe_len];
    probe[TITLE_MENU_HEADER_OFFSET..TITLE_MENU_HEADER_OFFSET + encoded.header.len()]
        .copy_from_slice(&encoded.header);
    probe[TITLE_MENU_BANK_OFFSET..TITLE_MENU_BANK_OFFSET + encoded.bank.len()]
        .copy_from_slice(&encoded.bank);
    let decoded = decode_mode1_pack_entry(&probe, TITLE_MENU_HEADER_OFFSET)?;
    if decoded.vram_destination != TITLE_MENU_VRAM_DESTINATION || decoded.data != data {
        return Err("title-menu semantic no-op pack round-trip failed".to_string());
    }
    Ok(())
}

fn encode_locked_mode1_pack(
    base_offset: usize,
    vram_destination: u16,
    data: &[u8],
) -> Result<EncodedPack, String> {
    if data.is_empty() || !data.len().is_multiple_of(MD_TILE_BYTES) {
        return Err(format!(
            "mode-1 graphics input must contain whole non-empty tiles, got {} bytes",
            data.len()
        ));
    }
    encode_locked_mode1_bytes(base_offset, vram_destination, data)
}

fn encode_locked_mode1_bytes(
    base_offset: usize,
    vram_destination: u16,
    data: &[u8],
) -> Result<EncodedPack, String> {
    if data.is_empty() {
        return Err("mode-1 input must not be empty".to_string());
    }
    if base_offset > 0xFF_FFFF || base_offset & 0x7F != 0 {
        return Err(format!(
            "mode-1 pack base 0x{base_offset:06X} is not a packed-pointer boundary"
        ));
    }
    let encoded_pointer_address = (base_offset & 0xFF_8000) | (base_offset & 0x7F);
    if encoded_pointer_address != base_offset {
        return Err(format!(
            "mode-1 pack base 0x{base_offset:06X} is outside the first 128 bytes of its 32 KiB pointer bank"
        ));
    }

    let data_offset = base_offset + 2 + MODE1_SUBHEADER_BYTES + MODE1_CHAIN_TERMINATOR.len();
    if data_offset & 0xFF_0000 != base_offset & 0xFF_0000 {
        return Err("mode-1 data crosses a 64 KiB source bank".to_string());
    }
    let mut bank = Vec::with_capacity(10 + data.len() + data.len().div_ceil(0x7F) + 1);
    bank.extend_from_slice(&2u16.to_be_bytes());
    bank.extend_from_slice(&[0xC0, 0x00, 0x00, 0x00]);
    bank.extend_from_slice(&(data_offset as u16).to_be_bytes());
    bank.extend_from_slice(&MODE1_CHAIN_TERMINATOR);
    for chunk in data.chunks(0x7F) {
        bank.push(chunk.len() as u8);
        bank.extend_from_slice(chunk);
    }
    bank.push(0);
    let remaining_source_bank = 0x1_0000 - (base_offset & 0xFFFF);
    if bank.len() > remaining_source_bank {
        return Err(format!(
            "mode-1 pack is {} bytes and crosses the 64 KiB source bank with only \
             {remaining_source_bank} bytes remaining",
            bank.len(),
        ));
    }

    let packed_source = (((base_offset & 0xFF8000) >> 8) | (base_offset & 0x7F)) as u16;
    let [source_high, source_low] = packed_source.to_be_bytes();
    let [vram_high, vram_low] = vram_destination.to_be_bytes();
    Ok(EncodedPack {
        header: [0x80, 0x00, source_high, source_low, vram_high, vram_low],
        bank,
    })
}

fn decode_mode1_pack_entry(rom: &[u8], header_offset: usize) -> Result<DecodedPack, String> {
    let header = source_range(rom, header_offset, 6, "graphics pack header")?;
    let command = header[0];
    if command != 0x00 && command != 0x80 {
        return Err(format!(
            "graphics pack header at 0x{header_offset:06X} uses unsupported command 0x{command:02X}"
        ));
    }
    if header[1] != 0 {
        return Err(format!(
            "graphics pack header at 0x{header_offset:06X} has nonzero padding"
        ));
    }
    let packed_source = u16::from_be_bytes([header[2], header[3]]) as usize;
    let source_bank = (packed_source << 8) & 0xFF8000;
    let pointer_address = source_bank | (packed_source & 0x7F);
    if !pointer_address.is_multiple_of(2) {
        return Err(format!(
            "graphics pack pointer at 0x{pointer_address:06X} is not 68000 word-aligned"
        ));
    }
    let subheader_pointer = read_u16(rom, pointer_address, "graphics pack pointer")? as usize;
    let subheader_address = source_bank | subheader_pointer;
    if !subheader_address.is_multiple_of(2) {
        return Err(format!(
            "graphics pack subheader at 0x{subheader_address:06X} is not 68000 word-aligned"
        ));
    }
    let subheader = source_range(rom, subheader_address, 6, "graphics pack subheader")?;
    if subheader[0] != 0xC0 || subheader[1] != 0 || subheader[2] != 0 || subheader[3] != 0 {
        return Err(format!(
            "graphics subheader at 0x{subheader_address:06X} is not locked mode-1 lookback data"
        ));
    }
    let chain_terminator = source_range(
        rom,
        subheader_address + MODE1_SUBHEADER_BYTES,
        MODE1_CHAIN_TERMINATOR.len(),
        "graphics pack subheader-chain terminator",
    )?;
    if chain_terminator != MODE1_CHAIN_TERMINATOR {
        return Err(format!(
            "graphics pack at 0x{header_offset:06X} does not end its single-subheader chain with FF FF"
        ));
    }
    let compressed_address =
        (subheader_address & 0xFF8000) | u16::from_be_bytes([subheader[4], subheader[5]]) as usize;
    let vram_destination = u16::from_be_bytes([header[4], header[5]]);
    let max_output = 0x10000usize.saturating_sub(vram_destination as usize);
    let mut cursor = compressed_address;
    let mut output = Vec::new();
    loop {
        let command = *rom.get(cursor).ok_or_else(|| {
            format!("mode-1 stream at 0x{compressed_address:06X} has no terminator")
        })?;
        cursor += 1;
        if command == 0 {
            break;
        }
        if command & 0x80 == 0 {
            let length = command as usize;
            let bytes = source_range(rom, cursor, length, "mode-1 absolute command")?;
            output.extend_from_slice(bytes);
            cursor += length;
        } else {
            let length = (command & 0x7F) as usize + 3;
            let distance = *rom
                .get(cursor)
                .ok_or_else(|| "mode-1 lookback command is truncated".to_string())?
                as usize
                + 1;
            cursor += 1;
            if distance > output.len() {
                return Err(format!(
                    "mode-1 lookback distance {distance} exceeds {} decoded bytes",
                    output.len()
                ));
            }
            for _ in 0..length {
                let byte = output[output.len() - distance];
                output.push(byte);
            }
        }
        if output.len() > max_output {
            return Err(format!(
                "graphics stream exceeds its 64 KiB destination boundary with {} decoded bytes",
                output.len()
            ));
        }
    }
    Ok(DecodedPack {
        vram_destination,
        data: output,
    })
}

fn apply_expected_write(
    output: &mut [u8],
    offset: usize,
    expected: &[u8],
    replacement: &[u8],
    label: &str,
) -> Result<(), String> {
    if expected.len() != replacement.len() {
        return Err(format!("{label}: expected/replacement lengths differ"));
    }
    let actual = output
        .get(offset..offset + expected.len())
        .ok_or_else(|| format!("{label}: range is outside cumulative ROM"))?;
    if actual != expected {
        return Err(format!(
            "{label}: cumulative bytes do not match expected source at 0x{offset:06X}"
        ));
    }
    output[offset..offset + replacement.len()].copy_from_slice(replacement);
    Ok(())
}

fn validate_only_ranges_changed(
    before: &[u8],
    after: &[u8],
    ranges: &[(usize, usize)],
) -> Result<(), String> {
    if before.len() != after.len() {
        return Err("graphics application changed cumulative ROM length".to_string());
    }
    for (offset, (&left, &right)) in before.iter().zip(after).enumerate() {
        if left != right
            && !ranges
                .iter()
                .any(|&(start, end)| (start..end).contains(&offset))
        {
            return Err(format!(
                "graphics application changed undeclared byte at 0x{offset:06X}"
            ));
        }
    }
    Ok(())
}

fn calculate_checksum(rom: &[u8]) -> u16 {
    rom[CHECKSUM_START..]
        .chunks_exact(2)
        .fold(0u16, |checksum, pair| {
            checksum.wrapping_add(u16::from_be_bytes([pair[0], pair[1]]))
        })
}

fn source_range<'a>(
    data: &'a [u8],
    offset: usize,
    length: usize,
    label: &str,
) -> Result<&'a [u8], String> {
    data.get(offset..offset + length)
        .ok_or_else(|| format!("{label}: range 0x{offset:06X}+0x{length:X} is outside input"))
}

fn read_u16(data: &[u8], offset: usize, label: &str) -> Result<u16, String> {
    let bytes = source_range(data, offset, 2, label)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn parse_hex(value: &str) -> Result<usize, String> {
    let digits = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .ok_or_else(|| format!("expected 0x-prefixed hexadecimal value, got {value:?}"))?;
    usize::from_str_radix(digits, 16)
        .map_err(|error| format!("invalid hexadecimal value {value:?}: {error}"))
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
