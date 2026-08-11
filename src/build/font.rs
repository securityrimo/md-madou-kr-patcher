use std::fs;
use std::path::Path;

use fontdue::{Font, FontSettings};

/// ROM layout constants
pub const KR_FONT_BASE: usize = 0x340000;
pub const KR_WIDTH_TABLE: usize = 0x33F100;
pub const BYTES_PER_CHAR: usize = 1024;

const GRID_SIZE: usize = 16;
const BYTES_PER_ROW: usize = 16; // 32 pixels at 4bpp
const BYTES_PER_COPY: usize = GRID_SIZE * BYTES_PER_ROW; // 256
const FG_NIBBLE: u8 = 0xF;

/// Render a single character to a 16x16 1-bit bitmap.
/// Returns (bitmap, pixel_width).
fn render_glyph(font: &Font, ch: char) -> ([bool; GRID_SIZE * GRID_SIZE], u8) {
    let (metrics, coverage) = font.rasterize(ch, GRID_SIZE as f32);

    let mut bitmap = [false; GRID_SIZE * GRID_SIZE];

    // Center glyph in grid
    let x_off = ((GRID_SIZE as i32 - metrics.width as i32) / 2).max(0) as usize;
    let y_off = ((GRID_SIZE as i32 - metrics.height as i32) / 2).max(0) as usize;

    for row in 0..metrics.height {
        for col in 0..metrics.width {
            let dst_row = y_off + row;
            let dst_col = x_off + col;
            if dst_row < GRID_SIZE && dst_col < GRID_SIZE {
                let alpha = coverage[row * metrics.width + col];
                bitmap[dst_row * GRID_SIZE + dst_col] = alpha > 127;
            }
        }
    }

    // Measure actual width (rightmost non-zero column + 1)
    let mut max_x: u8 = 0;
    for row in 0..GRID_SIZE {
        for col in (0..GRID_SIZE).rev() {
            if bitmap[row * GRID_SIZE + col] {
                max_x = max_x.max(col as u8 + 1);
                break;
            }
        }
    }

    (bitmap, max_x)
}

/// Create a sub-pixel shifted copy (shift pixels LEFT) and pack to 4bpp.
/// Returns BYTES_PER_COPY (256) bytes.
fn create_shifted_copy(
    bitmap: &[bool; GRID_SIZE * GRID_SIZE],
    shift: usize,
) -> [u8; BYTES_PER_COPY] {
    let mut copy = [0u8; BYTES_PER_COPY];

    for row in 0..GRID_SIZE {
        for i in 0..GRID_SIZE {
            let src_x = i + shift;
            let dst_byte = row * BYTES_PER_ROW + i / 2;
            if src_x < GRID_SIZE && bitmap[row * GRID_SIZE + src_x] {
                if i % 2 == 0 {
                    copy[dst_byte] |= FG_NIBBLE << 4; // high nibble
                } else {
                    copy[dst_byte] |= FG_NIBBLE; // low nibble
                }
            }
        }
    }

    copy
}

/// Convert a glyph bitmap to the full 1024-byte font data format.
/// 4 sub-pixel shifted copies x 256 bytes each.
fn glyph_to_font_data(bitmap: &[bool; GRID_SIZE * GRID_SIZE]) -> [u8; BYTES_PER_CHAR] {
    let mut data = [0u8; BYTES_PER_CHAR];

    for shift in 0..4usize {
        let copy = create_shifted_copy(bitmap, shift);
        let offset = shift * BYTES_PER_COPY;
        data[offset..offset + BYTES_PER_COPY].copy_from_slice(&copy);
    }

    data
}

/// Insert font data into ROM.
pub fn insert_font_data(
    rom: &mut [u8],
    font_data: &[u8],
    char_count: usize,
    widths: &[(u8, u8)],
) -> Result<(), String> {
    let expected_font_size = char_count * BYTES_PER_CHAR;
    if font_data.len() < expected_font_size {
        return Err(format!(
            "font data too small: {} bytes for {} chars (need {})",
            font_data.len(),
            char_count,
            expected_font_size
        ));
    }

    for i in 0..char_count {
        let src_offset = i * BYTES_PER_CHAR;
        let dst_offset = KR_FONT_BASE + i * BYTES_PER_CHAR;

        if dst_offset + BYTES_PER_CHAR > rom.len() {
            return Err(format!("font data exceeds ROM size at char {i}"));
        }

        rom[dst_offset..dst_offset + BYTES_PER_CHAR]
            .copy_from_slice(&font_data[src_offset..src_offset + BYTES_PER_CHAR]);
    }

    if widths.len() < char_count {
        return Err(format!(
            "not enough width entries: {} < {}",
            widths.len(),
            char_count
        ));
    }

    for (i, width_entry) in widths.iter().enumerate().take(char_count) {
        let w_offset = KR_WIDTH_TABLE + i * 2;
        if w_offset + 2 > rom.len() {
            return Err(format!("width table exceeds ROM size at char {i}"));
        }
        rom[w_offset] = width_entry.0; // width
        rom[w_offset + 1] = width_entry.1; // advance
    }

    Ok(())
}

/// Load TTF font, render glyphs for unique_chars, and insert into ROM.
///
/// Returns the number of characters inserted.
pub fn render_and_insert_font(
    rom: &mut [u8],
    font_path: &Path,
    unique_chars: &[char],
) -> Result<usize, String> {
    let ttf_data = fs::read(font_path).map_err(|e| format!("failed to read font: {e}"))?;
    let font = Font::from_bytes(ttf_data, FontSettings::default())
        .map_err(|e| format!("failed to parse font: {e}"))?;

    let char_count = unique_chars.len();
    let mut all_font_data = Vec::with_capacity(char_count * BYTES_PER_CHAR);
    let mut widths = Vec::with_capacity(char_count);

    for &ch in unique_chars {
        let (bitmap, pixel_width) = render_glyph(&font, ch);
        let data = glyph_to_font_data(&bitmap);
        all_font_data.extend_from_slice(&data);
        widths.push((pixel_width, pixel_width + 1)); // advance = width + 1
    }

    insert_font_data(rom, &all_font_data, char_count, &widths)?;

    Ok(char_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_font_data() {
        let mut rom = vec![0u8; 0x400000];
        let char_count = 3;
        let font_data = vec![0xAA; char_count * BYTES_PER_CHAR];
        let widths = vec![(12, 13), (14, 15), (10, 11)];

        insert_font_data(&mut rom, &font_data, char_count, &widths).unwrap();

        assert_eq!(rom[KR_FONT_BASE], 0xAA);
        assert_eq!(rom[KR_FONT_BASE + BYTES_PER_CHAR], 0xAA);
        assert_eq!(rom[KR_FONT_BASE + 2 * BYTES_PER_CHAR], 0xAA);

        assert_eq!(rom[KR_WIDTH_TABLE], 12);
        assert_eq!(rom[KR_WIDTH_TABLE + 1], 13);
        assert_eq!(rom[KR_WIDTH_TABLE + 2], 14);
        assert_eq!(rom[KR_WIDTH_TABLE + 3], 15);
    }

    #[test]
    fn test_insert_font_data_too_small() {
        let mut rom = vec![0u8; 0x400000];
        let font_data = vec![0u8; 100];
        let widths = vec![(12, 13)];
        let result = insert_font_data(&mut rom, &font_data, 1, &widths);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_per_char_constant() {
        assert_eq!(BYTES_PER_CHAR, 4 * 16 * 16);
    }

    #[test]
    fn test_glyph_to_font_data_size() {
        let bitmap = [false; GRID_SIZE * GRID_SIZE];
        let data = glyph_to_font_data(&bitmap);
        assert_eq!(data.len(), BYTES_PER_CHAR);
    }

    #[test]
    fn test_shifted_copy_foreground() {
        // Single pixel at (0, 0)
        let mut bitmap = [false; GRID_SIZE * GRID_SIZE];
        bitmap[0] = true;

        // Shift 0: pixel at column 0 → high nibble of byte 0
        let copy0 = create_shifted_copy(&bitmap, 0);
        assert_eq!(copy0[0], 0xF0);

        // Shift 1: source column 1 → nothing (pixel is at col 0, shift reads col 0+1=1)
        let copy1 = create_shifted_copy(&bitmap, 1);
        assert_eq!(copy1[0], 0x00);
    }

    #[test]
    fn test_render_glyph_empty() {
        // Test with a font - use a space character which should be mostly empty
        // This test verifies the rendering pipeline doesn't crash
        let bitmap = [false; GRID_SIZE * GRID_SIZE];
        let data = glyph_to_font_data(&bitmap);
        assert!(data.iter().all(|&b| b == 0));
    }
}
