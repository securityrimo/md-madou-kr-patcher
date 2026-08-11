//! JP-source automatic-prologue `ドキッ` -> `두근!` sprite compiler.
//!
//! The JP scene loads three mode-1 transfers. Only the final six tiles of the
//! high-pattern transfer belong to the effect lettering. They are rebuilt as
//! a 24x16 Korean sprite while every other decoded byte remains JP-owned. The
//! non-executable sprite template changes from 2x3 to 3x2 tiles so the same
//! six-tile budget can display three readable Korean symbols.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::pixel::{
    PixelBounds, encode_md_tiles_column_major, md_color, parse_md_palette, read_verified_rgba,
    reduce_rgba_to_indexed_surface, write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, read_u16, sha256_hex,
    source_range, validate_only_ranges_changed,
};

const DOKI_MAP_HEADER_OFFSET: usize = 0x08_D3E2;
const DOKI_PATTERN_HEADER_OFFSET: usize = 0x08_D3E8;
const DOKI_LOW_PATTERN_HEADER_OFFSET: usize = 0x08_D3EE;
const DOKI_PATTERN_VRAM: u16 = 0x4000;
const DOKI_PATTERN_BANK_OFFSET: usize = 0x28_0000;
const DOKI_PATTERN_BANK_LIMIT: usize = 0x28_8000;
const DOKI_MAP_DECODED_BYTES: usize = 2_176;
const DOKI_SOURCE_PATTERN_BYTES: usize = 7_424;
const DOKI_LOW_PATTERN_BYTES: usize = 3_136;
const DOKI_MUTABLE_TILE_START: usize = 226;
const DOKI_MUTABLE_TILE_END: usize = 232;
const DOKI_MUTABLE_BYTES: usize = (DOKI_MUTABLE_TILE_END - DOKI_MUTABLE_TILE_START) * MD_TILE_BYTES;
const DOKI_SPRITE_COUNT_OFFSET: usize = 0x08_E168;
const DOKI_SPRITE_ENTRY_OFFSET: usize = 0x08_E16A;
const DOKI_SPRITE_SIZE_OFFSET: usize = DOKI_SPRITE_ENTRY_OFFSET + 2;
const PREVIEW_SCALE: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntroDokiSummary {
    pub source_pattern_tiles: usize,
    pub rewritten_tiles: usize,
    pub protected_pattern_bytes: usize,
    pub companion_decoded_bytes: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct IntroDokiManifest {
    schema_version: u32,
    asset_group_id: String,
    master_asset: String,
    master_sha256: String,
    master_width: usize,
    master_height: usize,
    master_alpha_bounds: PixelBounds,
    output_surface: OutputSurface,
    palette_line_words: Vec<String>,
    transparent_palette_index: usize,
    source_palette_indices: Vec<usize>,
    allowed_opaque_palette_indices: Vec<usize>,
    final_palette_role_remap: Vec<PaletteRoleRemap>,
    final_opaque_palette_indices: Vec<usize>,
    final_pixel_role_overrides: Vec<PixelRoleOverride>,
    mutable_tile_range: MutableTileRange,
    sprite_template: SpriteTemplate,
    source_packs: Vec<SourcePack>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
struct PaletteRoleRemap {
    from: usize,
    to: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
struct PixelRoleOverride {
    x: usize,
    y: usize,
    from: usize,
    to: usize,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
    content_box: PixelBounds,
    alpha_threshold: u8,
}

#[derive(Debug, Deserialize)]
struct MutableTileRange {
    start: usize,
    end_exclusive: usize,
}

#[derive(Debug, Deserialize)]
struct SpriteTemplate {
    count_offset: String,
    count: String,
    entry_offset: String,
    source_y: String,
    source_size_link: String,
    target_size_link: String,
    source_attribute: String,
    source_x: String,
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
struct IntroDokiBuild {
    manifest: IntroDokiManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    pixel_fit: Vec<u8>,
    companion_decoded_bytes: usize,
    target_size_link: [u8; 2],
    header: [u8; 6],
    bank: Vec<u8>,
}

/// Insert the Korean automatic-prologue `두근!` sprite into the cumulative ROM.
pub fn apply_intro_doki(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<IntroDokiSummary, String> {
    let build = build_intro_doki(source, assets_dir)?;
    let bank_end = DOKI_PATTERN_BANK_OFFSET + build.bank.len();
    if bank_end > DOKI_PATTERN_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "intro doki pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    apply_expected_write(
        output,
        DOKI_PATTERN_HEADER_OFFSET,
        source_range(
            source,
            DOKI_PATTERN_HEADER_OFFSET,
            build.header.len(),
            "intro doki source pattern header",
        )?,
        &build.header,
        "intro doki pattern pack header",
    )?;
    apply_expected_write(
        output,
        DOKI_SPRITE_SIZE_OFFSET,
        source_range(
            source,
            DOKI_SPRITE_SIZE_OFFSET,
            build.target_size_link.len(),
            "intro doki source sprite size",
        )?,
        &build.target_size_link,
        "intro doki 3x2 sprite data",
    )?;
    apply_expected_write(
        output,
        DOKI_PATTERN_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "intro doki expanded pattern pack",
    )?;

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after intro doki graphics",
    )?;
    let changed_ranges = [
        (
            DOKI_PATTERN_HEADER_OFFSET,
            DOKI_PATTERN_HEADER_OFFSET + build.header.len(),
        ),
        (
            DOKI_SPRITE_SIZE_OFFSET,
            DOKI_SPRITE_SIZE_OFFSET + build.target_size_link.len(),
        ),
        (DOKI_PATTERN_BANK_OFFSET, bank_end),
        (CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2),
    ];
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let inserted = decode_mode1_pack_entry(output, DOKI_PATTERN_HEADER_OFFSET)?;
    if inserted.vram_destination != DOKI_PATTERN_VRAM || inserted.data != build.payload {
        return Err(
            "inserted intro doki pack does not decode to the planned JP-derived payload"
                .to_string(),
        );
    }
    validate_sprite_template(output, &build.manifest, true)?;
    for declaration in &build.manifest.source_packs {
        let header_offset = parse_hex(&declaration.header_offset)?;
        if header_offset == DOKI_PATTERN_HEADER_OFFSET {
            continue;
        }
        let source_pack = checked_source_pack(source, declaration)?;
        let output_pack = decode_mode1_pack_entry(output, header_offset)?;
        if output_pack.vram_destination != parse_u16_hex(&declaration.vram_destination)?
            || output_pack.data != source_pack
        {
            return Err(format!(
                "intro doki insertion changed protected JP transfer {}",
                declaration.id
            ));
        }
    }

    eprintln!("JP graphics GFX-INTRO-DOKI Expected Writes:");
    eprintln!(
        "  0x{DOKI_PATTERN_HEADER_OFFSET:06X}..0x{:06X}  intro doki pattern header ({} bytes)",
        DOKI_PATTERN_HEADER_OFFSET + build.header.len(),
        build.header.len()
    );
    eprintln!(
        "  0x{DOKI_SPRITE_SIZE_OFFSET:06X}..0x{:06X}  intro doki 3x2 sprite data (2 bytes)",
        DOKI_SPRITE_SIZE_OFFSET + build.target_size_link.len()
    );
    eprintln!(
        "  0x{DOKI_PATTERN_BANK_OFFSET:06X}..0x{bank_end:06X}  intro doki pattern pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render an 8x checker-backed JP/KR comparison of the exact six-tile sprite.
///
/// The JP 2x3-tile source is centered in the left 24x24 panel. The KR 3x2-tile
/// replacement is centered in the right panel. Both use the same source CRAM
/// line, so color differences are visible without runtime state.
pub fn write_intro_doki_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<IntroDokiSummary, String> {
    let build = build_intro_doki(source, assets_dir)?;
    let surface = build.manifest.output_surface;
    let mutable_start = DOKI_MUTABLE_TILE_START * MD_TILE_BYTES;
    let mutable_end = DOKI_MUTABLE_TILE_END * MD_TILE_BYTES;
    let source_pixels = decode_md_tiles_column_major(
        &build.source_payload[mutable_start..mutable_end],
        16,
        24,
        "intro doki JP preview",
    )?;
    const PANEL_SIZE: usize = 24;
    const PANEL_GAP: usize = 4;
    let comparison_width = PANEL_SIZE * 2 + PANEL_GAP;
    let comparison_height = PANEL_SIZE;
    let preview_width = comparison_width * PREVIEW_SCALE;
    let preview_height = comparison_height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for y in 0..comparison_height {
        for x in 0..comparison_width {
            let color = if (x / 2 + y / 2).is_multiple_of(2) {
                [210, 210, 210]
            } else {
                [242, 242, 242]
            };
            for scale_y in 0..PREVIEW_SCALE {
                for scale_x in 0..PREVIEW_SCALE {
                    let preview_x = x * PREVIEW_SCALE + scale_x;
                    let preview_y = y * PREVIEW_SCALE + scale_y;
                    let offset = (preview_y * preview_width + preview_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                }
            }
        }
    }
    paint_indexed_preview(
        &mut rgba,
        preview_width,
        IndexedPreviewSurface {
            pixels: &source_pixels,
            width: 16,
            height: 24,
            origin_x: 4,
            origin_y: 0,
        },
        &build,
    );
    paint_indexed_preview(
        &mut rgba,
        preview_width,
        IndexedPreviewSurface {
            pixels: &build.pixel_fit,
            width: surface.width,
            height: surface.height,
            origin_x: PANEL_SIZE + PANEL_GAP,
            origin_y: 4,
        },
        &build,
    );
    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "intro doki",
    )?;
    Ok(summary(&build, 0))
}

fn paint_indexed_preview(
    rgba: &mut [u8],
    preview_width: usize,
    surface: IndexedPreviewSurface<'_>,
    build: &IntroDokiBuild,
) {
    for y in 0..surface.height {
        for x in 0..surface.width {
            let palette_index = surface.pixels[y * surface.width + x] as usize;
            if palette_index == build.manifest.transparent_palette_index {
                continue;
            }
            let color = md_color(build.palette[palette_index]);
            for scale_y in 0..PREVIEW_SCALE {
                for scale_x in 0..PREVIEW_SCALE {
                    let preview_x = (surface.origin_x + x) * PREVIEW_SCALE + scale_x;
                    let preview_y = (surface.origin_y + y) * PREVIEW_SCALE + scale_y;
                    let offset = (preview_y * preview_width + preview_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct IndexedPreviewSurface<'a> {
    pixels: &'a [u8],
    width: usize,
    height: usize,
    origin_x: usize,
    origin_y: usize,
}

fn build_intro_doki(source: &[u8], assets_dir: &Path) -> Result<IntroDokiBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    validate_sprite_template(source, &manifest, false)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "intro doki")?;
    let master = read_master(assets_dir, &manifest)?;
    let pixel_fit = remap_to_source_palette_roles(
        reduce_master_to_surface(&master, &manifest, &palette)?,
        &manifest,
    )?;
    let encoded_tiles = encode_md_tiles_column_major(
        &pixel_fit,
        manifest.output_surface.width,
        manifest.output_surface.height,
        "intro doki",
    )?;
    if encoded_tiles.len() != DOKI_MUTABLE_BYTES {
        return Err(format!(
            "intro doki encoded as {} bytes, expected {DOKI_MUTABLE_BYTES}",
            encoded_tiles.len()
        ));
    }

    let mut companion_decoded_bytes = 0usize;
    for declaration in &manifest.source_packs {
        let decoded = checked_source_pack(source, declaration)?;
        if parse_hex(&declaration.header_offset)? != DOKI_PATTERN_HEADER_OFFSET {
            companion_decoded_bytes += decoded.len();
        }
    }
    let pattern_declaration = source_pack(&manifest, DOKI_PATTERN_HEADER_OFFSET)?;
    let source_payload = checked_source_pack(source, pattern_declaration)?;
    let mutable_start = DOKI_MUTABLE_TILE_START * MD_TILE_BYTES;
    let mutable_end = DOKI_MUTABLE_TILE_END * MD_TILE_BYTES;
    validate_source_effect_tiles(&source_payload[mutable_start..mutable_end], &manifest)?;
    let mut payload = source_payload.clone();
    payload[mutable_start..mutable_end].copy_from_slice(&encoded_tiles);
    if payload[..mutable_start] != source_payload[..mutable_start]
        || payload[mutable_end..] != source_payload[mutable_end..]
    {
        return Err("intro doki compiler changed protected JP pattern bytes".to_string());
    }

    let encoded = encode_locked_mode1_pack(DOKI_PATTERN_BANK_OFFSET, DOKI_PATTERN_VRAM, &payload)?;
    validate_pack_roundtrip(&encoded.header, &encoded.bank, &payload)?;
    let target_size_link = parse_u16_hex(&manifest.sprite_template.target_size_link)?.to_be_bytes();

    Ok(IntroDokiBuild {
        manifest,
        palette,
        source_payload,
        payload,
        pixel_fit,
        companion_decoded_bytes,
        target_size_link,
        header: encoded.header,
        bank: encoded.bank,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<IntroDokiManifest, String> {
    let path = assets_dir.join("graphics_text/intro_doki.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read intro doki source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid intro doki source {}: {error}", path.display()))
}

fn read_master(assets_dir: &Path, manifest: &IntroDokiManifest) -> Result<Vec<u8>, String> {
    read_verified_rgba(
        assets_dir,
        &manifest.master_asset,
        &manifest.master_sha256,
        manifest.master_width,
        manifest.master_height,
        manifest.master_alpha_bounds,
        "intro doki master",
    )
}

fn validate_manifest_shape(manifest: &IntroDokiManifest) -> Result<(), String> {
    if manifest.schema_version != 1 || manifest.asset_group_id != "GFX-INTRO-DOKI" {
        return Err("unsupported intro doki manifest identity".to_string());
    }
    let surface = manifest.output_surface;
    let box_ = surface.content_box;
    if surface.width != 24
        || surface.height != 16
        || !surface.width.is_multiple_of(8)
        || !surface.height.is_multiple_of(8)
        || box_.x + box_.width > surface.width
        || box_.y + box_.height > surface.height
        || box_.width == 0
        || box_.height == 0
        || surface.alpha_threshold == 0
        || surface.alpha_threshold == u8::MAX
    {
        return Err("intro doki output surface drifted from the six-tile sprite".to_string());
    }
    if manifest.transparent_palette_index != 0
        || manifest.allowed_opaque_palette_indices.is_empty()
        || manifest
            .allowed_opaque_palette_indices
            .contains(&manifest.transparent_palette_index)
        || manifest
            .allowed_opaque_palette_indices
            .iter()
            .any(|&index| index >= 16)
    {
        return Err("intro doki palette admission is invalid".to_string());
    }
    if manifest
        .source_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        != BTreeSet::from([0, 1, 5])
    {
        return Err("intro doki JP source palette roles drifted".to_string());
    }
    let expected_role_remap = [
        PaletteRoleRemap { from: 1, to: 5 },
        PaletteRoleRemap { from: 5, to: 5 },
        PaletteRoleRemap { from: 7, to: 1 },
        PaletteRoleRemap { from: 9, to: 1 },
        PaletteRoleRemap { from: 11, to: 1 },
    ];
    if manifest.final_palette_role_remap.as_slice() != expected_role_remap
        || manifest.final_opaque_palette_indices.as_slice() != [1, 5]
    {
        return Err("intro doki final black-fill and blue-gray-outline roles drifted".to_string());
    }
    let expected_pixel_overrides = [
        PixelRoleOverride {
            x: 5,
            y: 8,
            from: 1,
            to: 5,
        },
        PixelRoleOverride {
            x: 3,
            y: 9,
            from: 1,
            to: 5,
        },
        PixelRoleOverride {
            x: 7,
            y: 9,
            from: 5,
            to: 1,
        },
        PixelRoleOverride {
            x: 17,
            y: 9,
            from: 1,
            to: 5,
        },
        PixelRoleOverride {
            x: 19,
            y: 9,
            from: 5,
            to: 0,
        },
        PixelRoleOverride {
            x: 1,
            y: 10,
            from: 1,
            to: 5,
        },
        PixelRoleOverride {
            x: 4,
            y: 10,
            from: 5,
            to: 1,
        },
        PixelRoleOverride {
            x: 19,
            y: 10,
            from: 5,
            to: 0,
        },
        PixelRoleOverride {
            x: 2,
            y: 11,
            from: 5,
            to: 1,
        },
        PixelRoleOverride {
            x: 3,
            y: 11,
            from: 5,
            to: 1,
        },
    ];
    if manifest.final_pixel_role_overrides.as_slice() != expected_pixel_overrides {
        return Err("intro doki reviewed per-pixel role overrides drifted".to_string());
    }
    if manifest.mutable_tile_range.start != DOKI_MUTABLE_TILE_START
        || manifest.mutable_tile_range.end_exclusive != DOKI_MUTABLE_TILE_END
        || DOKI_MUTABLE_TILE_END * MD_TILE_BYTES != DOKI_SOURCE_PATTERN_BYTES
    {
        return Err("intro doki mutable range drifted from the final six JP tiles".to_string());
    }

    let expected_packs = [
        (
            "intro-doki-map",
            DOKI_MAP_HEADER_OFFSET,
            0xE000,
            DOKI_MAP_DECODED_BYTES,
        ),
        (
            "intro-doki-high-patterns",
            DOKI_PATTERN_HEADER_OFFSET,
            DOKI_PATTERN_VRAM,
            DOKI_SOURCE_PATTERN_BYTES,
        ),
        (
            "intro-doki-low-patterns",
            DOKI_LOW_PATTERN_HEADER_OFFSET,
            0x2000,
            DOKI_LOW_PATTERN_BYTES,
        ),
    ];
    if manifest.source_packs.len() != expected_packs.len() {
        return Err("intro doki manifest must declare exactly three JP transfers".to_string());
    }
    for (pack, (id, header, vram, decoded_bytes)) in
        manifest.source_packs.iter().zip(expected_packs)
    {
        if pack.id != id
            || parse_hex(&pack.header_offset)? != header
            || parse_u16_hex(&pack.vram_destination)? != vram
            || pack.decoded_bytes != decoded_bytes
        {
            return Err(format!(
                "intro doki source declaration {} drifted from the JP consumer",
                pack.id
            ));
        }
    }

    let template = &manifest.sprite_template;
    let source_size = parse_u16_hex(&template.source_size_link)?;
    let target_size = parse_u16_hex(&template.target_size_link)?;
    let source_attribute = parse_u16_hex(&template.source_attribute)?;
    let expected_tile =
        DOKI_PATTERN_VRAM / MD_TILE_BYTES as u16 + manifest.mutable_tile_range.start as u16;
    if parse_hex(&template.count_offset)? != DOKI_SPRITE_COUNT_OFFSET
        || parse_u16_hex(&template.count)? != 1
        || parse_hex(&template.entry_offset)? != DOKI_SPRITE_ENTRY_OFFSET
        || parse_u16_hex(&template.source_y)? != 0x0080
        || source_size != sprite_size_link(2, 3, 4)?
        || target_size != sprite_size_link(3, 2, 4)?
        || source_attribute & 0x07FF != expected_tile
        || (source_attribute >> 13) & 0x03 != 2
        || source_attribute & 0x9800 != 0
        || parse_u16_hex(&template.source_x)? != 0x0080
    {
        return Err("intro doki sprite template drifted from the JP consumer".to_string());
    }
    Ok(())
}

fn reduce_master_to_surface(
    master: &[u8],
    manifest: &IntroDokiManifest,
    palette: &[u16; 16],
) -> Result<Vec<u8>, String> {
    let surface = manifest.output_surface;
    reduce_rgba_to_indexed_surface(
        master,
        manifest.master_width,
        manifest.master_height,
        manifest.master_alpha_bounds,
        surface.width,
        surface.height,
        surface.content_box,
        surface.alpha_threshold,
        manifest.transparent_palette_index,
        palette,
        &manifest.allowed_opaque_palette_indices,
        "intro doki",
    )
}

fn remap_to_source_palette_roles(
    mut pixels: Vec<u8>,
    manifest: &IntroDokiManifest,
) -> Result<Vec<u8>, String> {
    for pixel in &mut pixels {
        let palette_index = usize::from(*pixel);
        if palette_index == manifest.transparent_palette_index {
            continue;
        }
        let replacement = manifest
            .final_palette_role_remap
            .iter()
            .find(|mapping| mapping.from == palette_index)
            .ok_or_else(|| {
                format!("intro doki palette index {palette_index} has no final JP-role mapping")
            })?;
        *pixel = u8::try_from(replacement.to).map_err(|_| {
            format!(
                "intro doki final palette index {} exceeds 8 bits",
                replacement.to
            )
        })?;
    }
    for role_override in &manifest.final_pixel_role_overrides {
        let offset = role_override
            .y
            .checked_mul(manifest.output_surface.width)
            .and_then(|row| row.checked_add(role_override.x))
            .filter(|&offset| offset < pixels.len())
            .ok_or_else(|| {
                format!(
                    "intro doki pixel override ({}, {}) is outside the output surface",
                    role_override.x, role_override.y
                )
            })?;
        if usize::from(pixels[offset]) != role_override.from {
            return Err(format!(
                "intro doki pixel override ({}, {}) expected palette {}, got {}",
                role_override.x, role_override.y, role_override.from, pixels[offset]
            ));
        }
        pixels[offset] = u8::try_from(role_override.to).map_err(|_| {
            format!(
                "intro doki pixel override target {} exceeds 8 bits",
                role_override.to
            )
        })?;
    }
    let actual = pixels
        .iter()
        .map(|&pixel| usize::from(pixel))
        .collect::<BTreeSet<_>>();
    let mut expected = manifest
        .final_opaque_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    expected.insert(manifest.transparent_palette_index);
    if actual != expected {
        return Err(format!(
            "intro doki final JP-role palette drifted: expected {expected:?}, got {actual:?}"
        ));
    }
    Ok(pixels)
}

fn validate_source_effect_tiles(tiles: &[u8], manifest: &IntroDokiManifest) -> Result<(), String> {
    let pixels = decode_md_tiles_column_major(tiles, 16, 24, "intro doki JP effect")?;
    let actual = pixels
        .iter()
        .map(|&pixel| pixel as usize)
        .collect::<BTreeSet<_>>();
    let expected = manifest
        .source_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "intro doki JP effect palette drifted: expected {expected:?}, got {actual:?}"
        ));
    }
    Ok(())
}

fn decode_md_tiles_column_major(
    tiles: &[u8],
    width: usize,
    height: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    if !width.is_multiple_of(8)
        || !height.is_multiple_of(8)
        || tiles.len() != width / 8 * (height / 8) * MD_TILE_BYTES
    {
        return Err(format!("{label} tile payload length is invalid"));
    }
    let tile_rows = height / 8;
    let mut pixels = vec![0u8; width * height];
    for tile_x in 0..width / 8 {
        for tile_y in 0..tile_rows {
            let tile = tile_x * tile_rows + tile_y;
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
                    pixels[y * width + x] = pixel;
                }
            }
        }
    }
    Ok(pixels)
}

fn validate_sprite_template(
    rom: &[u8],
    manifest: &IntroDokiManifest,
    target: bool,
) -> Result<(), String> {
    let template = &manifest.sprite_template;
    let count_offset = parse_hex(&template.count_offset)?;
    let entry_offset = parse_hex(&template.entry_offset)?;
    let expected_count = parse_u16_hex(&template.count)?;
    let expected_words = [
        parse_u16_hex(&template.source_y)?,
        parse_u16_hex(if target {
            &template.target_size_link
        } else {
            &template.source_size_link
        })?,
        parse_u16_hex(&template.source_attribute)?,
        parse_u16_hex(&template.source_x)?,
    ];
    if read_u16(rom, count_offset, "intro doki sprite count")? != expected_count {
        return Err("intro doki sprite count drifted from the JP template".to_string());
    }
    for (word_index, expected) in expected_words.into_iter().enumerate() {
        let actual = read_u16(
            rom,
            entry_offset + word_index * 2,
            "intro doki sprite template",
        )?;
        if actual != expected {
            return Err(format!(
                "intro doki sprite word {word_index} drifted: expected 0x{expected:04X}, got 0x{actual:04X}"
            ));
        }
    }
    Ok(())
}

fn source_pack(manifest: &IntroDokiManifest, header_offset: usize) -> Result<&SourcePack, String> {
    manifest
        .source_packs
        .iter()
        .find(|pack| {
            parse_hex(&pack.header_offset)
                .map(|offset| offset == header_offset)
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("intro doki manifest has no source pack at 0x{header_offset:06X}"))
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
    let mut probe = vec![0u8; DOKI_PATTERN_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[DOKI_PATTERN_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != DOKI_PATTERN_VRAM || decoded.data != expected_payload {
        return Err("intro doki mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn sprite_size_link(width_tiles: u16, height_tiles: u16, link: u16) -> Result<u16, String> {
    if !(1..=4).contains(&width_tiles) || !(1..=4).contains(&height_tiles) || link > 0x7F {
        return Err("intro doki sprite dimensions or link are invalid".to_string());
    }
    Ok(((width_tiles - 1) << 10) | ((height_tiles - 1) << 8) | link)
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn summary(build: &IntroDokiBuild, checksum: u16) -> IntroDokiSummary {
    debug_assert_eq!(build.source_payload.len(), build.payload.len());
    IntroDokiSummary {
        source_pattern_tiles: build.source_payload.len() / MD_TILE_BYTES,
        rewritten_tiles: DOKI_MUTABLE_TILE_END - DOKI_MUTABLE_TILE_START,
        protected_pattern_bytes: build.source_payload.len() - DOKI_MUTABLE_BYTES,
        companion_decoded_bytes: build.companion_decoded_bytes,
        pack_bytes: build.bank.len(),
        checksum,
    }
}
