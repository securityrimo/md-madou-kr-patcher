//! JP-source compiler for native-cell ending-credit frame sequences.
//!
//! This batch covers three generic-sprite consumers whose visible geometry is
//! already closed from the JP ROM:
//!
//! - page 7 `阿門` keeps its original one-frame 4x2 sprite;
//! - page 7 `壱` widens the original one-frame 2x2 sprite to 4x2;
//! - page 9 `コンパイルのみんな` keeps all nine timed objects and remaps their
//!   one-cell frames to `컴파일 일동`, including explicit blank frames.
//!
//! Korean patterns come only from the checked repository font. Source frame
//! tables remain byte-identical in place. Geometry-changing tables are rebuilt
//! as non-executable data in an owned bank, and only their checked object-type
//! pointers are redirected. No executable bytes are written.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use crate::m68k::{self, AddressReg, DataReg, Inst};

use super::credits_generic::{
    DataPatch, FRAME_INDEX_DISPLACEMENT, GENERIC_RENDERER_BYTES, GENERIC_RENDERER_OFFSET,
    OBJECT_TYPE_DISPLACEMENT, POINTER_ENTRY_BYTES, POINTER_TABLE_OFFSET,
    validate_generic_renderer_contract,
};
use super::credits_timed::CreditLinePreview;
use super::font_effect::{native_text_width, read_verified_font, render_native_text_line};
use super::pixel::{decode_md_tiles_column_major, encode_md_tiles_column_major};
use super::{MD_TILE_BYTES, parse_hex, sha256_hex, source_range, validate_only_ranges_changed};

const LINE_COUNT: usize = 3;
const PAGE_COUNT: usize = 10;
const CELL_WIDTH_PIXELS: usize = 16;
const CELL_HEIGHT_PIXELS: usize = 16;
const SPACE_WIDTH_PIXELS: usize = 8;
const GLYPH_TILES: usize = 4;
const BLANK_TILE: usize = 5;
const TILE_INDEX_MASK: u16 = 0x07FF;
const TARGET_TABLE_BANK_OFFSET: usize = 0x2F_C000;
const TARGET_TABLE_BANK_LIMIT: usize = 0x2F_E000;
const TARGET_TABLE_BANK_FILL: u8 = 0xFF;

#[derive(Debug, Deserialize)]
struct CreditsNativeFramesManifest {
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
    pattern_placement: String,
    table_profile: String,
    source_logical_tiles: Vec<usize>,
    target_frame_cells: Vec<Option<usize>>,
    source_pattern_start: usize,
    source_pattern_end_exclusive: usize,
    source_pattern_sha256: String,
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
    object_type_instruction_offset: String,
    frame_index_instruction_offset: Option<String>,
    render_segments: Vec<RenderSegmentDeclaration>,
}

#[derive(Debug, Deserialize)]
struct RenderSegmentDeclaration {
    ko: String,
    target_cell_start: usize,
    target_cells: usize,
}

#[derive(Debug)]
struct NativeLineBuild {
    preview: CreditLinePreview,
    first_target_tile: usize,
    target_logical_tiles: Vec<usize>,
    target_table_offset: Option<usize>,
    target_table: Option<Vec<u8>>,
}

#[derive(Debug)]
pub(super) struct CreditsNativeFramesBuild {
    manifest: CreditsNativeFramesManifest,
    lines: Vec<NativeLineBuild>,
    glyphs: BTreeSet<char>,
    overwritten_source_tiles: usize,
    appended_tiles: usize,
    relocated_lines: usize,
    verified_consumer_bytes: usize,
    table_bank: Vec<u8>,
    data_patches: Vec<DataPatch>,
}

impl CreditsNativeFramesBuild {
    pub(super) fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub(super) fn preview_lines(&self) -> impl Iterator<Item = &CreditLinePreview> {
        self.lines.iter().map(|line| &line.preview)
    }

    pub(super) fn glyphs(&self) -> &BTreeSet<char> {
        &self.glyphs
    }

    pub(super) fn rewritten_tiles(&self) -> usize {
        self.overwritten_source_tiles + self.appended_tiles
    }

    pub(super) fn overwritten_source_tiles(&self) -> usize {
        self.overwritten_source_tiles
    }

    pub(super) fn appended_tiles(&self) -> usize {
        self.appended_tiles
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
        validate_generic_renderer_contract(
            output,
            &self.manifest.generic_renderer.window_sha256,
            label,
        )?;
        if source_range(
            output,
            TARGET_TABLE_BANK_OFFSET,
            self.table_bank.len(),
            "native-frame output table bank",
        )? != self.table_bank
        {
            return Err(format!("{label} native-frame table bank drifted"));
        }

        for (line, built) in self.manifest.lines.iter().zip(&self.lines) {
            validate_selection(output, line, label)?;
            let source_table_offset = parse_hex(&line.source_table_offset)?;
            let source_table = source_range(
                source_rom,
                source_table_offset,
                line.source_table_bytes,
                &format!("{} JP source frame table", line.id),
            )?;
            if source_range(
                output,
                source_table_offset,
                line.source_table_bytes,
                &format!("{} protected source frame table", line.id),
            )? != source_table
            {
                return Err(format!("{label} {} changed its JP frame table", line.id));
            }

            match (&built.target_table_offset, &built.target_table) {
                (None, None) => {
                    validate_type_pointer(output, line, source_table_offset, label)?;
                    validate_target_table(
                        source_table,
                        source_table,
                        line,
                        built.first_target_tile,
                        &built.target_logical_tiles,
                        label,
                    )?;
                }
                (Some(offset), Some(table)) => {
                    validate_type_pointer(output, line, *offset, label)?;
                    if source_range(
                        output,
                        *offset,
                        table.len(),
                        &format!("{} relocated output table", line.id),
                    )? != table
                    {
                        return Err(format!("{label} {} relocated table drifted", line.id));
                    }
                    validate_target_table(
                        source_table,
                        table,
                        line,
                        built.first_target_tile,
                        &built.target_logical_tiles,
                        label,
                    )?;
                }
                _ => {
                    return Err(format!(
                        "{label} {} has an incomplete relocated-table build",
                        line.id
                    ));
                }
            }
        }
        Ok(())
    }
}

pub(super) fn compile_credits_native_frames(
    source_rom: &[u8],
    source_pages: &[Vec<u8>],
    target_pages: &mut [Vec<u8>],
    assets_dir: &Path,
) -> Result<CreditsNativeFramesBuild, String> {
    if source_pages.len() != PAGE_COUNT || target_pages.len() != PAGE_COUNT {
        return Err(
            "native-frame credit compiler requires ten source and target pages".to_string(),
        );
    }
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    validate_generic_renderer_contract(
        source_rom,
        &manifest.generic_renderer.window_sha256,
        "JP source",
    )?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "native-frame credit",
    )?;

    let baseline = target_pages.to_vec();
    let mut changed_ranges = vec![Vec::new(); PAGE_COUNT];
    let mut lines = Vec::with_capacity(LINE_COUNT);
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
        let source_pattern_start = line.source_pattern_start * MD_TILE_BYTES;
        let source_pattern_end = line.source_pattern_end_exclusive * MD_TILE_BYTES;
        let source_pattern = source_range(
            source_page,
            source_pattern_start,
            source_pattern_end - source_pattern_start,
            &format!("{} JP pattern ownership", line.id),
        )?;
        if sha256_hex(source_pattern) != line.source_pattern_sha256 {
            return Err(format!("{} JP pattern SHA-256 drifted", line.id));
        }
        if target_page.len() < source_page.len()
            || target_page[source_pattern_start..source_pattern_end] != *source_pattern
        {
            return Err(format!(
                "{} pattern ownership overlaps an earlier credit mutation",
                line.id
            ));
        }

        let source_surface = extract_line_surface(
            source_page,
            &line.source_logical_tiles,
            &format!("{} JP", line.id),
        )?;
        let target_patterns = render_target_patterns(&font, &manifest, line)?;
        for segment in &line.render_segments {
            glyphs.extend(
                segment
                    .ko
                    .chars()
                    .filter(|character| !character.is_whitespace()),
            );
        }

        let first_target_tile = match line.pattern_placement.as_str() {
            "in_place_exact" => {
                if target_patterns.len() != source_pattern.len() {
                    return Err(format!(
                        "{} exact in-place target has {} bytes instead of {}",
                        line.id,
                        target_patterns.len(),
                        source_pattern.len()
                    ));
                }
                target_page[source_pattern_start..source_pattern_end]
                    .copy_from_slice(&target_patterns);
                changed_ranges[line.page].push((source_pattern_start, source_pattern_end));
                overwritten_source_tiles += target_patterns.len() / MD_TILE_BYTES;
                line.source_pattern_start
            }
            "in_place_prefix" => {
                if target_patterns.len() >= source_pattern.len() {
                    return Err(format!(
                        "{} prefix target does not leave a protected source suffix",
                        line.id
                    ));
                }
                let target_end = source_pattern_start + target_patterns.len();
                target_page[source_pattern_start..target_end].copy_from_slice(&target_patterns);
                changed_ranges[line.page].push((source_pattern_start, target_end));
                overwritten_source_tiles += target_patterns.len() / MD_TILE_BYTES;
                line.source_pattern_start
            }
            "append" => {
                if !target_page.len().is_multiple_of(MD_TILE_BYTES) {
                    return Err(format!("{} target page is not tile-aligned", line.id));
                }
                let first_tile = target_page.len() / MD_TILE_BYTES;
                target_page.extend_from_slice(&target_patterns);
                appended_tiles += target_patterns.len() / MD_TILE_BYTES;
                first_tile
            }
            other => {
                return Err(format!(
                    "{} has unsupported pattern placement {other:?}",
                    line.id
                ));
            }
        };

        if line.target_frame_cells.iter().any(Option::is_none) {
            validate_blank_cell(target_page, &line.id)?;
        }
        let target_logical_tiles = line
            .target_frame_cells
            .iter()
            .map(|cell| cell.map_or(BLANK_TILE, |index| first_target_tile + index * GLYPH_TILES))
            .collect::<Vec<_>>();

        let source_table_offset = parse_hex(&line.source_table_offset)?;
        let source_table = source_range(
            source_rom,
            source_table_offset,
            line.source_table_bytes,
            &format!("{} JP source frame table", line.id),
        )?;
        let target_table =
            build_target_table(source_table, line, first_target_tile, &target_logical_tiles)?;
        let target_table_offset = if let Some(table) = &target_table {
            let offset = TARGET_TABLE_BANK_OFFSET
                .checked_add(table_bank.len())
                .ok_or_else(|| "native-frame table allocation overflowed".to_string())?;
            if offset + table.len() > TARGET_TABLE_BANK_LIMIT {
                return Err(format!("{} relocated table exceeds its bank", line.id));
            }
            table_bank.extend_from_slice(table);
            data_patches.push(DataPatch {
                offset: parse_hex(&line.type_pointer_offset)?,
                source: u32::try_from(source_table_offset)
                    .map_err(|_| format!("{} source table exceeds 32 bits", line.id))?
                    .to_be_bytes()
                    .to_vec(),
                target: u32::try_from(offset)
                    .map_err(|_| format!("{} target table exceeds 32 bits", line.id))?
                    .to_be_bytes()
                    .to_vec(),
                label: format!("{} non-executable object-type table pointer", line.id),
            });
            relocated_lines += 1;
            Some(offset)
        } else {
            None
        };

        let target_surface = extract_line_surface(
            target_page,
            &target_logical_tiles,
            &format!("{} Korean", line.id),
        )?;
        lines.push(NativeLineBuild {
            preview: CreditLinePreview {
                source_surface,
                target_surface,
                source_width: line.source_logical_tiles.len() * CELL_WIDTH_PIXELS,
                target_width: target_logical_tiles.len() * CELL_WIDTH_PIXELS,
            },
            first_target_tile,
            target_logical_tiles,
            target_table_offset,
            target_table,
        });
    }

    for page in 0..PAGE_COUNT {
        let source_len = baseline[page].len();
        validate_only_ranges_changed(
            &baseline[page][..source_len],
            &target_pages[page][..source_len],
            &changed_ranges[page],
        )
        .map_err(|error| format!("native-frame credit page {page} ownership failed: {error}"))?;
        if !target_pages[page][source_len..]
            .len()
            .is_multiple_of(MD_TILE_BYTES)
        {
            return Err(format!(
                "native-frame credit page {page} appended suffix is not tile-aligned"
            ));
        }
    }

    data_patches.sort_by_key(|patch| patch.offset);
    let verified_consumer_bytes = GENERIC_RENDERER_BYTES
        + manifest
            .lines
            .iter()
            .map(|line| POINTER_ENTRY_BYTES + line.source_table_bytes + line.selection_window_bytes)
            .sum::<usize>();
    Ok(CreditsNativeFramesBuild {
        manifest,
        lines,
        glyphs,
        overwritten_source_tiles,
        appended_tiles,
        relocated_lines,
        verified_consumer_bytes,
        table_bank,
        data_patches,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<CreditsNativeFramesManifest, String> {
    let path = assets_dir.join("graphics_text/credits_native_frames.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read native-frame credit source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "invalid native-frame credit source {}: {error}",
            path.display()
        )
    })
}

fn validate_manifest_shape(manifest: &CreditsNativeFramesManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.asset_group_id != "GFX-CREDITS-NATIVE-FRAMES"
        || !manifest.source_policy.contains("English VWF")
        || manifest.standalone_ascii_policy
            != "preserve_unmapped_standalone_ascii_escape_doors_owned_by_escape_doors_manifest"
        || manifest.preserved_complete_latin_names != ["Jemini", "RIU", "TAKIN"]
        || manifest.font_asset != "neodgm.ttf"
        || manifest.render_mode != "jp_native_16x16_no_horizontal_scaling"
        || manifest.space_width_px != SPACE_WIDTH_PIXELS
        || manifest.lines.len() != LINE_COUNT
    {
        return Err("native-frame credit manifest identity drifted".to_string());
    }
    validate_sha256(&manifest.font_sha256, "native-frame credit font")?;
    let renderer = &manifest.generic_renderer;
    if parse_hex(&renderer.window_offset)? != GENERIC_RENDERER_OFFSET
        || renderer.window_bytes != GENERIC_RENDERER_BYTES
        || parse_hex(&renderer.pointer_table_offset)? != POINTER_TABLE_OFFSET
        || renderer.pointer_entry_bytes != POINTER_ENTRY_BYTES
        || parse_u16_hex(&renderer.object_type_displacement)? != OBJECT_TYPE_DISPLACEMENT
        || parse_u16_hex(&renderer.frame_index_displacement)? != FRAME_INDEX_DISPLACEMENT
    {
        return Err("native-frame generic renderer declaration drifted".to_string());
    }
    validate_sha256(&renderer.window_sha256, "native-frame generic renderer")?;
    if parse_hex(&manifest.target_table_bank.offset)? != TARGET_TABLE_BANK_OFFSET
        || parse_hex(&manifest.target_table_bank.limit)? != TARGET_TABLE_BANK_LIMIT
        || parse_u8_hex(&manifest.target_table_bank.fill_byte)? != TARGET_TABLE_BANK_FILL
    {
        return Err("native-frame target table bank declaration drifted".to_string());
    }

    let expected = [
        (
            "GFX-CREDITS-P07-L01",
            7usize,
            "in_place_exact",
            "preserve_one_frame_4x2",
            2usize,
            1usize,
        ),
        (
            "GFX-CREDITS-P07-L05",
            7,
            "append",
            "relocate_widen_one_frame_4x2",
            1,
            1,
        ),
        (
            "GFX-CREDITS-P09-L07",
            9,
            "in_place_prefix",
            "relocate_one_cell_frame_sequence",
            9,
            9,
        ),
    ];
    for (line, (id, page, placement, profile, source_cells, frame_count)) in
        manifest.lines.iter().zip(expected)
    {
        if line.id != id
            || line.page != page
            || line.pattern_placement != placement
            || line.table_profile != profile
            || line.source_logical_tiles.len() != source_cells
            || line.frame_count != frame_count
            || line.source_table_bytes != line.frame_count * 10 + line.frame_count * 2
            || line.source_pattern_end_exclusive <= line.source_pattern_start
            || line.source_pattern_end_exclusive - line.source_pattern_start
                != line.source_logical_tiles.len() * GLYPH_TILES
            || line.jp.is_empty()
            || line.ko.is_empty()
            || is_single_ascii_letter(&line.jp)
            || is_single_ascii_letter(&line.ko)
            || line.selection_window_bytes == 0
            || line.render_segments.is_empty()
        {
            return Err(format!("{} identity or geometry drifted", line.id));
        }
        for hash in [
            &line.source_pattern_sha256,
            &line.type_pointer_sha256,
            &line.source_table_sha256,
            &line.selection_window_sha256,
        ] {
            validate_sha256(hash, &line.id)?;
        }
        let object_type = usize::from(parse_u8_hex(&line.object_type)?);
        if parse_hex(&line.type_pointer_offset)?
            != POINTER_TABLE_OFFSET + object_type * POINTER_ENTRY_BYTES
        {
            return Err(format!("{} object-type pointer slot drifted", line.id));
        }
        let window_start = parse_hex(&line.selection_window_offset)?;
        let window_end = window_start + line.selection_window_bytes;
        let object_instruction_offset = parse_hex(&line.object_type_instruction_offset)?;
        let object_instruction = object_type_instruction(line)?;
        if object_instruction_offset < window_start
            || object_instruction_offset + object_instruction.len() > window_end
        {
            return Err(format!(
                "{} object-type instruction escapes its selection window",
                line.id
            ));
        }
        match (
            &line.frame_index_instruction_offset,
            line.table_profile.as_str(),
        ) {
            (Some(offset), "relocate_one_cell_frame_sequence") => {
                let offset = parse_hex(offset)?;
                let instruction = frame_index_instruction();
                if offset < window_start || offset + instruction.len() > window_end {
                    return Err(format!(
                        "{} frame-index instruction escapes its selection window",
                        line.id
                    ));
                }
            }
            (None, "preserve_one_frame_4x2" | "relocate_widen_one_frame_4x2") => {}
            _ => {
                return Err(format!(
                    "{} frame-index instruction declaration drifted",
                    line.id
                ));
            }
        }

        let target_cells = line
            .render_segments
            .iter()
            .map(|segment| segment.target_cell_start + segment.target_cells)
            .max()
            .ok_or_else(|| format!("{} has no target cells", line.id))?;
        let mut covered = vec![false; target_cells];
        let mut rendered_text = String::new();
        for segment in &line.render_segments {
            if segment.ko.is_empty()
                || segment.target_cells == 0
                || segment.target_cell_start + segment.target_cells > target_cells
                || native_text_width(&segment.ko, manifest.space_width_px)
                    != segment.target_cells * CELL_WIDTH_PIXELS
            {
                return Err(format!("{} has an invalid render segment", line.id));
            }
            for cell in covered
                .iter_mut()
                .skip(segment.target_cell_start)
                .take(segment.target_cells)
            {
                if *cell {
                    return Err(format!("{} render segments overlap", line.id));
                }
                *cell = true;
            }
            rendered_text.push_str(&segment.ko);
        }
        if covered.contains(&false)
            || rendered_text.chars().collect::<String>()
                != line
                    .ko
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>()
        {
            return Err(format!("{} rendered text coverage drifted", line.id));
        }
        let referenced_cells = line
            .target_frame_cells
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        if referenced_cells != (0..target_cells).collect()
            || line
                .target_frame_cells
                .iter()
                .flatten()
                .any(|&cell| cell >= target_cells)
        {
            return Err(format!("{} target frame-cell mapping drifted", line.id));
        }
        match line.pattern_placement.as_str() {
            "in_place_exact"
                if target_cells * GLYPH_TILES
                    == line.source_pattern_end_exclusive - line.source_pattern_start => {}
            "in_place_prefix"
                if target_cells * GLYPH_TILES
                    < line.source_pattern_end_exclusive - line.source_pattern_start => {}
            "append" => {}
            _ => {
                return Err(format!("{} pattern placement capacity drifted", line.id));
            }
        }
        if profile == "relocate_one_cell_frame_sequence" {
            if line.target_frame_cells.len() != line.frame_count
                || !line.target_frame_cells.iter().any(Option::is_none)
            {
                return Err(format!("{} timed frame sequence drifted", line.id));
            }
        } else if line.target_frame_cells.iter().any(Option::is_none) {
            return Err(format!("{} unexpectedly declares a blank frame", line.id));
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
        &format!("{} type-table pointer", line.id),
    )?;
    if sha256_hex(pointer) != line.type_pointer_sha256 {
        return Err(format!("{label} {} type-pointer SHA-256 drifted", line.id));
    }
    let source_table_offset = parse_hex(&line.source_table_offset)?;
    validate_type_pointer(rom, line, source_table_offset, label)?;
    let source_table = source_range(
        rom,
        source_table_offset,
        line.source_table_bytes,
        &format!("{} source frame table", line.id),
    )?;
    if sha256_hex(source_table) != line.source_table_sha256 {
        return Err(format!("{label} {} source-table SHA-256 drifted", line.id));
    }
    validate_source_table(source_table, line, label)
}

fn validate_selection(rom: &[u8], line: &LineDeclaration, label: &str) -> Result<(), String> {
    let window_offset = parse_hex(&line.selection_window_offset)?;
    let window = source_range(
        rom,
        window_offset,
        line.selection_window_bytes,
        &format!("{} selection window", line.id),
    )?;
    if sha256_hex(window) != line.selection_window_sha256 {
        return Err(format!(
            "{label} {} selection-window SHA-256 drifted",
            line.id
        ));
    }
    let object_offset = parse_hex(&line.object_type_instruction_offset)?;
    let object_instruction = object_type_instruction(line)?;
    if source_range(
        rom,
        object_offset,
        object_instruction.len(),
        &format!("{} typed object-type instruction", line.id),
    )? != object_instruction
    {
        return Err(format!(
            "{label} {} typed object-type instruction drifted",
            line.id
        ));
    }
    if let Some(offset) = &line.frame_index_instruction_offset {
        let offset = parse_hex(offset)?;
        let instruction = frame_index_instruction();
        if source_range(
            rom,
            offset,
            instruction.len(),
            &format!("{} typed frame-index instruction", line.id),
        )? != instruction
        {
            return Err(format!(
                "{label} {} typed frame-index instruction drifted",
                line.id
            ));
        }
    }
    Ok(())
}

fn object_type_instruction(line: &LineDeclaration) -> Result<Vec<u8>, String> {
    m68k::assemble(&[Inst::MoveByteImmediateToDisplacementAddress {
        immediate: parse_u8_hex(&line.object_type)?,
        displacement: OBJECT_TYPE_DISPLACEMENT,
        destination: AddressReg::A1,
    }])
}

fn frame_index_instruction() -> Vec<u8> {
    m68k::assemble(&[Inst::MoveByteDataToDisplacementAddress {
        source: DataReg::D0,
        displacement: FRAME_INDEX_DISPLACEMENT,
        destination: AddressReg::A1,
    }])
    .expect("fixed native-frame selector is a valid typed instruction")
}

fn validate_type_pointer(
    rom: &[u8],
    line: &LineDeclaration,
    expected_table_offset: usize,
    label: &str,
) -> Result<(), String> {
    let expected = u32::try_from(expected_table_offset)
        .map_err(|_| format!("{} table offset exceeds 32 bits", line.id))?
        .to_be_bytes();
    if source_range(
        rom,
        parse_hex(&line.type_pointer_offset)?,
        POINTER_ENTRY_BYTES,
        &format!("{} object-type pointer", line.id),
    )? != expected
    {
        return Err(format!(
            "{label} {} object-type pointer does not select 0x{expected_table_offset:06X}",
            line.id
        ));
    }
    Ok(())
}

fn validate_source_table(table: &[u8], line: &LineDeclaration, label: &str) -> Result<(), String> {
    let offsets = parse_frame_offsets(table, line.frame_count, &line.id)?;
    match line.table_profile.as_str() {
        "preserve_one_frame_4x2" => {
            let frame = frame_slice(table, &offsets, 0, &line.id)?;
            validate_one_record_frame(
                frame,
                line.source_logical_tiles[0],
                0x0D00,
                0x0070,
                label,
                &line.id,
            )
        }
        "relocate_widen_one_frame_4x2" => {
            let frame = frame_slice(table, &offsets, 0, &line.id)?;
            validate_one_record_frame(
                frame,
                line.source_logical_tiles[0],
                0x0500,
                0x0078,
                label,
                &line.id,
            )
        }
        "relocate_one_cell_frame_sequence" => {
            for (frame_index, &tile) in line.source_logical_tiles.iter().enumerate() {
                let frame = frame_slice(table, &offsets, frame_index, &line.id)?;
                validate_one_record_frame(frame, tile, 0x0500, 0x0078, label, &line.id)?;
            }
            Ok(())
        }
        _ => unreachable!("table profile was validated"),
    }
}

fn build_target_table(
    source_table: &[u8],
    line: &LineDeclaration,
    first_target_tile: usize,
    target_logical_tiles: &[usize],
) -> Result<Option<Vec<u8>>, String> {
    let target = match line.table_profile.as_str() {
        "preserve_one_frame_4x2" => None,
        "relocate_widen_one_frame_4x2" => {
            let mut target = source_table.to_vec();
            let offsets = parse_frame_offsets(&target, 1, &line.id)?;
            let frame_offset = offsets[0];
            let record_offset = frame_offset + 2;
            let source_size =
                u16::from_be_bytes([target[record_offset + 2], target[record_offset + 3]]);
            target[record_offset + 2..record_offset + 4]
                .copy_from_slice(&((source_size & !0x0F00) | 0x0D00).to_be_bytes());
            replace_record_tile(
                &mut target[record_offset..record_offset + 8],
                first_target_tile,
            )?;
            target[record_offset + 6..record_offset + 8].copy_from_slice(&0x0070u16.to_be_bytes());
            Some(target)
        }
        "relocate_one_cell_frame_sequence" => {
            let mut target = source_table.to_vec();
            let offsets = parse_frame_offsets(&target, line.frame_count, &line.id)?;
            for (&frame_offset, &tile) in offsets.iter().zip(target_logical_tiles) {
                let record_offset = frame_offset + 2;
                replace_record_tile(&mut target[record_offset..record_offset + 8], tile)?;
            }
            Some(target)
        }
        _ => unreachable!("table profile was validated"),
    };
    if let Some(table) = &target {
        validate_target_table(
            source_table,
            table,
            line,
            first_target_tile,
            target_logical_tiles,
            "compiled",
        )?;
    }
    Ok(target)
}

fn validate_target_table(
    source_table: &[u8],
    target_table: &[u8],
    line: &LineDeclaration,
    first_target_tile: usize,
    target_logical_tiles: &[usize],
    label: &str,
) -> Result<(), String> {
    match line.table_profile.as_str() {
        "preserve_one_frame_4x2" => {
            if target_table != source_table {
                return Err(format!("{label} {} changed its preserved table", line.id));
            }
            let offsets = parse_frame_offsets(target_table, 1, &line.id)?;
            validate_one_record_frame(
                frame_slice(target_table, &offsets, 0, &line.id)?,
                first_target_tile,
                0x0D00,
                0x0070,
                label,
                &line.id,
            )
        }
        "relocate_widen_one_frame_4x2" => {
            let offsets = parse_frame_offsets(target_table, 1, &line.id)?;
            validate_one_record_frame(
                frame_slice(target_table, &offsets, 0, &line.id)?,
                first_target_tile,
                0x0D00,
                0x0070,
                label,
                &line.id,
            )
        }
        "relocate_one_cell_frame_sequence" => {
            if target_logical_tiles.len() != line.frame_count {
                return Err(format!("{label} {} target frame count drifted", line.id));
            }
            let offsets = parse_frame_offsets(target_table, line.frame_count, &line.id)?;
            for (frame_index, &tile) in target_logical_tiles.iter().enumerate() {
                validate_one_record_frame(
                    frame_slice(target_table, &offsets, frame_index, &line.id)?,
                    tile,
                    0x0500,
                    0x0078,
                    label,
                    &line.id,
                )?;
            }
            Ok(())
        }
        _ => unreachable!("table profile was validated"),
    }
}

fn validate_one_record_frame(
    frame: &[u8],
    expected_tile: usize,
    expected_size: u16,
    expected_x: u16,
    label: &str,
    id: &str,
) -> Result<(), String> {
    if frame.len() != 10 || u16::from_be_bytes([frame[0], frame[1]]) != 1 {
        return Err(format!("{label} {id} is not a one-record frame"));
    }
    let record = &frame[2..10];
    let size = u16::from_be_bytes([record[2], record[3]]);
    let tile = u16::from_be_bytes([record[4], record[5]]);
    if u16::from_be_bytes([record[0], record[1]]) != 0x0078
        || size & 0x0F00 != expected_size
        || usize::from(tile & TILE_INDEX_MASK) != expected_tile
        || u16::from_be_bytes([record[6], record[7]]) != expected_x
    {
        return Err(format!("{label} {id} one-record frame geometry drifted"));
    }
    Ok(())
}

fn replace_record_tile(record: &mut [u8], tile: usize) -> Result<(), String> {
    if record.len() != 8 {
        return Err("native-frame target record is not eight bytes".to_string());
    }
    let tile =
        u16::try_from(tile).map_err(|_| "native-frame target tile exceeds 16 bits".to_string())?;
    if tile & !TILE_INDEX_MASK != 0 {
        return Err("native-frame target tile exceeds the consumer mask".to_string());
    }
    let source = u16::from_be_bytes([record[4], record[5]]);
    record[4..6].copy_from_slice(&((source & !TILE_INDEX_MASK) | tile).to_be_bytes());
    Ok(())
}

fn parse_frame_offsets(
    table: &[u8],
    frame_count: usize,
    label: &str,
) -> Result<Vec<usize>, String> {
    let header_bytes = frame_count
        .checked_mul(2)
        .ok_or_else(|| format!("{label} frame-offset table overflowed"))?;
    let header = source_range(table, 0, header_bytes, "native-frame offset table")?;
    let offsets = header
        .chunks_exact(2)
        .map(|pair| usize::from(u16::from_be_bytes([pair[0], pair[1]])))
        .collect::<Vec<_>>();
    if offsets.first().copied() != Some(header_bytes)
        || offsets.iter().any(|&offset| offset < header_bytes)
        || offsets.windows(2).any(|pair| pair[0] >= pair[1])
        || offsets.last().is_none_or(|&offset| offset >= table.len())
    {
        return Err(format!("{label} frame offsets are not a strict table"));
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

fn render_target_patterns(
    font: &Font,
    manifest: &CreditsNativeFramesManifest,
    line: &LineDeclaration,
) -> Result<Vec<u8>, String> {
    let target_cells = line
        .render_segments
        .iter()
        .map(|segment| segment.target_cell_start + segment.target_cells)
        .max()
        .ok_or_else(|| format!("{} has no target cells", line.id))?;
    let mut output = vec![0u8; target_cells * GLYPH_TILES * MD_TILE_BYTES];
    for segment in &line.render_segments {
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
            &format!("{} Korean native-frame segment", line.id),
        )?;
        let start = segment.target_cell_start * GLYPH_TILES * MD_TILE_BYTES;
        output[start..start + encoded.len()].copy_from_slice(&encoded);
    }
    let roles = output
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 15]) {
        return Err(format!(
            "{} Korean native-frame patterns use unexpected palette roles {roles:?}",
            line.id
        ));
    }
    Ok(output)
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

fn validate_blank_cell(page: &[u8], id: &str) -> Result<(), String> {
    let start = BLANK_TILE * MD_TILE_BYTES;
    let bytes = source_range(
        page,
        start,
        GLYPH_TILES * MD_TILE_BYTES,
        &format!("{id} target blank cell"),
    )?;
    if bytes.iter().any(|&byte| byte != 0) {
        return Err(format!("{id} target blank cell is not transparent"));
    }
    Ok(())
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
