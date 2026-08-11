//! JP-source ending-escape door-mark compiler.
//!
//! The original mode-1 pack stores twenty-six decorative tiles followed by
//! three 3x3-tile kana panels: `ひ`, `じ`, and `ば`.  The Korean build keeps
//! the decorative prefix and every consumer structure byte-identical, and
//! replaces only those panels with `ㅎ`, `ㅈ`, and `ㅂ`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::pixel::{
    PixelBounds, decode_md_tiles_column_major, encode_md_tiles_column_major, md_color,
    nearest_palette_index, parse_md_palette, read_verified_rgba, write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const SOURCE_HEADER_OFFSET: usize = 0x07_8534;
const SOURCE_VRAM_DESTINATION: u16 = 0x5C80;
const SOURCE_DECODED_BYTES: usize = 1_696;
const PROTECTED_TILE_END: usize = 26;
const MUTABLE_TILE_END: usize = 53;
const MARK_TILES: usize = 9;
const MARK_WIDTH: usize = 24;
const MARK_HEIGHT: usize = 24;
const MARK_COUNT: usize = 3;
const TARGET_BANK_OFFSET: usize = 0x33_0000;
const TARGET_BANK_LIMIT: usize = 0x33_8000;
const TYPE_POINTER_TABLE_OFFSET: usize = 0x06_AD18;
const TYPE_POINTER_ENTRY_OFFSET: usize = 0x06_AD94;
const OBJECT_TYPE: usize = 0x1F;
const DEFINITION_TABLE_OFFSET: usize = 0x06_721A;
const SPRITE_COUNT: usize = 9;
const SPRITE_DEFINITION_BYTES: usize = 2 + SPRITE_COUNT * 8;
const PREVIEW_SCALE: usize = 10;
const PREVIEW_GAP: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscapeDoorsSummary {
    pub source_tiles: usize,
    pub rewritten_tiles: usize,
    pub protected_tiles: usize,
    pub marks: usize,
    pub source_sprite_records: usize,
    pub verified_consumer_bytes: usize,
    pub written_executable_bytes: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct EscapeDoorsManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    master_asset: String,
    master_sha256: String,
    master_width: usize,
    master_height: usize,
    master_alpha_bounds: PixelBounds,
    panel_layout: PanelLayout,
    output_surface: OutputSurface,
    palette_line_words: Vec<String>,
    allowed_palette_indices: Vec<usize>,
    source_pack: SourcePack,
    protected_tile_range: TileRange,
    mutable_tile_range: TileRange,
    target_pack_bank: TargetPackBank,
    consumer_binding: ConsumerBinding,
    marks: Vec<MarkDeclaration>,
}

#[derive(Debug, Deserialize)]
struct PanelLayout {
    columns: usize,
    panel_width: usize,
    panel_height: usize,
    ordered_mark_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OutputSurface {
    width: usize,
    height: usize,
}

#[derive(Debug, Deserialize)]
struct SourcePack {
    header_offset: String,
    vram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
}

#[derive(Debug, Deserialize)]
struct TileRange {
    start: usize,
    end_exclusive: usize,
    decoded_sha256: String,
}

#[derive(Debug, Deserialize)]
struct TargetPackBank {
    offset: String,
    limit: String,
    fill_byte: String,
}

#[derive(Debug, Deserialize)]
struct ConsumerBinding {
    type_pointer_table_offset: String,
    type_pointer_entry_offset: String,
    object_type: String,
    definition_table_offset: String,
    definition_table_sha256: String,
    sprite_count: usize,
    records: Vec<SpriteRecord>,
    definitions: Vec<SpriteDefinition>,
}

#[derive(Debug, Deserialize)]
struct SpriteRecord {
    y: String,
    size_link: String,
    tile_words: Vec<String>,
    x: String,
}

#[derive(Debug, Deserialize)]
struct SpriteDefinition {
    mark_id: String,
    relative_offset: String,
    definition_offset: String,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
struct MarkDeclaration {
    id: String,
    jp: String,
    ko: String,
    spell: String,
    compatible_mnemonics: Vec<String>,
    source_tile_start: usize,
    source_sha256: String,
}

#[derive(Debug)]
struct EscapeDoorsBuild {
    manifest: EscapeDoorsManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    source_surfaces: Vec<Vec<u8>>,
    target_surfaces: Vec<Vec<u8>>,
    header: [u8; 6],
    bank: Vec<u8>,
}

/// Insert the three Korean initial-consonant panels into the cumulative ROM.
pub fn apply_escape_doors(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<EscapeDoorsSummary, String> {
    let build = build_escape_doors(source, assets_dir)?;
    let bank_end = TARGET_BANK_OFFSET + build.bank.len();
    if bank_end > TARGET_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "escape-door pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(3);
    apply_expected_write(
        output,
        SOURCE_HEADER_OFFSET,
        source_range(
            source,
            SOURCE_HEADER_OFFSET,
            build.header.len(),
            "escape-door source pack header",
        )?,
        &build.header,
        "escape-door pack header",
    )?;
    changed_ranges.push((
        SOURCE_HEADER_OFFSET,
        SOURCE_HEADER_OFFSET + build.header.len(),
    ));

    let fill = parse_u8_hex(&build.manifest.target_pack_bank.fill_byte)?;
    apply_expected_write(
        output,
        TARGET_BANK_OFFSET,
        &vec![fill; build.bank.len()],
        &build.bank,
        "escape-door expanded pattern pack",
    )?;
    changed_ranges.push((TARGET_BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after escape-door initials",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let inserted = decode_mode1_pack_entry(output, SOURCE_HEADER_OFFSET)?;
    if inserted.vram_destination != SOURCE_VRAM_DESTINATION || inserted.data != build.payload {
        return Err("inserted escape-door pack failed semantic readback".to_string());
    }
    validate_consumer_binding(output, &build.manifest)?;

    eprintln!("JP graphics GFX-ESCAPE-DOOR-INITIALS Expected Writes:");
    eprintln!(
        "  0x{SOURCE_HEADER_OFFSET:06X}..0x{:06X}  source pack header ({} bytes)",
        SOURCE_HEADER_OFFSET + build.header.len(),
        build.header.len()
    );
    eprintln!(
        "  0x{TARGET_BANK_OFFSET:06X}..0x{bank_end:06X}  Korean door-mark pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");
    eprintln!("  executable bytes written: 0");

    Ok(summary(&build, checksum))
}

/// Render the three JP source marks above the three Korean target marks.
pub fn write_escape_doors_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<EscapeDoorsSummary, String> {
    let build = build_escape_doors(source, assets_dir)?;
    let sheet_width = MARK_COUNT * MARK_WIDTH + (MARK_COUNT - 1) * PREVIEW_GAP;
    let sheet_height = MARK_HEIGHT * 2 + PREVIEW_GAP;
    let preview_width = sheet_width * PREVIEW_SCALE;
    let preview_height = sheet_height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];

    for (row, surfaces) in [&build.source_surfaces, &build.target_surfaces]
        .into_iter()
        .enumerate()
    {
        for (column, surface) in surfaces.iter().enumerate() {
            let origin_x = column * (MARK_WIDTH + PREVIEW_GAP);
            let origin_y = row * (MARK_HEIGHT + PREVIEW_GAP);
            for y in 0..MARK_HEIGHT {
                for x in 0..MARK_WIDTH {
                    let palette_index = surface[y * MARK_WIDTH + x] as usize;
                    write_scaled_pixel(
                        &mut rgba,
                        preview_width,
                        origin_x + x,
                        origin_y + y,
                        md_color(build.palette[palette_index]),
                    );
                }
            }
        }
    }

    for y in 0..sheet_height {
        for x in 0..sheet_width {
            let vertical_gap = x % (MARK_WIDTH + PREVIEW_GAP) >= MARK_WIDTH;
            let horizontal_gap = (MARK_HEIGHT..MARK_HEIGHT + PREVIEW_GAP).contains(&y);
            if vertical_gap || horizontal_gap {
                write_scaled_pixel(&mut rgba, preview_width, x, y, [64, 64, 64]);
            }
        }
    }

    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "escape-door initials",
    )?;
    Ok(summary(&build, 0))
}

fn build_escape_doors(source: &[u8], assets_dir: &Path) -> Result<EscapeDoorsBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "escape-door initials")?;
    let source_payload = checked_source_pack(source, &manifest)?;
    validate_source_ranges(&source_payload, &manifest)?;
    validate_consumer_binding(source, &manifest)?;

    let source_surfaces = manifest
        .marks
        .iter()
        .map(|mark| {
            let start = mark.source_tile_start * MD_TILE_BYTES;
            let end = start + MARK_TILES * MD_TILE_BYTES;
            decode_md_tiles_column_major(
                &source_payload[start..end],
                MARK_WIDTH,
                MARK_HEIGHT,
                &format!("{} JP panel", mark.id),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let master = read_verified_rgba(
        assets_dir,
        &manifest.master_asset,
        &manifest.master_sha256,
        manifest.master_width,
        manifest.master_height,
        manifest.master_alpha_bounds,
        "escape-door initials master",
    )?;
    let target_surfaces = (0..MARK_COUNT)
        .map(|panel| reduce_master_panel(&master, panel, &manifest, &palette))
        .collect::<Result<Vec<_>, _>>()?;

    let mut payload = source_payload.clone();
    for (mark, surface) in manifest.marks.iter().zip(&target_surfaces) {
        let encoded = encode_md_tiles_column_major(
            surface,
            MARK_WIDTH,
            MARK_HEIGHT,
            &format!("{} Korean panel", mark.id),
        )?;
        let start = mark.source_tile_start * MD_TILE_BYTES;
        let end = start + encoded.len();
        payload[start..end].copy_from_slice(&encoded);
    }
    let protected_end = PROTECTED_TILE_END * MD_TILE_BYTES;
    if payload[..protected_end] != source_payload[..protected_end] {
        return Err("escape-door compiler changed protected decorative tiles".to_string());
    }
    if payload.len() != source_payload.len() {
        return Err("escape-door compiler changed decoded payload length".to_string());
    }

    let encoded = encode_locked_mode1_pack(TARGET_BANK_OFFSET, SOURCE_VRAM_DESTINATION, &payload)?;
    validate_pack_roundtrip(
        &encoded.header,
        &encoded.bank,
        SOURCE_VRAM_DESTINATION,
        &payload,
    )?;

    Ok(EscapeDoorsBuild {
        manifest,
        palette,
        source_payload,
        payload,
        source_surfaces,
        target_surfaces,
        header: encoded.header,
        bank: encoded.bank,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<EscapeDoorsManifest, String> {
    let path = assets_dir.join("graphics_text/escape_doors.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read escape-door graphics source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid escape-door source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &EscapeDoorsManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-ESCAPE-DOOR-INITIALS"
        || !manifest.source_policy.contains("JP")
        || !manifest
            .source_policy
            .contains("not define a unique answer set")
        || !manifest.source_policy.contains("logic remain")
    {
        return Err("unsupported escape-door manifest identity or policy".to_string());
    }
    if manifest.master_width != 2_172
        || manifest.master_height != 724
        || manifest.master_alpha_bounds
            != (PixelBounds {
                x: 0,
                y: 0,
                width: 2_172,
                height: 724,
            })
        || manifest.panel_layout.columns != MARK_COUNT
        || manifest.panel_layout.panel_width != 724
        || manifest.panel_layout.panel_height != 724
        || manifest.output_surface.width != MARK_WIDTH
        || manifest.output_surface.height != MARK_HEIGHT
    {
        return Err("escape-door master or panel geometry drifted".to_string());
    }
    if manifest
        .allowed_palette_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        != BTreeSet::from([8, 10, 12, 13, 14, 15])
    {
        return Err("escape-door palette roles drifted".to_string());
    }
    if parse_hex(&manifest.source_pack.header_offset)? != SOURCE_HEADER_OFFSET
        || parse_u16_hex(&manifest.source_pack.vram_destination)? != SOURCE_VRAM_DESTINATION
        || manifest.source_pack.decoded_bytes != SOURCE_DECODED_BYTES
        || manifest.protected_tile_range.start != 0
        || manifest.protected_tile_range.end_exclusive != PROTECTED_TILE_END
        || manifest.mutable_tile_range.start != PROTECTED_TILE_END
        || manifest.mutable_tile_range.end_exclusive != MUTABLE_TILE_END
        || parse_hex(&manifest.target_pack_bank.offset)? != TARGET_BANK_OFFSET
        || parse_hex(&manifest.target_pack_bank.limit)? != TARGET_BANK_LIMIT
        || parse_u8_hex(&manifest.target_pack_bank.fill_byte)? != 0xFF
    {
        return Err("escape-door source or target pack declaration drifted".to_string());
    }

    let expected_marks = [
        (
            "GFX-ESCAPE-DOOR-HIDON",
            "ひ",
            "ㅎ",
            "히돈",
            26usize,
            &["히돈"][..],
        ),
        (
            "GFX-ESCAPE-DOOR-JUGEM",
            "じ",
            "ㅈ",
            "쥬겜",
            35usize,
            &["쥬겜"][..],
        ),
        (
            "GFX-ESCAPE-DOOR-BAYOEN",
            "ば",
            "ㅂ",
            "바요엔",
            44usize,
            &["바요엔", "브레인다므드"][..],
        ),
    ];
    if manifest.marks.len() != MARK_COUNT
        || manifest.panel_layout.ordered_mark_ids
            != expected_marks
                .iter()
                .map(|mark| mark.0.to_string())
                .collect::<Vec<_>>()
    {
        return Err("escape-door panel order drifted".to_string());
    }
    for (mark, expected) in manifest.marks.iter().zip(expected_marks) {
        if mark.id != expected.0
            || mark.jp != expected.1
            || mark.ko != expected.2
            || mark.spell != expected.3
            || mark.source_tile_start != expected.4
            || mark
                .compatible_mnemonics
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected.5
        {
            return Err(format!("escape-door mark {} drifted", expected.0));
        }
    }

    let consumer = &manifest.consumer_binding;
    if parse_hex(&consumer.type_pointer_table_offset)? != TYPE_POINTER_TABLE_OFFSET
        || parse_hex(&consumer.type_pointer_entry_offset)? != TYPE_POINTER_ENTRY_OFFSET
        || parse_hex(&consumer.object_type)? != OBJECT_TYPE
        || TYPE_POINTER_TABLE_OFFSET + OBJECT_TYPE * 4 != TYPE_POINTER_ENTRY_OFFSET
        || parse_hex(&consumer.definition_table_offset)? != DEFINITION_TABLE_OFFSET
        || consumer.sprite_count != SPRITE_COUNT
        || consumer.records.len() != SPRITE_COUNT
        || consumer.definitions.len() != MARK_COUNT
    {
        return Err("escape-door consumer declaration drifted".to_string());
    }
    for (index, record) in consumer.records.iter().enumerate() {
        let expected_tile_words = if index == 4 { MARK_COUNT } else { 1 };
        if record.tile_words.len() != expected_tile_words {
            return Err(format!(
                "escape-door sprite record {index} has the wrong tile-word arity"
            ));
        }
    }
    for (index, definition) in consumer.definitions.iter().enumerate() {
        let relative = parse_hex(&definition.relative_offset)?;
        if definition.mark_id != manifest.marks[index].id
            || parse_hex(&definition.definition_offset)? != DEFINITION_TABLE_OFFSET + relative
        {
            return Err(format!(
                "escape-door definition {} no longer resolves through its relative table",
                definition.mark_id
            ));
        }
    }
    Ok(())
}

fn checked_source_pack(source: &[u8], manifest: &EscapeDoorsManifest) -> Result<Vec<u8>, String> {
    let decoded = decode_mode1_pack_entry(source, SOURCE_HEADER_OFFSET)?;
    if decoded.vram_destination != SOURCE_VRAM_DESTINATION
        || decoded.data.len() != SOURCE_DECODED_BYTES
    {
        return Err("escape-door JP pack geometry drifted".to_string());
    }
    let hash = sha256_hex(&decoded.data);
    if hash != manifest.source_pack.decoded_sha256 {
        return Err(format!(
            "escape-door JP pack SHA-256 mismatch: expected {}, got {hash}",
            manifest.source_pack.decoded_sha256
        ));
    }
    Ok(decoded.data)
}

fn validate_source_ranges(
    source_payload: &[u8],
    manifest: &EscapeDoorsManifest,
) -> Result<(), String> {
    let protected_end = PROTECTED_TILE_END * MD_TILE_BYTES;
    if sha256_hex(&source_payload[..protected_end]) != manifest.protected_tile_range.decoded_sha256
    {
        return Err("escape-door protected decorative tile hash drifted".to_string());
    }
    let mutable_end = MUTABLE_TILE_END * MD_TILE_BYTES;
    if sha256_hex(&source_payload[protected_end..mutable_end])
        != manifest.mutable_tile_range.decoded_sha256
    {
        return Err("escape-door mutable tile-range hash drifted".to_string());
    }
    for mark in &manifest.marks {
        let start = mark.source_tile_start * MD_TILE_BYTES;
        let end = start + MARK_TILES * MD_TILE_BYTES;
        if sha256_hex(&source_payload[start..end]) != mark.source_sha256 {
            return Err(format!("{} JP tile hash drifted", mark.id));
        }
    }
    Ok(())
}

fn validate_consumer_binding(rom: &[u8], manifest: &EscapeDoorsManifest) -> Result<(), String> {
    let consumer = &manifest.consumer_binding;
    let pointer = source_range(
        rom,
        TYPE_POINTER_ENTRY_OFFSET,
        4,
        "escape-door object-type pointer",
    )?;
    if u32::from_be_bytes([pointer[0], pointer[1], pointer[2], pointer[3]]) as usize
        != DEFINITION_TABLE_OFFSET
    {
        return Err("escape-door object type no longer points to its definition table".to_string());
    }

    let table = source_range(
        rom,
        DEFINITION_TABLE_OFFSET,
        MARK_COUNT * 2,
        "escape-door relative definition table",
    )?;
    if sha256_hex(table) != consumer.definition_table_sha256 {
        return Err("escape-door relative definition table hash drifted".to_string());
    }
    for (index, definition) in consumer.definitions.iter().enumerate() {
        let table_relative =
            usize::from(u16::from_be_bytes([table[index * 2], table[index * 2 + 1]]));
        let declared_relative = parse_hex(&definition.relative_offset)?;
        if table_relative != declared_relative {
            return Err(format!(
                "{} relative definition offset drifted",
                definition.mark_id
            ));
        }
        let expected = encode_sprite_definition(consumer, index)?;
        let offset = parse_hex(&definition.definition_offset)?;
        let actual = source_range(
            rom,
            offset,
            SPRITE_DEFINITION_BYTES,
            &format!("{} sprite definition", definition.mark_id),
        )?;
        if actual != expected || sha256_hex(actual) != definition.source_sha256 {
            return Err(format!("{} sprite definition drifted", definition.mark_id));
        }

        let central_tile_word = parse_u16_hex(&consumer.records[4].tile_words[index])?;
        let vram_tile = usize::from(SOURCE_VRAM_DESTINATION) / MD_TILE_BYTES;
        let relative_tile = usize::from(central_tile_word & 0x07FF)
            .checked_sub(vram_tile)
            .ok_or_else(|| format!("{} central tile precedes its pack", definition.mark_id))?;
        if central_tile_word & !0x07FF != 0xA000
            || relative_tile != manifest.marks[index].source_tile_start
        {
            return Err(format!(
                "{} central sprite no longer selects its 3x3 panel",
                definition.mark_id
            ));
        }
    }
    Ok(())
}

fn encode_sprite_definition(
    consumer: &ConsumerBinding,
    definition_index: usize,
) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(SPRITE_DEFINITION_BYTES);
    append_word(&mut output, SPRITE_COUNT as u16);
    for record in &consumer.records {
        let tile_word = if record.tile_words.len() == 1 {
            &record.tile_words[0]
        } else {
            record
                .tile_words
                .get(definition_index)
                .ok_or_else(|| "escape-door per-definition tile word is absent".to_string())?
        };
        append_word(&mut output, parse_u16_hex(&record.y)?);
        append_word(&mut output, parse_u16_hex(&record.size_link)?);
        append_word(&mut output, parse_u16_hex(tile_word)?);
        append_word(&mut output, parse_u16_hex(&record.x)?);
    }
    Ok(output)
}

fn reduce_master_panel(
    master: &[u8],
    panel: usize,
    manifest: &EscapeDoorsManifest,
    palette: &[u16; 16],
) -> Result<Vec<u8>, String> {
    if panel >= MARK_COUNT || master.len() != manifest.master_width * manifest.master_height * 4 {
        return Err("escape-door master panel contract is invalid".to_string());
    }
    let panel_width = manifest.panel_layout.panel_width;
    let panel_height = manifest.panel_layout.panel_height;
    let panel_x = panel * panel_width;
    let surface = manifest.output_surface;
    let label = &manifest.marks[panel].id;
    let mut output = vec![0u8; surface.width * surface.height];
    for target_y in 0..surface.height {
        for target_x in 0..surface.width {
            let source_x0 = panel_x + target_x * panel_width / surface.width;
            let source_x1 = panel_x + (target_x + 1) * panel_width / surface.width;
            let source_y0 = target_y * panel_height / surface.height;
            let source_y1 = (target_y + 1) * panel_height / surface.height;
            let mut weighted_rgb = [0u64; 3];
            let mut alpha_sum = 0u64;
            for source_y in source_y0..source_y1 {
                for source_x in source_x0..source_x1 {
                    let offset = (source_y * manifest.master_width + source_x) * 4;
                    let alpha = master[offset + 3] as u64;
                    alpha_sum += alpha;
                    for channel in 0..3 {
                        weighted_rgb[channel] += master[offset + channel] as u64 * alpha;
                    }
                }
            }
            if alpha_sum == 0 {
                return Err(format!("{label} panel has a transparent output sample"));
            }
            let color = [
                ((weighted_rgb[0] + alpha_sum / 2) / alpha_sum) as u8,
                ((weighted_rgb[1] + alpha_sum / 2) / alpha_sum) as u8,
                ((weighted_rgb[2] + alpha_sum / 2) / alpha_sum) as u8,
            ];
            output[target_y * surface.width + target_x] =
                nearest_palette_index(color, palette, &manifest.allowed_palette_indices, label)?
                    as u8;
        }
    }
    Ok(output)
}

fn validate_pack_roundtrip(
    header: &[u8; 6],
    bank: &[u8],
    expected_vram: u16,
    expected_payload: &[u8],
) -> Result<(), String> {
    let mut probe = vec![0u8; TARGET_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[TARGET_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != expected_vram || decoded.data != expected_payload {
        return Err("escape-door mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn write_scaled_pixel(rgba: &mut [u8], preview_width: usize, x: usize, y: usize, color: [u8; 3]) {
    for scale_y in 0..PREVIEW_SCALE {
        for scale_x in 0..PREVIEW_SCALE {
            let preview_x = x * PREVIEW_SCALE + scale_x;
            let preview_y = y * PREVIEW_SCALE + scale_y;
            let offset = (preview_y * preview_width + preview_x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
        }
    }
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn parse_u8_hex(value: &str) -> Result<u8, String> {
    let parsed = parse_hex(value)?;
    u8::try_from(parsed).map_err(|_| format!("{value} does not fit in an 8-bit value"))
}

fn append_word(output: &mut Vec<u8>, word: u16) {
    output.extend_from_slice(&word.to_be_bytes());
}

fn summary(build: &EscapeDoorsBuild, checksum: u16) -> EscapeDoorsSummary {
    EscapeDoorsSummary {
        source_tiles: build.source_payload.len() / MD_TILE_BYTES,
        rewritten_tiles: MUTABLE_TILE_END - PROTECTED_TILE_END,
        protected_tiles: PROTECTED_TILE_END,
        marks: MARK_COUNT,
        source_sprite_records: MARK_COUNT * SPRITE_COUNT,
        verified_consumer_bytes: 4 + MARK_COUNT * 2 + MARK_COUNT * SPRITE_DEFINITION_BYTES,
        written_executable_bytes: 0,
        pack_bytes: build.bank.len(),
        checksum,
    }
}
