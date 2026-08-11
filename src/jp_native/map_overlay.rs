//! Source-owned footprint contracts for the map's persistent floor label.

use crate::build::text::Token;

use super::dynamic_layout::{
    DynamicDisplayControl, dynamic_trailing_word_half_cells, fixed_width_token_half_cells,
    validate_fixed_width_layout,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MapLabelFootprintSpec {
    pub(super) id: &'static str,
    pub(super) source_owned_half_cells: usize,
}

/// The floor label remains in Plane A after the map closes. Both the dungeon
/// view and another floor label reuse the seven JP-owned cells instead of
/// clearing them first. Every localized variant must therefore advance across
/// all fourteen half-cells without placing visible content in an eighth cell.
pub(super) const MAP_LABEL_FOOTPRINT_SPECS: [MapLabelFootprintSpec; 2] = [
    MapLabelFootprintSpec {
        id: "script_0007",
        source_owned_half_cells: 14,
    },
    MapLabelFootprintSpec {
        id: "script_0008",
        source_owned_half_cells: 14,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MapLabelWriteFootprint {
    advanced_half_cells: usize,
    touched_half_cells: usize,
}

fn map_label_write_footprint(id: &str, tokens: &[Token]) -> Result<MapLabelWriteFootprint, String> {
    let mut advanced_half_cells = 0usize;
    let mut touched_half_cells = 0usize;
    for token in tokens {
        match token {
            Token::KrChar(_) | Token::Tile(_) | Token::LayoutPad | Token::Raw(_) => {
                let width = fixed_width_token_half_cells(token);
                touched_half_cells = touched_half_cells.max(advanced_half_cells + width);
                advanced_half_cells += width;
            }
            Token::EnChar(_) => {
                let width = fixed_width_token_half_cells(token);
                // The JP renderer allocates and uploads a complete 16-pixel
                // cache cell even when a localized glyph advances by only one
                // 8-pixel half-cell. A following token normally overwrites the
                // unused half. A terminal half-width glyph therefore touches
                // one half-cell beyond the logical cursor.
                touched_half_cells = touched_half_cells.max(advanced_half_cells + 2);
                advanced_half_cells += width;
            }
            Token::CtrlParam(code, trailing_word)
                if let Some(consumer) = DynamicDisplayControl::from_code(*code) =>
            {
                let width =
                    consumer.visible_words() * 2 + dynamic_trailing_word_half_cells(*trailing_word);
                touched_half_cells = touched_half_cells.max(advanced_half_cells + width);
                advanced_half_cells += width;
            }
            Token::Ctrl(0xFF04 | 0xFF80) => {}
            Token::Ctrl(code) | Token::CtrlParam(code, _) => {
                return Err(format!(
                    "{id}: map label contains unsupported control 0x{code:04X}"
                ));
            }
            Token::SourceRowFinalize { .. } => {
                return Err(format!(
                    "{id}: map label must express its overwrite width in source text"
                ));
            }
        }
    }
    Ok(MapLabelWriteFootprint {
        advanced_half_cells,
        touched_half_cells,
    })
}

pub(super) fn validate_map_label_source_footprint(
    id: &str,
    tokens: &[Token],
) -> Result<(), String> {
    let Some(spec) = MAP_LABEL_FOOTPRINT_SPECS.iter().find(|spec| spec.id == id) else {
        return Ok(());
    };

    let source_owned_cells = spec.source_owned_half_cells.div_ceil(2);
    validate_fixed_width_layout(tokens, id, source_owned_cells, None).map_err(|error| {
        format!(
            "{id}: map label exceeds the JP source-owned {}-cell overlay footprint: {error}",
            source_owned_cells
        )
    })?;

    let footprint = map_label_write_footprint(id, tokens)?;
    if footprint.advanced_half_cells != spec.source_owned_half_cells {
        return Err(format!(
            "{id}: map label advances {} half-cells, but the persistent JP overlay requires exactly {}",
            footprint.advanced_half_cells, spec.source_owned_half_cells
        ));
    }
    if footprint.touched_half_cells > spec.source_owned_half_cells {
        return Err(format!(
            "{id}: map label physically touches {} half-cells through the JP 16-pixel glyph cache, beyond the persistent JP overlay's {} half-cells",
            footprint.touched_half_cells, spec.source_owned_half_cells
        ));
    }
    Ok(())
}
