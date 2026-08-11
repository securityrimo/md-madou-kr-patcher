//! JP-source demon-escape `びゅんっ` -> `휙!` effect compiler.
//!
//! Two source sprites consume sixteen tiles. Fourteen are arranged as the
//! authored 40x24 sparse surface; the final two are residual source pixels
//! that the older EN tool left behind. This compiler owns and clears all
//! sixteen consumer tiles.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use super::font_effect::{blit_indexed_glyph, read_verified_font, render_indexed_glyph};
use super::pixel::{md_color, parse_md_palette, write_rgba_png};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const BYUN_HEADER_OFFSET: usize = 0x07_5FEE;
const BYUN_VRAM_DESTINATION: u16 = 0x5600;
const BYUN_BANK_OFFSET: usize = 0x2C_8000;
const BYUN_BANK_LIMIT: usize = 0x2D_0000;
const BYUN_SOURCE_BYTES: usize = 3_552;
const BYUN_SOURCE_TILES: usize = BYUN_SOURCE_BYTES / MD_TILE_BYTES;
const BYUN_TILE_START: usize = 46;
const BYUN_MAPPED_TILE_END: usize = 60;
const BYUN_CONSUMER_TILE_END: usize = 62;
const BYUN_MAPPED_TILES: usize = BYUN_MAPPED_TILE_END - BYUN_TILE_START;
const BYUN_CONSUMER_TILES: usize = BYUN_CONSUMER_TILE_END - BYUN_TILE_START;
const BYUN_WIDTH: usize = 40;
const BYUN_HEIGHT: usize = 24;
const BYUN_LAYOUT: [[i8; 5]; 3] = [[0, 3, 6, 9, -1], [1, 4, 7, 10, 12], [2, 5, 8, 11, 13]];
const PREVIEW_SCALE: usize = 10;
const PREVIEW_GAP: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemonByunSummary {
    pub source_tiles: usize,
    pub rewritten_tiles: usize,
    pub mapped_tiles: usize,
    pub cleared_residual_tiles: usize,
    pub protected_source_bytes: usize,
    pub consumer_sprites: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct DemonByunManifest {
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
    target_palette_index: usize,
    mutable_tile_range: MutableTileRange,
    tile_layout: Vec<Vec<Option<usize>>>,
    source_pack: SourcePack,
    sprite_consumer: SpriteConsumer,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
}

#[derive(Debug, Deserialize)]
struct MutableTileRange {
    start: usize,
    mapped_end_exclusive: usize,
    consumer_end_exclusive: usize,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
struct SourcePack {
    id: String,
    header_offset: String,
    vram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
}

#[derive(Debug, Deserialize)]
struct SpriteConsumer {
    table_base: String,
    table_entries: Vec<String>,
    definition_offsets: Vec<String>,
    records: Vec<SpriteRecord>,
}

#[derive(Debug, Deserialize)]
struct SpriteRecord {
    y: String,
    size_link: String,
    tile: String,
    x: String,
}

#[derive(Debug)]
struct DemonByunBuild {
    manifest: DemonByunManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    source_surface: Vec<u8>,
    target_surface: Vec<u8>,
    header: [u8; 6],
    bank: Vec<u8>,
}

pub fn apply_demon_byun(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<DemonByunSummary, String> {
    let build = build_demon_byun(source, assets_dir)?;
    let bank_end = BYUN_BANK_OFFSET + build.bank.len();
    if bank_end > BYUN_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "demon byun pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(3);
    apply_expected_write(
        output,
        BYUN_HEADER_OFFSET,
        source_range(
            source,
            BYUN_HEADER_OFFSET,
            build.header.len(),
            "demon byun source header",
        )?,
        &build.header,
        "demon byun pack header",
    )?;
    changed_ranges.push((BYUN_HEADER_OFFSET, BYUN_HEADER_OFFSET + build.header.len()));
    apply_expected_write(
        output,
        BYUN_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "demon byun expanded pack",
    )?;
    changed_ranges.push((BYUN_BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after demon byun graphics",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let inserted = decode_mode1_pack_entry(output, BYUN_HEADER_OFFSET)?;
    if inserted.vram_destination != BYUN_VRAM_DESTINATION || inserted.data != build.payload {
        return Err("inserted demon byun pack does not match the planned payload".to_string());
    }

    eprintln!("JP graphics GFX-EVENT-DEMON-BYUN Expected Writes:");
    eprintln!(
        "  0x{BYUN_HEADER_OFFSET:06X}..0x{:06X}  demon-escape header ({} bytes)",
        BYUN_HEADER_OFFSET + build.header.len(),
        build.header.len()
    );
    eprintln!(
        "  0x{BYUN_BANK_OFFSET:06X}..0x{bank_end:06X}  demon byun pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render the exact mapped JP and Korean 40x24 surfaces side by side.
pub fn write_demon_byun_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<DemonByunSummary, String> {
    let build = build_demon_byun(source, assets_dir)?;
    let contact_width = BYUN_WIDTH * 2 + PREVIEW_GAP;
    let preview_width = contact_width * PREVIEW_SCALE;
    let preview_height = BYUN_HEIGHT * PREVIEW_SCALE;
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
        let x = panel * (BYUN_WIDTH + PREVIEW_GAP);
        draw_scaled_surface(
            &mut rgba,
            preview_width,
            x,
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
        "Demon byun",
    )?;
    Ok(summary(&build, 0))
}

fn build_demon_byun(source: &[u8], assets_dir: &Path) -> Result<DemonByunBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "demon byun")?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "demon byun",
    )?;
    let source_payload = checked_source_pack(source, &manifest.source_pack)?;
    validate_sprite_consumer(source, &manifest)?;

    let mutable_start = BYUN_TILE_START * MD_TILE_BYTES;
    let mutable_end = BYUN_CONSUMER_TILE_END * MD_TILE_BYTES;
    let source_tiles = &source_payload[mutable_start..mutable_end];
    validate_source_tiles(source_tiles, &manifest)?;
    let source_surface = decode_sparse_surface(
        &source_tiles[..BYUN_MAPPED_TILES * MD_TILE_BYTES],
        &manifest,
    )?;
    let target_surface = render_target_surface(&font, &manifest)?;
    let mut replacement_tiles = encode_sparse_surface(&target_surface, &manifest)?;
    replacement_tiles.resize(BYUN_CONSUMER_TILES * MD_TILE_BYTES, 0);

    let mut payload = source_payload.clone();
    payload[mutable_start..mutable_end].copy_from_slice(&replacement_tiles);
    if payload[..mutable_start] != source_payload[..mutable_start]
        || payload[mutable_end..] != source_payload[mutable_end..]
    {
        return Err("demon byun compiler changed protected JP bytes".to_string());
    }

    let encoded = encode_locked_mode1_pack(BYUN_BANK_OFFSET, BYUN_VRAM_DESTINATION, &payload)?;
    validate_pack_roundtrip(&encoded.header, &encoded.bank, &payload)?;
    Ok(DemonByunBuild {
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

fn read_manifest(assets_dir: &Path) -> Result<DemonByunManifest, String> {
    let path = assets_dir.join("graphics_text/demon_byun.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read demon byun source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid demon byun source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &DemonByunManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-EVENT-DEMON-BYUN"
        || !manifest.source_policy.contains("JP")
        || manifest.font_asset != "neodgm.ttf"
        || manifest.jp_text != "びゅんっ"
        || manifest.ko != "휙!"
        || manifest.output_surface.width != BYUN_WIDTH
        || manifest.output_surface.height != BYUN_HEIGHT
        || manifest.transparent_palette_index != 0
        || manifest
            .source_palette_indices
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != BTreeSet::from([0, 13])
        || manifest.target_palette_index != 13
    {
        return Err("unsupported demon byun manifest identity".to_string());
    }
    let range = &manifest.mutable_tile_range;
    if range.start != BYUN_TILE_START
        || range.mapped_end_exclusive != BYUN_MAPPED_TILE_END
        || range.consumer_end_exclusive != BYUN_CONSUMER_TILE_END
        || range.source_sha256.len() != 64
    {
        return Err("demon byun mutable range drifted".to_string());
    }
    validate_layout(manifest)?;
    let pack = &manifest.source_pack;
    if pack.id != "demon-escape-byun"
        || parse_hex(&pack.header_offset)? != BYUN_HEADER_OFFSET
        || parse_u16_hex(&pack.vram_destination)? != BYUN_VRAM_DESTINATION
        || pack.decoded_bytes != BYUN_SOURCE_BYTES
        || pack.decoded_sha256.len() != 64
    {
        return Err("demon byun source pack declaration drifted".to_string());
    }
    let consumer = &manifest.sprite_consumer;
    if parse_hex(&consumer.table_base)? != 0x06_A984
        || consumer.table_entries != ["0x06A98E".to_string(), "0x06A990".to_string()]
        || consumer.definition_offsets != ["0x06AA1C".to_string(), "0x06AA26".to_string()]
        || consumer.records.len() != 2
    {
        return Err("demon byun sprite consumer declaration drifted".to_string());
    }
    Ok(())
}

fn validate_layout(manifest: &DemonByunManifest) -> Result<(), String> {
    if manifest.tile_layout.len() != BYUN_LAYOUT.len() {
        return Err("demon byun tile layout must have three rows".to_string());
    }
    let mut mapped = BTreeSet::new();
    for (row, expected_row) in manifest.tile_layout.iter().zip(BYUN_LAYOUT) {
        if row.len() != expected_row.len() {
            return Err("demon byun tile layout must have five columns".to_string());
        }
        for (&actual, expected) in row.iter().zip(expected_row) {
            let expected = usize::try_from(expected).ok();
            if actual != expected {
                return Err("demon byun tile layout drifted".to_string());
            }
            if let Some(tile) = actual {
                mapped.insert(tile);
            }
        }
    }
    if mapped != (0..BYUN_MAPPED_TILES).collect::<BTreeSet<_>>() {
        return Err("demon byun sparse layout does not own fourteen tiles".to_string());
    }
    Ok(())
}

fn checked_source_pack(source: &[u8], declaration: &SourcePack) -> Result<Vec<u8>, String> {
    let decoded = decode_mode1_pack_entry(source, parse_hex(&declaration.header_offset)?)?;
    let actual_hash = sha256_hex(&decoded.data);
    if decoded.vram_destination != parse_u16_hex(&declaration.vram_destination)?
        || decoded.data.len() != declaration.decoded_bytes
        || actual_hash != declaration.decoded_sha256
    {
        return Err(format!(
            "{} source pack drifted: VRAM 0x{:04X}, {} bytes, SHA-256 {actual_hash}",
            declaration.id,
            decoded.vram_destination,
            decoded.data.len()
        ));
    }
    Ok(decoded.data)
}

fn validate_source_tiles(tiles: &[u8], manifest: &DemonByunManifest) -> Result<(), String> {
    if tiles.len() != BYUN_CONSUMER_TILES * MD_TILE_BYTES {
        return Err("demon byun source tile range length is invalid".to_string());
    }
    let actual_hash = sha256_hex(tiles);
    if actual_hash != manifest.mutable_tile_range.source_sha256 {
        return Err(format!(
            "demon byun source SHA-256 mismatch: expected {}, got {actual_hash}",
            manifest.mutable_tile_range.source_sha256
        ));
    }
    let roles = tiles
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .map(usize::from)
        .collect::<BTreeSet<_>>();
    let expected = manifest
        .source_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if roles != expected {
        return Err(format!(
            "demon byun source palette drifted: expected {expected:?}, got {roles:?}"
        ));
    }
    Ok(())
}

fn validate_sprite_consumer(source: &[u8], manifest: &DemonByunManifest) -> Result<(), String> {
    let consumer = &manifest.sprite_consumer;
    let table_base = parse_hex(&consumer.table_base)?;
    let pack_base_tile = usize::from(BYUN_VRAM_DESTINATION) / MD_TILE_BYTES;
    let mut owned_tiles = 0usize;
    for index in 0..consumer.records.len() {
        let entry = parse_hex(&consumer.table_entries[index])?;
        let definition = parse_hex(&consumer.definition_offsets[index])?;
        let relative = source_range(source, entry, 2, "demon byun table entry")?;
        if table_base + u16::from_be_bytes([relative[0], relative[1]]) as usize != definition {
            return Err(format!("demon byun table entry {index} drifted"));
        }
        let bytes = source_range(source, definition, 10, "demon byun sprite definition")?;
        if u16::from_be_bytes([bytes[0], bytes[1]]) != 1 {
            return Err(format!("demon byun definition {index} is not one sprite"));
        }
        let actual = [
            u16::from_be_bytes([bytes[2], bytes[3]]),
            u16::from_be_bytes([bytes[4], bytes[5]]),
            u16::from_be_bytes([bytes[6], bytes[7]]),
            u16::from_be_bytes([bytes[8], bytes[9]]),
        ];
        let declared = &consumer.records[index];
        let expected = [
            parse_u16_hex(&declared.y)?,
            parse_u16_hex(&declared.size_link)?,
            parse_u16_hex(&declared.tile)?,
            parse_u16_hex(&declared.x)?,
        ];
        if actual != expected {
            return Err(format!("demon byun sprite record {index} drifted"));
        }
        let tile = usize::from(actual[2] & 0x07FF);
        let relative_start = tile
            .checked_sub(pack_base_tile)
            .ok_or_else(|| "demon byun sprite tile precedes its pack".to_string())?;
        let tiles = sprite_tile_count(actual[1]);
        if relative_start != BYUN_TILE_START + owned_tiles {
            return Err(format!(
                "demon byun sprite {index} starts at relative tile {relative_start}"
            ));
        }
        owned_tiles += tiles;
    }
    if owned_tiles != BYUN_CONSUMER_TILES {
        return Err(format!(
            "demon byun sprites own {owned_tiles} tiles, expected {BYUN_CONSUMER_TILES}"
        ));
    }
    Ok(())
}

fn sprite_tile_count(size_link: u16) -> usize {
    let size = (size_link >> 8) & 0x0F;
    let width = usize::from((size >> 2) + 1);
    let height = usize::from((size & 0x03) + 1);
    width * height
}

fn render_target_surface(font: &Font, manifest: &DemonByunManifest) -> Result<Vec<u8>, String> {
    let transparent = manifest.transparent_palette_index as u8;
    let opaque = manifest.target_palette_index as u8;
    let mut output = vec![transparent; BYUN_WIDTH * BYUN_HEIGHT];
    // This source effect exposes only one opaque palette role. Reusing that
    // role for both outline rings closes the dense Hangul counters and turns
    // the glyph into a block, so retain the native glyph core without an
    // artificial outline.
    let glyph = render_indexed_glyph(font, '휙', transparent, opaque, transparent, transparent);
    blit_indexed_glyph(&mut output, BYUN_WIDTH, BYUN_HEIGHT, 2, 4, &glyph)?;
    draw_exclamation(&mut output, opaque)?;
    for y in 0..8 {
        for x in 32..40 {
            if output[y * BYUN_WIDTH + x] != transparent {
                return Err("demon byun target uses the unmapped top-right cell".to_string());
            }
        }
    }
    Ok(output)
}

fn draw_exclamation(output: &mut [u8], opaque: u8) -> Result<(), String> {
    if output.len() != BYUN_WIDTH * BYUN_HEIGHT {
        return Err("demon byun exclamation surface is invalid".to_string());
    }
    for y in 4..15 {
        for x in 24..28 {
            output[y * BYUN_WIDTH + x] = opaque;
        }
    }
    for y in 17..21 {
        for x in 24..28 {
            output[y * BYUN_WIDTH + x] = opaque;
        }
    }
    Ok(())
}

fn encode_sparse_surface(pixels: &[u8], manifest: &DemonByunManifest) -> Result<Vec<u8>, String> {
    if pixels.len() != BYUN_WIDTH * BYUN_HEIGHT {
        return Err("demon byun surface length is invalid".to_string());
    }
    let transparent = manifest.transparent_palette_index as u8;
    let mut output = vec![0u8; BYUN_MAPPED_TILES * MD_TILE_BYTES];
    for (tile_y, row) in manifest.tile_layout.iter().enumerate() {
        for (tile_x, &tile) in row.iter().enumerate() {
            let Some(tile) = tile else {
                for y in tile_y * 8..(tile_y + 1) * 8 {
                    for x in tile_x * 8..(tile_x + 1) * 8 {
                        if pixels[y * BYUN_WIDTH + x] != transparent {
                            return Err(format!(
                                "demon byun has an opaque pixel in unmapped cell ({tile_x},{tile_y})"
                            ));
                        }
                    }
                }
                continue;
            };
            let destination = &mut output[tile * MD_TILE_BYTES..(tile + 1) * MD_TILE_BYTES];
            for local_y in 0..8 {
                for pair_x in 0..4 {
                    let x = tile_x * 8 + pair_x * 2;
                    let y = tile_y * 8 + local_y;
                    let left = pixels[y * BYUN_WIDTH + x];
                    let right = pixels[y * BYUN_WIDTH + x + 1];
                    destination[local_y * 4 + pair_x] = (left << 4) | right;
                }
            }
        }
    }
    Ok(output)
}

fn decode_sparse_surface(tiles: &[u8], manifest: &DemonByunManifest) -> Result<Vec<u8>, String> {
    if tiles.len() != BYUN_MAPPED_TILES * MD_TILE_BYTES {
        return Err("demon byun mapped tile length is invalid".to_string());
    }
    let mut output = vec![manifest.transparent_palette_index as u8; BYUN_WIDTH * BYUN_HEIGHT];
    for (tile_y, row) in manifest.tile_layout.iter().enumerate() {
        for (tile_x, &tile) in row.iter().enumerate() {
            let Some(tile) = tile else {
                continue;
            };
            let source = &tiles[tile * MD_TILE_BYTES..(tile + 1) * MD_TILE_BYTES];
            for local_y in 0..8 {
                for local_x in 0..8 {
                    let byte = source[local_y * 4 + local_x / 2];
                    let pixel = if local_x.is_multiple_of(2) {
                        byte >> 4
                    } else {
                        byte & 0x0F
                    };
                    let x = tile_x * 8 + local_x;
                    let y = tile_y * 8 + local_y;
                    output[y * BYUN_WIDTH + x] = pixel;
                }
            }
        }
    }
    Ok(output)
}

fn validate_pack_roundtrip(header: &[u8; 6], bank: &[u8], payload: &[u8]) -> Result<(), String> {
    let mut probe = vec![0u8; BYUN_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[BYUN_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != BYUN_VRAM_DESTINATION || decoded.data != payload {
        return Err("demon byun mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn draw_scaled_surface(
    rgba: &mut [u8],
    preview_width: usize,
    x_origin: usize,
    pixels: &[u8],
    palette: &[u16; 16],
    transparent: usize,
) {
    for y in 0..BYUN_HEIGHT {
        for x in 0..BYUN_WIDTH {
            let palette_index = pixels[y * BYUN_WIDTH + x] as usize;
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
                    let preview_x = (x_origin + x) * PREVIEW_SCALE + scale_x;
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

fn summary(build: &DemonByunBuild, checksum: u16) -> DemonByunSummary {
    debug_assert_eq!(build.source_payload.len(), build.payload.len());
    DemonByunSummary {
        source_tiles: BYUN_SOURCE_TILES,
        rewritten_tiles: BYUN_CONSUMER_TILES,
        mapped_tiles: BYUN_MAPPED_TILES,
        cleared_residual_tiles: BYUN_CONSUMER_TILES - BYUN_MAPPED_TILES,
        protected_source_bytes: BYUN_SOURCE_BYTES - BYUN_CONSUMER_TILES * MD_TILE_BYTES,
        consumer_sprites: build.manifest.sprite_consumer.records.len(),
        pack_bytes: build.bank.len(),
        checksum,
    }
}
