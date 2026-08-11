//! JP-source compiler for the two timed-sequence `あと [digits] 秒` consumers.
//!
//! Both consumers share one packed pattern payload and one irregular relative
//! frame table. Korean `앞으로 [digits]초` is rendered from NeoDGM at native
//! 16x16. The original digit patterns stay byte-identical. The rebuilt label
//! frames and source digits live in a non-executable table bank. The original
//! object-type pointer is rebound to that relocated table; only two LEAs and
//! five horizontal `ADDI.W` immediates are executable rewrites, all generated
//! through typed ISA variants.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use crate::m68k::{self, AddressReg, DataReg, Inst};

use super::font_effect::{read_verified_font, render_native_text_line};
use super::pixel::{decode_md_tiles_column_major, encode_md_tiles_column_major, write_rgba_png};
use super::sprite_map::{SpriteFrame, SpriteRecord};
use super::{
    CHECKSUM_OFFSET, MD_TILE_BYTES, apply_expected_write, calculate_checksum,
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
    validate_only_ranges_changed,
};

const HEADER_BYTES: usize = 6;
const SOURCE_VRAM: u16 = 0x5600;
const SOURCE_TILE: usize = 0x02B0;
const SOURCE_PAYLOAD_BYTES: usize = 1664;
const SOURCE_PREFIX_TILES: usize = 12;
const SOURCE_DIGIT_TILES: usize = 40;
const TARGET_PREFIX_CELLS: usize = 3;
const TARGET_SUFFIX_CELLS: usize = 1;
const CELL_TILES: usize = 4;
const TARGET_PAYLOAD_TILES: usize =
    SOURCE_PREFIX_TILES + SOURCE_DIGIT_TILES + TARGET_SUFFIX_CELLS * CELL_TILES;
const SOURCE_TABLE_OFFSET: usize = 0x06_739A;
const SOURCE_TABLE_BYTES: usize = 160;
const OBJECT_POINTER_TABLE_OFFSET: usize = 0x06_AD18;
const OBJECT_POINTER_ENTRY_BYTES: usize = 4;
const TIMER_OBJECT_TYPE: usize = 0x22;
const SOURCE_TYPE_POINTER_OFFSET: usize =
    OBJECT_POINTER_TABLE_OFFSET + TIMER_OBJECT_TYPE * OBJECT_POINTER_ENTRY_BYTES;
const TABLE_HEADER_WORDS: usize = 12;
const TARGET_PACK_BANK_OFFSET: usize = 0x32_0000;
const TARGET_PACK_BANK_LIMIT: usize = 0x32_8000;
const TARGET_TABLE_BANK_OFFSET: usize = 0x32_A000;
const TARGET_TABLE_BANK_LIMIT: usize = 0x32_C000;
const BANK_FILL: u8 = 0xFF;
const PREVIEW_SCALE: usize = 3;
const PREVIEW_GAP: usize = 16;
const PREVIEW_WIDTH: usize = 120;
const PREVIEW_HEIGHT: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerRemainingSummary {
    pub alias_headers: usize,
    pub source_tiles: usize,
    pub rewritten_tiles: usize,
    pub protected_digit_tiles: usize,
    pub table_bytes: usize,
    pub written_executable_bytes: usize,
    pub verified_consumer_bytes: usize,
    pub pack_bank_bytes: usize,
    pub checksum: u16,
}

#[derive(Debug, Deserialize)]
struct TimerRemainingManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    jp: String,
    ko: String,
    font_asset: String,
    font_sha256: String,
    render_mode: String,
    source_pack: SourcePackDeclaration,
    source_table: SourceTableDeclaration,
    object_type_binding: ObjectTypeBindingDeclaration,
    target_pack_bank: BankDeclaration,
    target_table_bank: BankDeclaration,
    consumers: Vec<ConsumerDeclaration>,
}

#[derive(Debug, Deserialize)]
struct SourcePackDeclaration {
    header_offsets: Vec<String>,
    header_sha256: String,
    vram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
    prefix_tiles: usize,
    digit_tiles: usize,
}

#[derive(Debug, Deserialize)]
struct SourceTableDeclaration {
    offset: String,
    bytes: usize,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct ObjectTypeBindingDeclaration {
    pointer_table_offset: String,
    pointer_entry_bytes: usize,
    object_type: String,
    type_pointer_offset: String,
    type_pointer_sha256: String,
}

#[derive(Debug, Deserialize)]
struct BankDeclaration {
    offset: String,
    limit: String,
    fill_byte: String,
}

#[derive(Debug, Deserialize)]
struct ConsumerDeclaration {
    id: String,
    profile: String,
    window_offset: String,
    window_bytes: usize,
    window_sha256: String,
    table_lea_offset: String,
    digit_offsets: Vec<DigitOffsetDeclaration>,
    suffix_x: String,
}

#[derive(Debug, Deserialize)]
struct DigitOffsetDeclaration {
    instruction_offset: String,
    source_x: String,
    target_x: String,
}

#[derive(Debug)]
struct CodePatch {
    offset: usize,
    source: Vec<u8>,
    target: Vec<u8>,
    label: String,
}

#[derive(Debug)]
struct DataPatch {
    offset: usize,
    source: Vec<u8>,
    target: Vec<u8>,
    label: String,
}

#[derive(Debug)]
struct TimerRemainingBuild {
    manifest: TimerRemainingManifest,
    source_payload: Vec<u8>,
    target_payload: Vec<u8>,
    target_table: Vec<u8>,
    source_header: Vec<u8>,
    target_header: Vec<u8>,
    pack_bank: Vec<u8>,
    type_pointer_patch: DataPatch,
    code_patches: Vec<CodePatch>,
    source_preview: Vec<u8>,
    target_preview: Vec<u8>,
}

pub fn apply_timer_remaining(
    source: &[u8],
    output: &mut [u8],
    assets_dir: &Path,
) -> Result<TimerRemainingSummary, String> {
    let build = build(source, assets_dir)?;
    if TARGET_PACK_BANK_OFFSET + build.pack_bank.len() > TARGET_PACK_BANK_LIMIT
        || TARGET_PACK_BANK_OFFSET + build.pack_bank.len() > output.len()
        || TARGET_TABLE_BANK_OFFSET + build.target_table.len() > TARGET_TABLE_BANK_LIMIT
        || TARGET_TABLE_BANK_OFFSET + build.target_table.len() > output.len()
    {
        return Err("timer target bank exceeds its allocation".to_string());
    }
    let baseline = output.to_vec();
    let mut changed_ranges = Vec::new();
    for header in &build.manifest.source_pack.header_offsets {
        let offset = parse_hex(header)?;
        apply_expected_write(
            output,
            offset,
            &build.source_header,
            &build.target_header,
            "timer Korean pattern header alias",
        )?;
        changed_ranges.push((offset, offset + HEADER_BYTES));
    }
    apply_expected_write(
        output,
        TARGET_PACK_BANK_OFFSET,
        &vec![BANK_FILL; build.pack_bank.len()],
        &build.pack_bank,
        "timer Korean pattern pack bank",
    )?;
    changed_ranges.push((
        TARGET_PACK_BANK_OFFSET,
        TARGET_PACK_BANK_OFFSET + build.pack_bank.len(),
    ));
    apply_expected_write(
        output,
        TARGET_TABLE_BANK_OFFSET,
        &vec![BANK_FILL; build.target_table.len()],
        &build.target_table,
        "timer Korean sprite-table bank",
    )?;
    changed_ranges.push((
        TARGET_TABLE_BANK_OFFSET,
        TARGET_TABLE_BANK_OFFSET + build.target_table.len(),
    ));
    apply_expected_write(
        output,
        build.type_pointer_patch.offset,
        &build.type_pointer_patch.source,
        &build.type_pointer_patch.target,
        &build.type_pointer_patch.label,
    )?;
    changed_ranges.push((
        build.type_pointer_patch.offset,
        build.type_pointer_patch.offset + build.type_pointer_patch.target.len(),
    ));
    for patch in &build.code_patches {
        apply_expected_write(
            output,
            patch.offset,
            &patch.source,
            &patch.target,
            &patch.label,
        )?;
        changed_ranges.push((patch.offset, patch.offset + patch.target.len()));
    }

    let checksum = calculate_checksum(output);
    apply_expected_write(
        output,
        CHECKSUM_OFFSET,
        &baseline[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 2],
        &checksum.to_be_bytes(),
        "Mega Drive checksum after timer remaining label",
    )?;
    changed_ranges.push((CHECKSUM_OFFSET, CHECKSUM_OFFSET + 2));
    validate_only_ranges_changed(&baseline, output, &changed_ranges)?;
    validate_output(source, output, &build)?;

    eprintln!("JP graphics GFX-TIMER-REMAINING Expected Writes:");
    for header in &build.manifest.source_pack.header_offsets {
        let offset = parse_hex(header)?;
        eprintln!(
            "  0x{offset:06X}..0x{:06X}  shared Korean timer pack header",
            offset + HEADER_BYTES
        );
    }
    eprintln!(
        "  0x{TARGET_PACK_BANK_OFFSET:06X}..0x{:06X}  Korean timer pattern pack ({} bytes)",
        TARGET_PACK_BANK_OFFSET + build.pack_bank.len(),
        build.pack_bank.len()
    );
    eprintln!(
        "  0x{TARGET_TABLE_BANK_OFFSET:06X}..0x{:06X}  Korean timer sprite table ({} bytes)",
        TARGET_TABLE_BANK_OFFSET + build.target_table.len(),
        build.target_table.len()
    );
    eprintln!(
        "  0x{:06X}..0x{:06X}  {}",
        build.type_pointer_patch.offset,
        build.type_pointer_patch.offset + build.type_pointer_patch.target.len(),
        build.type_pointer_patch.label
    );
    for patch in &build.code_patches {
        eprintln!(
            "  0x{:06X}..0x{:06X}  {}",
            patch.offset,
            patch.offset + patch.target.len(),
            patch.label
        );
    }
    eprintln!("  0x{CHECKSUM_OFFSET:06X}..0x000190  checksum -> 0x{checksum:04X}");
    Ok(summary(&build, checksum))
}

pub fn write_timer_remaining_preview(
    source: &[u8],
    assets_dir: &Path,
    output_path: &Path,
) -> Result<TimerRemainingSummary, String> {
    let build = build(source, assets_dir)?;
    let logical_width = PREVIEW_WIDTH * 2 + PREVIEW_GAP;
    let width = logical_width * PREVIEW_SCALE;
    let height = PREVIEW_HEIGHT * PREVIEW_SCALE;
    let mut rgba = vec![0u8; width * height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[30, 15, 18, 255]);
    }
    draw_preview(&mut rgba, width, 0, &build.source_preview)?;
    draw_preview(
        &mut rgba,
        width,
        PREVIEW_WIDTH + PREVIEW_GAP,
        &build.target_preview,
    )?;
    write_rgba_png(
        output_path,
        width as u32,
        height as u32,
        &rgba,
        "timer remaining static preview",
    )?;
    Ok(summary(&build, 0))
}

fn build(source: &[u8], assets_dir: &Path) -> Result<TimerRemainingBuild, String> {
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "timer remaining",
    )?;
    let (source_header, source_payload) = checked_source_pack(source, &manifest)?;
    let target_payload = build_target_payload(&font, &source_payload)?;
    let target_table = build_target_table(source, &manifest)?;
    let type_pointer_patch = build_type_pointer_patch(source, &manifest)?;
    let encoded = encode_locked_mode1_pack(TARGET_PACK_BANK_OFFSET, SOURCE_VRAM, &target_payload)?;
    if encoded.bank.len() + TARGET_PACK_BANK_OFFSET > TARGET_PACK_BANK_LIMIT {
        return Err("timer encoded pattern pack exceeds its bank".to_string());
    }
    let code_patches = build_code_patches(source, &manifest)?;
    let source_preview =
        build_preview_surface(&source_payload, &[0, 4], &[12, 16, 20], 8, &[0, 16])?;
    let target_preview =
        build_preview_surface(&target_payload, &[0, 4, 8], &[12, 16, 20], 52, &[0, 16, 32])?;
    Ok(TimerRemainingBuild {
        manifest,
        source_payload,
        target_payload,
        target_table,
        source_header,
        target_header: encoded.header.to_vec(),
        pack_bank: encoded.bank,
        type_pointer_patch,
        code_patches,
        source_preview,
        target_preview,
    })
}

fn build_type_pointer_patch(
    source: &[u8],
    manifest: &TimerRemainingManifest,
) -> Result<DataPatch, String> {
    let binding = &manifest.object_type_binding;
    let offset = parse_hex(&binding.type_pointer_offset)?;
    let pointer = source_range(
        source,
        offset,
        OBJECT_POINTER_ENTRY_BYTES,
        "timer object-type table pointer",
    )?;
    if sha256_hex(pointer) != binding.type_pointer_sha256 {
        return Err("timer object-type pointer SHA-256 drifted".to_string());
    }
    let actual = u32::from_be_bytes([pointer[0], pointer[1], pointer[2], pointer[3]]) as usize;
    if actual != SOURCE_TABLE_OFFSET {
        return Err(format!(
            "timer object-type pointer selects 0x{actual:06X}, expected 0x{SOURCE_TABLE_OFFSET:06X}"
        ));
    }
    Ok(DataPatch {
        offset,
        source: pointer.to_vec(),
        target: u32::try_from(TARGET_TABLE_BANK_OFFSET)
            .map_err(|_| "timer target table pointer exceeds 32 bits".to_string())?
            .to_be_bytes()
            .to_vec(),
        label: "non-executable timer object-type table pointer".to_string(),
    })
}

fn build_target_payload(font: &Font, source_payload: &[u8]) -> Result<Vec<u8>, String> {
    let prefix = render_native_text_line(
        font,
        "앞으로",
        TARGET_PREFIX_CELLS * 16,
        8,
        0,
        15,
        "timer Korean prefix",
    )?;
    let prefix =
        encode_md_tiles_column_major(&prefix, TARGET_PREFIX_CELLS * 16, 16, "timer Korean prefix")?;
    let suffix = render_native_text_line(font, "초", 16, 8, 0, 15, "timer Korean suffix")?;
    let suffix = encode_md_tiles_column_major(&suffix, 16, 16, "timer Korean suffix")?;
    if prefix.len() != SOURCE_PREFIX_TILES * MD_TILE_BYTES
        || suffix.len() != CELL_TILES * MD_TILE_BYTES
    {
        return Err("timer Korean native-cell geometry drifted".to_string());
    }
    let digit_start = SOURCE_PREFIX_TILES * MD_TILE_BYTES;
    let digit_end = digit_start + SOURCE_DIGIT_TILES * MD_TILE_BYTES;
    let mut target = prefix;
    target.extend_from_slice(source_range(
        source_payload,
        digit_start,
        digit_end - digit_start,
        "timer protected JP digits",
    )?);
    target.extend_from_slice(&suffix);
    if target.len() != TARGET_PAYLOAD_TILES * MD_TILE_BYTES {
        return Err("timer target payload length drifted".to_string());
    }
    let roles = target[..SOURCE_PREFIX_TILES * MD_TILE_BYTES]
        .iter()
        .chain(&target[digit_end..])
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 15]) {
        return Err(format!(
            "timer Korean glyph patterns use unexpected palette roles {roles:?}"
        ));
    }
    Ok(target)
}

fn build_target_table(source: &[u8], manifest: &TimerRemainingManifest) -> Result<Vec<u8>, String> {
    let source_table = source_range(
        source,
        SOURCE_TABLE_OFFSET,
        SOURCE_TABLE_BYTES,
        "timer source table",
    )?;
    if sha256_hex(source_table) != manifest.source_table.sha256 {
        return Err("timer source table SHA-256 drifted".to_string());
    }
    let source_offsets = source_table[..TABLE_HEADER_WORDS * 2]
        .chunks_exact(2)
        .map(|pair| usize::from(u16::from_be_bytes([pair[0], pair[1]])))
        .collect::<Vec<_>>();
    if source_offsets[0..10] != [0x3C, 0x46, 0x50, 0x5A, 0x64, 0x6E, 0x78, 0x82, 0x8C, 0x96]
        || source_offsets[10] != 0x2A
        || source_offsets[11] != 0x18
    {
        return Err("timer source relative table shape drifted".to_string());
    }
    let escape_source = parse_source_label_frame(&source_table[0x18..0x2A], "escape")?;
    let treasure_source = parse_source_label_frame(&source_table[0x2A..0x3C], "treasure")?;
    let suffix_tile = SOURCE_TILE + SOURCE_PAYLOAD_BYTES / MD_TILE_BYTES;
    let escape = target_label_frame(&escape_source, suffix_tile, 0x0068)?;
    let treasure = target_label_frame(&treasure_source, suffix_tile, 0x0058)?;

    let mut target = vec![0u8; TABLE_HEADER_WORDS * 2];
    let escape_offset = target.len();
    target.extend_from_slice(&escape.encode()?);
    let treasure_offset = target.len();
    target.extend_from_slice(&treasure.encode()?);
    let mut digit_offsets = Vec::new();
    for digit in 0..10 {
        digit_offsets.push(target.len());
        let start = source_offsets[digit];
        let end = if digit == 9 {
            source_table.len()
        } else {
            source_offsets[digit + 1]
        };
        if end - start != 10 {
            return Err(format!("timer digit {digit} frame length drifted"));
        }
        target.extend_from_slice(&source_table[start..end]);
    }
    for (index, offset) in digit_offsets.into_iter().enumerate() {
        target[index * 2..index * 2 + 2].copy_from_slice(
            &u16::try_from(offset)
                .map_err(|_| "timer digit offset exceeds 16 bits".to_string())?
                .to_be_bytes(),
        );
    }
    target[20..22].copy_from_slice(
        &u16::try_from(treasure_offset)
            .map_err(|_| "timer treasure offset exceeds 16 bits".to_string())?
            .to_be_bytes(),
    );
    target[22..24].copy_from_slice(
        &u16::try_from(escape_offset)
            .map_err(|_| "timer escape offset exceeds 16 bits".to_string())?
            .to_be_bytes(),
    );
    Ok(target)
}

fn parse_source_label_frame(bytes: &[u8], label: &str) -> Result<SpriteFrame, String> {
    if bytes.len() != 18 || u16::from_be_bytes([bytes[0], bytes[1]]) != 2 {
        return Err(format!("timer {label} source label frame drifted"));
    }
    let records = bytes[2..]
        .chunks_exact(8)
        .map(|record| SpriteRecord {
            y: u16::from_be_bytes([record[0], record[1]]),
            size_and_link: u16::from_be_bytes([record[2], record[3]]),
            tile_and_attributes: u16::from_be_bytes([record[4], record[5]]),
            x: u16::from_be_bytes([record[6], record[7]]),
        })
        .collect::<Vec<_>>();
    if records[0].width_tiles() != 4
        || records[0].height_tiles() != 2
        || records[0].tile_index() != SOURCE_TILE
        || records[1].width_tiles() != 2
        || records[1].height_tiles() != 2
        || records[1].tile_index() != SOURCE_TILE + 8
    {
        return Err(format!("timer {label} source label geometry drifted"));
    }
    Ok(SpriteFrame { records })
}

fn target_label_frame(
    source: &SpriteFrame,
    suffix_tile: usize,
    suffix_x: u16,
) -> Result<SpriteFrame, String> {
    let first = &source.records[0];
    let second = &source.records[1];
    let suffix_tile =
        u16::try_from(suffix_tile).map_err(|_| "timer suffix tile exceeds 16 bits".to_string())?;
    Ok(SpriteFrame {
        records: vec![
            SpriteRecord {
                y: first.y,
                size_and_link: first.size_and_link,
                tile_and_attributes: first.tile_and_attributes,
                x: 0,
            },
            SpriteRecord {
                y: second.y,
                size_and_link: second.size_and_link,
                tile_and_attributes: (second.tile_and_attributes & !0x07FF)
                    | u16::try_from(SOURCE_TILE + 8)
                        .map_err(|_| "timer prefix tile exceeds 16 bits".to_string())?,
                x: 0x0020,
            },
            SpriteRecord {
                y: second.y,
                size_and_link: second.size_and_link,
                tile_and_attributes: (second.tile_and_attributes & !0x07FF) | suffix_tile,
                x: suffix_x,
            },
        ],
    })
}

fn build_code_patches(
    source: &[u8],
    manifest: &TimerRemainingManifest,
) -> Result<Vec<CodePatch>, String> {
    let mut patches = Vec::new();
    for consumer in &manifest.consumers {
        validate_consumer_window(source, consumer, "JP source")?;
        let lea_offset = parse_hex(&consumer.table_lea_offset)?;
        let source_lea = m68k::assemble(&[Inst::LeaAbsoluteLong {
            address: SOURCE_TABLE_OFFSET as u32,
            destination: AddressReg::A1,
        }])?;
        let target_lea = m68k::assemble(&[Inst::LeaAbsoluteLong {
            address: TARGET_TABLE_BANK_OFFSET as u32,
            destination: AddressReg::A1,
        }])?;
        if source_range(source, lea_offset, source_lea.len(), "timer source LEA")? != source_lea {
            return Err(format!("{} typed table LEA drifted", consumer.id));
        }
        patches.push(CodePatch {
            offset: lea_offset,
            source: source_lea,
            target: target_lea,
            label: format!("{} typed timer-table LEA", consumer.id),
        });
        for digit in &consumer.digit_offsets {
            let offset = parse_hex(&digit.instruction_offset)?;
            let source_x = parse_u16_hex(&digit.source_x)?;
            let target_x = parse_u16_hex(&digit.target_x)?;
            let source_instruction = m68k::assemble(&[Inst::AddiWordImmediate {
                immediate: source_x,
                destination: DataReg::D2,
            }])?;
            let target_instruction = m68k::assemble(&[Inst::AddiWordImmediate {
                immediate: target_x,
                destination: DataReg::D2,
            }])?;
            if source_range(
                source,
                offset,
                source_instruction.len(),
                "timer source digit ADDI",
            )? != source_instruction
            {
                return Err(format!("{} typed digit position drifted", consumer.id));
            }
            patches.push(CodePatch {
                offset,
                source: source_instruction,
                target: target_instruction,
                label: format!("{} typed digit horizontal position", consumer.id),
            });
        }
    }
    patches.sort_by_key(|patch| patch.offset);
    Ok(patches)
}

fn validate_consumer_window(
    rom: &[u8],
    consumer: &ConsumerDeclaration,
    label: &str,
) -> Result<(), String> {
    let offset = parse_hex(&consumer.window_offset)?;
    let window = source_range(
        rom,
        offset,
        consumer.window_bytes,
        &format!("{} timer consumer window", consumer.id),
    )?;
    if sha256_hex(window) != consumer.window_sha256 {
        return Err(format!(
            "{label} {} timer consumer window drifted",
            consumer.id
        ));
    }
    Ok(())
}

fn validate_output(
    source: &[u8],
    output: &[u8],
    build: &TimerRemainingBuild,
) -> Result<(), String> {
    if source_range(
        output,
        SOURCE_TABLE_OFFSET,
        SOURCE_TABLE_BYTES,
        "protected timer source table",
    )? != source_range(
        source,
        SOURCE_TABLE_OFFSET,
        SOURCE_TABLE_BYTES,
        "JP timer source table",
    )? {
        return Err("timer source table changed in place".to_string());
    }
    if source_range(
        output,
        build.type_pointer_patch.offset,
        build.type_pointer_patch.target.len(),
        "output timer object-type pointer",
    )? != build.type_pointer_patch.target
    {
        return Err("timer output object-type pointer drifted".to_string());
    }
    if source_range(
        output,
        TARGET_TABLE_BANK_OFFSET,
        build.target_table.len(),
        "output timer target table",
    )? != build.target_table
    {
        return Err("timer output target table drifted".to_string());
    }
    for header in &build.manifest.source_pack.header_offsets {
        let decoded = decode_mode1_pack_entry(output, parse_hex(header)?)?;
        if decoded.vram_destination != SOURCE_VRAM || decoded.data != build.target_payload {
            return Err("timer output pack alias drifted".to_string());
        }
    }
    for consumer in &build.manifest.consumers {
        let window_offset = parse_hex(&consumer.window_offset)?;
        let source_window = source_range(
            source,
            window_offset,
            consumer.window_bytes,
            "JP timer consumer window",
        )?;
        let mut expected = source_window.to_vec();
        for patch in build.code_patches.iter().filter(|patch| {
            patch.offset >= window_offset
                && patch.offset + patch.source.len() <= window_offset + consumer.window_bytes
        }) {
            let relative = patch.offset - window_offset;
            expected[relative..relative + patch.target.len()].copy_from_slice(&patch.target);
        }
        if source_range(
            output,
            window_offset,
            consumer.window_bytes,
            "output timer consumer window",
        )? != expected
        {
            return Err(format!(
                "{} output consumer changed outside typed patches",
                consumer.id
            ));
        }
    }
    Ok(())
}

fn checked_source_pack(
    source: &[u8],
    manifest: &TimerRemainingManifest,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut canonical_header = None;
    let mut canonical_payload = None;
    for value in &manifest.source_pack.header_offsets {
        let offset = parse_hex(value)?;
        let header = source_range(source, offset, HEADER_BYTES, "timer source pack header")?;
        if sha256_hex(header) != manifest.source_pack.header_sha256 {
            return Err(format!("timer source header at 0x{offset:06X} drifted"));
        }
        let decoded = decode_mode1_pack_entry(source, offset)?;
        if decoded.vram_destination != SOURCE_VRAM
            || decoded.data.len() != SOURCE_PAYLOAD_BYTES
            || sha256_hex(&decoded.data) != manifest.source_pack.decoded_sha256
        {
            return Err(format!("timer source pack at 0x{offset:06X} drifted"));
        }
        match (&canonical_header, &canonical_payload) {
            (None, None) => {
                canonical_header = Some(header.to_vec());
                canonical_payload = Some(decoded.data);
            }
            (Some(expected_header), Some(expected_payload))
                if expected_header == header && expected_payload == &decoded.data => {}
            _ => return Err("timer pack aliases do not decode identically".to_string()),
        }
    }
    Ok((
        canonical_header.ok_or_else(|| "timer source has no pack aliases".to_string())?,
        canonical_payload.ok_or_else(|| "timer source has no payload".to_string())?,
    ))
}

fn build_preview_surface(
    payload: &[u8],
    prefix_cells: &[usize],
    digit_cells: &[usize],
    suffix_cell: usize,
    prefix_x: &[usize],
) -> Result<Vec<u8>, String> {
    let mut surface = vec![0u8; PREVIEW_WIDTH * PREVIEW_HEIGHT];
    for (&tile, &x) in prefix_cells.iter().zip(prefix_x) {
        blit_cell(payload, tile, &mut surface, x, "timer preview prefix")?;
    }
    let digit_start_x = if prefix_cells.len() == 3 { 56 } else { 32 };
    for (index, &tile) in digit_cells.iter().enumerate() {
        blit_cell(
            payload,
            tile,
            &mut surface,
            digit_start_x + index * 16,
            "timer preview digit",
        )?;
    }
    let suffix_x = if prefix_cells.len() == 3 { 104 } else { 80 };
    blit_cell(
        payload,
        suffix_cell,
        &mut surface,
        suffix_x,
        "timer preview suffix",
    )?;
    Ok(surface)
}

fn blit_cell(
    payload: &[u8],
    tile: usize,
    surface: &mut [u8],
    x: usize,
    label: &str,
) -> Result<(), String> {
    let decoded = decode_md_tiles_column_major(
        source_range(
            payload,
            tile * MD_TILE_BYTES,
            CELL_TILES * MD_TILE_BYTES,
            label,
        )?,
        16,
        16,
        label,
    )?;
    for y in 0..16 {
        surface[y * PREVIEW_WIDTH + x..y * PREVIEW_WIDTH + x + 16]
            .copy_from_slice(&decoded[y * 16..(y + 1) * 16]);
    }
    Ok(())
}

fn draw_preview(
    rgba: &mut [u8],
    output_width: usize,
    logical_x: usize,
    surface: &[u8],
) -> Result<(), String> {
    if surface.len() != PREVIEW_WIDTH * PREVIEW_HEIGHT {
        return Err("timer preview surface length drifted".to_string());
    }
    let roles = surface.iter().copied().collect::<BTreeSet<_>>();
    if !roles.is_subset(&BTreeSet::from([0, 1, 15])) || !roles.contains(&15) {
        return Err(format!(
            "timer preview uses unexpected palette roles {roles:?}"
        ));
    }
    for y in 0..PREVIEW_HEIGHT {
        for x in 0..PREVIEW_WIDTH {
            if surface[y * PREVIEW_WIDTH + x] != 15 {
                continue;
            }
            for scale_y in 0..PREVIEW_SCALE {
                for scale_x in 0..PREVIEW_SCALE {
                    let target_x = (logical_x + x) * PREVIEW_SCALE + scale_x;
                    let target_y = y * PREVIEW_SCALE + scale_y;
                    let offset = (target_y * output_width + target_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[246, 236, 226, 255]);
                }
            }
        }
    }
    Ok(())
}

fn read_manifest(assets_dir: &Path) -> Result<TimerRemainingManifest, String> {
    let path = assets_dir.join("graphics_text/timer_remaining.json");
    let bytes = fs::read(&path)
        .map_err(|error| format!("failed to read timer source {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid timer source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &TimerRemainingManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-TIMER-REMAINING"
        || !manifest.source_policy.contains("English")
        || manifest.jp != "あと [digits] 秒"
        || manifest.ko != "앞으로 [digits]초"
        || manifest.font_asset != "neodgm.ttf"
        || manifest.render_mode != "jp_native_16x16_no_horizontal_scaling"
        || manifest.source_pack.header_offsets != ["0x078276", "0x07852C", "0x07857C", "0x0786FA"]
        || parse_u16_hex(&manifest.source_pack.vram_destination)? != SOURCE_VRAM
        || manifest.source_pack.decoded_bytes != SOURCE_PAYLOAD_BYTES
        || manifest.source_pack.prefix_tiles != SOURCE_PREFIX_TILES
        || manifest.source_pack.digit_tiles != SOURCE_DIGIT_TILES
        || parse_hex(&manifest.source_table.offset)? != SOURCE_TABLE_OFFSET
        || manifest.source_table.bytes != SOURCE_TABLE_BYTES
        || parse_hex(&manifest.object_type_binding.pointer_table_offset)?
            != OBJECT_POINTER_TABLE_OFFSET
        || manifest.object_type_binding.pointer_entry_bytes != OBJECT_POINTER_ENTRY_BYTES
        || parse_hex(&manifest.object_type_binding.object_type)? != TIMER_OBJECT_TYPE
        || parse_hex(&manifest.object_type_binding.type_pointer_offset)?
            != SOURCE_TYPE_POINTER_OFFSET
        || parse_hex(&manifest.target_pack_bank.offset)? != TARGET_PACK_BANK_OFFSET
        || parse_hex(&manifest.target_pack_bank.limit)? != TARGET_PACK_BANK_LIMIT
        || parse_u8_hex(&manifest.target_pack_bank.fill_byte)? != BANK_FILL
        || parse_hex(&manifest.target_table_bank.offset)? != TARGET_TABLE_BANK_OFFSET
        || parse_hex(&manifest.target_table_bank.limit)? != TARGET_TABLE_BANK_LIMIT
        || parse_u8_hex(&manifest.target_table_bank.fill_byte)? != BANK_FILL
        || manifest.consumers.len() != 2
    {
        return Err("timer remaining manifest identity drifted".to_string());
    }
    for hash in [
        &manifest.font_sha256,
        &manifest.source_pack.header_sha256,
        &manifest.source_pack.decoded_sha256,
        &manifest.source_table.sha256,
        &manifest.object_type_binding.type_pointer_sha256,
    ] {
        validate_sha256(hash, "timer remaining")?;
    }
    let expected = [
        (
            "GFX-TIMER-REMAINING-ESCAPE",
            "escape_three_digits",
            3usize,
            "0x0068",
        ),
        (
            "GFX-TIMER-REMAINING-TREASURE",
            "treasure_two_digits",
            2usize,
            "0x0058",
        ),
    ];
    for (consumer, expected) in manifest.consumers.iter().zip(expected) {
        if consumer.id != expected.0
            || consumer.profile != expected.1
            || consumer.digit_offsets.len() != expected.2
            || consumer.suffix_x != expected.3
            || consumer.window_bytes == 0
        {
            return Err(format!("{} identity drifted", consumer.id));
        }
        validate_sha256(&consumer.window_sha256, &consumer.id)?;
    }
    Ok(())
}

fn validate_sha256(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label} has an invalid lowercase SHA-256"));
    }
    Ok(())
}

fn parse_u8_hex(value: &str) -> Result<u8, String> {
    u8::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 8 bits"))
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    u16::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 16 bits"))
}

fn summary(build: &TimerRemainingBuild, checksum: u16) -> TimerRemainingSummary {
    TimerRemainingSummary {
        alias_headers: build.manifest.source_pack.header_offsets.len(),
        source_tiles: build.source_payload.len() / MD_TILE_BYTES,
        rewritten_tiles: SOURCE_PREFIX_TILES + TARGET_SUFFIX_CELLS * CELL_TILES,
        protected_digit_tiles: SOURCE_DIGIT_TILES,
        table_bytes: build.target_table.len(),
        written_executable_bytes: build
            .code_patches
            .iter()
            .map(|patch| patch.target.len())
            .sum(),
        verified_consumer_bytes: build
            .manifest
            .consumers
            .iter()
            .map(|consumer| consumer.window_bytes)
            .sum(),
        pack_bank_bytes: build.pack_bank.len(),
        checksum,
    }
}
