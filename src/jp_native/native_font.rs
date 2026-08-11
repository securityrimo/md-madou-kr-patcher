//! JP-native glyph inventory, rasterization, and font-table addressing.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fontdue::{Font, FontSettings};

use super::{
    BYTES_PER_GLYPH, JP_HIGH_CODE_START, JP_KANA_FONT_BASE, JP_KANJI_FONT_BASE,
    JP_NATIVE_DEFERRED_GLYPHS, JP_NATIVE_HALF_WIDTH_CHARS, JP_NATIVE_RETIRED_GLYPHS,
    M2_KR_PUNCTUATION, M3_KR_EXTRA_GLYPHS,
};

const GRID_SIZE: usize = 16;
const LOWER_LEFT_PUNCT_X: usize = 2;
const LOWER_LEFT_PUNCT_BOTTOM_MARGIN: usize = 2;

pub(super) fn render_native_font(assets_dir: &Path, glyphs: &[char]) -> Result<Vec<u8>, String> {
    let ttf_data = fs::read(assets_dir.join("neodgm.ttf"))
        .map_err(|e| format!("failed to read Korean font: {e}"))?;
    let font = Font::from_bytes(ttf_data, FontSettings::default())
        .map_err(|e| format!("failed to parse Korean font: {e}"))?;
    let mut font_data = Vec::with_capacity(glyphs.len() * BYTES_PER_GLYPH);
    for &ch in glyphs {
        font_data.extend_from_slice(&render_native_glyph(&font, ch));
    }
    Ok(font_data)
}

pub(super) fn collect_jp_native_glyphs(assets_dir: &Path) -> Result<Vec<char>, String> {
    let texts = crate::translation::collect_all_kr_texts(&assets_dir.join("translation"))?;
    let mut glyphs = BTreeSet::from(JP_NATIVE_RETIRED_GLYPHS);
    for text in texts {
        for token in crate::build::text::parse_display_text(&text) {
            match token {
                crate::build::text::Token::KrChar(ch)
                    if !JP_NATIVE_DEFERRED_GLYPHS.contains(&ch) =>
                {
                    glyphs.insert(ch);
                }
                crate::build::text::Token::EnChar('~') => {
                    glyphs.insert('~');
                }
                _ => {}
            }
        }
    }
    Ok(glyphs.into_iter().collect())
}

pub(super) fn collect_m2_jp_native_glyphs(assets_dir: &Path) -> Result<Vec<char>, String> {
    let glyphs = collect_jp_native_glyphs(assets_dir)?;
    Ok(append_m2_kr_punctuation(glyphs))
}

pub(super) fn collect_m3_jp_native_glyphs(assets_dir: &Path) -> Result<Vec<char>, String> {
    let glyphs = collect_m2_jp_native_glyphs(assets_dir)?;
    Ok(append_m3_kr_glyphs(glyphs))
}

pub(super) fn append_m2_kr_punctuation(mut glyphs: Vec<char>) -> Vec<char> {
    for ch in M2_KR_PUNCTUATION {
        if !glyphs.contains(&ch) {
            glyphs.push(ch);
        }
    }
    glyphs
}

pub(super) fn append_m3_kr_glyphs(mut glyphs: Vec<char>) -> Vec<char> {
    for ch in M3_KR_EXTRA_GLYPHS {
        if !glyphs.contains(&ch) {
            glyphs.push(ch);
        }
    }
    glyphs
}

pub(super) fn append_scope_glyphs(mut glyphs: Vec<char>, extra_glyphs: &[char]) -> Vec<char> {
    for &ch in extra_glyphs {
        if !glyphs.contains(&ch) {
            glyphs.push(ch);
        }
    }
    glyphs
}

pub(super) fn append_half_space_glyph(mut glyphs: Vec<char>) -> Vec<char> {
    if !glyphs.contains(&' ') {
        glyphs.push(' ');
    }
    glyphs
}

pub(super) fn collect_scoped_jp_native_glyphs(
    assets_dir: &Path,
    include_item_names: bool,
    extra_glyphs: &[char],
) -> Result<Vec<char>, String> {
    let glyphs = if include_item_names {
        collect_m3_jp_native_glyphs(assets_dir)?
    } else {
        collect_m2_jp_native_glyphs(assets_dir)?
    };
    Ok(append_half_space_glyph(append_scope_glyphs(
        glyphs,
        extra_glyphs,
    )))
}

pub(super) fn jp_font_offset(code: u16) -> Result<usize, String> {
    if code < JP_HIGH_CODE_START {
        Ok(JP_KANA_FONT_BASE + code as usize * BYTES_PER_GLYPH)
    } else {
        let index = (code - JP_HIGH_CODE_START) as usize;
        let offset = JP_KANJI_FONT_BASE + index * BYTES_PER_GLYPH;
        if offset + BYTES_PER_GLYPH > JP_KANA_FONT_BASE {
            return Err(format!(
                "JP high-font code 0x{code:04X} exceeds the original table"
            ));
        }
        Ok(offset)
    }
}

pub(crate) fn render_native_glyph(font: &Font, ch: char) -> [u8; BYTES_PER_GLYPH] {
    if let Some(glyph) = render_heavy_lower_left_punctuation(ch) {
        return glyph;
    }
    if JP_NATIVE_HALF_WIDTH_CHARS.contains(&ch) {
        return render_half_width_native_glyph(font, ch);
    }

    let (metrics, coverage) = font.rasterize(ch, GRID_SIZE as f32);
    let mut pixels = [[false; GRID_SIZE]; GRID_SIZE];
    let (x_offset, y_offset) = native_glyph_canvas_offset(ch, metrics.width, metrics.height);

    for y in 0..metrics.height {
        for x in 0..metrics.width {
            let dst_y = y_offset + y;
            let dst_x = x_offset + x;
            if dst_y < GRID_SIZE && dst_x < GRID_SIZE && coverage[y * metrics.width + x] > 127 {
                pixels[dst_y][dst_x] = true;
            }
        }
    }

    pack_native_glyph(&pixels)
}

pub(super) fn render_heavy_lower_left_punctuation(ch: char) -> Option<[u8; BYTES_PER_GLYPH]> {
    let dot_starts: &[usize] = match ch {
        '.' | '．' => &[LOWER_LEFT_PUNCT_X],
        '…' => &[
            LOWER_LEFT_PUNCT_X,
            LOWER_LEFT_PUNCT_X + 5,
            LOWER_LEFT_PUNCT_X + 10,
        ],
        _ => return None,
    };
    let mut pixels = [[false; GRID_SIZE]; GRID_SIZE];
    let top = GRID_SIZE - LOWER_LEFT_PUNCT_BOTTOM_MARGIN - 3;
    for &left in dot_starts {
        for row in pixels.iter_mut().skip(top).take(3) {
            for pixel in row.iter_mut().skip(left).take(3) {
                *pixel = true;
            }
        }
    }
    Some(pack_native_glyph(&pixels))
}

fn render_half_width_native_glyph(font: &Font, ch: char) -> [u8; BYTES_PER_GLYPH] {
    let mut pixels = [[false; GRID_SIZE]; GRID_SIZE];
    if ch == ' ' {
        return pack_native_glyph(&pixels);
    }

    let (metrics, coverage) = font.rasterize(ch, GRID_SIZE as f32);
    if metrics.width == 0 || metrics.height == 0 {
        return pack_native_glyph(&pixels);
    }

    let scale_x = (8.0 / metrics.width as f32).min(1.0);
    let scale_y = (GRID_SIZE as f32 / metrics.height as f32).min(1.0);
    let scaled_width = ((metrics.width as f32 * scale_x).ceil() as usize).clamp(1, 8);
    let scaled_height = ((metrics.height as f32 * scale_y).ceil() as usize).clamp(1, GRID_SIZE);
    let x_offset = if ch == ',' {
        LOWER_LEFT_PUNCT_X.min(8usize.saturating_sub(scaled_width))
    } else {
        (8 - scaled_width) / 2
    };
    let y_offset = if ch == ',' {
        GRID_SIZE
            .saturating_sub(scaled_height)
            .saturating_sub(LOWER_LEFT_PUNCT_BOTTOM_MARGIN)
    } else {
        (GRID_SIZE - scaled_height) / 2
    };

    for source_y in 0..metrics.height {
        for source_x in 0..metrics.width {
            if coverage[source_y * metrics.width + source_x] <= 127 {
                continue;
            }
            let x = x_offset + ((source_x as f32 * scale_x).floor() as usize).min(scaled_width - 1);
            let y =
                y_offset + ((source_y as f32 * scale_y).floor() as usize).min(scaled_height - 1);
            pixels[y][x] = true;
        }
    }

    pack_native_glyph(&pixels)
}

fn pack_native_glyph(pixels: &[[bool; GRID_SIZE]; GRID_SIZE]) -> [u8; BYTES_PER_GLYPH] {
    let mut packed = [0u8; BYTES_PER_GLYPH];
    for (y, pixel_row) in pixels.iter().enumerate() {
        let mut row = 0u16;
        for (x, filled) in pixel_row.iter().enumerate() {
            if *filled {
                row |= 1 << (15 - x);
            }
        }
        let [high, low] = row.to_be_bytes();
        packed[y * 2] = high;
        packed[y * 2 + 1] = low;
    }
    packed
}

pub(super) fn native_glyph_canvas_offset(ch: char, width: usize, height: usize) -> (usize, usize) {
    if matches!(ch, ',' | '.' | '…' | '，' | '．' | '、' | '。') {
        return (
            LOWER_LEFT_PUNCT_X.min(GRID_SIZE.saturating_sub(width)),
            GRID_SIZE
                .saturating_sub(height)
                .saturating_sub(LOWER_LEFT_PUNCT_BOTTOM_MARGIN),
        );
    }

    (
        ((GRID_SIZE as isize - width as isize) / 2).max(0) as usize,
        ((GRID_SIZE as isize - height as isize) / 2).max(0) as usize,
    )
}
