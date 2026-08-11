//! JP-source compiler for the final original-credit graphic consumers.
//!
//! This module owns the generic-sprite surfaces that are not fixed-cell timed
//! names: the page-0 producer, the page-7 two-state name/nickname, the page-9
//! staged name and the five moving cooperation names. It decodes and validates
//! the exact JP frame tables and extra page transfers, renders NeoDGM in each
//! source consumer's native geometry (including the page-0 96x8 signature),
//! rebuilds only the selected non-executable frames, and redirects their
//! checked object-type pointers. Executable selection paths are verified with
//! the typed 68000 ISA and are never rewritten.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::credits_generic::{
    DataPatch, GENERIC_RENDERER_BYTES, validate_generic_renderer_contract,
};
use super::credits_timed::CreditLinePreview;
use super::font_effect::read_verified_font;
use super::{
    decode_mode1_pack_entry, encode_locked_mode1_pack, parse_hex, sha256_hex, source_range,
};

#[path = "credits_remaining/common.rs"]
mod common;
#[path = "credits_remaining/page0.rs"]
mod page0;
#[path = "credits_remaining/page7.rs"]
mod page7;
#[path = "credits_remaining/page9.rs"]
mod page9;

const PAGE_COUNT: usize = 10;
const PAGE7_PACK_BANK_OFFSET: usize = 0x31_0000;
const PAGE7_PACK_BANK_LIMIT: usize = 0x31_8000;
const PAGE9_PACK_BANK_OFFSET: usize = 0x31_8000;
const PAGE9_PACK_BANK_LIMIT: usize = 0x32_0000;
const TARGET_TABLE_BANK_OFFSET: usize = 0x32_8000;
const TARGET_TABLE_BANK_LIMIT: usize = 0x32_A000;
const BANK_FILL: u8 = 0xFF;

#[derive(Debug, Deserialize)]
struct CreditsRemainingManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    standalone_ascii_policy: String,
    preserved_complete_latin_names: Vec<String>,
    font_asset: String,
    font_sha256: String,
    render_mode: String,
    space_width_px: usize,
    generic_renderer_sha256: String,
    target_table_bank: BankDeclaration,
    consumers: Vec<ConsumerDeclaration>,
    extra_packs: Vec<PackDeclaration>,
}

#[derive(Debug, Deserialize)]
struct BankDeclaration {
    offset: String,
    limit: String,
    fill_byte: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ConsumerDeclaration {
    id: String,
    page: usize,
    lines: Vec<CreditTextDeclaration>,
    object_type: String,
    type_pointer_offset: String,
    type_pointer_sha256: String,
    source_table_offset: String,
    source_table_bytes: usize,
    source_table_sha256: String,
    frame_count: usize,
    selection_window_offset: String,
    selection_window_bytes: usize,
    selection_window_sha256: String,
    typed_selectors: Vec<SelectorDeclaration>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CreditTextDeclaration {
    id: String,
    jp: String,
    ko: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SelectorDeclaration {
    offset: String,
    profile: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PackDeclaration {
    id: String,
    page: usize,
    header_offset: String,
    header_sha256: String,
    vram_destination: String,
    decoded_bytes: usize,
    decoded_sha256: String,
    target_bank_offset: String,
    target_bank_limit: String,
}

#[derive(Debug)]
pub(super) struct TableReplacement {
    id: String,
    pointer_offset: usize,
    source_offset: usize,
    source: Vec<u8>,
    target: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct ExtraPayload {
    target: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct PageCompile {
    tables: Vec<TableReplacement>,
    extra_payload: Option<ExtraPayload>,
    previews: Vec<CreditLinePreview>,
    glyphs: BTreeSet<char>,
    appended_tiles: usize,
}

#[derive(Debug)]
pub(super) struct PackPatch {
    pub(super) id: String,
    pub(super) header_offset: usize,
    pub(super) source_header: Vec<u8>,
    pub(super) target_header: Vec<u8>,
    pub(super) bank_offset: usize,
    pub(super) bank_limit: usize,
    pub(super) bank: Vec<u8>,
    destination: u16,
    target_payload: Vec<u8>,
}

#[derive(Debug)]
struct BuiltTable {
    id: String,
    source_offset: usize,
    source: Vec<u8>,
    target_offset: usize,
    target: Vec<u8>,
}

#[derive(Debug)]
struct AllocatedTables {
    bank: Vec<u8>,
    tables: Vec<BuiltTable>,
    patches: Vec<DataPatch>,
}

#[derive(Debug)]
pub(super) struct CreditsRemainingBuild {
    manifest: CreditsRemainingManifest,
    previews: Vec<CreditLinePreview>,
    glyphs: BTreeSet<char>,
    appended_tiles: usize,
    verified_consumer_bytes: usize,
    table_bank: Vec<u8>,
    tables: Vec<BuiltTable>,
    data_patches: Vec<DataPatch>,
    pack_patches: Vec<PackPatch>,
}

impl CreditsRemainingBuild {
    pub(super) fn line_count(&self) -> usize {
        self.previews.len()
    }

    pub(super) fn preview_lines(&self) -> &[CreditLinePreview] {
        &self.previews
    }

    pub(super) fn glyphs(&self) -> &BTreeSet<char> {
        &self.glyphs
    }

    pub(super) fn rewritten_tiles(&self) -> usize {
        self.appended_tiles
    }

    pub(super) fn appended_tiles(&self) -> usize {
        self.appended_tiles
    }

    pub(super) fn relocated_lines(&self) -> usize {
        self.tables.len()
    }

    pub(super) fn verified_consumer_bytes(&self) -> usize {
        self.verified_consumer_bytes
    }

    pub(super) fn table_bank_offset(&self) -> usize {
        TARGET_TABLE_BANK_OFFSET
    }

    pub(super) fn table_bank(&self) -> &[u8] {
        &self.table_bank
    }

    pub(super) fn data_patches(&self) -> &[DataPatch] {
        &self.data_patches
    }

    pub(super) fn pack_patches(&self) -> &[PackPatch] {
        &self.pack_patches
    }

    pub(super) fn pack_bank_bytes(&self) -> usize {
        self.pack_patches.iter().map(|patch| patch.bank.len()).sum()
    }

    pub(super) fn validate_output_consumers(
        &self,
        source_rom: &[u8],
        output: &[u8],
        label: &str,
    ) -> Result<(), String> {
        validate_generic_renderer_contract(output, &self.manifest.generic_renderer_sha256, label)?;
        if source_range(
            output,
            TARGET_TABLE_BANK_OFFSET,
            self.table_bank.len(),
            "remaining-credit output table bank",
        )? != self.table_bank
        {
            return Err(format!("{label} remaining-credit table bank drifted"));
        }
        for (declaration, table) in self.manifest.consumers.iter().zip(&self.tables) {
            common::validate_consumer(source_rom, declaration, "JP source")?;
            common::validate_selectors(output, declaration, label)?;
            let window_offset = parse_hex(&declaration.selection_window_offset)?;
            if source_range(
                output,
                window_offset,
                declaration.selection_window_bytes,
                &format!("{} output selection window", declaration.id),
            )? != source_range(
                source_rom,
                window_offset,
                declaration.selection_window_bytes,
                &format!("{} JP selection window", declaration.id),
            )? {
                return Err(format!(
                    "{label} {} changed its selection window",
                    declaration.id
                ));
            }
            if source_range(
                output,
                table.source_offset,
                table.source.len(),
                &format!("{} protected source table", table.id),
            )? != table.source
                || source_range(
                    source_rom,
                    table.source_offset,
                    table.source.len(),
                    &format!("{} JP source table", table.id),
                )? != table.source
            {
                return Err(format!("{label} {} source table changed", table.id));
            }
            if source_range(
                output,
                table.target_offset,
                table.target.len(),
                &format!("{} relocated target table", table.id),
            )? != table.target
            {
                return Err(format!("{label} {} target table drifted", table.id));
            }
            let pointer = source_range(
                output,
                parse_hex(&declaration.type_pointer_offset)?,
                4,
                &format!("{} output type pointer", table.id),
            )?;
            if pointer
                != u32::try_from(table.target_offset)
                    .map_err(|_| format!("{} target offset exceeds 32 bits", table.id))?
                    .to_be_bytes()
            {
                return Err(format!("{label} {} target pointer drifted", table.id));
            }
        }
        for patch in &self.pack_patches {
            if source_range(
                output,
                patch.bank_offset,
                patch.bank.len(),
                &format!("{} output pack bank", patch.id),
            )? != patch.bank
            {
                return Err(format!("{label} {} pack bank drifted", patch.id));
            }
            let decoded = decode_mode1_pack_entry(output, patch.header_offset)?;
            if decoded.vram_destination != patch.destination || decoded.data != patch.target_payload
            {
                return Err(format!("{label} {} decoded target pack drifted", patch.id));
            }
        }
        Ok(())
    }
}

pub(super) fn compile_credits_remaining(
    source_rom: &[u8],
    source_pages: &[Vec<u8>],
    target_pages: &mut [Vec<u8>],
    assets_dir: &Path,
) -> Result<CreditsRemainingBuild, String> {
    if source_pages.len() != PAGE_COUNT || target_pages.len() != PAGE_COUNT {
        return Err("remaining-credit compiler requires ten page payloads".to_string());
    }
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    validate_generic_renderer_contract(source_rom, &manifest.generic_renderer_sha256, "JP source")?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "remaining credit",
    )?;
    let source_extra = manifest
        .extra_packs
        .iter()
        .map(|pack| checked_source_pack(source_rom, pack))
        .collect::<Result<Vec<_>, _>>()?;

    let page0 = page0::compile(
        source_rom,
        &source_pages[0],
        &mut target_pages[0],
        &font,
        &manifest.consumers[0],
    )?;
    let page7 = page7::compile(
        source_rom,
        &font,
        &manifest.consumers[1],
        &source_extra[0],
        parse_u16_hex(&manifest.extra_packs[0].vram_destination)?,
    )?;
    let page9_sequence = page9::compile_sequence(
        source_rom,
        &source_pages[9],
        &mut target_pages[9],
        &font,
        &manifest.consumers[2],
    )?;
    let page9_names = page9::compile_names(
        source_rom,
        &font,
        &manifest.consumers[3],
        &source_extra[1],
        parse_u16_hex(&manifest.extra_packs[1].vram_destination)?,
    )?;

    let mut tables = Vec::new();
    let mut previews = Vec::new();
    let mut glyphs = BTreeSet::new();
    let mut appended_tiles = 0usize;
    let mut extra_payloads = Vec::new();
    for compiled in [page0, page7, page9_sequence, page9_names] {
        tables.extend(compiled.tables);
        previews.extend(compiled.previews);
        glyphs.extend(compiled.glyphs);
        appended_tiles += compiled.appended_tiles;
        if let Some(payload) = compiled.extra_payload {
            extra_payloads.push(payload);
        }
    }
    if previews.len() != 10 || tables.len() != 4 || extra_payloads.len() != 2 {
        return Err("remaining-credit compiler output denominator drifted".to_string());
    }

    let allocated_tables = allocate_tables(tables)?;
    let pack_patches = manifest
        .extra_packs
        .iter()
        .zip(extra_payloads)
        .map(|(declaration, payload)| build_pack_patch(source_rom, declaration, payload.target))
        .collect::<Result<Vec<_>, _>>()?;
    let verified_consumer_bytes = GENERIC_RENDERER_BYTES
        + manifest
            .consumers
            .iter()
            .map(|consumer| 4 + consumer.source_table_bytes + consumer.selection_window_bytes)
            .sum::<usize>()
        + manifest.extra_packs.len() * 6;

    Ok(CreditsRemainingBuild {
        manifest,
        previews,
        glyphs,
        appended_tiles,
        verified_consumer_bytes,
        table_bank: allocated_tables.bank,
        tables: allocated_tables.tables,
        data_patches: allocated_tables.patches,
        pack_patches,
    })
}

fn allocate_tables(replacements: Vec<TableReplacement>) -> Result<AllocatedTables, String> {
    let mut bank = Vec::new();
    let mut tables = Vec::new();
    let mut patches = Vec::new();
    for replacement in replacements {
        if !bank.len().is_multiple_of(2) {
            bank.push(0);
        }
        let target_offset = TARGET_TABLE_BANK_OFFSET
            .checked_add(bank.len())
            .ok_or_else(|| "remaining-credit table allocation overflowed".to_string())?;
        if target_offset + replacement.target.len() > TARGET_TABLE_BANK_LIMIT {
            return Err(format!("{} target table exceeds its bank", replacement.id));
        }
        bank.extend_from_slice(&replacement.target);
        patches.push(DataPatch {
            offset: replacement.pointer_offset,
            source: u32::try_from(replacement.source_offset)
                .map_err(|_| format!("{} source offset exceeds 32 bits", replacement.id))?
                .to_be_bytes()
                .to_vec(),
            target: u32::try_from(target_offset)
                .map_err(|_| format!("{} target offset exceeds 32 bits", replacement.id))?
                .to_be_bytes()
                .to_vec(),
            label: format!(
                "{} non-executable object-type table pointer",
                replacement.id
            ),
        });
        tables.push(BuiltTable {
            id: replacement.id,
            source_offset: replacement.source_offset,
            source: replacement.source,
            target_offset,
            target: replacement.target,
        });
    }
    Ok(AllocatedTables {
        bank,
        tables,
        patches,
    })
}

fn build_pack_patch(
    source_rom: &[u8],
    declaration: &PackDeclaration,
    target_payload: Vec<u8>,
) -> Result<PackPatch, String> {
    let header_offset = parse_hex(&declaration.header_offset)?;
    let bank_offset = parse_hex(&declaration.target_bank_offset)?;
    let bank_limit = parse_hex(&declaration.target_bank_limit)?;
    let destination = parse_u16_hex(&declaration.vram_destination)?;
    let encoded = encode_locked_mode1_pack(bank_offset, destination, &target_payload)?;
    if bank_offset + encoded.bank.len() > bank_limit {
        return Err(format!("{} encoded pack exceeds its bank", declaration.id));
    }
    Ok(PackPatch {
        id: declaration.id.clone(),
        header_offset,
        source_header: source_range(
            source_rom,
            header_offset,
            6,
            &format!("{} source header", declaration.id),
        )?
        .to_vec(),
        target_header: encoded.header.to_vec(),
        bank_offset,
        bank_limit,
        bank: encoded.bank,
        destination,
        target_payload,
    })
}

fn checked_source_pack(
    source_rom: &[u8],
    declaration: &PackDeclaration,
) -> Result<Vec<u8>, String> {
    let header_offset = parse_hex(&declaration.header_offset)?;
    let header = source_range(source_rom, header_offset, 6, "remaining-credit pack header")?;
    if sha256_hex(header) != declaration.header_sha256 {
        return Err(format!("{} source header SHA-256 drifted", declaration.id));
    }
    let decoded = decode_mode1_pack_entry(source_rom, header_offset)?;
    if decoded.vram_destination != parse_u16_hex(&declaration.vram_destination)?
        || decoded.data.len() != declaration.decoded_bytes
        || sha256_hex(&decoded.data) != declaration.decoded_sha256
    {
        return Err(format!("{} decoded JP pack drifted", declaration.id));
    }
    Ok(decoded.data)
}

fn read_manifest(assets_dir: &Path) -> Result<CreditsRemainingManifest, String> {
    let path = assets_dir.join("graphics_text/credits_remaining.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read remaining-credit source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "invalid remaining-credit source {}: {error}",
            path.display()
        )
    })
}

fn validate_manifest_shape(manifest: &CreditsRemainingManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-CREDITS-REMAINING"
        || !manifest.source_policy.contains("English VWF")
        || manifest.standalone_ascii_policy
            != "preserve_unmapped_standalone_ascii_escape_doors_owned_by_escape_doors_manifest"
        || manifest.preserved_complete_latin_names != ["MOO"]
        || manifest.font_asset != "neodgm.ttf"
        || manifest.render_mode
            != "jp_source_frame_specific_signature_and_shared_native_16x16_no_horizontal_scaling"
        || manifest.space_width_px != common::SPACE_WIDTH
        || manifest.consumers.len() != 4
        || manifest.extra_packs.len() != 2
    {
        return Err("remaining-credit manifest identity drifted".to_string());
    }
    validate_sha256(&manifest.font_sha256, "remaining-credit repository font")?;
    validate_sha256(
        &manifest.generic_renderer_sha256,
        "remaining-credit generic renderer",
    )?;
    if parse_hex(&manifest.target_table_bank.offset)? != TARGET_TABLE_BANK_OFFSET
        || parse_hex(&manifest.target_table_bank.limit)? != TARGET_TABLE_BANK_LIMIT
        || parse_u8_hex(&manifest.target_table_bank.fill_byte)? != BANK_FILL
    {
        return Err("remaining-credit table-bank declaration drifted".to_string());
    }

    let expected_consumers = [
        (
            "GFX-CREDITS-P00-L01",
            0usize,
            "0xB5",
            10usize,
            &["GFX-CREDITS-P00-L01"][..],
        ),
        (
            "GFX-CREDITS-P07-L02-L04",
            7,
            "0xCF",
            2,
            &[
                "GFX-CREDITS-P07-L02",
                "GFX-CREDITS-P07-L03",
                "GFX-CREDITS-P07-L04",
            ][..],
        ),
        (
            "GFX-CREDITS-P09-L01",
            9,
            "0xBE",
            18,
            &["GFX-CREDITS-P09-L01"][..],
        ),
        (
            "GFX-CREDITS-P09-L02-L06",
            9,
            "0xC8",
            5,
            &[
                "GFX-CREDITS-P09-L02",
                "GFX-CREDITS-P09-L03",
                "GFX-CREDITS-P09-L04",
                "GFX-CREDITS-P09-L05",
                "GFX-CREDITS-P09-L06",
            ][..],
        ),
    ];
    for (declaration, expected) in manifest.consumers.iter().zip(expected_consumers) {
        if declaration.id != expected.0
            || declaration.page != expected.1
            || declaration.object_type != expected.2
            || declaration.frame_count != expected.3
            || declaration
                .lines
                .iter()
                .map(|line| line.id.as_str())
                .collect::<Vec<_>>()
                != expected.4
            || declaration
                .lines
                .iter()
                .any(|line| line.jp.is_empty() || line.ko.is_empty())
            || declaration.source_table_bytes == 0
            || declaration.selection_window_bytes == 0
            || declaration.typed_selectors.is_empty()
        {
            return Err(format!("{} identity or geometry drifted", declaration.id));
        }
        common::validate_declaration_hashes(declaration)?;
    }
    let page7 = &manifest.consumers[1].lines;
    if page7[1].ko != format!("{} {}", page7[0].ko, page7[2].ko)
        || page7[1].jp != format!("{}{}", page7[0].jp, page7[2].jp)
    {
        return Err("page-7 combined credit text drifted".to_string());
    }

    let expected_packs = [
        (
            "GFX-CREDITS-P07-NAME-PACK",
            7usize,
            0x09_E074usize,
            0x2000u16,
            1024usize,
            PAGE7_PACK_BANK_OFFSET,
            PAGE7_PACK_BANK_LIMIT,
        ),
        (
            "GFX-CREDITS-P09-NAME-PACK",
            9,
            0x09_E0A2,
            0x4000,
            2816,
            PAGE9_PACK_BANK_OFFSET,
            PAGE9_PACK_BANK_LIMIT,
        ),
    ];
    for (declaration, expected) in manifest.extra_packs.iter().zip(expected_packs) {
        if declaration.id != expected.0
            || declaration.page != expected.1
            || parse_hex(&declaration.header_offset)? != expected.2
            || parse_u16_hex(&declaration.vram_destination)? != expected.3
            || declaration.decoded_bytes != expected.4
            || parse_hex(&declaration.target_bank_offset)? != expected.5
            || parse_hex(&declaration.target_bank_limit)? != expected.6
        {
            return Err(format!("{} pack identity drifted", declaration.id));
        }
        validate_sha256(&declaration.header_sha256, &declaration.id)?;
        validate_sha256(&declaration.decoded_sha256, &declaration.id)?;
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
