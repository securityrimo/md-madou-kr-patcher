//! JP-source bad-ending `ガーン` -> `뜨악` effect compiler.
//!
//! The JP bad-ending group contains three mode-1 transfers. The active text
//! lives in the 21,152-byte high-pattern transfer. Its bytes remain untouched;
//! the Korean 160x32 effect is appended as eighty tiles and a source-checked
//! sprite definition is redirected to those new tiles.

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
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const BAD_END_HEADER_OFFSETS: [usize; 3] = [0x09_61C0, 0x09_61C6, 0x09_61CC];
const BAD_END_VRAM_DESTINATIONS: [u16; 3] = [0x2000, 0x4000, 0xE000];
const BAD_END_DECODED_BYTES: [usize; 3] = [448, 21_152, 8_192];
const BAD_END_TARGET_HEADER_OFFSET: usize = BAD_END_HEADER_OFFSETS[1];
const BAD_END_TARGET_VRAM: u16 = BAD_END_VRAM_DESTINATIONS[1];
const BAD_END_SOURCE_BYTES: usize = BAD_END_DECODED_BYTES[1];
const BAD_END_SOURCE_TILES: usize = BAD_END_SOURCE_BYTES / MD_TILE_BYTES;
const BAD_END_BANK_OFFSET: usize = 0x29_8000;
const BAD_END_BANK_LIMIT: usize = 0x2A_0000;
const BAD_END_SPRITE_DEFINITION_OFFSET: usize = 0x09_77E6;
const BAD_END_SOURCE_SPRITES: usize = 8;
const BAD_END_TARGET_SPRITES: usize = 5;
const BAD_END_SURFACE_WIDTH: usize = 160;
const BAD_END_SURFACE_HEIGHT: usize = 32;
const BAD_END_EFFECT_TILES: usize = BAD_END_SURFACE_WIDTH / 8 * (BAD_END_SURFACE_HEIGHT / 8);
const PREVIEW_SCALE: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadEndGaanSummary {
    pub source_tiles: usize,
    pub appended_tiles: usize,
    pub protected_bytes: usize,
    pub companion_transfers: usize,
    pub target_sprites: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct BadEndGaanManifest {
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
    source_effect_palette_indices: Vec<usize>,
    allowed_opaque_palette_indices: Vec<usize>,
    source_packs: Vec<SourcePack>,
    target_pack_id: String,
    consumer_loader: ConsumerLoader,
    sprite_offset_binding: SpriteOffsetBinding,
    source_sprite_definition: SourceSpriteDefinition,
    target_sprite_layout: TargetSpriteLayout,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
    content_box: PixelBounds,
    alpha_threshold: u8,
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
struct SourceSpriteDefinition {
    offset: String,
    sprites: Vec<SpriteRecord>,
}

#[derive(Debug, Deserialize)]
struct ConsumerLoader {
    lea_offset: String,
    header_group_offset: String,
    loader_call_target: String,
}

#[derive(Debug, Deserialize)]
struct SpriteOffsetBinding {
    pointer_offset: String,
    table_offset: String,
    subtype_index: usize,
    relative_offset: String,
}

#[derive(Debug, Deserialize)]
struct SpriteRecord {
    y: String,
    size: String,
    tile: String,
    x: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TargetSpriteLayout {
    count: usize,
    base_x: String,
    base_y: String,
    x_step: usize,
    width_tiles: usize,
    height_tiles: usize,
    tile_attribute_flags: String,
}

#[derive(Debug)]
struct BadEndGaanBuild {
    manifest: BadEndGaanManifest,
    palette: [u16; 16],
    source_payloads: Vec<Vec<u8>>,
    payload: Vec<u8>,
    surface: Vec<u8>,
    header: [u8; 6],
    sprite_definition: Vec<u8>,
    bank: Vec<u8>,
}

/// Insert the Korean bad-ending effect into the cumulative JP-to-KR ROM.
pub fn apply_bad_end_gaan(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<BadEndGaanSummary, String> {
    let build = build_bad_end_gaan(source, assets_dir)?;
    let bank_end = BAD_END_BANK_OFFSET + build.bank.len();
    if bank_end > BAD_END_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "bad-ending effect pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(4);
    apply_expected_write(
        output,
        BAD_END_TARGET_HEADER_OFFSET,
        source_range(
            source,
            BAD_END_TARGET_HEADER_OFFSET,
            build.header.len(),
            "bad-ending source high-pattern header",
        )?,
        &build.header,
        "bad-ending high-pattern header",
    )?;
    changed_ranges.push((
        BAD_END_TARGET_HEADER_OFFSET,
        BAD_END_TARGET_HEADER_OFFSET + build.header.len(),
    ));

    let sprite_offset = parse_hex(&build.manifest.source_sprite_definition.offset)?;
    apply_expected_write(
        output,
        sprite_offset,
        source_range(
            source,
            sprite_offset,
            build.sprite_definition.len(),
            "bad-ending source sprite prefix",
        )?,
        &build.sprite_definition,
        "bad-ending Korean sprite definition",
    )?;
    changed_ranges.push((sprite_offset, sprite_offset + build.sprite_definition.len()));

    apply_expected_write(
        output,
        BAD_END_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "bad-ending expanded pattern pack",
    )?;
    changed_ranges.push((BAD_END_BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after bad-ending graphics",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    for (index, declaration) in build.manifest.source_packs.iter().enumerate() {
        let header_offset = parse_hex(&declaration.header_offset)?;
        let inserted = decode_mode1_pack_entry(output, header_offset)?;
        let expected = if declaration.id == build.manifest.target_pack_id {
            &build.payload
        } else {
            &build.source_payloads[index]
        };
        if inserted.vram_destination != parse_u16_hex(&declaration.vram_destination)?
            || inserted.data != *expected
        {
            return Err(format!(
                "inserted bad-ending group does not match transfer {}",
                declaration.id
            ));
        }
    }

    eprintln!("JP graphics GFX-BAD-END-GAAN Expected Writes:");
    eprintln!(
        "  0x{BAD_END_TARGET_HEADER_OFFSET:06X}..0x{:06X}  bad-ending high-pattern header ({} bytes)",
        BAD_END_TARGET_HEADER_OFFSET + build.header.len(),
        build.header.len()
    );
    eprintln!(
        "  0x{sprite_offset:06X}..0x{:06X}  bad-ending Korean sprite definition ({} bytes)",
        sprite_offset + build.sprite_definition.len(),
        build.sprite_definition.len()
    );
    eprintln!(
        "  0x{BAD_END_BANK_OFFSET:06X}..0x{bank_end:06X}  bad-ending pattern pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render the exact 160x32 indexed surface consumed by the five sprites.
pub fn write_bad_end_gaan_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<BadEndGaanSummary, String> {
    let build = build_bad_end_gaan(source, assets_dir)?;
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
        "bad-ending effect",
    )?;
    Ok(summary(&build, 0))
}

fn build_bad_end_gaan(source: &[u8], assets_dir: &Path) -> Result<BadEndGaanBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "bad-ending effect")?;
    let master = read_master(assets_dir, &manifest)?;
    let surface = reduce_master_to_surface(&master, &manifest, &palette)?;
    let effect_tiles = encode_md_tiles_column_major(
        &surface,
        BAD_END_SURFACE_WIDTH,
        BAD_END_SURFACE_HEIGHT,
        "bad-ending effect",
    )?;
    if effect_tiles.len() != BAD_END_EFFECT_TILES * MD_TILE_BYTES {
        return Err("bad-ending effect tile count drifted".to_string());
    }

    let mut source_payloads = Vec::with_capacity(manifest.source_packs.len());
    for declaration in &manifest.source_packs {
        source_payloads.push(checked_source_pack(source, declaration)?);
    }
    let target_index = manifest
        .source_packs
        .iter()
        .position(|pack| pack.id == manifest.target_pack_id)
        .ok_or_else(|| "bad-ending target pack is absent".to_string())?;
    let source_payload = &source_payloads[target_index];
    if source_payload.len() != BAD_END_SOURCE_BYTES {
        return Err("bad-ending target payload length drifted".to_string());
    }
    let mut payload = source_payload.clone();
    payload.extend_from_slice(&effect_tiles);
    if payload[..source_payload.len()] != source_payload[..] {
        return Err("bad-ending compiler changed protected JP pattern bytes".to_string());
    }

    validate_source_sprite_definition(source, &manifest)?;
    validate_consumer_bindings(source, &manifest)?;
    let sprite_definition = build_target_sprite_definition(&manifest, source_payload.len())?;
    let encoded = encode_locked_mode1_pack(BAD_END_BANK_OFFSET, BAD_END_TARGET_VRAM, &payload)?;
    validate_pack_roundtrip(
        &encoded.header,
        &encoded.bank,
        BAD_END_TARGET_VRAM,
        &payload,
    )?;

    Ok(BadEndGaanBuild {
        manifest,
        palette,
        source_payloads,
        payload,
        surface,
        header: encoded.header,
        sprite_definition,
        bank: encoded.bank,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<BadEndGaanManifest, String> {
    let path = assets_dir.join("graphics_text/bad_end_gaan.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read bad-ending graphics source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid bad-ending source {}: {error}", path.display()))
}

fn read_master(assets_dir: &Path, manifest: &BadEndGaanManifest) -> Result<Vec<u8>, String> {
    read_verified_rgba(
        assets_dir,
        &manifest.master_asset,
        &manifest.master_sha256,
        manifest.master_width,
        manifest.master_height,
        manifest.master_alpha_bounds,
        "bad-ending master",
    )
}

fn reduce_master_to_surface(
    master: &[u8],
    manifest: &BadEndGaanManifest,
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
        "bad-ending effect",
    )
}

fn validate_manifest_shape(manifest: &BadEndGaanManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-BAD-END-GAAN"
        || !manifest.source_policy.contains("JP")
    {
        return Err("unsupported bad-ending manifest identity".to_string());
    }
    let surface = manifest.output_surface;
    if surface.width != BAD_END_SURFACE_WIDTH
        || surface.height != BAD_END_SURFACE_HEIGHT
        || surface.content_box
            != (PixelBounds {
                x: 0,
                y: 0,
                width: BAD_END_SURFACE_WIDTH,
                height: BAD_END_SURFACE_HEIGHT,
            })
        || surface.alpha_threshold == 0
        || surface.alpha_threshold == u8::MAX
    {
        return Err("bad-ending output surface drifted".to_string());
    }
    if manifest.transparent_palette_index != 0
        || manifest
            .source_effect_palette_indices
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != BTreeSet::from([0, 1, 5, 12, 15])
        || manifest
            .allowed_opaque_palette_indices
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != BTreeSet::from([1, 5, 12, 15])
    {
        return Err("bad-ending palette roles drifted".to_string());
    }
    let anchors = manifest
        .source_packs
        .iter()
        .map(|pack| {
            Ok((
                parse_hex(&pack.header_offset)?,
                parse_u16_hex(&pack.vram_destination)?,
                pack.decoded_bytes,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let expected = BAD_END_HEADER_OFFSETS
        .into_iter()
        .zip(BAD_END_VRAM_DESTINATIONS)
        .zip(BAD_END_DECODED_BYTES)
        .map(|((header, vram), bytes)| (header, vram, bytes))
        .collect::<Vec<_>>();
    if anchors != expected
        || manifest.target_pack_id != "bad-end-patterns-high"
        || manifest.source_packs[1].id != manifest.target_pack_id
    {
        return Err("bad-ending JP transfer set drifted".to_string());
    }
    if parse_hex(&manifest.source_sprite_definition.offset)? != BAD_END_SPRITE_DEFINITION_OFFSET
        || manifest.source_sprite_definition.sprites.len() != BAD_END_SOURCE_SPRITES
    {
        return Err("bad-ending source sprite definition drifted".to_string());
    }
    if parse_hex(&manifest.consumer_loader.lea_offset)? != 0x09_43C6
        || parse_hex(&manifest.consumer_loader.header_group_offset)? != BAD_END_HEADER_OFFSETS[0]
        || parse_hex(&manifest.consumer_loader.loader_call_target)? != 0x000A72
        || parse_hex(&manifest.sprite_offset_binding.pointer_offset)? != 0x06_AFE8
        || parse_hex(&manifest.sprite_offset_binding.table_offset)? != 0x09_770C
        || manifest.sprite_offset_binding.subtype_index != 3
        || parse_u16_hex(&manifest.sprite_offset_binding.relative_offset)? != 0x00DA
    {
        return Err("bad-ending consumer binding declarations drifted".to_string());
    }
    let layout = &manifest.target_sprite_layout;
    if layout.count != BAD_END_TARGET_SPRITES
        || parse_u16_hex(&layout.base_x)? != 0x00D0
        || parse_u16_hex(&layout.base_y)? != 0x00C0
        || layout.x_step != 32
        || layout.width_tiles != 4
        || layout.height_tiles != 4
        || parse_u16_hex(&layout.tile_attribute_flags)? != 0x4000
    {
        return Err("bad-ending target sprite layout drifted".to_string());
    }
    Ok(())
}

fn checked_source_pack(source: &[u8], declaration: &SourcePack) -> Result<Vec<u8>, String> {
    let header_offset = parse_hex(&declaration.header_offset)?;
    let decoded = decode_mode1_pack_entry(source, header_offset)?;
    let expected_vram = parse_u16_hex(&declaration.vram_destination)?;
    if decoded.vram_destination != expected_vram {
        return Err(format!(
            "bad-ending transfer {} targets VRAM 0x{:04X}, expected 0x{expected_vram:04X}",
            declaration.id, decoded.vram_destination
        ));
    }
    if decoded.data.len() != declaration.decoded_bytes {
        return Err(format!(
            "bad-ending transfer {} decoded to {} bytes, expected {}",
            declaration.id,
            decoded.data.len(),
            declaration.decoded_bytes
        ));
    }
    let hash = sha256_hex(&decoded.data);
    if hash != declaration.decoded_sha256 {
        return Err(format!(
            "bad-ending transfer {} SHA-256 mismatch: expected {}, got {hash}",
            declaration.id, declaration.decoded_sha256
        ));
    }
    Ok(decoded.data)
}

fn validate_source_sprite_definition(
    source: &[u8],
    manifest: &BadEndGaanManifest,
) -> Result<(), String> {
    let expected = encode_source_sprite_definition(manifest)?;
    let offset = parse_hex(&manifest.source_sprite_definition.offset)?;
    let actual = source_range(
        source,
        offset,
        expected.len(),
        "bad-ending source sprite definition",
    )?;
    if actual != expected {
        return Err("bad-ending source sprite definition no longer matches JP ROM".to_string());
    }
    Ok(())
}

fn validate_consumer_bindings(source: &[u8], manifest: &BadEndGaanManifest) -> Result<(), String> {
    let loader = &manifest.consumer_loader;
    let lea_offset = parse_hex(&loader.lea_offset)?;
    let lea = source_range(source, lea_offset, 10, "bad-ending graphics loader")?;
    if u16::from_be_bytes([lea[0], lea[1]]) != 0x45FA {
        return Err(
            "bad-ending consumer no longer loads its group with LEA d16(PC),A2".to_string(),
        );
    }
    let displacement = i16::from_be_bytes([lea[2], lea[3]]) as isize;
    let target = (lea_offset as isize + 2 + displacement) as usize;
    if target != parse_hex(&loader.header_group_offset)? {
        return Err(format!(
            "bad-ending loader resolves to 0x{target:06X}, not its declared header group"
        ));
    }
    if u16::from_be_bytes([lea[4], lea[5]]) != 0x4EB9 {
        return Err("bad-ending group load is not followed by JSR abs.l".to_string());
    }
    let call_target = u32::from_be_bytes([lea[6], lea[7], lea[8], lea[9]]) as usize;
    if call_target != parse_hex(&loader.loader_call_target)? {
        return Err(format!(
            "bad-ending graphics loader call drifted to 0x{call_target:06X}"
        ));
    }

    let binding = &manifest.sprite_offset_binding;
    let pointer_offset = parse_hex(&binding.pointer_offset)?;
    let pointer = source_range(source, pointer_offset, 4, "bad-ending sprite table pointer")?;
    let table = u32::from_be_bytes([pointer[0], pointer[1], pointer[2], pointer[3]]) as usize;
    if table != parse_hex(&binding.table_offset)? {
        return Err(format!(
            "bad-ending sprite table pointer resolves to 0x{table:06X}"
        ));
    }
    let relative_offset_address = table + binding.subtype_index * 2;
    let relative = source_range(
        source,
        relative_offset_address,
        2,
        "bad-ending subtype sprite offset",
    )?;
    let relative = u16::from_be_bytes([relative[0], relative[1]]);
    if relative != parse_u16_hex(&binding.relative_offset)?
        || table + usize::from(relative) != parse_hex(&manifest.source_sprite_definition.offset)?
    {
        return Err(
            "bad-ending subtype 3 no longer owns the declared sprite definition".to_string(),
        );
    }
    Ok(())
}

fn encode_source_sprite_definition(manifest: &BadEndGaanManifest) -> Result<Vec<u8>, String> {
    let sprites = &manifest.source_sprite_definition.sprites;
    let mut output = Vec::with_capacity(2 + sprites.len() * 8);
    append_word(
        &mut output,
        u16::try_from(sprites.len())
            .map_err(|_| "bad-ending source sprite count is too large".to_string())?,
    );
    for sprite in sprites {
        append_word(&mut output, parse_u16_hex(&sprite.y)?);
        append_word(&mut output, parse_u16_hex(&sprite.size)?);
        append_word(&mut output, parse_u16_hex(&sprite.tile)?);
        append_word(&mut output, parse_u16_hex(&sprite.x)?);
    }
    Ok(output)
}

fn build_target_sprite_definition(
    manifest: &BadEndGaanManifest,
    source_payload_bytes: usize,
) -> Result<Vec<u8>, String> {
    if !source_payload_bytes.is_multiple_of(MD_TILE_BYTES) {
        return Err("bad-ending source payload is not tile-aligned".to_string());
    }
    let layout = &manifest.target_sprite_layout;
    let base_x = parse_u16_hex(&layout.base_x)?;
    let base_y = parse_u16_hex(&layout.base_y)?;
    let flags = parse_u16_hex(&layout.tile_attribute_flags)?;
    if flags & 0x07FF != 0 {
        return Err("bad-ending tile flags overlap the tile index".to_string());
    }
    let vram_base_tile = usize::from(BAD_END_TARGET_VRAM) / MD_TILE_BYTES;
    let appended_base_tile = vram_base_tile + source_payload_bytes / MD_TILE_BYTES;
    let sprite_tiles = layout.width_tiles * layout.height_tiles;
    let size = sprite_size_word(layout.width_tiles, layout.height_tiles)?;
    let mut output = Vec::with_capacity(2 + layout.count * 8);
    append_word(
        &mut output,
        u16::try_from(layout.count)
            .map_err(|_| "bad-ending target sprite count is too large".to_string())?,
    );
    for index in 0..layout.count {
        let tile = appended_base_tile + index * sprite_tiles;
        if tile > 0x07FF {
            return Err("bad-ending target tile exceeds the VDP tile index".to_string());
        }
        let x = usize::from(base_x) + index * layout.x_step;
        append_word(&mut output, base_y);
        append_word(&mut output, size);
        append_word(&mut output, flags | tile as u16);
        append_word(
            &mut output,
            u16::try_from(x).map_err(|_| "bad-ending target X exceeds u16".to_string())?,
        );
    }
    Ok(output)
}

fn sprite_size_word(width_tiles: usize, height_tiles: usize) -> Result<u16, String> {
    if !(1..=4).contains(&width_tiles) || !(1..=4).contains(&height_tiles) {
        return Err("Mega Drive sprite dimensions must be 1..=4 tiles".to_string());
    }
    Ok((((width_tiles - 1) as u16) << 10) | (((height_tiles - 1) as u16) << 8))
}

fn append_word(output: &mut Vec<u8>, word: u16) {
    output.extend_from_slice(&word.to_be_bytes());
}

fn validate_pack_roundtrip(
    header: &[u8; 6],
    bank: &[u8],
    expected_vram: u16,
    expected_payload: &[u8],
) -> Result<(), String> {
    let mut probe = vec![0u8; BAD_END_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[BAD_END_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != expected_vram || decoded.data != expected_payload {
        return Err("bad-ending mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn summary(build: &BadEndGaanBuild, checksum: u16) -> BadEndGaanSummary {
    BadEndGaanSummary {
        source_tiles: BAD_END_SOURCE_TILES,
        appended_tiles: BAD_END_EFFECT_TILES,
        protected_bytes: BAD_END_SOURCE_BYTES,
        companion_transfers: build.source_payloads.len() - 1,
        target_sprites: BAD_END_TARGET_SPRITES,
        pack_bytes: build.bank.len(),
        checksum,
    }
}
