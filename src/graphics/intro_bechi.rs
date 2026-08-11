//! JP-source automatic-prologue `べちっ` -> `꽈당!` impact compiler.
//!
//! The final prologue scene loads three transfers. The last fifteen tiles of
//! the high-pattern transfer form a 24x40 sprite: JP lettering occupies the
//! upper 24x16 pixels and the impact burst occupies the lower 24x24 pixels.
//! Korean lettering is rendered deterministically while the burst and every
//! other decoded byte remain JP-owned.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::font_effect::{read_verified_font, render_compact_8x16_glyph};
use super::pixel::{md_color, parse_md_palette, write_rgba_png};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const MAP_HEADER_OFFSET: usize = 0x08_D432;
const LOW_PATTERN_HEADER_OFFSET: usize = 0x08_D438;
const HIGH_PATTERN_HEADER_OFFSET: usize = 0x08_D43E;
const HIGH_PATTERN_VRAM: u16 = 0x4000;
const MAP_DECODED_BYTES: usize = 2_176;
const LOW_PATTERN_DECODED_BYTES: usize = 640;
const HIGH_PATTERN_DECODED_BYTES: usize = 9_056;
const MUTABLE_TILE_START: usize = 0x10C;
const MUTABLE_TILE_END: usize = 0x11B;
const MUTABLE_BYTES: usize = (MUTABLE_TILE_END - MUTABLE_TILE_START) * MD_TILE_BYTES;
const SURFACE_WIDTH: usize = 24;
const SURFACE_HEIGHT: usize = 40;
const TEXT_HEIGHT: usize = 16;
const TARGET_BANK_OFFSET: usize = 0x33_8000;
const TARGET_BANK_LIMIT: usize = 0x33_E000;
const PREVIEW_SCALE: usize = 8;
const PREVIEW_GAP: usize = 8;
const TILE_LAYOUT: [[usize; 3]; 5] = [[0, 4, 8], [1, 5, 9], [2, 6, 10], [3, 7, 11], [12, 13, 14]];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntroBechiSummary {
    pub source_pattern_tiles: usize,
    pub rewritten_tiles: usize,
    pub protected_pattern_bytes: usize,
    pub protected_companion_bytes: usize,
    pub protected_decoration_pixels: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct IntroBechiManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    font_asset: String,
    font_sha256: String,
    font_size_px: f32,
    coverage_threshold: u8,
    jp: String,
    ko: String,
    output_surface: OutputSurface,
    palette_line_words: Vec<String>,
    transparent_palette_index: usize,
    source_text_palette_indices: Vec<usize>,
    source_decoration_palette_indices: Vec<usize>,
    target_ink_palette_index: usize,
    mutable_tile_range: MutableTileRange,
    tile_layout: Vec<Vec<usize>>,
    target_pack_bank: TargetPackBank,
    source_packs: Vec<SourcePack>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
    text_height: usize,
}

#[derive(Debug, Deserialize)]
struct MutableTileRange {
    start: usize,
    end_exclusive: usize,
}

#[derive(Debug, Deserialize)]
struct TargetPackBank {
    offset: String,
    limit: String,
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
struct IntroBechiBuild {
    manifest: IntroBechiManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    source_surface: Vec<u8>,
    target_surface: Vec<u8>,
    protected_companion_bytes: usize,
    header: [u8; 6],
    bank: Vec<u8>,
}

/// Insert the Korean final-prologue impact lettering into the cumulative ROM.
pub fn apply_intro_bechi(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<IntroBechiSummary, String> {
    let build = build_intro_bechi(source, assets_dir)?;
    let bank_end = TARGET_BANK_OFFSET + build.bank.len();
    if bank_end > TARGET_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "intro bechi pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    apply_expected_write(
        output,
        HIGH_PATTERN_HEADER_OFFSET,
        source_range(
            source,
            HIGH_PATTERN_HEADER_OFFSET,
            build.header.len(),
            "intro bechi source high-pattern header",
        )?,
        &build.header,
        "intro bechi high-pattern header",
    )?;
    apply_expected_write(
        output,
        TARGET_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "intro bechi expanded pattern pack",
    )?;

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after intro bechi graphics",
    )?;
    validate_only_ranges_changed(
        &baseline,
        output,
        &[
            (
                HIGH_PATTERN_HEADER_OFFSET,
                HIGH_PATTERN_HEADER_OFFSET + build.header.len(),
            ),
            (TARGET_BANK_OFFSET, bank_end),
            (CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2),
        ],
    )?;

    let inserted = decode_mode1_pack_entry(output, HIGH_PATTERN_HEADER_OFFSET)?;
    if inserted.vram_destination != HIGH_PATTERN_VRAM || inserted.data != build.payload {
        return Err(
            "inserted intro bechi pack does not decode to the planned JP-derived payload"
                .to_string(),
        );
    }
    for declaration in &build.manifest.source_packs {
        let header_offset = parse_hex(&declaration.header_offset)?;
        if header_offset == HIGH_PATTERN_HEADER_OFFSET {
            continue;
        }
        let source_pack = checked_source_pack(source, declaration)?;
        let output_pack = decode_mode1_pack_entry(output, header_offset)?;
        if output_pack.vram_destination != parse_u16_hex(&declaration.vram_destination)?
            || output_pack.data != source_pack
        {
            return Err(format!(
                "intro bechi insertion changed protected JP transfer {}",
                declaration.id
            ));
        }
    }

    eprintln!("JP graphics GFX-INTRO-BECHI Expected Writes:");
    eprintln!(
        "  0x{HIGH_PATTERN_HEADER_OFFSET:06X}..0x{:06X}  intro final high-pattern header ({} bytes)",
        HIGH_PATTERN_HEADER_OFFSET + build.header.len(),
        build.header.len()
    );
    eprintln!(
        "  0x{TARGET_BANK_OFFSET:06X}..0x{bank_end:06X}  intro final pattern pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render JP and Korean 24x40 sprites side by side as deterministic static QA.
pub fn write_intro_bechi_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<IntroBechiSummary, String> {
    let build = build_intro_bechi(source, assets_dir)?;
    let panel_width = SURFACE_WIDTH * PREVIEW_SCALE;
    let preview_width = panel_width * 2 + PREVIEW_GAP;
    let preview_height = SURFACE_HEIGHT * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[32, 32, 32, 255]);
    }
    draw_preview_surface(
        &mut rgba,
        preview_width,
        0,
        &build.source_surface,
        &build.palette,
        build.manifest.transparent_palette_index,
    )?;
    draw_preview_surface(
        &mut rgba,
        preview_width,
        panel_width + PREVIEW_GAP,
        &build.target_surface,
        &build.palette,
        build.manifest.transparent_palette_index,
    )?;
    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "intro bechi",
    )?;
    Ok(summary(&build, 0))
}

fn build_intro_bechi(source: &[u8], assets_dir: &Path) -> Result<IntroBechiBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "intro bechi")?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "intro bechi",
    )?;

    let mut protected_companion_bytes = 0usize;
    for declaration in &manifest.source_packs {
        let decoded = checked_source_pack(source, declaration)?;
        if parse_hex(&declaration.header_offset)? != HIGH_PATTERN_HEADER_OFFSET {
            protected_companion_bytes += decoded.len();
        }
    }
    let high_declaration = source_pack(&manifest, HIGH_PATTERN_HEADER_OFFSET)?;
    let source_payload = checked_source_pack(source, high_declaration)?;
    let mutable_start = MUTABLE_TILE_START * MD_TILE_BYTES;
    let mutable_end = MUTABLE_TILE_END * MD_TILE_BYTES;
    let source_surface = decode_effect_surface(
        &source_payload[mutable_start..mutable_end],
        "intro bechi JP",
    )?;
    validate_source_surface(&source_surface, &manifest)?;
    let target_surface = render_target_surface(&source_surface, &manifest, &font)?;
    let encoded_tiles = encode_effect_surface(&target_surface, "intro bechi KR")?;
    if encoded_tiles.len() != MUTABLE_BYTES {
        return Err(format!(
            "intro bechi encoded as {} bytes, expected {MUTABLE_BYTES}",
            encoded_tiles.len()
        ));
    }

    let mut payload = source_payload.clone();
    payload[mutable_start..mutable_end].copy_from_slice(&encoded_tiles);
    if payload[..mutable_start] != source_payload[..mutable_start]
        || payload[mutable_end..] != source_payload[mutable_end..]
    {
        return Err("intro bechi compiler changed protected JP pattern bytes".to_string());
    }

    let encoded = encode_locked_mode1_pack(TARGET_BANK_OFFSET, HIGH_PATTERN_VRAM, &payload)?;
    validate_pack_roundtrip(&encoded.header, &encoded.bank, &payload)?;
    Ok(IntroBechiBuild {
        manifest,
        palette,
        source_payload,
        payload,
        source_surface,
        target_surface,
        protected_companion_bytes,
        header: encoded.header,
        bank: encoded.bank,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<IntroBechiManifest, String> {
    let path = assets_dir.join("graphics_text/intro_bechi.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read intro bechi source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid intro bechi source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &IntroBechiManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-INTRO-BECHI"
        || !manifest.source_policy.contains("English graphics")
        || manifest.jp != "べちっ"
        || manifest.ko != "꽈당!"
        || manifest.ko.chars().count() != 3
        || manifest.font_size_px != 16.0
        || manifest.coverage_threshold != 127
    {
        return Err("unsupported intro bechi manifest identity or text contract".to_string());
    }
    let surface = manifest.output_surface;
    if surface.width != SURFACE_WIDTH
        || surface.height != SURFACE_HEIGHT
        || surface.text_height != TEXT_HEIGHT
        || manifest.transparent_palette_index != 0
        || manifest.target_ink_palette_index != 15
    {
        return Err("intro bechi output surface or palette roles drifted".to_string());
    }
    if manifest.mutable_tile_range.start != MUTABLE_TILE_START
        || manifest.mutable_tile_range.end_exclusive != MUTABLE_TILE_END
        || MUTABLE_TILE_END * MD_TILE_BYTES != HIGH_PATTERN_DECODED_BYTES
    {
        return Err("intro bechi mutable range drifted from the final fifteen tiles".to_string());
    }
    let layout = manifest
        .tile_layout
        .iter()
        .map(|row| row.as_slice())
        .collect::<Vec<_>>();
    if layout
        != TILE_LAYOUT
            .iter()
            .map(|row| row.as_slice())
            .collect::<Vec<_>>()
    {
        return Err("intro bechi tile layout drifted from the JP sprite map".to_string());
    }
    if parse_hex(&manifest.target_pack_bank.offset)? != TARGET_BANK_OFFSET
        || parse_hex(&manifest.target_pack_bank.limit)? != TARGET_BANK_LIMIT
    {
        return Err("intro bechi target bank drifted".to_string());
    }
    let expected_packs = [
        (
            "intro-final-map",
            MAP_HEADER_OFFSET,
            0xE000,
            MAP_DECODED_BYTES,
        ),
        (
            "intro-final-low-patterns",
            LOW_PATTERN_HEADER_OFFSET,
            0x2000,
            LOW_PATTERN_DECODED_BYTES,
        ),
        (
            "intro-final-high-patterns",
            HIGH_PATTERN_HEADER_OFFSET,
            HIGH_PATTERN_VRAM,
            HIGH_PATTERN_DECODED_BYTES,
        ),
    ];
    if manifest.source_packs.len() != expected_packs.len() {
        return Err("intro bechi manifest must declare exactly three JP transfers".to_string());
    }
    for (pack, (id, header, destination, decoded_bytes)) in
        manifest.source_packs.iter().zip(expected_packs)
    {
        if pack.id != id
            || parse_hex(&pack.header_offset)? != header
            || parse_u16_hex(&pack.vram_destination)? != destination
            || pack.decoded_bytes != decoded_bytes
        {
            return Err(format!(
                "intro bechi source declaration {} drifted from the JP consumer",
                pack.id
            ));
        }
    }
    let text_roles = manifest
        .source_text_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let decoration_roles = manifest
        .source_decoration_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if text_roles != BTreeSet::from([5, 12, 15])
        || decoration_roles != BTreeSet::from([8, 11])
        || !text_roles.is_disjoint(&decoration_roles)
    {
        return Err("intro bechi JP palette-role declaration drifted".to_string());
    }
    Ok(())
}

fn validate_source_surface(surface: &[u8], manifest: &IntroBechiManifest) -> Result<(), String> {
    if surface.len() != SURFACE_WIDTH * SURFACE_HEIGHT {
        return Err("intro bechi source surface has the wrong dimensions".to_string());
    }
    let text_roles = manifest
        .source_text_palette_indices
        .iter()
        .map(|&value| value as u8)
        .collect::<BTreeSet<_>>();
    let decoration_roles = manifest
        .source_decoration_palette_indices
        .iter()
        .map(|&value| value as u8)
        .collect::<BTreeSet<_>>();
    let top = surface[..SURFACE_WIDTH * TEXT_HEIGHT]
        .iter()
        .copied()
        .filter(|&pixel| pixel != manifest.transparent_palette_index as u8)
        .collect::<BTreeSet<_>>();
    let bottom = surface[SURFACE_WIDTH * TEXT_HEIGHT..]
        .iter()
        .copied()
        .filter(|&pixel| pixel != manifest.transparent_palette_index as u8)
        .collect::<BTreeSet<_>>();
    if top != text_roles || bottom != decoration_roles {
        return Err(format!(
            "intro bechi JP text/decoration separation drifted: top={top:?}, bottom={bottom:?}"
        ));
    }
    Ok(())
}

fn render_target_surface(
    source: &[u8],
    manifest: &IntroBechiManifest,
    font: &fontdue::Font,
) -> Result<Vec<u8>, String> {
    let transparent = manifest.transparent_palette_index as u8;
    let ink = manifest.target_ink_palette_index as u8;
    let mut target = source.to_vec();
    target[..SURFACE_WIDTH * TEXT_HEIGHT].fill(transparent);
    for (index, character) in manifest.ko.chars().enumerate() {
        let glyph = render_compact_8x16_glyph(
            font,
            character,
            manifest.font_size_px,
            manifest.coverage_threshold,
            transparent,
            ink,
            "intro bechi",
        )?;
        for y in 0..TEXT_HEIGHT {
            let destination = y * SURFACE_WIDTH + index * 8;
            target[destination..destination + 8].copy_from_slice(&glyph[y * 8..(y + 1) * 8]);
        }
    }
    if target[SURFACE_WIDTH * TEXT_HEIGHT..] != source[SURFACE_WIDTH * TEXT_HEIGHT..] {
        return Err("intro bechi renderer changed the JP impact burst".to_string());
    }
    if !target[..SURFACE_WIDTH * TEXT_HEIGHT].contains(&ink) {
        return Err("intro bechi Korean lettering rendered blank".to_string());
    }
    Ok(target)
}

fn decode_effect_surface(tiles: &[u8], label: &str) -> Result<Vec<u8>, String> {
    if tiles.len() != MUTABLE_BYTES {
        return Err(format!("{label} tile payload length is invalid"));
    }
    let mut pixels = vec![0u8; SURFACE_WIDTH * SURFACE_HEIGHT];
    for (tile_y, row) in TILE_LAYOUT.iter().enumerate() {
        for (tile_x, &tile) in row.iter().enumerate() {
            let tile_offset = tile * MD_TILE_BYTES;
            for local_y in 0..8 {
                for local_x in 0..8 {
                    let byte = tiles[tile_offset + local_y * 4 + local_x / 2];
                    let pixel = if local_x.is_multiple_of(2) {
                        byte >> 4
                    } else {
                        byte & 0x0F
                    };
                    let x = tile_x * 8 + local_x;
                    let y = tile_y * 8 + local_y;
                    pixels[y * SURFACE_WIDTH + x] = pixel;
                }
            }
        }
    }
    Ok(pixels)
}

fn encode_effect_surface(pixels: &[u8], label: &str) -> Result<Vec<u8>, String> {
    if pixels.len() != SURFACE_WIDTH * SURFACE_HEIGHT {
        return Err(format!("{label} pixel surface has the wrong dimensions"));
    }
    let mut tiles = vec![0u8; MUTABLE_BYTES];
    for (tile_y, row) in TILE_LAYOUT.iter().enumerate() {
        for (tile_x, &tile) in row.iter().enumerate() {
            let tile_offset = tile * MD_TILE_BYTES;
            for local_y in 0..8usize {
                for pair_x in 0..4usize {
                    let x = tile_x * 8 + pair_x * 2;
                    let y = tile_y * 8 + local_y;
                    let left = pixels[y * SURFACE_WIDTH + x];
                    let right = pixels[y * SURFACE_WIDTH + x + 1];
                    if left >= 16 || right >= 16 {
                        return Err(format!("{label} contains a palette index above 15"));
                    }
                    tiles[tile_offset + local_y * 4 + pair_x] = (left << 4) | right;
                }
            }
        }
    }
    Ok(tiles)
}

fn draw_preview_surface(
    rgba: &mut [u8],
    preview_width: usize,
    origin_x: usize,
    surface: &[u8],
    palette: &[u16; 16],
    transparent_index: usize,
) -> Result<(), String> {
    if !rgba.len().is_multiple_of(4) || surface.len() != SURFACE_WIDTH * SURFACE_HEIGHT {
        return Err("intro bechi preview dimensions are invalid".to_string());
    }
    for y in 0..SURFACE_HEIGHT {
        for x in 0..SURFACE_WIDTH {
            let index = surface[y * SURFACE_WIDTH + x] as usize;
            let color = if index == transparent_index {
                if (x / 2 + y / 2).is_multiple_of(2) {
                    [28, 28, 28]
                } else {
                    [48, 48, 48]
                }
            } else {
                md_color(palette[index])
            };
            for scale_y in 0..PREVIEW_SCALE {
                for scale_x in 0..PREVIEW_SCALE {
                    let preview_x = origin_x + x * PREVIEW_SCALE + scale_x;
                    let preview_y = y * PREVIEW_SCALE + scale_y;
                    let offset = (preview_y * preview_width + preview_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                }
            }
        }
    }
    Ok(())
}

fn source_pack(manifest: &IntroBechiManifest, header_offset: usize) -> Result<&SourcePack, String> {
    manifest
        .source_packs
        .iter()
        .find(|pack| {
            parse_hex(&pack.header_offset)
                .map(|offset| offset == header_offset)
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("intro bechi manifest has no source pack at 0x{header_offset:06X}"))
}

fn checked_source_pack(source: &[u8], declaration: &SourcePack) -> Result<Vec<u8>, String> {
    let header = parse_hex(&declaration.header_offset)?;
    let expected_vram = parse_u16_hex(&declaration.vram_destination)?;
    let decoded = decode_mode1_pack_entry(source, header)?;
    let actual_hash = sha256_hex(&decoded.data);
    if decoded.vram_destination != expected_vram || decoded.data.len() != declaration.decoded_bytes
    {
        return Err(format!(
            "{} decoded as {} bytes at VRAM 0x{:04X}, SHA-256 {actual_hash}",
            declaration.id,
            decoded.data.len(),
            decoded.vram_destination
        ));
    }
    if actual_hash != declaration.decoded_sha256 {
        return Err(format!(
            "{} decoded SHA-256 mismatch: expected {}, got {actual_hash}",
            declaration.id, declaration.decoded_sha256
        ));
    }
    Ok(decoded.data)
}

fn validate_pack_roundtrip(
    header: &[u8; 6],
    bank: &[u8],
    expected_payload: &[u8],
) -> Result<(), String> {
    let mut probe = vec![0u8; TARGET_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[TARGET_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != HIGH_PATTERN_VRAM || decoded.data != expected_payload {
        return Err("intro bechi mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn summary(build: &IntroBechiBuild, checksum: u16) -> IntroBechiSummary {
    debug_assert_eq!(build.source_payload.len(), build.payload.len());
    IntroBechiSummary {
        source_pattern_tiles: build.source_payload.len() / MD_TILE_BYTES,
        rewritten_tiles: MUTABLE_TILE_END - MUTABLE_TILE_START,
        protected_pattern_bytes: build.source_payload.len() - MUTABLE_BYTES,
        protected_companion_bytes: build.protected_companion_bytes,
        protected_decoration_pixels: SURFACE_WIDTH * (SURFACE_HEIGHT - TEXT_HEIGHT),
        pack_bytes: build.bank.len(),
        checksum,
    }
}
