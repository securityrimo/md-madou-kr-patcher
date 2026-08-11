use std::collections::{BTreeMap, BTreeSet};

use fontdue::Font;

use super::common::{
    frame_preview, load_vram, records_for_cells, render_cells, table_replacement, validate_consumer,
};
use super::{ConsumerDeclaration, ExtraPayload, PageCompile};
use crate::graphics::sprite_map::SpriteFrame;

pub(super) fn compile(
    rom: &[u8],
    font: &Font,
    declaration: &ConsumerDeclaration,
    source_payload: &[u8],
    vram_destination: u16,
) -> Result<PageCompile, String> {
    let frames = validate_consumer(rom, declaration, "JP source")?;
    if frames.len() != 2
        || frames.iter().any(|frame| frame.records.len() != 2)
        || frames
            .iter()
            .flat_map(|frame| &frame.records)
            .any(|record| record.width_tiles() != 4 || record.height_tiles() != 2)
    {
        return Err(format!(
            "{} source two-frame geometry drifted",
            declaration.id
        ));
    }
    if source_payload.len() != 32 * crate::graphics::MD_TILE_BYTES {
        return Err(format!("{} source payload is not 32 tiles", declaration.id));
    }

    let name_line = &declaration.lines[0];
    let combined_line = &declaration.lines[1];
    let nickname_line = &declaration.lines[2];
    let name_cells = super::common::native_cell_count(&name_line.ko, &name_line.id)?;
    let nickname_cells = super::common::native_cell_count(&nickname_line.ko, &nickname_line.id)?;
    let combined_cells = name_cells
        .checked_add(nickname_cells)
        .ok_or_else(|| format!("{} combined width overflowed", combined_line.id))?;
    let combined_width = combined_cells
        .checked_mul(16)
        .ok_or_else(|| format!("{} combined width overflowed", combined_line.id))?;
    let name_first_x = 0x80usize
        .checked_sub(combined_width / 2)
        .ok_or_else(|| format!("{} cannot be centered", combined_line.id))?;
    let nickname_first_x = name_first_x
        .checked_add(name_cells * 16)
        .ok_or_else(|| format!("{} nickname position overflowed", combined_line.id))?;
    let name = render_cells(font, &name_line.ko, name_cells, &name_line.id)?;
    let nickname = render_cells(font, &nickname_line.ko, nickname_cells, &nickname_line.id)?;
    let mut target_payload = name.clone();
    target_payload.extend_from_slice(&nickname);
    let first_tile = usize::from(vram_destination) / crate::graphics::MD_TILE_BYTES;
    let nickname_tile = first_tile + name.len() / crate::graphics::MD_TILE_BYTES;

    let name_frame = SpriteFrame {
        records: records_for_cells(
            &frames[0].records[0],
            first_tile,
            name_cells,
            u16::try_from(name_first_x)
                .map_err(|_| format!("{} cannot be centered", name_line.id))?,
        )?,
    };
    let nickname_frame = SpriteFrame {
        records: records_for_cells(
            &frames[1].records[0],
            nickname_tile,
            nickname_cells,
            u16::try_from(nickname_first_x)
                .map_err(|_| format!("{} nickname position is invalid", combined_line.id))?,
        )?,
    };
    let table = table_replacement(
        rom,
        declaration,
        BTreeMap::from([
            (0usize, name_frame.clone()),
            (1usize, nickname_frame.clone()),
        ]),
    )?;

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
    let name_preview = frame_preview(
        &source_vram,
        &frames[0],
        &target_vram,
        &name_frame,
        &name_line.id,
    )?;
    let mut source_combined = frames[0].clone();
    source_combined.records.extend(frames[1].records.clone());
    let mut target_combined = name_frame.clone();
    target_combined
        .records
        .extend(nickname_frame.records.clone());
    let combined_preview = frame_preview(
        &source_vram,
        &source_combined,
        &target_vram,
        &target_combined,
        &combined_line.id,
    )?;
    let nickname_preview = frame_preview(
        &source_vram,
        &frames[1],
        &target_vram,
        &nickname_frame,
        &nickname_line.id,
    )?;

    Ok(PageCompile {
        tables: vec![table],
        extra_payload: Some(ExtraPayload {
            target: target_payload,
        }),
        previews: vec![name_preview, combined_preview, nickname_preview],
        glyphs: declaration
            .lines
            .iter()
            .flat_map(|line| line.ko.chars())
            .filter(|character| !character.is_whitespace())
            .collect::<BTreeSet<_>>(),
        appended_tiles: (name.len() + nickname.len()) / crate::graphics::MD_TILE_BYTES,
    })
}
