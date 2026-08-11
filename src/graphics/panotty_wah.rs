//! JP-source Panotty `わっ` / `ふえ〜ん` battle-effect compiler.
//!
//! Enemy and amigo paths share one 100-tile mode-1 payload through three
//! headers. Tiles 0..40 remain byte-identical, tiles 40..56 become the
//! font-rendered Korean `흐엥`, and the final 44 tiles become the approved
//! Korean `와!`. Both effects retain their exact JP column-major or sparse
//! layouts; unmapped cells must stay transparent.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::{Font, FontSettings};
use serde::Deserialize;

use crate::jp_native;

use super::pixel::{
    PixelBounds, decode_md_tiles_column_major, encode_md_tiles_column_major, md_color,
    native_glyph_pixel, nearest_core_distance, parse_md_palette, read_verified_rgba,
    reduce_rgba_to_indexed_surface, write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const PANOTTY_WAH_HEADER_OFFSETS: [usize; 3] = [0x07_6116, 0x07_88B0, 0x07_88C0];
const PANOTTY_WAH_VRAM_DESTINATIONS: [u16; 3] = [0x9000, 0x3C00, 0x3C00];
const PANOTTY_WAH_BANK_OFFSET: usize = 0x28_8000;
const PANOTTY_WAH_BANK_LIMIT: usize = 0x29_0000;
const PANOTTY_WAH_SOURCE_BYTES: usize = 3_200;
const PANOTTY_WAH_SOURCE_TILES: usize = PANOTTY_WAH_SOURCE_BYTES / MD_TILE_BYTES;
const PANOTTY_FUEEN_TILE_START: usize = 40;
const PANOTTY_FUEEN_TILE_END: usize = 56;
const PANOTTY_FUEEN_TILES: usize = PANOTTY_FUEEN_TILE_END - PANOTTY_FUEEN_TILE_START;
const PANOTTY_FUEEN_BYTES: usize = PANOTTY_FUEEN_TILES * MD_TILE_BYTES;
const PANOTTY_FUEEN_WIDTH: usize = 64;
const PANOTTY_FUEEN_HEIGHT: usize = 16;
const PANOTTY_FUEEN_LAYOUT: [[usize; 8]; 2] =
    [[0, 2, 4, 6, 8, 10, 12, 14], [1, 3, 5, 7, 9, 11, 13, 15]];
const PANOTTY_FUEEN_ENEMY_TABLE_BASE: usize = 0x08_98DA;
const PANOTTY_FUEEN_ENEMY_TABLE_ENTRY: usize = 0x08_9918;
const PANOTTY_FUEEN_ENEMY_DEFINITION: usize = 0x08_9DB6;
const PANOTTY_WAH_MUTABLE_TILE_START: usize = 56;
const PANOTTY_WAH_MUTABLE_TILE_END: usize = 100;
const PANOTTY_WAH_MUTABLE_TILES: usize =
    PANOTTY_WAH_MUTABLE_TILE_END - PANOTTY_WAH_MUTABLE_TILE_START;
const PANOTTY_WAH_MUTABLE_BYTES: usize = PANOTTY_WAH_MUTABLE_TILES * MD_TILE_BYTES;
const PANOTTY_WAH_LAYOUT: [[i8; 10]; 5] = [
    [0, 4, 8, 12, -1, -1, -1, -1, -1, -1],
    [1, 5, 9, 13, 20, 24, 28, 32, 36, 40],
    [2, 6, 10, 14, 21, 25, 29, 33, 37, 41],
    [3, 7, 11, 15, 22, 26, 30, 34, 38, 42],
    [16, 17, 18, 19, 23, 27, 31, 35, 39, 43],
];
const PREVIEW_SCALE: usize = 8;
const FUEEN_PREVIEW_SCALE: usize = 12;
const FUEEN_PREVIEW_GAP: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanottyWahSummary {
    pub source_tiles: usize,
    pub rewritten_tiles: usize,
    pub fueen_rewritten_tiles: usize,
    pub wah_rewritten_tiles: usize,
    pub fueen_visible_pixels: usize,
    pub protected_bytes: usize,
    pub consumer_headers: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct PanottyWahManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    font_asset: String,
    font_sha256: String,
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
    fueen: FueenPlan,
    mutable_tile_range: MutableTileRange,
    tile_layout: Vec<Vec<Option<usize>>>,
    source_packs: Vec<SourcePack>,
}

#[derive(Debug, Deserialize)]
struct FueenPlan {
    id: String,
    jp_text: String,
    ko: String,
    tile_range: MutableTileRange,
    source_tile_sha256: String,
    output_surface: FueenSurface,
    source_palette_indices: Vec<usize>,
    target_palette_indices: Vec<usize>,
    tile_layout: Vec<Vec<usize>>,
    enemy_sprite_consumer: FueenEnemyConsumer,
}

#[derive(Debug, Clone, Deserialize)]
struct FueenSurface {
    width: usize,
    height: usize,
    glyph_size: usize,
    cluster_boxes: Vec<PixelBounds>,
}

#[derive(Debug, Deserialize)]
struct FueenEnemyConsumer {
    offset_table_base: String,
    offset_table_entry: String,
    definition_offset: String,
    record_count: usize,
    y: String,
    size_link: String,
    first_tile: String,
    tile_step: usize,
    x_positions: Vec<String>,
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
struct SourcePack {
    id: String,
    header_offset: String,
    vram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
}

#[derive(Debug)]
struct PanottyWahBuild {
    manifest: PanottyWahManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    pixel_fit: Vec<u8>,
    source_fueen: Vec<u8>,
    fueen_fit: Vec<u8>,
    headers: Vec<[u8; 6]>,
    bank: Vec<u8>,
}

/// Insert the Korean Panotty `와!` and `흐엥` effects into the shared JP pack.
pub fn apply_panotty_wah(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<PanottyWahSummary, String> {
    let build = build_panotty_wah(source, assets_dir)?;
    let bank_end = PANOTTY_WAH_BANK_OFFSET + build.bank.len();
    if bank_end > PANOTTY_WAH_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "Panotty wah pack ends outside its expanded bank at 0x{bank_end:06X}"
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
                "Panotty wah source header",
            )?,
            header,
            &format!("Panotty wah {} header", declaration.id),
        )?;
        changed_ranges.push((header_offset, header_offset + header.len()));
    }
    apply_expected_write(
        output,
        PANOTTY_WAH_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "Panotty wah expanded pack",
    )?;
    changed_ranges.push((PANOTTY_WAH_BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after Panotty wah graphics",
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
                "inserted Panotty wah pack does not match consumer {}",
                declaration.id
            ));
        }
    }

    eprintln!("JP graphics GFX-BATTLE-PANOTTY-WAH/FUEEN Expected Writes:");
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
        "  0x{PANOTTY_WAH_BANK_OFFSET:06X}..0x{bank_end:06X}  Panotty wah pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render an 8x checker-backed preview of the exact sparse 80x40 effect surface.
pub fn write_panotty_wah_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<PanottyWahSummary, String> {
    let build = build_panotty_wah(source, assets_dir)?;
    let surface = build.manifest.output_surface;
    let preview_width = surface.width * PREVIEW_SCALE;
    let preview_height = surface.height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for y in 0..surface.height {
        for x in 0..surface.width {
            let palette_index = build.pixel_fit[y * surface.width + x] as usize;
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
        "Panotty wah",
    )?;
    Ok(summary(&build, 0))
}

/// Render the exact JP and Korean 64x16 defeat-cries side by side.
///
/// This is deterministic static QA evidence, not direct conditioned runtime
/// consumption of either enemy or amigo defeat.
pub fn write_panotty_fueen_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<PanottyWahSummary, String> {
    let build = build_panotty_wah(source, assets_dir)?;
    let surface = &build.manifest.fueen.output_surface;
    let contact_width = surface.width * 2 + FUEEN_PREVIEW_GAP;
    let preview_width = contact_width * FUEEN_PREVIEW_SCALE;
    let preview_height = surface.height * FUEEN_PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[28, 28, 28, 255]);
    }
    for (panel, pixels) in [build.source_fueen.as_slice(), build.fueen_fit.as_slice()]
        .into_iter()
        .enumerate()
    {
        let panel_x = panel * (surface.width + FUEEN_PREVIEW_GAP);
        for y in 0..surface.height {
            for x in 0..surface.width {
                let palette_index = pixels[y * surface.width + x] as usize;
                let color = if palette_index == build.manifest.transparent_palette_index {
                    if (x / 2 + y / 2).is_multiple_of(2) {
                        [210, 210, 210]
                    } else {
                        [242, 242, 242]
                    }
                } else {
                    md_color(build.palette[palette_index])
                };
                for scale_y in 0..FUEEN_PREVIEW_SCALE {
                    for scale_x in 0..FUEEN_PREVIEW_SCALE {
                        let preview_x = (panel_x + x) * FUEEN_PREVIEW_SCALE + scale_x;
                        let preview_y = y * FUEEN_PREVIEW_SCALE + scale_y;
                        let offset = (preview_y * preview_width + preview_x) * 4;
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
        "Panotty fueen",
    )?;
    Ok(summary(&build, 0))
}

fn build_panotty_wah(source: &[u8], assets_dir: &Path) -> Result<PanottyWahBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "Panotty wah")?;
    let font = read_font(assets_dir, &manifest)?;
    let master = read_master(assets_dir, &manifest)?;
    let pixel_fit = reduce_master_to_surface(&master, &manifest, &palette)?;
    let replacement_tiles = encode_layout_surface(&pixel_fit, &manifest)?;

    let mut source_payload = None;
    for declaration in &manifest.source_packs {
        let decoded = checked_source_pack(source, declaration)?;
        if let Some(expected) = &source_payload {
            if expected != &decoded {
                return Err(format!(
                    "Panotty wah consumer {} does not share the canonical JP payload",
                    declaration.id
                ));
            }
        } else {
            source_payload = Some(decoded);
        }
    }
    let source_payload =
        source_payload.ok_or_else(|| "Panotty wah has no JP source payload".to_string())?;
    validate_fueen_enemy_consumer(source, &manifest)?;

    let fueen_start = PANOTTY_FUEEN_TILE_START * MD_TILE_BYTES;
    let fueen_end = PANOTTY_FUEEN_TILE_END * MD_TILE_BYTES;
    let source_fueen_tiles = &source_payload[fueen_start..fueen_end];
    validate_fueen_source(source_fueen_tiles, &manifest)?;
    let source_fueen = decode_md_tiles_column_major(
        source_fueen_tiles,
        PANOTTY_FUEEN_WIDTH,
        PANOTTY_FUEEN_HEIGHT,
        "Panotty fueen JP effect",
    )?;
    let fueen_fit = render_fueen_surface(&font, &manifest)?;
    let fueen_tiles = encode_md_tiles_column_major(
        &fueen_fit,
        PANOTTY_FUEEN_WIDTH,
        PANOTTY_FUEEN_HEIGHT,
        "Panotty fueen Korean effect",
    )?;
    if fueen_tiles.len() != PANOTTY_FUEEN_BYTES {
        return Err("Panotty fueen encoded tile length drifted".to_string());
    }

    let mutable_start = PANOTTY_WAH_MUTABLE_TILE_START * MD_TILE_BYTES;
    let mutable_end = PANOTTY_WAH_MUTABLE_TILE_END * MD_TILE_BYTES;
    validate_source_effect_tiles(&source_payload[mutable_start..mutable_end], &manifest)?;

    let mut payload = source_payload.clone();
    payload[fueen_start..fueen_end].copy_from_slice(&fueen_tiles);
    payload[mutable_start..mutable_end].copy_from_slice(&replacement_tiles);
    if payload[..fueen_start] != source_payload[..fueen_start]
        || payload[fueen_end..mutable_start] != source_payload[fueen_end..mutable_start]
        || payload[mutable_end..] != source_payload[mutable_end..]
    {
        return Err("Panotty wah compiler changed protected JP bytes".to_string());
    }

    let mut headers = Vec::with_capacity(manifest.source_packs.len());
    let mut bank = None;
    for declaration in &manifest.source_packs {
        let vram = parse_u16_hex(&declaration.vram_destination)?;
        let encoded = encode_locked_mode1_pack(PANOTTY_WAH_BANK_OFFSET, vram, &payload)?;
        if let Some(expected_bank) = &bank {
            if expected_bank != &encoded.bank {
                return Err("Panotty wah headers produced divergent packed payloads".to_string());
            }
        } else {
            bank = Some(encoded.bank.clone());
        }
        validate_pack_roundtrip(&encoded.header, &encoded.bank, vram, &payload)?;
        headers.push(encoded.header);
    }

    Ok(PanottyWahBuild {
        manifest,
        palette,
        source_payload,
        payload,
        pixel_fit,
        source_fueen,
        fueen_fit,
        headers,
        bank: bank.ok_or_else(|| "Panotty wah pack was not encoded".to_string())?,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<PanottyWahManifest, String> {
    let path = assets_dir.join("graphics_text/panotty_wah.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read Panotty wah source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid Panotty wah source {}: {error}", path.display()))
}

fn read_master(assets_dir: &Path, manifest: &PanottyWahManifest) -> Result<Vec<u8>, String> {
    read_verified_rgba(
        assets_dir,
        &manifest.master_asset,
        &manifest.master_sha256,
        manifest.master_width,
        manifest.master_height,
        manifest.master_alpha_bounds,
        "Panotty wah master",
    )
}

fn read_font(assets_dir: &Path, manifest: &PanottyWahManifest) -> Result<Font, String> {
    let path = assets_dir.join(&manifest.font_asset);
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let actual = sha256_hex(&bytes);
    if actual != manifest.font_sha256 {
        return Err(format!(
            "{}: Panotty fueen font SHA-256 mismatch: expected {}, got {actual}",
            path.display(),
            manifest.font_sha256
        ));
    }
    Font::from_bytes(bytes, FontSettings::default())
        .map_err(|error| format!("failed to parse Panotty fueen font: {error}"))
}

fn validate_manifest_shape(manifest: &PanottyWahManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-BATTLE-PANOTTY-WAH"
        || !manifest.source_policy.contains("JP")
        || manifest.font_asset != "neodgm.ttf"
        || manifest.font_sha256.len() != 64
    {
        return Err("unsupported Panotty wah manifest identity".to_string());
    }
    let surface = manifest.output_surface;
    if surface.width != 80
        || surface.height != 40
        || surface.content_box
            != (PixelBounds {
                x: 0,
                y: 8,
                width: 80,
                height: 32,
            })
        || surface.alpha_threshold == 0
        || surface.alpha_threshold == u8::MAX
    {
        return Err("Panotty wah output surface drifted from the JP sparse layout".to_string());
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
        return Err("Panotty wah palette admission is invalid".to_string());
    }
    if manifest
        .source_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        != BTreeSet::from([0, 7, 11, 13])
    {
        return Err("Panotty wah JP source palette roles drifted".to_string());
    }
    if manifest.mutable_tile_range.start != PANOTTY_WAH_MUTABLE_TILE_START
        || manifest.mutable_tile_range.end_exclusive != PANOTTY_WAH_MUTABLE_TILE_END
        || PANOTTY_WAH_MUTABLE_TILE_END * MD_TILE_BYTES != PANOTTY_WAH_SOURCE_BYTES
    {
        return Err("Panotty wah mutable range drifted from the final 44 JP tiles".to_string());
    }
    validate_fueen_manifest(manifest)?;
    validate_tile_layout(manifest)?;

    let expected_packs = [
        (
            "panotty-wah-enemy",
            PANOTTY_WAH_HEADER_OFFSETS[0],
            PANOTTY_WAH_VRAM_DESTINATIONS[0],
        ),
        (
            "panotty-wah-amigo-initial",
            PANOTTY_WAH_HEADER_OFFSETS[1],
            PANOTTY_WAH_VRAM_DESTINATIONS[1],
        ),
        (
            "panotty-wah-amigo-reload",
            PANOTTY_WAH_HEADER_OFFSETS[2],
            PANOTTY_WAH_VRAM_DESTINATIONS[2],
        ),
    ];
    if manifest.source_packs.len() != expected_packs.len() {
        return Err("Panotty wah manifest must declare all three JP consumers".to_string());
    }
    for (pack, (id, header, vram)) in manifest.source_packs.iter().zip(expected_packs) {
        if pack.id != id
            || parse_hex(&pack.header_offset)? != header
            || parse_u16_hex(&pack.vram_destination)? != vram
            || pack.decoded_bytes != PANOTTY_WAH_SOURCE_BYTES
        {
            return Err(format!(
                "Panotty wah source declaration {} drifted from its JP consumer",
                pack.id
            ));
        }
    }
    Ok(())
}

fn validate_fueen_manifest(manifest: &PanottyWahManifest) -> Result<(), String> {
    let fueen = &manifest.fueen;
    if fueen.id != "GFX-BATTLE-PANOTTY-FUEEN"
        || fueen.jp_text != "ふえ〜ん"
        || fueen.ko != "흐엥"
        || fueen.tile_range.start != PANOTTY_FUEEN_TILE_START
        || fueen.tile_range.end_exclusive != PANOTTY_FUEEN_TILE_END
        || fueen.source_tile_sha256.len() != 64
    {
        return Err("Panotty fueen identity or source range drifted".to_string());
    }
    let surface = &fueen.output_surface;
    if surface.width != PANOTTY_FUEEN_WIDTH
        || surface.height != PANOTTY_FUEEN_HEIGHT
        || surface.glyph_size != 16
        || surface.cluster_boxes
            != [
                PixelBounds {
                    x: 0,
                    y: 0,
                    width: 32,
                    height: 16,
                },
                PixelBounds {
                    x: 32,
                    y: 0,
                    width: 32,
                    height: 16,
                },
            ]
    {
        return Err("Panotty fueen output clusters drifted from the JP sprites".to_string());
    }
    let source_roles = fueen
        .source_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if source_roles != BTreeSet::from([0, 7, 11, 13])
        || fueen.target_palette_indices != [13, 11, 7]
        || fueen
            .target_palette_indices
            .iter()
            .any(|&index| index >= 16 || index == manifest.transparent_palette_index)
    {
        return Err("Panotty fueen palette roles drifted".to_string());
    }
    if fueen.tile_layout.len() != PANOTTY_FUEEN_LAYOUT.len()
        || fueen
            .tile_layout
            .iter()
            .zip(PANOTTY_FUEEN_LAYOUT)
            .any(|(actual, expected)| actual.as_slice() != expected)
    {
        return Err("Panotty fueen tile layout drifted from the JP structure".to_string());
    }

    let consumer = &fueen.enemy_sprite_consumer;
    if parse_hex(&consumer.offset_table_base)? != PANOTTY_FUEEN_ENEMY_TABLE_BASE
        || parse_hex(&consumer.offset_table_entry)? != PANOTTY_FUEEN_ENEMY_TABLE_ENTRY
        || parse_hex(&consumer.definition_offset)? != PANOTTY_FUEEN_ENEMY_DEFINITION
        || consumer.record_count != 4
        || parse_u16_hex(&consumer.y)? != 0x0078
        || parse_u16_hex(&consumer.size_link)? != 0x0504
        || parse_u16_hex(&consumer.first_tile)? != 0x64A8
        || consumer.tile_step != 4
        || consumer
            .x_positions
            .iter()
            .map(|value| parse_u16_hex(value))
            .collect::<Result<Vec<_>, _>>()?
            != [0x0050, 0x0060, 0x0090, 0x00A0]
    {
        return Err("Panotty fueen enemy consumer declaration drifted".to_string());
    }
    Ok(())
}

fn validate_tile_layout(manifest: &PanottyWahManifest) -> Result<(), String> {
    if manifest.tile_layout.len() != PANOTTY_WAH_LAYOUT.len() {
        return Err("Panotty wah tile layout must have five rows".to_string());
    }
    let mut mapped = BTreeSet::new();
    for (row, expected_row) in manifest.tile_layout.iter().zip(PANOTTY_WAH_LAYOUT) {
        if row.len() != expected_row.len() {
            return Err("Panotty wah tile layout must have ten columns".to_string());
        }
        for (&actual, expected) in row.iter().zip(expected_row) {
            let expected = usize::try_from(expected).ok();
            if actual != expected {
                return Err("Panotty wah tile layout drifted from the JP structure".to_string());
            }
            if let Some(tile) = actual {
                mapped.insert(tile);
            }
        }
    }
    if mapped != (0..PANOTTY_WAH_MUTABLE_TILES).collect::<BTreeSet<_>>() {
        return Err("Panotty wah tile layout does not cover each owned tile once".to_string());
    }
    Ok(())
}

fn validate_fueen_source(tiles: &[u8], manifest: &PanottyWahManifest) -> Result<(), String> {
    if tiles.len() != PANOTTY_FUEEN_BYTES {
        return Err("Panotty fueen JP tile payload length is invalid".to_string());
    }
    let actual_hash = sha256_hex(tiles);
    if actual_hash != manifest.fueen.source_tile_sha256 {
        return Err(format!(
            "Panotty fueen JP tile SHA-256 mismatch: expected {}, got {actual_hash}",
            manifest.fueen.source_tile_sha256
        ));
    }
    let pixels = decode_md_tiles_column_major(
        tiles,
        PANOTTY_FUEEN_WIDTH,
        PANOTTY_FUEEN_HEIGHT,
        "Panotty fueen JP effect",
    )?;
    let actual_roles = pixels
        .iter()
        .map(|&pixel| pixel as usize)
        .collect::<BTreeSet<_>>();
    let expected_roles = manifest
        .fueen
        .source_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual_roles != expected_roles {
        return Err(format!(
            "Panotty fueen JP palette drifted: expected {expected_roles:?}, got {actual_roles:?}"
        ));
    }
    Ok(())
}

fn render_fueen_surface(font: &Font, manifest: &PanottyWahManifest) -> Result<Vec<u8>, String> {
    let fueen = &manifest.fueen;
    let surface = &fueen.output_surface;
    let chars = fueen.ko.chars().collect::<Vec<_>>();
    if chars.len() != surface.cluster_boxes.len() {
        return Err(format!(
            "Panotty fueen needs one Korean glyph per JP cluster, got {} glyphs",
            chars.len()
        ));
    }

    let mut core = vec![false; surface.width * surface.height];
    for (ch, bounds) in chars.into_iter().zip(&surface.cluster_boxes) {
        if bounds.width < surface.glyph_size || bounds.height < surface.glyph_size {
            return Err("Panotty fueen cluster cannot contain a native glyph".to_string());
        }
        let glyph = jp_native::render_native_glyph(font, ch);
        let left = bounds.x + (bounds.width - surface.glyph_size) / 2;
        let top = bounds.y + (bounds.height - surface.glyph_size) / 2;
        for y in 0..surface.glyph_size {
            for x in 0..surface.glyph_size {
                if native_glyph_pixel(&glyph, x, y) {
                    core[(top + y) * surface.width + left + x] = true;
                }
            }
        }
    }

    let mut pixels = vec![manifest.transparent_palette_index as u8; surface.width * surface.height];
    for y in 0..surface.height {
        for x in 0..surface.width {
            let palette = if core[y * surface.width + x] {
                Some(fueen.target_palette_indices[0])
            } else {
                match nearest_core_distance(&core, surface.width, surface.height, x, y, 2) {
                    Some(1) => Some(fueen.target_palette_indices[1]),
                    Some(2) => Some(fueen.target_palette_indices[2]),
                    _ => None,
                }
            };
            if let Some(palette) = palette {
                pixels[y * surface.width + x] = palette as u8;
            }
        }
    }
    let actual_roles = pixels
        .iter()
        .map(|&pixel| pixel as usize)
        .collect::<BTreeSet<_>>();
    let expected_roles = std::iter::once(manifest.transparent_palette_index)
        .chain(fueen.target_palette_indices.iter().copied())
        .collect::<BTreeSet<_>>();
    if actual_roles != expected_roles {
        return Err(format!(
            "Panotty fueen Korean palette roles are incomplete: expected {expected_roles:?}, got {actual_roles:?}"
        ));
    }
    Ok(pixels)
}

fn validate_fueen_enemy_consumer(
    source: &[u8],
    manifest: &PanottyWahManifest,
) -> Result<(), String> {
    let consumer = &manifest.fueen.enemy_sprite_consumer;
    let table_base = parse_hex(&consumer.offset_table_base)?;
    let table_entry = parse_hex(&consumer.offset_table_entry)?;
    let definition = parse_hex(&consumer.definition_offset)?;
    let relative_bytes = source_range(source, table_entry, 2, "Panotty fueen offset table entry")?;
    let relative = u16::from_be_bytes([relative_bytes[0], relative_bytes[1]]) as usize;
    if table_base + relative != definition {
        return Err(format!(
            "Panotty fueen table resolves to 0x{:06X}, expected 0x{definition:06X}",
            table_base + relative
        ));
    }

    let definition_bytes = source_range(
        source,
        definition,
        2 + consumer.record_count * 8,
        "Panotty fueen enemy sprite definition",
    )?;
    let record_count = u16::from_be_bytes([definition_bytes[0], definition_bytes[1]]) as usize;
    if record_count != consumer.record_count {
        return Err(format!(
            "Panotty fueen enemy definition has {record_count} records, expected {}",
            consumer.record_count
        ));
    }
    let expected_y = parse_u16_hex(&consumer.y)?;
    let expected_size_link = parse_u16_hex(&consumer.size_link)?;
    let first_tile = parse_u16_hex(&consumer.first_tile)?;
    let expected_x = consumer
        .x_positions
        .iter()
        .map(|value| parse_u16_hex(value))
        .collect::<Result<Vec<_>, _>>()?;
    for (index, record) in definition_bytes[2..].chunks_exact(8).enumerate() {
        let y = u16::from_be_bytes([record[0], record[1]]);
        let size_link = u16::from_be_bytes([record[2], record[3]]);
        let tile = u16::from_be_bytes([record[4], record[5]]);
        let x = u16::from_be_bytes([record[6], record[7]]);
        let expected_tile = first_tile
            .checked_add(
                u16::try_from(index * consumer.tile_step)
                    .map_err(|_| "Panotty fueen tile step overflowed".to_string())?,
            )
            .ok_or_else(|| "Panotty fueen tile attribute overflowed".to_string())?;
        if y != expected_y
            || size_link != expected_size_link
            || tile != expected_tile
            || x != expected_x[index]
        {
            return Err(format!(
                "Panotty fueen enemy sprite record {index} drifted: y=0x{y:04X}, size/link=0x{size_link:04X}, tile=0x{tile:04X}, x=0x{x:04X}"
            ));
        }
    }
    let vram_base_tile = usize::from(PANOTTY_WAH_VRAM_DESTINATIONS[0]) / MD_TILE_BYTES;
    let owned_start = usize::from(first_tile & 0x07FF)
        .checked_sub(vram_base_tile)
        .ok_or_else(|| "Panotty fueen first sprite tile precedes its VRAM pack".to_string())?;
    let owned_end = owned_start + consumer.record_count * consumer.tile_step;
    if owned_start != PANOTTY_FUEEN_TILE_START || owned_end != PANOTTY_FUEEN_TILE_END {
        return Err(format!(
            "Panotty fueen enemy sprites own tiles {owned_start}..{owned_end}, expected {PANOTTY_FUEEN_TILE_START}..{PANOTTY_FUEEN_TILE_END}"
        ));
    }
    Ok(())
}

fn reduce_master_to_surface(
    master: &[u8],
    manifest: &PanottyWahManifest,
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
        "Panotty wah",
    )
}

fn encode_layout_surface(pixels: &[u8], manifest: &PanottyWahManifest) -> Result<Vec<u8>, String> {
    let surface = manifest.output_surface;
    if pixels.len() != surface.width * surface.height {
        return Err("Panotty wah pixel surface length is invalid".to_string());
    }
    let mut output = vec![0u8; PANOTTY_WAH_MUTABLE_BYTES];
    for (tile_y, row) in manifest.tile_layout.iter().enumerate() {
        for (tile_x, &tile) in row.iter().enumerate() {
            if let Some(tile) = tile {
                encode_surface_tile(
                    pixels,
                    surface.width,
                    tile_x * 8,
                    tile_y * 8,
                    &mut output[tile * MD_TILE_BYTES..(tile + 1) * MD_TILE_BYTES],
                )?;
            } else {
                for y in tile_y * 8..(tile_y + 1) * 8 {
                    for x in tile_x * 8..(tile_x + 1) * 8 {
                        if pixels[y * surface.width + x] as usize
                            != manifest.transparent_palette_index
                        {
                            return Err(format!(
                                "Panotty wah has an opaque pixel in unmapped cell ({tile_x},{tile_y})"
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(output)
}

fn encode_surface_tile(
    pixels: &[u8],
    surface_width: usize,
    x: usize,
    y: usize,
    output: &mut [u8],
) -> Result<(), String> {
    if output.len() != MD_TILE_BYTES {
        return Err("Panotty wah tile output has the wrong length".to_string());
    }
    for local_y in 0..8 {
        for pair_x in 0..4 {
            let left = pixels[(y + local_y) * surface_width + x + pair_x * 2];
            let right = pixels[(y + local_y) * surface_width + x + pair_x * 2 + 1];
            if left >= 16 || right >= 16 {
                return Err("Panotty wah uses an invalid palette index".to_string());
            }
            output[local_y * 4 + pair_x] = (left << 4) | right;
        }
    }
    Ok(())
}

fn decode_layout_surface(tiles: &[u8], manifest: &PanottyWahManifest) -> Result<Vec<u8>, String> {
    if tiles.len() != PANOTTY_WAH_MUTABLE_BYTES {
        return Err("Panotty wah tile payload length is invalid".to_string());
    }
    let surface = manifest.output_surface;
    let mut pixels = vec![manifest.transparent_palette_index as u8; surface.width * surface.height];
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
                    pixels[y * surface.width + x] = pixel;
                }
            }
        }
    }
    Ok(pixels)
}

fn validate_source_effect_tiles(tiles: &[u8], manifest: &PanottyWahManifest) -> Result<(), String> {
    let pixels = decode_layout_surface(tiles, manifest)?;
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
            "Panotty wah JP effect palette drifted: expected {expected:?}, got {actual:?}"
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
    let mut probe = vec![0u8; PANOTTY_WAH_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[PANOTTY_WAH_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != expected_vram || decoded.data != expected_payload {
        return Err("Panotty wah mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn summary(build: &PanottyWahBuild, checksum: u16) -> PanottyWahSummary {
    debug_assert_eq!(build.source_payload.len(), build.payload.len());
    PanottyWahSummary {
        source_tiles: PANOTTY_WAH_SOURCE_TILES,
        rewritten_tiles: PANOTTY_FUEEN_TILES + PANOTTY_WAH_MUTABLE_TILES,
        fueen_rewritten_tiles: PANOTTY_FUEEN_TILES,
        wah_rewritten_tiles: PANOTTY_WAH_MUTABLE_TILES,
        fueen_visible_pixels: build
            .fueen_fit
            .iter()
            .filter(|&&pixel| pixel as usize != build.manifest.transparent_palette_index)
            .count(),
        protected_bytes: PANOTTY_FUEEN_TILE_START * MD_TILE_BYTES,
        consumer_headers: build.headers.len(),
        pack_bytes: build.bank.len(),
        checksum,
    }
}
