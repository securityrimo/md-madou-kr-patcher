//! Shared source-owned font helpers for compact battle-effect lettering.

use std::fs;
use std::path::Path;

use fontdue::{Font, FontSettings};

use crate::jp_native;

use super::pixel::{native_glyph_pixel, nearest_core_distance};
use super::sha256_hex;

/// Render a repository-font glyph into the JP-native one-column 8x16 budget.
///
/// Wider source glyphs are reduced by coverage rather than rasterized at an
/// unreadably small font size.  Callers choose the original surface's palette
/// polarity so no rectangular cell background is introduced.
pub(super) fn render_compact_8x16_glyph(
    font: &Font,
    ch: char,
    font_size_px: f32,
    coverage_threshold: u8,
    background_index: u8,
    ink_index: u8,
    label: &str,
) -> Result<[u8; 8 * 16], String> {
    if background_index >= 16 || ink_index >= 16 || background_index == ink_index {
        return Err(format!("{label} uses invalid compact-glyph palette roles"));
    }
    let (metrics, coverage) = font.rasterize(ch, font_size_px);
    if metrics.width == 0 || metrics.height == 0 || metrics.width > 16 || metrics.height > 16 {
        return Err(format!(
            "{label} glyph {ch:?} is {}x{}, outside its reducible 16x16 source cell",
            metrics.width, metrics.height
        ));
    }

    let origin_y = (16 - metrics.height) / 2;
    let mut surface = [background_index; 8 * 16];
    for y in 0..metrics.height {
        if metrics.width <= 8 {
            let origin_x = (8 - metrics.width) / 2;
            for x in 0..metrics.width {
                if coverage[y * metrics.width + x] > coverage_threshold {
                    surface[(origin_y + y) * 8 + origin_x + x] = ink_index;
                }
            }
        } else {
            for target_x in 0..8usize {
                let source_start = target_x * metrics.width / 8;
                let source_end = ((target_x + 1) * metrics.width).div_ceil(8);
                if coverage[y * metrics.width + source_start..y * metrics.width + source_end]
                    .iter()
                    .any(|&value| value > coverage_threshold)
                {
                    surface[(origin_y + y) * 8 + target_x] = ink_index;
                }
            }
        }
    }
    if !surface.contains(&ink_index) {
        return Err(format!("{label} glyph {ch:?} rendered blank"));
    }
    Ok(surface)
}

pub(super) fn read_verified_font(
    assets_dir: &Path,
    relative_path: &str,
    expected_sha256: &str,
    label: &str,
) -> Result<Font, String> {
    let path = assets_dir.join(relative_path);
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let actual = sha256_hex(&bytes);
    if actual != expected_sha256 {
        return Err(format!(
            "{}: {label} font SHA-256 mismatch: expected {expected_sha256}, got {actual}",
            path.display()
        ));
    }
    Font::from_bytes(bytes, FontSettings::default())
        .map_err(|error| format!("failed to parse {label} font: {error}"))
}

pub(super) const NATIVE_GLYPH_SIZE: usize = 16;

/// Literal white used when a static proof visualizes the ending-credit ink
/// role. The ROM compiler still writes palette index 15 and preserves the JP
/// sprite palette selection; this colour is only the proof-image mapping.
pub(super) const CREDIT_PREVIEW_INK: [u8; 4] = [255, 255, 255, 255];

/// Return the exact horizontal budget for an unscaled native-font line.
///
/// Every visible glyph keeps its 16x16 NeoDGM cell. Whitespace may use the
/// surface-specific half-cell advance supplied by the caller.
pub(super) fn native_text_width(text: &str, space_width: usize) -> usize {
    text.chars()
        .map(|character| {
            if character.is_whitespace() {
                space_width
            } else {
                NATIVE_GLYPH_SIZE
            }
        })
        .sum()
}

/// Render a centered, unscaled NeoDGM line with native JP-engine glyph cells.
pub(super) fn render_native_text_line(
    font: &Font,
    text: &str,
    surface_width: usize,
    space_width: usize,
    background_index: u8,
    ink_index: u8,
    label: &str,
) -> Result<Vec<u8>, String> {
    if surface_width == 0
        || space_width == 0
        || space_width > NATIVE_GLYPH_SIZE
        || background_index >= 16
        || ink_index >= 16
        || background_index == ink_index
    {
        return Err(format!("{label} uses an invalid native-text layout"));
    }
    let text_width = native_text_width(text, space_width);
    if text_width > surface_width {
        return Err(format!(
            "{label} needs {text_width} pixels at native 16x16 width, but only {surface_width} are available"
        ));
    }

    let mut surface = vec![background_index; surface_width * NATIVE_GLYPH_SIZE];
    let mut cursor_x = (surface_width - text_width) / 2;
    for character in text.chars() {
        if character.is_whitespace() {
            cursor_x += space_width;
            continue;
        }
        let glyph = jp_native::render_native_glyph(font, character);
        let mut rendered = false;
        for y in 0..NATIVE_GLYPH_SIZE {
            for x in 0..NATIVE_GLYPH_SIZE {
                if native_glyph_pixel(&glyph, x, y) {
                    surface[y * surface_width + cursor_x + x] = ink_index;
                    rendered = true;
                }
            }
        }
        if !rendered {
            return Err(format!(
                "{label} glyph {character:?} rendered blank in the verified repository font"
            ));
        }
        cursor_x += NATIVE_GLYPH_SIZE;
    }
    Ok(surface)
}

/// Render one native 16x16 glyph with clipped inner and outer outlines.
///
/// Clipping each glyph to its own cell is required for effects whose letters
/// are animated as independent sprites.
pub(super) fn render_indexed_glyph(
    font: &Font,
    ch: char,
    transparent: u8,
    core_index: u8,
    inner_outline_index: u8,
    outer_outline_index: u8,
) -> [u8; 16 * 16] {
    let glyph = jp_native::render_native_glyph(font, ch);
    let mut core = [false; 16 * 16];
    for y in 0..16 {
        for x in 0..16 {
            core[y * 16 + x] = native_glyph_pixel(&glyph, x, y);
        }
    }

    let mut output = [transparent; 16 * 16];
    for y in 0..16 {
        for x in 0..16 {
            output[y * 16 + x] = if core[y * 16 + x] {
                core_index
            } else {
                match nearest_core_distance(&core, 16, 16, x, y, 2) {
                    Some(1) => inner_outline_index,
                    Some(2) => outer_outline_index,
                    _ => transparent,
                }
            };
        }
    }
    output
}

pub(super) fn blit_indexed_glyph(
    surface: &mut [u8],
    surface_width: usize,
    surface_height: usize,
    x: usize,
    y: usize,
    glyph: &[u8; 16 * 16],
) -> Result<(), String> {
    if surface.len() != surface_width * surface_height
        || x + 16 > surface_width
        || y + 16 > surface_height
    {
        return Err("indexed glyph destination is outside its surface".to_string());
    }
    for glyph_y in 0..16 {
        let source = &glyph[glyph_y * 16..(glyph_y + 1) * 16];
        let destination = (y + glyph_y) * surface_width + x;
        surface[destination..destination + 16].copy_from_slice(source);
    }
    Ok(())
}
