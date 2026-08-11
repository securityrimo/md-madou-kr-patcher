use std::collections::{BTreeMap, BTreeSet};

use fontdue::Font;

use crate::m68k::{self, AddressReg, DataReg, Inst};

use super::{
    ConsumerDeclaration, SelectorDeclaration, TableReplacement, parse_u8_hex, validate_sha256,
};
use crate::graphics::credits_generic::{
    FRAME_INDEX_DISPLACEMENT, OBJECT_TYPE_DISPLACEMENT, POINTER_ENTRY_BYTES, POINTER_TABLE_OFFSET,
};
use crate::graphics::credits_timed::CreditLinePreview;
use crate::graphics::font_effect::{native_text_width, render_native_text_line};
use crate::graphics::pixel::{decode_md_tiles_column_major, encode_md_tiles_column_major};
use crate::graphics::sprite_map::{
    FrameSurface, SpriteFrame, SpriteRecord, VirtualVram, parse_frame_table, render_frame,
};
use crate::graphics::{MD_TILE_BYTES, parse_hex, sha256_hex, source_range};

pub(super) const CELL_WIDTH: usize = 16;
pub(super) const CELL_HEIGHT: usize = 16;
pub(super) const CELL_TILES: usize = 4;
pub(super) const SPACE_WIDTH: usize = 8;
pub(super) const TILE_INDEX_MASK: u16 = 0x07FF;
pub(super) const SIGNATURE_SOURCE_WIDTH: usize = 96;
pub(super) const SIGNATURE_TARGET_WIDTH: usize = 128;
pub(super) const SIGNATURE_HEIGHT: usize = 8;
const SIGNATURE_MOO_WIDTH: usize = 48;

pub(super) fn validate_consumer(
    rom: &[u8],
    declaration: &ConsumerDeclaration,
    label: &str,
) -> Result<Vec<SpriteFrame>, String> {
    let object_type = parse_u8_hex(&declaration.object_type)?;
    let pointer_offset = parse_hex(&declaration.type_pointer_offset)?;
    if pointer_offset != POINTER_TABLE_OFFSET + usize::from(object_type) * POINTER_ENTRY_BYTES {
        return Err(format!(
            "{} object-type pointer slot drifted",
            declaration.id
        ));
    }
    let pointer = source_range(
        rom,
        pointer_offset,
        POINTER_ENTRY_BYTES,
        &format!("{} object-type pointer", declaration.id),
    )?;
    if sha256_hex(pointer) != declaration.type_pointer_sha256 {
        return Err(format!(
            "{label} {} object-type pointer SHA-256 drifted",
            declaration.id
        ));
    }
    let source_table_offset = parse_hex(&declaration.source_table_offset)?;
    if usize::try_from(u32::from_be_bytes([
        pointer[0], pointer[1], pointer[2], pointer[3],
    ]))
    .map_err(|_| format!("{} source pointer does not fit usize", declaration.id))?
        != source_table_offset
    {
        return Err(format!(
            "{label} {} object-type pointer does not select its declared table",
            declaration.id
        ));
    }
    let table = source_range(
        rom,
        source_table_offset,
        declaration.source_table_bytes,
        &format!("{} source frame table", declaration.id),
    )?;
    if sha256_hex(table) != declaration.source_table_sha256 {
        return Err(format!(
            "{label} {} source frame-table SHA-256 drifted",
            declaration.id
        ));
    }
    let window_offset = parse_hex(&declaration.selection_window_offset)?;
    let window = source_range(
        rom,
        window_offset,
        declaration.selection_window_bytes,
        &format!("{} selection window", declaration.id),
    )?;
    if sha256_hex(window) != declaration.selection_window_sha256 {
        return Err(format!(
            "{label} {} selection-window SHA-256 drifted",
            declaration.id
        ));
    }
    for selector in &declaration.typed_selectors {
        validate_selector(rom, selector, &declaration.id, label)?;
    }
    parse_frame_table(
        table,
        declaration.frame_count,
        &format!("{} source", declaration.id),
    )
}

pub(super) fn validate_selectors(
    rom: &[u8],
    declaration: &ConsumerDeclaration,
    label: &str,
) -> Result<(), String> {
    for selector in &declaration.typed_selectors {
        validate_selector(rom, selector, &declaration.id, label)?;
    }
    Ok(())
}

fn validate_selector(
    rom: &[u8],
    selector: &SelectorDeclaration,
    id: &str,
    label: &str,
) -> Result<(), String> {
    let instruction = match selector.profile.as_str() {
        "object_type_immediate_a1" => Inst::MoveByteImmediateToDisplacementAddress {
            immediate: parse_u8_hex(&selector.value)?,
            displacement: OBJECT_TYPE_DISPLACEMENT,
            destination: AddressReg::A1,
        },
        "frame_index_immediate_a1" => Inst::MoveByteImmediateToDisplacementAddress {
            immediate: parse_u8_hex(&selector.value)?,
            displacement: FRAME_INDEX_DISPLACEMENT,
            destination: AddressReg::A1,
        },
        "frame_index_data_d0_a1" => Inst::MoveByteDataToDisplacementAddress {
            source: DataReg::D0,
            displacement: FRAME_INDEX_DISPLACEMENT,
            destination: AddressReg::A1,
        },
        "frame_index_data_d7_a1" => Inst::MoveByteDataToDisplacementAddress {
            source: DataReg::D7,
            displacement: FRAME_INDEX_DISPLACEMENT,
            destination: AddressReg::A1,
        },
        other => return Err(format!("{id} has unsupported selector profile {other:?}")),
    };
    let expected = m68k::assemble(&[instruction])?;
    let offset = parse_hex(&selector.offset)?;
    if source_range(rom, offset, expected.len(), "typed credit selector")? != expected {
        return Err(format!(
            "{label} {id} typed selector at 0x{offset:06X} drifted"
        ));
    }
    Ok(())
}

pub(super) fn table_replacement(
    rom: &[u8],
    declaration: &ConsumerDeclaration,
    replacements: BTreeMap<usize, SpriteFrame>,
) -> Result<TableReplacement, String> {
    let source_offset = parse_hex(&declaration.source_table_offset)?;
    let source = source_range(
        rom,
        source_offset,
        declaration.source_table_bytes,
        &format!("{} source frame table", declaration.id),
    )?
    .to_vec();
    let target = rebuild_selected_frames(
        &source,
        declaration.frame_count,
        &replacements,
        &declaration.id,
    )?;
    Ok(TableReplacement {
        id: declaration.id.clone(),
        pointer_offset: parse_hex(&declaration.type_pointer_offset)?,
        source_offset,
        source,
        target,
    })
}

fn rebuild_selected_frames(
    source: &[u8],
    frame_count: usize,
    replacements: &BTreeMap<usize, SpriteFrame>,
    label: &str,
) -> Result<Vec<u8>, String> {
    let header_bytes = frame_count
        .checked_mul(2)
        .ok_or_else(|| format!("{label} frame-header size overflowed"))?;
    let header = source_range(source, 0, header_bytes, "source frame offsets")?;
    let offsets = header
        .chunks_exact(2)
        .map(|pair| usize::from(u16::from_be_bytes([pair[0], pair[1]])))
        .collect::<Vec<_>>();
    if offsets.first().copied() != Some(header_bytes)
        || offsets.iter().any(|&offset| offset < header_bytes)
        || offsets.windows(2).any(|pair| pair[0] > pair[1])
        || offsets.last().is_none_or(|&offset| offset >= source.len())
    {
        return Err(format!(
            "{label} source offsets are not a nondecreasing frame table"
        ));
    }
    if replacements.keys().any(|&frame| frame >= frame_count) {
        return Err(format!("{label} replacement frame is out of range"));
    }

    let mut target = vec![0u8; header_bytes];
    for frame in 0..frame_count {
        let offset = u16::try_from(target.len())
            .map_err(|_| format!("{label} rebuilt table exceeds 64 KiB"))?;
        target[frame * 2..frame * 2 + 2].copy_from_slice(&offset.to_be_bytes());
        if let Some(replacement) = replacements.get(&frame) {
            target.extend_from_slice(&replacement.encode()?);
        } else {
            let start = offsets[frame];
            let end = offsets.get(frame + 1).copied().unwrap_or(source.len());
            target.extend_from_slice(source_range(
                source,
                start,
                end - start,
                &format!("{label} protected frame {frame}"),
            )?);
        }
    }

    let target_frames = parse_frame_table(&target, frame_count, &format!("{label} target"))?;
    for frame in 0..frame_count {
        if let Some(replacement) = replacements.get(&frame) {
            if &target_frames[frame] != replacement {
                return Err(format!("{label} target frame {frame} did not round-trip"));
            }
        } else {
            let source_start = offsets[frame];
            let source_end = offsets.get(frame + 1).copied().unwrap_or(source.len());
            let target_header = &target[..header_bytes];
            let target_offsets = target_header
                .chunks_exact(2)
                .map(|pair| usize::from(u16::from_be_bytes([pair[0], pair[1]])))
                .collect::<Vec<_>>();
            let target_start = target_offsets[frame];
            let target_end = target_offsets
                .get(frame + 1)
                .copied()
                .unwrap_or(target.len());
            if source[source_start..source_end] != target[target_start..target_end] {
                return Err(format!("{label} unrelated frame {frame} changed"));
            }
        }
    }
    Ok(target)
}

pub(super) fn render_cells(
    font: &Font,
    text: &str,
    cells: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let width = cells
        .checked_mul(CELL_WIDTH)
        .ok_or_else(|| format!("{label} target width overflowed"))?;
    if native_text_width(text, SPACE_WIDTH) > width {
        return Err(format!("{label} does not fit {cells} native cells"));
    }
    let surface = render_native_text_line(font, text, width, SPACE_WIDTH, 0, 15, label)?;
    let encoded = encode_md_tiles_column_major(&surface, width, CELL_HEIGHT, label)?;
    let roles = encoded
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 15]) {
        return Err(format!("{label} uses unexpected palette roles {roles:?}"));
    }
    Ok(encoded)
}

pub(super) fn native_cell_count(text: &str, label: &str) -> Result<usize, String> {
    let width = native_text_width(text, SPACE_WIDTH);
    let cells = width
        .checked_add(CELL_WIDTH - 1)
        .ok_or_else(|| format!("{label} native-cell width overflowed"))?
        / CELL_WIDTH;
    if cells == 0 {
        return Err(format!("{label} has no visible native cells"));
    }
    Ok(cells)
}

pub(super) fn render_signature_frame(
    font: &Font,
    text: &str,
    source: &FrameSurface,
    label: &str,
) -> Result<Vec<u8>, String> {
    let korean = text
        .strip_prefix("MOO ")
        .ok_or_else(|| format!("{label} must preserve the MOO signature prefix"))?;
    if source.width != SIGNATURE_SOURCE_WIDTH
        || !matches!(source.height, SIGNATURE_HEIGHT | CELL_HEIGHT)
        || source.pixels.len() != SIGNATURE_SOURCE_WIDTH * source.height
        || native_text_width(korean, SPACE_WIDTH) != 64
    {
        return Err(format!(
            "{label} does not match the source-native MOO signature geometry"
        ));
    }
    let korean_surface = render_native_text_line(font, korean, 64, SPACE_WIDTH, 0, 15, label)?;
    let (source_ink_top, source_ink_bottom) = ink_row_bounds(
        &source.pixels,
        source.width,
        source.height,
        &format!("{label} JP signature"),
    )?;
    let (korean_ink_top, korean_ink_bottom) = ink_row_bounds(
        &korean_surface,
        64,
        CELL_HEIGHT,
        &format!("{label} Korean signature"),
    )?;
    let source_ink_height = source_ink_bottom - source_ink_top + 1;
    let korean_ink_height = korean_ink_bottom - korean_ink_top + 1;

    // Preserve the source frame's M/O/O pixels byte-for-pixel, retain one
    // half-cell gap, and vertically map the four Korean cells to that exact
    // animation frame's ink envelope.  Frame 9 therefore stays full-height,
    // while frames 6-8 retain the original squash-and-bounce progression.
    let mut surface = vec![0u8; SIGNATURE_TARGET_WIDTH * source.height];
    let text_width = SIGNATURE_MOO_WIDTH + SPACE_WIDTH + 64;
    let start_x = (SIGNATURE_TARGET_WIDTH - text_width) / 2;
    for y in 0..source.height {
        let source_row = y * SIGNATURE_SOURCE_WIDTH;
        let target_row = y * SIGNATURE_TARGET_WIDTH;
        surface[target_row + start_x..target_row + start_x + SIGNATURE_MOO_WIDTH]
            .copy_from_slice(&source.pixels[source_row..source_row + SIGNATURE_MOO_WIDTH]);
    }
    let korean_x = start_x + SIGNATURE_MOO_WIDTH + SPACE_WIDTH;
    for target_row_in_ink in 0..source_ink_height {
        let source_row_in_glyph =
            korean_ink_top + target_row_in_ink * korean_ink_height / source_ink_height;
        let target_y = source_ink_top + target_row_in_ink;
        for x in 0..64 {
            if korean_surface[source_row_in_glyph * 64 + x] != 0 {
                surface[target_y * SIGNATURE_TARGET_WIDTH + korean_x + x] = 15;
            }
        }
    }
    let encoded =
        encode_md_tiles_column_major(&surface, SIGNATURE_TARGET_WIDTH, source.height, label)?;
    let roles = encoded
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0F])
        .collect::<BTreeSet<_>>();
    if roles != BTreeSet::from([0, 15]) {
        return Err(format!("{label} uses unexpected palette roles {roles:?}"));
    }
    Ok(encoded)
}

fn ink_row_bounds(
    surface: &[u8],
    width: usize,
    height: usize,
    label: &str,
) -> Result<(usize, usize), String> {
    if surface.len() != width * height {
        return Err(format!("{label} surface dimensions drifted"));
    }
    let rows = (0..height)
        .filter(|&y| {
            surface[y * width..(y + 1) * width]
                .iter()
                .any(|&pixel| pixel != 0)
        })
        .collect::<Vec<_>>();
    match (rows.first(), rows.last()) {
        (Some(&top), Some(&bottom)) => Ok((top, bottom)),
        _ => Err(format!("{label} has no visible ink rows")),
    }
}

pub(super) fn append_patterns(
    page: &mut Vec<u8>,
    patterns: &[u8],
    label: &str,
) -> Result<usize, String> {
    if !page.len().is_multiple_of(MD_TILE_BYTES)
        || patterns.is_empty()
        || !patterns.len().is_multiple_of(CELL_TILES * MD_TILE_BYTES)
    {
        return Err(format!("{label} append is not native-cell aligned"));
    }
    let first_tile = page.len() / MD_TILE_BYTES;
    page.extend_from_slice(patterns);
    Ok(first_tile)
}

pub(super) fn records_for_cells(
    template: &SpriteRecord,
    first_tile: usize,
    cells: usize,
    first_x: u16,
) -> Result<Vec<SpriteRecord>, String> {
    let mut records = Vec::new();
    let mut used_cells = 0usize;
    while used_cells < cells {
        let chunk_cells = (cells - used_cells).min(2);
        let tile = first_tile
            .checked_add(used_cells * CELL_TILES)
            .ok_or_else(|| "credit tile index overflowed".to_string())?;
        let tile =
            u16::try_from(tile).map_err(|_| "credit tile index exceeds 16 bits".to_string())?;
        if tile & !TILE_INDEX_MASK != 0 {
            return Err("credit tile index exceeds the sprite consumer mask".to_string());
        }
        let x = usize::from(first_x)
            .checked_add(used_cells * CELL_WIDTH)
            .ok_or_else(|| "credit record x overflowed".to_string())?;
        let width_bits = if chunk_cells == 2 { 0x0C00 } else { 0x0400 };
        records.push(SpriteRecord {
            y: 0x0078,
            size_and_link: (template.size_and_link & !0x0F00) | width_bits | 0x0100,
            tile_and_attributes: (template.tile_and_attributes & !TILE_INDEX_MASK) | tile,
            x: u16::try_from(x).map_err(|_| "credit record x exceeds 16 bits".to_string())?,
        });
        used_cells += chunk_cells;
    }
    Ok(records)
}

pub(super) fn frame_preview(
    source_vram: &VirtualVram,
    source_frame: &SpriteFrame,
    target_vram: &VirtualVram,
    target_frame: &SpriteFrame,
    label: &str,
) -> Result<CreditLinePreview, String> {
    let source = render_frame(source_vram, source_frame, &format!("{label} JP"))?;
    let target = render_frame(target_vram, target_frame, &format!("{label} KR"))?;
    let (source_surface, source_width) = pad_surface(source, &format!("{label} JP"))?;
    let (target_surface, target_width) = pad_surface(target, &format!("{label} KR"))?;
    Ok(CreditLinePreview {
        source_surface,
        target_surface,
        source_width,
        target_width,
    })
}

pub(super) fn logical_preview(
    source_page: &[u8],
    source_tiles: &[Option<usize>],
    target_patterns: &[u8],
    target_text: &str,
    target_cells: usize,
    font: &Font,
    label: &str,
) -> Result<CreditLinePreview, String> {
    let source_width = source_tiles.len() * CELL_WIDTH;
    let mut source_surface = vec![0u8; source_width * CELL_HEIGHT];
    for (cell, tile) in source_tiles.iter().enumerate() {
        let Some(tile) = tile else {
            continue;
        };
        let bytes = source_range(
            source_page,
            tile * MD_TILE_BYTES,
            CELL_TILES * MD_TILE_BYTES,
            &format!("{label} JP logical cell"),
        )?;
        let decoded = decode_md_tiles_column_major(
            bytes,
            CELL_WIDTH,
            CELL_HEIGHT,
            &format!("{label} JP logical cell"),
        )?;
        for y in 0..CELL_HEIGHT {
            source_surface
                [y * source_width + cell * CELL_WIDTH..y * source_width + (cell + 1) * CELL_WIDTH]
                .copy_from_slice(&decoded[y * CELL_WIDTH..(y + 1) * CELL_WIDTH]);
        }
    }
    let target_width = target_cells * CELL_WIDTH;
    let target_surface = decode_md_tiles_column_major(
        target_patterns,
        target_width,
        CELL_HEIGHT,
        &format!("{label} KR logical line"),
    )?;
    if native_text_width(target_text, SPACE_WIDTH) > target_width {
        return Err(format!("{label} target preview width drifted"));
    }
    let independently_rendered =
        render_native_text_line(font, target_text, target_width, SPACE_WIDTH, 0, 15, label)?;
    if target_surface != independently_rendered {
        return Err(format!(
            "{label} target preview differs from its font source"
        ));
    }
    Ok(CreditLinePreview {
        source_surface,
        target_surface,
        source_width,
        target_width,
    })
}

fn pad_surface(surface: FrameSurface, label: &str) -> Result<(Vec<u8>, usize), String> {
    if surface.height > CELL_HEIGHT || surface.width == 0 {
        return Err(format!("{label} preview surface has invalid geometry"));
    }
    let mut output = vec![0u8; surface.width * CELL_HEIGHT];
    let y_offset = (CELL_HEIGHT - surface.height) / 2;
    for y in 0..surface.height {
        let source = &surface.pixels[y * surface.width..(y + 1) * surface.width];
        let destination = (y + y_offset) * surface.width;
        output[destination..destination + surface.width].copy_from_slice(source);
    }
    Ok((output, surface.width))
}

pub(super) fn load_vram(
    destination: u16,
    payload: &[u8],
    label: &str,
) -> Result<VirtualVram, String> {
    let mut vram = VirtualVram::new();
    vram.load(destination, payload, label)?;
    Ok(vram)
}

pub(super) fn validate_declaration_hashes(declaration: &ConsumerDeclaration) -> Result<(), String> {
    for hash in [
        &declaration.type_pointer_sha256,
        &declaration.source_table_sha256,
        &declaration.selection_window_sha256,
    ] {
        validate_sha256(hash, &declaration.id)?;
    }
    Ok(())
}
