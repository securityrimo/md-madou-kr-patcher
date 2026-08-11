use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAnimationMasterSummary {
    pub animation_id: String,
    pub frames: usize,
    pub visible_cells: usize,
    pub width: usize,
    pub height: usize,
    pub alpha_x: usize,
    pub alpha_y: usize,
    pub alpha_width: usize,
    pub alpha_height: usize,
}

/// Rasterize one declared spelling with its pinned font provenance into the
/// binary 8x8-cell master consumed by the source-only build.
pub fn write_spell_animation_font_master(
    assets_dir: &Path,
    animation_id: &str,
    output_path: &Path,
) -> Result<SpellAnimationMasterSummary, String> {
    let manifest = read_manifest(assets_dir)?;
    let declaration = manifest
        .animations
        .iter()
        .find(|declaration| declaration.id == animation_id)
        .ok_or_else(|| format!("unknown spell-animation ID {animation_id:?}"))?;
    let master = declaration.normalized_master.as_ref().ok_or_else(|| {
        format!(
            "{} has no normalized-master font provenance",
            declaration.id
        )
    })?;
    if master.generation_method != "font_raster" {
        return Err(format!(
            "{} uses an approved normalized master rather than a reproducible direct font raster",
            declaration.id
        ));
    }
    let font = read_verified_font(
        assets_dir,
        &master.source_font_asset,
        &master.source_font_sha256,
        &format!("{} normalized-master source", declaration.id),
    )?;
    let width = FRAME_WIDTH_TILES * FRAME_COUNT * master.cell_size;
    let height = FRAME_HEIGHT_TILES * master.cell_size;
    if master.cell_size != 8 || master.width != width || master.height != height {
        return Err(format!(
            "{} normalized master geometry drifted",
            declaration.id
        ));
    }

    let mut rgba = vec![0u8; width * height * 4];
    let mut visible_cells = 0usize;
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    for (frame_index, frame) in declaration.target_frames.iter().enumerate() {
        let cells = if frame.is_empty() {
            vec![false; FRAME_WORDS]
        } else {
            let mut chars = frame.chars();
            let ch = chars
                .next()
                .ok_or_else(|| format!("{} has an empty frame", declaration.id))?;
            if chars.next().is_some() {
                return Err(format!(
                    "{} frame {frame_index} contains more than one glyph",
                    declaration.id
                ));
            }
            render_lettering_grid(
                &font,
                ch,
                master.source_font_size_px,
                master.source_coverage_threshold,
                &declaration.id,
            )?
        };
        for (cell_index, visible) in cells.into_iter().enumerate() {
            if !visible {
                continue;
            }
            visible_cells += 1;
            let cell_x = frame_index * FRAME_WIDTH_TILES + cell_index % FRAME_WIDTH_TILES;
            let cell_y = cell_index / FRAME_WIDTH_TILES;
            let pixel_x = cell_x * master.cell_size;
            let pixel_y = cell_y * master.cell_size;
            min_x = min_x.min(pixel_x);
            min_y = min_y.min(pixel_y);
            max_x = max_x.max(pixel_x + master.cell_size);
            max_y = max_y.max(pixel_y + master.cell_size);
            for local_y in 0..master.cell_size {
                for local_x in 0..master.cell_size {
                    let offset = ((pixel_y + local_y) * width + pixel_x + local_x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[0, 0, 0, 255]);
                }
            }
        }
    }
    if visible_cells == 0 {
        return Err(format!(
            "{} normalized master rendered blank",
            declaration.id
        ));
    }
    write_rgba_png(
        output_path,
        width as u32,
        height as u32,
        &rgba,
        &format!("{} normalized cell master", declaration.id),
    )?;
    Ok(SpellAnimationMasterSummary {
        animation_id: declaration.id.clone(),
        frames: declaration.target_frames.len(),
        visible_cells,
        width,
        height,
        alpha_x: min_x,
        alpha_y: min_y,
        alpha_width: max_x - min_x,
        alpha_height: max_y - min_y,
    })
}
