//! JP-source graduation-exam result-card compiler.
//!
//! The parchment, flourishes, pseudo-writing, score digits, and seals are
//! protected source art. Four baked JP phrases are replaced on the decoded
//! 64x64 plane while the original 110 low-pattern tiles remain a byte-exact
//! prefix. Changed cells receive appended patterns below the next JP transfer.
//! The dynamic `てん` sprite call is suppressed with a typed 68000 `RTS`, so
//! the background can carry the complete Korean `점입니다`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use fontdue::{Font, FontSettings};
use serde::Deserialize;

use crate::jp_native;
use crate::m68k::{self, Inst};

use super::pixel::{
    PixelBounds, md_color, native_glyph_pixel, nearest_core_distance, parse_md_palette,
    write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_bytes, encode_locked_mode1_pack, parse_hex,
    sha256_hex, source_range, validate_only_ranges_changed,
};

const EXAM_LOW_HEADER_OFFSET: usize = 0x09_61A6;
const EXAM_HIGH_HEADER_OFFSET: usize = 0x09_61AC;
const EXAM_MAP_HEADER_OFFSET: usize = 0x09_61B2;
const EXAM_WINDOW_HEADER_OFFSET: usize = 0x09_61B8;
const EXAM_LOW_VRAM: u16 = 0x2000;
const EXAM_HIGH_VRAM: u16 = 0x4000;
const EXAM_MAP_VRAM: u16 = 0xE000;
const EXAM_WINDOW_VRAM: u16 = 0xD000;
const EXAM_LOW_BYTES: usize = 3_520;
const EXAM_HIGH_BYTES: usize = 6_144;
const EXAM_MAP_BYTES: usize = 8_192;
const EXAM_WINDOW_BYTES: usize = 4_096;
const EXAM_LOW_SOURCE_TILES: usize = EXAM_LOW_BYTES / MD_TILE_BYTES;
const EXAM_PATTERN_BANK_OFFSET: usize = 0x2A_8000;
const EXAM_MAP_BANK_OFFSET: usize = 0x2B_0000;
const EXAM_MAP_BANK_LIMIT: usize = 0x2B_8000;
const EXAM_BASE_TILE: u16 = 0x0100;
const EXAM_NEXT_PATTERN_VRAM: u16 = 0x4000;
const EXAM_SURFACE_WIDTH: usize = 512;
const EXAM_SURFACE_HEIGHT: usize = 512;
const EXAM_TILE_COLUMNS: usize = 64;
const EXAM_TILE_ROWS: usize = 64;
const EXAM_PALETTE_LINE: u16 = 1;
const EXAM_PRIORITY_MASK: u16 = 0x8000;
const EXAM_PALETTE_MASK: u16 = 0x6000;
const EXAM_FLIP_MASK: u16 = 0x1800;
const EXAM_TILE_MASK: u16 = 0x07FF;
const EXAM_SUFFIX_ROUTINE_OFFSET: usize = 0x09_5D5E;
const EXAM_SUFFIX_DEFINITION_OFFSET: usize = 0x09_769E;
const EXAM_DIGIT_TABLE_OFFSET: usize = 0x09_76A8;
const EXAM_DRAW_CALL_TARGET: usize = 0x0000_1856;
const PREVIEW_SCALE: usize = 3;
const PREVIEW_GAP: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExamCardSummary {
    pub blocks: usize,
    pub source_text_pixels: usize,
    pub protected_pixels: usize,
    pub source_pattern_tiles: usize,
    pub appended_pattern_tiles: usize,
    pub changed_map_cells: usize,
    pub pattern_pack_bytes: usize,
    pub map_pack_bytes: usize,
    pub typed_code_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct ExamCardManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    source_pack_manifest: String,
    font_asset: String,
    font_sha256: String,
    target_banks: TargetBanks,
    output_surface: OutputSurface,
    tilemap: TilemapPlan,
    palette_line_words: Vec<String>,
    background_palette_indices: Vec<usize>,
    blocks: Vec<TextBlock>,
    score_digits: ScoreDigits,
    suffix_suppression: SuffixSuppression,
}

#[derive(Debug, Deserialize)]
struct TargetBanks {
    pattern_pack_offset: String,
    map_pack_offset: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
    tile_columns: usize,
    tile_rows: usize,
    paper_crop: PixelBounds,
}

#[derive(Debug, Deserialize)]
struct TilemapPlan {
    base_tile: String,
    palette_line: u16,
    priority: bool,
    next_pattern_vram: String,
}

#[derive(Debug, Deserialize)]
struct TextBlock {
    id: String,
    jp_text: String,
    ko: String,
    source_bounds: PixelBounds,
    source_palette_indices: Vec<usize>,
    expected_source_pixels: usize,
    content_box: PixelBounds,
    style: RenderStyle,
    glyph_size: usize,
    space_width: usize,
    target_palette_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RenderStyle {
    HeadingGraded,
    BodyCrisp,
}

#[derive(Debug, Deserialize)]
struct ScoreDigits {
    sprite_table_offset: String,
    digits: usize,
    source_y: String,
    source_size_link: String,
    source_first_tile: String,
    tiles_per_digit: usize,
    source_x: String,
    preview_text: String,
    preview_x: usize,
    preview_y: usize,
    preview_advance: usize,
}

#[derive(Debug, Deserialize)]
struct SuffixSuppression {
    routine_offset: String,
    sprite_definition_offset: String,
    draw_call_target: String,
    sprite_record: SpriteRecord,
}

#[derive(Debug, Deserialize)]
struct SpriteRecord {
    y: String,
    size_link: String,
    tile: String,
    x: String,
}

#[derive(Debug, Deserialize)]
struct SharedExamSourceManifest {
    source_packs: Vec<SourcePack>,
}

#[derive(Debug, Deserialize)]
struct SourcePack {
    id: String,
    header_offset: String,
    vram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
}

#[derive(Debug)]
struct CompiledSurface {
    map: Vec<u8>,
    patterns: Vec<u8>,
    appended_tiles: usize,
    changed_cells: usize,
}

#[derive(Debug)]
struct ExamCardBuild {
    manifest: ExamCardManifest,
    palette: [u16; 16],
    source_surface: Vec<u8>,
    target_surface: Vec<u8>,
    source_high_patterns: Vec<u8>,
    source_text_pixels: usize,
    protected_pixels: usize,
    compiled: CompiledSurface,
    pattern_header: [u8; 6],
    pattern_bank: Vec<u8>,
    map_header: [u8; 6],
    map_bank: Vec<u8>,
    suffix_expected: Vec<u8>,
    suffix_replacement: Vec<u8>,
}

/// Insert all four Korean result-card phrases into the cumulative JP-to-KR ROM.
pub fn apply_exam_card(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<ExamCardSummary, String> {
    let build = build_exam_card(source, assets_dir)?;
    let pattern_end = EXAM_PATTERN_BANK_OFFSET + build.pattern_bank.len();
    let map_end = EXAM_MAP_BANK_OFFSET + build.map_bank.len();
    if pattern_end > EXAM_MAP_BANK_OFFSET || map_end > EXAM_MAP_BANK_LIMIT || map_end > output.len()
    {
        return Err(format!(
            "exam card packs exceed their expanded banks: patterns end 0x{pattern_end:06X}, map end 0x{map_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(6);
    apply_expected_write(
        output,
        EXAM_LOW_HEADER_OFFSET,
        source_range(
            source,
            EXAM_LOW_HEADER_OFFSET,
            build.pattern_header.len(),
            "exam source low-pattern header",
        )?,
        &build.pattern_header,
        "exam card low-pattern header",
    )?;
    changed_ranges.push((
        EXAM_LOW_HEADER_OFFSET,
        EXAM_LOW_HEADER_OFFSET + build.pattern_header.len(),
    ));
    apply_expected_write(
        output,
        EXAM_MAP_HEADER_OFFSET,
        source_range(
            source,
            EXAM_MAP_HEADER_OFFSET,
            build.map_header.len(),
            "exam source map header",
        )?,
        &build.map_header,
        "exam card map header",
    )?;
    changed_ranges.push((
        EXAM_MAP_HEADER_OFFSET,
        EXAM_MAP_HEADER_OFFSET + build.map_header.len(),
    ));
    apply_expected_write(
        output,
        EXAM_SUFFIX_ROUTINE_OFFSET,
        &build.suffix_expected,
        &build.suffix_replacement,
        "exam dynamic JP score suffix suppression",
    )?;
    changed_ranges.push((
        EXAM_SUFFIX_ROUTINE_OFFSET,
        EXAM_SUFFIX_ROUTINE_OFFSET + build.suffix_replacement.len(),
    ));
    apply_expected_write(
        output,
        EXAM_PATTERN_BANK_OFFSET,
        &vec![0xFF; build.pattern_bank.len()],
        &build.pattern_bank,
        "exam card expanded low-pattern pack",
    )?;
    changed_ranges.push((EXAM_PATTERN_BANK_OFFSET, pattern_end));
    apply_expected_write(
        output,
        EXAM_MAP_BANK_OFFSET,
        &vec![0xFF; build.map_bank.len()],
        &build.map_bank,
        "exam card expanded map pack",
    )?;
    changed_ranges.push((EXAM_MAP_BANK_OFFSET, map_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after exam card",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let inserted_patterns = decode_mode1_pack_entry(output, EXAM_LOW_HEADER_OFFSET)?;
    let inserted_map = decode_mode1_pack_entry(output, EXAM_MAP_HEADER_OFFSET)?;
    if inserted_patterns.vram_destination != EXAM_LOW_VRAM
        || inserted_patterns.data != build.compiled.patterns
        || inserted_map.vram_destination != EXAM_MAP_VRAM
        || inserted_map.data != build.compiled.map
    {
        return Err("inserted exam card packs differ from the JP-derived plan".to_string());
    }
    let rebuilt =
        render_indexed_surface(&inserted_map.data, &inserted_patterns.data, &build.manifest)?;
    if rebuilt != build.target_surface {
        return Err("inserted exam card packs do not reconstruct the target surface".to_string());
    }
    if source_range(
        output,
        EXAM_SUFFIX_ROUTINE_OFFSET,
        build.suffix_replacement.len(),
        "inserted exam suffix patch",
    )? != build.suffix_replacement
    {
        return Err("inserted exam suffix patch differs from typed 68000 output".to_string());
    }

    eprintln!("JP graphics GFX-EXAM-CARD-* Expected Writes:");
    eprintln!(
        "  0x{EXAM_LOW_HEADER_OFFSET:06X}..0x{:06X}  exam low-pattern header ({} bytes)",
        EXAM_LOW_HEADER_OFFSET + build.pattern_header.len(),
        build.pattern_header.len()
    );
    eprintln!(
        "  0x{EXAM_MAP_HEADER_OFFSET:06X}..0x{:06X}  exam map header ({} bytes)",
        EXAM_MAP_HEADER_OFFSET + build.map_header.len(),
        build.map_header.len()
    );
    eprintln!(
        "  0x{EXAM_SUFFIX_ROUTINE_OFFSET:06X}..0x{:06X}  suppress JP `てん` ({} bytes, typed 68000)",
        EXAM_SUFFIX_ROUTINE_OFFSET + build.suffix_replacement.len(),
        build.suffix_replacement.len()
    );
    eprintln!(
        "  0x{EXAM_PATTERN_BANK_OFFSET:06X}..0x{pattern_end:06X}  exam low-pattern pack ({} bytes)",
        build.pattern_bank.len()
    );
    eprintln!(
        "  0x{EXAM_MAP_BANK_OFFSET:06X}..0x{map_end:06X}  exam map pack ({} bytes)",
        build.map_bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render JP (left) and Korean (right) result cards with a representative 100 score.
///
/// This is deterministic static QA evidence. It does not prove that the
/// conditioned exam-result scene was reached in an emulator.
pub fn write_exam_card_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<ExamCardSummary, String> {
    let build = build_exam_card(source, assets_dir)?;
    let mut jp = build.source_surface.clone();
    let mut kr = build.target_surface.clone();
    overlay_score(&mut jp, &build.source_high_patterns, &build.manifest, true)?;
    overlay_score(&mut kr, &build.source_high_patterns, &build.manifest, false)?;

    let crop = build.manifest.output_surface.paper_crop;
    let contact_width = crop.width * 2 + PREVIEW_GAP;
    let preview_width = contact_width * PREVIEW_SCALE;
    let preview_height = crop.height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[28, 28, 28, 255]);
    }
    for (panel, pixels) in [jp.as_slice(), kr.as_slice()].into_iter().enumerate() {
        let panel_x = panel * (crop.width + PREVIEW_GAP);
        for y in 0..crop.height {
            for x in 0..crop.width {
                let source_x = crop.x + x;
                let source_y = crop.y + y;
                let index = pixels[source_y * EXAM_SURFACE_WIDTH + source_x] as usize;
                let color = md_color(build.palette[index]);
                for scale_y in 0..PREVIEW_SCALE {
                    for scale_x in 0..PREVIEW_SCALE {
                        let destination_x = (panel_x + x) * PREVIEW_SCALE + scale_x;
                        let destination_y = y * PREVIEW_SCALE + scale_y;
                        let offset = (destination_y * preview_width + destination_x) * 4;
                        rgba[offset..offset + 4]
                            .copy_from_slice(&[color[0], color[1], color[2], 255]);
                    }
                }
            }
        }
    }
    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "exam card",
    )?;
    Ok(summary(&build, 0))
}

fn build_exam_card(source: &[u8], assets_dir: &Path) -> Result<ExamCardBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let shared = read_shared_sources(assets_dir, &manifest)?;
    validate_shared_sources(&shared)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "exam card")?;

    let mut payloads = BTreeMap::new();
    for declaration in &shared.source_packs {
        payloads.insert(
            declaration.id.clone(),
            checked_source_pack(source, declaration)?,
        );
    }
    let source_patterns = payloads
        .get("exam-patterns-low")
        .ok_or_else(|| "shared exam source has no low-pattern pack".to_string())?
        .clone();
    let source_map = payloads
        .get("exam-map")
        .ok_or_else(|| "shared exam source has no map pack".to_string())?
        .clone();
    let source_high_patterns = payloads
        .get("exam-patterns-high")
        .ok_or_else(|| "shared exam source has no high-pattern pack".to_string())?
        .clone();
    validate_source_pack_roundtrips(&source_patterns, &source_map, &manifest)?;
    let source_surface = render_indexed_surface(&source_map, &source_patterns, &manifest)?;

    let (mut target_surface, source_text_pixels) = erase_source_text(&source_surface, &manifest)?;
    let font = read_font(assets_dir, &manifest)?;
    for block in &manifest.blocks {
        render_block(
            &font,
            block,
            &manifest,
            &source_surface,
            &mut target_surface,
        )?;
    }
    let protected_pixels = validate_protected_surface(&source_surface, &target_surface, &manifest)?;
    let compiled = compile_preserving_source_prefix(
        &source_map,
        &source_patterns,
        &source_surface,
        &target_surface,
        &manifest,
    )?;
    let rebuilt = render_indexed_surface(&compiled.map, &compiled.patterns, &manifest)?;
    if rebuilt != target_surface {
        return Err("exam card compiler changed the planned indexed surface".to_string());
    }

    let pattern_encoded =
        encode_locked_mode1_pack(EXAM_PATTERN_BANK_OFFSET, EXAM_LOW_VRAM, &compiled.patterns)?;
    let map_encoded =
        encode_locked_mode1_bytes(EXAM_MAP_BANK_OFFSET, EXAM_MAP_VRAM, &compiled.map)?;
    validate_pack_roundtrip(
        EXAM_PATTERN_BANK_OFFSET,
        &pattern_encoded.header,
        &pattern_encoded.bank,
        EXAM_LOW_VRAM,
        &compiled.patterns,
    )?;
    validate_pack_roundtrip(
        EXAM_MAP_BANK_OFFSET,
        &map_encoded.header,
        &map_encoded.bank,
        EXAM_MAP_VRAM,
        &compiled.map,
    )?;

    validate_score_digits(source, &source_high_patterns, &manifest)?;
    let (suffix_expected, suffix_replacement) =
        validate_suffix_source_and_build_patch(source, &source_high_patterns, &manifest)?;

    Ok(ExamCardBuild {
        manifest,
        palette,
        source_surface,
        target_surface,
        source_high_patterns,
        source_text_pixels,
        protected_pixels,
        compiled,
        pattern_header: pattern_encoded.header,
        pattern_bank: pattern_encoded.bank,
        map_header: map_encoded.header,
        map_bank: map_encoded.bank,
        suffix_expected,
        suffix_replacement,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<ExamCardManifest, String> {
    let path = assets_dir.join("graphics_text/exam_card.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read exam card graphics source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid exam card source {}: {error}", path.display()))
}

fn read_shared_sources(
    assets_dir: &Path,
    manifest: &ExamCardManifest,
) -> Result<SharedExamSourceManifest, String> {
    let path = assets_dir.join(&manifest.source_pack_manifest);
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read shared exam graphics source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid shared exam source {}: {error}", path.display()))
}

fn read_font(assets_dir: &Path, manifest: &ExamCardManifest) -> Result<Font, String> {
    let path = assets_dir.join(&manifest.font_asset);
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let actual = sha256_hex(&bytes);
    if actual != manifest.font_sha256 {
        return Err(format!(
            "{}: exam card font SHA-256 mismatch: expected {}, got {actual}",
            path.display(),
            manifest.font_sha256
        ));
    }
    Font::from_bytes(bytes, FontSettings::default())
        .map_err(|error| format!("failed to parse exam card font: {error}"))
}

fn validate_manifest_shape(manifest: &ExamCardManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-EXAM-CARD"
        || !manifest.source_policy.contains("JP")
        || manifest.source_pack_manifest != "graphics_text/exam_seals.json"
        || manifest.font_asset != "neodgm.ttf"
        || manifest.font_sha256.len() != 64
    {
        return Err("unsupported exam card manifest identity".to_string());
    }
    if parse_hex(&manifest.target_banks.pattern_pack_offset)? != EXAM_PATTERN_BANK_OFFSET
        || parse_hex(&manifest.target_banks.map_pack_offset)? != EXAM_MAP_BANK_OFFSET
    {
        return Err("exam card target banks drifted".to_string());
    }
    let surface = manifest.output_surface;
    if surface.width != EXAM_SURFACE_WIDTH
        || surface.height != EXAM_SURFACE_HEIGHT
        || surface.tile_columns != EXAM_TILE_COLUMNS
        || surface.tile_rows != EXAM_TILE_ROWS
        || surface.width != surface.tile_columns * 8
        || surface.height != surface.tile_rows * 8
        || surface.paper_crop
            != (PixelBounds {
                x: 64,
                y: 0,
                width: 192,
                height: 224,
            })
    {
        return Err("exam card output surface drifted from the JP 64x64 plane".to_string());
    }
    if parse_u16_hex(&manifest.tilemap.base_tile)? != EXAM_BASE_TILE
        || manifest.tilemap.palette_line != EXAM_PALETTE_LINE
        || !manifest.tilemap.priority
        || parse_u16_hex(&manifest.tilemap.next_pattern_vram)? != EXAM_NEXT_PATTERN_VRAM
    {
        return Err("exam card tilemap plan drifted from the JP consumer".to_string());
    }
    if manifest
        .background_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        != BTreeSet::from([7, 8])
    {
        return Err("exam card parchment palette roles drifted".to_string());
    }

    let expected = [
        (
            "GFX-EXAM-CARD-HEADING",
            "卒園試験",
            "졸업시험",
            PixelBounds {
                x: 112,
                y: 24,
                width: 96,
                height: 24,
            },
            [3usize, 4, 5, 6].as_slice(),
            1_406,
            PixelBounds {
                x: 112,
                y: 24,
                width: 96,
                height: 24,
            },
            RenderStyle::HeadingGraded,
            22,
            [3usize, 4, 5, 6].as_slice(),
        ),
        (
            "GFX-EXAM-CARD-RESULT",
            "結果",
            "결과",
            PixelBounds {
                x: 144,
                y: 56,
                width: 32,
                height: 16,
            },
            [2usize].as_slice(),
            286,
            PixelBounds {
                x: 144,
                y: 56,
                width: 32,
                height: 16,
            },
            RenderStyle::BodyCrisp,
            16,
            [2usize].as_slice(),
        ),
        (
            "GFX-EXAM-CARD-SCORE-STEM",
            "あなたのせいせき",
            "성적은",
            PixelBounds {
                x: 96,
                y: 72,
                width: 128,
                height: 16,
            },
            [1usize].as_slice(),
            538,
            PixelBounds {
                x: 96,
                y: 72,
                width: 128,
                height: 24,
            },
            RenderStyle::BodyCrisp,
            16,
            [1usize].as_slice(),
        ),
        (
            "GFX-EXAM-CARD-COPULA",
            "です",
            "점입니다",
            PixelBounds {
                x: 192,
                y: 96,
                width: 32,
                height: 16,
            },
            [1usize].as_slice(),
            117,
            PixelBounds {
                x: 176,
                y: 96,
                width: 64,
                height: 16,
            },
            RenderStyle::BodyCrisp,
            16,
            [1usize].as_slice(),
        ),
    ];
    if manifest.blocks.len() != expected.len() {
        return Err("exam card manifest must declare exactly four phrases".to_string());
    }
    for (block, expected) in manifest.blocks.iter().zip(expected) {
        if block.id != expected.0
            || block.jp_text != expected.1
            || block.ko != expected.2
            || block.source_bounds != expected.3
            || block.source_palette_indices.as_slice() != expected.4
            || block.expected_source_pixels != expected.5
            || block.content_box != expected.6
            || block.style != expected.7
            || block.glyph_size != expected.8
            || block.space_width != 8
            || block.target_palette_indices.as_slice() != expected.9
        {
            return Err(format!("{} declaration drifted", expected.0));
        }
        validate_bounds(block.source_bounds, surface, &block.id)?;
        validate_bounds(block.content_box, surface, &block.id)?;
        if !contains_bounds(block.content_box, block.source_bounds) {
            return Err(format!(
                "{} source bounds escape its mutable content box",
                block.id
            ));
        }
    }
    for pair in manifest.blocks.iter().enumerate() {
        for other in manifest.blocks.iter().skip(pair.0 + 1) {
            if bounds_overlap(pair.1.content_box, other.content_box) {
                return Err(format!(
                    "exam card content boxes {} and {} overlap",
                    pair.1.id, other.id
                ));
            }
        }
    }

    let digits = &manifest.score_digits;
    if parse_hex(&digits.sprite_table_offset)? != EXAM_DIGIT_TABLE_OFFSET
        || digits.digits != 10
        || parse_u16_hex(&digits.source_y)? != 0x0080
        || parse_u16_hex(&digits.source_size_link)? != 0x0500
        || parse_u16_hex(&digits.source_first_tile)? != 0xA24A
        || digits.tiles_per_digit != 4
        || parse_u16_hex(&digits.source_x)? != 0x0080
        || digits.preview_text != "100"
        || digits.preview_x != 128
        || digits.preview_y != 96
        || digits.preview_advance != 16
    {
        return Err("exam score digit declaration drifted".to_string());
    }
    let suffix = &manifest.suffix_suppression;
    if parse_hex(&suffix.routine_offset)? != EXAM_SUFFIX_ROUTINE_OFFSET
        || parse_hex(&suffix.sprite_definition_offset)? != EXAM_SUFFIX_DEFINITION_OFFSET
        || parse_hex(&suffix.draw_call_target)? != EXAM_DRAW_CALL_TARGET
        || parse_u16_hex(&suffix.sprite_record.y)? != 0x00E0
        || parse_u16_hex(&suffix.sprite_record.size_link)? != 0x0500
        || parse_u16_hex(&suffix.sprite_record.tile)? != 0xA272
        || parse_u16_hex(&suffix.sprite_record.x)? != 0x0130
    {
        return Err("exam dynamic score suffix declaration drifted".to_string());
    }
    Ok(())
}

fn validate_shared_sources(shared: &SharedExamSourceManifest) -> Result<(), String> {
    let expected = [
        (
            "exam-patterns-low",
            EXAM_LOW_HEADER_OFFSET,
            EXAM_LOW_VRAM,
            EXAM_LOW_BYTES,
        ),
        (
            "exam-patterns-high",
            EXAM_HIGH_HEADER_OFFSET,
            EXAM_HIGH_VRAM,
            EXAM_HIGH_BYTES,
        ),
        (
            "exam-map",
            EXAM_MAP_HEADER_OFFSET,
            EXAM_MAP_VRAM,
            EXAM_MAP_BYTES,
        ),
        (
            "exam-window",
            EXAM_WINDOW_HEADER_OFFSET,
            EXAM_WINDOW_VRAM,
            EXAM_WINDOW_BYTES,
        ),
    ];
    if shared.source_packs.len() != expected.len() {
        return Err("shared exam source must declare all four transfers".to_string());
    }
    for (pack, expected) in shared.source_packs.iter().zip(expected) {
        if pack.id != expected.0
            || parse_hex(&pack.header_offset)? != expected.1
            || parse_u16_hex(&pack.vram_destination)? != expected.2
            || pack.decoded_bytes != expected.3
            || pack.decoded_sha256.len() != 64
        {
            return Err(format!("shared exam transfer {} drifted", expected.0));
        }
    }
    Ok(())
}

fn checked_source_pack(source: &[u8], declaration: &SourcePack) -> Result<Vec<u8>, String> {
    let header_offset = parse_hex(&declaration.header_offset)?;
    let decoded = decode_mode1_pack_entry(source, header_offset)?;
    let expected_vram = parse_u16_hex(&declaration.vram_destination)?;
    if decoded.vram_destination != expected_vram || decoded.data.len() != declaration.decoded_bytes
    {
        return Err(format!(
            "{} decoded as {} bytes at VRAM 0x{:04X}",
            declaration.id,
            decoded.data.len(),
            decoded.vram_destination
        ));
    }
    let hash = sha256_hex(&decoded.data);
    if hash != declaration.decoded_sha256 {
        return Err(format!(
            "{} decoded SHA-256 mismatch: expected {}, got {hash}",
            declaration.id, declaration.decoded_sha256
        ));
    }
    Ok(decoded.data)
}

fn validate_source_pack_roundtrips(
    patterns: &[u8],
    map: &[u8],
    manifest: &ExamCardManifest,
) -> Result<(), String> {
    let pattern_encoded =
        encode_locked_mode1_pack(EXAM_PATTERN_BANK_OFFSET, EXAM_LOW_VRAM, patterns)?;
    validate_pack_roundtrip(
        EXAM_PATTERN_BANK_OFFSET,
        &pattern_encoded.header,
        &pattern_encoded.bank,
        EXAM_LOW_VRAM,
        patterns,
    )?;
    let map_encoded = encode_locked_mode1_bytes(EXAM_MAP_BANK_OFFSET, EXAM_MAP_VRAM, map)?;
    validate_pack_roundtrip(
        EXAM_MAP_BANK_OFFSET,
        &map_encoded.header,
        &map_encoded.bank,
        EXAM_MAP_VRAM,
        map,
    )?;
    let surface = render_indexed_surface(map, patterns, manifest)?;
    if surface.len() != EXAM_SURFACE_WIDTH * EXAM_SURFACE_HEIGHT {
        return Err("exam source no-op surface round-trip failed".to_string());
    }
    Ok(())
}

fn render_indexed_surface(
    map: &[u8],
    patterns: &[u8],
    manifest: &ExamCardManifest,
) -> Result<Vec<u8>, String> {
    if map.len() != EXAM_MAP_BYTES
        || patterns.len() < EXAM_LOW_BYTES
        || !patterns.len().is_multiple_of(MD_TILE_BYTES)
    {
        return Err("exam card decoded map or pattern length is invalid".to_string());
    }
    let base_tile = parse_u16_hex(&manifest.tilemap.base_tile)?;
    let pattern_tiles = patterns.len() / MD_TILE_BYTES;
    let mut pixels = vec![0u8; EXAM_SURFACE_WIDTH * EXAM_SURFACE_HEIGHT];
    let mut used_tiles = BTreeSet::new();
    for cell in 0..EXAM_TILE_COLUMNS * EXAM_TILE_ROWS {
        let word = u16::from_be_bytes([map[cell * 2], map[cell * 2 + 1]]);
        let tile = word & EXAM_TILE_MASK;
        let palette_line = (word & EXAM_PALETTE_MASK) >> 13;
        let priority = word & EXAM_PRIORITY_MASK != 0;
        if palette_line != manifest.tilemap.palette_line || priority != manifest.tilemap.priority {
            return Err(format!(
                "exam card map cell {cell} has unsupported attributes 0x{word:04X}"
            ));
        }
        if tile < base_tile || tile as usize >= base_tile as usize + pattern_tiles {
            return Err(format!(
                "exam card map cell {cell} references tile 0x{tile:03X} outside the low-pattern transfer"
            ));
        }
        used_tiles.insert(tile);
        let relative = usize::from(tile - base_tile);
        let pattern = &patterns[relative * MD_TILE_BYTES..(relative + 1) * MD_TILE_BYTES];
        let horizontal_flip = word & 0x0800 != 0;
        let vertical_flip = word & 0x1000 != 0;
        let tile_x = cell % EXAM_TILE_COLUMNS;
        let tile_y = cell / EXAM_TILE_COLUMNS;
        for local_y in 0..8 {
            for local_x in 0..8 {
                let source_x = if horizontal_flip {
                    7 - local_x
                } else {
                    local_x
                };
                let source_y = if vertical_flip { 7 - local_y } else { local_y };
                let byte = pattern[source_y * 4 + source_x / 2];
                let pixel = if source_x.is_multiple_of(2) {
                    byte >> 4
                } else {
                    byte & 0x0F
                };
                let x = tile_x * 8 + local_x;
                let y = tile_y * 8 + local_y;
                pixels[y * EXAM_SURFACE_WIDTH + x] = pixel;
            }
        }
    }
    if patterns.len() == EXAM_LOW_BYTES
        && used_tiles
            != (EXAM_BASE_TILE..EXAM_BASE_TILE + EXAM_LOW_SOURCE_TILES as u16)
                .collect::<BTreeSet<_>>()
    {
        return Err("exam source map does not consume every declared low-pattern tile".to_string());
    }
    Ok(pixels)
}

fn erase_source_text(
    source: &[u8],
    manifest: &ExamCardManifest,
) -> Result<(Vec<u8>, usize), String> {
    let mut target = source.to_vec();
    let mut total = 0usize;
    for block in &manifest.blocks {
        let mut count = 0usize;
        for y in block.source_bounds.y..block.source_bounds.y + block.source_bounds.height {
            for x in block.source_bounds.x..block.source_bounds.x + block.source_bounds.width {
                let offset = y * EXAM_SURFACE_WIDTH + x;
                if block
                    .source_palette_indices
                    .contains(&(source[offset] as usize))
                {
                    target[offset] = nearest_background(source, x, y, manifest)?;
                    count += 1;
                }
            }
        }
        if count != block.expected_source_pixels {
            return Err(format!(
                "{} source text mask has {count} pixels, expected {}",
                block.id, block.expected_source_pixels
            ));
        }
        total += count;
    }
    Ok((target, total))
}

fn nearest_background(
    source: &[u8],
    x: usize,
    y: usize,
    manifest: &ExamCardManifest,
) -> Result<u8, String> {
    let crop = manifest.output_surface.paper_crop;
    let x_min = crop.x as isize;
    let y_min = crop.y as isize;
    let x_max = (crop.x + crop.width) as isize;
    let y_max = (crop.y + crop.height) as isize;
    for radius in 1isize..=64 {
        for delta_y in -radius..=radius {
            for delta_x in -radius..=radius {
                if delta_x.abs().max(delta_y.abs()) != radius {
                    continue;
                }
                let candidate_x = x as isize + delta_x;
                let candidate_y = y as isize + delta_y;
                if candidate_x < x_min
                    || candidate_x >= x_max
                    || candidate_y < y_min
                    || candidate_y >= y_max
                {
                    continue;
                }
                let pixel =
                    source[candidate_y as usize * EXAM_SURFACE_WIDTH + candidate_x as usize];
                if manifest
                    .background_palette_indices
                    .contains(&(pixel as usize))
                {
                    return Ok(pixel);
                }
            }
        }
    }
    Err(format!(
        "exam card could not recover parchment below source text pixel ({x}, {y})"
    ))
}

fn render_block(
    font: &Font,
    block: &TextBlock,
    manifest: &ExamCardManifest,
    source: &[u8],
    target: &mut [u8],
) -> Result<(), String> {
    let visible = match block.style {
        RenderStyle::HeadingGraded => render_heading(font, block, manifest, source, target)?,
        RenderStyle::BodyCrisp => render_body(font, block, manifest, source, target)?,
    };
    let minimum = block.ko.chars().filter(|ch| !ch.is_whitespace()).count() * 12;
    if visible < minimum {
        return Err(format!(
            "{} Korean rendering has only {visible} visible pixels",
            block.id
        ));
    }
    Ok(())
}

fn render_heading(
    font: &Font,
    block: &TextBlock,
    manifest: &ExamCardManifest,
    source: &[u8],
    target: &mut [u8],
) -> Result<usize, String> {
    if block.target_palette_indices.len() != 4 {
        return Err(format!("{} heading needs four graded colors", block.id));
    }
    let line_width = text_width(block)?;
    if line_width > block.content_box.width || block.glyph_size > block.content_box.height {
        return Err(format!("{} Korean heading does not fit", block.id));
    }
    let mut cursor_x = block.content_box.x + (block.content_box.width - line_width) / 2;
    let origin_y = block.content_box.y + (block.content_box.height - block.glyph_size) / 2;
    let mut visible = 0usize;
    for ch in block.ko.chars() {
        if ch.is_whitespace() {
            cursor_x += block.space_width;
            continue;
        }
        let glyph = jp_native::render_native_glyph(font, ch);
        let mut core = vec![false; block.glyph_size * block.glyph_size];
        for y in 0..block.glyph_size {
            for x in 0..block.glyph_size {
                let source_x = x * 16 / block.glyph_size;
                let source_y = y * 16 / block.glyph_size;
                core[y * block.glyph_size + x] = native_glyph_pixel(&glyph, source_x, source_y);
            }
        }
        for y in 0..block.glyph_size {
            for x in 0..block.glyph_size {
                let palette = if core[y * block.glyph_size + x] {
                    Some(block.target_palette_indices[0])
                } else {
                    let distance =
                        nearest_core_distance(&core, block.glyph_size, block.glyph_size, x, y, 2);
                    match distance {
                        Some(1) => Some(block.target_palette_indices[1]),
                        Some(2) if (x + y).is_multiple_of(2) => {
                            Some(block.target_palette_indices[2])
                        }
                        Some(2) => Some(block.target_palette_indices[3]),
                        _ => None,
                    }
                };
                if let Some(palette) = palette {
                    let destination_x = cursor_x + x;
                    let destination_y = origin_y + y;
                    if write_text_pixel(
                        source,
                        target,
                        destination_x,
                        destination_y,
                        palette,
                        block,
                        manifest,
                    )? {
                        visible += 1;
                    }
                }
            }
        }
        cursor_x += block.glyph_size;
    }
    Ok(visible)
}

fn render_body(
    font: &Font,
    block: &TextBlock,
    manifest: &ExamCardManifest,
    source: &[u8],
    target: &mut [u8],
) -> Result<usize, String> {
    if block.glyph_size != 16 || block.target_palette_indices.len() != 1 {
        return Err(format!("{} body text contract is invalid", block.id));
    }
    let line_width = text_width(block)?;
    if line_width > block.content_box.width || block.glyph_size > block.content_box.height {
        return Err(format!(
            "{} Korean body text is {line_width}px wide for a {}px box",
            block.id, block.content_box.width
        ));
    }
    let mut cursor_x = block.content_box.x + (block.content_box.width - line_width) / 2;
    let origin_y = block.content_box.y + (block.content_box.height - block.glyph_size) / 2;
    let mut visible = 0usize;
    for ch in block.ko.chars() {
        if ch.is_whitespace() {
            cursor_x += block.space_width;
            continue;
        }
        let glyph = jp_native::render_native_glyph(font, ch);
        for y in 0..16 {
            for x in 0..16 {
                if native_glyph_pixel(&glyph, x, y)
                    && write_text_pixel(
                        source,
                        target,
                        cursor_x + x,
                        origin_y + y,
                        block.target_palette_indices[0],
                        block,
                        manifest,
                    )?
                {
                    visible += 1;
                }
            }
        }
        cursor_x += block.glyph_size;
    }
    Ok(visible)
}

fn text_width(block: &TextBlock) -> Result<usize, String> {
    block.ko.chars().try_fold(0usize, |width, ch| {
        width
            .checked_add(if ch.is_whitespace() {
                block.space_width
            } else {
                block.glyph_size
            })
            .ok_or_else(|| format!("{} text width overflow", block.id))
    })
}

#[allow(clippy::too_many_arguments)]
fn write_text_pixel(
    source: &[u8],
    target: &mut [u8],
    x: usize,
    y: usize,
    palette_index: usize,
    block: &TextBlock,
    manifest: &ExamCardManifest,
) -> Result<bool, String> {
    if x >= EXAM_SURFACE_WIDTH || y >= EXAM_SURFACE_HEIGHT || palette_index >= 16 {
        return Err(format!(
            "{} rendering escaped the indexed surface",
            block.id
        ));
    }
    let offset = y * EXAM_SURFACE_WIDTH + x;
    let source_index = source[offset] as usize;
    let writable = manifest.background_palette_indices.contains(&source_index)
        || block.source_palette_indices.contains(&source_index);
    if !writable {
        return Ok(false);
    }
    target[offset] = palette_index as u8;
    Ok(true)
}

fn validate_protected_surface(
    source: &[u8],
    target: &[u8],
    manifest: &ExamCardManifest,
) -> Result<usize, String> {
    if source.len() != target.len() || source.len() != EXAM_SURFACE_WIDTH * EXAM_SURFACE_HEIGHT {
        return Err("exam card protected-surface length is invalid".to_string());
    }
    let mut protected = 0usize;
    for y in 0..EXAM_SURFACE_HEIGHT {
        for x in 0..EXAM_SURFACE_WIDTH {
            let offset = y * EXAM_SURFACE_WIDTH + x;
            let source_index = source[offset] as usize;
            let writable = manifest.blocks.iter().any(|block| {
                contains_point(block.content_box, x, y)
                    && (manifest.background_palette_indices.contains(&source_index)
                        || block.source_palette_indices.contains(&source_index))
            });
            if !writable {
                if source[offset] != target[offset] {
                    return Err(format!(
                        "exam card changed protected source pixel ({x}, {y})"
                    ));
                }
                protected += 1;
            }
        }
    }
    Ok(protected)
}

fn compile_preserving_source_prefix(
    source_map: &[u8],
    source_patterns: &[u8],
    source_surface: &[u8],
    target_surface: &[u8],
    manifest: &ExamCardManifest,
) -> Result<CompiledSurface, String> {
    if source_map.len() != EXAM_MAP_BYTES
        || source_patterns.len() != EXAM_LOW_BYTES
        || source_surface.len() != target_surface.len()
    {
        return Err("exam card compile inputs have invalid lengths".to_string());
    }
    let maximum_tiles = usize::from(EXAM_NEXT_PATTERN_VRAM - EXAM_LOW_VRAM) / MD_TILE_BYTES;
    let mut map = source_map.to_vec();
    let mut patterns = source_patterns.to_vec();
    let mut appended = BTreeMap::<Vec<u8>, u16>::new();
    let mut changed_cells = 0usize;
    for cell in 0..EXAM_TILE_COLUMNS * EXAM_TILE_ROWS {
        let tile_x = cell % EXAM_TILE_COLUMNS;
        let tile_y = cell / EXAM_TILE_COLUMNS;
        if cell_pixels_equal(source_surface, target_surface, tile_x, tile_y) {
            continue;
        }
        let pattern = encode_surface_cell(target_surface, tile_x, tile_y)?;
        let relative = if let Some(&relative) = appended.get(&pattern) {
            relative
        } else {
            let relative = u16::try_from(patterns.len() / MD_TILE_BYTES)
                .map_err(|_| "exam card uses too many patterns".to_string())?;
            if relative as usize >= maximum_tiles {
                return Err(format!(
                    "exam card needs {} patterns but only {maximum_tiles} fit below VRAM 0x{EXAM_NEXT_PATTERN_VRAM:04X}",
                    relative as usize + 1
                ));
            }
            patterns.extend_from_slice(&pattern);
            appended.insert(pattern, relative);
            relative
        };
        let original = u16::from_be_bytes([source_map[cell * 2], source_map[cell * 2 + 1]]);
        let replacement = (original & (EXAM_PRIORITY_MASK | EXAM_PALETTE_MASK))
            | (parse_u16_hex(&manifest.tilemap.base_tile)? + relative);
        map[cell * 2..cell * 2 + 2].copy_from_slice(&replacement.to_be_bytes());
        changed_cells += 1;
    }
    if patterns[..source_patterns.len()] != *source_patterns {
        return Err("exam card compiler changed the source pattern prefix".to_string());
    }
    for cell in 0..EXAM_TILE_COLUMNS * EXAM_TILE_ROWS {
        let tile_x = cell % EXAM_TILE_COLUMNS;
        let tile_y = cell / EXAM_TILE_COLUMNS;
        let changed = !cell_pixels_equal(source_surface, target_surface, tile_x, tile_y);
        let source_word = &source_map[cell * 2..cell * 2 + 2];
        let target_word = &map[cell * 2..cell * 2 + 2];
        if !changed && source_word != target_word {
            return Err(format!("exam card changed an unmodified map cell {cell}"));
        }
        if changed {
            let source_attributes = u16::from_be_bytes([source_word[0], source_word[1]]) & 0xE000;
            let target_attributes = u16::from_be_bytes([target_word[0], target_word[1]]) & 0xE000;
            if source_attributes != target_attributes {
                return Err(format!(
                    "exam card changed priority or palette on map cell {cell}"
                ));
            }
        }
    }
    Ok(CompiledSurface {
        map,
        patterns,
        appended_tiles: appended.len(),
        changed_cells,
    })
}

fn cell_pixels_equal(left: &[u8], right: &[u8], tile_x: usize, tile_y: usize) -> bool {
    (0..8).all(|local_y| {
        let start = (tile_y * 8 + local_y) * EXAM_SURFACE_WIDTH + tile_x * 8;
        left[start..start + 8] == right[start..start + 8]
    })
}

fn encode_surface_cell(pixels: &[u8], tile_x: usize, tile_y: usize) -> Result<Vec<u8>, String> {
    let mut pattern = vec![0u8; MD_TILE_BYTES];
    for local_y in 0..8 {
        for pair_x in 0..4 {
            let x = tile_x * 8 + pair_x * 2;
            let y = tile_y * 8 + local_y;
            let left = pixels[y * EXAM_SURFACE_WIDTH + x];
            let right = pixels[y * EXAM_SURFACE_WIDTH + x + 1];
            if left >= 16 || right >= 16 {
                return Err("exam card uses an invalid palette index".to_string());
            }
            pattern[local_y * 4 + pair_x] = (left << 4) | right;
        }
    }
    Ok(pattern)
}

fn validate_score_digits(
    source: &[u8],
    high_patterns: &[u8],
    manifest: &ExamCardManifest,
) -> Result<(), String> {
    let digits = &manifest.score_digits;
    let table = parse_hex(&digits.sprite_table_offset)?;
    let expected_y = parse_u16_hex(&digits.source_y)?;
    let expected_size = parse_u16_hex(&digits.source_size_link)?;
    let first_tile = parse_u16_hex(&digits.source_first_tile)?;
    let expected_x = parse_u16_hex(&digits.source_x)?;
    for digit in 0..digits.digits {
        let bytes = source_range(
            source,
            table + digit * 10,
            10,
            "exam score digit definition",
        )?;
        if u16::from_be_bytes([bytes[0], bytes[1]]) != 1
            || u16::from_be_bytes([bytes[2], bytes[3]]) != expected_y
            || u16::from_be_bytes([bytes[4], bytes[5]]) != expected_size
            || u16::from_be_bytes([bytes[6], bytes[7]])
                != first_tile + (digit * digits.tiles_per_digit) as u16
            || u16::from_be_bytes([bytes[8], bytes[9]]) != expected_x
        {
            return Err(format!("exam score digit {digit} definition drifted"));
        }
    }
    let last_tile = usize::from((first_tile & EXAM_TILE_MASK) - EXAM_HIGH_VRAM / 32)
        + digits.digits * digits.tiles_per_digit;
    if last_tile > high_patterns.len() / MD_TILE_BYTES {
        return Err("exam score digits escape the protected high-pattern transfer".to_string());
    }
    Ok(())
}

fn validate_suffix_source_and_build_patch(
    source: &[u8],
    high_patterns: &[u8],
    manifest: &ExamCardManifest,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let suffix = &manifest.suffix_suppression;
    let routine = parse_hex(&suffix.routine_offset)?;
    let definition = parse_hex(&suffix.sprite_definition_offset)?;
    let draw_target = parse_hex(&suffix.draw_call_target)?;
    let bytes = source_range(source, routine, 20, "exam dynamic score suffix routine")?;
    if u16::from_be_bytes([bytes[0], bytes[1]]) != 0x45F9
        || u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]) as usize != definition
        || u16::from_be_bytes([bytes[6], bytes[7]]) != 0x4241
        || u16::from_be_bytes([bytes[8], bytes[9]]) != 0x4242
        || u16::from_be_bytes([bytes[10], bytes[11]]) != 0x4243
        || u16::from_be_bytes([bytes[12], bytes[13]]) != 0x4EB9
        || u32::from_be_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]) as usize != draw_target
        || u16::from_be_bytes([bytes[18], bytes[19]]) != 0x4E75
    {
        return Err("exam dynamic score suffix routine drifted".to_string());
    }
    let record = &suffix.sprite_record;
    let expected_definition = [
        0x00,
        0x01,
        (parse_u16_hex(&record.y)? >> 8) as u8,
        parse_u16_hex(&record.y)? as u8,
        (parse_u16_hex(&record.size_link)? >> 8) as u8,
        parse_u16_hex(&record.size_link)? as u8,
        (parse_u16_hex(&record.tile)? >> 8) as u8,
        parse_u16_hex(&record.tile)? as u8,
        (parse_u16_hex(&record.x)? >> 8) as u8,
        parse_u16_hex(&record.x)? as u8,
    ];
    if source_range(
        source,
        definition,
        10,
        "exam score suffix sprite definition",
    )? != expected_definition
    {
        return Err("exam score suffix sprite definition drifted".to_string());
    }
    let suffix_tile =
        usize::from((parse_u16_hex(&record.tile)? & EXAM_TILE_MASK) - EXAM_HIGH_VRAM / 32);
    let suffix_tiles = sprite_tile_count(parse_u16_hex(&record.size_link)?);
    if suffix_tile + suffix_tiles > high_patterns.len() / MD_TILE_BYTES {
        return Err("exam score suffix escapes the protected high-pattern transfer".to_string());
    }

    let replacement = m68k::assemble(&[Inst::Rts])?;
    if replacement.len() != 2 {
        return Err("typed exam suffix suppression is not one 68000 instruction".to_string());
    }
    Ok((bytes[..replacement.len()].to_vec(), replacement))
}

fn overlay_score(
    surface: &mut [u8],
    high_patterns: &[u8],
    manifest: &ExamCardManifest,
    include_jp_suffix: bool,
) -> Result<(), String> {
    for (index, ch) in manifest.score_digits.preview_text.chars().enumerate() {
        let digit = ch
            .to_digit(10)
            .ok_or_else(|| "exam score preview contains a non-digit".to_string())?
            as usize;
        let tile = parse_u16_hex(&manifest.score_digits.source_first_tile)?
            + (digit * manifest.score_digits.tiles_per_digit) as u16;
        draw_sprite(
            surface,
            high_patterns,
            tile,
            parse_u16_hex(&manifest.score_digits.source_size_link)?,
            manifest.score_digits.preview_x + index * manifest.score_digits.preview_advance,
            manifest.score_digits.preview_y,
        )?;
    }
    if include_jp_suffix {
        let record = &manifest.suffix_suppression.sprite_record;
        let x = usize::from(
            parse_u16_hex(&record.x)?
                .checked_sub(0x0080)
                .ok_or_else(|| {
                    "exam suffix preview X precedes the VDP sprite origin".to_string()
                })?,
        );
        let y = usize::from(
            parse_u16_hex(&record.y)?
                .checked_sub(0x0080)
                .ok_or_else(|| {
                    "exam suffix preview Y precedes the VDP sprite origin".to_string()
                })?,
        );
        draw_sprite(
            surface,
            high_patterns,
            parse_u16_hex(&record.tile)?,
            parse_u16_hex(&record.size_link)?,
            x,
            y,
        )?;
    }
    Ok(())
}

fn draw_sprite(
    surface: &mut [u8],
    patterns: &[u8],
    tile_word: u16,
    size_link: u16,
    origin_x: usize,
    origin_y: usize,
) -> Result<(), String> {
    if tile_word & EXAM_FLIP_MASK != 0 {
        return Err("exam score preview does not admit flipped sprites".to_string());
    }
    let size = (size_link >> 8) as u8;
    let width = usize::from((size >> 2) & 0x03) + 1;
    let height = usize::from(size & 0x03) + 1;
    let base_tile = tile_word & EXAM_TILE_MASK;
    let high_base_tile = EXAM_HIGH_VRAM / MD_TILE_BYTES as u16;
    if base_tile < high_base_tile {
        return Err("exam score sprite precedes the high-pattern transfer".to_string());
    }
    let relative = usize::from(base_tile - high_base_tile);
    for tile_x in 0..width {
        for tile_y in 0..height {
            let tile = relative + tile_x * height + tile_y;
            let pattern = patterns
                .get(tile * MD_TILE_BYTES..(tile + 1) * MD_TILE_BYTES)
                .ok_or_else(|| "exam score sprite tile escapes its transfer".to_string())?;
            for local_y in 0..8 {
                for local_x in 0..8 {
                    let byte = pattern[local_y * 4 + local_x / 2];
                    let pixel = if local_x.is_multiple_of(2) {
                        byte >> 4
                    } else {
                        byte & 0x0F
                    };
                    if pixel == 0 {
                        continue;
                    }
                    let x = origin_x + tile_x * 8 + local_x;
                    let y = origin_y + tile_y * 8 + local_y;
                    if x >= EXAM_SURFACE_WIDTH || y >= EXAM_SURFACE_HEIGHT {
                        return Err("exam score preview sprite escapes the surface".to_string());
                    }
                    surface[y * EXAM_SURFACE_WIDTH + x] = pixel;
                }
            }
        }
    }
    Ok(())
}

fn sprite_tile_count(size_link: u16) -> usize {
    let size = (size_link >> 8) as u8;
    (usize::from((size >> 2) & 0x03) + 1) * (usize::from(size & 0x03) + 1)
}

fn validate_pack_roundtrip(
    bank_offset: usize,
    header: &[u8; 6],
    bank: &[u8],
    expected_vram: u16,
    expected_payload: &[u8],
) -> Result<(), String> {
    let mut probe = vec![0u8; bank_offset + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[bank_offset..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != expected_vram || decoded.data != expected_payload {
        return Err("exam card mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn validate_bounds(bounds: PixelBounds, surface: OutputSurface, label: &str) -> Result<(), String> {
    if bounds.width == 0
        || bounds.height == 0
        || bounds.x + bounds.width > surface.width
        || bounds.y + bounds.height > surface.height
    {
        return Err(format!("{label} has invalid pixel bounds"));
    }
    Ok(())
}

fn contains_bounds(outer: PixelBounds, inner: PixelBounds) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

fn contains_point(bounds: PixelBounds, x: usize, y: usize) -> bool {
    x >= bounds.x && x < bounds.x + bounds.width && y >= bounds.y && y < bounds.y + bounds.height
}

fn bounds_overlap(left: PixelBounds, right: PixelBounds) -> bool {
    left.x < right.x + right.width
        && right.x < left.x + left.width
        && left.y < right.y + right.height
        && right.y < left.y + left.height
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn summary(build: &ExamCardBuild, checksum: u16) -> ExamCardSummary {
    ExamCardSummary {
        blocks: build.manifest.blocks.len(),
        source_text_pixels: build.source_text_pixels,
        protected_pixels: build.protected_pixels,
        source_pattern_tiles: EXAM_LOW_SOURCE_TILES,
        appended_pattern_tiles: build.compiled.appended_tiles,
        changed_map_cells: build.compiled.changed_cells,
        pattern_pack_bytes: build.pattern_bank.len(),
        map_pack_bytes: build.map_bank.len(),
        typed_code_bytes: build.suffix_replacement.len(),
        checksum,
    }
}
