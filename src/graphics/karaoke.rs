//! JP-source Madou Ondo karaoke lyric compiler.
//!
//! The JP game already owns the useful layout: nine 38x2 tile surfaces,
//! selected by one table and sent by one unchanged routine.  Korean therefore
//! keeps that execution path and timing data, replaces only the source-owned
//! lyric patterns and maps, and writes both packs into expanded ROM.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use crate::m68k::{self, AddressReg, BranchCondition, BranchWidth, DataReg, Inst};

use super::font_effect::read_verified_font;
use super::pixel::{encode_md_tiles_column_major, write_rgba_png};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_bytes, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const GROUP_HEADER_OFFSET: usize = 0x08_A12C;
const GROUP_HEADER_BYTES: usize = 14;
const PATTERN_HEADER_OFFSET: usize = GROUP_HEADER_OFFSET;
const MAP_HEADER_OFFSET: usize = GROUP_HEADER_OFFSET + 6;
const PATTERN_BANK_OFFSET: usize = 0x2E_0000;
const PATTERN_BANK_LIMIT: usize = 0x2F_0000;
const MAP_BANK_OFFSET: usize = 0x2F_0000;
const MAP_BANK_LIMIT: usize = 0x2F_8000;
const PATTERN_BYTES: usize = 47_072;
const PATTERN_TILES: usize = PATTERN_BYTES / MD_TILE_BYTES;
const MUTABLE_TILE_START: usize = 0x04FF;
const MUTABLE_TILE_END: usize = 0x05AD;
const MUTABLE_TILES: usize = MUTABLE_TILE_END - MUTABLE_TILE_START;
const MAP_BYTES: usize = 3_648;
const MAP_LYRIC_START: usize = 0x08C0;
const MAP_LYRIC_END: usize = 0x0E18;
const MAP_SUFFIX_END: usize = 0x0E40;
const LINE_COUNT: usize = 9;
const LINE_WIDTH_TILES: usize = 38;
const LINE_HEIGHT_TILES: usize = 2;
const LINE_BYTES: usize = LINE_WIDTH_TILES * LINE_HEIGHT_TILES * 2;
const LINE_WIDTH_PIXELS: usize = LINE_WIDTH_TILES * 8;
const LINE_HEIGHT_PIXELS: usize = LINE_HEIGHT_TILES * 8;
const GLYPH_CELL_PIXELS: usize = 16;
const GLYPH_TILES: usize = 4;
const LINE_WIDTH_CELLS: usize = LINE_WIDTH_TILES / 2;
const POINTER_TABLE_OFFSET: usize = 0x08_FC7A;
const POINTER_TABLE_BYTES: usize = 36;
const LINE_SENDER_OFFSET: usize = 0x08_FC9E;
const LINE_SENDER_BYTES: usize = 42;
const TIMING_TABLE_OFFSET: usize = 0x08_FCC8;
const TIMING_TABLE_BYTES: usize = 100;
const PREVIEW_SCALE: usize = 2;
const PREVIEW_COLUMN_GAP: usize = 16;
const PREVIEW_ROW_GAP: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KaraokeSummary {
    pub lines: usize,
    pub unique_glyphs: usize,
    pub source_pattern_bytes: usize,
    pub rewritten_pattern_tiles: usize,
    pub rewritten_map_bytes: usize,
    pub pattern_pack_bytes: usize,
    pub map_pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct KaraokeManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    font_asset: String,
    font_sha256: String,
    font_size_px: f32,
    coverage_threshold: u8,
    surface: SurfaceDeclaration,
    source_group: SourceGroupDeclaration,
    pattern_ownership: PatternOwnership,
    map_ownership: MapOwnership,
    consumer: ConsumerDeclaration,
    lines: Vec<LineDeclaration>,
}

#[derive(Debug, Deserialize)]
struct SurfaceDeclaration {
    width_tiles: usize,
    height_tiles: usize,
    storage: String,
    blank_tile_word: String,
    runtime_palette_or: String,
}

#[derive(Debug, Deserialize)]
struct SourceGroupDeclaration {
    header_offset: String,
    header_bytes: usize,
    header_sha256: String,
    terminator: String,
    pattern_pack: PackDeclaration,
    map_pack: PackDeclaration,
}

#[derive(Debug, Deserialize)]
struct PackDeclaration {
    command: String,
    #[serde(default)]
    vram_destination: String,
    #[serde(default)]
    ram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
    target_bank_offset: String,
    target_bank_limit: String,
}

#[derive(Debug, Deserialize)]
struct PatternOwnership {
    protected_prefix_tiles: TileRangeDeclaration,
    mutable_lyric_tiles: MutableTileRangeDeclaration,
    protected_suffix_tiles: TileRangeDeclaration,
}

#[derive(Debug, Deserialize)]
struct TileRangeDeclaration {
    start: usize,
    end_exclusive: usize,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
struct MutableTileRangeDeclaration {
    start: usize,
    end_exclusive: usize,
    source_sha256: String,
    source_palette_indices: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct MapOwnership {
    protected_prefix: ByteRangeDeclaration,
    lyric_surfaces: LyricRangeDeclaration,
    protected_suffix: ByteRangeDeclaration,
}

#[derive(Debug, Deserialize)]
struct ByteRangeDeclaration {
    start: String,
    end_exclusive: String,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
struct LyricRangeDeclaration {
    start: String,
    end_exclusive: String,
    bytes_per_line: usize,
    line_count: usize,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
struct ConsumerDeclaration {
    line_ram_base: String,
    pointer_table_offset: String,
    pointer_table_bytes: usize,
    pointer_table_sha256: String,
    line_sender_offset: String,
    line_sender_bytes: usize,
    line_sender_sha256: String,
    line_sender_target: String,
    entries: Vec<PointerEntry>,
    timing_table_offset: String,
    timing_table_bytes: usize,
    timing_table_sha256: String,
    timing_records: Vec<TimingRecord>,
}

#[derive(Debug, Deserialize)]
struct PointerEntry {
    source_offset: String,
    destination_offset: String,
}

#[derive(Debug, Deserialize)]
struct TimingRecord {
    timestamp: String,
    y: i16,
    limit: i16,
    step: String,
}

#[derive(Debug, Deserialize)]
struct LineDeclaration {
    id: String,
    jp: String,
    ko: String,
}

#[derive(Debug)]
struct KaraokeBuild {
    manifest: KaraokeManifest,
    source_patterns: Vec<u8>,
    source_map: Vec<u8>,
    target_patterns: Vec<u8>,
    target_map: Vec<u8>,
    source_surfaces: Vec<Vec<u8>>,
    target_surfaces: Vec<Vec<u8>>,
    glyph_tiles: BTreeMap<char, [u16; GLYPH_TILES]>,
    pattern_header: [u8; 6],
    pattern_bank: Vec<u8>,
    map_header: [u8; 6],
    map_bank: Vec<u8>,
}

#[derive(Debug)]
struct KaraokeTarget {
    patterns: Vec<u8>,
    map: Vec<u8>,
    glyph_tiles: BTreeMap<char, [u16; GLYPH_TILES]>,
}

pub fn apply_karaoke(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<KaraokeSummary, String> {
    let build = build_karaoke(source, assets_dir)?;
    let pattern_bank_end = PATTERN_BANK_OFFSET + build.pattern_bank.len();
    let map_bank_end = MAP_BANK_OFFSET + build.map_bank.len();
    if pattern_bank_end > PATTERN_BANK_LIMIT || pattern_bank_end > output.len() {
        return Err(format!(
            "karaoke pattern pack ends outside its expanded bank at 0x{pattern_bank_end:06X}"
        ));
    }
    if map_bank_end > MAP_BANK_LIMIT || map_bank_end > output.len() {
        return Err(format!(
            "karaoke map pack ends outside its expanded bank at 0x{map_bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(5);
    apply_expected_write(
        output,
        PATTERN_HEADER_OFFSET,
        source_range(
            source,
            PATTERN_HEADER_OFFSET,
            6,
            "karaoke JP pattern header",
        )?,
        &build.pattern_header,
        "karaoke Korean pattern header",
    )?;
    changed_ranges.push((PATTERN_HEADER_OFFSET, PATTERN_HEADER_OFFSET + 6));
    apply_expected_write(
        output,
        MAP_HEADER_OFFSET,
        source_range(source, MAP_HEADER_OFFSET, 6, "karaoke JP map header")?,
        &build.map_header,
        "karaoke Korean map header",
    )?;
    changed_ranges.push((MAP_HEADER_OFFSET, MAP_HEADER_OFFSET + 6));
    apply_expected_write(
        output,
        PATTERN_BANK_OFFSET,
        &vec![0xFF; build.pattern_bank.len()],
        &build.pattern_bank,
        "karaoke expanded pattern pack",
    )?;
    changed_ranges.push((PATTERN_BANK_OFFSET, pattern_bank_end));
    apply_expected_write(
        output,
        MAP_BANK_OFFSET,
        &vec![0xFF; build.map_bank.len()],
        &build.map_bank,
        "karaoke expanded map pack",
    )?;
    changed_ranges.push((MAP_BANK_OFFSET, map_bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after karaoke graphics",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;

    let inserted_patterns = decode_mode1_pack_entry(output, PATTERN_HEADER_OFFSET)?;
    if inserted_patterns.vram_destination != 0 || inserted_patterns.data != build.target_patterns {
        return Err("inserted karaoke pattern pack differs from the compiled payload".to_string());
    }
    let inserted_map = decode_mode1_pack_entry(output, MAP_HEADER_OFFSET)?;
    if inserted_map.vram_destination != 0xA000 || inserted_map.data != build.target_map {
        return Err("inserted karaoke map pack differs from the compiled payload".to_string());
    }
    validate_consumer(output, &build.manifest.consumer, "output")?;
    if source_range(
        output,
        GROUP_HEADER_OFFSET + 12,
        2,
        "karaoke output terminator",
    )? != [0xFF, 0xFF]
    {
        return Err("karaoke output changed the JP group terminator".to_string());
    }

    eprintln!("JP graphics GFX-KARAOKE Expected Writes:");
    eprintln!(
        "  0x{PATTERN_HEADER_OFFSET:06X}..0x{:06X}  pattern header (6 bytes)",
        PATTERN_HEADER_OFFSET + 6
    );
    eprintln!(
        "  0x{MAP_HEADER_OFFSET:06X}..0x{:06X}  RAM map header (6 bytes)",
        MAP_HEADER_OFFSET + 6
    );
    eprintln!(
        "  0x{PATTERN_BANK_OFFSET:06X}..0x{pattern_bank_end:06X}  pattern pack ({} bytes)",
        build.pattern_bank.len()
    );
    eprintln!(
        "  0x{MAP_BANK_OFFSET:06X}..0x{map_bank_end:06X}  map pack ({} bytes)",
        build.map_bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");
    Ok(summary(&build, checksum))
}

/// Render all nine exact JP/Korean row-major 16x16-cell surfaces side by side.
pub fn write_karaoke_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<KaraokeSummary, String> {
    let build = build_karaoke(source, assets_dir)?;
    let logical_width = LINE_WIDTH_PIXELS * 2 + PREVIEW_COLUMN_GAP;
    let logical_height = LINE_COUNT * LINE_HEIGHT_PIXELS + (LINE_COUNT - 1) * PREVIEW_ROW_GAP;
    let width = logical_width * PREVIEW_SCALE;
    let height = logical_height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; width * height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[22, 20, 28, 255]);
    }
    for line in 0..LINE_COUNT {
        let y = line * (LINE_HEIGHT_PIXELS + PREVIEW_ROW_GAP);
        draw_surface(&mut rgba, width, 0, y, &build.source_surfaces[line])?;
        draw_surface(
            &mut rgba,
            width,
            LINE_WIDTH_PIXELS + PREVIEW_COLUMN_GAP,
            y,
            &build.target_surfaces[line],
        )?;
    }
    write_rgba_png(
        output_path,
        width as u32,
        height as u32,
        &rgba,
        "karaoke static preview",
    )?;
    Ok(summary(&build, 0))
}

fn build_karaoke(source: &[u8], assets_dir: &Path) -> Result<KaraokeBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    validate_source_group(source, &manifest)?;
    validate_consumer(source, &manifest.consumer, "JP source")?;

    let source_patterns = checked_pack(
        source,
        PATTERN_HEADER_OFFSET,
        0,
        PATTERN_BYTES,
        &manifest.source_group.pattern_pack.decoded_sha256,
        "karaoke JP patterns",
    )?;
    let source_map = checked_pack(
        source,
        MAP_HEADER_OFFSET,
        0xA000,
        MAP_BYTES,
        &manifest.source_group.map_pack.decoded_sha256,
        "karaoke JP map",
    )?;
    validate_source_ownership(&source_patterns, &source_map, &manifest)?;

    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "karaoke",
    )?;
    let target = compile_target(&font, &source_patterns, &source_map, &manifest)?;
    validate_target_ownership(
        &source_patterns,
        &source_map,
        &target.patterns,
        &target.map,
        &target.glyph_tiles,
        &manifest,
    )?;

    let source_surfaces = render_all_surfaces(
        &source_patterns,
        &source_map,
        &manifest,
        "karaoke JP surface",
    )?;
    let target_surfaces = render_all_surfaces(
        &target.patterns,
        &target.map,
        &manifest,
        "karaoke Korean surface",
    )?;

    let pattern_encoded = encode_locked_mode1_bytes(PATTERN_BANK_OFFSET, 0, &target.patterns)?;
    let map_encoded = encode_locked_mode1_bytes(MAP_BANK_OFFSET, 0xA000, &target.map)?;
    let mut map_header = map_encoded.header;
    map_header[0] = 0x00;
    validate_pack_roundtrip(
        &pattern_encoded.header,
        &pattern_encoded.bank,
        PATTERN_BANK_OFFSET,
        0,
        &target.patterns,
        "karaoke pattern",
    )?;
    validate_pack_roundtrip(
        &map_header,
        &map_encoded.bank,
        MAP_BANK_OFFSET,
        0xA000,
        &target.map,
        "karaoke map",
    )?;

    Ok(KaraokeBuild {
        manifest,
        source_patterns,
        source_map,
        target_patterns: target.patterns,
        target_map: target.map,
        source_surfaces,
        target_surfaces,
        glyph_tiles: target.glyph_tiles,
        pattern_header: pattern_encoded.header,
        pattern_bank: pattern_encoded.bank,
        map_header,
        map_bank: map_encoded.bank,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<KaraokeManifest, String> {
    let path = assets_dir.join("graphics_text/karaoke.json");
    let bytes = fs::read(&path)
        .map_err(|error| format!("failed to read karaoke source {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid karaoke source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &KaraokeManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-KARAOKE"
        || !manifest
            .source_policy
            .contains("English VWF font, eleven-line maps")
        || manifest.font_asset != "../fonts/Galmuri11.ttf"
        || manifest.font_sha256.len() != 64
        || manifest.font_size_px != 12.0
        || manifest.coverage_threshold != 96
        || manifest.lines.len() != LINE_COUNT
    {
        return Err("karaoke manifest identity drifted".to_string());
    }
    if manifest.surface.width_tiles != LINE_WIDTH_TILES
        || manifest.surface.height_tiles != LINE_HEIGHT_TILES
        || manifest.surface.storage != "row_major"
        || parse_u16_hex(&manifest.surface.blank_tile_word)? != 1
        || parse_u16_hex(&manifest.surface.runtime_palette_or)? != 0x4000
    {
        return Err("karaoke surface contract drifted".to_string());
    }

    let group = &manifest.source_group;
    if parse_hex(&group.header_offset)? != GROUP_HEADER_OFFSET
        || group.header_bytes != GROUP_HEADER_BYTES
        || group.header_sha256.len() != 64
        || parse_u16_hex(&group.terminator)? != 0xFFFF
    {
        return Err("karaoke source group declaration drifted".to_string());
    }
    validate_pack_declaration(
        &group.pattern_pack,
        0x80,
        0,
        PATTERN_BYTES,
        PATTERN_BANK_OFFSET,
        PATTERN_BANK_LIMIT,
        "pattern",
    )?;
    validate_pack_declaration(
        &group.map_pack,
        0x00,
        0xA000,
        MAP_BYTES,
        MAP_BANK_OFFSET,
        MAP_BANK_LIMIT,
        "map",
    )?;

    let pattern = &manifest.pattern_ownership;
    if pattern.protected_prefix_tiles.start != 0
        || pattern.protected_prefix_tiles.end_exclusive != MUTABLE_TILE_START
        || pattern.mutable_lyric_tiles.start != MUTABLE_TILE_START
        || pattern.mutable_lyric_tiles.end_exclusive != MUTABLE_TILE_END
        || pattern.protected_suffix_tiles.start != MUTABLE_TILE_END
        || pattern.protected_suffix_tiles.end_exclusive != PATTERN_TILES
        || pattern.mutable_lyric_tiles.source_palette_indices != [0, 1]
    {
        return Err("karaoke pattern ownership drifted".to_string());
    }
    for hash in [
        &pattern.protected_prefix_tiles.source_sha256,
        &pattern.mutable_lyric_tiles.source_sha256,
        &pattern.protected_suffix_tiles.source_sha256,
    ] {
        if hash.len() != 64 {
            return Err("karaoke pattern ownership hash is invalid".to_string());
        }
    }

    let map = &manifest.map_ownership;
    if parse_hex(&map.protected_prefix.start)? != 0
        || parse_hex(&map.protected_prefix.end_exclusive)? != MAP_LYRIC_START
        || parse_hex(&map.lyric_surfaces.start)? != MAP_LYRIC_START
        || parse_hex(&map.lyric_surfaces.end_exclusive)? != MAP_LYRIC_END
        || map.lyric_surfaces.bytes_per_line != LINE_BYTES
        || map.lyric_surfaces.line_count != LINE_COUNT
        || parse_hex(&map.protected_suffix.start)? != MAP_LYRIC_END
        || parse_hex(&map.protected_suffix.end_exclusive)? != MAP_SUFFIX_END
    {
        return Err("karaoke map ownership drifted".to_string());
    }
    for hash in [
        &map.protected_prefix.source_sha256,
        &map.lyric_surfaces.source_sha256,
        &map.protected_suffix.source_sha256,
    ] {
        if hash.len() != 64 {
            return Err("karaoke map ownership hash is invalid".to_string());
        }
    }

    validate_consumer_declaration(&manifest.consumer)?;
    let mut unique = BTreeSet::new();
    for (index, line) in manifest.lines.iter().enumerate() {
        if line.id != format!("GFX-KARAOKE-LINE-{index:02}")
            || line.jp.is_empty()
            || line.ko.is_empty()
            || line.ko.chars().count() > LINE_WIDTH_CELLS
            || line.ko.chars().count() > line.jp.chars().count()
            || line.ko.chars().any(|ch| ch.is_control())
        {
            return Err(format!("karaoke line {index} declaration drifted"));
        }
        unique.extend(line.ko.chars().filter(|ch| !ch.is_whitespace()));
    }
    if unique.is_empty() {
        return Err("karaoke declares no Korean glyphs".to_string());
    }
    Ok(())
}

fn validate_pack_declaration(
    pack: &PackDeclaration,
    command: u8,
    destination: u16,
    decoded_bytes: usize,
    bank_offset: usize,
    bank_limit: usize,
    label: &str,
) -> Result<(), String> {
    let declared_destination = if command == 0x80 {
        &pack.vram_destination
    } else {
        &pack.ram_destination
    };
    if parse_u8_hex(&pack.command)? != command
        || parse_u16_hex(declared_destination)? != destination
        || pack.decoded_bytes != decoded_bytes
        || pack.decoded_sha256.len() != 64
        || parse_hex(&pack.target_bank_offset)? != bank_offset
        || parse_hex(&pack.target_bank_limit)? != bank_limit
    {
        return Err(format!("karaoke {label} pack declaration drifted"));
    }
    Ok(())
}

fn validate_consumer_declaration(consumer: &ConsumerDeclaration) -> Result<(), String> {
    if parse_u32_hex(&consumer.line_ram_base)? != 0x00FF_A8C0
        || parse_hex(&consumer.pointer_table_offset)? != POINTER_TABLE_OFFSET
        || consumer.pointer_table_bytes != POINTER_TABLE_BYTES
        || consumer.pointer_table_sha256.len() != 64
        || parse_hex(&consumer.line_sender_offset)? != LINE_SENDER_OFFSET
        || consumer.line_sender_bytes != LINE_SENDER_BYTES
        || consumer.line_sender_sha256.len() != 64
        || parse_u32_hex(&consumer.line_sender_target)? != 0x0008_FFB0
        || consumer.entries.len() != LINE_COUNT
        || parse_hex(&consumer.timing_table_offset)? != TIMING_TABLE_OFFSET
        || consumer.timing_table_bytes != TIMING_TABLE_BYTES
        || consumer.timing_table_sha256.len() != 64
        || consumer.timing_records.len() != 10
    {
        return Err("karaoke consumer declaration drifted".to_string());
    }
    for (index, entry) in consumer.entries.iter().enumerate() {
        let expected_source = index * LINE_BYTES;
        let expected_destination = if index.is_multiple_of(2) { 0 } else { 0x0300 };
        if parse_hex(&entry.source_offset)? != expected_source
            || parse_hex(&entry.destination_offset)? != expected_destination
        {
            return Err(format!("karaoke pointer entry {index} drifted"));
        }
    }
    let mut previous = 0usize;
    for (index, timing) in consumer.timing_records.iter().enumerate() {
        let timestamp = parse_hex(&timing.timestamp)?;
        if index != 0 && timestamp <= previous {
            return Err("karaoke timing records are not strictly ordered".to_string());
        }
        parse_u32_hex(&timing.step)?;
        previous = timestamp;
    }
    Ok(())
}

fn validate_source_group(source: &[u8], manifest: &KaraokeManifest) -> Result<(), String> {
    let header = source_range(
        source,
        GROUP_HEADER_OFFSET,
        GROUP_HEADER_BYTES,
        "karaoke JP transfer group",
    )?;
    if sha256_hex(header) != manifest.source_group.header_sha256 {
        return Err("karaoke JP transfer group SHA-256 drifted".to_string());
    }
    if header[0] != 0x80
        || header[6] != 0x00
        || u16::from_be_bytes([header[4], header[5]]) != 0
        || u16::from_be_bytes([header[10], header[11]]) != 0xA000
        || u16::from_be_bytes([header[12], header[13]]) != 0xFFFF
    {
        return Err("karaoke JP pattern/map/terminator contract drifted".to_string());
    }
    Ok(())
}

fn checked_pack(
    source: &[u8],
    header_offset: usize,
    destination: u16,
    decoded_bytes: usize,
    expected_sha256: &str,
    label: &str,
) -> Result<Vec<u8>, String> {
    let decoded = decode_mode1_pack_entry(source, header_offset)?;
    let actual_hash = sha256_hex(&decoded.data);
    if decoded.vram_destination != destination
        || decoded.data.len() != decoded_bytes
        || actual_hash != expected_sha256
    {
        return Err(format!(
            "{label} drifted: destination 0x{:04X}, {} bytes, SHA-256 {actual_hash}",
            decoded.vram_destination,
            decoded.data.len()
        ));
    }
    Ok(decoded.data)
}

fn validate_source_ownership(
    patterns: &[u8],
    map: &[u8],
    manifest: &KaraokeManifest,
) -> Result<(), String> {
    if patterns.len() != PATTERN_BYTES || map.len() != MAP_BYTES {
        return Err("karaoke decoded source lengths drifted".to_string());
    }
    checked_slice_hash(
        patterns,
        0,
        MUTABLE_TILE_START * MD_TILE_BYTES,
        &manifest
            .pattern_ownership
            .protected_prefix_tiles
            .source_sha256,
        "karaoke protected pattern prefix",
    )?;
    checked_slice_hash(
        patterns,
        MUTABLE_TILE_START * MD_TILE_BYTES,
        MUTABLE_TILE_END * MD_TILE_BYTES,
        &manifest.pattern_ownership.mutable_lyric_tiles.source_sha256,
        "karaoke mutable JP lyric patterns",
    )?;
    checked_slice_hash(
        patterns,
        MUTABLE_TILE_END * MD_TILE_BYTES,
        PATTERN_BYTES,
        &manifest
            .pattern_ownership
            .protected_suffix_tiles
            .source_sha256,
        "karaoke protected pattern suffix",
    )?;
    let mutable = &patterns[MUTABLE_TILE_START * MD_TILE_BYTES..MUTABLE_TILE_END * MD_TILE_BYTES];
    let roles = mutable
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 1]) {
        return Err(format!(
            "karaoke JP lyric patterns use unexpected palette roles {roles:?}"
        ));
    }
    if patterns[MD_TILE_BYTES..2 * MD_TILE_BYTES]
        .iter()
        .any(|&byte| byte != 0x11)
    {
        return Err("karaoke JP blank tile 0x001 is not the solid palette-1 field".to_string());
    }

    checked_slice_hash(
        map,
        0,
        MAP_LYRIC_START,
        &manifest.map_ownership.protected_prefix.source_sha256,
        "karaoke protected map prefix",
    )?;
    checked_slice_hash(
        map,
        MAP_LYRIC_START,
        MAP_LYRIC_END,
        &manifest.map_ownership.lyric_surfaces.source_sha256,
        "karaoke JP lyric maps",
    )?;
    checked_slice_hash(
        map,
        MAP_LYRIC_END,
        MAP_SUFFIX_END,
        &manifest.map_ownership.protected_suffix.source_sha256,
        "karaoke protected map suffix",
    )?;
    for (word_index, word) in map[MAP_LYRIC_START..MAP_LYRIC_END]
        .chunks_exact(2)
        .enumerate()
    {
        let value = u16::from_be_bytes([word[0], word[1]]);
        let tile = (value & 0x07FF) as usize;
        if value == 1 {
            continue;
        }
        if !(MUTABLE_TILE_START..MUTABLE_TILE_END).contains(&tile) || value & 0xE000 != 0 {
            return Err(format!(
                "karaoke JP lyric map word {word_index} is outside its tile ownership: \
                 0x{value:04X}"
            ));
        }
    }
    Ok(())
}

fn compile_target(
    font: &Font,
    source_patterns: &[u8],
    source_map: &[u8],
    manifest: &KaraokeManifest,
) -> Result<KaraokeTarget, String> {
    let unique = manifest
        .lines
        .iter()
        .flat_map(|line| line.ko.chars())
        .filter(|ch| !ch.is_whitespace())
        .collect::<BTreeSet<_>>();
    let mut target_patterns = source_patterns.to_vec();
    target_patterns[MUTABLE_TILE_START * MD_TILE_BYTES..MUTABLE_TILE_END * MD_TILE_BYTES]
        .fill(0x11);
    let mut glyph_tiles = BTreeMap::new();
    let mut tile_by_pattern = BTreeMap::<[u8; MD_TILE_BYTES], u16>::new();
    let mut next_tile = MUTABLE_TILE_START;
    let blank = parse_u16_hex(&manifest.surface.blank_tile_word)?;
    for ch in unique {
        let surface = render_glyph(font, ch, manifest.font_size_px, manifest.coverage_threshold)?;
        let tiles = encode_md_tiles_column_major(
            &surface,
            GLYPH_CELL_PIXELS,
            GLYPH_CELL_PIXELS,
            "karaoke Korean 16x16 glyph",
        )?;
        if tiles.len() != GLYPH_TILES * MD_TILE_BYTES {
            return Err("karaoke Korean glyph did not encode to four tiles".to_string());
        }
        let mut mapped = [blank; GLYPH_TILES];
        for (index, pattern) in tiles.chunks_exact(MD_TILE_BYTES).enumerate() {
            if pattern.iter().all(|&byte| byte == 0x11) {
                continue;
            }
            let pattern: [u8; MD_TILE_BYTES] = pattern
                .try_into()
                .expect("whole encoded tile has a fixed length");
            let (canonical, flip_flags) = canonicalize_tile_pattern(pattern);
            let tile = if let Some(&tile) = tile_by_pattern.get(&canonical) {
                tile
            } else {
                if next_tile >= MUTABLE_TILE_END {
                    return Err(format!(
                        "karaoke Korean quadrant atlas needs more than {MUTABLE_TILES} JP-owned tiles"
                    ));
                }
                let tile = next_tile as u16;
                let offset = next_tile * MD_TILE_BYTES;
                target_patterns[offset..offset + MD_TILE_BYTES].copy_from_slice(&canonical);
                tile_by_pattern.insert(canonical, tile);
                next_tile += 1;
                tile
            };
            mapped[index] = tile | flip_flags;
        }
        glyph_tiles.insert(ch, mapped);
    }

    let mut target_map = source_map.to_vec();
    for (line_index, line) in manifest.lines.iter().enumerate() {
        let chars = line.ko.chars().collect::<Vec<_>>();
        let source_cells = line.jp.chars().count();
        let origin = (LINE_WIDTH_TILES - source_cells * 2) / 2;
        let mut words = vec![blank; LINE_WIDTH_TILES * LINE_HEIGHT_TILES];
        for (cell, ch) in chars.into_iter().enumerate() {
            if ch.is_whitespace() {
                continue;
            }
            let tiles = glyph_tiles
                .get(&ch)
                .ok_or_else(|| format!("karaoke glyph {ch:?} was not allocated"))?;
            let tile_x = origin + cell * 2;
            words[tile_x] = tiles[0];
            words[tile_x + 1] = tiles[2];
            words[LINE_WIDTH_TILES + tile_x] = tiles[1];
            words[LINE_WIDTH_TILES + tile_x + 1] = tiles[3];
        }
        let encoded_line = words
            .into_iter()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>();
        if encoded_line.len() != LINE_BYTES {
            return Err(format!("karaoke line {line_index} encoded length drifted"));
        }
        let start = MAP_LYRIC_START + line_index * LINE_BYTES;
        target_map[start..start + LINE_BYTES].copy_from_slice(&encoded_line);
    }
    Ok(KaraokeTarget {
        patterns: target_patterns,
        map: target_map,
        glyph_tiles,
    })
}

fn canonicalize_tile_pattern(pattern: [u8; MD_TILE_BYTES]) -> ([u8; MD_TILE_BYTES], u16) {
    let candidates = [
        (pattern, 0x0000),
        (flip_tile_pattern(&pattern, true, false), 0x0800),
        (flip_tile_pattern(&pattern, false, true), 0x1000),
        (flip_tile_pattern(&pattern, true, true), 0x1800),
    ];
    candidates
        .into_iter()
        .min_by_key(|(bytes, flags)| (*bytes, *flags))
        .expect("a tile has four flip candidates")
}

fn flip_tile_pattern(
    pattern: &[u8; MD_TILE_BYTES],
    horizontal: bool,
    vertical: bool,
) -> [u8; MD_TILE_BYTES] {
    let mut flipped = [0u8; MD_TILE_BYTES];
    for y in 0..8 {
        for x in 0..8 {
            let source_x = if horizontal { 7 - x } else { x };
            let source_y = if vertical { 7 - y } else { y };
            let source_byte = pattern[source_y * 4 + source_x / 2];
            let value = if source_x.is_multiple_of(2) {
                source_byte >> 4
            } else {
                source_byte & 0x0F
            };
            let destination = &mut flipped[y * 4 + x / 2];
            if x.is_multiple_of(2) {
                *destination |= value << 4;
            } else {
                *destination |= value;
            }
        }
    }
    flipped
}

fn render_glyph(
    font: &Font,
    ch: char,
    font_size_px: f32,
    coverage_threshold: u8,
) -> Result<[u8; GLYPH_CELL_PIXELS * GLYPH_CELL_PIXELS], String> {
    let (metrics, coverage) = font.rasterize(ch, font_size_px);
    if metrics.width == 0
        || metrics.height == 0
        || metrics.width > GLYPH_CELL_PIXELS
        || metrics.height > GLYPH_CELL_PIXELS
    {
        return Err(format!(
            "karaoke glyph {ch:?} is {}x{}, outside its unscaled 16x16 cell",
            metrics.width, metrics.height
        ));
    }
    // JP karaoke uses palette role 1 as the field and role 0 as the ink.
    let mut surface = [1u8; GLYPH_CELL_PIXELS * GLYPH_CELL_PIXELS];
    let origin_x = (GLYPH_CELL_PIXELS - metrics.width) / 2;
    let origin_y = (GLYPH_CELL_PIXELS - metrics.height) / 2;
    for y in 0..metrics.height {
        for x in 0..metrics.width {
            if coverage[y * metrics.width + x] > coverage_threshold {
                surface[(origin_y + y) * GLYPH_CELL_PIXELS + origin_x + x] = 0;
            }
        }
    }
    if !surface.contains(&0) {
        return Err(format!("karaoke glyph {ch:?} rendered blank"));
    }
    Ok(surface)
}

fn validate_target_ownership(
    source_patterns: &[u8],
    source_map: &[u8],
    target_patterns: &[u8],
    target_map: &[u8],
    glyph_tiles: &BTreeMap<char, [u16; GLYPH_TILES]>,
    manifest: &KaraokeManifest,
) -> Result<(), String> {
    if target_patterns.len() != PATTERN_BYTES
        || target_map.len() != MAP_BYTES
        || target_patterns[..MUTABLE_TILE_START * MD_TILE_BYTES]
            != source_patterns[..MUTABLE_TILE_START * MD_TILE_BYTES]
        || target_patterns[MUTABLE_TILE_END * MD_TILE_BYTES..]
            != source_patterns[MUTABLE_TILE_END * MD_TILE_BYTES..]
        || target_map[..MAP_LYRIC_START] != source_map[..MAP_LYRIC_START]
        || target_map[MAP_LYRIC_END..] != source_map[MAP_LYRIC_END..]
    {
        return Err("karaoke compiler changed protected JP bytes".to_string());
    }
    let roles = target_patterns
        [MUTABLE_TILE_START * MD_TILE_BYTES..MUTABLE_TILE_END * MD_TILE_BYTES]
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 1]) {
        return Err(format!(
            "karaoke Korean patterns use unexpected palette roles {roles:?}"
        ));
    }
    let admitted = glyph_tiles
        .values()
        .flat_map(|tiles| tiles.iter().map(|word| word & 0x07FF))
        .chain(std::iter::once(parse_u16_hex(
            &manifest.surface.blank_tile_word,
        )?))
        .collect::<BTreeSet<_>>();
    let blank = parse_u16_hex(&manifest.surface.blank_tile_word)?;
    for (word_index, word) in target_map[MAP_LYRIC_START..MAP_LYRIC_END]
        .chunks_exact(2)
        .enumerate()
    {
        let value = u16::from_be_bytes([word[0], word[1]]);
        let tile = value & 0x07FF;
        if value & 0xE000 != 0 || (tile != blank && !admitted.contains(&tile)) {
            return Err(format!(
                "karaoke Korean map word {word_index} uses undeclared tile 0x{value:04X}"
            ));
        }
    }
    Ok(())
}

fn validate_consumer(
    rom: &[u8],
    declaration: &ConsumerDeclaration,
    label: &str,
) -> Result<(), String> {
    let pointer_bytes = semantic_pointer_table(declaration)?;
    let actual_pointers = source_range(
        rom,
        POINTER_TABLE_OFFSET,
        POINTER_TABLE_BYTES,
        "karaoke pointer table",
    )?;
    if sha256_hex(actual_pointers) != declaration.pointer_table_sha256
        || actual_pointers != pointer_bytes
    {
        return Err(format!("{label} karaoke pointer table drifted"));
    }

    let sender = karaoke_line_sender()?;
    let actual_sender = source_range(
        rom,
        LINE_SENDER_OFFSET,
        LINE_SENDER_BYTES,
        "karaoke typed line sender",
    )?;
    if sha256_hex(actual_sender) != declaration.line_sender_sha256 || actual_sender != sender {
        return Err(format!("{label} karaoke typed line sender drifted"));
    }

    let timing_bytes = semantic_timing_table(declaration)?;
    let actual_timing = source_range(
        rom,
        TIMING_TABLE_OFFSET,
        TIMING_TABLE_BYTES,
        "karaoke timing table",
    )?;
    if sha256_hex(actual_timing) != declaration.timing_table_sha256 || actual_timing != timing_bytes
    {
        return Err(format!("{label} karaoke timing table drifted"));
    }
    Ok(())
}

fn semantic_pointer_table(declaration: &ConsumerDeclaration) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(POINTER_TABLE_BYTES);
    for entry in &declaration.entries {
        output.extend_from_slice(&parse_u16_hex(&entry.source_offset)?.to_be_bytes());
        output.extend_from_slice(&parse_u16_hex(&entry.destination_offset)?.to_be_bytes());
    }
    if output.len() != POINTER_TABLE_BYTES {
        return Err("karaoke semantic pointer table length drifted".to_string());
    }
    if sha256_hex(&output) != declaration.pointer_table_sha256 {
        return Err("karaoke semantic pointer declaration hash drifted".to_string());
    }
    Ok(output)
}

fn semantic_timing_table(declaration: &ConsumerDeclaration) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(TIMING_TABLE_BYTES);
    for timing in &declaration.timing_records {
        output.extend_from_slice(&parse_u16_hex(&timing.timestamp)?.to_be_bytes());
        output.extend_from_slice(&timing.y.to_be_bytes());
        output.extend_from_slice(&timing.limit.to_be_bytes());
        output.extend_from_slice(&parse_u32_hex(&timing.step)?.to_be_bytes());
    }
    if output.len() != TIMING_TABLE_BYTES {
        return Err("karaoke semantic timing table length drifted".to_string());
    }
    if sha256_hex(&output) != declaration.timing_table_sha256 {
        return Err("karaoke semantic timing declaration hash drifted".to_string());
    }
    Ok(output)
}

fn karaoke_line_sender() -> Result<Vec<u8>, String> {
    m68k::assemble_at(
        LINE_SENDER_OFFSET as u32,
        &[
            Inst::LslWordImmediate {
                count: 2,
                destination: DataReg::D0,
            },
            Inst::LeaProgramCounterIndexedWord {
                displacement: -40,
                index: DataReg::D0,
                destination: AddressReg::A3,
            },
            Inst::LeaAbsoluteLong {
                address: 0x00FF_A8C0,
                destination: AddressReg::A2,
            },
            Inst::AddaWordPostincrementAddress {
                source: AddressReg::A3,
                destination: AddressReg::A2,
            },
            Inst::MoveWordImmediateToData {
                immediate: 0xD602,
                destination: DataReg::D0,
            },
            Inst::AddWordAddressIndirect {
                source: AddressReg::A3,
                destination: DataReg::D0,
            },
            Inst::MoveWordImmediateToData {
                immediate: 0x0025,
                destination: DataReg::D1,
            },
            Inst::MoveWordImmediateToData {
                immediate: 0x0001,
                destination: DataReg::D2,
            },
            Inst::MoveLongImmediateToData {
                immediate: 0x0100_0000,
                destination: DataReg::D3,
            },
            Inst::MoveWordImmediateToData {
                immediate: 0x4000,
                destination: DataReg::D5,
            },
            Inst::BranchAbsolute {
                condition: BranchCondition::Always,
                width: BranchWidth::Word,
                target: 0x0008_FFB0,
            },
        ],
    )
}

fn render_all_surfaces(
    patterns: &[u8],
    map: &[u8],
    manifest: &KaraokeManifest,
    label: &str,
) -> Result<Vec<Vec<u8>>, String> {
    (0..LINE_COUNT)
        .map(|line| render_surface(patterns, map, line, manifest, label))
        .collect()
}

fn render_surface(
    patterns: &[u8],
    map: &[u8],
    line: usize,
    manifest: &KaraokeManifest,
    label: &str,
) -> Result<Vec<u8>, String> {
    if patterns.len() < PATTERN_BYTES
        || !patterns.len().is_multiple_of(MD_TILE_BYTES)
        || map.len() != MAP_BYTES
        || line >= LINE_COUNT
    {
        return Err(format!("{label} inputs are invalid"));
    }
    let start = parse_hex(&manifest.map_ownership.lyric_surfaces.start)? + line * LINE_BYTES;
    let words = source_range(map, start, LINE_BYTES, label)?;
    let mut surface = vec![0u8; LINE_WIDTH_PIXELS * LINE_HEIGHT_PIXELS];
    for tile_x in 0..LINE_WIDTH_TILES {
        for tile_y in 0..LINE_HEIGHT_TILES {
            // The original sender writes 38 consecutive words, advances the
            // VDP destination by one plane row, then writes the next 38.  The
            // source surface is therefore row-major (top row, bottom row),
            // not interleaved top/bottom pairs per column.
            let word_offset = (tile_y * LINE_WIDTH_TILES + tile_x) * 2;
            let word = u16::from_be_bytes([words[word_offset], words[word_offset + 1]]);
            let tile = (word & 0x07FF) as usize;
            if tile >= patterns.len() / MD_TILE_BYTES {
                return Err(format!(
                    "{label} references tile 0x{tile:03X} outside the pack"
                ));
            }
            let horizontal_flip = word & 0x0800 != 0;
            let vertical_flip = word & 0x1000 != 0;
            let pattern = &patterns[tile * MD_TILE_BYTES..(tile + 1) * MD_TILE_BYTES];
            for output_y in 0..8usize {
                for output_x in 0..8usize {
                    let source_x = if horizontal_flip {
                        7 - output_x
                    } else {
                        output_x
                    };
                    let source_y = if vertical_flip {
                        7 - output_y
                    } else {
                        output_y
                    };
                    let byte = pattern[source_y * 4 + source_x / 2];
                    let pixel = if source_x.is_multiple_of(2) {
                        byte >> 4
                    } else {
                        byte & 0x0F
                    };
                    let x = tile_x * 8 + output_x;
                    let y = tile_y * 8 + output_y;
                    surface[y * LINE_WIDTH_PIXELS + x] = pixel;
                }
            }
        }
    }
    Ok(surface)
}

fn draw_surface(
    rgba: &mut [u8],
    output_width: usize,
    logical_x: usize,
    logical_y: usize,
    surface: &[u8],
) -> Result<(), String> {
    if surface.len() != LINE_WIDTH_PIXELS * LINE_HEIGHT_PIXELS {
        return Err("karaoke preview surface length drifted".to_string());
    }
    let output_height = rgba.len() / 4 / output_width;
    let origin_x = logical_x * PREVIEW_SCALE;
    let origin_y = logical_y * PREVIEW_SCALE;
    if origin_x + LINE_WIDTH_PIXELS * PREVIEW_SCALE > output_width
        || origin_y + LINE_HEIGHT_PIXELS * PREVIEW_SCALE > output_height
    {
        return Err("karaoke preview surface is out of bounds".to_string());
    }
    for y in 0..LINE_HEIGHT_PIXELS {
        for x in 0..LINE_WIDTH_PIXELS {
            let color = match surface[y * LINE_WIDTH_PIXELS + x] {
                0 => [32, 24, 18],
                1 => [252, 216, 48],
                value => {
                    return Err(format!(
                        "karaoke preview encountered undeclared palette index {value}"
                    ));
                }
            };
            for scale_y in 0..PREVIEW_SCALE {
                for scale_x in 0..PREVIEW_SCALE {
                    let pixel_x = origin_x + x * PREVIEW_SCALE + scale_x;
                    let pixel_y = origin_y + y * PREVIEW_SCALE + scale_y;
                    let offset = (pixel_y * output_width + pixel_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                }
            }
        }
    }
    Ok(())
}

fn validate_pack_roundtrip(
    header: &[u8; 6],
    bank: &[u8],
    bank_offset: usize,
    destination: u16,
    data: &[u8],
    label: &str,
) -> Result<(), String> {
    let mut probe = vec![0u8; bank_offset + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[bank_offset..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != destination || decoded.data != data {
        return Err(format!("{label} locked lookback round-trip failed"));
    }
    Ok(())
}

fn checked_slice_hash(
    bytes: &[u8],
    start: usize,
    end: usize,
    expected: &str,
    label: &str,
) -> Result<(), String> {
    let slice = source_range(bytes, start, end.saturating_sub(start), label)?;
    let actual = sha256_hex(slice);
    if actual != expected {
        return Err(format!(
            "{label} SHA-256 mismatch: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn parse_u8_hex(value: &str) -> Result<u8, String> {
    u8::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 8 bits"))
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    u16::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 16 bits"))
}

fn parse_u32_hex(value: &str) -> Result<u32, String> {
    u32::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 32 bits"))
}

fn summary(build: &KaraokeBuild, checksum: u16) -> KaraokeSummary {
    debug_assert_eq!(build.target_patterns.len(), build.source_patterns.len());
    debug_assert_eq!(build.source_map.len(), build.target_map.len());
    let blank = parse_u16_hex(&build.manifest.surface.blank_tile_word)
        .expect("validated karaoke blank tile parses");
    let rewritten_pattern_tiles = build
        .glyph_tiles
        .values()
        .flat_map(|tiles| tiles.iter().map(|word| word & 0x07FF))
        .filter(|&tile| tile != blank)
        .collect::<BTreeSet<_>>()
        .len();
    KaraokeSummary {
        lines: build.manifest.lines.len(),
        unique_glyphs: build.glyph_tiles.len(),
        source_pattern_bytes: build.source_patterns.len(),
        rewritten_pattern_tiles,
        rewritten_map_bytes: MAP_LYRIC_END - MAP_LYRIC_START,
        pattern_pack_bytes: build.pattern_bank.len(),
        map_pack_bytes: build.map_bank.len(),
        checksum,
    }
}
