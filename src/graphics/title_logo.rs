//! JP-source title-logo pixel fitting and insertion.
//!
//! The approved Korean text-only master is reduced to one committed compact
//! RGBA layer. This module separates its green face into transparent shine
//! apertures, accepts the remaining colors only from the declared JP title
//! palette line, replaces only the four minified pronunciation glyphs with
//! verified repository-font output, copies that layer at one scale into both
//! title states, recovers the ornate main-state decorations from the JP ROM,
//! preserves the animated sub-plane stripe, and emits source-owned mode-1
//! packs.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::bdf::{BdfGlyph, read_verified_bdf_glyphs as read_bdf_glyphs};
use super::pixel::{
    PixelBounds, fit_bounds_within, md_color, read_verified_rgba, reduce_rgba_to_indexed_surface,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const TITLE_TILE_COUNT: usize = 512;
const TITLE_TILE_BYTES: usize = TITLE_TILE_COUNT * MD_TILE_BYTES;
const TITLE_MAP_WIDTH: usize = 64;
const TITLE_MAP_HEIGHT: usize = 64;
const TITLE_MAP_BYTES: usize = TITLE_MAP_WIDTH * TITLE_MAP_HEIGHT * 2;
const TITLE_LOGO_PACK_LOW_OFFSET: usize = 0x24_8000;
const TITLE_LOGO_PACK_HIGH_OFFSET: usize = 0x25_0000;
const TITLE_LOGO_MAP_PACK_OFFSET: usize = 0x25_8000;
const TITLE_TILE_LOW_HEADER_OFFSET: usize = 0x09_2BC8;
const TITLE_TILE_HIGH_HEADER_OFFSET: usize = 0x09_2BCE;
const TITLE_MAIN_MAP_HEADER_OFFSET: usize = 0x09_2BE0;
const TITLE_SUB_MAP_HEADER_OFFSET: usize = 0x09_2BE6;
const TITLE_TILE_LOW_VRAM: u16 = 0x0000;
const TITLE_TILE_HIGH_VRAM: u16 = 0x2000;
const TITLE_MAIN_MAP_VRAM: u16 = 0xC000;
const TITLE_SUB_MAP_VRAM: u16 = 0xE000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleLogoSummary {
    pub regions: usize,
    pub allocated_tiles: usize,
    pub highest_tile: u16,
    pub tile_pack_bytes: usize,
    pub map_pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct TitleLogoManifest {
    text_master_asset: String,
    text_master_sha256: String,
    text_master_width: usize,
    text_master_height: usize,
    text_master_bounds: PixelBounds,
    text_layer: TextLayer,
    background_palette_index: usize,
    palette_line_words: Vec<String>,
    source_packs: Vec<SourcePack>,
    protected_tile_ranges: Vec<ProtectedTileRange>,
    logo_regions: Vec<LogoRegion>,
}

#[derive(Debug, Deserialize)]
struct TextLayer {
    content_box: PixelBounds,
    vertical_alignment: String,
    alpha_threshold: u8,
    allowed_palette_indices: Vec<usize>,
    shine_aperture: ShineAperture,
    pronunciation_strokes: PronunciationStrokes,
}

#[derive(Debug, Deserialize)]
struct ShineAperture {
    minimum_green: u8,
    green_over_red: u8,
    green_over_blue: u8,
    target_alpha_threshold: u8,
    minimum_source_pixels: usize,
    minimum_target_pixels: usize,
}

#[derive(Debug, Deserialize)]
struct PronunciationStrokes {
    font_asset: String,
    font_sha256: String,
    font_family: String,
    font_version: String,
    cell_size_px: usize,
    maximum_glyph_height_px: usize,
    target_top_y: i32,
    target_palette_index: usize,
    replacement_background_palette_index: usize,
    minimum_pixels_per_box: usize,
    boxes: Vec<PronunciationBox>,
}

#[derive(Debug, Deserialize)]
struct PronunciationBox {
    syllable: String,
    bounds: PixelBounds,
    paint_offset_x: i32,
    paint_offset_y: i32,
    source_glyph_clear_rows: Vec<u8>,
    protected_rim_rows: Vec<u8>,
    #[serde(default)]
    column_fit_map: Option<Vec<usize>>,
    #[serde(default)]
    rim_recolor: Option<RimRecolor>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct RimRecolor {
    x: usize,
    palette_indices: Vec<usize>,
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
struct ProtectedTileRange {
    start: usize,
    end_exclusive: usize,
}

#[derive(Debug, Deserialize)]
struct LogoRegion {
    id: String,
    tile_x: usize,
    tile_y: usize,
    tile_width: usize,
    tile_height: usize,
}

#[derive(Debug)]
struct TitleLogoBuild {
    manifest: TitleLogoManifest,
    palette: [u16; 16],
    tiles: Vec<u8>,
    main_map: Vec<u8>,
    headers: [(usize, [u8; 6]); 3],
    banks: [(usize, Vec<u8>); 3],
    allocated_tiles: usize,
    highest_tile: u16,
}

#[derive(Debug)]
struct IndexedTextLayer {
    pixels: Vec<u8>,
    shine_apertures: Vec<bool>,
}

/// Insert both Korean title-logo instances into the cumulative JP-to-KR ROM.
pub fn apply_title_logo(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<TitleLogoSummary, String> {
    let build = build_title_logo(source, assets_dir)?;
    let baseline = output.to_vec();
    let mut changed_ranges = Vec::new();

    for (header_offset, header) in &build.headers {
        apply_expected_write(
            output,
            *header_offset,
            source_range(
                source,
                *header_offset,
                header.len(),
                "title-logo pack header",
            )?,
            header,
            "title-logo pack header",
        )?;
        changed_ranges.push((*header_offset, *header_offset + header.len()));
    }

    for (bank_offset, bank) in &build.banks {
        apply_expected_write(
            output,
            *bank_offset,
            &vec![0xFF; bank.len()],
            bank,
            "title-logo expanded graphics pack",
        )?;
        changed_ranges.push((*bank_offset, *bank_offset + bank.len()));
    }

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after title-logo graphics",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let low = decode_mode1_pack_entry(output, TITLE_TILE_LOW_HEADER_OFFSET)?;
    let high = decode_mode1_pack_entry(output, TITLE_TILE_HIGH_HEADER_OFFSET)?;
    let map = decode_mode1_pack_entry(output, TITLE_MAIN_MAP_HEADER_OFFSET)?;
    if low.vram_destination != TITLE_TILE_LOW_VRAM
        || high.vram_destination != TITLE_TILE_HIGH_VRAM
        || map.vram_destination != TITLE_MAIN_MAP_VRAM
        || low.data != build.tiles[..0x2000]
        || high.data != build.tiles[0x2000..]
        || map.data != build.main_map
    {
        return Err("inserted title-logo packs do not decode to planned payloads".to_string());
    }
    let source_sub_map = checked_source_pack(
        source,
        source_pack(&build.manifest, "title-sub-map")?,
        TITLE_SUB_MAP_HEADER_OFFSET,
        TITLE_SUB_MAP_VRAM,
    )?;
    let output_sub_map = decode_mode1_pack_entry(output, TITLE_SUB_MAP_HEADER_OFFSET)?;
    if output_sub_map.data != source_sub_map {
        return Err("title-logo insertion changed the protected JP sub-plane map".to_string());
    }

    eprintln!("JP graphics GFX-TITLE-LOGO Expected Writes:");
    for (header_offset, header) in &build.headers {
        eprintln!(
            "  0x{header_offset:06X}..0x{:06X}  title-logo pack header ({} bytes)",
            header_offset + header.len(),
            header.len()
        );
    }
    for (bank_offset, bank) in &build.banks {
        eprintln!(
            "  0x{bank_offset:06X}..0x{:06X}  title-logo pack ({} bytes)",
            bank_offset + bank.len(),
            bank.len()
        );
    }
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render the compiled MD main-plane logo as a static PNG.
///
/// Palette index zero remains transparent, so this preview does not prove the
/// animated sub-plane composition or runtime consumption.
pub fn write_title_logo_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<TitleLogoSummary, String> {
    let build = build_title_logo(source, assets_dir)?;
    let mut rgba = vec![0u8; 512 * 512 * 4];
    for tile_y in 0..TITLE_MAP_HEIGHT {
        for tile_x in 0..TITLE_MAP_WIDTH {
            let map_offset = (tile_y * TITLE_MAP_WIDTH + tile_x) * 2;
            let word =
                u16::from_be_bytes([build.main_map[map_offset], build.main_map[map_offset + 1]]);
            let tile = (word & 0x07FF) as usize;
            let hflip = word & 0x0800 != 0;
            let vflip = word & 0x1000 != 0;
            let palette_line = ((word >> 13) & 0x03) as usize;
            if palette_line != 0 {
                return Err(format!(
                    "title main-plane preview encountered undeclared palette line {palette_line}"
                ));
            }
            for local_y in 0..8usize {
                for local_x in 0..8usize {
                    let source_x = if hflip { 7 - local_x } else { local_x };
                    let source_y = if vflip { 7 - local_y } else { local_y };
                    let color_index = tile_pixel(&build.tiles, tile, source_x, source_y)?;
                    let x = tile_x * 8 + local_x;
                    let y = tile_y * 8 + local_y;
                    let offset = (y * 512 + x) * 4;
                    if color_index == 0 {
                        rgba[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
                    } else {
                        let [r, g, b] = md_color(build.palette[color_index as usize]);
                        rgba[offset..offset + 4].copy_from_slice(&[r, g, b, 255]);
                    }
                }
            }
        }
    }
    write_rgba_png(output_path, 512, 512, &rgba)?;
    Ok(summary(&build, 0))
}

fn build_title_logo(source: &[u8], assets_dir: &Path) -> Result<TitleLogoBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    let palette = parse_palette(&manifest.palette_line_words)?;
    let text_layer = build_text_layer(assets_dir, &manifest, &palette)?;

    let low = checked_source_pack(
        source,
        source_pack(&manifest, "title-patterns-low")?,
        TITLE_TILE_LOW_HEADER_OFFSET,
        TITLE_TILE_LOW_VRAM,
    )?;
    let high = checked_source_pack(
        source,
        source_pack(&manifest, "title-patterns-high")?,
        TITLE_TILE_HIGH_HEADER_OFFSET,
        TITLE_TILE_HIGH_VRAM,
    )?;
    let main_map = checked_source_pack(
        source,
        source_pack(&manifest, "title-main-map")?,
        TITLE_MAIN_MAP_HEADER_OFFSET,
        TITLE_MAIN_MAP_VRAM,
    )?;
    checked_source_pack(
        source,
        source_pack(&manifest, "title-sub-map")?,
        TITLE_SUB_MAP_HEADER_OFFSET,
        TITLE_SUB_MAP_VRAM,
    )?;
    if main_map.len() != TITLE_MAP_BYTES {
        return Err(format!(
            "title main map has {} bytes, expected {TITLE_MAP_BYTES}",
            main_map.len()
        ));
    }

    validate_pack_roundtrip(TITLE_LOGO_PACK_LOW_OFFSET, TITLE_TILE_LOW_VRAM, &low)?;
    validate_pack_roundtrip(TITLE_LOGO_PACK_HIGH_OFFSET, TITLE_TILE_HIGH_VRAM, &high)?;
    validate_pack_roundtrip(TITLE_LOGO_MAP_PACK_OFFSET, TITLE_MAIN_MAP_VRAM, &main_map)?;

    let mut source_tiles = vec![0u8; TITLE_TILE_BYTES];
    source_tiles[..low.len()].copy_from_slice(&low);
    source_tiles[0x2000..0x2000 + high.len()].copy_from_slice(&high);
    let (tiles, main_map, allocated_tiles, highest_tile) =
        compile_logo_tiles(&source_tiles, &main_map, &text_layer, &manifest)?;

    for range in &manifest.protected_tile_ranges {
        let start = range.start * MD_TILE_BYTES;
        let end = range.end_exclusive * MD_TILE_BYTES;
        if tiles.get(start..end) != source_tiles.get(start..end) {
            return Err(format!(
                "title logo changed protected tiles {}..{}",
                range.start, range.end_exclusive
            ));
        }
    }

    let encoded_low = encode_locked_mode1_pack(
        TITLE_LOGO_PACK_LOW_OFFSET,
        TITLE_TILE_LOW_VRAM,
        &tiles[..0x2000],
    )?;
    let encoded_high = encode_locked_mode1_pack(
        TITLE_LOGO_PACK_HIGH_OFFSET,
        TITLE_TILE_HIGH_VRAM,
        &tiles[0x2000..],
    )?;
    let encoded_map =
        encode_locked_mode1_pack(TITLE_LOGO_MAP_PACK_OFFSET, TITLE_MAIN_MAP_VRAM, &main_map)?;

    Ok(TitleLogoBuild {
        manifest,
        palette,
        tiles,
        main_map,
        headers: [
            (TITLE_TILE_LOW_HEADER_OFFSET, encoded_low.header),
            (TITLE_TILE_HIGH_HEADER_OFFSET, encoded_high.header),
            (TITLE_MAIN_MAP_HEADER_OFFSET, encoded_map.header),
        ],
        banks: [
            (TITLE_LOGO_PACK_LOW_OFFSET, encoded_low.bank),
            (TITLE_LOGO_PACK_HIGH_OFFSET, encoded_high.bank),
            (TITLE_LOGO_MAP_PACK_OFFSET, encoded_map.bank),
        ],
        allocated_tiles,
        highest_tile,
    })
}

fn compile_logo_tiles(
    source_tiles: &[u8],
    source_map: &[u8],
    text_layer: &IndexedTextLayer,
    manifest: &TitleLogoManifest,
) -> Result<(Vec<u8>, Vec<u8>, usize, u16), String> {
    if source_tiles.len() != TITLE_TILE_BYTES || source_map.len() != TITLE_MAP_BYTES {
        return Err("title-logo compiler received an invalid source surface".to_string());
    }
    let composed = compose_logo_indices(source_tiles, source_map, text_layer, manifest)?;
    let protected = protected_tiles(manifest)?;
    let mut free_tiles: Vec<_> = (1usize..TITLE_TILE_COUNT)
        .filter(|tile| !protected.contains(tile))
        .collect();
    free_tiles.sort_unstable();

    let mut tiles = source_tiles.to_vec();
    let mut map = source_map.to_vec();
    let mut variants: BTreeMap<Vec<u8>, (usize, u16)> = BTreeMap::new();
    insert_tile_variants(&mut variants, tile_indices(&tiles, 0)?, 0);
    for &tile in &protected {
        insert_tile_variants(&mut variants, tile_indices(&tiles, tile)?, tile);
    }

    let mut free_cursor = 0usize;
    let mut allocated = BTreeSet::new();
    for region in &manifest.logo_regions {
        if region.tile_x + region.tile_width > TITLE_MAP_WIDTH
            || region.tile_y + region.tile_height > TITLE_MAP_HEIGHT
        {
            return Err(format!("{} lies outside the title map", region.id));
        }
        for tile_y in region.tile_y..region.tile_y + region.tile_height {
            for tile_x in region.tile_x..region.tile_x + region.tile_width {
                let indices = composed_tile(&composed, tile_x, tile_y)?;
                let (tile, flip_flags) = if let Some(&(tile, flags)) = variants.get(&indices) {
                    (tile, flags)
                } else {
                    let tile = *free_tiles.get(free_cursor).ok_or_else(|| {
                        format!(
                            "{} exceeds the {}-tile title pattern budget",
                            region.id, TITLE_TILE_COUNT
                        )
                    })?;
                    free_cursor += 1;
                    let encoded = encode_tile(&indices)?;
                    let offset = tile * MD_TILE_BYTES;
                    tiles[offset..offset + MD_TILE_BYTES].copy_from_slice(&encoded);
                    insert_tile_variants(&mut variants, indices, tile);
                    allocated.insert(tile);
                    (tile, 0)
                };
                let map_offset = (tile_y * TITLE_MAP_WIDTH + tile_x) * 2;
                let original = u16::from_be_bytes([map[map_offset], map[map_offset + 1]]);
                let replacement = (original & 0x8000) | flip_flags | tile as u16;
                map[map_offset..map_offset + 2].copy_from_slice(&replacement.to_be_bytes());
            }
        }
    }

    for tile_y in 0..TITLE_MAP_HEIGHT {
        for tile_x in 0..TITLE_MAP_WIDTH {
            if manifest.logo_regions.iter().any(|region| {
                tile_x >= region.tile_x
                    && tile_x < region.tile_x + region.tile_width
                    && tile_y >= region.tile_y
                    && tile_y < region.tile_y + region.tile_height
            }) {
                continue;
            }
            let offset = (tile_y * TITLE_MAP_WIDTH + tile_x) * 2;
            if map[offset..offset + 2] != source_map[offset..offset + 2] {
                return Err(format!(
                    "title-logo compiler changed map cell ({tile_x},{tile_y}) outside declared regions"
                ));
            }
        }
    }

    let highest = allocated.iter().next_back().copied().unwrap_or(0) as u16;
    Ok((tiles, map, allocated.len(), highest))
}

/// Compose the JP-native two-stage title presentation.
///
/// In the source map, the lower compact region is exactly the central title
/// lettering from the upper main region. The upper region adds ornaments
/// around that shared lettering. Comparing the two aligned source regions
/// therefore recovers a decoration-only main layer without importing an EN
/// graphic or maintaining a hand-authored mask.
///
/// The compiler reduces the committed text-only master once. Copying those
/// exact indexed pixels to both aligned positions makes scale and extrusion
/// invariant across the transition.
fn compose_logo_indices(
    source_tiles: &[u8],
    source_map: &[u8],
    text_layer: &IndexedTextLayer,
    manifest: &TitleLogoManifest,
) -> Result<Vec<u8>, String> {
    let main = logo_region(manifest, "GFX-TITLE-LOGO-MAIN")?;
    let compact = logo_region(manifest, "GFX-TITLE-LOGO-COMPACT")?;
    if compact.tile_x < main.tile_x
        || compact.tile_x + compact.tile_width > main.tile_x + main.tile_width
        || main.tile_y + compact.tile_height > main.tile_y + main.tile_height
    {
        return Err(
            "title compact region cannot align inside the source main logo region".to_string(),
        );
    }
    let mut composed = vec![0u8; TITLE_MAP_WIDTH * 8 * TITLE_MAP_HEIGHT * 8];
    let main_text_x = compact.tile_x * 8;
    let main_text_y = main.tile_y * 8;
    let compact_x = compact.tile_x * 8;
    let compact_y = compact.tile_y * 8;
    let text_width = compact.tile_width * 8;
    let text_height = compact.tile_height * 8;
    if manifest.background_palette_index >= 16 {
        return Err("title background palette index is invalid".to_string());
    }
    if text_layer.pixels.len() != text_width * text_height
        || text_layer.shine_apertures.len() != text_width * text_height
    {
        return Err(format!(
            "title text layer has {}/{} pixels and apertures, expected {}",
            text_layer.pixels.len(),
            text_layer.shine_apertures.len(),
            text_width * text_height
        ));
    }

    for pixel_y in main.tile_y * 8..(main.tile_y + main.tile_height) * 8 {
        for pixel_x in main.tile_x * 8..(main.tile_x + main.tile_width) * 8 {
            let source_main = source_surface_pixel(source_tiles, source_map, pixel_x, pixel_y)?;
            let in_text_rect = pixel_x >= main_text_x
                && pixel_x < main_text_x + text_width
                && pixel_y >= main_text_y
                && pixel_y < main_text_y + text_height;
            let decoration = if in_text_rect {
                let compact_pixel_x = compact_x + pixel_x - main_text_x;
                let compact_pixel_y = compact_y + pixel_y - main_text_y;
                let source_compact = source_surface_pixel(
                    source_tiles,
                    source_map,
                    compact_pixel_x,
                    compact_pixel_y,
                )?;
                if source_main != source_compact {
                    source_main
                } else {
                    manifest.background_palette_index as u8
                }
            } else {
                source_main
            };
            composed[pixel_y * TITLE_MAP_WIDTH * 8 + pixel_x] = decoration;
        }
    }

    for local_y in 0..text_height {
        for local_x in 0..text_width {
            let local_offset = local_y * text_width + local_x;
            let text = text_layer.pixels[local_offset];
            let shine_aperture = text_layer.shine_apertures[local_offset];
            let compact_offset = (compact_y + local_y) * TITLE_MAP_WIDTH * 8 + compact_x + local_x;
            composed[compact_offset] = if shine_aperture {
                0
            } else if text == 0 {
                manifest.background_palette_index as u8
            } else {
                text
            };
            let main_offset = (main_text_y + local_y) * TITLE_MAP_WIDTH * 8 + main_text_x + local_x;
            if shine_aperture {
                composed[main_offset] = 0;
            } else if text != 0 {
                composed[main_offset] = text;
            }
        }
    }

    Ok(composed)
}

fn source_surface_pixel(
    tiles: &[u8],
    map: &[u8],
    pixel_x: usize,
    pixel_y: usize,
) -> Result<u8, String> {
    let tile_x = pixel_x / 8;
    let tile_y = pixel_y / 8;
    let map_offset = (tile_y * TITLE_MAP_WIDTH + tile_x) * 2;
    let word = u16::from_be_bytes([
        *map.get(map_offset)
            .ok_or_else(|| "title source map is truncated".to_string())?,
        *map.get(map_offset + 1)
            .ok_or_else(|| "title source map is truncated".to_string())?,
    ]);
    let palette_line = (word >> 13) & 0x03;
    if palette_line != 0 {
        return Err(format!(
            "title source composition encountered undeclared palette line {palette_line}"
        ));
    }
    let local_x = pixel_x % 8;
    let local_y = pixel_y % 8;
    let source_x = if word & 0x0800 != 0 {
        7 - local_x
    } else {
        local_x
    };
    let source_y = if word & 0x1000 != 0 {
        7 - local_y
    } else {
        local_y
    };
    tile_pixel(tiles, (word & 0x07FF) as usize, source_x, source_y)
}

fn composed_tile(composed: &[u8], tile_x: usize, tile_y: usize) -> Result<Vec<u8>, String> {
    let mut indices = Vec::with_capacity(64);
    for local_y in 0..8 {
        for local_x in 0..8 {
            let pixel_x = tile_x * 8 + local_x;
            let pixel_y = tile_y * 8 + local_y;
            indices.push(
                *composed
                    .get(pixel_y * TITLE_MAP_WIDTH * 8 + pixel_x)
                    .ok_or_else(|| "composed title surface is truncated".to_string())?,
            );
        }
    }
    Ok(indices)
}

fn insert_tile_variants(
    variants: &mut BTreeMap<Vec<u8>, (usize, u16)>,
    tile: Vec<u8>,
    index: usize,
) {
    variants.entry(tile.clone()).or_insert((index, 0));
    variants
        .entry(flip_tile(&tile, true, false))
        .or_insert((index, 0x0800));
    variants
        .entry(flip_tile(&tile, false, true))
        .or_insert((index, 0x1000));
    variants
        .entry(flip_tile(&tile, true, true))
        .or_insert((index, 0x1800));
}

fn flip_tile(tile: &[u8], horizontal: bool, vertical: bool) -> Vec<u8> {
    let mut flipped = vec![0u8; 64];
    for y in 0..8usize {
        for x in 0..8usize {
            let source_x = if horizontal { 7 - x } else { x };
            let source_y = if vertical { 7 - y } else { y };
            flipped[y * 8 + x] = tile[source_y * 8 + source_x];
        }
    }
    flipped
}

fn tile_indices(tiles: &[u8], tile: usize) -> Result<Vec<u8>, String> {
    let start = tile
        .checked_mul(MD_TILE_BYTES)
        .ok_or_else(|| "title tile offset overflow".to_string())?;
    let bytes = tiles
        .get(start..start + MD_TILE_BYTES)
        .ok_or_else(|| format!("title tile {tile} is outside source payload"))?;
    let mut indices = Vec::with_capacity(64);
    for &byte in bytes {
        indices.push(byte >> 4);
        indices.push(byte & 0x0F);
    }
    Ok(indices)
}

fn encode_tile(indices: &[u8]) -> Result<[u8; MD_TILE_BYTES], String> {
    if indices.len() != 64 || indices.iter().any(|&index| index > 0x0F) {
        return Err("invalid indexed title tile".to_string());
    }
    let mut output = [0u8; MD_TILE_BYTES];
    for (offset, pair) in indices.chunks_exact(2).enumerate() {
        output[offset] = (pair[0] << 4) | pair[1];
    }
    Ok(output)
}

fn tile_pixel(tiles: &[u8], tile: usize, x: usize, y: usize) -> Result<u8, String> {
    let offset = tile
        .checked_mul(MD_TILE_BYTES)
        .and_then(|base| base.checked_add(y * 4 + x / 2))
        .ok_or_else(|| "title preview tile offset overflow".to_string())?;
    let byte = *tiles
        .get(offset)
        .ok_or_else(|| format!("title preview tile {tile} is outside payload"))?;
    Ok(if x.is_multiple_of(2) {
        byte >> 4
    } else {
        byte & 0x0F
    })
}

fn read_manifest(assets_dir: &Path) -> Result<TitleLogoManifest, String> {
    let path = assets_dir.join("graphics_text/title_logo.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read title-logo source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid title-logo source {}: {error}", path.display()))
}

fn build_text_layer(
    assets_dir: &Path,
    manifest: &TitleLogoManifest,
    palette: &[u16; 16],
) -> Result<IndexedTextLayer, String> {
    let compact = logo_region(manifest, "GFX-TITLE-LOGO-COMPACT")?;
    let width = compact.tile_width * 8;
    let height = compact.tile_height * 8;
    if manifest.text_layer.content_box.x + manifest.text_layer.content_box.width > width
        || manifest.text_layer.content_box.y + manifest.text_layer.content_box.height > height
        || manifest
            .text_layer
            .allowed_palette_indices
            .iter()
            .any(|&index| index == 0 || index >= 16)
    {
        return Err("title text-layer contract is invalid".to_string());
    }
    let master = read_verified_rgba(
        assets_dir,
        &manifest.text_master_asset,
        &manifest.text_master_sha256,
        manifest.text_master_width,
        manifest.text_master_height,
        manifest.text_master_bounds,
        "title text-only master",
    )?;
    let resolved_content_box = resolve_text_content_box(manifest)?;
    let (opaque_master, aperture_master, source_aperture_pixels) =
        split_shine_aperture(&master, &manifest.text_layer.shine_aperture)?;
    if source_aperture_pixels < manifest.text_layer.shine_aperture.minimum_source_pixels {
        return Err(format!(
            "title shine aperture has {source_aperture_pixels} source pixels, expected at least {}",
            manifest.text_layer.shine_aperture.minimum_source_pixels
        ));
    }
    let mut pixels = reduce_rgba_to_indexed_surface(
        &opaque_master,
        manifest.text_master_width,
        manifest.text_master_height,
        manifest.text_master_bounds,
        width,
        height,
        resolved_content_box,
        manifest.text_layer.alpha_threshold,
        0,
        palette,
        &manifest.text_layer.allowed_palette_indices,
        "title text-only layer",
    )?;
    let aperture_indices = reduce_rgba_to_indexed_surface(
        &aperture_master,
        manifest.text_master_width,
        manifest.text_master_height,
        manifest.text_master_bounds,
        width,
        height,
        resolved_content_box,
        manifest.text_layer.shine_aperture.target_alpha_threshold,
        0,
        palette,
        &[1],
        "title shine-aperture mask",
    )?;
    let mut shine_apertures: Vec<_> = aperture_indices
        .into_iter()
        .map(|index| index != 0)
        .collect();
    let target_aperture_pixels = shine_apertures.iter().filter(|&&pixel| pixel).count();
    if target_aperture_pixels < manifest.text_layer.shine_aperture.minimum_target_pixels {
        return Err(format!(
            "title shine aperture has {target_aperture_pixels} reduced pixels, expected at least {}",
            manifest.text_layer.shine_aperture.minimum_target_pixels
        ));
    }
    for (pixel, &aperture) in pixels.iter_mut().zip(&shine_apertures) {
        if aperture {
            *pixel = 0;
        }
    }
    apply_pronunciation_strokes(
        assets_dir,
        &mut pixels,
        &mut shine_apertures,
        width,
        height,
        &manifest.text_layer.pronunciation_strokes,
    )?;
    Ok(IndexedTextLayer {
        pixels,
        shine_apertures,
    })
}

fn split_shine_aperture(
    master: &[u8],
    spec: &ShineAperture,
) -> Result<(Vec<u8>, Vec<u8>, usize), String> {
    if !master.len().is_multiple_of(4)
        || spec.target_alpha_threshold == 0
        || spec.target_alpha_threshold == u8::MAX
    {
        return Err("title shine-aperture contract is invalid".to_string());
    }
    let mut opaque = master.to_vec();
    let mut aperture = master.to_vec();
    let mut aperture_pixels = 0usize;
    for (opaque_pixel, aperture_pixel) in
        opaque.chunks_exact_mut(4).zip(aperture.chunks_exact_mut(4))
    {
        let red = opaque_pixel[0];
        let green = opaque_pixel[1];
        let blue = opaque_pixel[2];
        let alpha = opaque_pixel[3];
        let green_face = alpha != 0
            && green >= spec.minimum_green
            && green.saturating_sub(red) >= spec.green_over_red
            && green.saturating_sub(blue) >= spec.green_over_blue;
        if green_face {
            aperture_pixels += 1;
            opaque_pixel[3] = 0;
        } else {
            aperture_pixel[3] = 0;
        }
    }
    Ok((opaque, aperture, aperture_pixels))
}

fn apply_pronunciation_strokes(
    assets_dir: &Path,
    pixels: &mut [u8],
    shine_apertures: &mut [bool],
    width: usize,
    height: usize,
    spec: &PronunciationStrokes,
) -> Result<(), String> {
    if pixels.len() != width * height
        || shine_apertures.len() != width * height
        || spec.boxes.len() != 4
        || spec.cell_size_px != 8
        || spec.maximum_glyph_height_px == 0
        || spec.maximum_glyph_height_px > spec.cell_size_px
        || spec.target_top_y < 0
        || spec.target_top_y as usize >= spec.cell_size_px
        || spec.target_palette_index == 0
        || spec.target_palette_index >= 16
        || spec.replacement_background_palette_index >= 16
        || spec.target_palette_index == spec.replacement_background_palette_index
    {
        return Err("title pronunciation-stroke contract is invalid".to_string());
    }
    let required = pronunciation_characters(spec)?;
    let glyphs = read_verified_bdf_glyphs(assets_dir, spec, &required)?;
    for pronunciation in &spec.boxes {
        let bounds = pronunciation.bounds;
        if bounds.x + bounds.width > width
            || bounds.y + bounds.height > height
            || bounds.width != spec.cell_size_px
            || bounds.height != spec.cell_size_px
            || pronunciation.source_glyph_clear_rows.len() != bounds.height
            || pronunciation.protected_rim_rows.len() != bounds.height
            || pronunciation
                .source_glyph_clear_rows
                .iter()
                .zip(&pronunciation.protected_rim_rows)
                .any(|(&clear, &protected)| clear & protected != 0)
        {
            return Err(format!(
                "title pronunciation box {:?}, clear mask, or rim mask is invalid",
                pronunciation.syllable
            ));
        }
        if let Some(recolor) = &pronunciation.rim_recolor
            && (recolor.x >= bounds.width
                || recolor.palette_indices.len() != bounds.height
                || recolor
                    .palette_indices
                    .iter()
                    .any(|&palette_index| palette_index >= 16))
        {
            return Err(format!(
                "title pronunciation {:?} rim recolor is invalid",
                pronunciation.syllable
            ));
        }
        let mut characters = pronunciation.syllable.chars();
        let character = characters.next().ok_or_else(|| {
            format!(
                "title pronunciation box {:?} has no syllable",
                pronunciation.syllable
            )
        })?;
        if characters.next().is_some() {
            return Err(format!(
                "title pronunciation box {:?} must contain exactly one syllable",
                pronunciation.syllable
            ));
        }
        let cell_origin_x = bounds.x + (bounds.width - spec.cell_size_px) / 2;
        let cell_origin_y = bounds.y + (bounds.height - spec.cell_size_px) / 2;
        let paint_origin_x = cell_origin_x as i32 + pronunciation.paint_offset_x;
        let paint_origin_y = cell_origin_y as i32 + pronunciation.paint_offset_y;
        if paint_origin_x < 0
            || paint_origin_y < 0
            || paint_origin_x + spec.cell_size_px as i32 > width as i32
            || paint_origin_y + spec.cell_size_px as i32 > height as i32
        {
            return Err(format!(
                "title pronunciation {:?} paint cell leaves the text layer",
                pronunciation.syllable
            ));
        }
        let mut protected_rim = Vec::new();
        for (local_y, &row) in pronunciation.protected_rim_rows.iter().enumerate() {
            for local_x in 0..bounds.width {
                if row & (1 << (7 - local_x)) == 0 {
                    continue;
                }
                let offset = (bounds.y + local_y) * width + bounds.x + local_x;
                if pixels[offset] as usize != spec.target_palette_index {
                    return Err(format!(
                        "title pronunciation {:?} rim mask selects palette index {} instead of the declared main-gold rim at ({}, {})",
                        pronunciation.syllable,
                        pixels[offset],
                        bounds.x + local_x,
                        bounds.y + local_y
                    ));
                }
                protected_rim.push((offset, pixels[offset], shine_apertures[offset]));
            }
        }
        for (local_y, &row) in pronunciation.source_glyph_clear_rows.iter().enumerate() {
            for local_x in 0..bounds.width {
                if row & (1 << (7 - local_x)) == 0 {
                    continue;
                }
                let offset = (bounds.y + local_y) * width + bounds.x + local_x;
                pixels[offset] = spec.replacement_background_palette_index as u8;
                shine_apertures[offset] = false;
            }
        }
        let source_glyph = glyphs.get(&character).ok_or_else(|| {
            format!(
                "title pronunciation BDF has no glyph for {:?}",
                pronunciation.syllable
            )
        })?;
        let glyph = fit_glyph_columns(
            *source_glyph,
            pronunciation.column_fit_map.as_deref(),
            spec.cell_size_px,
            &pronunciation.syllable,
        )?;
        let mut rendered = 0usize;
        for (y, &row) in glyph.iter().enumerate() {
            for x in 0..spec.cell_size_px {
                if row & (1 << (7 - x)) == 0 {
                    continue;
                }
                let target_x = (paint_origin_x + x as i32) as usize;
                let target_y = (paint_origin_y + y as i32) as usize;
                let offset = target_y * width + target_x;
                pixels[offset] = spec.target_palette_index as u8;
                shine_apertures[offset] = false;
                rendered += 1;
            }
        }
        if rendered < spec.minimum_pixels_per_box {
            return Err(format!(
                "title pronunciation {:?} rendered only {rendered} pixels, expected at least {}",
                pronunciation.syllable, spec.minimum_pixels_per_box
            ));
        }
        for (offset, pixel, aperture) in protected_rim {
            pixels[offset] = pixel;
            shine_apertures[offset] = aperture;
        }
        if let Some(recolor) = &pronunciation.rim_recolor {
            for (local_y, &palette_index) in recolor.palette_indices.iter().enumerate() {
                let offset = (bounds.y + local_y) * width + bounds.x + recolor.x;
                pixels[offset] = palette_index as u8;
                shine_apertures[offset] = false;
            }
        }
    }
    Ok(())
}

fn fit_glyph_columns(
    source: [u8; 8],
    column_fit_map: Option<&[usize]>,
    cell_size_px: usize,
    syllable: &str,
) -> Result<[u8; 8], String> {
    let Some(column_fit_map) = column_fit_map else {
        return Ok(source);
    };
    if column_fit_map.len() != cell_size_px
        || column_fit_map
            .iter()
            .any(|&target_x| target_x >= cell_size_px)
        || column_fit_map.windows(2).any(|pair| pair[0] > pair[1])
    {
        return Err(format!(
            "title pronunciation {syllable:?} column-fit map is invalid"
        ));
    }

    let mut fitted = [0u8; 8];
    for (y, &row) in source.iter().enumerate() {
        for (source_x, &target_x) in column_fit_map.iter().enumerate() {
            if row & (1 << (7 - source_x)) == 0 {
                continue;
            }
            fitted[y] |= 1 << (7 - target_x);
        }
    }
    Ok(fitted)
}

fn pronunciation_characters(spec: &PronunciationStrokes) -> Result<BTreeSet<char>, String> {
    let mut required = BTreeSet::new();
    for pronunciation in &spec.boxes {
        let mut characters = pronunciation.syllable.chars();
        let character = characters.next().ok_or_else(|| {
            format!(
                "title pronunciation box {:?} has no syllable",
                pronunciation.syllable
            )
        })?;
        if characters.next().is_some() {
            return Err(format!(
                "title pronunciation box {:?} must contain exactly one syllable",
                pronunciation.syllable
            ));
        }
        if !required.insert(character) {
            return Err(format!(
                "title pronunciation box duplicates {:?}",
                pronunciation.syllable
            ));
        }
    }
    Ok(required)
}

fn read_verified_bdf_glyphs(
    assets_dir: &Path,
    spec: &PronunciationStrokes,
    required: &BTreeSet<char>,
) -> Result<BTreeMap<char, [u8; 8]>, String> {
    read_bdf_glyphs(
        assets_dir,
        &spec.font_asset,
        &spec.font_sha256,
        &spec.font_family,
        &spec.font_version,
        required,
        "title pronunciation",
    )?
    .into_iter()
    .map(|(character, glyph)| {
        render_bdf_glyph(character, &glyph, spec).map(|rows| (character, rows))
    })
    .collect()
}

fn render_bdf_glyph(
    character: char,
    glyph: &BdfGlyph,
    spec: &PronunciationStrokes,
) -> Result<[u8; 8], String> {
    if glyph.device_width != [spec.cell_size_px as i32, 0] {
        return Err(format!(
            "title pronunciation BDF U+{:04X} has unsupported DWIDTH {:?}",
            u32::from(character),
            glyph.device_width
        ));
    }
    if glyph.width > spec.cell_size_px
        || glyph.height > spec.maximum_glyph_height_px
        || glyph.x_offset != 0
        || glyph.pixels.len() != glyph.width * glyph.height
    {
        return Err(format!(
            "title pronunciation BDF U+{:04X} has unsupported BBX or bitmap height",
            u32::from(character)
        ));
    }
    let start_row = spec.target_top_y + 1 - glyph.y_offset - glyph.height as i32;
    if start_row < 0 || start_row + glyph.height as i32 > spec.cell_size_px as i32 {
        return Err(format!(
            "title pronunciation BDF U+{:04X} falls outside the target cell",
            u32::from(character)
        ));
    }
    let mut rows = [0u8; 8];
    for y in 0..glyph.height {
        for x in 0..glyph.width {
            if glyph.pixels[y * glyph.width + x] {
                rows[start_row as usize + y] |= 1 << (7 - x);
            }
        }
    }
    Ok(rows)
}

fn resolve_text_content_box(manifest: &TitleLogoManifest) -> Result<PixelBounds, String> {
    if manifest.text_layer.vertical_alignment != "bottom" {
        return Err(format!(
            "unsupported title text vertical alignment {:?}",
            manifest.text_layer.vertical_alignment
        ));
    }
    let content_box = manifest.text_layer.content_box;
    let (_, fit_height) = fit_bounds_within(
        manifest.text_master_bounds,
        content_box,
        "title text-only layer",
    )?;
    Ok(PixelBounds {
        x: content_box.x,
        y: content_box.y + content_box.height - fit_height,
        width: content_box.width,
        height: fit_height,
    })
}

fn parse_palette(words: &[String]) -> Result<[u16; 16], String> {
    if words.len() != 16 {
        return Err(format!(
            "title-logo palette declares {} colors, expected 16",
            words.len()
        ));
    }
    let mut palette = [0u16; 16];
    for (index, word) in words.iter().enumerate() {
        let value = parse_hex(word)?;
        if value > u16::MAX as usize || value & !0x0EEE != 0 {
            return Err(format!("invalid Mega Drive palette word {word}"));
        }
        palette[index] = value as u16;
    }
    Ok(palette)
}

fn source_pack<'a>(manifest: &'a TitleLogoManifest, id: &str) -> Result<&'a SourcePack, String> {
    manifest
        .source_packs
        .iter()
        .find(|pack| pack.id == id)
        .ok_or_else(|| format!("title-logo manifest has no source pack {id}"))
}

fn logo_region<'a>(manifest: &'a TitleLogoManifest, id: &str) -> Result<&'a LogoRegion, String> {
    manifest
        .logo_regions
        .iter()
        .find(|region| region.id == id)
        .ok_or_else(|| format!("title-logo manifest has no logo region {id}"))
}

fn checked_source_pack(
    source: &[u8],
    declaration: &SourcePack,
    expected_header: usize,
    expected_vram: u16,
) -> Result<Vec<u8>, String> {
    let header = parse_hex(&declaration.header_offset)?;
    let vram = parse_hex(&declaration.vram_destination)?;
    if header != expected_header || vram != expected_vram as usize {
        return Err(format!(
            "{} source-pack declaration drifted from header 0x{expected_header:06X}, VRAM 0x{expected_vram:04X}",
            declaration.id
        ));
    }
    let decoded = decode_mode1_pack_entry(source, header)?;
    if decoded.vram_destination != expected_vram || decoded.data.len() != declaration.decoded_bytes
    {
        return Err(format!(
            "{} decoded as {} bytes at VRAM 0x{:04X}",
            declaration.id,
            decoded.data.len(),
            decoded.vram_destination
        ));
    }
    let actual_hash = sha256_hex(&decoded.data);
    if actual_hash != declaration.decoded_sha256 {
        return Err(format!(
            "{} decoded SHA-256 mismatch: expected {}, got {actual_hash}",
            declaration.id, declaration.decoded_sha256
        ));
    }
    Ok(decoded.data)
}

fn protected_tiles(manifest: &TitleLogoManifest) -> Result<BTreeSet<usize>, String> {
    let mut protected = BTreeSet::new();
    for range in &manifest.protected_tile_ranges {
        if range.start >= range.end_exclusive || range.end_exclusive > TITLE_TILE_COUNT {
            return Err(format!(
                "invalid protected title tile range {}..{}",
                range.start, range.end_exclusive
            ));
        }
        for tile in range.start..range.end_exclusive {
            if !protected.insert(tile) {
                return Err(format!("title tile {tile} is protected more than once"));
            }
        }
    }
    Ok(protected)
}

fn validate_pack_roundtrip(base: usize, vram: u16, data: &[u8]) -> Result<(), String> {
    let encoded = encode_locked_mode1_pack(base, vram, data)?;
    let mut probe = vec![0u8; base + encoded.bank.len()];
    probe[0x100..0x106].copy_from_slice(&encoded.header);
    probe[base..base + encoded.bank.len()].copy_from_slice(&encoded.bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != vram || decoded.data != data {
        return Err(format!(
            "title-logo mode-1 no-op round-trip failed at VRAM 0x{vram:04X}"
        ));
    }
    Ok(())
}

fn write_rgba_png(output_path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create title-logo preview directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let file = fs::File::create(output_path).map_err(|error| {
        format!(
            "failed to create title-logo preview {}: {error}",
            output_path.display()
        )
    })?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to write title-logo PNG header: {error}"))?;
    writer
        .write_image_data(rgba)
        .map_err(|error| format!("failed to write title-logo PNG pixels: {error}"))
}

fn summary(build: &TitleLogoBuild, checksum: u16) -> TitleLogoSummary {
    TitleLogoSummary {
        regions: build.manifest.logo_regions.len(),
        allocated_tiles: build.allocated_tiles,
        highest_tile: build.highest_tile,
        tile_pack_bytes: build.banks[0].1.len() + build.banks[1].1.len(),
        map_pack_bytes: build.banks[2].1.len(),
        checksum,
    }
}
