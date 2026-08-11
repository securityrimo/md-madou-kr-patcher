//! JP-source Compile splash slogan pixel fitting and insertion.
//!
//! The Korean high-resolution master is cropped by its exact alpha bounds,
//! area-reduced to the original 136x16 surface, mapped only to the JP palette,
//! and written into the first 34 tiles of the original high-pattern transfer.
//! Every other decoded byte and every other Compile transfer remains JP-owned.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::pixel::{
    PixelBounds, encode_md_tiles_column_major, md_color,
    nearest_palette_index as nearest_md_palette_index, parse_md_palette, read_verified_rgba,
    write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const COMPILE_SLOGAN_HEADER_OFFSET: usize = 0x09_2BA0;
const COMPILE_SLOGAN_VRAM: u16 = 0x2000;
const COMPILE_SLOGAN_BANK_OFFSET: usize = 0x26_8000;
const COMPILE_SLOGAN_TILES: usize = 34;
const COMPILE_SLOGAN_BYTES: usize = COMPILE_SLOGAN_TILES * MD_TILE_BYTES;
const PREVIEW_SCALE: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileSloganSummary {
    pub rewritten_tiles: usize,
    pub decoded_bytes: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct CompileSloganManifest {
    schema_version: u32,
    asset_group_id: String,
    master_asset: String,
    master_sha256: String,
    master_width: usize,
    master_height: usize,
    master_alpha_bounds: PixelBounds,
    output_surface: OutputSurface,
    palette_line_words: Vec<String>,
    allowed_palette_indices: Vec<usize>,
    background_palette_index: usize,
    source_packs: Vec<SourcePack>,
    mutable_tile_range: MutableTileRange,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
    content_x: usize,
    content_y: usize,
    content_width: usize,
    content_height: usize,
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
struct MutableTileRange {
    start: usize,
    end_exclusive: usize,
}

#[derive(Debug)]
struct CompileSloganBuild {
    manifest: CompileSloganManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    pixel_fit: Vec<u8>,
    header: [u8; 6],
    bank: Vec<u8>,
}

/// Insert the Korean Compile splash slogan into the cumulative JP-to-KR ROM.
pub fn apply_compile_slogan(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<CompileSloganSummary, String> {
    let build = build_compile_slogan(source, assets_dir)?;
    let bank_end = COMPILE_SLOGAN_BANK_OFFSET + build.bank.len();
    if bank_end > output.len() {
        return Err(format!(
            "Compile slogan pack ends outside output at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    apply_expected_write(
        output,
        COMPILE_SLOGAN_HEADER_OFFSET,
        source_range(
            source,
            COMPILE_SLOGAN_HEADER_OFFSET,
            build.header.len(),
            "Compile slogan source header",
        )?,
        &build.header,
        "Compile slogan pack header",
    )?;
    let expected_bank = vec![0xFF; build.bank.len()];
    apply_expected_write(
        output,
        COMPILE_SLOGAN_BANK_OFFSET,
        &expected_bank,
        &build.bank,
        "Compile slogan expanded graphics pack",
    )?;

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after Compile slogan graphics",
    )?;
    let changed_ranges = [
        (
            COMPILE_SLOGAN_HEADER_OFFSET,
            COMPILE_SLOGAN_HEADER_OFFSET + build.header.len(),
        ),
        (COMPILE_SLOGAN_BANK_OFFSET, bank_end),
        (CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2),
    ];
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let decoded = decode_mode1_pack_entry(output, COMPILE_SLOGAN_HEADER_OFFSET)?;
    if decoded.vram_destination != COMPILE_SLOGAN_VRAM || decoded.data != build.payload {
        return Err(
            "inserted Compile slogan pack does not decode to the planned JP-derived payload"
                .to_string(),
        );
    }
    for declaration in &build.manifest.source_packs {
        let header_offset = parse_hex(&declaration.header_offset)?;
        if header_offset == COMPILE_SLOGAN_HEADER_OFFSET {
            continue;
        }
        let source_pack = checked_source_pack(source, declaration)?;
        let output_pack = decode_mode1_pack_entry(output, header_offset)?;
        if output_pack.vram_destination != parse_u16_hex(&declaration.vram_destination)?
            || output_pack.data != source_pack
        {
            return Err(format!(
                "Compile slogan insertion changed protected JP transfer {}",
                declaration.id
            ));
        }
    }

    eprintln!("JP graphics GFX-COMPILE-SLOGAN Expected Writes:");
    eprintln!(
        "  0x{COMPILE_SLOGAN_HEADER_OFFSET:06X}..0x{:06X}  Compile slogan pack header ({} bytes)",
        COMPILE_SLOGAN_HEADER_OFFSET + build.header.len(),
        build.header.len()
    );
    eprintln!(
        "  0x{COMPILE_SLOGAN_BANK_OFFSET:06X}..0x{bank_end:06X}  Compile slogan pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render a 4x nearest-neighbor static QA preview of the exact MD pixel fit.
///
/// This proves source compilation and palette admission, not runtime
/// consumption by the game.
pub fn write_compile_slogan_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<CompileSloganSummary, String> {
    let build = build_compile_slogan(source, assets_dir)?;
    let width = build.manifest.output_surface.width;
    let height = build.manifest.output_surface.height;
    let preview_width = width * PREVIEW_SCALE;
    let preview_height = height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for y in 0..height {
        for x in 0..width {
            let palette_index = build.pixel_fit[y * width + x] as usize;
            let color = md_color(build.palette[palette_index]);
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
        "Compile slogan",
    )?;
    Ok(summary(&build, 0))
}

fn build_compile_slogan(source: &[u8], assets_dir: &Path) -> Result<CompileSloganBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_palette(&manifest.palette_line_words)?;
    let master = read_master(assets_dir, &manifest)?;
    let pixel_fit = reduce_master_to_surface(&master, &manifest, &palette)?;

    for declaration in &manifest.source_packs {
        checked_source_pack(source, declaration)?;
    }
    let slogan_pack = source_pack(&manifest, COMPILE_SLOGAN_HEADER_OFFSET)?;
    let source_payload = checked_source_pack(source, slogan_pack)?;
    let mut payload = source_payload.clone();
    let slogan_tiles = encode_slogan_tiles(&pixel_fit, &manifest.output_surface)?;
    if slogan_tiles.len() != COMPILE_SLOGAN_BYTES {
        return Err(format!(
            "Compile slogan encoded as {} bytes, expected {COMPILE_SLOGAN_BYTES}",
            slogan_tiles.len()
        ));
    }
    payload[..COMPILE_SLOGAN_BYTES].copy_from_slice(&slogan_tiles);
    if payload[COMPILE_SLOGAN_BYTES..] != source_payload[COMPILE_SLOGAN_BYTES..] {
        return Err("Compile slogan compiler changed protected JP pattern bytes".to_string());
    }

    let encoded =
        encode_locked_mode1_pack(COMPILE_SLOGAN_BANK_OFFSET, COMPILE_SLOGAN_VRAM, &payload)?;
    validate_pack_roundtrip(&encoded.header, &encoded.bank, &payload)?;

    Ok(CompileSloganBuild {
        manifest,
        palette,
        source_payload,
        payload,
        pixel_fit,
        header: encoded.header,
        bank: encoded.bank,
    })
}

fn validate_manifest_shape(manifest: &CompileSloganManifest) -> Result<(), String> {
    if manifest.schema_version != 1 || manifest.asset_group_id != "GFX-COMPILE-SLOGAN" {
        return Err("unsupported Compile slogan manifest identity".to_string());
    }
    let surface = manifest.output_surface;
    if surface.width != 136
        || surface.height != 16
        || surface.content_x + surface.content_width > surface.width
        || surface.content_y + surface.content_height > surface.height
        || surface.content_width == 0
        || surface.content_height == 0
    {
        return Err("Compile slogan output surface must be a bounded 136x16 region".to_string());
    }
    if manifest.mutable_tile_range.start != 0
        || manifest.mutable_tile_range.end_exclusive != COMPILE_SLOGAN_TILES
        || surface.width / 8 * (surface.height / 8) != COMPILE_SLOGAN_TILES
    {
        return Err("Compile slogan mutable tile range drifted from the 34 JP tiles".to_string());
    }
    if manifest.allowed_palette_indices.is_empty()
        || !manifest
            .allowed_palette_indices
            .contains(&manifest.background_palette_index)
        || manifest
            .allowed_palette_indices
            .iter()
            .any(|&index| index >= 16)
    {
        return Err("Compile slogan palette admission is invalid".to_string());
    }
    Ok(())
}

fn read_manifest(assets_dir: &Path) -> Result<CompileSloganManifest, String> {
    let path = assets_dir.join("graphics_text/compile_slogan.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read Compile slogan source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid Compile slogan source {}: {error}", path.display()))
}

fn read_master(assets_dir: &Path, manifest: &CompileSloganManifest) -> Result<Vec<u8>, String> {
    read_verified_rgba(
        assets_dir,
        &manifest.master_asset,
        &manifest.master_sha256,
        manifest.master_width,
        manifest.master_height,
        manifest.master_alpha_bounds,
        "Compile slogan master",
    )
}

fn reduce_master_to_surface(
    master: &[u8],
    manifest: &CompileSloganManifest,
    palette: &[u16; 16],
) -> Result<Vec<u8>, String> {
    let surface = manifest.output_surface;
    let bounds = manifest.master_alpha_bounds;
    if bounds.x + bounds.width > manifest.master_width
        || bounds.y + bounds.height > manifest.master_height
    {
        return Err("Compile slogan alpha bounds lie outside the master".to_string());
    }

    let mut output = vec![manifest.background_palette_index as u8; surface.width * surface.height];
    let background = md_color(palette[manifest.background_palette_index]);
    for output_y in 0..surface.content_height {
        for output_x in 0..surface.content_width {
            let source_x0 = bounds.x + output_x * bounds.width / surface.content_width;
            let source_x1 = bounds.x + (output_x + 1) * bounds.width / surface.content_width;
            let source_y0 = bounds.y + output_y * bounds.height / surface.content_height;
            let source_y1 = bounds.y + (output_y + 1) * bounds.height / surface.content_height;
            if source_x0 >= source_x1 || source_y0 >= source_y1 {
                return Err("Compile slogan area reduction produced an empty sample".to_string());
            }
            let mut sums = [0u64; 3];
            let mut samples = 0u64;
            for source_y in source_y0..source_y1 {
                for source_x in source_x0..source_x1 {
                    let offset = (source_y * manifest.master_width + source_x) * 4;
                    let alpha = master[offset + 3] as u64;
                    for channel in 0..3 {
                        let foreground = master[offset + channel] as u64;
                        let composite =
                            foreground * alpha + background[channel] as u64 * (255 - alpha);
                        sums[channel] += (composite + 127) / 255;
                    }
                    samples += 1;
                }
            }
            let averaged = [
                ((sums[0] + samples / 2) / samples) as u8,
                ((sums[1] + samples / 2) / samples) as u8,
                ((sums[2] + samples / 2) / samples) as u8,
            ];
            let palette_index =
                nearest_palette_index(averaged, palette, &manifest.allowed_palette_indices)?;
            let destination_x = surface.content_x + output_x;
            let destination_y = surface.content_y + output_y;
            output[destination_y * surface.width + destination_x] = palette_index as u8;
        }
    }
    Ok(output)
}

fn nearest_palette_index(
    color: [u8; 3],
    palette: &[u16; 16],
    allowed: &[usize],
) -> Result<usize, String> {
    nearest_md_palette_index(color, palette, allowed, "Compile slogan")
}

fn encode_slogan_tiles(pixels: &[u8], surface: &OutputSurface) -> Result<Vec<u8>, String> {
    encode_md_tiles_column_major(pixels, surface.width, surface.height, "Compile slogan")
}

fn source_pack(
    manifest: &CompileSloganManifest,
    header_offset: usize,
) -> Result<&SourcePack, String> {
    manifest
        .source_packs
        .iter()
        .find(|pack| {
            parse_hex(&pack.header_offset)
                .map(|offset| offset == header_offset)
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            format!("Compile slogan manifest has no source pack at 0x{header_offset:06X}")
        })
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

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a VRAM address"))
}

fn parse_palette(words: &[String]) -> Result<[u16; 16], String> {
    parse_md_palette(words, "Compile slogan")
}

fn validate_pack_roundtrip(
    header: &[u8; 6],
    bank: &[u8],
    expected_payload: &[u8],
) -> Result<(), String> {
    let mut probe = vec![0u8; COMPILE_SLOGAN_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[COMPILE_SLOGAN_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != COMPILE_SLOGAN_VRAM || decoded.data != expected_payload {
        return Err("Compile slogan mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn summary(build: &CompileSloganBuild, checksum: u16) -> CompileSloganSummary {
    debug_assert_eq!(build.source_payload.len(), build.payload.len());
    CompileSloganSummary {
        rewritten_tiles: COMPILE_SLOGAN_TILES,
        decoded_bytes: build.payload.len(),
        pack_bytes: build.bank.len(),
        checksum,
    }
}
