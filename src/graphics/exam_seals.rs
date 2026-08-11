//! JP-source graduation-exam `合格` / `不合格` seal compiler.
//!
//! The two seals share one 192-tile JP mode-1 payload. The pass seal owns
//! tiles 0..74, the score digits and suffix own tiles 74..118, and the fail
//! seal owns tiles 118..192. Only the two seal ranges are rebuilt; the middle
//! 44 tiles and the other three transfers remain byte-identical.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::pixel::{
    PixelBounds, md_color, parse_md_palette, read_verified_rgba, reduce_rgba_to_indexed_surface,
    write_rgba_png,
};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const EXAM_HEADER_OFFSETS: [usize; 4] = [0x09_61A6, 0x09_61AC, 0x09_61B2, 0x09_61B8];
const EXAM_VRAM_DESTINATIONS: [u16; 4] = [0x2000, 0x4000, 0xE000, 0xD000];
const EXAM_DECODED_BYTES: [usize; 4] = [3_520, 6_144, 8_192, 4_096];
const EXAM_TARGET_HEADER_OFFSET: usize = EXAM_HEADER_OFFSETS[1];
const EXAM_TARGET_VRAM: u16 = EXAM_VRAM_DESTINATIONS[1];
const EXAM_SOURCE_BYTES: usize = EXAM_DECODED_BYTES[1];
const EXAM_SOURCE_TILES: usize = EXAM_SOURCE_BYTES / MD_TILE_BYTES;
const EXAM_BANK_OFFSET: usize = 0x2A_0000;
const EXAM_BANK_LIMIT: usize = 0x2A_8000;
const EXAM_SURFACE_WIDTH: usize = 72;
const EXAM_SURFACE_HEIGHT: usize = 72;
const EXAM_SEAL_TILES: usize = 74;
const EXAM_PASS_TILE_START: usize = 0;
const EXAM_PASS_TILE_END: usize = 74;
const EXAM_PROTECTED_TILE_START: usize = 74;
const EXAM_PROTECTED_TILE_END: usize = 118;
const EXAM_FAIL_TILE_START: usize = 118;
const EXAM_FAIL_TILE_END: usize = 192;
const EXAM_SPRITES_PER_SEAL: usize = 8;
const EXAM_DRAW_CALL_TARGET: usize = 0x00001856;
const EXAM_PASS_CONSUMERS: [usize; 4] = [0x09_3E2E, 0x09_3E6C, 0x09_4116, 0x09_4154];
const EXAM_FAIL_CONSUMERS: [usize; 2] = [0x09_4354, 0x09_4392];
const PREVIEW_SCALE: usize = 6;
const PREVIEW_GAP: usize = 4;
const EXAM_TILE_LAYOUT: [[i8; 9]; 9] = [
    [0, 4, 8, 12, 16, 20, 24, 28, -1],
    [1, 5, 9, 13, 17, 21, 25, 29, -1],
    [2, 6, 10, 14, 18, 22, 26, 30, 64],
    [3, 7, 11, 15, 19, 23, 27, 31, 65],
    [32, 36, 40, 44, 48, 52, 56, 60, 66],
    [33, 37, 41, 45, 49, 53, 57, 61, 67],
    [34, 38, 42, 46, 50, 54, 58, 62, 68],
    [35, 39, 43, 47, 51, 55, 59, 63, -1],
    [-1, -1, 69, 70, 71, 72, 73, -1, -1],
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExamSealsSummary {
    pub source_tiles: usize,
    pub rewritten_tiles: usize,
    pub protected_tiles: usize,
    pub companion_transfers: usize,
    pub source_sprite_records: usize,
    pub pack_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct ExamSealsManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    output_surface: OutputSurface,
    palette_line_words: Vec<String>,
    transparent_palette_index: usize,
    source_seal_palette_indices: Vec<usize>,
    allowed_opaque_palette_indices: Vec<usize>,
    tile_layout: Vec<Vec<Option<usize>>>,
    source_packs: Vec<SourcePack>,
    target_pack_id: String,
    protected_tile_range: ProtectedTileRange,
    consumer_loader: ConsumerLoader,
    score_dispatch: ScoreDispatch,
    draw_call_target: String,
    seals: Vec<SealDeclaration>,
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
struct ProtectedTileRange {
    start: usize,
    end_exclusive: usize,
    decoded_sha256: String,
}

#[derive(Debug, Deserialize)]
struct ConsumerLoader {
    lea_offset: String,
    header_group_offset: String,
    loader_call_target: String,
}

#[derive(Debug, Deserialize)]
struct ScoreDispatch {
    compare_offset: String,
    pass_threshold: u16,
    perfect_score: u16,
    fail_branch_target: String,
    regular_pass_branch_target: String,
}

#[derive(Debug, Deserialize)]
struct SealDeclaration {
    id: String,
    text: String,
    master_asset: String,
    master_sha256: String,
    master_width: usize,
    master_height: usize,
    master_alpha_bounds: PixelBounds,
    mutable_tile_range: MutableTileRange,
    sprite_definition: SpriteDefinition,
}

#[derive(Debug, Deserialize)]
struct MutableTileRange {
    start: usize,
    end_exclusive: usize,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
struct SpriteDefinition {
    offset: String,
    consumers: Vec<String>,
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
struct ExamSealsBuild {
    manifest: ExamSealsManifest,
    palette: [u16; 16],
    source_payloads: Vec<Vec<u8>>,
    payload: Vec<u8>,
    surfaces: Vec<Vec<u8>>,
    header: [u8; 6],
    bank: Vec<u8>,
}

/// Insert both Korean exam seals into the cumulative JP-to-KR ROM.
pub fn apply_exam_seals(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<ExamSealsSummary, String> {
    let build = build_exam_seals(source, assets_dir)?;
    let bank_end = EXAM_BANK_OFFSET + build.bank.len();
    if bank_end > EXAM_BANK_LIMIT || bank_end > output.len() {
        return Err(format!(
            "exam seal pack ends outside its expanded bank at 0x{bank_end:06X}"
        ));
    }

    let baseline = output.to_vec();
    let mut changed_ranges = Vec::with_capacity(3);
    apply_expected_write(
        output,
        EXAM_TARGET_HEADER_OFFSET,
        source_range(
            source,
            EXAM_TARGET_HEADER_OFFSET,
            build.header.len(),
            "exam source high-pattern header",
        )?,
        &build.header,
        "exam high-pattern header",
    )?;
    changed_ranges.push((
        EXAM_TARGET_HEADER_OFFSET,
        EXAM_TARGET_HEADER_OFFSET + build.header.len(),
    ));

    apply_expected_write(
        output,
        EXAM_BANK_OFFSET,
        &vec![0xFF; build.bank.len()],
        &build.bank,
        "exam expanded high-pattern pack",
    )?;
    changed_ranges.push((EXAM_BANK_OFFSET, bank_end));

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after exam seals",
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
                "inserted exam group does not match transfer {}",
                declaration.id
            ));
        }
    }

    eprintln!("JP graphics GFX-EXAM-SEAL-PASS/FAIL Expected Writes:");
    eprintln!(
        "  0x{EXAM_TARGET_HEADER_OFFSET:06X}..0x{:06X}  exam high-pattern header ({} bytes)",
        EXAM_TARGET_HEADER_OFFSET + build.header.len(),
        build.header.len()
    );
    eprintln!(
        "  0x{EXAM_BANK_OFFSET:06X}..0x{bank_end:06X}  exam high-pattern pack ({} bytes)",
        build.bank.len()
    );
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");

    Ok(summary(&build, checksum))
}

/// Render a checker-backed contact sheet of the exact pass and fail surfaces.
pub fn write_exam_seals_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<ExamSealsSummary, String> {
    let build = build_exam_seals(source, assets_dir)?;
    let surface = build.manifest.output_surface;
    let contact_width = surface.width * 2 + PREVIEW_GAP;
    let preview_width = contact_width * PREVIEW_SCALE;
    let preview_height = surface.height * PREVIEW_SCALE;
    let mut rgba = vec![0u8; preview_width * preview_height * 4];

    for (panel, pixels) in build.surfaces.iter().enumerate() {
        let panel_x = panel * (surface.width + PREVIEW_GAP);
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
                write_scaled_pixel(
                    &mut rgba,
                    preview_width,
                    panel_x + x,
                    y,
                    PREVIEW_SCALE,
                    color,
                );
            }
        }
    }

    for x in surface.width..surface.width + PREVIEW_GAP {
        for y in 0..surface.height {
            write_scaled_pixel(&mut rgba, preview_width, x, y, PREVIEW_SCALE, [96, 96, 96]);
        }
    }

    write_rgba_png(
        output_path,
        preview_width as u32,
        preview_height as u32,
        &rgba,
        "exam seals",
    )?;
    Ok(summary(&build, 0))
}

fn build_exam_seals(source: &[u8], assets_dir: &Path) -> Result<ExamSealsBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let palette = parse_md_palette(&manifest.palette_line_words, "exam seals")?;

    let mut source_payloads = Vec::with_capacity(manifest.source_packs.len());
    for declaration in &manifest.source_packs {
        source_payloads.push(checked_source_pack(source, declaration)?);
    }
    let target_index = manifest
        .source_packs
        .iter()
        .position(|pack| pack.id == manifest.target_pack_id)
        .ok_or_else(|| "exam target pack is absent".to_string())?;
    let source_payload = &source_payloads[target_index];
    if source_payload.len() != EXAM_SOURCE_BYTES {
        return Err("exam target payload length drifted".to_string());
    }
    validate_source_tile_ranges(source_payload, &manifest)?;
    validate_consumer_loader(source, &manifest)?;
    validate_score_dispatch(source, &manifest)?;

    let mut payload = source_payload.clone();
    let mut surfaces = Vec::with_capacity(manifest.seals.len());
    for seal in &manifest.seals {
        validate_sprite_definition(source, seal, &manifest)?;
        let master = read_seal_master(assets_dir, seal)?;
        let pixels = reduce_seal_master(&master, seal, &manifest, &palette)?;
        let replacement = encode_layout_surface(&pixels, &manifest)?;
        let start = seal.mutable_tile_range.start * MD_TILE_BYTES;
        let end = seal.mutable_tile_range.end_exclusive * MD_TILE_BYTES;
        if replacement.len() != end - start {
            return Err(format!("{} replacement tile length drifted", seal.id));
        }
        payload[start..end].copy_from_slice(&replacement);
        surfaces.push(pixels);
    }

    let protected_start = EXAM_PROTECTED_TILE_START * MD_TILE_BYTES;
    let protected_end = EXAM_PROTECTED_TILE_END * MD_TILE_BYTES;
    if payload[protected_start..protected_end] != source_payload[protected_start..protected_end] {
        return Err("exam compiler changed protected score or suffix tiles".to_string());
    }

    let encoded = encode_locked_mode1_pack(EXAM_BANK_OFFSET, EXAM_TARGET_VRAM, &payload)?;
    validate_pack_roundtrip(&encoded.header, &encoded.bank, EXAM_TARGET_VRAM, &payload)?;

    Ok(ExamSealsBuild {
        manifest,
        palette,
        source_payloads,
        payload,
        surfaces,
        header: encoded.header,
        bank: encoded.bank,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<ExamSealsManifest, String> {
    let path = assets_dir.join("graphics_text/exam_seals.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read exam seal graphics source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid exam seal source {}: {error}", path.display()))
}

fn read_seal_master(assets_dir: &Path, seal: &SealDeclaration) -> Result<Vec<u8>, String> {
    read_verified_rgba(
        assets_dir,
        &seal.master_asset,
        &seal.master_sha256,
        seal.master_width,
        seal.master_height,
        seal.master_alpha_bounds,
        &format!("{} master", seal.id),
    )
}

fn reduce_seal_master(
    master: &[u8],
    seal: &SealDeclaration,
    manifest: &ExamSealsManifest,
    palette: &[u16; 16],
) -> Result<Vec<u8>, String> {
    let surface = manifest.output_surface;
    reduce_rgba_to_indexed_surface(
        master,
        seal.master_width,
        seal.master_height,
        seal.master_alpha_bounds,
        surface.width,
        surface.height,
        surface.content_box,
        surface.alpha_threshold,
        manifest.transparent_palette_index,
        palette,
        &manifest.allowed_opaque_palette_indices,
        &seal.id,
    )
}

fn validate_manifest_shape(manifest: &ExamSealsManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-EXAM-SEALS"
        || !manifest.source_policy.contains("JP")
    {
        return Err("unsupported exam seal manifest identity".to_string());
    }
    let surface = manifest.output_surface;
    if surface.width != EXAM_SURFACE_WIDTH
        || surface.height != EXAM_SURFACE_HEIGHT
        || surface.content_box
            != (PixelBounds {
                x: 3,
                y: 4,
                width: 61,
                height: 64,
            })
        || surface.alpha_threshold == 0
        || surface.alpha_threshold == u8::MAX
    {
        return Err("exam seal output surface drifted".to_string());
    }
    if manifest.transparent_palette_index != 0
        || manifest
            .source_seal_palette_indices
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != BTreeSet::from([0, 9, 10, 11])
        || manifest
            .allowed_opaque_palette_indices
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != BTreeSet::from([9, 10, 11])
    {
        return Err("exam seal palette roles drifted".to_string());
    }
    validate_tile_layout(manifest)?;

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
    let expected = EXAM_HEADER_OFFSETS
        .into_iter()
        .zip(EXAM_VRAM_DESTINATIONS)
        .zip(EXAM_DECODED_BYTES)
        .map(|((header, vram), bytes)| (header, vram, bytes))
        .collect::<Vec<_>>();
    if anchors != expected
        || manifest.target_pack_id != "exam-patterns-high"
        || manifest.source_packs[1].id != manifest.target_pack_id
    {
        return Err("exam JP transfer set drifted".to_string());
    }
    if manifest.protected_tile_range.start != EXAM_PROTECTED_TILE_START
        || manifest.protected_tile_range.end_exclusive != EXAM_PROTECTED_TILE_END
    {
        return Err("exam protected score range drifted".to_string());
    }
    if parse_hex(&manifest.consumer_loader.lea_offset)? != 0x09_3D4A
        || parse_hex(&manifest.consumer_loader.header_group_offset)? != EXAM_HEADER_OFFSETS[0]
        || parse_hex(&manifest.consumer_loader.loader_call_target)? != 0x000A72
        || parse_hex(&manifest.score_dispatch.compare_offset)? != 0x09_3DF2
        || manifest.score_dispatch.pass_threshold != 90
        || manifest.score_dispatch.perfect_score != 100
        || parse_hex(&manifest.score_dispatch.fail_branch_target)? != 0x09_4330
        || parse_hex(&manifest.score_dispatch.regular_pass_branch_target)? != 0x09_40F2
        || parse_hex(&manifest.draw_call_target)? != EXAM_DRAW_CALL_TARGET
    {
        return Err("exam consumer declarations drifted".to_string());
    }
    if manifest.seals.len() != 2 {
        return Err("exam seal manifest must declare pass and fail".to_string());
    }
    let expected_seals = [
        (
            "GFX-EXAM-SEAL-PASS",
            "합격",
            EXAM_PASS_TILE_START,
            EXAM_PASS_TILE_END,
            0x09_761A,
            EXAM_PASS_CONSUMERS.as_slice(),
        ),
        (
            "GFX-EXAM-SEAL-FAIL",
            "불합격",
            EXAM_FAIL_TILE_START,
            EXAM_FAIL_TILE_END,
            0x09_765C,
            EXAM_FAIL_CONSUMERS.as_slice(),
        ),
    ];
    for (seal, expected) in manifest.seals.iter().zip(expected_seals) {
        let consumers = seal
            .sprite_definition
            .consumers
            .iter()
            .map(|value| parse_hex(value))
            .collect::<Result<Vec<_>, _>>()?;
        if seal.id != expected.0
            || seal.text != expected.1
            || seal.mutable_tile_range.start != expected.2
            || seal.mutable_tile_range.end_exclusive != expected.3
            || parse_hex(&seal.sprite_definition.offset)? != expected.4
            || consumers.as_slice() != expected.5
            || seal.sprite_definition.records.len() != EXAM_SPRITES_PER_SEAL
        {
            return Err(format!("{} declaration drifted", expected.0));
        }
    }
    Ok(())
}

fn validate_tile_layout(manifest: &ExamSealsManifest) -> Result<(), String> {
    if manifest.tile_layout.len() != EXAM_TILE_LAYOUT.len() {
        return Err("exam tile layout must have nine rows".to_string());
    }
    let mut mapped = BTreeSet::new();
    for (row, expected_row) in manifest.tile_layout.iter().zip(EXAM_TILE_LAYOUT) {
        if row.len() != expected_row.len() {
            return Err("exam tile layout must have nine columns".to_string());
        }
        for (value, expected) in row.iter().zip(expected_row) {
            let actual = value.map_or(-1, |tile| tile as i8);
            if actual != expected {
                return Err("exam tile layout drifted from the JP structure".to_string());
            }
            if let Some(tile) = value {
                mapped.insert(*tile);
            }
        }
    }
    if mapped != (0..EXAM_SEAL_TILES).collect::<BTreeSet<_>>() {
        return Err("exam tile layout does not cover each seal tile once".to_string());
    }
    Ok(())
}

fn checked_source_pack(source: &[u8], declaration: &SourcePack) -> Result<Vec<u8>, String> {
    let header_offset = parse_hex(&declaration.header_offset)?;
    let decoded = decode_mode1_pack_entry(source, header_offset)?;
    let expected_vram = parse_u16_hex(&declaration.vram_destination)?;
    if decoded.vram_destination != expected_vram {
        return Err(format!(
            "exam transfer {} targets VRAM 0x{:04X}, expected 0x{expected_vram:04X}",
            declaration.id, decoded.vram_destination
        ));
    }
    if decoded.data.len() != declaration.decoded_bytes {
        return Err(format!(
            "exam transfer {} decoded to {} bytes, expected {}",
            declaration.id,
            decoded.data.len(),
            declaration.decoded_bytes
        ));
    }
    let hash = sha256_hex(&decoded.data);
    if hash != declaration.decoded_sha256 {
        return Err(format!(
            "exam transfer {} SHA-256 mismatch: expected {}, got {hash}",
            declaration.id, declaration.decoded_sha256
        ));
    }
    Ok(decoded.data)
}

fn validate_source_tile_ranges(
    source_payload: &[u8],
    manifest: &ExamSealsManifest,
) -> Result<(), String> {
    for seal in &manifest.seals {
        let start = seal.mutable_tile_range.start * MD_TILE_BYTES;
        let end = seal.mutable_tile_range.end_exclusive * MD_TILE_BYTES;
        let hash = sha256_hex(&source_payload[start..end]);
        if hash != seal.mutable_tile_range.source_sha256 {
            return Err(format!("{} source tile range SHA-256 drifted", seal.id));
        }
    }
    let protected = &manifest.protected_tile_range;
    let start = protected.start * MD_TILE_BYTES;
    let end = protected.end_exclusive * MD_TILE_BYTES;
    if sha256_hex(&source_payload[start..end]) != protected.decoded_sha256 {
        return Err("exam protected score tile SHA-256 drifted".to_string());
    }
    Ok(())
}

fn validate_consumer_loader(source: &[u8], manifest: &ExamSealsManifest) -> Result<(), String> {
    let loader = &manifest.consumer_loader;
    let lea_offset = parse_hex(&loader.lea_offset)?;
    let bytes = source_range(source, lea_offset, 10, "exam graphics loader")?;
    if u16::from_be_bytes([bytes[0], bytes[1]]) != 0x45FA {
        return Err("exam consumer no longer loads its group with LEA d16(PC),A2".to_string());
    }
    let displacement = i16::from_be_bytes([bytes[2], bytes[3]]) as isize;
    let target = (lea_offset as isize + 2 + displacement) as usize;
    if target != parse_hex(&loader.header_group_offset)? {
        return Err(format!(
            "exam loader resolves to 0x{target:06X}, not its declared header group"
        ));
    }
    if u16::from_be_bytes([bytes[4], bytes[5]]) != 0x4EB9 {
        return Err("exam group load is not followed by JSR abs.l".to_string());
    }
    let call_target = u32::from_be_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
    if call_target != parse_hex(&loader.loader_call_target)? {
        return Err(format!(
            "exam graphics loader call drifted to 0x{call_target:06X}"
        ));
    }
    Ok(())
}

fn validate_score_dispatch(source: &[u8], manifest: &ExamSealsManifest) -> Result<(), String> {
    let dispatch = &manifest.score_dispatch;
    let offset = parse_hex(&dispatch.compare_offset)?;
    let bytes = source_range(source, offset, 16, "exam score dispatch")?;
    let words = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    if words[0] != 0x0C00
        || words[1] != dispatch.pass_threshold
        || words[2] != 0x6500
        || words[4] != 0x0C00
        || words[5] != dispatch.perfect_score
        || words[6] != 0x6600
    {
        return Err("exam score comparison sequence drifted".to_string());
    }
    let fail_target = branch_word_target(offset + 4, words[3]);
    let pass_target = branch_word_target(offset + 12, words[7]);
    if fail_target != parse_hex(&dispatch.fail_branch_target)?
        || pass_target != parse_hex(&dispatch.regular_pass_branch_target)?
    {
        return Err("exam score branch target drifted".to_string());
    }
    Ok(())
}

fn branch_word_target(instruction_offset: usize, displacement: u16) -> usize {
    (instruction_offset as isize + 2 + i16::from_be_bytes(displacement.to_be_bytes()) as isize)
        as usize
}

fn validate_sprite_definition(
    source: &[u8],
    seal: &SealDeclaration,
    manifest: &ExamSealsManifest,
) -> Result<(), String> {
    let expected = encode_sprite_definition(&seal.sprite_definition)?;
    let definition_offset = parse_hex(&seal.sprite_definition.offset)?;
    let actual = source_range(
        source,
        definition_offset,
        expected.len(),
        &format!("{} source sprite definition", seal.id),
    )?;
    if actual != expected {
        return Err(format!("{} source sprite definition drifted", seal.id));
    }

    let covered = sprite_tile_coverage(seal, manifest)?;
    let expected_coverage = (seal.mutable_tile_range.start..seal.mutable_tile_range.end_exclusive)
        .collect::<BTreeSet<_>>();
    if covered != expected_coverage {
        return Err(format!(
            "{} sprite records do not own its tile range",
            seal.id
        ));
    }

    let draw_target = parse_hex(&manifest.draw_call_target)?;
    for consumer in &seal.sprite_definition.consumers {
        let offset = parse_hex(consumer)?;
        let bytes = source_range(source, offset, 18, &format!("{} draw consumer", seal.id))?;
        let target = u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]) as usize;
        let call_target = u32::from_be_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]) as usize;
        if u16::from_be_bytes([bytes[0], bytes[1]]) != 0x45F9
            || target != definition_offset
            || u16::from_be_bytes([bytes[6], bytes[7]]) != 0x4241
            || u16::from_be_bytes([bytes[8], bytes[9]]) != 0x4242
            || u16::from_be_bytes([bytes[10], bytes[11]]) != 0x4243
            || u16::from_be_bytes([bytes[12], bytes[13]]) != 0x4EB9
            || call_target != draw_target
        {
            return Err(format!(
                "{} draw consumer 0x{offset:06X} no longer binds its source sprite definition",
                seal.id
            ));
        }
    }
    Ok(())
}

fn encode_sprite_definition(definition: &SpriteDefinition) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(2 + definition.records.len() * 8);
    append_word(
        &mut output,
        u16::try_from(definition.records.len())
            .map_err(|_| "exam sprite count is too large".to_string())?,
    );
    for record in &definition.records {
        append_word(&mut output, parse_u16_hex(&record.y)?);
        append_word(&mut output, parse_u16_hex(&record.size_link)?);
        append_word(&mut output, parse_u16_hex(&record.tile)?);
        append_word(&mut output, parse_u16_hex(&record.x)?);
    }
    Ok(output)
}

fn sprite_tile_coverage(
    seal: &SealDeclaration,
    _manifest: &ExamSealsManifest,
) -> Result<BTreeSet<usize>, String> {
    let vram_base = usize::from(EXAM_TARGET_VRAM) / MD_TILE_BYTES;
    let mut covered = BTreeSet::new();
    for record in &seal.sprite_definition.records {
        let size_link = parse_u16_hex(&record.size_link)?;
        let size = (size_link >> 8) as u8;
        let width = usize::from((size >> 2) & 0x03) + 1;
        let height = usize::from(size & 0x03) + 1;
        let tile_word = parse_u16_hex(&record.tile)?;
        if tile_word & !0x07FF != 0xA000 {
            return Err(format!("{} sprite tile flags drifted", seal.id));
        }
        let tile = usize::from(tile_word & 0x07FF);
        if tile < vram_base {
            return Err(format!(
                "{} sprite tile precedes its VRAM transfer",
                seal.id
            ));
        }
        let relative = tile - vram_base;
        for index in relative..relative + width * height {
            if !covered.insert(index) {
                return Err(format!("{} sprite tile coverage overlaps", seal.id));
            }
        }
    }
    Ok(covered)
}

fn encode_layout_surface(pixels: &[u8], manifest: &ExamSealsManifest) -> Result<Vec<u8>, String> {
    let surface = manifest.output_surface;
    if pixels.len() != surface.width * surface.height
        || surface.width / 8 != 9
        || surface.height / 8 != 9
    {
        return Err("exam seal pixel surface is not 9x9 tiles".to_string());
    }
    let mut output = vec![0u8; EXAM_SEAL_TILES * MD_TILE_BYTES];
    for (tile_y, row) in manifest.tile_layout.iter().enumerate() {
        for (tile_x, destination) in row.iter().enumerate() {
            match destination {
                Some(tile) => encode_surface_tile(
                    pixels,
                    surface.width,
                    tile_x,
                    tile_y,
                    &mut output[*tile * MD_TILE_BYTES..(*tile + 1) * MD_TILE_BYTES],
                )?,
                None => {
                    for y in tile_y * 8..tile_y * 8 + 8 {
                        for x in tile_x * 8..tile_x * 8 + 8 {
                            if pixels[y * surface.width + x]
                                != manifest.transparent_palette_index as u8
                            {
                                return Err(format!(
                                    "exam seal master uses non-owned sparse tile ({tile_x}, {tile_y})"
                                ));
                            }
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
    tile_x: usize,
    tile_y: usize,
    output: &mut [u8],
) -> Result<(), String> {
    if output.len() != MD_TILE_BYTES {
        return Err("exam tile output length is invalid".to_string());
    }
    for local_y in 0..8 {
        for pair_x in 0..4 {
            let x = tile_x * 8 + pair_x * 2;
            let y = tile_y * 8 + local_y;
            let left = pixels[y * surface_width + x];
            let right = pixels[y * surface_width + x + 1];
            if left >= 16 || right >= 16 {
                return Err("exam seal uses an invalid palette index".to_string());
            }
            output[local_y * 4 + pair_x] = (left << 4) | right;
        }
    }
    Ok(())
}

fn write_scaled_pixel(
    rgba: &mut [u8],
    preview_width: usize,
    x: usize,
    y: usize,
    scale: usize,
    color: [u8; 3],
) {
    for scale_y in 0..scale {
        for scale_x in 0..scale {
            let preview_x = x * scale + scale_x;
            let preview_y = y * scale + scale_y;
            let offset = (preview_y * preview_width + preview_x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
        }
    }
}

fn validate_pack_roundtrip(
    header: &[u8; 6],
    bank: &[u8],
    expected_vram: u16,
    expected_payload: &[u8],
) -> Result<(), String> {
    let mut probe = vec![0u8; EXAM_BANK_OFFSET + bank.len()];
    probe[0x100..0x106].copy_from_slice(header);
    probe[EXAM_BANK_OFFSET..].copy_from_slice(bank);
    let decoded = decode_mode1_pack_entry(&probe, 0x100)?;
    if decoded.vram_destination != expected_vram || decoded.data != expected_payload {
        return Err("exam seal mode-1 semantic round-trip failed".to_string());
    }
    Ok(())
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("{value} does not fit in a 16-bit value"))
}

fn append_word(output: &mut Vec<u8>, word: u16) {
    output.extend_from_slice(&word.to_be_bytes());
}

fn summary(build: &ExamSealsBuild, checksum: u16) -> ExamSealsSummary {
    ExamSealsSummary {
        source_tiles: EXAM_SOURCE_TILES,
        rewritten_tiles: EXAM_SEAL_TILES * 2,
        protected_tiles: EXAM_PROTECTED_TILE_END - EXAM_PROTECTED_TILE_START,
        companion_transfers: build.source_payloads.len() - 1,
        source_sprite_records: EXAM_SPRITES_PER_SEAL * 2,
        pack_bytes: build.bank.len(),
        checksum,
    }
}
