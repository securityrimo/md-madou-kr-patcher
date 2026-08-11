//! Shared deterministic RGBA and Mega Drive palette helpers.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::{MD_TILE_BYTES, sha256_hex};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub(super) struct PixelBounds {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

pub(super) fn read_verified_rgba(
    assets_dir: &Path,
    relative_path: &str,
    expected_sha256: &str,
    expected_width: usize,
    expected_height: usize,
    expected_bounds: PixelBounds,
    label: &str,
) -> Result<Vec<u8>, String> {
    let path = assets_dir.join(relative_path);
    let bytes = fs::read(&path)
        .map_err(|error| format!("failed to read {label} {}: {error}", path.display()))?;
    let actual_hash = sha256_hex(&bytes);
    if actual_hash != expected_sha256 {
        return Err(format!(
            "{}: {label} SHA-256 mismatch: expected {expected_sha256}, got {actual_hash}",
            path.display()
        ));
    }

    let decoder = png::Decoder::new(bytes.as_slice());
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("failed to read {label} PNG header: {error}"))?;
    let mut output = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut output)
        .map_err(|error| format!("failed to decode {label} PNG: {error}"))?;
    if info.width as usize != expected_width
        || info.height as usize != expected_height
        || info.color_type != png::ColorType::Rgba
        || info.bit_depth != png::BitDepth::Eight
    {
        return Err(format!(
            "{}: {label} must be {expected_width}x{expected_height} RGBA8",
            path.display()
        ));
    }
    output.truncate(info.buffer_size());
    let actual_bounds = detect_alpha_bounds(&output, expected_width, expected_height, label)?;
    if actual_bounds != expected_bounds {
        return Err(format!(
            "{}: {label} alpha bounds drifted from {expected_bounds:?} to {actual_bounds:?}",
            path.display()
        ));
    }
    Ok(output)
}

pub(super) fn parse_md_palette(words: &[String], label: &str) -> Result<[u16; 16], String> {
    if words.len() != 16 {
        return Err(format!(
            "{label} palette declares {} colors, expected 16",
            words.len()
        ));
    }
    let mut palette = [0u16; 16];
    for (index, word) in words.iter().enumerate() {
        let value = super::parse_hex(word)?;
        if value > u16::MAX as usize || value & !0x0EEE != 0 {
            return Err(format!("invalid Mega Drive palette word {word}"));
        }
        palette[index] = value as u16;
    }
    Ok(palette)
}

pub(super) fn nearest_palette_index(
    color: [u8; 3],
    palette: &[u16; 16],
    allowed: &[usize],
    label: &str,
) -> Result<usize, String> {
    allowed
        .iter()
        .copied()
        .min_by_key(|&index| {
            let candidate = md_color(palette[index]);
            color
                .iter()
                .zip(candidate)
                .map(|(&left, right)| {
                    let delta = left as i32 - right as i32;
                    (delta * delta) as u32
                })
                .sum::<u32>()
        })
        .ok_or_else(|| format!("{label} has no admitted palette colors"))
}

pub(super) fn md_color(word: u16) -> [u8; 3] {
    [
        ((word & 0x000E) >> 1) as u8 * 36,
        ((word & 0x00E0) >> 5) as u8 * 36,
        ((word & 0x0E00) >> 9) as u8 * 36,
    ]
}

pub(super) fn fit_bounds_within(
    source: PixelBounds,
    target: PixelBounds,
    label: &str,
) -> Result<(usize, usize), String> {
    if source.width == 0 || source.height == 0 || target.width == 0 || target.height == 0 {
        return Err(format!("{label} fit bounds must be non-empty"));
    }
    let (width, height) = if source.width * target.height <= source.height * target.width {
        let height = target.height;
        let width = (source.width * height + source.height / 2) / source.height;
        (width, height)
    } else {
        let width = target.width;
        let height = (source.height * width + source.width / 2) / source.width;
        (width, height)
    };
    if width == 0 || height == 0 || width > target.width || height > target.height {
        return Err(format!("{label} master does not fit its content box"));
    }
    Ok((width, height))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn reduce_rgba_to_indexed_surface(
    master: &[u8],
    master_width: usize,
    master_height: usize,
    bounds: PixelBounds,
    surface_width: usize,
    surface_height: usize,
    content_box: PixelBounds,
    alpha_threshold: u8,
    transparent_palette_index: usize,
    palette: &[u16; 16],
    allowed_opaque_palette_indices: &[usize],
    label: &str,
) -> Result<Vec<u8>, String> {
    if master.len() != master_width * master_height * 4
        || content_box.x + content_box.width > surface_width
        || content_box.y + content_box.height > surface_height
        || alpha_threshold == 0
        || alpha_threshold == u8::MAX
        || transparent_palette_index >= 16
    {
        return Err(format!("{label} indexed surface contract is invalid"));
    }
    let (fit_width, fit_height) = fit_bounds_within(bounds, content_box, label)?;
    let target_x = content_box.x + (content_box.width - fit_width) / 2;
    let target_y = content_box.y + (content_box.height - fit_height) / 2;
    let mut output = vec![transparent_palette_index as u8; surface_width * surface_height];
    for output_y in 0..fit_height {
        for output_x in 0..fit_width {
            let source_x0 = bounds.x + output_x * bounds.width / fit_width;
            let source_x1 = bounds.x + (output_x + 1) * bounds.width / fit_width;
            let source_y0 = bounds.y + output_y * bounds.height / fit_height;
            let source_y1 = bounds.y + (output_y + 1) * bounds.height / fit_height;
            if source_x0 >= source_x1 || source_y0 >= source_y1 {
                return Err(format!("{label} area reduction produced an empty sample"));
            }
            let mut weighted_rgb = [0u64; 3];
            let mut alpha_sum = 0u64;
            let mut samples = 0u64;
            for source_y in source_y0..source_y1 {
                for source_x in source_x0..source_x1 {
                    let offset = (source_y * master_width + source_x) * 4;
                    let alpha = master[offset + 3] as u64;
                    alpha_sum += alpha;
                    for channel in 0..3 {
                        weighted_rgb[channel] += master[offset + channel] as u64 * alpha;
                    }
                    samples += 1;
                }
            }
            if alpha_sum < samples * alpha_threshold as u64 {
                continue;
            }
            let averaged = [
                ((weighted_rgb[0] + alpha_sum / 2) / alpha_sum) as u8,
                ((weighted_rgb[1] + alpha_sum / 2) / alpha_sum) as u8,
                ((weighted_rgb[2] + alpha_sum / 2) / alpha_sum) as u8,
            ];
            let palette_index =
                nearest_palette_index(averaged, palette, allowed_opaque_palette_indices, label)?;
            let destination_x = target_x + output_x;
            let destination_y = target_y + output_y;
            output[destination_y * surface_width + destination_x] = palette_index as u8;
        }
    }
    Ok(output)
}

pub(super) fn encode_md_tiles_column_major(
    pixels: &[u8],
    width: usize,
    height: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    if pixels.len() != width * height || !width.is_multiple_of(8) || !height.is_multiple_of(8) {
        return Err(format!("{label} pixel surface is not tile-aligned"));
    }
    let tile_columns = width / 8;
    let tile_rows = height / 8;
    let mut output = vec![0u8; tile_columns * tile_rows * MD_TILE_BYTES];
    for tile_x in 0..tile_columns {
        for tile_y in 0..tile_rows {
            let tile = tile_x * tile_rows + tile_y;
            let tile_offset = tile * MD_TILE_BYTES;
            for local_y in 0..8usize {
                for pair_x in 0..4usize {
                    let x = tile_x * 8 + pair_x * 2;
                    let y = tile_y * 8 + local_y;
                    let left = pixels[y * width + x];
                    let right = pixels[y * width + x + 1];
                    if left >= 16 || right >= 16 {
                        return Err(format!("{label} uses an invalid palette index"));
                    }
                    output[tile_offset + local_y * 4 + pair_x] = (left << 4) | right;
                }
            }
        }
    }
    Ok(output)
}

pub(super) fn decode_md_tiles_column_major(
    tiles: &[u8],
    width: usize,
    height: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    if !width.is_multiple_of(8) || !height.is_multiple_of(8) {
        return Err(format!("{label} pixel surface is not tile-aligned"));
    }
    let tile_columns = width / 8;
    let tile_rows = height / 8;
    if tiles.len() != tile_columns * tile_rows * MD_TILE_BYTES {
        return Err(format!("{label} tile payload length is invalid"));
    }
    let mut output = vec![0u8; width * height];
    for tile_x in 0..tile_columns {
        for tile_y in 0..tile_rows {
            let tile = tile_x * tile_rows + tile_y;
            let tile_offset = tile * MD_TILE_BYTES;
            for local_y in 0..8usize {
                for local_x in 0..8usize {
                    let byte = tiles[tile_offset + local_y * 4 + local_x / 2];
                    let pixel = if local_x.is_multiple_of(2) {
                        byte >> 4
                    } else {
                        byte & 0x0F
                    };
                    let x = tile_x * 8 + local_x;
                    let y = tile_y * 8 + local_y;
                    output[y * width + x] = pixel;
                }
            }
        }
    }
    Ok(output)
}

pub(super) fn native_glyph_pixel(glyph: &[u8; 32], x: usize, y: usize) -> bool {
    if x >= 16 || y >= 16 {
        return false;
    }
    let row = u16::from_be_bytes([glyph[y * 2], glyph[y * 2 + 1]]);
    row & (1 << (15 - x)) != 0
}

pub(super) fn nearest_core_distance(
    core: &[bool],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    max_distance: usize,
) -> Option<usize> {
    if core.len() != width * height || x >= width || y >= height {
        return None;
    }
    for distance in 1..=max_distance {
        let radius = distance as isize;
        for delta_y in -radius..=radius {
            for delta_x in -radius..=radius {
                if delta_x.abs().max(delta_y.abs()) != radius {
                    continue;
                }
                let candidate_x = x as isize + delta_x;
                let candidate_y = y as isize + delta_y;
                if candidate_x >= 0
                    && candidate_x < width as isize
                    && candidate_y >= 0
                    && candidate_y < height as isize
                    && core[candidate_y as usize * width + candidate_x as usize]
                {
                    return Some(distance);
                }
            }
        }
    }
    None
}

pub(super) fn write_rgba_png(
    output_path: &Path,
    width: u32,
    height: u32,
    rgba: &[u8],
    label: &str,
) -> Result<(), String> {
    if rgba.len() != width as usize * height as usize * 4 {
        return Err(format!("{label} RGBA output length is invalid"));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create {label} preview directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let file = fs::File::create(output_path).map_err(|error| {
        format!(
            "failed to create {label} preview {}: {error}",
            output_path.display()
        )
    })?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to write {label} PNG header: {error}"))?;
    writer
        .write_image_data(rgba)
        .map_err(|error| format!("failed to write {label} PNG pixels: {error}"))
}

fn detect_alpha_bounds(
    rgba: &[u8],
    width: usize,
    height: usize,
    label: &str,
) -> Result<PixelBounds, String> {
    if rgba.len() != width * height * 4 {
        return Err(format!("{label} RGBA buffer length is invalid"));
    }
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            if rgba[(y * width + x) * 4 + 3] == 0 {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if !found {
        return Err(format!("{label} has no visible pixels"));
    }
    Ok(PixelBounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    })
}
