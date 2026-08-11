//! JP-source compiler for fixed 16x16 ending-credit name cells.
//!
//! These names already use the original credit-page pattern transfer and the
//! original timed sprite renderer. Korean text is compiled into declared
//! four-tile glyph cells. Lines that fit keep their source map; wider lines
//! append native cells, relocate only their map, and replace the checked count
//! and map-pointer instructions through the typed 68000 ISA.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::Font;
use serde::Deserialize;

use crate::m68k::{self, AddressReg, DataReg, Inst};

use super::font_effect::{
    CREDIT_PREVIEW_INK, native_text_width, read_verified_font, render_native_text_line,
};
use super::pixel::{decode_md_tiles_column_major, encode_md_tiles_column_major, write_rgba_png};
use super::{MD_TILE_BYTES, parse_hex, sha256_hex, source_range, validate_only_ranges_changed};

const LINE_COUNT: usize = 7;
const PAGE_COUNT: usize = 10;
const CELL_WIDTH_PIXELS: usize = 16;
const CELL_HEIGHT_PIXELS: usize = 16;
const SPACE_WIDTH_PIXELS: usize = 8;
const GLYPH_TILES: usize = 4;
const DEFINITION_COUNT_BYTES: usize = 2;
const DEFINITION_RECORD_BYTES: usize = 8;
const DEFINITION_BYTES: usize = DEFINITION_COUNT_BYTES + DEFINITION_RECORD_BYTES;
const DRAW_TARGET: u32 = 0x0000_1856;
const TILE_INDEX_MASK: u16 = 0x07FF;
const BLANK_TILE: usize = 5;
const MAP_BANK_OFFSET: usize = 0x2F_8000;
const MAP_BANK_LIMIT: usize = 0x30_0000;
const MAP_BANK_FILL: u8 = 0xFF;
const PREVIEW_SCALE: usize = 3;
const PREVIEW_COLUMN_GAP: usize = 16;
const PREVIEW_ROW_GAP: usize = 8;

#[derive(Debug, Deserialize)]
struct CreditsTimedManifest {
    schema_version: u32,
    asset_group_id: String,
    source_policy: String,
    standalone_ascii_policy: String,
    preserved_complete_latin_names: Vec<String>,
    font_asset: String,
    font_sha256: String,
    render_mode: String,
    space_width_px: usize,
    consumer: ConsumerDeclaration,
    target_map_bank: TargetMapBankDeclaration,
    lines: Vec<LineDeclaration>,
}

#[derive(Debug, Deserialize)]
struct ConsumerDeclaration {
    draw_target: String,
    definition_count: usize,
    definition_record_bytes: usize,
    tile_index_mask: String,
    glyph_tiles: usize,
    blank_tile: usize,
}

#[derive(Debug, Deserialize)]
struct TargetMapBankDeclaration {
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
    #[serde(default)]
    ascii_classification: Option<String>,
    placement: String,
    source_map_order: String,
    source_logical_tiles: Vec<usize>,
    source_map_offset: String,
    source_map_bytes: usize,
    source_map_sha256: String,
    map_pointer_offset: String,
    draw_call_offset: String,
    consumer_window_offset: String,
    consumer_window_bytes: usize,
    consumer_window_sha256: String,
    #[serde(default)]
    target_map_profile: Option<String>,
    #[serde(default)]
    target_first_x: Option<String>,
    #[serde(default)]
    target_x_step: Option<usize>,
    #[serde(default)]
    count_patch: Option<CountPatchDeclaration>,
    #[serde(default)]
    draw_count_patches: Vec<CountPatchDeclaration>,
    #[serde(default)]
    protected_trailing_definition: Option<ProtectedTrailingDefinitionDeclaration>,
    segments: Vec<SegmentDeclaration>,
}

#[derive(Debug, Deserialize)]
struct CountPatchDeclaration {
    offset: String,
    profile: String,
    #[serde(default)]
    displacement: Option<String>,
    source_count: u16,
    target_count: u16,
}

#[derive(Debug, Deserialize)]
struct ProtectedTrailingDefinitionDeclaration {
    text: String,
    source_record_sha256: String,
    source_tile_start: usize,
    source_tile_end_exclusive: usize,
    source_pattern_sha256: String,
    width_tiles: usize,
    height_tiles: usize,
    y_offset_px: usize,
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
struct LineBuild {
    preview: CreditLinePreview,
    target_logical_tiles: Vec<usize>,
    target_map_offset: Option<usize>,
    target_map: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub(super) struct CreditLinePreview {
    pub(super) source_surface: Vec<u8>,
    pub(super) target_surface: Vec<u8>,
    pub(super) source_width: usize,
    pub(super) target_width: usize,
}

#[derive(Debug)]
pub(super) struct InstructionPatch {
    pub(super) offset: usize,
    pub(super) source: Vec<u8>,
    pub(super) target: Vec<u8>,
    pub(super) label: String,
}

#[derive(Debug)]
pub(super) struct CreditsTimedBuild {
    manifest: CreditsTimedManifest,
    lines: Vec<LineBuild>,
    glyphs: BTreeSet<char>,
    rewritten_tiles: usize,
    overwritten_source_tiles: usize,
    appended_tiles: usize,
    relocated_lines: usize,
    verified_consumer_bytes: usize,
    map_bank: Vec<u8>,
    instruction_patches: Vec<InstructionPatch>,
}

impl CreditsTimedBuild {
    pub(super) fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub(super) fn glyphs(&self) -> &BTreeSet<char> {
        &self.glyphs
    }

    pub(super) fn rewritten_tiles(&self) -> usize {
        self.rewritten_tiles
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

    pub(super) fn map_bank_offset(&self) -> usize {
        MAP_BANK_OFFSET
    }

    pub(super) fn map_bank(&self) -> &[u8] {
        &self.map_bank
    }

    pub(super) fn instruction_patches(&self) -> &[InstructionPatch] {
        &self.instruction_patches
    }

    pub(super) fn written_executable_bytes(&self) -> usize {
        self.instruction_patches
            .iter()
            .map(|patch| patch.target.len())
            .sum()
    }

    pub(super) fn validate_output_consumers(
        &self,
        source_rom: &[u8],
        output: &[u8],
        label: &str,
    ) -> Result<(), String> {
        for line in &self.manifest.lines {
            let source_map_offset = parse_hex(&line.source_map_offset)?;
            let source_map = source_range(
                source_rom,
                source_map_offset,
                line.source_map_bytes,
                &format!("{} JP source map", line.id),
            )?;
            let output_source_map = source_range(
                output,
                source_map_offset,
                line.source_map_bytes,
                &format!("{} protected source map", line.id),
            )?;
            if output_source_map != source_map {
                return Err(format!(
                    "{label} {} changed its protected JP sprite map",
                    line.id
                ));
            }

            let consumer_offset = parse_hex(&line.consumer_window_offset)?;
            let source_consumer = source_range(
                source_rom,
                consumer_offset,
                line.consumer_window_bytes,
                &format!("{} JP consumer window", line.id),
            )?;
            let mut expected_consumer = source_consumer.to_vec();
            for patch in self.instruction_patches.iter().filter(|patch| {
                patch.offset >= consumer_offset
                    && patch.offset + patch.target.len()
                        <= consumer_offset + line.consumer_window_bytes
            }) {
                let relative = patch.offset - consumer_offset;
                if expected_consumer[relative..relative + patch.source.len()] != patch.source {
                    return Err(format!(
                        "{} typed source patch does not match its JP consumer window",
                        patch.label
                    ));
                }
                expected_consumer[relative..relative + patch.target.len()]
                    .copy_from_slice(&patch.target);
            }
            let output_consumer = source_range(
                output,
                consumer_offset,
                line.consumer_window_bytes,
                &format!("{} output consumer window", line.id),
            )?;
            if output_consumer != expected_consumer {
                return Err(format!(
                    "{label} {} consumer differs outside its typed count/map-pointer edits",
                    line.id
                ));
            }

            let draw = m68k::assemble(&[Inst::JsrAbsoluteLong(DRAW_TARGET)])?;
            let draw_offset = parse_hex(&line.draw_call_offset)?;
            if source_range(output, draw_offset, draw.len(), "timed-credit output draw")? != draw {
                return Err(format!(
                    "{label} {} typed sprite-draw call drifted",
                    line.id
                ));
            }
        }

        for (declaration, built) in self.manifest.lines.iter().zip(&self.lines) {
            if let (Some(offset), Some(expected)) =
                (built.target_map_offset, built.target_map.as_deref())
            {
                let actual = source_range(
                    output,
                    offset,
                    expected.len(),
                    &format!("{} relocated output map", declaration.id),
                )?;
                if actual != expected {
                    return Err(format!(
                        "{label} {} relocated sprite map drifted",
                        declaration.id
                    ));
                }
                let expected_record_tiles = match declaration.target_map_profile.as_deref() {
                    Some("static_centered_right_to_left") => {
                        built.target_logical_tiles.iter().rev().copied().collect()
                    }
                    Some("consumer_positioned_left_to_right") => built.target_logical_tiles.clone(),
                    _ => {
                        return Err(format!(
                            "{} has no valid relocated-map profile",
                            declaration.id
                        ));
                    }
                };
                validate_declared_map(expected, &expected_record_tiles, declaration, label)?;
            }
        }
        Ok(())
    }

    pub(super) fn write_preview(
        &self,
        additional_lines: &[CreditLinePreview],
        output_path: &Path,
    ) -> Result<(), String> {
        let lines = self
            .lines
            .iter()
            .map(|line| &line.preview)
            .chain(additional_lines.iter())
            .collect::<Vec<_>>();
        let source_max_width = lines
            .iter()
            .map(|line| line.source_width)
            .max()
            .ok_or_else(|| "credit-name preview has no lines".to_string())?;
        let target_max_width = lines
            .iter()
            .map(|line| line.target_width)
            .max()
            .ok_or_else(|| "credit-name preview has no lines".to_string())?;
        let logical_width = source_max_width + PREVIEW_COLUMN_GAP + target_max_width;
        let logical_height = lines.len() * CELL_HEIGHT_PIXELS + (lines.len() - 1) * PREVIEW_ROW_GAP;
        let width = logical_width * PREVIEW_SCALE;
        let height = logical_height * PREVIEW_SCALE;
        let mut rgba = vec![0u8; width * height * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[30, 15, 18, 255]);
        }

        for (row, line) in lines.iter().enumerate() {
            let y = row * (CELL_HEIGHT_PIXELS + PREVIEW_ROW_GAP);
            let left_x = source_max_width - line.source_width;
            let right_x = source_max_width + PREVIEW_COLUMN_GAP;
            draw_surface(
                &mut rgba,
                width,
                left_x,
                y,
                line.source_width,
                &line.source_surface,
                CREDIT_PREVIEW_INK,
            )?;
            draw_surface(
                &mut rgba,
                width,
                right_x,
                y,
                line.target_width,
                &line.target_surface,
                CREDIT_PREVIEW_INK,
            )?;
        }
        write_rgba_png(
            output_path,
            width as u32,
            height as u32,
            &rgba,
            "timed credit-name static preview",
        )
    }
}

pub(super) fn compile_credits_timed(
    source_rom: &[u8],
    source_pages: &[Vec<u8>],
    target_pages: &mut [Vec<u8>],
    assets_dir: &Path,
) -> Result<CreditsTimedBuild, String> {
    if source_pages.len() != PAGE_COUNT || target_pages.len() != PAGE_COUNT {
        return Err("credit-name compiler requires ten source and target pages".to_string());
    }
    let manifest = read_manifest(assets_dir)?;
    validate_manifest_shape(&manifest)?;
    let font = read_verified_font(
        assets_dir,
        &manifest.font_asset,
        &manifest.font_sha256,
        "timed credit name",
    )?;

    for line in &manifest.lines {
        validate_consumer(source_rom, line, "JP source")?;
    }

    let baseline = target_pages.to_vec();
    let mut changed_ranges = vec![Vec::new(); PAGE_COUNT];
    let mut glyphs = BTreeSet::new();
    let mut overwritten_source_tiles = 0usize;
    let mut appended_tiles = 0usize;
    let mut relocated_lines = 0usize;
    let mut map_bank = Vec::new();
    let mut instruction_patches = Vec::new();
    let mut lines = Vec::with_capacity(manifest.lines.len());
    for line in &manifest.lines {
        let source_page = source_pages
            .get(line.page)
            .ok_or_else(|| format!("{} references absent credit page {}", line.id, line.page))?;
        let target_page = target_pages
            .get_mut(line.page)
            .ok_or_else(|| format!("{} references absent target page {}", line.id, line.page))?;
        if target_page.len() < source_page.len()
            || target_page[..source_page.len()].len() != source_page.len()
        {
            return Err(format!(
                "{} target credit page is shorter than its JP source",
                line.id
            ));
        }

        let source_name_surface = extract_line_surface(
            source_page,
            &line.source_logical_tiles,
            &format!("{} JP", line.id),
        )?;
        let (source_surface, source_width) = append_protected_preview(
            source_page,
            source_name_surface,
            line.source_logical_tiles.len() * CELL_WIDTH_PIXELS,
            line,
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
        if let Some(protected) = &line.protected_trailing_definition {
            let start = protected.source_tile_start * MD_TILE_BYTES;
            let end = protected.source_tile_end_exclusive * MD_TILE_BYTES;
            let source_bytes = source_range(
                source_page,
                start,
                end - start,
                &format!("{} protected {:?} patterns", line.id, protected.text),
            )?;
            if sha256_hex(source_bytes) != protected.source_pattern_sha256 {
                return Err(format!(
                    "{} protected {:?} pattern SHA-256 drifted",
                    line.id, protected.text
                ));
            }
        }

        let (target_logical_tiles, target_map_offset, target_map) = match line.placement.as_str() {
            "in_place" => {
                for segment in &line.segments {
                    let start = segment.source_tile_start * MD_TILE_BYTES;
                    let end = segment.source_tile_end_exclusive * MD_TILE_BYTES;
                    let source_bytes = &source_page[start..end];
                    if target_page[start..end] != *source_bytes {
                        return Err(format!(
                            "{} target segment overlaps an earlier credit mutation",
                            line.id
                        ));
                    }
                    let encoded = render_segment(&font, &manifest, line, segment)?;
                    if encoded.len() != end - start {
                        return Err(format!(
                            "{} segment {:?} encoded to {} bytes instead of {}",
                            line.id,
                            segment.ko,
                            encoded.len(),
                            end - start
                        ));
                    }
                    target_page[start..end].copy_from_slice(&encoded);
                    changed_ranges[line.page].push((start, end));
                    overwritten_source_tiles +=
                        segment.source_tile_end_exclusive - segment.source_tile_start;
                }
                (line.source_logical_tiles.clone(), None, None)
            }
            "relocated" => {
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

                let source_map_offset = parse_hex(&line.source_map_offset)?;
                let source_map = source_range(
                    source_rom,
                    source_map_offset,
                    line.source_map_bytes,
                    &format!("{} JP source map", line.id),
                )?;
                let relocated_map = build_target_map(line, source_map, &target_tiles)?;
                let relocated_offset = MAP_BANK_OFFSET
                    .checked_add(map_bank.len())
                    .ok_or_else(|| "timed-credit map allocation overflowed".to_string())?;
                if relocated_offset + relocated_map.len() > MAP_BANK_LIMIT {
                    return Err(format!(
                        "{} relocated map exceeds its declared target bank",
                        line.id
                    ));
                }
                map_bank.extend_from_slice(&relocated_map);
                instruction_patches.extend(build_relocation_patches(line, relocated_offset)?);
                relocated_lines += 1;
                (target_tiles, Some(relocated_offset), Some(relocated_map))
            }
            other => {
                return Err(format!(
                    "{} has unsupported placement profile {other:?}",
                    line.id
                ));
            }
        };

        let target_name_surface = extract_line_surface(
            target_page,
            &target_logical_tiles,
            &format!("{} Korean", line.id),
        )?;
        let (target_surface, target_width) = append_protected_preview(
            source_page,
            target_name_surface,
            target_logical_tiles.len() * CELL_WIDTH_PIXELS,
            line,
            &format!("{} Korean", line.id),
        )?;
        lines.push(LineBuild {
            preview: CreditLinePreview {
                source_surface,
                target_surface,
                source_width,
                target_width,
            },
            target_logical_tiles,
            target_map_offset,
            target_map,
        });
    }

    for page in 0..PAGE_COUNT {
        let source_len = baseline[page].len();
        validate_only_ranges_changed(
            &baseline[page],
            &target_pages[page][..source_len],
            &changed_ranges[page],
        )
        .map_err(|error| format!("credit-name page {page} ownership failed: {error}"))?;
        if !target_pages[page][source_len..]
            .len()
            .is_multiple_of(MD_TILE_BYTES)
        {
            return Err(format!(
                "credit-name page {page} appended suffix is not tile-aligned"
            ));
        }
    }

    instruction_patches.sort_by_key(|patch| patch.offset);
    for pair in instruction_patches.windows(2) {
        if pair[0].offset + pair[0].target.len() > pair[1].offset {
            return Err("timed-credit typed instruction patches overlap".to_string());
        }
    }

    let rewritten_tiles = overwritten_source_tiles + appended_tiles;
    Ok(CreditsTimedBuild {
        verified_consumer_bytes: manifest
            .lines
            .iter()
            .map(|line| line.consumer_window_bytes)
            .sum(),
        manifest,
        lines,
        glyphs,
        rewritten_tiles,
        overwritten_source_tiles,
        appended_tiles,
        relocated_lines,
        map_bank,
        instruction_patches,
    })
}

fn read_manifest(assets_dir: &Path) -> Result<CreditsTimedManifest, String> {
    let path = assets_dir.join("graphics_text/credits_timed_cells.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read timed credit-name source {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "invalid timed credit-name source {}: {error}",
            path.display()
        )
    })
}

fn validate_manifest_shape(manifest: &CreditsTimedManifest) -> Result<(), String> {
    if manifest.schema_version != 3
        || manifest.asset_group_id != "GFX-CREDITS-TIMED-CELLS"
        || !manifest.source_policy.contains("English VWF")
        || manifest.standalone_ascii_policy
            != "preserve_unmapped_standalone_ascii_escape_doors_owned_by_escape_doors_manifest"
        || manifest.preserved_complete_latin_names != ["Jemini", "RIU", "TAKIN"]
        || manifest.font_asset != "neodgm.ttf"
        || manifest.font_sha256.len() != 64
        || manifest.render_mode != "jp_native_16x16_no_horizontal_scaling"
        || manifest.space_width_px != SPACE_WIDTH_PIXELS
        || manifest.lines.len() != LINE_COUNT
    {
        return Err("timed credit-name manifest identity drifted".to_string());
    }
    if parse_u32_hex(&manifest.consumer.draw_target)? != DRAW_TARGET
        || manifest.consumer.definition_count != 1
        || manifest.consumer.definition_record_bytes != DEFINITION_RECORD_BYTES
        || parse_u16_hex(&manifest.consumer.tile_index_mask)? != TILE_INDEX_MASK
        || manifest.consumer.glyph_tiles != GLYPH_TILES
        || manifest.consumer.blank_tile != BLANK_TILE
    {
        return Err("timed credit-name consumer declaration drifted".to_string());
    }
    if parse_hex(&manifest.target_map_bank.offset)? != MAP_BANK_OFFSET
        || parse_hex(&manifest.target_map_bank.limit)? != MAP_BANK_LIMIT
        || parse_u8_hex(&manifest.target_map_bank.fill_byte)? != MAP_BANK_FILL
    {
        return Err("timed credit-name target map bank drifted".to_string());
    }

    let expected = [
        ("GFX-CREDITS-P01-L01", 1usize, 8usize, "in_place"),
        ("GFX-CREDITS-P02-L01", 2, 5, "relocated"),
        ("GFX-CREDITS-P02-L02", 2, 6, "in_place"),
        ("GFX-CREDITS-P03-L01", 3, 5, "relocated"),
        ("GFX-CREDITS-P03-L02", 3, 5, "relocated"),
        ("GFX-CREDITS-P05-L01", 5, 4, "relocated"),
        ("GFX-CREDITS-P05-L02", 5, 9, "in_place"),
    ];
    for (line, (expected_id, expected_page, expected_cells, expected_placement)) in
        manifest.lines.iter().zip(expected)
    {
        if line.id != expected_id
            || line.page != expected_page
            || line.placement != expected_placement
            || line.jp.is_empty()
            || line.ko.is_empty()
            || is_single_ascii_letter(&line.jp)
            || is_single_ascii_letter(&line.ko)
            || line.source_logical_tiles.len() != expected_cells
            || line.source_map_bytes
                != (line.source_logical_tiles.len()
                    + usize::from(line.protected_trailing_definition.is_some()))
                    * DEFINITION_BYTES
            || line.consumer_window_bytes == 0
            || line.segments.is_empty()
            || !matches!(
                line.source_map_order.as_str(),
                "left_to_right" | "right_to_left"
            )
        {
            return Err(format!(
                "{} identity, geometry or standalone-letter policy drifted",
                line.id
            ));
        }
        match line.id.as_str() {
            "GFX-CREDITS-P02-L02" | "GFX-CREDITS-P05-L01" => {
                if line.ascii_classification.as_deref() != Some("embedded_mixed_name_token") {
                    return Err(format!("{} mixed ASCII classification drifted", line.id));
                }
            }
            "GFX-CREDITS-P03-L01" => {
                if line.ascii_classification.as_deref() != Some("protected_embedded_latin_suffix") {
                    return Err(format!(
                        "{} protected Latin-suffix classification drifted",
                        line.id
                    ));
                }
            }
            _ if line.ascii_classification.is_some() => {
                return Err(format!(
                    "{} unexpectedly declares an ASCII classification",
                    line.id
                ));
            }
            _ => {}
        }
        if let Some(protected) = &line.protected_trailing_definition {
            if line.id != "GFX-CREDITS-P03-L01"
                || protected.text != "ss"
                || protected.source_tile_start != 61
                || protected.source_tile_end_exclusive != 63
                || protected.width_tiles != 2
                || protected.height_tiles != 1
                || protected.y_offset_px != 8
                || protected.source_tile_end_exclusive - protected.source_tile_start
                    != protected.width_tiles * protected.height_tiles
            {
                return Err(format!(
                    "{} protected trailing sprite declaration drifted",
                    line.id
                ));
            }
            validate_sha256(&protected.source_record_sha256, &line.id)?;
            validate_sha256(&protected.source_pattern_sha256, &line.id)?;
        } else if line.id == "GFX-CREDITS-P03-L01" {
            return Err(format!("{} protected ss sprite is absent", line.id));
        }
        for hash in [
            &line.source_map_sha256,
            &line.consumer_window_sha256,
            &manifest.font_sha256,
        ] {
            validate_sha256(hash, &line.id)?;
        }
        let mut declared_tiles = Vec::new();
        let mut previous_end = None;
        for segment in &line.segments {
            if segment.jp.is_empty()
                || segment.ko.is_empty()
                || is_single_ascii_letter(&segment.jp)
                || is_single_ascii_letter(&segment.ko)
                || segment.source_tile_end_exclusive <= segment.source_tile_start
                || !(segment.source_tile_end_exclusive - segment.source_tile_start)
                    .is_multiple_of(GLYPH_TILES)
                || segment.target_cells == 0
                || previous_end.is_some_and(|end| segment.source_tile_start < end)
            {
                return Err(format!("{} has an invalid pattern segment", line.id));
            }
            if native_text_width(&segment.ko, manifest.space_width_px)
                > segment.target_cells * CELL_WIDTH_PIXELS
            {
                return Err(format!(
                    "{} Korean segment {:?} exceeds its {}-pixel native-cell budget",
                    line.id,
                    segment.ko,
                    segment.target_cells * CELL_WIDTH_PIXELS
                ));
            }
            validate_sha256(&segment.source_sha256, &line.id)?;
            declared_tiles.extend(
                (segment.source_tile_start..segment.source_tile_end_exclusive).step_by(GLYPH_TILES),
            );
            previous_end = Some(segment.source_tile_end_exclusive);
        }
        let mapped_tiles = line
            .source_logical_tiles
            .iter()
            .copied()
            .filter(|&tile| tile != BLANK_TILE)
            .collect::<Vec<_>>();
        if mapped_tiles != declared_tiles {
            return Err(format!(
                "{} map tiles do not exactly cover its mutable segments",
                line.id
            ));
        }

        let target_cells = line
            .segments
            .iter()
            .map(|segment| segment.target_cells)
            .sum::<usize>()
            + line.segments.len().saturating_sub(1);
        let protected_definitions = usize::from(line.protected_trailing_definition.is_some());
        match line.placement.as_str() {
            "in_place" => {
                if target_cells != line.source_logical_tiles.len()
                    || line.target_map_profile.is_some()
                    || line.target_first_x.is_some()
                    || line.target_x_step.is_some()
                    || line.count_patch.is_some()
                    || !line.draw_count_patches.is_empty()
                {
                    return Err(format!(
                        "{} in-place target geometry or patch policy drifted",
                        line.id
                    ));
                }
                for segment in &line.segments {
                    let source_cells = (segment.source_tile_end_exclusive
                        - segment.source_tile_start)
                        / GLYPH_TILES;
                    if source_cells != segment.target_cells {
                        return Err(format!(
                            "{} in-place segment changes its native cell count",
                            line.id
                        ));
                    }
                }
            }
            "relocated" => {
                let count = line.count_patch.as_ref().ok_or_else(|| {
                    format!("{} relocated line lacks its typed count patch", line.id)
                })?;
                if usize::from(count.source_count)
                    != line.source_logical_tiles.len() + protected_definitions
                    || usize::from(count.target_count) != target_cells + protected_definitions
                {
                    return Err(format!("{} relocated count geometry drifted", line.id));
                }
                let count_offset = parse_hex(&count.offset)?;
                let consumer_start = parse_hex(&line.consumer_window_offset)?;
                let count_bytes =
                    assemble_count_instruction(count, count.source_count, &line.id)?.len();
                if count_offset < consumer_start
                    || count_offset + count_bytes > consumer_start + line.consumer_window_bytes
                {
                    return Err(format!(
                        "{} typed count patch escapes its consumer window",
                        line.id
                    ));
                }
                for draw_count in &line.draw_count_patches {
                    if draw_count.source_count != count.source_count
                        || draw_count.target_count != count.target_count
                    {
                        return Err(format!(
                            "{} draw-count geometry disagrees with its primary count patch",
                            line.id
                        ));
                    }
                    let offset = parse_hex(&draw_count.offset)?;
                    let bytes =
                        assemble_count_instruction(draw_count, draw_count.source_count, &line.id)?
                            .len();
                    if offset < consumer_start
                        || offset + bytes > consumer_start + line.consumer_window_bytes
                    {
                        return Err(format!(
                            "{} typed draw-count patch escapes its consumer window",
                            line.id
                        ));
                    }
                }
                match line.target_map_profile.as_deref() {
                    Some("static_centered_right_to_left") => {
                        if line.source_map_order != "right_to_left"
                            || line
                                .target_first_x
                                .as_deref()
                                .map(parse_u16_hex)
                                .transpose()?
                                .is_none()
                            || line.target_x_step.is_none_or(|step| step == 0)
                        {
                            return Err(format!(
                                "{} static relocated-map profile drifted",
                                line.id
                            ));
                        }
                    }
                    Some("consumer_positioned_left_to_right") => {
                        if line.source_map_order != "left_to_right"
                            || line.target_first_x.is_some()
                            || line.target_x_step.is_some()
                        {
                            return Err(format!(
                                "{} consumer-positioned map profile drifted",
                                line.id
                            ));
                        }
                    }
                    _ => {
                        return Err(format!("{} relocated-map profile is unsupported", line.id));
                    }
                }
            }
            _ => unreachable!("placement identity was checked above"),
        }
    }
    Ok(())
}

fn validate_consumer(rom: &[u8], line: &LineDeclaration, label: &str) -> Result<(), String> {
    let map_offset = parse_hex(&line.source_map_offset)?;
    let map = source_range(
        rom,
        map_offset,
        line.source_map_bytes,
        &format!("{} sprite definitions", line.id),
    )?;
    if sha256_hex(map) != line.source_map_sha256 {
        return Err(format!(
            "{label} {} sprite-definition SHA-256 drifted",
            line.id
        ));
    }
    let expected_tiles = map_record_tiles(line)?;
    validate_declared_map(map, &expected_tiles, line, label)?;

    let consumer_offset = parse_hex(&line.consumer_window_offset)?;
    let consumer = source_range(
        rom,
        consumer_offset,
        line.consumer_window_bytes,
        &format!("{} consumer window", line.id),
    )?;
    if sha256_hex(consumer) != line.consumer_window_sha256 {
        return Err(format!("{label} {} consumer window drifted", line.id));
    }
    let map_address = u32::try_from(map_offset)
        .map_err(|_| format!("{} map address does not fit in 32 bits", line.id))?;
    let typed_map_pointer = m68k::assemble(&[Inst::LeaAbsoluteLong {
        address: map_address,
        destination: AddressReg::A2,
    }])?;
    let map_pointer_offset = parse_hex(&line.map_pointer_offset)?;
    let actual_map_pointer = source_range(
        rom,
        map_pointer_offset,
        typed_map_pointer.len(),
        &format!("{} typed map pointer", line.id),
    )?;
    if actual_map_pointer != typed_map_pointer {
        return Err(format!(
            "{label} {} typed map-pointer instruction drifted",
            line.id
        ));
    }
    let typed_draw_call = m68k::assemble(&[Inst::JsrAbsoluteLong(DRAW_TARGET)])?;
    let draw_call_offset = parse_hex(&line.draw_call_offset)?;
    let actual_draw_call = source_range(
        rom,
        draw_call_offset,
        typed_draw_call.len(),
        &format!("{} typed sprite draw call", line.id),
    )?;
    if actual_draw_call != typed_draw_call {
        return Err(format!(
            "{label} {} typed sprite-draw call drifted",
            line.id
        ));
    }
    if map_pointer_offset < consumer_offset
        || map_pointer_offset + typed_map_pointer.len()
            > consumer_offset + line.consumer_window_bytes
        || draw_call_offset < consumer_offset
        || draw_call_offset + typed_draw_call.len() > consumer_offset + line.consumer_window_bytes
    {
        return Err(format!(
            "{} typed bindings escape the declared consumer window",
            line.id
        ));
    }

    if let Some(count) = &line.count_patch {
        let typed_count = assemble_count_instruction(count, count.source_count, &line.id)?;
        let count_offset = parse_hex(&count.offset)?;
        if source_range(
            rom,
            count_offset,
            typed_count.len(),
            &format!("{} typed source count", line.id),
        )? != typed_count
        {
            return Err(format!(
                "{label} {} typed count instruction drifted",
                line.id
            ));
        }
    }
    for draw_count in &line.draw_count_patches {
        let typed_count =
            assemble_count_instruction(draw_count, draw_count.source_count, &line.id)?;
        let count_offset = parse_hex(&draw_count.offset)?;
        if source_range(
            rom,
            count_offset,
            typed_count.len(),
            &format!("{} typed source draw count", line.id),
        )? != typed_count
        {
            return Err(format!(
                "{label} {} typed draw-count instruction drifted",
                line.id
            ));
        }
    }
    Ok(())
}

fn validate_declared_map(
    map: &[u8],
    expected_tiles: &[usize],
    line: &LineDeclaration,
    label: &str,
) -> Result<(), String> {
    let visible_bytes = expected_tiles.len() * DEFINITION_BYTES;
    let expected_total = visible_bytes
        + usize::from(line.protected_trailing_definition.is_some()) * DEFINITION_BYTES;
    if map.len() != expected_total {
        return Err(format!("{label} {} map length drifted", line.id));
    }
    validate_map_records(&map[..visible_bytes], expected_tiles, line, label)?;
    if let Some(protected) = &line.protected_trailing_definition {
        let record = &map[visible_bytes..];
        if sha256_hex(record) != protected.source_record_sha256 {
            return Err(format!(
                "{label} {} protected trailing {:?} sprite record drifted",
                line.id, protected.text
            ));
        }
    }
    Ok(())
}

fn validate_map_records(
    map: &[u8],
    expected_tiles: &[usize],
    line: &LineDeclaration,
    label: &str,
) -> Result<(), String> {
    if map.len() != expected_tiles.len() * DEFINITION_BYTES {
        return Err(format!("{label} {} map length drifted", line.id));
    }
    for (index, (definition, expected_tile)) in map
        .chunks_exact(DEFINITION_BYTES)
        .zip(expected_tiles)
        .enumerate()
    {
        let count = u16::from_be_bytes([definition[0], definition[1]]) as usize;
        let y = u16::from_be_bytes([definition[2], definition[3]]);
        let size_link = u16::from_be_bytes([definition[4], definition[5]]);
        let tile_word = u16::from_be_bytes([definition[6], definition[7]]);
        if count != 1
            || y != 0x0078
            || size_link & 0x0F00 != 0x0500
            || tile_word & !TILE_INDEX_MASK != 0
            || usize::from(tile_word & TILE_INDEX_MASK) != *expected_tile
        {
            return Err(format!(
                "{label} {} definition {index} is not its declared one-cell 2x2 sprite",
                line.id
            ));
        }
    }
    Ok(())
}

fn map_record_tiles(line: &LineDeclaration) -> Result<Vec<usize>, String> {
    match line.source_map_order.as_str() {
        "left_to_right" => Ok(line.source_logical_tiles.clone()),
        "right_to_left" => Ok(line.source_logical_tiles.iter().rev().copied().collect()),
        other => Err(format!(
            "{} has unsupported source map order {other:?}",
            line.id
        )),
    }
}

fn build_target_map(
    line: &LineDeclaration,
    source_map: &[u8],
    target_logical_tiles: &[usize],
) -> Result<Vec<u8>, String> {
    let template = source_map
        .get(..DEFINITION_BYTES)
        .ok_or_else(|| format!("{} source map lacks a template record", line.id))?;
    let display_indices = match line.target_map_profile.as_deref() {
        Some("static_centered_right_to_left") => {
            (0..target_logical_tiles.len()).rev().collect::<Vec<_>>()
        }
        Some("consumer_positioned_left_to_right") => {
            (0..target_logical_tiles.len()).collect::<Vec<_>>()
        }
        _ => {
            return Err(format!("{} has an unsupported target map profile", line.id));
        }
    };

    let protected_bytes =
        usize::from(line.protected_trailing_definition.is_some()) * DEFINITION_BYTES;
    let mut output =
        Vec::with_capacity(target_logical_tiles.len() * DEFINITION_BYTES + protected_bytes);
    for display_index in display_indices {
        let target_tile = target_logical_tiles[display_index];
        let target_tile = u16::try_from(target_tile)
            .map_err(|_| format!("{} target tile does not fit in 16 bits", line.id))?;
        if target_tile & !TILE_INDEX_MASK != 0 {
            return Err(format!(
                "{} target tile 0x{target_tile:04X} exceeds the consumer mask",
                line.id
            ));
        }
        let mut definition = template.to_vec();
        let source_tile_word = u16::from_be_bytes([definition[6], definition[7]]);
        let target_tile_word = (source_tile_word & !TILE_INDEX_MASK) | target_tile;
        definition[6..8].copy_from_slice(&target_tile_word.to_be_bytes());

        match line.target_map_profile.as_deref() {
            Some("static_centered_right_to_left") => {
                let first_x = parse_u16_hex(
                    line.target_first_x
                        .as_deref()
                        .ok_or_else(|| format!("{} target first x is absent", line.id))?,
                )?;
                let step = line
                    .target_x_step
                    .ok_or_else(|| format!("{} target x step is absent", line.id))?;
                let x =
                    usize::from(first_x)
                        .checked_add(display_index.checked_mul(step).ok_or_else(|| {
                            format!("{} target x multiplication overflowed", line.id)
                        })?)
                        .ok_or_else(|| format!("{} target x overflowed", line.id))?;
                let x = u16::try_from(x)
                    .map_err(|_| format!("{} target x does not fit in 16 bits", line.id))?;
                definition[8..10].copy_from_slice(&x.to_be_bytes());
            }
            Some("consumer_positioned_left_to_right") => {}
            _ => unreachable!("target profile was checked above"),
        }
        output.extend_from_slice(&definition);
    }
    if line.protected_trailing_definition.is_some() {
        let suffix = source_map
            .get(source_map.len().saturating_sub(DEFINITION_BYTES)..)
            .ok_or_else(|| format!("{} protected suffix record is absent", line.id))?;
        output.extend_from_slice(suffix);
    }
    Ok(output)
}

fn build_relocation_patches(
    line: &LineDeclaration,
    target_map_offset: usize,
) -> Result<Vec<InstructionPatch>, String> {
    let count = line
        .count_patch
        .as_ref()
        .ok_or_else(|| format!("{} relocated line lacks a count patch", line.id))?;
    let source_count = assemble_count_instruction(count, count.source_count, &line.id)?;
    let target_count = assemble_count_instruction(count, count.target_count, &line.id)?;

    let source_map_address = parse_u32_hex(&line.source_map_offset)?;
    let target_map_address = u32::try_from(target_map_offset)
        .map_err(|_| format!("{} relocated map address exceeds 32 bits", line.id))?;
    let source_pointer = m68k::assemble(&[Inst::LeaAbsoluteLong {
        address: source_map_address,
        destination: AddressReg::A2,
    }])?;
    let target_pointer = m68k::assemble(&[Inst::LeaAbsoluteLong {
        address: target_map_address,
        destination: AddressReg::A2,
    }])?;

    let mut patches = vec![InstructionPatch {
        offset: parse_hex(&count.offset)?,
        source: source_count,
        target: target_count,
        label: format!("{} typed sprite-count comparison", line.id),
    }];
    for draw_count in &line.draw_count_patches {
        patches.push(InstructionPatch {
            offset: parse_hex(&draw_count.offset)?,
            source: assemble_count_instruction(draw_count, draw_count.source_count, &line.id)?,
            target: assemble_count_instruction(draw_count, draw_count.target_count, &line.id)?,
            label: format!("{} typed sprite draw-count field", line.id),
        });
    }
    patches.push(InstructionPatch {
        offset: parse_hex(&line.map_pointer_offset)?,
        source: source_pointer,
        target: target_pointer,
        label: format!("{} typed relocated-map pointer", line.id),
    });
    Ok(patches)
}

fn assemble_count_instruction(
    declaration: &CountPatchDeclaration,
    count: u16,
    line_id: &str,
) -> Result<Vec<u8>, String> {
    let instruction = match declaration.profile.as_str() {
        "cmpi_word_d0" => {
            if declaration.displacement.is_some() {
                return Err(format!(
                    "{line_id} comparison count patch unexpectedly declares a displacement"
                ));
            }
            Inst::CmpiWordImmediate {
                immediate: count,
                destination: DataReg::D0,
            }
        }
        "move_word_immediate_to_displacement_a0" => {
            let displacement = parse_u16_hex(
                declaration
                    .displacement
                    .as_deref()
                    .ok_or_else(|| format!("{line_id} count-field displacement is absent"))?,
            )?;
            Inst::MoveWordImmediateToDisplacementAddress {
                immediate: count,
                displacement,
                destination: AddressReg::A0,
            }
        }
        other => {
            return Err(format!(
                "{line_id} has unsupported typed count profile {other:?}"
            ));
        }
    };
    m68k::assemble(&[instruction])
}

fn render_segment(
    font: &Font,
    manifest: &CreditsTimedManifest,
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
        &format!("{} Korean credit segment", line.id),
    )?;
    let roles = encoded
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 15]) {
        return Err(format!(
            "{} Korean credit segment uses unexpected palette roles {roles:?}",
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

fn append_protected_preview(
    source_page: &[u8],
    base_surface: Vec<u8>,
    base_width: usize,
    line: &LineDeclaration,
    label: &str,
) -> Result<(Vec<u8>, usize), String> {
    let Some(protected) = &line.protected_trailing_definition else {
        return Ok((base_surface, base_width));
    };
    if base_surface.len() != base_width * CELL_HEIGHT_PIXELS {
        return Err(format!("{label} base preview geometry drifted"));
    }
    let suffix_width = protected.width_tiles * 8;
    let suffix_height = protected.height_tiles * 8;
    if protected.y_offset_px + suffix_height > CELL_HEIGHT_PIXELS {
        return Err(format!("{label} protected suffix escapes its preview row"));
    }
    let start = protected.source_tile_start * MD_TILE_BYTES;
    let end = protected.source_tile_end_exclusive * MD_TILE_BYTES;
    let encoded = source_range(
        source_page,
        start,
        end - start,
        &format!("{label} protected suffix patterns"),
    )?;
    let suffix = decode_md_tiles_column_major(
        encoded,
        suffix_width,
        suffix_height,
        &format!("{label} protected suffix surface"),
    )?;
    let width = base_width + suffix_width;
    let mut surface = vec![0u8; width * CELL_HEIGHT_PIXELS];
    for y in 0..CELL_HEIGHT_PIXELS {
        let source_row = &base_surface[y * base_width..(y + 1) * base_width];
        let destination = y * width;
        surface[destination..destination + base_width].copy_from_slice(source_row);
    }
    for y in 0..suffix_height {
        let source_row = &suffix[y * suffix_width..(y + 1) * suffix_width];
        let destination = (protected.y_offset_px + y) * width + base_width;
        surface[destination..destination + suffix_width].copy_from_slice(source_row);
    }
    Ok((surface, width))
}

fn draw_surface(
    rgba: &mut [u8],
    output_width: usize,
    logical_x: usize,
    logical_y: usize,
    surface_width: usize,
    surface: &[u8],
    ink: [u8; 4],
) -> Result<(), String> {
    let output_height = rgba.len() / 4 / output_width;
    if surface.len() != surface_width * CELL_HEIGHT_PIXELS {
        return Err("credit-name preview surface length is invalid".to_string());
    }
    for y in 0..CELL_HEIGHT_PIXELS {
        for x in 0..surface_width {
            let role = surface[y * surface_width + x];
            if role != 0 && role != 15 {
                return Err(format!(
                    "credit-name preview uses unexpected palette role {role}"
                ));
            }
            if role == 0 {
                continue;
            }
            for scale_y in 0..PREVIEW_SCALE {
                for scale_x in 0..PREVIEW_SCALE {
                    let output_x = (logical_x + x) * PREVIEW_SCALE + scale_x;
                    let output_y = (logical_y + y) * PREVIEW_SCALE + scale_y;
                    if output_x >= output_width || output_y >= output_height {
                        return Err("credit-name preview draw escaped its canvas".to_string());
                    }
                    let offset = (output_y * output_width + output_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&ink);
                }
            }
        }
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

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    u16::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 16 bits"))
}

fn parse_u8_hex(value: &str) -> Result<u8, String> {
    u8::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 8 bits"))
}

fn parse_u32_hex(value: &str) -> Result<u32, String> {
    u32::try_from(parse_hex(value)?).map_err(|_| format!("{value} does not fit in 32 bits"))
}
