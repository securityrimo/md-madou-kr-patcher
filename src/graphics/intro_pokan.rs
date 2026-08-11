//! JP-source automatic-prologue `ぽかんっ` -> `콩!` graphics compiler.
//!
//! The JP map and pattern transfers are decoded into their 512x136 indexed
//! surface. Only the declared JP-letter palette roles are cleared. The yellow
//! burst remains in the JP-derived base beneath the Korean composite, and the
//! committed RGBA master is reduced into the original effect envelope before
//! both transfers are rebuilt.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::pixel::{
    PixelBounds, fit_bounds_within, md_color, nearest_palette_index, parse_md_palette,
    read_verified_rgba, write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_bytes, encode_locked_mode1_pack, parse_hex,
    sha256_hex, source_range, validate_only_ranges_changed,
};

const POKAN_MAP_HEADER_OFFSET: usize = 0x08_D33A;
const POKAN_PATTERN_HEADER_OFFSET: usize = 0x08_D340;
const POKAN_MAP_VRAM: u16 = 0xE000;
const POKAN_PATTERN_VRAM: u16 = 0x2000;
const POKAN_MAP_DECODED_BYTES: usize = 2_110;
const POKAN_PATTERN_DECODED_BYTES: usize = 3_392;
const POKAN_MAP_BANK_OFFSET: usize = 0x27_0000;
const POKAN_PATTERN_BANK_OFFSET: usize = 0x27_8000;
const PREVIEW_SCALE: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntroPokanSummary {
    pub source_pattern_tiles: usize,
    pub output_pattern_tiles: usize,
    pub map_bytes: usize,
    pub protected_pixels: usize,
    pub map_pack_bytes: usize,
    pub pattern_pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct IntroPokanManifest {
    schema_version: u32,
    asset_group_id: String,
    master_asset: String,
    master_sha256: String,
    master_width: usize,
    master_height: usize,
    master_alpha_bounds: PixelBounds,
    output_surface: OutputSurface,
    tilemap: TilemapPlan,
    palette_line_words: Vec<String>,
    source_palette_roles: PaletteRoles,
    allowed_composite_palette_indices: Vec<usize>,
    source_packs: Vec<SourcePack>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
    tile_columns: usize,
    tile_rows: usize,
    transmitted_cells: usize,
    content_box: PixelBounds,
}

#[derive(Debug, Deserialize)]
struct TilemapPlan {
    base_tile: String,
    palette_line: u16,
    next_pattern_vram: String,
}

#[derive(Debug, Deserialize)]
struct PaletteRoles {
    background: usize,
    decoration: Vec<usize>,
    jp_text: Vec<usize>,
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
struct IndexedSurface {
    pixels: Vec<u8>,
    owned: Vec<bool>,
}

#[derive(Debug)]
struct IntroPokanBuild {
    manifest: IntroPokanManifest,
    palette: [u16; 16],
    target_surface: IndexedSurface,
    source_pattern_tiles: usize,
    protected_pixels: usize,
    map_payload: Vec<u8>,
    pattern_payload: Vec<u8>,
    map_header: [u8; 6],
    map_bank: Vec<u8>,
    pattern_header: [u8; 6],
    pattern_bank: Vec<u8>,
}

/// Insert the Korean automatic-prologue `콩!` effect into the cumulative ROM.
pub fn apply_intro_pokan(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<IntroPokanSummary, String> {
    let build = build_intro_pokan(source, assets_dir)?;
    let map_bank_end = POKAN_MAP_BANK_OFFSET + build.map_bank.len();
    let pattern_bank_end = POKAN_PATTERN_BANK_OFFSET + build.pattern_bank.len();
    if map_bank_end > POKAN_PATTERN_BANK_OFFSET || pattern_bank_end > output.len() {
        return Err(format!(
            "intro pokan packs exceed their expanded banks: map end 0x{map_bank_end:06X}, pattern end 0x{pattern_bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    apply_expected_write(
        output,
        POKAN_MAP_HEADER_OFFSET,
        source_range(
            source,
            POKAN_MAP_HEADER_OFFSET,
            build.map_header.len(),
            "intro pokan source map header",
        )?,
        &build.map_header,
        "intro pokan map pack header",
    )?;
    apply_expected_write(
        output,
        POKAN_PATTERN_HEADER_OFFSET,
        source_range(
            source,
            POKAN_PATTERN_HEADER_OFFSET,
            build.pattern_header.len(),
            "intro pokan source pattern header",
        )?,
        &build.pattern_header,
        "intro pokan pattern pack header",
    )?;
    apply_expected_write(
        output,
        POKAN_MAP_BANK_OFFSET,
        &vec![0xFF; build.map_bank.len()],
        &build.map_bank,
        "intro pokan expanded map pack",
    )?;
    apply_expected_write(
        output,
        POKAN_PATTERN_BANK_OFFSET,
        &vec![0xFF; build.pattern_bank.len()],
        &build.pattern_bank,
        "intro pokan expanded pattern pack",
    )?;

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after intro pokan graphics",
    )?;
    let changed_ranges = [
        (
            POKAN_MAP_HEADER_OFFSET,
            POKAN_MAP_HEADER_OFFSET + build.map_header.len(),
        ),
        (
            POKAN_PATTERN_HEADER_OFFSET,
            POKAN_PATTERN_HEADER_OFFSET + build.pattern_header.len(),
        ),
        (POKAN_MAP_BANK_OFFSET, map_bank_end),
        (POKAN_PATTERN_BANK_OFFSET, pattern_bank_end),
        (CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2),
    ];
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let inserted_map = decode_mode1_pack_entry(output, POKAN_MAP_HEADER_OFFSET)?;
    let inserted_patterns = decode_mode1_pack_entry(output, POKAN_PATTERN_HEADER_OFFSET)?;
    if inserted_map.vram_destination != POKAN_MAP_VRAM
        || inserted_map.data != build.map_payload
        || inserted_patterns.vram_destination != POKAN_PATTERN_VRAM
        || inserted_patterns.data != build.pattern_payload
    {
        return Err(
            "inserted intro pokan packs do not decode to the planned JP-derived surfaces"
                .to_string(),
        );
    }
    let rebuilt =
        render_indexed_surface(&inserted_map.data, &inserted_patterns.data, &build.manifest)?;
    if rebuilt.pixels != build.target_surface.pixels || rebuilt.owned != build.target_surface.owned
    {
        return Err("inserted intro pokan packs do not reconstruct the target surface".to_string());
    }

    eprintln!("JP graphics GFX-INTRO-POKAN Expected Writes:");
    eprintln!(
        "  0x{POKAN_MAP_HEADER_OFFSET:06X}..0x{:06X}  intro pokan map header ({} bytes)",
        POKAN_MAP_HEADER_OFFSET + build.map_header.len(),
        build.map_header.len()
    );
    eprintln!(
        "  0x{POKAN_PATTERN_HEADER_OFFSET:06X}..0x{:06X}  intro pokan pattern header ({} bytes)",
        POKAN_PATTERN_HEADER_OFFSET + build.pattern_header.len(),
        build.pattern_header.len()
    );
    eprintln!(
        "  0x{POKAN_MAP_BANK_OFFSET:06X}..0x{map_bank_end:06X}  intro pokan map pack ({} bytes)",
        build.map_bank.len()
    );
    eprintln!(
        "  0x{POKAN_PATTERN_BANK_OFFSET:06X}..0x{pattern_bank_end:06X}  intro pokan pattern pack ({} bytes)",
        build.pattern_bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render a 2x nearest-neighbor preview of the exact indexed effect surface.
pub fn write_intro_pokan_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<IntroPokanSummary, String> {
    let build = build_intro_pokan(source, assets_dir)?;
    let surface = build.manifest.output_surface;
    let preview_width = surface.width * PREVIEW_SCALE;
    let preview_height = surface.height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for y in 0..surface.height {
        for x in 0..surface.width {
            let source_offset = y * surface.width + x;
            let color = if build.target_surface.owned[source_offset] {
                md_color(build.palette[build.target_surface.pixels[source_offset] as usize])
            } else {
                [0, 0, 0]
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
    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "intro pokan",
    )?;
    Ok(summary(&build, 0))
}

fn build_intro_pokan(source: &[u8], assets_dir: &Path) -> Result<IntroPokanBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "intro pokan")?;
    let map_declaration = source_pack(&manifest, POKAN_MAP_HEADER_OFFSET)?;
    let pattern_declaration = source_pack(&manifest, POKAN_PATTERN_HEADER_OFFSET)?;
    let source_map = checked_source_pack(source, map_declaration)?;
    let source_patterns = checked_source_pack(source, pattern_declaration)?;
    let source_surface = render_indexed_surface(&source_map, &source_patterns, &manifest)?;
    validate_source_palette_roles(&source_surface, &manifest)?;

    let mut cleaned = source_surface.pixels.clone();
    for (index, pixel) in cleaned.iter_mut().enumerate() {
        if source_surface.owned[index]
            && manifest
                .source_palette_roles
                .jp_text
                .contains(&(*pixel as usize))
        {
            *pixel = manifest.source_palette_roles.background as u8;
        }
    }
    let master = read_master(assets_dir, &manifest)?;
    let mut target_pixels = cleaned.clone();
    composite_master(
        &master,
        &manifest,
        &palette,
        &mut target_pixels,
        &source_surface.owned,
    )?;
    let protected_pixels =
        validate_protected_surface(&source_surface.pixels, &target_pixels, &manifest)?;

    let (map_payload, pattern_payload) =
        compile_indexed_surface(&target_pixels, &source_surface.owned, &manifest)?;
    let rebuilt = render_indexed_surface(&map_payload, &pattern_payload, &manifest)?;
    if rebuilt.pixels != target_pixels || rebuilt.owned != source_surface.owned {
        return Err("intro pokan map and pattern compiler changed the indexed surface".to_string());
    }

    let map_encoded =
        encode_locked_mode1_bytes(POKAN_MAP_BANK_OFFSET, POKAN_MAP_VRAM, &map_payload)?;
    let pattern_encoded = encode_locked_mode1_pack(
        POKAN_PATTERN_BANK_OFFSET,
        POKAN_PATTERN_VRAM,
        &pattern_payload,
    )?;
    validate_pack_roundtrip(
        POKAN_MAP_BANK_OFFSET,
        &map_encoded.header,
        &map_encoded.bank,
        POKAN_MAP_VRAM,
        &map_payload,
    )?;
    validate_pack_roundtrip(
        POKAN_PATTERN_BANK_OFFSET,
        &pattern_encoded.header,
        &pattern_encoded.bank,
        POKAN_PATTERN_VRAM,
        &pattern_payload,
    )?;

    Ok(IntroPokanBuild {
        manifest,
        palette,
        target_surface: IndexedSurface {
            pixels: target_pixels,
            owned: source_surface.owned,
        },
        source_pattern_tiles: source_patterns.len() / MD_TILE_BYTES,
        protected_pixels,
        map_payload,
        pattern_payload,
        map_header: map_encoded.header,
        map_bank: map_encoded.bank,
        pattern_header: pattern_encoded.header,
        pattern_bank: pattern_encoded.bank,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<IntroPokanManifest, String> {
    let path = assets_dir.join("graphics_text/intro_pokan.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read intro pokan source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid intro pokan source {}: {error}", path.display()))
}

fn read_master(assets_dir: &Path, manifest: &IntroPokanManifest) -> Result<Vec<u8>, String> {
    read_verified_rgba(
        assets_dir,
        &manifest.master_asset,
        &manifest.master_sha256,
        manifest.master_width,
        manifest.master_height,
        manifest.master_alpha_bounds,
        "intro pokan master",
    )
}

fn validate_manifest_shape(manifest: &IntroPokanManifest) -> Result<(), String> {
    if manifest.schema_version != 1 || manifest.asset_group_id != "GFX-INTRO-POKAN" {
        return Err("unsupported intro pokan manifest identity".to_string());
    }
    let surface = manifest.output_surface;
    if surface.width != 512
        || surface.height != 136
        || surface.tile_columns != 64
        || surface.tile_rows != 17
        || surface.width != surface.tile_columns * 8
        || surface.height != surface.tile_rows * 8
        || surface.transmitted_cells != 1055
        || surface.content_box.x + surface.content_box.width > surface.width
        || surface.content_box.y + surface.content_box.height > surface.height
    {
        return Err("intro pokan output surface drifted from the JP 64x17 map".to_string());
    }
    if parse_hex(&manifest.tilemap.base_tile)? != 0x0100
        || manifest.tilemap.palette_line != 1
        || parse_hex(&manifest.tilemap.next_pattern_vram)? != 0x4000
    {
        return Err("intro pokan tilemap plan drifted from the JP consumer".to_string());
    }
    if manifest.source_palette_roles.background >= 16
        || manifest.source_palette_roles.decoration.is_empty()
        || manifest.source_palette_roles.jp_text.is_empty()
        || manifest.allowed_composite_palette_indices.is_empty()
        || manifest
            .allowed_composite_palette_indices
            .iter()
            .any(|&index| index >= 16)
    {
        return Err("intro pokan palette roles are invalid".to_string());
    }
    let mut roles = BTreeSet::from([manifest.source_palette_roles.background]);
    for &index in &manifest.source_palette_roles.decoration {
        if !roles.insert(index) {
            return Err("intro pokan palette roles overlap".to_string());
        }
    }
    for &index in &manifest.source_palette_roles.jp_text {
        if !roles.insert(index) {
            return Err("intro pokan palette roles overlap".to_string());
        }
    }
    if !roles
        .iter()
        .all(|index| manifest.allowed_composite_palette_indices.contains(index))
    {
        return Err("intro pokan composite palette omits a source role".to_string());
    }
    if manifest.source_packs.len() != 2 {
        return Err("intro pokan manifest must declare exactly two JP transfers".to_string());
    }
    let expected_headers = BTreeSet::from([POKAN_MAP_HEADER_OFFSET, POKAN_PATTERN_HEADER_OFFSET]);
    let actual_headers = manifest
        .source_packs
        .iter()
        .map(|pack| parse_hex(&pack.header_offset))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if actual_headers != expected_headers {
        return Err("intro pokan manifest must close exactly two JP transfers".to_string());
    }
    for (header, id, vram, decoded_bytes) in [
        (
            POKAN_MAP_HEADER_OFFSET,
            "intro-pokan-map",
            POKAN_MAP_VRAM,
            POKAN_MAP_DECODED_BYTES,
        ),
        (
            POKAN_PATTERN_HEADER_OFFSET,
            "intro-pokan-patterns",
            POKAN_PATTERN_VRAM,
            POKAN_PATTERN_DECODED_BYTES,
        ),
    ] {
        let pack = source_pack(manifest, header)?;
        if pack.id != id
            || parse_u16_hex(&pack.vram_destination)? != vram
            || pack.decoded_bytes != decoded_bytes
        {
            return Err(format!(
                "intro pokan source declaration at 0x{header:06X} drifted from the JP consumer"
            ));
        }
    }
    Ok(())
}

fn source_pack(manifest: &IntroPokanManifest, header_offset: usize) -> Result<&SourcePack, String> {
    manifest
        .source_packs
        .iter()
        .find(|pack| {
            parse_hex(&pack.header_offset)
                .map(|offset| offset == header_offset)
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("intro pokan manifest has no source pack at 0x{header_offset:06X}"))
}

fn checked_source_pack(source: &[u8], declaration: &SourcePack) -> Result<Vec<u8>, String> {
    let header = parse_hex(&declaration.header_offset)?;
    let expected_vram = parse_u16_hex(&declaration.vram_destination)?;
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

fn render_indexed_surface(
    map: &[u8],
    patterns: &[u8],
    manifest: &IntroPokanManifest,
) -> Result<IndexedSurface, String> {
    let surface = manifest.output_surface;
    if map.len() != surface.transmitted_cells * 2 || !patterns.len().is_multiple_of(MD_TILE_BYTES) {
        return Err("intro pokan decoded map or pattern length is invalid".to_string());
    }
    let base_tile = parse_u16_hex(&manifest.tilemap.base_tile)?;
    let pattern_tiles = patterns.len() / MD_TILE_BYTES;
    let mut pixels = vec![0u8; surface.width * surface.height];
    let mut owned = vec![false; pixels.len()];
    for cell in 0..surface.transmitted_cells {
        let word = u16::from_be_bytes([map[cell * 2], map[cell * 2 + 1]]);
        let tile = word & 0x07FF;
        let palette_line = (word >> 13) & 0x03;
        if word & 0x9800 != 0 || palette_line != manifest.tilemap.palette_line {
            return Err(format!(
                "intro pokan map cell {cell} uses unsupported attributes 0x{word:04X}"
            ));
        }
        if tile < base_tile || tile as usize >= base_tile as usize + pattern_tiles {
            return Err(format!(
                "intro pokan map cell {cell} references tile 0x{tile:03X} outside the pattern transfer"
            ));
        }
        let pattern = &patterns[(tile - base_tile) as usize * MD_TILE_BYTES
            ..(tile - base_tile + 1) as usize * MD_TILE_BYTES];
        let tile_x = cell % surface.tile_columns;
        let tile_y = cell / surface.tile_columns;
        for local_y in 0..8 {
            for local_x in 0..8 {
                let byte = pattern[local_y * 4 + local_x / 2];
                let pixel = if local_x.is_multiple_of(2) {
                    byte >> 4
                } else {
                    byte & 0x0F
                };
                let x = tile_x * 8 + local_x;
                let y = tile_y * 8 + local_y;
                let offset = y * surface.width + x;
                pixels[offset] = pixel;
                owned[offset] = true;
            }
        }
    }
    Ok(IndexedSurface { pixels, owned })
}

fn validate_source_palette_roles(
    source: &IndexedSurface,
    manifest: &IntroPokanManifest,
) -> Result<(), String> {
    let mut expected = BTreeSet::from([manifest.source_palette_roles.background]);
    expected.extend(manifest.source_palette_roles.decoration.iter().copied());
    expected.extend(manifest.source_palette_roles.jp_text.iter().copied());
    let actual = source
        .pixels
        .iter()
        .zip(&source.owned)
        .filter_map(|(&pixel, &owned)| owned.then_some(pixel as usize))
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "intro pokan JP palette roles drifted: expected {expected:?}, got {actual:?}"
        ));
    }
    Ok(())
}

fn composite_master(
    master: &[u8],
    manifest: &IntroPokanManifest,
    palette: &[u16; 16],
    target: &mut [u8],
    owned: &[bool],
) -> Result<(), String> {
    let surface = manifest.output_surface;
    if target.len() != surface.width * surface.height || owned.len() != target.len() {
        return Err("intro pokan target surface length is invalid".to_string());
    }
    let bounds = manifest.master_alpha_bounds;
    let (fit_width, fit_height) = fit_bounds_within(bounds, surface.content_box, "intro pokan")?;
    let target_x = surface.content_box.x + (surface.content_box.width - fit_width) / 2;
    let target_y = surface.content_box.y + (surface.content_box.height - fit_height) / 2;
    for output_y in 0..fit_height {
        for output_x in 0..fit_width {
            let destination_x = target_x + output_x;
            let destination_y = target_y + output_y;
            let destination = destination_y * surface.width + destination_x;
            if !owned[destination] {
                return Err("intro pokan master reaches an unowned JP map cell".to_string());
            }
            let source_x0 = bounds.x + output_x * bounds.width / fit_width;
            let source_x1 = bounds.x + (output_x + 1) * bounds.width / fit_width;
            let source_y0 = bounds.y + output_y * bounds.height / fit_height;
            let source_y1 = bounds.y + (output_y + 1) * bounds.height / fit_height;
            if source_x0 >= source_x1 || source_y0 >= source_y1 {
                return Err("intro pokan area reduction produced an empty sample".to_string());
            }
            let background = md_color(palette[target[destination] as usize]);
            let mut sums = [0u64; 3];
            let mut samples = 0u64;
            let mut alpha_sum = 0u64;
            for source_y in source_y0..source_y1 {
                for source_x in source_x0..source_x1 {
                    let source_offset = (source_y * manifest.master_width + source_x) * 4;
                    let alpha = master[source_offset + 3] as u64;
                    alpha_sum += alpha;
                    for channel in 0..3 {
                        let foreground = master[source_offset + channel] as u64;
                        let composite =
                            foreground * alpha + background[channel] as u64 * (255 - alpha);
                        sums[channel] += (composite + 127) / 255;
                    }
                    samples += 1;
                }
            }
            if alpha_sum == 0 {
                continue;
            }
            let averaged = [
                ((sums[0] + samples / 2) / samples) as u8,
                ((sums[1] + samples / 2) / samples) as u8,
                ((sums[2] + samples / 2) / samples) as u8,
            ];
            target[destination] = nearest_palette_index(
                averaged,
                palette,
                &manifest.allowed_composite_palette_indices,
                "intro pokan",
            )? as u8;
        }
    }
    Ok(())
}

fn validate_protected_surface(
    source: &[u8],
    target: &[u8],
    manifest: &IntroPokanManifest,
) -> Result<usize, String> {
    let surface = manifest.output_surface;
    if source.len() != target.len() || source.len() != surface.width * surface.height {
        return Err("intro pokan protected-surface length is invalid".to_string());
    }
    let box_ = surface.content_box;
    let mut protected = 0usize;
    for y in 0..surface.height {
        for x in 0..surface.width {
            let inside =
                x >= box_.x && x < box_.x + box_.width && y >= box_.y && y < box_.y + box_.height;
            if !inside {
                let offset = y * surface.width + x;
                if source[offset] != target[offset] {
                    return Err(format!(
                        "intro pokan changed protected source pixel ({x}, {y})"
                    ));
                }
                protected += 1;
            }
        }
    }
    Ok(protected)
}

fn compile_indexed_surface(
    pixels: &[u8],
    owned: &[bool],
    manifest: &IntroPokanManifest,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let surface = manifest.output_surface;
    if pixels.len() != surface.width * surface.height || owned.len() != pixels.len() {
        return Err("intro pokan indexed surface length is invalid".to_string());
    }
    let base_tile = parse_u16_hex(&manifest.tilemap.base_tile)?;
    let next_pattern_vram = parse_u16_hex(&manifest.tilemap.next_pattern_vram)? as usize;
    let maximum_tiles = (next_pattern_vram - POKAN_PATTERN_VRAM as usize) / MD_TILE_BYTES;
    let mut tile_by_pattern = BTreeMap::<Vec<u8>, u16>::new();
    let mut patterns = Vec::new();
    let mut map = Vec::with_capacity(surface.transmitted_cells * 2);
    for cell in 0..surface.transmitted_cells {
        let tile_x = cell % surface.tile_columns;
        let tile_y = cell / surface.tile_columns;
        let mut pattern = vec![0u8; MD_TILE_BYTES];
        for local_y in 0..8 {
            for pair_x in 0..4 {
                let x = tile_x * 8 + pair_x * 2;
                let y = tile_y * 8 + local_y;
                let left_offset = y * surface.width + x;
                if !owned[left_offset] || !owned[left_offset + 1] {
                    return Err(
                        "intro pokan transmitted tile contains an unowned pixel".to_string()
                    );
                }
                let left = pixels[left_offset];
                let right = pixels[left_offset + 1];
                if left >= 16 || right >= 16 {
                    return Err("intro pokan uses an invalid palette index".to_string());
                }
                pattern[local_y * 4 + pair_x] = (left << 4) | right;
            }
        }
        let pattern_index = if let Some(&index) = tile_by_pattern.get(&pattern) {
            index
        } else {
            let index = u16::try_from(patterns.len() / MD_TILE_BYTES)
                .map_err(|_| "intro pokan uses too many patterns".to_string())?;
            patterns.extend_from_slice(&pattern);
            tile_by_pattern.insert(pattern, index);
            index
        };
        if pattern_index as usize >= maximum_tiles {
            return Err(format!(
                "intro pokan needs {} patterns but only {maximum_tiles} fit before VRAM 0x{next_pattern_vram:04X}",
                pattern_index as usize + 1
            ));
        }
        let word = (manifest.tilemap.palette_line << 13) | (base_tile + pattern_index);
        map.extend_from_slice(&word.to_be_bytes());
    }
    Ok((map, patterns))
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
        return Err("intro pokan mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn summary(build: &IntroPokanBuild, checksum: u16) -> IntroPokanSummary {
    IntroPokanSummary {
        source_pattern_tiles: build.source_pattern_tiles,
        output_pattern_tiles: build.pattern_payload.len() / MD_TILE_BYTES,
        map_bytes: build.map_payload.len(),
        protected_pixels: build.protected_pixels,
        map_pack_bytes: build.map_bank.len(),
        pattern_pack_bytes: build.pattern_bank.len(),
        checksum,
    }
}
