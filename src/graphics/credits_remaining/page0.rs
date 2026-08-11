use std::collections::{BTreeMap, BTreeSet};

use fontdue::Font;

use super::common::{
    TILE_INDEX_MASK, append_patterns, frame_preview, load_vram, render_signature_frame,
    table_replacement, validate_consumer,
};
use super::{ConsumerDeclaration, PageCompile};
use crate::graphics::sprite_map::{SpriteFrame, render_frame};

pub(super) fn compile(
    rom: &[u8],
    source_page: &[u8],
    target_page: &mut Vec<u8>,
    font: &Font,
    declaration: &ConsumerDeclaration,
) -> Result<PageCompile, String> {
    let frames = validate_consumer(rom, declaration, "JP source")?;
    if frames.len() != 10
        || frames[6].records.len() != 3
        || frames[6]
            .records
            .iter()
            .any(|record| record.width_tiles() != 4 || record.height_tiles() != 1)
    {
        return Err(format!(
            "{} source frame-6 geometry drifted",
            declaration.id
        ));
    }

    let source_vram = load_vram(0, source_page, &format!("{} JP page", declaration.id))?;
    let line = &declaration.lines[0];
    let mut first_tiles = Vec::with_capacity(4);
    let mut appended_pattern_bytes = 0usize;
    for (frame_index, frame) in frames.iter().enumerate().skip(6) {
        let source_surface = render_frame(
            &source_vram,
            frame,
            &format!("{} JP signature frame {frame_index}", declaration.id),
        )?;
        let patterns = render_signature_frame(font, &line.ko, &source_surface, &line.id)?;
        first_tiles.push(append_patterns(target_page, &patterns, &declaration.id)?);
        appended_pattern_bytes += patterns.len();
    }
    // The producer animates frame indices 0 through 9.  Frames 0-5 unfold the
    // signature horizontally, 6-8 bounce it, and 9 is the held final frame.
    // Every selected frame must therefore reference translated patterns; only
    // replacing the immediate selector's initial frame 6 lets the animation
    // advance back into the JP frame 9.
    const TARGET_X: [[u16; 4]; 10] = [
        [0x0040, 0x0040, 0x0040, 0x0040],
        [0x0060, 0x0060, 0x0060, 0x0060],
        [0x0040, 0x0060, 0x0060, 0x0060],
        [0x0040, 0x0060, 0x0080, 0x0080],
        [0x0040, 0x0060, 0x0080, 0x00A0],
        [0x0040, 0x0060, 0x0080, 0x00A0],
        [0x0040, 0x0060, 0x0080, 0x00A0],
        [0x0040, 0x0060, 0x0080, 0x00A0],
        [0x0040, 0x0060, 0x0080, 0x00A0],
        [0x0040, 0x0060, 0x0080, 0x00A0],
    ];
    let mut target_frames = Vec::with_capacity(frames.len());
    for frame_index in 0..frames.len() {
        let pattern_frame = if frame_index <= 5 { 3 } else { frame_index - 6 };
        let first_tile = first_tiles[pattern_frame];
        let height_tiles = if matches!(pattern_frame, 0 | 1) { 1 } else { 2 };
        let records = (0..4usize)
            .map(|record_index| {
                let tile = first_tile
                    .checked_add(record_index * 4 * height_tiles)
                    .ok_or_else(|| format!("{} tile index overflowed", declaration.id))?;
                let tile = u16::try_from(tile)
                    .map_err(|_| format!("{} tile index exceeds 16 bits", declaration.id))?;
                if tile & !TILE_INDEX_MASK != 0 {
                    return Err(format!(
                        "{} tile index exceeds the sprite consumer mask",
                        declaration.id
                    ));
                }
                let mut target = frames[frame_index].records[0].clone();
                target.size_and_link = (target.size_and_link & !0x0F00)
                    | 0x0C00
                    | u16::try_from((height_tiles - 1) << 8)
                        .map_err(|_| format!("{} height bits overflowed", declaration.id))?;
                target.tile_and_attributes = (target.tile_and_attributes & !TILE_INDEX_MASK) | tile;
                target.x = TARGET_X[frame_index][record_index];
                Ok(target)
            })
            .collect::<Result<Vec<_>, String>>()?;
        target_frames.push(SpriteFrame { records });
    }
    let replacements = target_frames
        .iter()
        .cloned()
        .enumerate()
        .collect::<BTreeMap<_, _>>();
    let table = table_replacement(rom, declaration, replacements)?;

    let target_vram = load_vram(0, target_page, &format!("{} KR page", declaration.id))?;
    let preview = frame_preview(
        &source_vram,
        &frames[9],
        &target_vram,
        &target_frames[9],
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
        appended_tiles: appended_pattern_bytes / crate::graphics::MD_TILE_BYTES,
    })
}
