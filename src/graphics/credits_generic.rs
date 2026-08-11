//! JP-source compiler for ending-credit names drawn by the generic sprite path.
//!
//! Page 4 keeps the original one-frame 4x2 sprite and replaces only its two
//! 16x16 pattern cells. Page 8 appends native Korean cells, reproduces the
//! complete 31-frame source table with every unrelated frame byte-identical,
//! and redirects the object-type data pointer to that rebuilt table. The
//! generic renderer and both frame selectors are checked through typed 68000
//! instructions; this compiler writes no executable bytes.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use crate::m68k::{self, AddressReg, DataReg, Inst};

use super::credits_timed::CreditLinePreview;
use super::font_effect::{native_text_width, read_verified_font, render_native_text_line};
use super::pixel::{decode_md_tiles_column_major, encode_md_tiles_column_major};
use super::{MD_TILE_BYTES, parse_hex, sha256_hex, source_range, validate_only_ranges_changed};

const LINE_COUNT: usize = 2;
const PAGE_COUNT: usize = 10;
const CELL_WIDTH_PIXELS: usize = 16;
const CELL_HEIGHT_PIXELS: usize = 16;
const SPACE_WIDTH_PIXELS: usize = 8;
const GLYPH_TILES: usize = 4;
const BLANK_TILE: usize = 5;
const TILE_INDEX_MASK: u16 = 0x07FF;
pub(super) const GENERIC_RENDERER_OFFSET: usize = 0x0000_18D4;
pub(super) const GENERIC_RENDERER_BYTES: usize = 58;
pub(super) const POINTER_TABLE_OFFSET: usize = 0x0006_AD18;
pub(super) const POINTER_ENTRY_BYTES: usize = 4;
pub(super) const OBJECT_TYPE_DISPLACEMENT: u16 = 0x0008;
pub(super) const FRAME_INDEX_DISPLACEMENT: u16 = 0x0009;
const TARGET_TABLE_BANK_OFFSET: usize = 0x2F_A000;
const TARGET_TABLE_BANK_LIMIT: usize = 0x2F_C000;
const TARGET_TABLE_BANK_FILL: u8 = 0xFF;

#[derive(Debug, Deserialize)]
struct CreditsGenericManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    standalone_ascii_policy: String,
    preserved_complete_latin_names: Vec<String>,
    font_asset: String,
    font_sha256: String,
    render_mode: String,
    space_width_px: usize,
    generic_renderer: GenericRendererDeclaration,
    target_table_bank: TargetTableBankDeclaration,
    lines: Vec<LineDeclaration>,
}

#[derive(Debug, Deserialize)]
struct GenericRendererDeclaration {
    window_offset: String,
    window_bytes: usize,
    window_sha256: String,
    pointer_table_offset: String,
    pointer_entry_bytes: usize,
    object_type_displacement: String,
    frame_index_displacement: String,
}

#[derive(Debug, Deserialize)]
struct TargetTableBankDeclaration {
    offset: String,
    limit: String,
    fill_byte: String,
}

#[derive(Debug, Deserialize)]
struct LineDeclaration {
    id: String,
    page: usize,
    jp: String,
    ko: String,
    placement: String,
    source_logical_tiles: Vec<usize>,
    object_type: String,
    type_pointer_offset: String,
    type_pointer_sha256: String,
    source_table_offset: String,
    source_table_bytes: usize,
    source_table_sha256: Option<String>,
    frame_count: usize,
    frame_index: usize,
    source_frame_offset: String,
    source_frame_map_offset: String,
    source_frame_map_bytes: usize,
    source_frame_map_sha256: String,
    selection_window_offset: String,
    selection_window_bytes: usize,
    selection_window_sha256: String,
    selection_instruction_offset: String,
    selection_profile: String,
    #[serde(default)]
    target_first_x: Option<String>,
    #[serde(default)]
    target_x_step: Option<usize>,
    segments: Vec<SegmentDeclaration>,
}

#[derive(Debug, Deserialize)]
struct SegmentDeclaration {
    jp: String,
    ko: String,
    source_tile_start: usize,
    source_tile_end_exclusive: usize,
    target_cells: usize,
    source_sha256: String,
}

#[derive(Debug)]
pub(super) struct DataPatch {
    pub(super) offset: usize,
    pub(super) source: Vec<u8>,
    pub(super) target: Vec<u8>,
    pub(super) label: String,
}

#[derive(Debug)]
pub(super) struct CreditsGenericBuild {
    manifest: CreditsGenericManifest,
    previews: Vec<CreditLinePreview>,
    glyphs: BTreeSet<char>,
    overwritten_source_tiles: usize,
    appended_tiles: usize,
    relocated_lines: usize,
    verified_consumer_bytes: usize,
    table_bank: Vec<u8>,
    data_patches: Vec<DataPatch>,
}

impl CreditsGenericBuild {
    pub(super) fn line_count(&self) -> usize {
        self.previews.len()
    }

    pub(super) fn preview_lines(&self) -> &[CreditLinePreview] {
        &self.previews
    }

    pub(super) fn glyphs(&self) -> &BTreeSet<char> {
        &self.glyphs
    }

    pub(super) fn overwritten_source_tiles(&self) -> usize {
        self.overwritten_source_tiles
    }

    pub(super) fn appended_tiles(&self) -> usize {
        self.appended_tiles
    }

    pub(super) fn rewritten_tiles(&self) -> usize {
        self.overwritten_source_tiles + self.appended_tiles
    }

    pub(super) fn relocated_lines(&self) -> usize {
        self.relocated_lines
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

    pub(super) fn validate_output_consumers(
        &self,
        source_rom: &[u8],
        output: &[u8],
        label: &str,
    ) -> Result<(), String> {
        validate_generic_renderer(output, &self.manifest, label)?;
        for line in &self.manifest.lines {
            validate_selection(output, line, label)?;
            let source_table_offset = parse_hex(&line.source_table_offset)?;
            let source_table_bytes = if line.source_table_bytes == 0 {
                line.frame_count * 2
            } else {
                line.source_table_bytes
            };
            let source_table = source_range(
                source_rom,
                source_table_offset,
                source_table_bytes,
                &format!("{} JP source frame table", line.id),
            )?;
            let protected_source_table = source_range(
                output,
                source_table_offset,
                source_table_bytes,
                &format!("{} protected source frame table", line.id),
            )?;
            if protected_source_table != source_table {
                return Err(format!("{label} {} changed its JP frame table", line.id));
            }

            match line.placement.as_str() {
                "in_place" => {
                    validate_type_pointer(output, line, source_table_offset, label)?;
                    let map_offset = parse_hex(&line.source_frame_map_offset)?;
                    let source_map = source_range(
                        source_rom,
                        map_offset,
                        line.source_frame_map_bytes,
                        &format!("{} JP frame map", line.id),
                    )?;
                    if source_range(
                        output,
                        map_offset,
                        line.source_frame_map_bytes,
                        &format!("{} protected frame map", line.id),
                    )? != source_map
                    {
                        return Err(format!("{label} {} changed its JP frame map", line.id));
                    }
                }
                "relocated_table" => {
                    validate_type_pointer(output, line, TARGET_TABLE_BANK_OFFSET, label)?;
                    validate_relocated_table(source_table, &self.table_bank, line, label)?;
                    if source_range(
                        output,
                        TARGET_TABLE_BANK_OFFSET,
                        self.table_bank.len(),
                        "output generic credit table",
                    )? != self.table_bank
                    {
                        return Err(format!("{label} {} relocated frame table drifted", line.id));
                    }
                }
                _ => unreachable!("manifest placement was validated"),
            }
        }
        Ok(())
    }
}

pub(super) fn compile_credits_generic(
    source_rom: &[u8],
    source_pages: &[Vec<u8>],
    target_pages: &mut [Vec<u8>],
    assets_dir: &Path,
) -> Result<CreditsGenericBuild, String> {
    if source_pages.len() != PAGE_COUNT || target_pages.len() != PAGE_COUNT {
        return Err("generic credit compiler requires ten source and target pages".to_string());
    }
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    validate_generic_renderer(source_rom, &manifest, "JP source")?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "generic credit name",
    )?;

    let baseline = target_pages.to_vec();
    let mut changed_ranges = vec![Vec::new(); PAGE_COUNT];
    let mut previews = Vec::with_capacity(LINE_COUNT);
    let mut glyphs = BTreeSet::new();
    let mut overwritten_source_tiles = 0usize;
    let mut appended_tiles = 0usize;
    let mut relocated_lines = 0usize;
    let mut table_bank = Vec::new();
    let mut data_patches = Vec::new();

    for line in &manifest.lines {
        validate_line_source(source_rom, line, "JP source")?;
        let source_page = source_pages
            .get(line.page)
            .ok_or_else(|| format!("{} references absent source page {}", line.id, line.page))?;
        let target_page = target_pages
            .get_mut(line.page)
            .ok_or_else(|| format!("{} references absent target page {}", line.id, line.page))?;
        if target_page.len() < source_page.len() {
            return Err(format!(
                "{} target page is shorter than its JP source",
                line.id
            ));
        }
        let source_surface = extract_line_surface(
            source_page,
            &line.source_logical_tiles,
            &format!("{} JP", line.id),
        )?;
        for segment in &line.segments {
            let start = segment.source_tile_start * MD_TILE_BYTES;
            let end = segment.source_tile_end_exclusive * MD_TILE_BYTES;
            let source_bytes = source_range(
                source_page,
                start,
                end - start,
                &format!("{} JP pattern segment", line.id),
            )?;
            if sha256_hex(source_bytes) != segment.source_sha256 {
                return Err(format!(
                    "{} JP segment {:?} pattern SHA-256 drifted",
                    line.id, segment.jp
                ));
            }
            glyphs.extend(
                segment
                    .ko
                    .chars()
                    .filter(|character| !character.is_whitespace()),
            );
        }

        let target_logical_tiles = match line.placement.as_str() {
            "in_place" => {
                for segment in &line.segments {
                    let start = segment.source_tile_start * MD_TILE_BYTES;
                    let end = segment.source_tile_end_exclusive * MD_TILE_BYTES;
                    if target_page[start..end] != source_page[start..end] {
                        return Err(format!(
                            "{} in-place patterns overlap an earlier credit mutation",
                            line.id
                        ));
                    }
                    let encoded = render_segment(&font, &manifest, line, segment)?;
                    if encoded.len() != end - start {
                        return Err(format!(
                            "{} in-place segment encoded to {} bytes instead of {}",
                            line.id,
                            encoded.len(),
                            end - start
                        ));
                    }
                    target_page[start..end].copy_from_slice(&encoded);
                    changed_ranges[line.page].push((start, end));
                    overwritten_source_tiles +=
                        segment.source_tile_end_exclusive - segment.source_tile_start;
                }
                line.source_logical_tiles.clone()
            }
            "relocated_table" => {
                if !target_page.len().is_multiple_of(MD_TILE_BYTES) {
                    return Err(format!("{} target page is not tile-aligned", line.id));
                }
                let mut target_tiles = Vec::new();
                for (index, segment) in line.segments.iter().enumerate() {
                    if index != 0 {
                        target_tiles.push(BLANK_TILE);
                    }
                    let encoded = render_segment(&font, &manifest, line, segment)?;
                    let first_tile = target_page.len() / MD_TILE_BYTES;
                    for cell in 0..segment.target_cells {
                        target_tiles.push(first_tile + cell * GLYPH_TILES);
                    }
                    appended_tiles += encoded.len() / MD_TILE_BYTES;
                    target_page.extend_from_slice(&encoded);
                }
                let source_table_offset = parse_hex(&line.source_table_offset)?;
                let source_table = source_range(
                    source_rom,
                    source_table_offset,
                    line.source_table_bytes,
                    &format!("{} source frame table", line.id),
                )?;
                let rebuilt = build_relocated_table(source_table, line, &target_tiles)?;
                if !table_bank.is_empty() {
                    return Err(
                        "generic credit compiler currently owns more than one relocated table"
                            .to_string(),
                    );
                }
                if TARGET_TABLE_BANK_OFFSET + rebuilt.len() > TARGET_TABLE_BANK_LIMIT {
                    return Err(format!("{} relocated table exceeds its bank", line.id));
                }
                table_bank = rebuilt;
                let source_pointer = u32::try_from(source_table_offset)
                    .map_err(|_| format!("{} source table address exceeds 32 bits", line.id))?
                    .to_be_bytes()
                    .to_vec();
                let target_pointer = u32::try_from(TARGET_TABLE_BANK_OFFSET)
                    .map_err(|_| format!("{} target table address exceeds 32 bits", line.id))?
                    .to_be_bytes()
                    .to_vec();
                data_patches.push(DataPatch {
                    offset: parse_hex(&line.type_pointer_offset)?,
                    source: source_pointer,
                    target: target_pointer,
                    label: format!("{} non-executable object-type table pointer", line.id),
                });
                relocated_lines += 1;
                target_tiles
            }
            other => {
                return Err(format!(
                    "{} has unsupported placement profile {other:?}",
                    line.id
                ));
            }
        };

        let target_surface = extract_line_surface(
            target_page,
            &target_logical_tiles,
            &format!("{} Korean", line.id),
        )?;
        previews.push(CreditLinePreview {
            source_surface,
            target_surface,
            source_width: line.source_logical_tiles.len() * CELL_WIDTH_PIXELS,
            target_width: target_logical_tiles.len() * CELL_WIDTH_PIXELS,
        });
    }

    for page in 0..PAGE_COUNT {
        let source_len = baseline[page].len();
        validate_only_ranges_changed(
            &baseline[page][..source_len],
            &target_pages[page][..source_len],
            &changed_ranges[page],
        )
        .map_err(|error| format!("generic credit page {page} ownership failed: {error}"))?;
        if !target_pages[page][source_len..]
            .len()
            .is_multiple_of(MD_TILE_BYTES)
        {
            return Err(format!(
                "generic credit page {page} appended suffix is not tile-aligned"
            ));
        }
    }

    let verified_consumer_bytes = manifest.generic_renderer.window_bytes
        + manifest
            .lines
            .iter()
            .map(|line| {
                POINTER_ENTRY_BYTES
                    + line.selection_window_bytes
                    + if line.source_table_bytes == 0 {
                        line.frame_count * 2 + line.source_frame_map_bytes
                    } else {
                        line.source_table_bytes
                    }
            })
            .sum::<usize>();
    Ok(CreditsGenericBuild {
        manifest,
        previews,
        glyphs,
        overwritten_source_tiles,
        appended_tiles,
        relocated_lines,
        verified_consumer_bytes,
        table_bank,
        data_patches,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<CreditsGenericManifest, String> {
    let path = assets_dir.join("graphics_text/credits_generic_frames.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read generic credit source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid generic credit source {}: {error}", path.display()))
}

fn validate_manifest_shape(manifest: &CreditsGenericManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-CREDITS-GENERIC-FRAMES"
        || !manifest.source_policy.contains("English VWF")
        || manifest.standalone_ascii_policy
            != "preserve_unmapped_standalone_ascii_escape_doors_owned_by_escape_doors_manifest"
        || manifest.preserved_complete_latin_names != ["Jemini", "RIU", "TAKIN"]
        || manifest.font_asset != "neodgm.ttf"
        || manifest.render_mode != "jp_native_16x16_no_horizontal_scaling"
        || manifest.space_width_px != SPACE_WIDTH_PIXELS
        || manifest.lines.len() != LINE_COUNT
    {
        return Err("generic credit manifest identity drifted".to_string());
    }
    validate_sha256(&manifest.font_sha256, "generic credit font")?;
    let renderer = &manifest.generic_renderer;
    if parse_hex(&renderer.window_offset)? != GENERIC_RENDERER_OFFSET
        || renderer.window_bytes != GENERIC_RENDERER_BYTES
        || parse_hex(&renderer.pointer_table_offset)? != POINTER_TABLE_OFFSET
        || renderer.pointer_entry_bytes != POINTER_ENTRY_BYTES
        || parse_u16_hex(&renderer.object_type_displacement)? != OBJECT_TYPE_DISPLACEMENT
        || parse_u16_hex(&renderer.frame_index_displacement)? != FRAME_INDEX_DISPLACEMENT
    {
        return Err("generic credit renderer declaration drifted".to_string());
    }
    validate_sha256(&renderer.window_sha256, "generic credit renderer")?;
    if parse_hex(&manifest.target_table_bank.offset)? != TARGET_TABLE_BANK_OFFSET
        || parse_hex(&manifest.target_table_bank.limit)? != TARGET_TABLE_BANK_LIMIT
        || parse_u8_hex(&manifest.target_table_bank.fill_byte)? != TARGET_TABLE_BANK_FILL
    {
        return Err("generic credit target-table bank drifted".to_string());
    }

    let expected = [
        ("GFX-CREDITS-P04-L01", 4usize, "in_place"),
        ("GFX-CREDITS-P08-L01", 8usize, "relocated_table"),
    ];
    for (line, (id, page, placement)) in manifest.lines.iter().zip(expected) {
        if line.id != id
            || line.page != page
            || line.placement != placement
            || line.jp.is_empty()
            || line.ko.is_empty()
            || is_single_ascii_letter(&line.jp)
            || is_single_ascii_letter(&line.ko)
            || line.segments.is_empty()
            || line.frame_count == 0
            || line.frame_index >= line.frame_count
        {
            return Err(format!("{} identity or geometry drifted", line.id));
        }
        for hash in [
            &line.type_pointer_sha256,
            &line.source_frame_map_sha256,
            &line.selection_window_sha256,
        ] {
            validate_sha256(hash, &line.id)?;
        }
        if let Some(hash) = &line.source_table_sha256 {
            validate_sha256(hash, &line.id)?;
        }
        let pointer_offset = parse_hex(&line.type_pointer_offset)?;
        let object_type = usize::from(parse_u8_hex(&line.object_type)?);
        if pointer_offset != POINTER_TABLE_OFFSET + object_type * POINTER_ENTRY_BYTES {
            return Err(format!("{} object-type pointer slot drifted", line.id));
        }
        let source_table_offset = parse_hex(&line.source_table_offset)?;
        if parse_hex(&line.source_frame_map_offset)?
            != source_table_offset + parse_hex(&line.source_frame_offset)?
        {
            return Err(format!("{} source frame binding drifted", line.id));
        }
        let selection_start = parse_hex(&line.selection_window_offset)?;
        let selection_offset = parse_hex(&line.selection_instruction_offset)?;
        let selector = selection_instruction(line)?;
        if selection_offset < selection_start
            || selection_offset + selector.len() > selection_start + line.selection_window_bytes
        {
            return Err(format!("{} selector escapes its consumer window", line.id));
        }

        let mut mutable_tiles = Vec::new();
        for segment in &line.segments {
            if segment.jp.is_empty()
                || segment.ko.is_empty()
                || segment.target_cells == 0
                || segment.source_tile_end_exclusive <= segment.source_tile_start
                || !(segment.source_tile_end_exclusive - segment.source_tile_start)
                    .is_multiple_of(GLYPH_TILES)
                || native_text_width(&segment.ko, SPACE_WIDTH_PIXELS)
                    > segment.target_cells * CELL_WIDTH_PIXELS
            {
                return Err(format!("{} has an invalid pattern segment", line.id));
            }
            validate_sha256(&segment.source_sha256, &line.id)?;
            mutable_tiles.extend(
                (segment.source_tile_start..segment.source_tile_end_exclusive).step_by(GLYPH_TILES),
            );
        }
        let visible_source_tiles = line
            .source_logical_tiles
            .iter()
            .copied()
            .filter(|&tile| tile != BLANK_TILE)
            .collect::<Vec<_>>();
        if visible_source_tiles != mutable_tiles {
            return Err(format!(
                "{} source map does not exactly cover its mutable patterns",
                line.id
            ));
        }
        let target_cells = line
            .segments
            .iter()
            .map(|segment| segment.target_cells)
            .sum::<usize>()
            + line.segments.len().saturating_sub(1);
        match line.placement.as_str() {
            "in_place" => {
                if target_cells != line.source_logical_tiles.len()
                    || line.source_table_bytes != 0
                    || line.source_table_sha256.is_some()
                    || line.target_first_x.is_some()
                    || line.target_x_step.is_some()
                    || line.selection_profile != "clear_byte_displacement_a0"
                    || line.frame_index != 0
                {
                    return Err(format!("{} in-place policy drifted", line.id));
                }
            }
            "relocated_table" => {
                if target_cells != 7
                    || line.source_table_bytes == 0
                    || line.source_table_sha256.is_none()
                    || line
                        .target_first_x
                        .as_deref()
                        .map(parse_u16_hex)
                        .transpose()?
                        != Some(0x0048)
                    || line.target_x_step != Some(16)
                    || line.selection_profile != "move_byte_immediate_to_displacement_a1"
                {
                    return Err(format!("{} relocated-table policy drifted", line.id));
                }
            }
            _ => unreachable!("placement identity was checked above"),
        }
    }
    Ok(())
}

fn validate_generic_renderer(
    rom: &[u8],
    manifest: &CreditsGenericManifest,
    label: &str,
) -> Result<(), String> {
    validate_generic_renderer_contract(rom, &manifest.generic_renderer.window_sha256, label)
}

pub(super) fn validate_generic_renderer_contract(
    rom: &[u8],
    expected_sha256: &str,
    label: &str,
) -> Result<(), String> {
    let window = source_range(
        rom,
        GENERIC_RENDERER_OFFSET,
        GENERIC_RENDERER_BYTES,
        "generic sprite renderer",
    )?;
    if sha256_hex(window) != expected_sha256 {
        return Err(format!("{label} generic sprite renderer window drifted"));
    }
    let fixtures = [
        (
            0x0000_18DE,
            m68k::assemble(&[Inst::MoveByteDisplacementAddressToData {
                displacement: OBJECT_TYPE_DISPLACEMENT,
                source: AddressReg::A0,
                destination: DataReg::D0,
            }])?,
        ),
        (
            0x0000_18E2,
            m68k::assemble(&[Inst::MoveByteDisplacementAddressToData {
                displacement: FRAME_INDEX_DISPLACEMENT,
                source: AddressReg::A0,
                destination: DataReg::D1,
            }])?,
        ),
        (
            0x0000_18E8,
            m68k::assemble(&[Inst::LeaAbsoluteLong {
                address: POINTER_TABLE_OFFSET as u32,
                destination: AddressReg::A1,
            }])?,
        ),
        (
            0x0000_18EE,
            m68k::assemble(&[Inst::AslWordImmediate {
                count: 2,
                destination: DataReg::D0,
            }])?,
        ),
        (
            0x0000_18F0,
            m68k::assemble(&[Inst::MoveAddressLongIndexedWordToAddress {
                base: AddressReg::A1,
                index: DataReg::D0,
                destination: AddressReg::A2,
            }])?,
        ),
        (
            0x0000_18F4,
            m68k::assemble(&[Inst::AslWordImmediate {
                count: 1,
                destination: DataReg::D1,
            }])?,
        ),
        (
            0x0000_18F6,
            m68k::assemble(&[Inst::MoveWordIndexedAddressToData {
                displacement: 0,
                base: AddressReg::A2,
                index: DataReg::D1,
                destination: DataReg::D0,
            }])?,
        ),
    ];
    for (offset, expected) in fixtures {
        if source_range(
            rom,
            offset,
            expected.len(),
            "typed generic renderer instruction",
        )? != expected
        {
            return Err(format!(
                "{label} typed generic renderer instruction at 0x{offset:06X} drifted"
            ));
        }
    }
    Ok(())
}

fn validate_line_source(rom: &[u8], line: &LineDeclaration, label: &str) -> Result<(), String> {
    validate_selection(rom, line, label)?;
    let pointer_offset = parse_hex(&line.type_pointer_offset)?;
    let pointer = source_range(
        rom,
        pointer_offset,
        POINTER_ENTRY_BYTES,
        "type table pointer",
    )?;
    if sha256_hex(pointer) != line.type_pointer_sha256 {
        return Err(format!("{label} {} type pointer SHA-256 drifted", line.id));
    }
    validate_type_pointer(rom, line, parse_hex(&line.source_table_offset)?, label)?;
    let table_offset = parse_hex(&line.source_table_offset)?;
    let first_offset = usize::from(u16::from_be_bytes([
        source_range(rom, table_offset, 2, "frame-zero offset")?[0],
        source_range(rom, table_offset, 2, "frame-zero offset")?[1],
    ]));
    if first_offset != line.frame_count * 2 {
        return Err(format!("{} frame-count boundary drifted", line.id));
    }
    let frame_offset_address = table_offset + line.frame_index * 2;
    let frame_offset_bytes = source_range(rom, frame_offset_address, 2, "declared frame offset")?;
    if usize::from(u16::from_be_bytes([
        frame_offset_bytes[0],
        frame_offset_bytes[1],
    ])) != parse_hex(&line.source_frame_offset)?
    {
        return Err(format!("{} selected frame offset drifted", line.id));
    }
    let map_offset = parse_hex(&line.source_frame_map_offset)?;
    let map = source_range(
        rom,
        map_offset,
        line.source_frame_map_bytes,
        &format!("{} selected frame map", line.id),
    )?;
    if sha256_hex(map) != line.source_frame_map_sha256 {
        return Err(format!("{label} {} selected frame map drifted", line.id));
    }
    match line.placement.as_str() {
        "in_place" => validate_page4_map(map, label),
        "relocated_table" => {
            let table = source_range(
                rom,
                table_offset,
                line.source_table_bytes,
                &format!("{} complete source table", line.id),
            )?;
            if sha256_hex(table)
                != line
                    .source_table_sha256
                    .as_deref()
                    .ok_or_else(|| format!("{} source table SHA is absent", line.id))?
            {
                return Err(format!("{label} {} complete frame table drifted", line.id));
            }
            validate_page8_map(map, &[45, 49, 53, 57], &[0x58, 0x68, 0x88, 0x98], label)
        }
        _ => unreachable!("manifest placement was validated"),
    }
}

fn validate_selection(rom: &[u8], line: &LineDeclaration, label: &str) -> Result<(), String> {
    let window_offset = parse_hex(&line.selection_window_offset)?;
    let window = source_range(
        rom,
        window_offset,
        line.selection_window_bytes,
        &format!("{} frame-selection window", line.id),
    )?;
    if sha256_hex(window) != line.selection_window_sha256 {
        return Err(format!(
            "{label} {} frame-selection window drifted",
            line.id
        ));
    }
    let selector = selection_instruction(line)?;
    let selector_offset = parse_hex(&line.selection_instruction_offset)?;
    if source_range(
        rom,
        selector_offset,
        selector.len(),
        &format!("{} typed frame selector", line.id),
    )? != selector
    {
        return Err(format!("{label} {} typed frame selector drifted", line.id));
    }
    Ok(())
}

fn selection_instruction(line: &LineDeclaration) -> Result<Vec<u8>, String> {
    let instruction = match line.selection_profile.as_str() {
        "clear_byte_displacement_a0" => Inst::ClearByteDisplacementAddress {
            displacement: FRAME_INDEX_DISPLACEMENT,
            destination: AddressReg::A0,
        },
        "move_byte_immediate_to_displacement_a1" => Inst::MoveByteImmediateToDisplacementAddress {
            immediate: u8::try_from(line.frame_index)
                .map_err(|_| format!("{} frame index exceeds one byte", line.id))?,
            displacement: FRAME_INDEX_DISPLACEMENT,
            destination: AddressReg::A1,
        },
        other => {
            return Err(format!(
                "{} has unsupported typed frame selector {other:?}",
                line.id
            ));
        }
    };
    m68k::assemble(&[instruction])
}

fn validate_type_pointer(
    rom: &[u8],
    line: &LineDeclaration,
    expected: usize,
    label: &str,
) -> Result<(), String> {
    let pointer = source_range(
        rom,
        parse_hex(&line.type_pointer_offset)?,
        POINTER_ENTRY_BYTES,
        &format!("{} type pointer", line.id),
    )?;
    let actual = u32::from_be_bytes([pointer[0], pointer[1], pointer[2], pointer[3]]) as usize;
    if actual != expected {
        return Err(format!(
            "{label} {} type pointer is 0x{actual:06X}, expected 0x{expected:06X}",
            line.id
        ));
    }
    Ok(())
}

fn validate_page4_map(map: &[u8], label: &str) -> Result<(), String> {
    if map.len() != 10
        || u16::from_be_bytes([map[0], map[1]]) != 1
        || u16::from_be_bytes([map[2], map[3]]) != 0x0080
        || u16::from_be_bytes([map[4], map[5]]) & 0x0F00 != 0x0D00
        || u16::from_be_bytes([map[6], map[7]]) & TILE_INDEX_MASK != 45
        || u16::from_be_bytes([map[8], map[9]]) != 0x00C0
    {
        return Err(format!(
            "{label} page-4 Kemi map is not its declared 4x2 sprite"
        ));
    }
    Ok(())
}

fn validate_page8_map(
    map: &[u8],
    expected_tiles: &[usize],
    expected_x: &[u16],
    label: &str,
) -> Result<(), String> {
    if expected_tiles.len() != expected_x.len()
        || map.len() != 2 + expected_tiles.len() * 8
        || usize::from(u16::from_be_bytes([map[0], map[1]])) != expected_tiles.len()
    {
        return Err(format!("{label} page-8 name map length drifted"));
    }
    for (index, ((record, &tile), &x)) in map[2..]
        .chunks_exact(8)
        .zip(expected_tiles)
        .zip(expected_x)
        .enumerate()
    {
        let tile_word = u16::from_be_bytes([record[4], record[5]]);
        if u16::from_be_bytes([record[0], record[1]]) != 0x0078
            || u16::from_be_bytes([record[2], record[3]]) & 0x0F00 != 0x0500
            || tile_word & !TILE_INDEX_MASK != 0x2000
            || usize::from(tile_word & TILE_INDEX_MASK) != tile
            || u16::from_be_bytes([record[6], record[7]]) != x
        {
            return Err(format!(
                "{label} page-8 name record {index} geometry drifted"
            ));
        }
    }
    Ok(())
}

fn build_relocated_table(
    source_table: &[u8],
    line: &LineDeclaration,
    target_tiles: &[usize],
) -> Result<Vec<u8>, String> {
    let source_offsets = parse_frame_offsets(source_table, line.frame_count, &line.id)?;
    if source_offsets[line.frame_index] != parse_hex(&line.source_frame_offset)? {
        return Err(format!("{} selected source offset drifted", line.id));
    }
    let source_map = frame_slice(source_table, &source_offsets, line.frame_index, &line.id)?;
    let target_map = build_page8_map(source_map, target_tiles, line)?;
    let table_bytes = line.frame_count * 2;
    let mut target = vec![0u8; table_bytes];
    for frame in 0..line.frame_count {
        let offset = u16::try_from(target.len())
            .map_err(|_| format!("{} rebuilt frame table exceeds 64 KiB", line.id))?;
        target[frame * 2..frame * 2 + 2].copy_from_slice(&offset.to_be_bytes());
        if frame == line.frame_index {
            target.extend_from_slice(&target_map);
        } else {
            target.extend_from_slice(frame_slice(source_table, &source_offsets, frame, &line.id)?);
        }
    }
    validate_relocated_table(source_table, &target, line, "compiled")?;
    Ok(target)
}

fn build_page8_map(
    source_map: &[u8],
    target_tiles: &[usize],
    line: &LineDeclaration,
) -> Result<Vec<u8>, String> {
    let template = source_map
        .get(2..10)
        .ok_or_else(|| format!("{} source frame lacks a template record", line.id))?;
    let first_x = parse_u16_hex(
        line.target_first_x
            .as_deref()
            .ok_or_else(|| format!("{} target first x is absent", line.id))?,
    )?;
    let step = line
        .target_x_step
        .ok_or_else(|| format!("{} target x step is absent", line.id))?;
    let count = u16::try_from(target_tiles.len())
        .map_err(|_| format!("{} target record count exceeds 16 bits", line.id))?;
    let mut output = Vec::with_capacity(2 + target_tiles.len() * 8);
    output.extend_from_slice(&count.to_be_bytes());
    for (index, &tile) in target_tiles.iter().enumerate() {
        let tile =
            u16::try_from(tile).map_err(|_| format!("{} target tile exceeds 16 bits", line.id))?;
        if tile & !TILE_INDEX_MASK != 0 {
            return Err(format!("{} target tile exceeds the consumer mask", line.id));
        }
        let mut record = template.to_vec();
        let source_tile_word = u16::from_be_bytes([record[4], record[5]]);
        record[4..6].copy_from_slice(&((source_tile_word & !TILE_INDEX_MASK) | tile).to_be_bytes());
        let x = usize::from(first_x)
            .checked_add(
                index
                    .checked_mul(step)
                    .ok_or_else(|| format!("{} target x overflowed", line.id))?,
            )
            .ok_or_else(|| format!("{} target x overflowed", line.id))?;
        record[6..8].copy_from_slice(
            &u16::try_from(x)
                .map_err(|_| format!("{} target x exceeds 16 bits", line.id))?
                .to_be_bytes(),
        );
        output.extend_from_slice(&record);
    }
    let expected_x = (0..target_tiles.len())
        .map(|index| first_x + u16::try_from(index * step).unwrap())
        .collect::<Vec<_>>();
    validate_page8_map(&output, target_tiles, &expected_x, "compiled")?;
    Ok(output)
}

fn validate_relocated_table(
    source_table: &[u8],
    target_table: &[u8],
    line: &LineDeclaration,
    label: &str,
) -> Result<(), String> {
    let source_offsets = parse_frame_offsets(source_table, line.frame_count, &line.id)?;
    let target_offsets = parse_frame_offsets(target_table, line.frame_count, &line.id)?;
    for frame in 0..line.frame_count {
        let target_frame = frame_slice(target_table, &target_offsets, frame, &line.id)?;
        if frame == line.frame_index {
            let first_x = parse_u16_hex(
                line.target_first_x
                    .as_deref()
                    .ok_or_else(|| format!("{} target first x is absent", line.id))?,
            )?;
            let step = line
                .target_x_step
                .ok_or_else(|| format!("{} target x step is absent", line.id))?;
            let expected_x = (0..7)
                .map(|index| first_x + u16::try_from(index * step).unwrap())
                .collect::<Vec<_>>();
            let tiles = target_frame[2..]
                .chunks_exact(8)
                .map(|record| {
                    usize::from(u16::from_be_bytes([record[4], record[5]]) & TILE_INDEX_MASK)
                })
                .collect::<Vec<_>>();
            validate_page8_map(target_frame, &tiles, &expected_x, label)?;
        } else if target_frame != frame_slice(source_table, &source_offsets, frame, &line.id)? {
            return Err(format!(
                "{label} {} unrelated frame {frame} changed",
                line.id
            ));
        }
    }
    Ok(())
}

fn parse_frame_offsets(
    table: &[u8],
    frame_count: usize,
    label: &str,
) -> Result<Vec<usize>, String> {
    let table_bytes = frame_count
        .checked_mul(2)
        .ok_or_else(|| format!("{label} frame table length overflowed"))?;
    let header = source_range(table, 0, table_bytes, "frame offset table")?;
    let offsets = header
        .chunks_exact(2)
        .map(|pair| usize::from(u16::from_be_bytes([pair[0], pair[1]])))
        .collect::<Vec<_>>();
    if offsets.first().copied() != Some(table_bytes)
        || offsets.iter().any(|&offset| offset < table_bytes)
        || offsets.windows(2).any(|pair| pair[0] >= pair[1])
        || offsets.last().is_none_or(|&offset| offset >= table.len())
    {
        return Err(format!(
            "{label} frame offsets are not a strict source table"
        ));
    }
    Ok(offsets)
}

fn frame_slice<'a>(
    table: &'a [u8],
    offsets: &[usize],
    frame: usize,
    label: &str,
) -> Result<&'a [u8], String> {
    let start = *offsets
        .get(frame)
        .ok_or_else(|| format!("{label} frame {frame} is absent"))?;
    let end = offsets.get(frame + 1).copied().unwrap_or(table.len());
    source_range(table, start, end - start, &format!("{label} frame {frame}"))
}

fn render_segment(
    font: &Font,
    manifest: &CreditsGenericManifest,
    line: &LineDeclaration,
    segment: &SegmentDeclaration,
) -> Result<Vec<u8>, String> {
    let width = segment.target_cells * CELL_WIDTH_PIXELS;
    let surface = render_native_text_line(
        font,
        &segment.ko,
        width,
        manifest.space_width_px,
        0,
        15,
        &line.id,
    )?;
    let encoded = encode_md_tiles_column_major(
        &surface,
        width,
        CELL_HEIGHT_PIXELS,
        &format!("{} Korean generic credit segment", line.id),
    )?;
    let roles = encoded
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 15]) {
        return Err(format!(
            "{} Korean generic credit segment uses unexpected palette roles {roles:?}",
            line.id
        ));
    }
    Ok(encoded)
}

fn extract_line_surface(
    payload: &[u8],
    logical_tiles: &[usize],
    label: &str,
) -> Result<Vec<u8>, String> {
    let width = logical_tiles.len() * CELL_WIDTH_PIXELS;
    let mut surface = vec![0u8; width * CELL_HEIGHT_PIXELS];
    for (cell, &tile) in logical_tiles.iter().enumerate() {
        if tile == BLANK_TILE {
            continue;
        }
        let start = tile * MD_TILE_BYTES;
        let bytes = source_range(
            payload,
            start,
            GLYPH_TILES * MD_TILE_BYTES,
            &format!("{label} visible pattern cell"),
        )?;
        let decoded = decode_md_tiles_column_major(
            bytes,
            CELL_WIDTH_PIXELS,
            CELL_HEIGHT_PIXELS,
            &format!("{label} visible pattern cell"),
        )?;
        for y in 0..CELL_HEIGHT_PIXELS {
            let source_row = &decoded[y * CELL_WIDTH_PIXELS..(y + 1) * CELL_WIDTH_PIXELS];
            let destination = y * width + cell * CELL_WIDTH_PIXELS;
            surface[destination..destination + CELL_WIDTH_PIXELS].copy_from_slice(source_row);
        }
    }
    Ok(surface)
}

fn is_single_ascii_letter(value: &str) -> bool {
    let mut characters = value.trim().chars();
    matches!(
        (characters.next(), characters.next()),
        (Some(character), None) if character.is_ascii_alphabetic()
    )
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
