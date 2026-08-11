//! JP-source Panotty `ぽか` -> `퍽` battle-effect compiler.
//!
//! Enemy and amigo paths share one 119-tile mode-1 payload. The final twelve
//! tiles form a dense 48x16 surface, but its leading 16x16 action mark is not
//! part of `ぽか`. Tiles 0..111 therefore remain byte-identical and only the
//! final eight text tiles are rebuilt from the committed Korean master.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::pixel::{
    PixelBounds, decode_md_tiles_column_major, encode_md_tiles_column_major, md_color,
    parse_md_palette, read_verified_rgba, reduce_rgba_to_indexed_surface, write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const PANOTTY_POKA_HEADER_OFFSETS: [usize; 2] = [0x07_610E, 0x07_88B8];
const PANOTTY_POKA_VRAM_DESTINATIONS: [u16; 2] = [0x9000, 0x3C00];
const PANOTTY_POKA_BANK_OFFSET: usize = 0x29_0000;
const PANOTTY_POKA_BANK_LIMIT: usize = 0x29_8000;
const PANOTTY_POKA_SOURCE_BYTES: usize = 3_808;
const PANOTTY_POKA_SOURCE_TILES: usize = PANOTTY_POKA_SOURCE_BYTES / MD_TILE_BYTES;
const PANOTTY_POKA_EFFECT_TILE_START: usize = 107;
const PANOTTY_POKA_PROTECTED_TILE_END: usize = 111;
const PANOTTY_POKA_MUTABLE_TILE_START: usize = PANOTTY_POKA_PROTECTED_TILE_END;
const PANOTTY_POKA_MUTABLE_TILE_END: usize = 119;
const PANOTTY_POKA_EFFECT_TILES: usize =
    PANOTTY_POKA_MUTABLE_TILE_END - PANOTTY_POKA_EFFECT_TILE_START;
const PANOTTY_POKA_MUTABLE_TILES: usize =
    PANOTTY_POKA_MUTABLE_TILE_END - PANOTTY_POKA_MUTABLE_TILE_START;
const PANOTTY_POKA_SURFACE_WIDTH: usize = 48;
const PANOTTY_POKA_SURFACE_HEIGHT: usize = 16;
const PANOTTY_POKA_PROTECTED_WIDTH: usize = 16;
const PANOTTY_POKA_LAYOUT: [[usize; 6]; 2] = [[0, 2, 4, 6, 8, 10], [1, 3, 5, 7, 9, 11]];
const PREVIEW_SCALE: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanottyPokaSummary {
    pub source_tiles: usize,
    pub rewritten_tiles: usize,
    pub protected_bytes: usize,
    pub consumer_headers: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct PanottyPokaManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
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
    effect_tile_range: TileRange,
    protected_tile_range: TileRange,
    mutable_tile_range: TileRange,
    tile_layout: Vec<Vec<usize>>,
    source_packs: Vec<SourcePack>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
    protected_box: PixelBounds,
    content_box: PixelBounds,
    alpha_threshold: u8,
}

#[derive(Debug, Deserialize)]
struct TileRange {
    start: usize,
    end_exclusive: usize,
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
struct PanottyPokaBuild {
    manifest: PanottyPokaManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    surface: Vec<u8>,
    headers: Vec<[u8; 6]>,
    bank: Vec<u8>,
}

/// Insert the Korean Panotty `퍽` effect into both JP enemy and amigo consumers.
pub fn apply_panotty_poka(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<PanottyPokaSummary, String> {
    let build = build_panotty_poka(source, assets_dir)?;
    let bank_end = PANOTTY_POKA_BANK_OFFSET + build.bank.len();
    if bank_end > PANOTTY_POKA_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "Panotty poka pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(build.headers.len() + 2);
    for (declaration, header) in build.manifest.source_packs.iter().zip(&build.headers) {
        let header_offset = parse_hex(&declaration.header_offset)?;
        apply_expected_write(
            output,
            header_offset,
            source_range(
                source,
                header_offset,
                header.len(),
                "Panotty poka source header",
            )?,
            header,
            &format!("Panotty poka {} header", declaration.id),
        )?;
        changed_ranges.push((header_offset, header_offset + header.len()));
    }
    apply_expected_write(
        output,
        PANOTTY_POKA_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "Panotty poka expanded pack",
    )?;
    changed_ranges.push((PANOTTY_POKA_BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after Panotty poka graphics",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    for declaration in &build.manifest.source_packs {
        let header_offset = parse_hex(&declaration.header_offset)?;
        let inserted = decode_mode1_pack_entry(output, header_offset)?;
        if inserted.vram_destination != parse_u16_hex(&declaration.vram_destination)?
            || inserted.data != build.payload
        {
            return Err(format!(
                "inserted Panotty poka pack does not match consumer {}",
                declaration.id
            ));
        }
    }

    eprintln!("JP graphics GFX-BATTLE-PANOTTY-POKA Expected Writes:");
    for (declaration, header) in build.manifest.source_packs.iter().zip(&build.headers) {
        let header_offset = parse_hex(&declaration.header_offset)?;
        eprintln!(
            "  0x{header_offset:06X}..0x{:06X}  {} header ({} bytes)",
            header_offset + header.len(),
            declaration.id,
            header.len()
        );
    }
    eprintln!(
        "  0x{PANOTTY_POKA_BANK_OFFSET:06X}..0x{bank_end:06X}  Panotty poka pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render a checker-backed preview of the exact 48x16 JP effect surface.
pub fn write_panotty_poka_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<PanottyPokaSummary, String> {
    let build = build_panotty_poka(source, assets_dir)?;
    let surface = build.manifest.output_surface;
    let preview_width = surface.width * PREVIEW_SCALE;
    let preview_height = surface.height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for y in 0..surface.height {
        for x in 0..surface.width {
            let palette_index = build.surface[y * surface.width + x] as usize;
            let color = if palette_index == build.manifest.transparent_palette_index {
                if (x / 2 + y / 2).is_multiple_of(2) {
                    [210, 210, 210]
                } else {
                    [242, 242, 242]
                }
            } else {
                md_color(build.palette[palette_index])
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
        "Panotty poka",
    )?;
    Ok(summary(&build, 0))
}

fn build_panotty_poka(source: &[u8], assets_dir: &Path) -> Result<PanottyPokaBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "Panotty poka")?;
    let master = read_master(assets_dir, &manifest)?;
    let master_surface = reduce_master_to_surface(&master, &manifest, &palette)?;

    let mut source_payload = None;
    for declaration in &manifest.source_packs {
        let decoded = checked_source_pack(source, declaration)?;
        if let Some(expected) = &source_payload {
            if expected != &decoded {
                return Err(format!(
                    "Panotty poka consumer {} does not share the canonical JP payload",
                    declaration.id
                ));
            }
        } else {
            source_payload = Some(decoded);
        }
    }
    let source_payload =
        source_payload.ok_or_else(|| "Panotty poka has no JP source payload".to_string())?;
    let effect_start = PANOTTY_POKA_EFFECT_TILE_START * MD_TILE_BYTES;
    let effect_end = PANOTTY_POKA_MUTABLE_TILE_END * MD_TILE_BYTES;
    let source_surface = decode_md_tiles_column_major(
        &source_payload[effect_start..effect_end],
        PANOTTY_POKA_SURFACE_WIDTH,
        PANOTTY_POKA_SURFACE_HEIGHT,
        "Panotty poka JP effect",
    )?;
    validate_source_effect(&source_surface, &manifest)?;
    let surface = compose_effect_surface(&source_surface, &master_surface, &manifest)?;
    let effect_tiles = encode_md_tiles_column_major(
        &surface,
        PANOTTY_POKA_SURFACE_WIDTH,
        PANOTTY_POKA_SURFACE_HEIGHT,
        "Panotty poka",
    )?;

    let mutable_payload_start = PANOTTY_POKA_MUTABLE_TILE_START * MD_TILE_BYTES;
    let mutable_effect_start =
        (PANOTTY_POKA_MUTABLE_TILE_START - PANOTTY_POKA_EFFECT_TILE_START) * MD_TILE_BYTES;
    let mut payload = source_payload.clone();
    payload[mutable_payload_start..effect_end]
        .copy_from_slice(&effect_tiles[mutable_effect_start..]);
    if payload[..mutable_payload_start] != source_payload[..mutable_payload_start] {
        return Err("Panotty poka compiler changed protected JP bytes".to_string());
    }

    let mut headers = Vec::with_capacity(manifest.source_packs.len());
    let mut bank = None;
    for declaration in &manifest.source_packs {
        let vram = parse_u16_hex(&declaration.vram_destination)?;
        let encoded = encode_locked_mode1_pack(PANOTTY_POKA_BANK_OFFSET, vram, &payload)?;
        if let Some(expected_bank) = &bank {
            if expected_bank != &encoded.bank {
                return Err("Panotty poka headers produced divergent packed payloads".to_string());
            }
        } else {
            bank = Some(encoded.bank.clone());
        }
        validate_pack_roundtrip(&encoded.header, &encoded.bank, vram, &payload)?;
        headers.push(encoded.header);
    }

    Ok(PanottyPokaBuild {
        manifest,
        palette,
        source_payload,
        payload,
        surface,
        headers,
        bank: bank.ok_or_else(|| "Panotty poka pack was not encoded".to_string())?,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<PanottyPokaManifest, String> {
    let path = assets_dir.join("graphics_text/panotty_poka.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read Panotty poka source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid Panotty poka source {}: {error}", path.display()))
}

fn read_master(assets_dir: &Path, manifest: &PanottyPokaManifest) -> Result<Vec<u8>, String> {
    read_verified_rgba(
        assets_dir,
        &manifest.master_asset,
        &manifest.master_sha256,
        manifest.master_width,
        manifest.master_height,
        manifest.master_alpha_bounds,
        "Panotty poka master",
    )
}

fn validate_manifest_shape(manifest: &PanottyPokaManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-BATTLE-PANOTTY-POKA"
        || !manifest.source_policy.contains("JP")
    {
        return Err("unsupported Panotty poka manifest identity".to_string());
    }
    let surface = manifest.output_surface;
    if surface.width != PANOTTY_POKA_SURFACE_WIDTH
        || surface.height != PANOTTY_POKA_SURFACE_HEIGHT
        || surface.protected_box
            != (PixelBounds {
                x: 0,
                y: 0,
                width: PANOTTY_POKA_PROTECTED_WIDTH,
                height: PANOTTY_POKA_SURFACE_HEIGHT,
            })
        || surface.content_box
            != (PixelBounds {
                x: PANOTTY_POKA_PROTECTED_WIDTH,
                y: 0,
                width: PANOTTY_POKA_SURFACE_WIDTH - PANOTTY_POKA_PROTECTED_WIDTH,
                height: PANOTTY_POKA_SURFACE_HEIGHT,
            })
        || surface.alpha_threshold == 0
        || surface.alpha_threshold == u8::MAX
    {
        return Err("Panotty poka output surface drifted from the JP layout".to_string());
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
        return Err("Panotty poka palette admission is invalid".to_string());
    }
    if manifest
        .source_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        != BTreeSet::from([0, 7, 11, 13])
    {
        return Err("Panotty poka JP source palette roles drifted".to_string());
    }
    if (
        manifest.effect_tile_range.start,
        manifest.effect_tile_range.end_exclusive,
    ) != (
        PANOTTY_POKA_EFFECT_TILE_START,
        PANOTTY_POKA_MUTABLE_TILE_END,
    ) || manifest.effect_tile_range.end_exclusive - manifest.effect_tile_range.start
        != PANOTTY_POKA_EFFECT_TILES
        || (
            manifest.protected_tile_range.start,
            manifest.protected_tile_range.end_exclusive,
        ) != (
            PANOTTY_POKA_EFFECT_TILE_START,
            PANOTTY_POKA_PROTECTED_TILE_END,
        )
        || (
            manifest.mutable_tile_range.start,
            manifest.mutable_tile_range.end_exclusive,
        ) != (
            PANOTTY_POKA_MUTABLE_TILE_START,
            PANOTTY_POKA_MUTABLE_TILE_END,
        )
        || PANOTTY_POKA_MUTABLE_TILE_END != PANOTTY_POKA_SOURCE_TILES
    {
        return Err("Panotty poka tile ownership drifted from the final JP tiles".to_string());
    }
    if manifest.tile_layout
        != PANOTTY_POKA_LAYOUT
            .iter()
            .map(|row| row.to_vec())
            .collect::<Vec<_>>()
    {
        return Err("Panotty poka tile layout drifted from the JP structure".to_string());
    }

    let expected_packs = [
        (
            "panotty-poka-enemy",
            PANOTTY_POKA_HEADER_OFFSETS[0],
            PANOTTY_POKA_VRAM_DESTINATIONS[0],
        ),
        (
            "panotty-poka-amigo",
            PANOTTY_POKA_HEADER_OFFSETS[1],
            PANOTTY_POKA_VRAM_DESTINATIONS[1],
        ),
    ];
    if manifest.source_packs.len() != expected_packs.len() {
        return Err("Panotty poka manifest must declare both JP consumers".to_string());
    }
    for (pack, (id, header, vram)) in manifest.source_packs.iter().zip(expected_packs) {
        if pack.id != id
            || parse_hex(&pack.header_offset)? != header
            || parse_u16_hex(&pack.vram_destination)? != vram
            || pack.decoded_bytes != PANOTTY_POKA_SOURCE_BYTES
        {
            return Err(format!(
                "Panotty poka source declaration {} drifted from its JP consumer",
                pack.id
            ));
        }
    }
    Ok(())
}

fn reduce_master_to_surface(
    master: &[u8],
    manifest: &PanottyPokaManifest,
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
        "Panotty poka",
    )
}

fn compose_effect_surface(
    source: &[u8],
    master: &[u8],
    manifest: &PanottyPokaManifest,
) -> Result<Vec<u8>, String> {
    let surface = manifest.output_surface;
    if source.len() != surface.width * surface.height
        || master.len() != surface.width * surface.height
    {
        return Err("Panotty poka surface length is invalid".to_string());
    }
    for y in 0..surface.height {
        for x in 0..PANOTTY_POKA_PROTECTED_WIDTH {
            if master[y * surface.width + x] as usize != manifest.transparent_palette_index {
                return Err("Panotty poka master overlaps the protected action mark".to_string());
            }
        }
    }
    let mut output = source.to_vec();
    for y in 0..surface.height {
        for x in PANOTTY_POKA_PROTECTED_WIDTH..surface.width {
            output[y * surface.width + x] = master[y * surface.width + x];
        }
    }
    Ok(output)
}

fn validate_source_effect(pixels: &[u8], manifest: &PanottyPokaManifest) -> Result<(), String> {
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
            "Panotty poka JP effect palette drifted: expected {expected:?}, got {actual:?}"
        ));
    }
    Ok(())
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
    expected_vram: u16,
    expected_payload: &[u8],
) -> Result<(), String> {
    let mut probe = vec![0u8; PANOTTY_POKA_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[PANOTTY_POKA_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != expected_vram || decoded.data != expected_payload {
        return Err("Panotty poka mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn summary(build: &PanottyPokaBuild, checksum: u16) -> PanottyPokaSummary {
    debug_assert_eq!(build.source_payload.len(), build.payload.len());
    PanottyPokaSummary {
        source_tiles: PANOTTY_POKA_SOURCE_TILES,
        rewritten_tiles: PANOTTY_POKA_MUTABLE_TILES,
        protected_bytes: build.source_payload.len() - PANOTTY_POKA_MUTABLE_TILES * MD_TILE_BYTES,
        consumer_headers: build.headers.len(),
        pack_bytes: build.bank.len(),
        checksum,
    }
}
