//! JP-source five-frame spell and monster lettering compiler.
//!
//! Bayoen and Cockadoodle share the same physical surface: five 16x14
//! tilemaps select one transparent and one solid source pattern.  The Korean
//! build preserves both JP pattern transfers and every original consumer byte,
//! and replaces only each RAM tilemap transfer.

use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use crate::m68k::{self, AddressReg, Inst};

use super::font_effect::read_verified_font;
use super::pixel::{PixelBounds, md_color, parse_md_palette, read_verified_rgba, write_rgba_png};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_bytes, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

#[path = "spell_animation_master.rs"]
mod master;
pub use master::{SpellAnimationMasterSummary, write_spell_animation_font_master};

const FRAME_WIDTH_TILES: usize = 16;
const FRAME_HEIGHT_TILES: usize = 14;
const FRAME_COUNT: usize = 5;
const FRAME_WORDS: usize = FRAME_WIDTH_TILES * FRAME_HEIGHT_TILES;
const FRAME_BYTES: usize = FRAME_WORDS * 2;
const MAP_BYTES: usize = FRAME_BYTES * FRAME_COUNT;
const FRAME_WIDTH_PIXELS: usize = FRAME_WIDTH_TILES * 8;
const FRAME_HEIGHT_PIXELS: usize = FRAME_HEIGHT_TILES * 8;
const PREVIEW_COLUMN_GAP: usize = 8;
const PREVIEW_ROW_GAP: usize = 8;
const PREVIEW_GROUP_GAP: usize = 20;
const BATTLE_MAP_BANK: usize = 0x2D_0000;
const COCKADOODLE_MAP_BANK: usize = 0x2D_8000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAnimationsSummary {
    pub animations: usize,
    pub frames: usize,
    pub source_pattern_bytes: usize,
    pub source_map_bytes: usize,
    pub target_map_bytes: usize,
    pub visible_target_tiles: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct SpellAnimationsManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    frame_width_tiles: usize,
    frame_height_tiles: usize,
    frame_count: usize,
    frame_policy: String,
    animations: Vec<AnimationDeclaration>,
}

#[derive(Debug, Clone, Deserialize)]
struct AnimationDeclaration {
    id: String,
    jp_text: String,
    ko: String,
    target_frames: Vec<String>,
    source_frame_visible: Vec<bool>,
    render_method: String,
    normalized_master: Option<NormalizedMasterDeclaration>,
    source_blank_tile_words: Vec<String>,
    blank_tile_word: String,
    solid_tile_word: String,
    tile_origin: String,
    solid_palette_index: usize,
    palette_line_words: Vec<String>,
    source_pattern_pack: PatternPackDeclaration,
    source_map_pack: MapPackDeclaration,
    target_map_bank_offset: String,
    consumer: ConsumerDeclaration,
}

#[derive(Debug, Clone, Deserialize)]
struct NormalizedMasterDeclaration {
    generation_method: String,
    asset: String,
    sha256: String,
    width: usize,
    height: usize,
    cell_size: usize,
    source_font_asset: String,
    source_font_sha256: String,
    source_font_size_px: f32,
    source_coverage_threshold: u8,
    alpha_bounds: PixelBounds,
}

#[derive(Debug, Clone, Deserialize)]
struct PatternPackDeclaration {
    header_offset: String,
    vram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MapPackDeclaration {
    header_offset: String,
    ram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ConsumerDeclaration {
    initial_lea_offset: String,
    initial_map_address: String,
    map_pointer_table_offset: String,
    map_pointers: Vec<String>,
    sequence_offset: String,
    sequences: Vec<Vec<u8>>,
}

#[derive(Debug)]
struct AnimationBuild {
    declaration: AnimationDeclaration,
    palette: [u16; 16],
    source_patterns: Vec<u8>,
    source_map: Vec<u8>,
    target_map: Vec<u8>,
    source_frames: Vec<Vec<u8>>,
    target_frames: Vec<Vec<u8>>,
    header: [u8; 6],
    bank: Vec<u8>,
    bank_offset: usize,
    visible_target_tiles: usize,
}

#[derive(Debug)]
struct SpellAnimationsBuild {
    animations: Vec<AnimationBuild>,
}

pub fn apply_spell_animations(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<SpellAnimationsSummary, String> {
    let build = build_spell_animations(source, assets_dir)?;
    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(build.animations.len() * 2 + 1);

    for animation in &build.animations {
        let map_header_offset = parse_hex(&animation.declaration.source_map_pack.header_offset)?;
        apply_expected_write(
            output,
            map_header_offset,
            source_range(
                source,
                map_header_offset,
                animation.header.len(),
                "spell-animation source map header",
            )?,
            &animation.header,
            &format!("{} map pack header", animation.declaration.id),
        )?;
        changed_ranges.push((
            map_header_offset,
            map_header_offset + animation.header.len(),
        ));

        let bank_end = animation.bank_offset + animation.bank.len();
        if bank_end > animation.bank_offset + 0x8000 || bank_end > output.len() {
            return Err(format!(
                "{} map pack ends outside its expanded bank at 0x{bank_end:06X}",
                animation.declaration.id
            ));
        }
        apply_expected_write(
            output,
            animation.bank_offset,
            &vec![0xFF; animation.bank.len()],
            &animation.bank,
            &format!("{} expanded map pack", animation.declaration.id),
        )?;
        changed_ranges.push((animation.bank_offset, bank_end));
    }

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after spell animations",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    for animation in &build.animations {
        let map_header_offset = parse_hex(&animation.declaration.source_map_pack.header_offset)?;
        let inserted = decode_mode1_pack_entry(output, map_header_offset)?;
        let expected_destination =
            parse_u16_hex(&animation.declaration.source_map_pack.ram_destination)?;
        if inserted.vram_destination != expected_destination
            || inserted.data != animation.target_map
        {
            return Err(format!(
                "{} inserted map pack does not match the planned five frames",
                animation.declaration.id
            ));
        }
        validate_consumer(source, &animation.declaration)?;
        validate_consumer(output, &animation.declaration)?;

        let pattern_header = parse_hex(&animation.declaration.source_pattern_pack.header_offset)?;
        let preserved_patterns = decode_mode1_pack_entry(output, pattern_header)?;
        if preserved_patterns.data != animation.source_patterns {
            return Err(format!(
                "{} changed its protected JP pattern transfer",
                animation.declaration.id
            ));
        }

        let bank_end = animation.bank_offset + animation.bank.len();
        eprintln!("JP graphics {} Expected Writes:", animation.declaration.id);
        eprintln!(
            "  0x{map_header_offset:06X}..0x{:06X}  RAM map header ({} bytes)",
            map_header_offset + animation.header.len(),
            animation.header.len()
        );
        eprintln!(
            "  0x{:06X}..0x{bank_end:06X}  five-frame map pack ({} bytes)",
            animation.bank_offset,
            animation.bank.len()
        );
    }
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render all ten JP/Korean frames in one deterministic local QA image.
pub fn write_spell_animations_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<SpellAnimationsSummary, String> {
    let build = build_spell_animations(source, assets_dir)?;
    let preview_width = FRAME_WIDTH_PIXELS * FRAME_COUNT + PREVIEW_COLUMN_GAP * (FRAME_COUNT - 1);
    let preview_height = FRAME_HEIGHT_PIXELS * 4 + PREVIEW_ROW_GAP * 2 + PREVIEW_GROUP_GAP;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[196, 196, 196, 255]);
    }

    let mut row = 0usize;
    for (animation_index, animation) in build.animations.iter().enumerate() {
        for frames in [&animation.source_frames, &animation.target_frames] {
            let y = row * (FRAME_HEIGHT_PIXELS + PREVIEW_ROW_GAP)
                + animation_index * (PREVIEW_GROUP_GAP - PREVIEW_ROW_GAP);
            for (frame_index, frame) in frames.iter().enumerate() {
                let x = frame_index * (FRAME_WIDTH_PIXELS + PREVIEW_COLUMN_GAP);
                draw_frame(&mut rgba, preview_width, x, y, frame, &animation.palette)?;
            }
            row += 1;
        }
    }

    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "spell-animation batch",
    )?;
    Ok(summary(&build, 0))
}

fn build_spell_animations(
    source: &[u8],
    assets_dir: &Path,
) -> Result<SpellAnimationsBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let mut animations = Vec::with_capacity(manifest.animations.len());
    for declaration in &manifest.animations {
        animations.push(build_animation(source, assets_dir, declaration)?);
    }
    Ok(SpellAnimationsBuild { animations })
}

fn build_animation(
    source: &[u8],
    assets_dir: &Path,
    declaration: &AnimationDeclaration,
) -> Result<AnimationBuild, String> {
    validate_consumer(source, declaration)?;
    let pattern_header = parse_hex(&declaration.source_pattern_pack.header_offset)?;
    let map_header = parse_hex(&declaration.source_map_pack.header_offset)?;
    let source_patterns = checked_pack(
        source,
        pattern_header,
        parse_u16_hex(&declaration.source_pattern_pack.vram_destination)?,
        declaration.source_pattern_pack.decoded_bytes,
        &declaration.source_pattern_pack.decoded_sha256,
        &format!("{} JP pattern", declaration.id),
    )?;
    let source_map = checked_pack(
        source,
        map_header,
        parse_u16_hex(&declaration.source_map_pack.ram_destination)?,
        declaration.source_map_pack.decoded_bytes,
        &declaration.source_map_pack.decoded_sha256,
        &format!("{} JP map", declaration.id),
    )?;
    if source_range(
        source,
        pattern_header,
        1,
        "spell-animation JP pattern command",
    )? != [0x80]
        || source_range(source, map_header, 1, "spell-animation JP map command")? != [0x00]
    {
        return Err(format!(
            "{} JP mode-1 pattern / mode-0 RAM map pair drifted",
            declaration.id
        ));
    }
    if source_range(
        source,
        map_header + 6,
        2,
        "spell-animation group terminator",
    )? != [0xFF, 0xFF]
    {
        return Err(format!(
            "{} map transfer is not followed by the JP group terminator",
            declaration.id
        ));
    }

    let source_blank_words = declaration
        .source_blank_tile_words
        .iter()
        .map(|word| parse_u16_hex(word))
        .collect::<Result<Vec<_>, _>>()?;
    let blank_word = parse_u16_hex(&declaration.blank_tile_word)?;
    let solid_word = parse_u16_hex(&declaration.solid_tile_word)?;
    let tile_origin = parse_u16_hex(&declaration.tile_origin)?;
    validate_source_map(
        &source_map,
        &source_blank_words,
        solid_word,
        &declaration.id,
    )?;
    validate_pattern_roles(
        &source_patterns,
        &source_blank_words,
        solid_word,
        tile_origin,
        declaration.solid_palette_index as u8,
        &declaration.id,
    )?;
    let source_frames = decode_frames(&source_patterns, &source_map, tile_origin, &declaration.id)?;
    let source_visibility = source_frames
        .iter()
        .map(|frame| frame.iter().any(|&palette_index| palette_index != 0))
        .collect::<Vec<_>>();
    if source_visibility != declaration.source_frame_visible {
        return Err(format!(
            "{} JP frame visibility drifted: expected {:?}, got {:?}",
            declaration.id, declaration.source_frame_visible, source_visibility
        ));
    }

    if declaration.render_method != "normalized_cell_master" {
        return Err(format!(
            "{} uses unsupported render method {:?}",
            declaration.id, declaration.render_method
        ));
    }
    let normalized_cells = read_normalized_master_cells(assets_dir, declaration)?;
    let target_map = render_target_map(
        &declaration.target_frames,
        &normalized_cells,
        TargetMapRenderConfig {
            blank_word,
            solid_word,
            label: &declaration.id,
        },
    )?;
    let target_frames = decode_frames(&source_patterns, &target_map, tile_origin, &declaration.id)?;
    let visible_target_tiles = target_map
        .chunks_exact(2)
        .filter(|word| u16::from_be_bytes([word[0], word[1]]) == solid_word)
        .count();
    if visible_target_tiles == 0 {
        return Err(format!("{} Korean frames are all blank", declaration.id));
    }

    let bank_offset = parse_hex(&declaration.target_map_bank_offset)?;
    let ram_destination = parse_u16_hex(&declaration.source_map_pack.ram_destination)?;
    let (header, bank) = encode_locked_mode0_map(bank_offset, ram_destination, &target_map)?;
    validate_pack_roundtrip(&header, &bank, bank_offset, ram_destination, &target_map)?;

    Ok(AnimationBuild {
        declaration: declaration.clone(),
        palette: parse_md_palette(
            &declaration.palette_line_words,
            &format!("{} preview", declaration.id),
        )?,
        source_patterns,
        source_map,
        target_map,
        source_frames,
        target_frames,
        header,
        bank,
        bank_offset,
        visible_target_tiles,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<SpellAnimationsManifest, String> {
    let path = assets_dir.join("graphics_text/spell_animations.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read spell-animation source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid spell-animation source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &SpellAnimationsManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-SPELL-ANIMATION"
        || manifest.animations.len() != 2
        || manifest.frame_width_tiles != FRAME_WIDTH_TILES
        || manifest.frame_height_tiles != FRAME_HEIGHT_TILES
        || manifest.frame_count != FRAME_COUNT
        || manifest.frame_policy != "derive every KR frame from one approved lettering master"
        || !manifest
            .source_policy
            .contains("English seven-frame expansions")
        || !manifest
            .source_policy
            .contains("Galmuri11-Bold fixed-12px threshold-96 normalized cell master")
    {
        return Err("spell-animation shared source contract drifted".to_string());
    }

    for (index, declaration) in manifest.animations.iter().enumerate() {
        let (
            expected_id,
            expected_jp,
            expected_ko,
            expected_pattern_header,
            expected_map_header,
            expected_bank,
        ) = match index {
            0 => (
                "GFX-SPELL-BAYOEN",
                "ばよえ〜ん",
                "바요에에엔",
                0x077ED2,
                0x077ED8,
                BATTLE_MAP_BANK,
            ),
            1 => (
                "GFX-SPELL-COCKADOODLE",
                "コケコッコー",
                "꼬끼오오",
                0x02710C,
                0x027112,
                COCKADOODLE_MAP_BANK,
            ),
            _ => unreachable!(),
        };
        if declaration.id != expected_id
            || declaration.jp_text != expected_jp
            || declaration.ko != expected_ko
            || parse_hex(&declaration.source_pattern_pack.header_offset)? != expected_pattern_header
            || parse_hex(&declaration.source_map_pack.header_offset)? != expected_map_header
            || parse_hex(&declaration.target_map_bank_offset)? != expected_bank
            || declaration.source_map_pack.decoded_bytes != MAP_BYTES
            || declaration.target_frames.len() != FRAME_COUNT
            || declaration.source_frame_visible.len() != FRAME_COUNT
            || declaration.solid_palette_index >= 16
            || !declaration
                .source_blank_tile_words
                .contains(&declaration.blank_tile_word)
        {
            return Err(format!("{} source contract drifted", declaration.id));
        }
        let expected_generation_method = if index == 0 {
            "approved_normalized"
        } else {
            "font_raster"
        };
        if declaration.render_method != "normalized_cell_master"
            || declaration
                .normalized_master
                .as_ref()
                .is_none_or(|master| master.generation_method != expected_generation_method)
        {
            return Err(format!(
                "{} render ownership contract drifted",
                declaration.id
            ));
        }
        let lettering_master = declaration.target_frames.concat();
        let target_visibility = declaration
            .target_frames
            .iter()
            .map(|frame| !frame.is_empty())
            .collect::<Vec<_>>();
        if lettering_master != declaration.ko
            || target_visibility != declaration.source_frame_visible
            || declaration
                .target_frames
                .iter()
                .any(|frame| frame.chars().count() > 1)
        {
            return Err(format!(
                "{} target frames do not derive from the declared Korean lettering master",
                declaration.id
            ));
        }
    }
    Ok(())
}

fn checked_pack(
    source: &[u8],
    header_offset: usize,
    destination: u16,
    decoded_bytes: usize,
    expected_sha256: &str,
    label: &str,
) -> Result<Vec<u8>, String> {
    let decoded = decode_mode1_pack_entry(source, header_offset)?;
    if decoded.vram_destination != destination || decoded.data.len() != decoded_bytes {
        return Err(format!(
            "{label} decoded contract drifted: destination 0x{:04X}, {} bytes",
            decoded.vram_destination,
            decoded.data.len()
        ));
    }
    let actual_sha256 = sha256_hex(&decoded.data);
    if actual_sha256 != expected_sha256 {
        return Err(format!(
            "{label} SHA-256 mismatch: expected {expected_sha256}, got {actual_sha256}"
        ));
    }
    Ok(decoded.data)
}

fn validate_source_map(
    map: &[u8],
    blank_words: &[u16],
    solid_word: u16,
    label: &str,
) -> Result<(), String> {
    if map.len() != MAP_BYTES || blank_words.is_empty() {
        return Err(format!("{label} source map length drifted"));
    }
    let mut saw_blank = false;
    let mut saw_solid = false;
    for word in map.chunks_exact(2) {
        match u16::from_be_bytes([word[0], word[1]]) {
            value if blank_words.contains(&value) => saw_blank = true,
            value if value == solid_word => saw_solid = true,
            value => {
                return Err(format!(
                    "{label} source map uses undeclared tile word 0x{value:04X}"
                ));
            }
        }
    }
    if !saw_blank || !saw_solid {
        return Err(format!("{label} source map lacks blank or solid cells"));
    }
    Ok(())
}

fn validate_pattern_roles(
    patterns: &[u8],
    blank_words: &[u16],
    solid_word: u16,
    tile_origin: u16,
    solid_palette_index: u8,
    label: &str,
) -> Result<(), String> {
    let pattern = |word: u16| -> Result<&[u8], String> {
        if word & 0xF800 != 0 {
            return Err(format!(
                "{label} role word 0x{word:04X} has unsupported attributes"
            ));
        }
        let tile = (word & 0x07FF)
            .checked_sub(tile_origin)
            .ok_or_else(|| format!("{label} role word 0x{word:04X} precedes its tile origin"))?
            as usize;
        source_range(
            patterns,
            tile * MD_TILE_BYTES,
            MD_TILE_BYTES,
            &format!("{label} role pattern"),
        )
    };
    for &blank_word in blank_words {
        if pattern(blank_word)?.iter().any(|&byte| byte != 0) {
            return Err(format!(
                "{label} blank word 0x{blank_word:04X} is not transparent"
            ));
        }
    }
    let solid_nibbles = pattern(solid_word)?
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F]);
    if solid_nibbles
        .into_iter()
        .any(|index| index != solid_palette_index)
    {
        return Err(format!(
            "{label} solid word 0x{solid_word:04X} does not use only palette index {solid_palette_index}"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct TargetMapRenderConfig<'a> {
    blank_word: u16,
    solid_word: u16,
    label: &'a str,
}

fn render_target_map(
    target_frames: &[String],
    normalized_cells: &[Vec<bool>],
    config: TargetMapRenderConfig<'_>,
) -> Result<Vec<u8>, String> {
    let TargetMapRenderConfig {
        blank_word,
        solid_word,
        label,
    } = config;
    let mut map = Vec::with_capacity(MAP_BYTES);
    if normalized_cells.len() != target_frames.len() {
        return Err(format!("{label} normalized frame count drifted"));
    }
    for cells in normalized_cells {
        for cell in cells {
            map.extend_from_slice(&(if *cell { solid_word } else { blank_word }).to_be_bytes());
        }
    }
    if map.len() != MAP_BYTES {
        return Err(format!("{label} target map length drifted"));
    }
    Ok(map)
}

fn read_normalized_master_cells(
    assets_dir: &Path,
    declaration: &AnimationDeclaration,
) -> Result<Vec<Vec<bool>>, String> {
    let master = declaration.normalized_master.as_ref().ok_or_else(|| {
        format!(
            "{} normalized render method lacks a cell master",
            declaration.id
        )
    })?;
    if master.source_font_asset != "../fonts/Galmuri11-Bold.ttf"
        || master.source_font_sha256
            != "5265b2f437fe81f0c8095b44c0173dd9a276b58a42552bf983f21c0e69e6e8af"
        || master.source_font_size_px != 12.0
        || master.source_coverage_threshold != 96
    {
        return Err(format!(
            "{} normalized master provenance drifted",
            declaration.id
        ));
    }
    let _source_font = read_verified_font(
        assets_dir,
        &master.source_font_asset,
        &master.source_font_sha256,
        &format!("{} normalized-master source", declaration.id),
    )?;
    let expected_width = FRAME_WIDTH_TILES * FRAME_COUNT * master.cell_size;
    let expected_height = FRAME_HEIGHT_TILES * master.cell_size;
    if master.cell_size != 8 || master.width != expected_width || master.height != expected_height {
        return Err(format!(
            "{} normalized master geometry drifted",
            declaration.id
        ));
    }
    let rgba = read_verified_rgba(
        assets_dir,
        &master.asset,
        &master.sha256,
        master.width,
        master.height,
        master.alpha_bounds,
        &format!("{} normalized cell master", declaration.id),
    )?;
    let mut frames = Vec::with_capacity(FRAME_COUNT);
    for frame_index in 0..FRAME_COUNT {
        let mut cells = vec![false; FRAME_WORDS];
        for cell_y in 0..FRAME_HEIGHT_TILES {
            for cell_x in 0..FRAME_WIDTH_TILES {
                let mut role = None;
                for local_y in 0..master.cell_size {
                    for local_x in 0..master.cell_size {
                        let x =
                            (frame_index * FRAME_WIDTH_TILES + cell_x) * master.cell_size + local_x;
                        let y = cell_y * master.cell_size + local_y;
                        let pixel = &rgba[(y * master.width + x) * 4..][..4];
                        let opaque = match pixel {
                            [0, 0, 0, 0] => false,
                            [0, 0, 0, 255] => true,
                            _ => {
                                return Err(format!(
                                    "{} normalized master has a non-binary pixel at ({x},{y})",
                                    declaration.id
                                ));
                            }
                        };
                        if role
                            .replace(opaque)
                            .is_some_and(|previous| previous != opaque)
                        {
                            return Err(format!(
                                "{} normalized master cell ({frame_index},{cell_x},{cell_y}) is not uniform",
                                declaration.id
                            ));
                        }
                    }
                }
                cells[cell_y * FRAME_WIDTH_TILES + cell_x] = role.unwrap_or(false);
            }
        }
        if cells[..FRAME_WIDTH_TILES].contains(&true)
            || cells[FRAME_WORDS - FRAME_WIDTH_TILES..].contains(&true)
            || (0..FRAME_HEIGHT_TILES).any(|y| {
                cells[y * FRAME_WIDTH_TILES] || cells[y * FRAME_WIDTH_TILES + FRAME_WIDTH_TILES - 1]
            })
        {
            return Err(format!(
                "{} normalized frame {frame_index} lost its blank outer rim",
                declaration.id
            ));
        }
        let frame_is_declared_blank = declaration.target_frames[frame_index].is_empty();
        if frame_is_declared_blank == cells.contains(&true) {
            return Err(format!(
                "{} normalized frame {frame_index} blank role drifted",
                declaration.id
            ));
        }
        frames.push(cells);
    }
    Ok(frames)
}

fn render_lettering_grid(
    font: &Font,
    ch: char,
    font_size_px: f32,
    coverage_threshold: u8,
    label: &str,
) -> Result<Vec<bool>, String> {
    let (metrics, coverage) = font.rasterize(ch, font_size_px);
    if metrics.width == 0
        || metrics.height == 0
        || metrics.width > FRAME_WIDTH_TILES
        || metrics.height > FRAME_HEIGHT_TILES
    {
        return Err(format!(
            "{label} glyph {ch:?} is {}x{}, outside the 16x14 lettering grid",
            metrics.width, metrics.height
        ));
    }
    let origin_x = (FRAME_WIDTH_TILES - metrics.width) / 2;
    let origin_y = (FRAME_HEIGHT_TILES - metrics.height) / 2;
    let mut cells = vec![false; FRAME_WORDS];
    for y in 0..metrics.height {
        for x in 0..metrics.width {
            if coverage[y * metrics.width + x] > coverage_threshold {
                cells[(origin_y + y) * FRAME_WIDTH_TILES + origin_x + x] = true;
            }
        }
    }
    if !cells.contains(&true) {
        return Err(format!("{label} glyph {ch:?} rendered blank"));
    }
    Ok(cells)
}

fn decode_frames(
    patterns: &[u8],
    map: &[u8],
    tile_origin: u16,
    label: &str,
) -> Result<Vec<Vec<u8>>, String> {
    if !patterns.len().is_multiple_of(MD_TILE_BYTES) || map.len() != MAP_BYTES {
        return Err(format!("{label} frame source lengths are invalid"));
    }
    let mut frames = Vec::with_capacity(FRAME_COUNT);
    for frame_map in map.chunks_exact(FRAME_BYTES) {
        let mut frame = vec![0u8; FRAME_WIDTH_PIXELS * FRAME_HEIGHT_PIXELS];
        for (cell_index, word) in frame_map.chunks_exact(2).enumerate() {
            let tile_word = u16::from_be_bytes([word[0], word[1]]);
            if tile_word & 0xF800 != 0 {
                return Err(format!(
                    "{label} frame uses unsupported tile attributes 0x{tile_word:04X}"
                ));
            }
            let tile_index = tile_word & 0x07FF;
            let relative = tile_index.checked_sub(tile_origin).ok_or_else(|| {
                format!("{label} tile 0x{tile_index:03X} precedes origin 0x{tile_origin:03X}")
            })? as usize;
            let tile = source_range(
                patterns,
                relative * MD_TILE_BYTES,
                MD_TILE_BYTES,
                &format!("{label} pattern tile"),
            )?;
            let cell_x = cell_index % FRAME_WIDTH_TILES;
            let cell_y = cell_index / FRAME_WIDTH_TILES;
            for local_y in 0..8 {
                for local_x in 0..8 {
                    let byte = tile[local_y * 4 + local_x / 2];
                    let palette_index = if local_x.is_multiple_of(2) {
                        byte >> 4
                    } else {
                        byte & 0x0F
                    };
                    let x = cell_x * 8 + local_x;
                    let y = cell_y * 8 + local_y;
                    frame[y * FRAME_WIDTH_PIXELS + x] = palette_index;
                }
            }
        }
        frames.push(frame);
    }
    Ok(frames)
}

fn validate_consumer(source: &[u8], declaration: &AnimationDeclaration) -> Result<(), String> {
    let lea_offset = parse_hex(&declaration.consumer.initial_lea_offset)?;
    let lea = m68k::assemble(&[Inst::LeaAbsoluteLong {
        address: parse_u32_hex(&declaration.consumer.initial_map_address)?,
        destination: AddressReg::A2,
    }])?;
    if source_range(source, lea_offset, lea.len(), "spell-animation typed LEA")? != lea {
        return Err(format!(
            "{} typed initial map consumer drifted",
            declaration.id
        ));
    }

    let pointer_table = parse_hex(&declaration.consumer.map_pointer_table_offset)?;
    for (index, expected) in declaration.consumer.map_pointers.iter().enumerate() {
        let actual = read_u32(
            source,
            pointer_table + index * 4,
            "spell-animation map pointer",
        )?;
        if actual != parse_u32_hex(expected)? {
            return Err(format!(
                "{} map pointer {index} drifted to 0x{actual:08X}",
                declaration.id
            ));
        }
    }

    let sequence_offset = parse_hex(&declaration.consumer.sequence_offset)?;
    let expected_sequences = declaration
        .consumer
        .sequences
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    if source_range(
        source,
        sequence_offset,
        expected_sequences.len(),
        "spell-animation sequence",
    )? != expected_sequences
    {
        return Err(format!(
            "{} original animation sequence drifted",
            declaration.id
        ));
    }
    Ok(())
}

fn validate_pack_roundtrip(
    header: &[u8; 6],
    bank: &[u8],
    bank_offset: usize,
    ram_destination: u16,
    data: &[u8],
) -> Result<(), String> {
    if header[0] != 0x00 || u16::from_be_bytes([header[4], header[5]]) != ram_destination {
        return Err(format!(
            "spell-animation map header is not mode-0 RAM destination 0x{ram_destination:04X}"
        ));
    }
    let mut probe = vec![0u8; bank_offset + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[bank_offset..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != ram_destination || decoded.data != data {
        return Err("spell-animation map pack round-trip failed".to_string());
    }
    Ok(())
}

fn encode_locked_mode0_map(
    bank_offset: usize,
    ram_destination: u16,
    data: &[u8],
) -> Result<([u8; 6], Vec<u8>), String> {
    let encoded = encode_locked_mode1_bytes(bank_offset, ram_destination, data)?;
    let mut header = encoded.header;
    header[0] = 0x00;
    Ok((header, encoded.bank))
}

fn draw_frame(
    rgba: &mut [u8],
    output_width: usize,
    origin_x: usize,
    origin_y: usize,
    frame: &[u8],
    palette: &[u16; 16],
) -> Result<(), String> {
    if frame.len() != FRAME_WIDTH_PIXELS * FRAME_HEIGHT_PIXELS {
        return Err("spell-animation preview frame length drifted".to_string());
    }
    let output_height = rgba.len() / 4 / output_width;
    if origin_x + FRAME_WIDTH_PIXELS > output_width
        || origin_y + FRAME_HEIGHT_PIXELS > output_height
    {
        return Err("spell-animation preview frame is out of bounds".to_string());
    }
    for y in 0..FRAME_HEIGHT_PIXELS {
        for x in 0..FRAME_WIDTH_PIXELS {
            let palette_index = frame[y * FRAME_WIDTH_PIXELS + x] as usize;
            if palette_index == 0 {
                continue;
            }
            let color = md_color(palette[palette_index]);
            let offset = ((origin_y + y) * output_width + origin_x + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
        }
    }
    Ok(())
}

fn summary(build: &SpellAnimationsBuild, checksum: u16) -> SpellAnimationsSummary {
    SpellAnimationsSummary {
        animations: build.animations.len(),
        frames: build
            .animations
            .iter()
            .map(|animation| animation.target_frames.len())
            .sum(),
        source_pattern_bytes: build
            .animations
            .iter()
            .map(|animation| animation.source_patterns.len())
            .sum(),
        source_map_bytes: build
            .animations
            .iter()
            .map(|animation| animation.source_map.len())
            .sum(),
        target_map_bytes: build
            .animations
            .iter()
            .map(|animation| animation.target_map.len())
            .sum(),
        visible_target_tiles: build
            .animations
            .iter()
            .map(|animation| animation.visible_target_tiles)
            .sum(),
        pack_bytes: build
            .animations
            .iter()
            .map(|animation| animation.bank.len())
            .sum(),
        checksum,
    }
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("hex value {value} exceeds u16"))
}

fn parse_u32_hex(value: &str) -> Result<u32, String> {
    let parsed = parse_hex(value)?;
    u32::try_from(parsed).map_err(|_| format!("hex value {value} exceeds u32"))
}

fn read_u32(data: &[u8], offset: usize, label: &str) -> Result<u32, String> {
    let bytes = source_range(data, offset, 4, label)?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}
