//! JP-source original ending-credit page compiler.
//!
//! Every original page starts with the same 22x2-tile, 176x16 heading surface
//! in tiles 1..45 of its first transfer.  The page packs differ in length and
//! in their artwork/timed-line data, so each exact JP payload is decoded and
//! checked independently before the heading and declared timed cells are
//! rebuilt together in one cumulative page bank.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use crate::m68k::{self, AddressReg, Inst};

use super::credits_generic::{CreditsGenericBuild, compile_credits_generic};
use super::credits_native_frames::{CreditsNativeFramesBuild, compile_credits_native_frames};
use super::credits_remaining::{CreditsRemainingBuild, compile_credits_remaining};
use super::credits_timed::{CreditsTimedBuild, compile_credits_timed};
use super::font_effect::{
    CREDIT_PREVIEW_INK, native_text_width, read_verified_font, render_native_text_line,
};
use super::pixel::{decode_md_tiles_column_major, encode_md_tiles_column_major, write_rgba_png};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, MODE1_CHAIN_TERMINATOR, MODE1_SUBHEADER_BYTES,
    apply_expected_write, calculate_checksum, decode_mode1_pack_entry, parse_hex, sha256_hex,
    source_range, validate_only_ranges_changed,
};

const PAGE_COUNT: usize = 10;
const HEADER_BYTES: usize = 6;
const LOADER_BYTES: usize = 12;
const LOADER_TARGET: u32 = 0x0000_0A72;
const SURFACE_WIDTH_TILES: usize = 22;
const SURFACE_HEIGHT_TILES: usize = 2;
const SURFACE_WIDTH_PIXELS: usize = SURFACE_WIDTH_TILES * 8;
const SURFACE_HEIGHT_PIXELS: usize = SURFACE_HEIGHT_TILES * 8;
const SPACE_WIDTH_PIXELS: usize = 8;
const MUTABLE_TILE_START: usize = 1;
const MUTABLE_TILE_END: usize = 45;
const MUTABLE_TILES: usize = MUTABLE_TILE_END - MUTABLE_TILE_START;
const MUTABLE_BYTES: usize = MUTABLE_TILES * MD_TILE_BYTES;
const BANK_OFFSET: usize = 0x30_0000;
const BANK_LIMIT: usize = 0x31_0000;
const POINTER_ENTRY_BYTES: usize = 2;
const PAYLOAD_START_ALIGNMENT: usize = 0x80;
const BANK_FILL: u8 = 0xFF;
const PREVIEW_SCALE: usize = 3;
const PREVIEW_COLUMN_GAP: usize = 16;
const PREVIEW_ROW_GAP: usize = 8;

const HEADER_OFFSETS: [usize; PAGE_COUNT] = [
    0x09_DF7E, 0x09_DF9E, 0x09_DFBE, 0x09_DFF2, 0x09_E020, 0x09_E040, 0x09_E054, 0x09_E06E,
    0x09_E088, 0x09_E096,
];
const LOADER_OFFSETS: [usize; PAGE_COUNT] = [
    0x09_7C1E, 0x09_7CEC, 0x09_7D84, 0x09_7F74, 0x09_80FC, 0x09_8222, 0x09_8382, 0x09_841A,
    0x09_85D0, 0x09_8678,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditsTopSummary {
    pub headings: usize,
    pub timed_lines: usize,
    pub relocated_timed_lines: usize,
    pub unique_glyphs: usize,
    pub decoded_bytes: usize,
    pub rewritten_tiles: usize,
    pub overwritten_timed_tiles: usize,
    pub appended_timed_tiles: usize,
    pub protected_bytes: usize,
    pub verified_loader_bytes: usize,
    pub verified_timed_consumer_bytes: usize,
    pub verified_generic_consumer_bytes: usize,
    pub verified_native_frame_consumer_bytes: usize,
    pub verified_remaining_consumer_bytes: usize,
    pub written_executable_bytes: usize,
    pub map_bank_bytes: usize,
    pub generic_table_bank_bytes: usize,
    pub native_frame_table_bank_bytes: usize,
    pub remaining_table_bank_bytes: usize,
    pub remaining_pack_bank_bytes: usize,
    pub pack_bank_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct CreditsTopManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    font_asset: String,
    font_sha256: String,
    render_mode: String,
    space_width_px: usize,
    surface: SurfaceDeclaration,
    target_bank: TargetBankDeclaration,
    consumer: ConsumerDeclaration,
    pages: Vec<PageDeclaration>,
}

#[derive(Debug, Deserialize)]
struct SurfaceDeclaration {
    width_tiles: usize,
    height_tiles: usize,
    storage: String,
    mutable_tile_start: usize,
    mutable_tile_end_exclusive: usize,
    source_palette_indices: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct TargetBankDeclaration {
    offset: String,
    limit: String,
    pointer_entry_bytes: usize,
    payload_start_alignment: String,
    fill_byte: String,
}

#[derive(Debug, Deserialize)]
struct ConsumerDeclaration {
    loader_target: String,
    loader_bytes: usize,
}

#[derive(Debug, Deserialize)]
struct PageDeclaration {
    id: String,
    page: usize,
    jp: String,
    ko: String,
    header_offset: String,
    header_sha256: String,
    decoded_bytes: usize,
    decoded_sha256: String,
    protected_prefix_sha256: String,
    mutable_heading_sha256: String,
    protected_suffix_sha256: String,
    loader_offset: String,
    loader_sha256: String,
}

#[derive(Debug)]
struct PageBuild {
    source: Vec<u8>,
    target: Vec<u8>,
    source_surface: Vec<u8>,
    target_surface: Vec<u8>,
    target_header: [u8; HEADER_BYTES],
}

#[derive(Debug)]
struct CreditsTopBuild {
    manifest: CreditsTopManifest,
    pages: Vec<PageBuild>,
    timed: CreditsTimedBuild,
    generic: CreditsGenericBuild,
    native_frames: CreditsNativeFramesBuild,
    remaining: CreditsRemainingBuild,
    bank: Vec<u8>,
    unique_glyphs: usize,
}

pub fn apply_credits_top(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<CreditsTopSummary, String> {
    let build = build_credits_top(source, assets_dir)?;
    let bank_end = BANK_OFFSET + build.bank.len();
    if bank_end > BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "credit-heading pack bank ends outside its allocation at 0x{bank_end:06X}"
        ));
    }
    let map_bank_offset = build.timed.map_bank_offset();
    let map_bank_end = map_bank_offset + build.timed.map_bank().len();
    let generic_table_bank_offset = build.generic.table_bank_offset();
    let generic_table_bank_end = generic_table_bank_offset + build.generic.table_bank().len();
    let native_frame_table_bank_offset = build.native_frames.table_bank_offset();
    let native_frame_table_bank_end =
        native_frame_table_bank_offset + build.native_frames.table_bank().len();
    let remaining_table_bank_offset = build.remaining.table_bank_offset();
    let remaining_table_bank_end = remaining_table_bank_offset + build.remaining.table_bank().len();
    if map_bank_end > generic_table_bank_offset || map_bank_end > output.len() {
        return Err(format!(
            "timed-credit map bank ends outside its allocation at 0x{map_bank_end:06X}"
        ));
    }
    if generic_table_bank_end > native_frame_table_bank_offset
        || generic_table_bank_end > output.len()
    {
        return Err(format!(
            "generic-credit table bank ends outside its allocation at 0x{generic_table_bank_end:06X}"
        ));
    }
    if native_frame_table_bank_end > BANK_OFFSET || native_frame_table_bank_end > output.len() {
        return Err(format!(
            "native-frame credit table bank ends outside its allocation at 0x{native_frame_table_bank_end:06X}"
        ));
    }
    if remaining_table_bank_end > output.len() {
        return Err(format!(
            "remaining-credit table bank ends outside the ROM at 0x{remaining_table_bank_end:06X}"
        ));
    }
    for patch in build.remaining.pack_patches() {
        if patch.bank_offset + patch.bank.len() > patch.bank_limit
            || patch.bank_offset + patch.bank.len() > output.len()
        {
            return Err(format!(
                "{} pack ends outside its allocation at 0x{:06X}",
                patch.id,
                patch.bank_offset + patch.bank.len()
            ));
        }
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(
        PAGE_COUNT
            + build.timed.instruction_patches().len()
            + build.generic.data_patches().len()
            + build.native_frames.data_patches().len()
            + build.remaining.data_patches().len()
            + build.remaining.pack_patches().len() * 2
            + 6,
    );
    for (declaration, page) in build.manifest.pages.iter().zip(&build.pages) {
        let header_offset = parse_hex(&declaration.header_offset)?;
        apply_expected_write(
            output,
            header_offset,
            source_range(
                source,
                header_offset,
                HEADER_BYTES,
                "credit-heading JP header",
            )?,
            &page.target_header,
            &format!("{} Korean heading header", declaration.id),
        )?;
        changed_ranges.push((header_offset, header_offset + HEADER_BYTES));
    }
    if !build.timed.map_bank().is_empty() {
        apply_expected_write(
            output,
            map_bank_offset,
            &vec![BANK_FILL; build.timed.map_bank().len()],
            build.timed.map_bank(),
            "timed-credit relocated sprite-map bank",
        )?;
        changed_ranges.push((map_bank_offset, map_bank_end));
    }
    for patch in build.timed.instruction_patches() {
        apply_expected_write(
            output,
            patch.offset,
            &patch.source,
            &patch.target,
            &patch.label,
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.target.len()));
    }
    if !build.generic.table_bank().is_empty() {
        apply_expected_write(
            output,
            generic_table_bank_offset,
            &vec![BANK_FILL; build.generic.table_bank().len()],
            build.generic.table_bank(),
            "generic-credit relocated frame-table bank",
        )?;
        changed_ranges.push((generic_table_bank_offset, generic_table_bank_end));
    }
    for patch in build.generic.data_patches() {
        apply_expected_write(
            output,
            patch.offset,
            &patch.source,
            &patch.target,
            &patch.label,
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.target.len()));
    }
    if !build.native_frames.table_bank().is_empty() {
        apply_expected_write(
            output,
            native_frame_table_bank_offset,
            &vec![BANK_FILL; build.native_frames.table_bank().len()],
            build.native_frames.table_bank(),
            "native-frame credit relocated frame-table bank",
        )?;
        changed_ranges.push((native_frame_table_bank_offset, native_frame_table_bank_end));
    }
    for patch in build.native_frames.data_patches() {
        apply_expected_write(
            output,
            patch.offset,
            &patch.source,
            &patch.target,
            &patch.label,
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.target.len()));
    }
    if !build.remaining.table_bank().is_empty() {
        apply_expected_write(
            output,
            remaining_table_bank_offset,
            &vec![BANK_FILL; build.remaining.table_bank().len()],
            build.remaining.table_bank(),
            "remaining-credit relocated frame-table bank",
        )?;
        changed_ranges.push((remaining_table_bank_offset, remaining_table_bank_end));
    }
    for patch in build.remaining.data_patches() {
        apply_expected_write(
            output,
            patch.offset,
            &patch.source,
            &patch.target,
            &patch.label,
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.target.len()));
    }
    for patch in build.remaining.pack_patches() {
        apply_expected_write(
            output,
            patch.header_offset,
            &patch.source_header,
            &patch.target_header,
            &format!("{} Korean transfer header", patch.id),
        )?;
        changed_ranges.push((
            patch.header_offset,
            patch.header_offset + patch.target_header.len(),
        ));
        apply_expected_write(
            output,
            patch.bank_offset,
            &vec![BANK_FILL; patch.bank.len()],
            &patch.bank,
            &format!("{} expanded pack bank", patch.id),
        )?;
        changed_ranges.push((patch.bank_offset, patch.bank_offset + patch.bank.len()));
    }
    apply_expected_write(
        output,
        BANK_OFFSET,
        &vec![BANK_FILL; build.bank.len()],
        &build.bank,
        "credit-heading expanded pack bank",
    )?;
    changed_ranges.push((BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after credit headings",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    for (declaration, page) in build.manifest.pages.iter().zip(&build.pages) {
        let header_offset = parse_hex(&declaration.header_offset)?;
        let inserted = decode_mode1_pack_entry(output, header_offset)?;
        if inserted.vram_destination != 0 || inserted.data != page.target {
            return Err(format!(
                "{} inserted heading pack differs from its compiled payload",
                declaration.id
            ));
        }
        validate_loader(output, declaration, "output")?;
    }
    build
        .timed
        .validate_output_consumers(source, output, "output")?;
    build
        .generic
        .validate_output_consumers(source, output, "output")?;
    build
        .native_frames
        .validate_output_consumers(source, output, "output")?;
    build
        .remaining
        .validate_output_consumers(source, output, "output")?;

    eprintln!("JP graphics GFX-CREDITS cumulative Expected Writes:");
    for declaration in &build.manifest.pages {
        let offset = parse_hex(&declaration.header_offset)?;
        eprintln!(
            "  0x{offset:06X}..0x{:06X}  {} first-transfer header",
            offset + HEADER_BYTES,
            declaration.id
        );
    }
    eprintln!(
        "  0x{BANK_OFFSET:06X}..0x{bank_end:06X}  ten-stream shared credit-page bank ({} bytes)",
        build.bank.len()
    );
    if !build.timed.map_bank().is_empty() {
        eprintln!(
            "  0x{map_bank_offset:06X}..0x{map_bank_end:06X}  relocated timed-name maps ({} bytes)",
            build.timed.map_bank().len()
        );
    }
    for patch in build.timed.instruction_patches() {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {}",
            patch.offset,
            patch.offset + patch.target.len(),
            patch.label
        );
    }
    if !build.generic.table_bank().is_empty() {
        eprintln!(
            "  0x{generic_table_bank_offset:06X}..0x{generic_table_bank_end:06X}  relocated generic frame table ({} bytes)",
            build.generic.table_bank().len()
        );
    }
    for patch in build.generic.data_patches() {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {}",
            patch.offset,
            patch.offset + patch.target.len(),
            patch.label
        );
    }
    if !build.native_frames.table_bank().is_empty() {
        eprintln!(
            "  0x{native_frame_table_bank_offset:06X}..0x{native_frame_table_bank_end:06X}  relocated native-frame tables ({} bytes)",
            build.native_frames.table_bank().len()
        );
    }
    for patch in build.native_frames.data_patches() {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {}",
            patch.offset,
            patch.offset + patch.target.len(),
            patch.label
        );
    }
    if !build.remaining.table_bank().is_empty() {
        eprintln!(
            "  0x{remaining_table_bank_offset:06X}..0x{remaining_table_bank_end:06X}  remaining relocated credit tables ({} bytes)",
            build.remaining.table_bank().len()
        );
    }
    for patch in build.remaining.data_patches() {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {}",
            patch.offset,
            patch.offset + patch.target.len(),
            patch.label
        );
    }
    for patch in build.remaining.pack_patches() {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {} transfer header",
            patch.header_offset,
            patch.header_offset + patch.target_header.len(),
            patch.id
        );
        eprintln!(
            "  0x{:06X}..0x{:06X}  {} expanded pack ({} bytes)",
            patch.bank_offset,
            patch.bank_offset + patch.bank.len(),
            patch.id,
            patch.bank.len()
        );
    }
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render all ten exact JP/Korean heading surfaces side by side.
pub fn write_credits_top_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<CreditsTopSummary, String> {
    let build = build_credits_top(source, assets_dir)?;
    let logical_width = SURFACE_WIDTH_PIXELS * 2 + PREVIEW_COLUMN_GAP;
    let logical_height = PAGE_COUNT * SURFACE_HEIGHT_PIXELS + (PAGE_COUNT - 1) * PREVIEW_ROW_GAP;
    let width = logical_width * PREVIEW_SCALE;
    let height = logical_height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; width * height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[30, 15, 18, 255]);
    }
    for (row, page) in build.pages.iter().enumerate() {
        let y = row * (SURFACE_HEIGHT_PIXELS + PREVIEW_ROW_GAP);
        draw_surface(
            &mut rgba,
            width,
            0,
            y,
            &page.source_surface,
            CREDIT_PREVIEW_INK,
        )?;
        draw_surface(
            &mut rgba,
            width,
            SURFACE_WIDTH_PIXELS + PREVIEW_COLUMN_GAP,
            y,
            &page.target_surface,
            CREDIT_PREVIEW_INK,
        )?;
    }
    write_rgba_png(
        output_path,
        width as u32,
        height as u32,
        &rgba,
        "credit-heading static preview",
    )?;
    Ok(summary(&build, 0))
}

/// Render the currently integrated fixed-cell timed names side by side.
pub fn write_credits_timed_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<CreditsTopSummary, String> {
    let build = build_credits_top(source, assets_dir)?;
    let mut additional_lines = build.generic.preview_lines().to_vec();
    additional_lines.extend(build.native_frames.preview_lines().cloned());
    additional_lines.extend(build.remaining.preview_lines().iter().cloned());
    build.timed.write_preview(&additional_lines, output_path)?;
    Ok(summary(&build, 0))
}

fn build_credits_top(source: &[u8], assets_dir: &Path) -> Result<CreditsTopBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "credit heading",
    )?;

    let mut pages = Vec::with_capacity(PAGE_COUNT);
    let mut unique_glyphs = BTreeSet::new();
    for declaration in &manifest.pages {
        let source_payload = checked_source_page(source, declaration)?;
        validate_loader(source, declaration, "JP source")?;
        let target = compile_heading(&font, &source_payload, declaration, &manifest)?;
        validate_target_ownership(&source_payload, &target, declaration)?;
        unique_glyphs.extend(
            declaration
                .ko
                .chars()
                .filter(|character| !character.is_whitespace()),
        );
        let mutable_start = MUTABLE_TILE_START * MD_TILE_BYTES;
        let mutable_end = MUTABLE_TILE_END * MD_TILE_BYTES;
        let source_surface = decode_md_tiles_column_major(
            &source_payload[mutable_start..mutable_end],
            SURFACE_WIDTH_PIXELS,
            SURFACE_HEIGHT_PIXELS,
            "credit-heading JP surface",
        )?;
        let target_surface = decode_md_tiles_column_major(
            &target[mutable_start..mutable_end],
            SURFACE_WIDTH_PIXELS,
            SURFACE_HEIGHT_PIXELS,
            "credit-heading Korean surface",
        )?;
        pages.push(PageBuild {
            source: source_payload,
            target,
            source_surface,
            target_surface,
            target_header: [0; HEADER_BYTES],
        });
    }

    let source_pages = pages
        .iter()
        .map(|page| page.source.clone())
        .collect::<Vec<_>>();
    let mut target_pages = pages
        .iter()
        .map(|page| page.target.clone())
        .collect::<Vec<_>>();
    let timed = compile_credits_timed(source, &source_pages, &mut target_pages, assets_dir)?;
    let generic = compile_credits_generic(source, &source_pages, &mut target_pages, assets_dir)?;
    let native_frames =
        compile_credits_native_frames(source, &source_pages, &mut target_pages, assets_dir)?;
    let remaining =
        compile_credits_remaining(source, &source_pages, &mut target_pages, assets_dir)?;
    for (page, target) in pages.iter_mut().zip(target_pages) {
        page.target = target;
    }
    unique_glyphs.extend(timed.glyphs());
    unique_glyphs.extend(generic.glyphs());
    unique_glyphs.extend(native_frames.glyphs());
    unique_glyphs.extend(remaining.glyphs());

    let payloads = pages
        .iter()
        .map(|page| page.target.as_slice())
        .collect::<Vec<_>>();
    let (headers, bank) = encode_shared_mode1_bank(&payloads)?;
    for (page, header) in pages.iter_mut().zip(headers) {
        page.target_header = header;
    }

    Ok(CreditsTopBuild {
        manifest,
        pages,
        timed,
        generic,
        native_frames,
        remaining,
        bank,
        unique_glyphs: unique_glyphs.len(),
    })
}

fn read_manifest(assets_dir: &Path) -> Result<CreditsTopManifest, String> {
    let path = assets_dir.join("graphics_text/credits_top.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read credit-heading source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid credit-heading source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &CreditsTopManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-CREDITS-TOP"
        || !manifest.source_policy.contains("English VWF")
        || manifest.font_asset != "neodgm.ttf"
        || manifest.font_sha256.len() != 64
        || manifest.render_mode != "jp_native_16x16_no_horizontal_scaling"
        || manifest.space_width_px != SPACE_WIDTH_PIXELS
        || manifest.pages.len() != PAGE_COUNT
    {
        return Err("credit-heading manifest identity drifted".to_string());
    }
    if manifest.surface.width_tiles != SURFACE_WIDTH_TILES
        || manifest.surface.height_tiles != SURFACE_HEIGHT_TILES
        || manifest.surface.storage != "column_major"
        || manifest.surface.mutable_tile_start != MUTABLE_TILE_START
        || manifest.surface.mutable_tile_end_exclusive != MUTABLE_TILE_END
        || manifest.surface.source_palette_indices != [0, 15]
    {
        return Err("credit-heading surface contract drifted".to_string());
    }
    if parse_hex(&manifest.target_bank.offset)? != BANK_OFFSET
        || parse_hex(&manifest.target_bank.limit)? != BANK_LIMIT
        || manifest.target_bank.pointer_entry_bytes != POINTER_ENTRY_BYTES
        || parse_hex(&manifest.target_bank.payload_start_alignment)? != PAYLOAD_START_ALIGNMENT
        || parse_u8_hex(&manifest.target_bank.fill_byte)? != BANK_FILL
    {
        return Err("credit-heading target bank declaration drifted".to_string());
    }
    if parse_u32_hex(&manifest.consumer.loader_target)? != LOADER_TARGET
        || manifest.consumer.loader_bytes != LOADER_BYTES
    {
        return Err("credit-heading consumer declaration drifted".to_string());
    }

    for (index, page) in manifest.pages.iter().enumerate() {
        if page.page != index
            || page.id != format!("GFX-CREDITS-P{index:02}-TOP")
            || parse_hex(&page.header_offset)? != HEADER_OFFSETS[index]
            || parse_hex(&page.loader_offset)? != LOADER_OFFSETS[index]
            || page.jp.is_empty()
            || page.ko.is_empty()
            || native_text_width(&page.ko, manifest.space_width_px) > SURFACE_WIDTH_PIXELS
            || page.decoded_bytes < MUTABLE_TILE_END * MD_TILE_BYTES
            || !page.decoded_bytes.is_multiple_of(MD_TILE_BYTES)
        {
            return Err(format!(
                "credit-heading page {index} identity or geometry drifted"
            ));
        }
        for hash in [
            &page.header_sha256,
            &page.decoded_sha256,
            &page.protected_prefix_sha256,
            &page.mutable_heading_sha256,
            &page.protected_suffix_sha256,
            &page.loader_sha256,
        ] {
            if hash.len() != 64 {
                return Err(format!(
                    "credit-heading page {index} has an invalid SHA-256 declaration"
                ));
            }
        }
    }
    Ok(())
}

fn checked_source_page(source: &[u8], declaration: &PageDeclaration) -> Result<Vec<u8>, String> {
    let header_offset = parse_hex(&declaration.header_offset)?;
    let header = source_range(
        source,
        header_offset,
        HEADER_BYTES,
        "credit-heading JP header",
    )?;
    if sha256_hex(header) != declaration.header_sha256
        || header[0] != 0x80
        || header[1] != 0
        || u16::from_be_bytes([header[4], header[5]]) != 0
    {
        return Err(format!(
            "{} JP first-transfer header drifted",
            declaration.id
        ));
    }
    let decoded = decode_mode1_pack_entry(source, header_offset)?;
    if decoded.vram_destination != 0
        || decoded.data.len() != declaration.decoded_bytes
        || sha256_hex(&decoded.data) != declaration.decoded_sha256
    {
        return Err(format!(
            "{} JP first transfer drifted: destination 0x{:04X}, {} bytes, SHA-256 {}",
            declaration.id,
            decoded.vram_destination,
            decoded.data.len(),
            sha256_hex(&decoded.data)
        ));
    }
    validate_source_ownership(&decoded.data, declaration)?;
    Ok(decoded.data)
}

fn validate_source_ownership(payload: &[u8], declaration: &PageDeclaration) -> Result<(), String> {
    let mutable_start = MUTABLE_TILE_START * MD_TILE_BYTES;
    let mutable_end = MUTABLE_TILE_END * MD_TILE_BYTES;
    checked_slice_hash(
        payload,
        0,
        mutable_start,
        &declaration.protected_prefix_sha256,
        &format!("{} protected tile-0 prefix", declaration.id),
    )?;
    checked_slice_hash(
        payload,
        mutable_start,
        mutable_end,
        &declaration.mutable_heading_sha256,
        &format!("{} JP heading surface", declaration.id),
    )?;
    checked_slice_hash(
        payload,
        mutable_end,
        payload.len(),
        &declaration.protected_suffix_sha256,
        &format!("{} protected suffix", declaration.id),
    )?;
    let roles = payload[mutable_start..mutable_end]
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 15]) {
        return Err(format!(
            "{} JP heading uses unexpected palette roles {roles:?}",
            declaration.id
        ));
    }
    Ok(())
}

fn compile_heading(
    font: &Font,
    source: &[u8],
    declaration: &PageDeclaration,
    manifest: &CreditsTopManifest,
) -> Result<Vec<u8>, String> {
    let surface = render_native_text_line(
        font,
        &declaration.ko,
        SURFACE_WIDTH_PIXELS,
        manifest.space_width_px,
        0,
        15,
        &declaration.id,
    )?;
    let encoded = encode_md_tiles_column_major(
        &surface,
        SURFACE_WIDTH_PIXELS,
        SURFACE_HEIGHT_PIXELS,
        &format!("{} Korean heading", declaration.id),
    )?;
    if encoded.len() != MUTABLE_BYTES {
        return Err(format!(
            "{} Korean heading encoded to {} bytes instead of {MUTABLE_BYTES}",
            declaration.id,
            encoded.len()
        ));
    }
    let mut target = source.to_vec();
    let mutable_start = MUTABLE_TILE_START * MD_TILE_BYTES;
    let mutable_end = MUTABLE_TILE_END * MD_TILE_BYTES;
    target[mutable_start..mutable_end].copy_from_slice(&encoded);
    Ok(target)
}

fn validate_target_ownership(
    source: &[u8],
    target: &[u8],
    declaration: &PageDeclaration,
) -> Result<(), String> {
    let mutable_start = MUTABLE_TILE_START * MD_TILE_BYTES;
    let mutable_end = MUTABLE_TILE_END * MD_TILE_BYTES;
    if source.len() != target.len()
        || source[..mutable_start] != target[..mutable_start]
        || source[mutable_end..] != target[mutable_end..]
    {
        return Err(format!(
            "{} compiler changed protected JP bytes",
            declaration.id
        ));
    }
    let roles = target[mutable_start..mutable_end]
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 15]) {
        return Err(format!(
            "{} Korean heading uses unexpected palette roles {roles:?}",
            declaration.id
        ));
    }
    Ok(())
}

fn validate_loader(rom: &[u8], declaration: &PageDeclaration, label: &str) -> Result<(), String> {
    let loader_offset = parse_hex(&declaration.loader_offset)?;
    let header_offset = parse_u32_hex(&declaration.header_offset)?;
    let semantic = credit_loader(header_offset)?;
    let actual = source_range(rom, loader_offset, LOADER_BYTES, "credit-heading loader")?;
    if semantic.len() != LOADER_BYTES
        || sha256_hex(&semantic) != declaration.loader_sha256
        || actual != semantic
    {
        return Err(format!(
            "{label} {} typed first-transfer loader drifted",
            declaration.id
        ));
    }
    Ok(())
}

fn credit_loader(header_offset: u32) -> Result<Vec<u8>, String> {
    m68k::assemble(&[
        Inst::LeaAbsoluteLong {
            address: header_offset,
            destination: AddressReg::A2,
        },
        Inst::JsrAbsoluteLong(LOADER_TARGET),
    ])
}

fn checked_slice_hash(
    data: &[u8],
    start: usize,
    end: usize,
    expected: &str,
    label: &str,
) -> Result<(), String> {
    if end < start {
        return Err(format!("{label} has an inverted range"));
    }
    let bytes = source_range(data, start, end - start, label)?;
    let actual = sha256_hex(bytes);
    if actual != expected {
        return Err(format!(
            "{label} SHA-256 drifted: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn align_up(value: usize, alignment: usize) -> Result<usize, String> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err("credit-heading pack alignment must be a power of two".to_string());
    }
    value
        .checked_add(alignment - 1)
        .map(|candidate| candidate & !(alignment - 1))
        .ok_or_else(|| "credit-heading pack alignment overflowed".to_string())
}

/// Encode several mode-1 streams behind distinct pointer slots in one source
/// bank. The format exposes only seven pointer-address bits per header, so
/// placing ten independent aligned packs in the same bank would alias them.
fn encode_shared_mode1_bank(
    payloads: &[&[u8]],
) -> Result<(Vec<[u8; HEADER_BYTES]>, Vec<u8>), String> {
    if payloads.is_empty() {
        return Err("credit-heading shared bank has no payloads".to_string());
    }
    let pointer_table_bytes = payloads
        .len()
        .checked_mul(POINTER_ENTRY_BYTES)
        .ok_or_else(|| "credit-heading pointer table length overflowed".to_string())?;
    if pointer_table_bytes > PAYLOAD_START_ALIGNMENT || pointer_table_bytes > 0x80 {
        return Err("credit-heading pointer table exceeds the packed-pointer window".to_string());
    }

    let payload_start = align_up(pointer_table_bytes, PAYLOAD_START_ALIGNMENT)?;
    let mut bank = vec![BANK_FILL; payload_start];
    let mut headers = Vec::with_capacity(payloads.len());
    for (index, payload) in payloads.iter().enumerate() {
        if payload.is_empty() {
            return Err(format!(
                "credit-heading shared payload {index} must not be empty"
            ));
        }
        // The 68000 consumer reads each mode-1 subheader as words. A previous
        // implementation packed the next subheader immediately after the
        // preceding byte stream, so an odd-sized stream made page 1 onward
        // point at an odd address. Page 0 decoded correctly, then the staff
        // roll consumed corrupt transfer metadata on the next page.
        if !bank.len().is_multiple_of(2) {
            bank.push(BANK_FILL);
        }
        let pointer_slot = index * POINTER_ENTRY_BYTES;
        let subheader_offset = bank.len();
        let compressed_offset =
            subheader_offset + MODE1_SUBHEADER_BYTES + MODE1_CHAIN_TERMINATOR.len();
        let subheader_pointer = u16::try_from(subheader_offset)
            .map_err(|_| "credit-heading subheader escaped its source bank".to_string())?;
        let compressed_pointer = u16::try_from(compressed_offset)
            .map_err(|_| "credit-heading stream escaped its source bank".to_string())?;
        bank[pointer_slot..pointer_slot + 2].copy_from_slice(&subheader_pointer.to_be_bytes());
        bank.extend_from_slice(&[0xC0, 0x00, 0x00, 0x00]);
        bank.extend_from_slice(&compressed_pointer.to_be_bytes());
        bank.extend_from_slice(&MODE1_CHAIN_TERMINATOR);
        for chunk in payload.chunks(0x7F) {
            bank.push(chunk.len() as u8);
            bank.extend_from_slice(chunk);
        }
        bank.push(0);
        if BANK_OFFSET + bank.len() > BANK_LIMIT {
            return Err(format!(
                "credit-heading shared bank exceeds 64 KiB after payload {index}"
            ));
        }

        let packed_source = (((BANK_OFFSET & 0xFF8000) >> 8) | pointer_slot) as u16;
        let [source_high, source_low] = packed_source.to_be_bytes();
        headers.push([0x80, 0x00, source_high, source_low, 0x00, 0x00]);
    }
    Ok((headers, bank))
}

fn draw_surface(
    rgba: &mut [u8],
    output_width: usize,
    logical_x: usize,
    logical_y: usize,
    surface: &[u8],
    ink: [u8; 4],
) -> Result<(), String> {
    let output_height = rgba.len() / 4 / output_width;
    if surface.len() != SURFACE_WIDTH_PIXELS * SURFACE_HEIGHT_PIXELS {
        return Err("credit-heading preview surface length is invalid".to_string());
    }
    for y in 0..SURFACE_HEIGHT_PIXELS {
        for x in 0..SURFACE_WIDTH_PIXELS {
            let role = surface[y * SURFACE_WIDTH_PIXELS + x];
            if role != 0 && role != 15 {
                return Err(format!(
                    "credit-heading preview uses unexpected palette role {role}"
                ));
            }
            if role == 0 {
                continue;
            }
            for scale_y in 0..PREVIEW_SCALE {
                for scale_x in 0..PREVIEW_SCALE {
                    let output_x = (logical_x + x) * PREVIEW_SCALE + scale_x;
                    let output_y = (logical_y + y) * PREVIEW_SCALE + scale_y;
                    if output_x >= output_width || output_y >= output_height {
                        return Err("credit-heading preview draw escaped its canvas".to_string());
                    }
                    let offset = (output_y * output_width + output_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&ink);
                }
            }
        }
    }
    Ok(())
}

fn parse_u8_hex(value: &str) -> Result<u8, String> {
    u8::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 8 bits"))
}

fn parse_u32_hex(value: &str) -> Result<u32, String> {
    u32::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 32 bits"))
}

fn summary(build: &CreditsTopBuild, checksum: u16) -> CreditsTopSummary {
    let decoded_bytes = build
        .pages
        .iter()
        .map(|page| page.source.len())
        .sum::<usize>();
    let heading_tiles = build.pages.len() * MUTABLE_TILES;
    let rewritten_tiles = heading_tiles
        + build.timed.rewritten_tiles()
        + build.generic.rewritten_tiles()
        + build.native_frames.rewritten_tiles()
        + build.remaining.rewritten_tiles();
    CreditsTopSummary {
        headings: build.pages.len(),
        timed_lines: build.timed.line_count()
            + build.generic.line_count()
            + build.native_frames.line_count()
            + build.remaining.line_count(),
        relocated_timed_lines: build.timed.relocated_lines()
            + build.generic.relocated_lines()
            + build.native_frames.relocated_lines()
            + build.remaining.relocated_lines(),
        unique_glyphs: build.unique_glyphs,
        decoded_bytes,
        rewritten_tiles,
        overwritten_timed_tiles: build.timed.overwritten_source_tiles()
            + build.generic.overwritten_source_tiles()
            + build.native_frames.overwritten_source_tiles(),
        appended_timed_tiles: build.timed.appended_tiles()
            + build.generic.appended_tiles()
            + build.native_frames.appended_tiles()
            + build.remaining.appended_tiles(),
        protected_bytes: decoded_bytes
            - (heading_tiles
                + build.timed.overwritten_source_tiles()
                + build.generic.overwritten_source_tiles()
                + build.native_frames.overwritten_source_tiles())
                * MD_TILE_BYTES,
        verified_loader_bytes: build.pages.len() * LOADER_BYTES,
        verified_timed_consumer_bytes: build.timed.verified_consumer_bytes(),
        verified_generic_consumer_bytes: build.generic.verified_consumer_bytes(),
        verified_native_frame_consumer_bytes: build.native_frames.verified_consumer_bytes(),
        verified_remaining_consumer_bytes: build.remaining.verified_consumer_bytes(),
        written_executable_bytes: build.timed.written_executable_bytes(),
        map_bank_bytes: build.timed.map_bank().len(),
        generic_table_bank_bytes: build.generic.table_bank().len(),
        native_frame_table_bank_bytes: build.native_frames.table_bank().len(),
        remaining_table_bank_bytes: build.remaining.table_bank().len(),
        remaining_pack_bank_bytes: build.remaining.pack_bank_bytes(),
        pack_bank_bytes: build.bank.len(),
        checksum,
    }
}
