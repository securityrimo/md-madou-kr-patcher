use std::collections::{BTreeMap, BTreeSet};

use fontdue::Font;

use super::common::{
    append_patterns, frame_preview, load_vram, logical_preview, records_for_cells, render_cells,
    table_replacement, validate_consumer,
};
use super::{ConsumerDeclaration, ExtraPayload, PageCompile};
use crate::graphics::sprite_map::SpriteFrame;

pub(super) fn compile_sequence(
    rom: &[u8],
    source_page: &[u8],
    target_page: &mut Vec<u8>,
    font: &Font,
    declaration: &ConsumerDeclaration,
) -> Result<PageCompile, String> {
    let frames = validate_consumer(rom, declaration, "JP source")?;
    if frames.len() != 18
        || frames[0..5].iter().any(|frame| frame.records.len() != 1)
        || frames[0].records[0].tile_index() != 0x2D
        || frames[1].records[0].tile_index() != 0
        || frames[2].records[0].tile_index() != 0x31
        || frames[3].records[0].tile_index() != 0x35
        || frames[4].records[0].tile_index() != 0x39
        || !frames[16].records.is_empty()
    {
        return Err(format!(
            "{} source sequence geometry drifted",
            declaration.id
        ));
    }

    let line = &declaration.lines[0];
    let compact = line
        .ko
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let compact_cells = super::common::native_cell_count(&compact, &line.id)?;
    let preview_cells = super::common::native_cell_count(&line.ko, &line.id)?;
    let patterns = render_cells(font, &compact, compact_cells, &line.id)?;
    let first_tile = append_patterns(target_page, &patterns, &declaration.id)?;
    let two_cell = |cell: usize, template: usize| -> Result<SpriteFrame, String> {
        Ok(SpriteFrame {
            records: records_for_cells(
                &frames[template].records[0],
                first_tile + cell * 4,
                2,
                0x0070,
            )?,
        })
    };
    let one_cell = |cell: usize, template: usize| -> Result<SpriteFrame, String> {
        Ok(SpriteFrame {
            records: records_for_cells(
                &frames[template].records[0],
                first_tile + cell * 4,
                1,
                0x0078,
            )?,
        })
    };
    let replacements = BTreeMap::from([
        (0usize, two_cell(0, 0)?),
        (2usize, one_cell(2, 2)?),
        (3usize, one_cell(3, 3)?),
        (4usize, two_cell(4, 4)?),
    ]);
    let table = table_replacement(rom, declaration, replacements)?;
    let preview = logical_preview(
        source_page,
        &[Some(0x2D), None, Some(0x31), Some(0x35), Some(0x39)],
        &render_cells(font, &line.ko, preview_cells, &line.id)?,
        &line.ko,
        preview_cells,
        font,
        &line.id,
    )?;

    Ok(PageCompile {
        tables: vec![table],
        extra_payload: None,
        previews: vec![preview],
        glyphs: line
            .ko
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<BTreeSet<_>>(),
        appended_tiles: patterns.len() / crate::graphics::MD_TILE_BYTES,
    })
}

pub(super) fn compile_names(
    rom: &[u8],
    font: &Font,
    declaration: &ConsumerDeclaration,
    source_payload: &[u8],
    vram_destination: u16,
) -> Result<PageCompile, String> {
    let frames = validate_consumer(rom, declaration, "JP source")?;
    if frames.len() != 5
        || frames
            .iter()
            .map(|frame| frame.records.len())
            .collect::<Vec<_>>()
            != [2, 3, 2, 2, 3]
        || source_payload.len() != 88 * crate::graphics::MD_TILE_BYTES
    {
        return Err(format!(
            "{} source five-name geometry drifted",
            declaration.id
        ));
    }

    let mut target_payload = Vec::new();
    let mut target_frames = Vec::new();
    let mut first_tile = usize::from(vram_destination) / crate::graphics::MD_TILE_BYTES;
    for (index, line) in declaration.lines.iter().enumerate() {
        let cells = super::common::native_cell_count(&line.ko, &line.id)?;
        let patterns = render_cells(font, &line.ko, cells, &line.id)?;
        let first_x = u16::try_from(0x80usize - cells * 8)
            .map_err(|_| format!("{} target name cannot be centered", line.id))?;
        target_frames.push(SpriteFrame {
            records: records_for_cells(&frames[index].records[0], first_tile, cells, first_x)?,
        });
        first_tile += patterns.len() / crate::graphics::MD_TILE_BYTES;
        target_payload.extend_from_slice(&patterns);
    }
    let replacements = target_frames
        .iter()
        .cloned()
        .enumerate()
        .collect::<BTreeMap<_, _>>();
    let table = table_replacement(rom, declaration, replacements)?;
    let source_vram = load_vram(
        vram_destination,
        source_payload,
        &format!("{} JP pack", declaration.id),
    )?;
    let target_vram = load_vram(
        vram_destination,
        &target_payload,
        &format!("{} KR pack", declaration.id),
    )?;
    let previews = frames
        .iter()
        .zip(&target_frames)
        .enumerate()
        .map(|(index, (source, target))| {
            frame_preview(
                &source_vram,
                source,
                &target_vram,
                target,
                &declaration.lines[index].id,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PageCompile {
        tables: vec![table],
        extra_payload: Some(ExtraPayload {
            target: target_payload.clone(),
        }),
        previews,
        glyphs: declaration
            .lines
            .iter()
            .flat_map(|line| line.ko.chars())
            .filter(|character| !character.is_whitespace())
            .collect::<BTreeSet<_>>(),
        appended_tiles: target_payload.len() / crate::graphics::MD_TILE_BYTES,
    })
}
