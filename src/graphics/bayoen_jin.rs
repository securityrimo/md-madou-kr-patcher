//! JP-source `じ〜ん` -> `찡…` battle-effect compiler.
//!
//! The effect occupies tiles 33..45 of one 66-tile battle-main transfer.
//! Its exact conditioned draw path remains a runtime gate, but the source
//! pack, mutable range, palette roles, and tile layout are all closed here.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use super::font_effect::{blit_indexed_glyph, read_verified_font, render_indexed_glyph};
use super::pixel::{
    decode_md_tiles_column_major, encode_md_tiles_column_major, md_color, nearest_core_distance,
    parse_md_palette, write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const JIN_HEADER_OFFSETS: [usize; 5] = [0x07_80AC, 0x07_80C0, 0x07_80D4, 0x07_80E8, 0x07_80FC];
const JIN_VRAM_DESTINATION: u16 = 0x0680;
const JIN_BANK_OFFSET: usize = 0x2B_8000;
const JIN_BANK_LIMIT: usize = 0x2C_0000;
const JIN_SOURCE_BYTES: usize = 2_112;
const JIN_SOURCE_TILES: usize = JIN_SOURCE_BYTES / MD_TILE_BYTES;
const JIN_TILE_START: usize = 33;
const JIN_TILE_END: usize = 45;
const JIN_TILES: usize = JIN_TILE_END - JIN_TILE_START;
const JIN_BYTES: usize = JIN_TILES * MD_TILE_BYTES;
const JIN_WIDTH: usize = 48;
const JIN_HEIGHT: usize = 16;
const JIN_LAYOUT: [[usize; 6]; 2] = [[0, 2, 4, 6, 8, 10], [1, 3, 5, 7, 9, 11]];
const PREVIEW_SCALE: usize = 12;
const PREVIEW_GAP: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BayoenJinSummary {
    pub source_tiles: usize,
    pub rewritten_tiles: usize,
    pub protected_bytes: usize,
    pub visible_pixels: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct BayoenJinManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    font_asset: String,
    font_sha256: String,
    jp_text: String,
    ko: String,
    output_surface: OutputSurface,
    palette_line_words: Vec<String>,
    transparent_palette_index: usize,
    source_palette_indices: Vec<usize>,
    target_palette_indices: Vec<usize>,
    mutable_tile_range: MutableTileRange,
    tile_layout: Vec<Vec<usize>>,
    source_pack: SourcePack,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
}

#[derive(Debug, Deserialize)]
struct MutableTileRange {
    start: usize,
    end_exclusive: usize,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
struct SourcePack {
    id: String,
    header_offsets: Vec<String>,
    vram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
}

#[derive(Debug)]
struct BayoenJinBuild {
    manifest: BayoenJinManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    source_surface: Vec<u8>,
    target_surface: Vec<u8>,
    header: [u8; 6],
    bank: Vec<u8>,
}

pub fn apply_bayoen_jin(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<BayoenJinSummary, String> {
    let build = build_bayoen_jin(source, assets_dir)?;
    let bank_end = JIN_BANK_OFFSET + build.bank.len();
    if bank_end > JIN_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "Bayoen jin pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(JIN_HEADER_OFFSETS.len() + 2);
    for &header_offset in &JIN_HEADER_OFFSETS {
        apply_expected_write(
            output,
            header_offset,
            source_range(
                source,
                header_offset,
                build.header.len(),
                "Bayoen jin source alias header",
            )?,
            &build.header,
            "Bayoen jin alias pack header",
        )?;
        changed_ranges.push((header_offset, header_offset + build.header.len()));
    }
    apply_expected_write(
        output,
        JIN_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "Bayoen jin expanded pack",
    )?;
    changed_ranges.push((JIN_BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after Bayoen jin graphics",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    for &header_offset in &JIN_HEADER_OFFSETS {
        let inserted = decode_mode1_pack_entry(output, header_offset)?;
        if inserted.vram_destination != JIN_VRAM_DESTINATION || inserted.data != build.payload {
            return Err(format!(
                "inserted Bayoen jin alias at 0x{header_offset:06X} does not match the planned payload"
            ));
        }
    }

    eprintln!("JP graphics GFX-BATTLE-BAYOEN-JIN Expected Writes:");
    for &header_offset in &JIN_HEADER_OFFSETS {
        eprintln!(
            "  0x{header_offset:06X}..0x{:06X}  battle-main alias header ({} bytes)",
            header_offset + build.header.len(),
            build.header.len()
        );
    }
    eprintln!(
        "  0x{JIN_BANK_OFFSET:06X}..0x{bank_end:06X}  Bayoen jin pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render JP and Korean effect pixels side by side as deterministic static QA.
pub fn write_bayoen_jin_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<BayoenJinSummary, String> {
    let build = build_bayoen_jin(source, assets_dir)?;
    let contact_width = JIN_WIDTH * 2 + PREVIEW_GAP;
    let preview_width = contact_width * PREVIEW_SCALE;
    let preview_height = JIN_HEIGHT * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[28, 28, 28, 255]);
    }
    for (panel, pixels) in [
        build.source_surface.as_slice(),
        build.target_surface.as_slice(),
    ]
    .into_iter()
    .enumerate()
    {
        let panel_x = panel * (JIN_WIDTH + PREVIEW_GAP);
        draw_scaled_surface(
            &mut rgba,
            preview_width,
            panel_x,
            pixels,
            &build.palette,
            build.manifest.transparent_palette_index,
        );
    }
    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "Bayoen jin",
    )?;
    Ok(summary(&build, 0))
}

fn build_bayoen_jin(source: &[u8], assets_dir: &Path) -> Result<BayoenJinBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "Bayoen jin")?;
    let font = read_font(assets_dir, &manifest)?;
    let source_payload = checked_source_pack(source, &manifest.source_pack)?;

    let mutable_start = JIN_TILE_START * MD_TILE_BYTES;
    let mutable_end = JIN_TILE_END * MD_TILE_BYTES;
    let source_tiles = &source_payload[mutable_start..mutable_end];
    validate_source_tiles(source_tiles, &manifest)?;
    let source_surface =
        decode_md_tiles_column_major(source_tiles, JIN_WIDTH, JIN_HEIGHT, "Bayoen jin JP")?;
    let target_surface = render_target_surface(&font, &manifest)?;
    let target_tiles =
        encode_md_tiles_column_major(&target_surface, JIN_WIDTH, JIN_HEIGHT, "Bayoen jin Korean")?;
    if target_tiles.len() != JIN_BYTES {
        return Err("Bayoen jin encoded tile length drifted".to_string());
    }

    let mut payload = source_payload.clone();
    payload[mutable_start..mutable_end].copy_from_slice(&target_tiles);
    if payload[..mutable_start] != source_payload[..mutable_start]
        || payload[mutable_end..] != source_payload[mutable_end..]
    {
        return Err("Bayoen jin compiler changed protected JP bytes".to_string());
    }

    let encoded = encode_locked_mode1_pack(JIN_BANK_OFFSET, JIN_VRAM_DESTINATION, &payload)?;
    validate_pack_roundtrip(&encoded.header, &encoded.bank, &payload)?;
    Ok(BayoenJinBuild {
        manifest,
        palette,
        source_payload,
        payload,
        source_surface,
        target_surface,
        header: encoded.header,
        bank: encoded.bank,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<BayoenJinManifest, String> {
    let path = assets_dir.join("graphics_text/bayoen_jin.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read Bayoen jin source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid Bayoen jin source {}: {error}", path.display()))
}

fn read_font(assets_dir: &Path, manifest: &BayoenJinManifest) -> Result<Font, String> {
    read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "Bayoen jin",
    )
}

fn validate_manifest_shape(manifest: &BayoenJinManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-BATTLE-BAYOEN-JIN"
        || !manifest.source_policy.contains("JP")
        || manifest.font_asset != "neodgm.ttf"
        || manifest.jp_text != "じ〜ん"
        || manifest.ko != "찡…"
        || manifest.output_surface.width != JIN_WIDTH
        || manifest.output_surface.height != JIN_HEIGHT
    {
        return Err("unsupported Bayoen jin manifest identity".to_string());
    }
    if manifest.transparent_palette_index != 0
        || manifest
            .source_palette_indices
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != BTreeSet::from([0, 13, 14, 15])
        || manifest.target_palette_indices != [13, 15, 14]
    {
        return Err("Bayoen jin palette roles drifted".to_string());
    }
    if manifest.mutable_tile_range.start != JIN_TILE_START
        || manifest.mutable_tile_range.end_exclusive != JIN_TILE_END
        || manifest.mutable_tile_range.source_sha256.len() != 64
    {
        return Err("Bayoen jin mutable tile range drifted".to_string());
    }
    if manifest.tile_layout.len() != JIN_LAYOUT.len()
        || manifest
            .tile_layout
            .iter()
            .zip(JIN_LAYOUT)
            .any(|(actual, expected)| actual.as_slice() != expected)
    {
        return Err("Bayoen jin tile layout drifted".to_string());
    }
    let pack = &manifest.source_pack;
    let header_offsets = pack
        .header_offsets
        .iter()
        .map(|offset| parse_hex(offset))
        .collect::<Result<Vec<_>, _>>()?;
    if pack.id != "battle-main-bayoen-jin"
        || header_offsets != JIN_HEADER_OFFSETS
        || parse_u16_hex(&pack.vram_destination)? != JIN_VRAM_DESTINATION
        || pack.decoded_bytes != JIN_SOURCE_BYTES
        || pack.decoded_sha256.len() != 64
    {
        return Err("Bayoen jin source pack declaration drifted".to_string());
    }
    Ok(())
}

fn checked_source_pack(source: &[u8], declaration: &SourcePack) -> Result<Vec<u8>, String> {
    let mut payload = None;
    for header_offset in &declaration.header_offsets {
        let header_offset = parse_hex(header_offset)?;
        let decoded = decode_mode1_pack_entry(source, header_offset)?;
        let actual_hash = sha256_hex(&decoded.data);
        if decoded.vram_destination != parse_u16_hex(&declaration.vram_destination)?
            || decoded.data.len() != declaration.decoded_bytes
            || actual_hash != declaration.decoded_sha256
        {
            return Err(format!(
                "{} source alias at 0x{header_offset:06X} drifted: VRAM 0x{:04X}, {} bytes, SHA-256 {actual_hash}",
                declaration.id,
                decoded.vram_destination,
                decoded.data.len()
            ));
        }
        if let Some(expected) = &payload
            && expected != &decoded.data
        {
            return Err(format!(
                "{} source alias at 0x{header_offset:06X} decodes a different payload",
                declaration.id
            ));
        }
        payload = Some(decoded.data);
    }
    payload.ok_or_else(|| format!("{} declares no source aliases", declaration.id))
}

fn validate_source_tiles(tiles: &[u8], manifest: &BayoenJinManifest) -> Result<(), String> {
    if tiles.len() != JIN_BYTES {
        return Err("Bayoen jin JP tile payload length is invalid".to_string());
    }
    let actual_hash = sha256_hex(tiles);
    if actual_hash != manifest.mutable_tile_range.source_sha256 {
        return Err(format!(
            "Bayoen jin JP tile SHA-256 mismatch: expected {}, got {actual_hash}",
            manifest.mutable_tile_range.source_sha256
        ));
    }
    let pixels = decode_md_tiles_column_major(tiles, JIN_WIDTH, JIN_HEIGHT, "Bayoen jin JP roles")?;
    let actual_roles = pixels
        .iter()
        .map(|&pixel| pixel as usize)
        .collect::<BTreeSet<_>>();
    let expected_roles = manifest
        .source_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual_roles != expected_roles {
        return Err(format!(
            "Bayoen jin JP palette drifted: expected {expected_roles:?}, got {actual_roles:?}"
        ));
    }
    Ok(())
}

fn render_target_surface(font: &Font, manifest: &BayoenJinManifest) -> Result<Vec<u8>, String> {
    let transparent = manifest.transparent_palette_index as u8;
    let [core, inner, outer] = manifest
        .target_palette_indices
        .as_slice()
        .try_into()
        .map_err(|_| "Bayoen jin needs exactly three target palette roles".to_string())?;
    let mut pixels = vec![transparent; JIN_WIDTH * JIN_HEIGHT];
    let glyph = render_indexed_glyph(
        font,
        '찡',
        transparent,
        core as u8,
        inner as u8,
        outer as u8,
    );
    blit_indexed_glyph(&mut pixels, JIN_WIDTH, JIN_HEIGHT, 2, 0, &glyph)?;
    render_large_ellipsis(
        &mut pixels,
        22,
        transparent,
        core as u8,
        inner as u8,
        outer as u8,
    )?;

    let actual_roles = pixels
        .iter()
        .map(|&pixel| pixel as usize)
        .collect::<BTreeSet<_>>();
    let expected_roles = std::iter::once(manifest.transparent_palette_index)
        .chain(manifest.target_palette_indices.iter().copied())
        .collect::<BTreeSet<_>>();
    if actual_roles != expected_roles {
        return Err(format!(
            "Bayoen jin target palette roles are incomplete: expected {expected_roles:?}, got {actual_roles:?}"
        ));
    }
    Ok(pixels)
}

fn render_large_ellipsis(
    surface: &mut [u8],
    destination_x: usize,
    transparent: u8,
    core_index: u8,
    inner_index: u8,
    outer_index: u8,
) -> Result<(), String> {
    const WIDTH: usize = 24;
    const HEIGHT: usize = 16;
    if surface.len() != JIN_WIDTH * JIN_HEIGHT || destination_x + WIDTH > JIN_WIDTH {
        return Err("Bayoen jin ellipsis destination is invalid".to_string());
    }
    let mut core = vec![false; WIDTH * HEIGHT];
    for dot_x in [2usize, 10, 18] {
        for y in 10..13 {
            for x in dot_x..dot_x + 3 {
                core[y * WIDTH + x] = true;
            }
        }
    }
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pixel = if core[y * WIDTH + x] {
                core_index
            } else {
                match nearest_core_distance(&core, WIDTH, HEIGHT, x, y, 2) {
                    Some(1) => inner_index,
                    Some(2) => outer_index,
                    _ => transparent,
                }
            };
            surface[y * JIN_WIDTH + destination_x + x] = pixel;
        }
    }
    Ok(())
}

fn validate_pack_roundtrip(header: &[u8; 6], bank: &[u8], payload: &[u8]) -> Result<(), String> {
    let mut probe = vec![0u8; JIN_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[JIN_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != JIN_VRAM_DESTINATION || decoded.data != payload {
        return Err("Bayoen jin mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn draw_scaled_surface(
    rgba: &mut [u8],
    preview_width: usize,
    panel_x: usize,
    pixels: &[u8],
    palette: &[u16; 16],
    transparent: usize,
) {
    for y in 0..JIN_HEIGHT {
        for x in 0..JIN_WIDTH {
            let palette_index = pixels[y * JIN_WIDTH + x] as usize;
            let color = if palette_index == transparent {
                if (x / 2 + y / 2).is_multiple_of(2) {
                    [54, 54, 54]
                } else {
                    [76, 76, 76]
                }
            } else {
                md_color(palette[palette_index])
            };
            for scale_y in 0..PREVIEW_SCALE {
                for scale_x in 0..PREVIEW_SCALE {
                    let preview_x = (panel_x + x) * PREVIEW_SCALE + scale_x;
                    let preview_y = y * PREVIEW_SCALE + scale_y;
                    let offset = (preview_y * preview_width + preview_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                }
            }
        }
    }
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    u16::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn summary(build: &BayoenJinBuild, checksum: u16) -> BayoenJinSummary {
    debug_assert_eq!(build.source_payload.len(), build.payload.len());
    BayoenJinSummary {
        source_tiles: JIN_SOURCE_TILES,
        rewritten_tiles: JIN_TILES,
        protected_bytes: JIN_SOURCE_BYTES - JIN_BYTES,
        visible_pixels: build
            .target_surface
            .iter()
            .filter(|&&pixel| pixel as usize != build.manifest.transparent_palette_index)
            .count(),
        pack_bytes: build.bank.len(),
        checksum,
    }
}
