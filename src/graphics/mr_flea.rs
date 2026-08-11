//! JP-source Mr. Flea battle-tag compiler.
//!
//! Three logically related effects share one source payload and one set of
//! sprite definitions. `여기 있다` and `꽈당큐` replace owned source ranges;
//! `막고 있다` is appended, preserving every original tile not owned by the
//! other two effects. The only executable changes are assembled from typed
//! `m68k::Inst` values.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use crate::m68k::{self, AddressReg, Inst};

use super::font_effect::{blit_indexed_glyph, read_verified_font, render_indexed_glyph};
use super::pixel::{
    decode_md_tiles_column_major, encode_md_tiles_column_major, md_color, parse_md_palette,
    write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const FLEA_HEADER_OFFSETS: [usize; 2] = [0x07_608E, 0x07_8838];
const FLEA_VRAM_DESTINATIONS: [u16; 2] = [0x5600, 0x1C00];
const FLEA_BANK_OFFSET: usize = 0x2C_0000;
const FLEA_BANK_LIMIT: usize = 0x2C_8000;
const FLEA_SOURCE_BYTES: usize = 2_208;
const FLEA_SOURCE_TILES: usize = FLEA_SOURCE_BYTES / MD_TILE_BYTES;
const FLEA_TARGET_TILES: usize = 85;
const FLEA_TARGET_BYTES: usize = FLEA_TARGET_TILES * MD_TILE_BYTES;
const HERE_TILE_START: usize = 4;
const HERE_TILE_END: usize = 20;
const HERE_TILES: usize = HERE_TILE_END - HERE_TILE_START;
const DEFENDING_TILE_START: usize = 69;
const DEFENDING_TILE_END: usize = 85;
const DEFENDING_TILES: usize = DEFENDING_TILE_END - DEFENDING_TILE_START;
const BATANKYU_TILE_START: usize = 36;
const BATANKYU_TILE_END: usize = 60;
const BATANKYU_TILES: usize = BATANKYU_TILE_END - BATANKYU_TILE_START;
const PHRASE_HEIGHT: usize = 16;
const HERE_WIDTH: usize = 64;
const DEFENDING_WIDTH: usize = 64;
const BATANKYU_WIDTH: usize = 96;
const PREVIEW_PANEL_WIDTH: usize = 96;
const PREVIEW_GAP_X: usize = 8;
const PREVIEW_GAP_Y: usize = 4;
const PREVIEW_SCALE: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrFleaSummary {
    pub source_tiles: usize,
    pub output_tiles: usize,
    pub rewritten_tiles: usize,
    pub appended_tiles: usize,
    pub protected_source_bytes: usize,
    pub consumer_headers: usize,
    pub typed_code_bytes: usize,
    pub semantic_data_patches: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct MrFleaManifest {
    schema_version: u32,
    asset_group_ids: Vec<String>,
    source_policy: String,
    font_asset: String,
    font_sha256: String,
    palette_line_words: Vec<String>,
    transparent_palette_index: usize,
    source_packs: Vec<SourcePack>,
    here: HerePlan,
    defending: DefendingPlan,
    batankyu: BatankyuPlan,
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
struct TileRange {
    start: usize,
    end_exclusive: usize,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
struct TargetTileRange {
    start: usize,
    end_exclusive: usize,
}

#[derive(Debug, Deserialize)]
struct HerePlan {
    id: String,
    jp_text: String,
    ko: String,
    tile_range: TileRange,
    target_palette_indices: Vec<usize>,
    sprite_table_base: String,
    sprite_table_entries: Vec<String>,
    sprite_definition_offsets: Vec<String>,
    sprite_definitions_sha256: String,
    source_tile_attributes: Vec<String>,
    child_spawners: Vec<ChildSpawner>,
    source_count: u16,
    target_count: u16,
    source_substates: Vec<u8>,
    target_substates: Vec<u8>,
    source_flags: Vec<u8>,
    target_flags: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct ChildSpawner {
    id: String,
    code_offset: String,
    array_offset: String,
    source_positions: Vec<i8>,
    target_positions: Vec<i8>,
}

#[derive(Debug, Deserialize)]
struct DefendingPlan {
    id: String,
    jp_text: String,
    ko: String,
    target_tile_range: TargetTileRange,
    target_palette_indices: Vec<usize>,
    sprite_table_base: String,
    sprite_table_entry: String,
    sprite_definition_offset: String,
    source_definition_sha256: String,
    source_records: Vec<SpriteRecord>,
    target_records: Vec<SpriteRecord>,
}

#[derive(Debug, Clone, Deserialize)]
struct SpriteRecord {
    y: String,
    size_link: String,
    tile: String,
    x: String,
}

#[derive(Debug, Deserialize)]
struct BatankyuPlan {
    id: String,
    jp_text: String,
    ko: String,
    tile_range: TileRange,
    glyph_slots: Vec<usize>,
    glyph_palette_indices: Vec<Vec<usize>>,
    sprite_table_base: String,
    sprite_table_entries: Vec<String>,
    sprite_definition_offsets: Vec<String>,
    sprite_definitions_sha256: String,
    x_offsets_offset: String,
    source_x_offsets: Vec<i16>,
    target_x_offsets: Vec<i16>,
}

#[derive(Debug)]
struct ExpectedPatch {
    id: String,
    offset: usize,
    expected: Vec<u8>,
    replacement: Vec<u8>,
    executable: bool,
}

#[derive(Debug)]
struct MrFleaBuild {
    manifest: MrFleaManifest,
    palette: [u16; 16],
    source_payload: Vec<u8>,
    payload: Vec<u8>,
    source_surfaces: [Vec<u8>; 3],
    target_surfaces: [Vec<u8>; 3],
    headers: Vec<[u8; 6]>,
    bank: Vec<u8>,
    patches: Vec<ExpectedPatch>,
}

pub fn apply_mr_flea(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<MrFleaSummary, String> {
    let build = build_mr_flea(source, assets_dir)?;
    let bank_end = FLEA_BANK_OFFSET + build.bank.len();
    if bank_end > FLEA_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "Mr. Flea pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(build.headers.len() + build.patches.len() + 2);
    for (declaration, header) in build.manifest.source_packs.iter().zip(&build.headers) {
        let offset = parse_hex(&declaration.header_offset)?;
        apply_expected_write(
            output,
            offset,
            source_range(source, offset, header.len(), "Mr. Flea source header")?,
            header,
            &format!("Mr. Flea {} header", declaration.id),
        )?;
        changed_ranges.push((offset, offset + header.len()));
    }
    for patch in &build.patches {
        apply_expected_write(
            output,
            patch.offset,
            &patch.expected,
            &patch.replacement,
            &patch.id,
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.replacement.len()));
    }
    apply_expected_write(
        output,
        FLEA_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "Mr. Flea expanded pack",
    )?;
    changed_ranges.push((FLEA_BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after Mr. Flea graphics",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    for declaration in &build.manifest.source_packs {
        let inserted = decode_mode1_pack_entry(output, parse_hex(&declaration.header_offset)?)?;
        if inserted.vram_destination != parse_u16_hex(&declaration.vram_destination)?
            || inserted.data != build.payload
        {
            return Err(format!(
                "inserted Mr. Flea pack does not match consumer {}",
                declaration.id
            ));
        }
    }

    eprintln!("JP graphics GFX-BATTLE-MR-FLEA-HERE/DEFENDING/BATANKYU Expected Writes:");
    for (declaration, header) in build.manifest.source_packs.iter().zip(&build.headers) {
        let offset = parse_hex(&declaration.header_offset)?;
        eprintln!(
            "  0x{offset:06X}..0x{:06X}  {} header ({} bytes)",
            offset + header.len(),
            declaration.id,
            header.len()
        );
    }
    for patch in &build.patches {
        let kind = if patch.executable {
            "typed 68000"
        } else {
            "semantic data"
        };
        eprintln!(
            "  0x{:06X}..0x{:06X}  {} ({kind}, {} bytes)",
            patch.offset,
            patch.offset + patch.replacement.len(),
            patch.id,
            patch.replacement.len()
        );
    }
    eprintln!(
        "  0x{FLEA_BANK_OFFSET:06X}..0x{bank_end:06X}  Mr. Flea pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render three rows: HERE, DEFENDING, BATANKYU; JP on the left, KR on the right.
pub fn write_mr_flea_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<MrFleaSummary, String> {
    let build = build_mr_flea(source, assets_dir)?;
    let contact_width = PREVIEW_PANEL_WIDTH * 2 + PREVIEW_GAP_X;
    let contact_height = PHRASE_HEIGHT * 3 + PREVIEW_GAP_Y * 2;
    let preview_width = contact_width * PREVIEW_SCALE;
    let preview_height = contact_height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[28, 28, 28, 255]);
    }
    for row in 0..3 {
        let y = row * (PHRASE_HEIGHT + PREVIEW_GAP_Y);
        for panel in 0..2 {
            let pixels = if panel == 0 {
                &build.source_surfaces[row]
            } else {
                &build.target_surfaces[row]
            };
            let surface_width = pixels.len() / PHRASE_HEIGHT;
            let panel_left = panel * (PREVIEW_PANEL_WIDTH + PREVIEW_GAP_X);
            let x = panel_left + (PREVIEW_PANEL_WIDTH - surface_width) / 2;
            draw_scaled_surface(
                &mut rgba,
                preview_width,
                x,
                y,
                pixels,
                surface_width,
                &build.palette,
                build.manifest.transparent_palette_index,
            );
        }
    }
    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "Mr. Flea tags",
    )?;
    Ok(summary(&build, 0))
}

fn build_mr_flea(source: &[u8], assets_dir: &Path) -> Result<MrFleaBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "Mr. Flea")?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "Mr. Flea",
    )?;

    let mut canonical_source = None;
    for declaration in &manifest.source_packs {
        let payload = checked_source_pack(source, declaration)?;
        if let Some(expected) = &canonical_source {
            if expected != &payload {
                return Err(format!(
                    "Mr. Flea consumer {} does not share the canonical JP payload",
                    declaration.id
                ));
            }
        } else {
            canonical_source = Some(payload);
        }
    }
    let source_payload =
        canonical_source.ok_or_else(|| "Mr. Flea has no JP source payload".to_string())?;
    validate_source_consumers(source, &source_payload, &manifest)?;

    let source_surfaces = reconstruct_source_surfaces(&source_payload)?;
    let target_here = render_phrase(
        &font,
        &manifest.here.ko,
        HERE_WIDTH,
        manifest.transparent_palette_index,
        &manifest.here.target_palette_indices,
    )?;
    let target_defending = render_phrase(
        &font,
        &manifest.defending.ko,
        DEFENDING_WIDTH,
        manifest.transparent_palette_index,
        &manifest.defending.target_palette_indices,
    )?;
    let target_batankyu = render_batankyu(&font, &manifest)?;

    let here_tiles =
        encode_md_tiles_column_major(&target_here, HERE_WIDTH, PHRASE_HEIGHT, "Mr. Flea here")?;
    let defending_tiles = encode_md_tiles_column_major(
        &target_defending,
        DEFENDING_WIDTH,
        PHRASE_HEIGHT,
        "Mr. Flea defending",
    )?;
    let batankyu_tiles = encode_md_tiles_column_major(
        &target_batankyu,
        BATANKYU_WIDTH,
        PHRASE_HEIGHT,
        "Mr. Flea batankyu",
    )?;
    if here_tiles.len() != HERE_TILES * MD_TILE_BYTES
        || defending_tiles.len() != DEFENDING_TILES * MD_TILE_BYTES
        || batankyu_tiles.len() != BATANKYU_TILES * MD_TILE_BYTES
    {
        return Err("Mr. Flea target tile counts drifted".to_string());
    }

    let mut payload = source_payload.clone();
    payload[HERE_TILE_START * MD_TILE_BYTES..HERE_TILE_END * MD_TILE_BYTES]
        .copy_from_slice(&here_tiles);
    payload[BATANKYU_TILE_START * MD_TILE_BYTES..BATANKYU_TILE_END * MD_TILE_BYTES]
        .copy_from_slice(&batankyu_tiles);
    payload.extend_from_slice(&defending_tiles);
    if payload.len() != FLEA_TARGET_BYTES
        || payload[..HERE_TILE_START * MD_TILE_BYTES]
            != source_payload[..HERE_TILE_START * MD_TILE_BYTES]
        || payload[HERE_TILE_END * MD_TILE_BYTES..BATANKYU_TILE_START * MD_TILE_BYTES]
            != source_payload[HERE_TILE_END * MD_TILE_BYTES..BATANKYU_TILE_START * MD_TILE_BYTES]
        || payload[BATANKYU_TILE_END * MD_TILE_BYTES..FLEA_SOURCE_BYTES]
            != source_payload[BATANKYU_TILE_END * MD_TILE_BYTES..]
    {
        return Err("Mr. Flea compiler changed protected JP bytes".to_string());
    }

    let patches = build_semantic_patches(source, &manifest)?;
    let mut headers = Vec::with_capacity(manifest.source_packs.len());
    let mut bank = None;
    for declaration in &manifest.source_packs {
        let vram = parse_u16_hex(&declaration.vram_destination)?;
        let encoded = encode_locked_mode1_pack(FLEA_BANK_OFFSET, vram, &payload)?;
        if let Some(expected) = &bank {
            if expected != &encoded.bank {
                return Err("Mr. Flea consumers produced divergent packed payloads".to_string());
            }
        } else {
            bank = Some(encoded.bank.clone());
        }
        validate_pack_roundtrip(&encoded.header, &encoded.bank, vram, &payload)?;
        headers.push(encoded.header);
    }

    Ok(MrFleaBuild {
        manifest,
        palette,
        source_payload,
        payload,
        source_surfaces,
        target_surfaces: [target_here, target_defending, target_batankyu],
        headers,
        bank: bank.ok_or_else(|| "Mr. Flea pack was not encoded".to_string())?,
        patches,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<MrFleaManifest, String> {
    let path = assets_dir.join("graphics_text/mr_flea.json");
    let bytes = fs::read(&path)
        .map_err(|error| format!("failed to read Mr. Flea source {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid Mr. Flea source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &MrFleaManifest) -> Result<(), String> {
    let expected_ids = [
        "GFX-BATTLE-MR-FLEA-HERE",
        "GFX-BATTLE-MR-FLEA-DEFENDING",
        "GFX-BATTLE-MR-FLEA-BATANKYU",
    ];
    if manifest.schema_version != 1
        || manifest.asset_group_ids != expected_ids
        || !manifest.source_policy.contains("JP")
        || !manifest.source_policy.contains("typed 68000")
        || manifest.font_asset != "neodgm.ttf"
        || manifest.transparent_palette_index != 0
    {
        return Err("unsupported Mr. Flea manifest identity".to_string());
    }
    if manifest.here.id != expected_ids[0]
        || manifest.here.jp_text != "ここにいる"
        || manifest.here.ko != "여기 있다"
        || manifest.here.tile_range.start != HERE_TILE_START
        || manifest.here.tile_range.end_exclusive != HERE_TILE_END
        || manifest.here.target_palette_indices != [13, 12, 1]
        || manifest.here.source_count != 5
        || manifest.here.target_count != 4
    {
        return Err("Mr. Flea here plan drifted".to_string());
    }
    if manifest.defending.id != expected_ids[1]
        || manifest.defending.jp_text != "ふせいでいる"
        || manifest.defending.ko != "막고 있다"
        || manifest.defending.target_tile_range.start != DEFENDING_TILE_START
        || manifest.defending.target_tile_range.end_exclusive != DEFENDING_TILE_END
        || manifest.defending.target_palette_indices != [13, 12, 1]
        || manifest.defending.source_records.len() != 6
        || manifest.defending.target_records.len() != 4
    {
        return Err("Mr. Flea defending plan drifted".to_string());
    }
    if manifest.batankyu.id != expected_ids[2]
        || manifest.batankyu.jp_text != "ばたんきゅー"
        || manifest.batankyu.ko != "꽈당큐"
        || manifest.batankyu.tile_range.start != BATANKYU_TILE_START
        || manifest.batankyu.tile_range.end_exclusive != BATANKYU_TILE_END
        || manifest.batankyu.glyph_slots != [0, 1, 2]
        || manifest.batankyu.glyph_palette_indices
            != [vec![5, 4, 1], vec![15, 14, 10], vec![8, 9, 1]]
    {
        return Err("Mr. Flea batankyu plan drifted".to_string());
    }
    let expected_packs = [
        (
            "mr-flea-enemy",
            FLEA_HEADER_OFFSETS[0],
            FLEA_VRAM_DESTINATIONS[0],
        ),
        (
            "mr-flea-amigo",
            FLEA_HEADER_OFFSETS[1],
            FLEA_VRAM_DESTINATIONS[1],
        ),
    ];
    if manifest.source_packs.len() != expected_packs.len() {
        return Err("Mr. Flea must declare both JP source consumers".to_string());
    }
    for (pack, (id, header, vram)) in manifest.source_packs.iter().zip(expected_packs) {
        if pack.id != id
            || parse_hex(&pack.header_offset)? != header
            || parse_u16_hex(&pack.vram_destination)? != vram
            || pack.decoded_bytes != FLEA_SOURCE_BYTES
            || pack.decoded_sha256.len() != 64
        {
            return Err(format!("Mr. Flea source declaration {} drifted", pack.id));
        }
    }
    validate_here_manifest(manifest)?;
    validate_defending_manifest(manifest)?;
    validate_batankyu_manifest(manifest)?;
    Ok(())
}

fn validate_here_manifest(manifest: &MrFleaManifest) -> Result<(), String> {
    let here = &manifest.here;
    if here.tile_range.source_sha256.len() != 64
        || parse_hex(&here.sprite_table_base)? != 0x07_4688
        || here.sprite_table_entries.len() != 4
        || here.sprite_definition_offsets.len() != 4
        || here.source_tile_attributes.len() != 4
        || here.sprite_definitions_sha256.len() != 64
        || here.child_spawners.len() != 2
        || here.source_substates != [4, 10, 10, 11, 12, 13]
        || here.target_substates != [4, 10, 11, 12, 13, 0]
        || here.source_flags != [16, 32, 32, 32, 32, 32]
        || here.target_flags != [16, 32, 32, 32, 32, 0]
    {
        return Err("Mr. Flea here semantic declarations drifted".to_string());
    }
    let expected_spawners = [
        ("enemy", 0x03_22FE, 0x03_2600),
        ("amigo", 0x04_EC0E, 0x04_EEC6),
    ];
    for (spawner, (id, code, array)) in here.child_spawners.iter().zip(expected_spawners) {
        if spawner.id != id
            || parse_hex(&spawner.code_offset)? != code
            || parse_hex(&spawner.array_offset)? != array
            || spawner.source_positions.len() != 6
            || spawner.target_positions.len() != 6
        {
            return Err(format!("Mr. Flea {id} child-spawner declaration drifted"));
        }
    }
    Ok(())
}

fn validate_defending_manifest(manifest: &MrFleaManifest) -> Result<(), String> {
    let plan = &manifest.defending;
    if parse_hex(&plan.sprite_table_base)? != 0x07_4688
        || parse_hex(&plan.sprite_table_entry)? != 0x07_46A4
        || parse_hex(&plan.sprite_definition_offset)? != 0x07_4766
        || plan.source_definition_sha256.len() != 64
    {
        return Err("Mr. Flea defending consumer declaration drifted".to_string());
    }
    let target_tiles = plan
        .target_records
        .iter()
        .map(|record| parse_u16_hex(&record.tile).map(|tile| usize::from(tile & 0x07FF)))
        .collect::<Result<Vec<_>, _>>()?;
    let source_base = usize::from(FLEA_VRAM_DESTINATIONS[0]) / MD_TILE_BYTES;
    if target_tiles
        != (0..DEFENDING_TILES / 4)
            .map(|index| source_base + DEFENDING_TILE_START + index * 4)
            .collect::<Vec<_>>()
    {
        return Err("Mr. Flea defending records do not own appended tiles 69..85".to_string());
    }
    Ok(())
}

fn validate_batankyu_manifest(manifest: &MrFleaManifest) -> Result<(), String> {
    let plan = &manifest.batankyu;
    if plan.tile_range.source_sha256.len() != 64
        || parse_hex(&plan.sprite_table_base)? != 0x07_4688
        || plan.sprite_table_entries.len() != 6
        || plan.sprite_definition_offsets.len() != 6
        || plan.sprite_definitions_sha256.len() != 64
        || parse_hex(&plan.x_offsets_offset)? != 0x03_30B8
        || plan.source_x_offsets != [-48, -32, -16, 0, 16, 32]
        || plan.target_x_offsets != [-16, 0, 16, 32, 48, 64]
    {
        return Err("Mr. Flea batankyu consumer declaration drifted".to_string());
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

fn validate_source_consumers(
    source: &[u8],
    payload: &[u8],
    manifest: &MrFleaManifest,
) -> Result<(), String> {
    validate_tile_range(payload, &manifest.here.tile_range, "Mr. Flea here")?;
    validate_tile_range(payload, &manifest.batankyu.tile_range, "Mr. Flea batankyu")?;
    validate_here_consumers(source, manifest)?;
    validate_defending_consumer(source, manifest)?;
    validate_batankyu_consumers(source, manifest)?;
    Ok(())
}

fn validate_tile_range(payload: &[u8], range: &TileRange, label: &str) -> Result<(), String> {
    let start = range.start * MD_TILE_BYTES;
    let end = range.end_exclusive * MD_TILE_BYTES;
    let bytes = source_range(payload, start, end - start, label)?;
    let actual = sha256_hex(bytes);
    if actual != range.source_sha256 {
        return Err(format!(
            "{label} JP tile SHA-256 mismatch: expected {}, got {actual}",
            range.source_sha256
        ));
    }
    Ok(())
}

fn validate_here_consumers(source: &[u8], manifest: &MrFleaManifest) -> Result<(), String> {
    let here = &manifest.here;
    let table_base = parse_hex(&here.sprite_table_base)?;
    for index in 0..4 {
        let entry = parse_hex(&here.sprite_table_entries[index])?;
        let definition = parse_hex(&here.sprite_definition_offsets[index])?;
        if resolve_table_entry(source, table_base, entry, "Mr. Flea here")? != definition {
            return Err(format!("Mr. Flea here table entry {index} drifted"));
        }
        let record = decode_single_record(source, definition, "Mr. Flea here definition")?;
        if record.y != 0x0078
            || record.size_link != 0x0508
            || record.tile != parse_u16_hex(&here.source_tile_attributes[index])?
            || record.x != 0x0078
        {
            return Err(format!("Mr. Flea here sprite definition {index} drifted"));
        }
    }
    let first_definition = parse_hex(&here.sprite_definition_offsets[0])?;
    let definitions = source_range(
        source,
        first_definition,
        4 * 10,
        "Mr. Flea here definitions",
    )?;
    if sha256_hex(definitions) != here.sprite_definitions_sha256 {
        return Err("Mr. Flea here definition SHA-256 drifted".to_string());
    }
    Ok(())
}

fn validate_defending_consumer(source: &[u8], manifest: &MrFleaManifest) -> Result<(), String> {
    let plan = &manifest.defending;
    let table_base = parse_hex(&plan.sprite_table_base)?;
    let entry = parse_hex(&plan.sprite_table_entry)?;
    let definition = parse_hex(&plan.sprite_definition_offset)?;
    if resolve_table_entry(source, table_base, entry, "Mr. Flea defending")? != definition {
        return Err("Mr. Flea defending table entry drifted".to_string());
    }
    let semantic = encode_sprite_definition(&plan.source_records)?;
    let actual = source_range(
        source,
        definition,
        semantic.len(),
        "Mr. Flea defending definition",
    )?;
    if actual != semantic || sha256_hex(actual) != plan.source_definition_sha256 {
        return Err("Mr. Flea defending source definition drifted".to_string());
    }
    Ok(())
}

fn validate_batankyu_consumers(source: &[u8], manifest: &MrFleaManifest) -> Result<(), String> {
    let plan = &manifest.batankyu;
    let table_base = parse_hex(&plan.sprite_table_base)?;
    for index in 0..6 {
        let entry = parse_hex(&plan.sprite_table_entries[index])?;
        let definition = parse_hex(&plan.sprite_definition_offsets[index])?;
        if resolve_table_entry(source, table_base, entry, "Mr. Flea batankyu")? != definition {
            return Err(format!("Mr. Flea batankyu table entry {index} drifted"));
        }
        let record = decode_single_record(source, definition, "Mr. Flea batankyu definition")?;
        let expected_tile = 0x62D4 + u16::try_from(index * 4).unwrap();
        if record.y != 0x0078
            || record.size_link != 0x0508
            || record.tile != expected_tile
            || record.x != 0x0078
        {
            return Err(format!(
                "Mr. Flea batankyu sprite definition {index} drifted"
            ));
        }
    }
    let first_definition = parse_hex(&plan.sprite_definition_offsets[0])?;
    let definitions = source_range(
        source,
        first_definition,
        6 * 10,
        "Mr. Flea batankyu definitions",
    )?;
    if sha256_hex(definitions) != plan.sprite_definitions_sha256 {
        return Err("Mr. Flea batankyu definition SHA-256 drifted".to_string());
    }
    Ok(())
}

#[derive(Debug)]
struct DecodedSpriteRecord {
    y: u16,
    size_link: u16,
    tile: u16,
    x: u16,
}

fn resolve_table_entry(
    source: &[u8],
    table_base: usize,
    entry: usize,
    label: &str,
) -> Result<usize, String> {
    let bytes = source_range(source, entry, 2, label)?;
    Ok(table_base + u16::from_be_bytes([bytes[0], bytes[1]]) as usize)
}

fn decode_single_record(
    source: &[u8],
    definition: usize,
    label: &str,
) -> Result<DecodedSpriteRecord, String> {
    let bytes = source_range(source, definition, 10, label)?;
    if u16::from_be_bytes([bytes[0], bytes[1]]) != 1 {
        return Err(format!("{label} is not a single-record definition"));
    }
    Ok(DecodedSpriteRecord {
        y: u16::from_be_bytes([bytes[2], bytes[3]]),
        size_link: u16::from_be_bytes([bytes[4], bytes[5]]),
        tile: u16::from_be_bytes([bytes[6], bytes[7]]),
        x: u16::from_be_bytes([bytes[8], bytes[9]]),
    })
}

fn build_semantic_patches(
    source: &[u8],
    manifest: &MrFleaManifest,
) -> Result<Vec<ExpectedPatch>, String> {
    let here = &manifest.here;
    let source_count = m68k::assemble(&[Inst::MoveWordImmediateToDisplacementAddress {
        immediate: here.source_count,
        displacement: 0x0026,
        destination: AddressReg::A0,
    }])?;
    let target_count = m68k::assemble(&[Inst::MoveWordImmediateToDisplacementAddress {
        immediate: here.target_count,
        displacement: 0x0026,
        destination: AddressReg::A0,
    }])?;
    let mut patches = Vec::new();
    for spawner in &here.child_spawners {
        let code_offset = parse_hex(&spawner.code_offset)?;
        if source_range(
            source,
            code_offset,
            source_count.len(),
            "Mr. Flea child-count code",
        )? != source_count
        {
            return Err(format!(
                "Mr. Flea {} child-count instruction drifted",
                spawner.id
            ));
        }
        patches.push(ExpectedPatch {
            id: format!("Mr. Flea {} child count 5 -> 4", spawner.id),
            offset: code_offset,
            expected: source_count.clone(),
            replacement: target_count.clone(),
            executable: true,
        });

        let array_offset = parse_hex(&spawner.array_offset)?;
        patches.push(semantic_byte_patch(
            source,
            &format!("Mr. Flea {} arc positions", spawner.id),
            array_offset,
            &encode_i8_values(&spawner.source_positions),
            &encode_i8_values(&spawner.target_positions),
        )?);
        patches.push(semantic_byte_patch(
            source,
            &format!("Mr. Flea {} child substates", spawner.id),
            array_offset + 6,
            &here.source_substates,
            &here.target_substates,
        )?);
        patches.push(semantic_byte_patch(
            source,
            &format!("Mr. Flea {} child flags", spawner.id),
            array_offset + 12,
            &here.source_flags,
            &here.target_flags,
        )?);
    }

    let defending = &manifest.defending;
    let source_definition = encode_sprite_definition(&defending.source_records)?;
    let target_definition = encode_sprite_definition(&defending.target_records)?;
    let definition_offset = parse_hex(&defending.sprite_definition_offset)?;
    let expected_prefix = source_definition[..target_definition.len()].to_vec();
    patches.push(semantic_byte_patch(
        source,
        "Mr. Flea defending sprite definition",
        definition_offset,
        &expected_prefix,
        &target_definition,
    )?);

    let batankyu = &manifest.batankyu;
    patches.push(semantic_byte_patch(
        source,
        "Mr. Flea batankyu x offsets",
        parse_hex(&batankyu.x_offsets_offset)?,
        &encode_i16_values(&batankyu.source_x_offsets),
        &encode_i16_values(&batankyu.target_x_offsets),
    )?);
    Ok(patches)
}

fn semantic_byte_patch(
    source: &[u8],
    id: &str,
    offset: usize,
    expected: &[u8],
    replacement: &[u8],
) -> Result<ExpectedPatch, String> {
    if expected.len() != replacement.len()
        || source_range(source, offset, expected.len(), id)? != expected
    {
        return Err(format!("{id}: semantic source values drifted"));
    }
    Ok(ExpectedPatch {
        id: id.to_string(),
        offset,
        expected: expected.to_vec(),
        replacement: replacement.to_vec(),
        executable: false,
    })
}

fn encode_i8_values(values: &[i8]) -> Vec<u8> {
    values.iter().map(|&value| value as u8).collect()
}

fn encode_i16_values(values: &[i16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

fn encode_sprite_definition(records: &[SpriteRecord]) -> Result<Vec<u8>, String> {
    let count =
        u16::try_from(records.len()).map_err(|_| "sprite record count overflowed".to_string())?;
    let mut output = Vec::with_capacity(2 + records.len() * 8);
    output.extend_from_slice(&count.to_be_bytes());
    for record in records {
        for word in [&record.y, &record.size_link, &record.tile, &record.x] {
            output.extend_from_slice(&parse_u16_hex(word)?.to_be_bytes());
        }
    }
    Ok(output)
}

fn reconstruct_source_surfaces(payload: &[u8]) -> Result<[Vec<u8>; 3], String> {
    let here = compose_source_glyphs(payload, &[4, 4, 8, 12, 16], "Mr. Flea JP here")?;
    let defending =
        compose_source_glyphs(payload, &[20, 24, 12, 28, 12, 16], "Mr. Flea JP defending")?;
    let batankyu = decode_md_tiles_column_major(
        &payload[BATANKYU_TILE_START * MD_TILE_BYTES..BATANKYU_TILE_END * MD_TILE_BYTES],
        BATANKYU_WIDTH,
        PHRASE_HEIGHT,
        "Mr. Flea JP batankyu",
    )?;
    Ok([here, defending, batankyu])
}

fn compose_source_glyphs(
    payload: &[u8],
    tile_starts: &[usize],
    label: &str,
) -> Result<Vec<u8>, String> {
    let width = tile_starts.len() * 16;
    let mut output = vec![0u8; width * PHRASE_HEIGHT];
    for (index, &tile_start) in tile_starts.iter().enumerate() {
        let start = tile_start * MD_TILE_BYTES;
        let glyph = decode_md_tiles_column_major(
            source_range(payload, start, 4 * MD_TILE_BYTES, label)?,
            16,
            16,
            label,
        )?;
        for y in 0..16 {
            let destination = y * width + index * 16;
            output[destination..destination + 16].copy_from_slice(&glyph[y * 16..(y + 1) * 16]);
        }
    }
    Ok(output)
}

fn render_phrase(
    font: &Font,
    text: &str,
    width: usize,
    transparent: usize,
    palette_indices: &[usize],
) -> Result<Vec<u8>, String> {
    let characters = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<Vec<_>>();
    if characters.len() * 16 != width || palette_indices.len() != 3 {
        return Err(format!(
            "Mr. Flea phrase {text:?} does not match its fixed sprite cells"
        ));
    }
    let mut output = vec![transparent as u8; width * PHRASE_HEIGHT];
    for (index, ch) in characters.into_iter().enumerate() {
        let glyph = render_indexed_glyph(
            font,
            ch,
            transparent as u8,
            palette_indices[0] as u8,
            palette_indices[1] as u8,
            palette_indices[2] as u8,
        );
        blit_indexed_glyph(&mut output, width, PHRASE_HEIGHT, index * 16, 0, &glyph)?;
    }
    Ok(output)
}

fn render_batankyu(font: &Font, manifest: &MrFleaManifest) -> Result<Vec<u8>, String> {
    let plan = &manifest.batankyu;
    let characters = plan.ko.chars().collect::<Vec<_>>();
    if characters.len() != plan.glyph_slots.len()
        || characters.len() != plan.glyph_palette_indices.len()
    {
        return Err("Mr. Flea batankyu glyph declarations drifted".to_string());
    }
    let transparent = manifest.transparent_palette_index as u8;
    let mut output = vec![transparent; BATANKYU_WIDTH * PHRASE_HEIGHT];
    for ((ch, &slot), palette) in characters
        .into_iter()
        .zip(&plan.glyph_slots)
        .zip(&plan.glyph_palette_indices)
    {
        if slot >= BATANKYU_WIDTH / 16 || palette.len() != 3 {
            return Err("Mr. Flea batankyu glyph slot is invalid".to_string());
        }
        let glyph = render_indexed_glyph(
            font,
            ch,
            transparent,
            palette[0] as u8,
            palette[1] as u8,
            palette[2] as u8,
        );
        blit_indexed_glyph(
            &mut output,
            BATANKYU_WIDTH,
            PHRASE_HEIGHT,
            slot * 16,
            0,
            &glyph,
        )?;
    }
    let admitted = std::iter::once(manifest.transparent_palette_index)
        .chain(
            plan.glyph_palette_indices
                .iter()
                .flat_map(|roles| roles.iter().copied()),
        )
        .collect::<BTreeSet<_>>();
    if output
        .iter()
        .any(|&pixel| !admitted.contains(&(pixel as usize)))
    {
        return Err("Mr. Flea batankyu uses an undeclared palette role".to_string());
    }
    Ok(output)
}

fn validate_pack_roundtrip(
    header: &[u8; 6],
    bank: &[u8],
    vram: u16,
    payload: &[u8],
) -> Result<(), String> {
    let mut probe = vec![0u8; FLEA_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[FLEA_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != vram || decoded.data != payload {
        return Err("Mr. Flea mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_scaled_surface(
    rgba: &mut [u8],
    preview_width: usize,
    x_origin: usize,
    y_origin: usize,
    pixels: &[u8],
    surface_width: usize,
    palette: &[u16; 16],
    transparent: usize,
) {
    for y in 0..PHRASE_HEIGHT {
        for x in 0..surface_width {
            let palette_index = pixels[y * surface_width + x] as usize;
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
                    let preview_y = (y_origin + y) * PREVIEW_SCALE + scale_y;
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

fn summary(build: &MrFleaBuild, checksum: u16) -> MrFleaSummary {
    debug_assert_eq!(build.source_payload.len(), FLEA_SOURCE_BYTES);
    MrFleaSummary {
        source_tiles: FLEA_SOURCE_TILES,
        output_tiles: build.payload.len() / MD_TILE_BYTES,
        rewritten_tiles: HERE_TILES + BATANKYU_TILES + DEFENDING_TILES,
        appended_tiles: DEFENDING_TILES,
        protected_source_bytes: FLEA_SOURCE_BYTES - (HERE_TILES + BATANKYU_TILES) * MD_TILE_BYTES,
        consumer_headers: build.headers.len(),
        typed_code_bytes: build
            .patches
            .iter()
            .filter(|patch| patch.executable)
            .map(|patch| patch.replacement.len())
            .sum(),
        semantic_data_patches: build
            .patches
            .iter()
            .filter(|patch| !patch.executable)
            .count(),
        pack_bytes: build.bank.len(),
        checksum,
    }
}
