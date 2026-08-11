//! Verified bitmap-font loading shared by graphics compilers.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::sha256_hex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BdfGlyph {
    pub device_width: [i32; 2],
    pub width: usize,
    pub height: usize,
    pub x_offset: i32,
    pub y_offset: i32,
    pub pixels: Vec<bool>,
}

#[derive(Debug, Default)]
struct BdfGlyphRecord {
    encoding: Option<u32>,
    device_width: Option<[i32; 2]>,
    bounding_box: Option<[i32; 4]>,
    bitmap: Vec<String>,
    reading_bitmap: bool,
}

pub(super) fn read_verified_bdf_glyphs(
    assets_dir: &Path,
    font_asset: &str,
    expected_sha256: &str,
    expected_family: &str,
    expected_version: &str,
    required: &BTreeSet<char>,
    label: &str,
) -> Result<BTreeMap<char, BdfGlyph>, String> {
    let path = assets_dir.join(font_asset);
    let bytes = fs::read(&path)
        .map_err(|error| format!("failed to read {label} BDF {}: {error}", path.display()))?;
    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != expected_sha256 {
        return Err(format!(
            "{label} BDF {} SHA-256 mismatch: expected {expected_sha256}, got {actual_sha256}",
            path.display()
        ));
    }
    let source = std::str::from_utf8(&bytes)
        .map_err(|error| format!("{label} BDF {} is not UTF-8 text: {error}", path.display()))?;
    let family_property = format!("FAMILY_NAME \"{expected_family}\"");
    let version_property = format!("FONT_VERSION \"{expected_version}\"");
    if !source.lines().any(|line| line == family_property)
        || !source.lines().any(|line| line == version_property)
    {
        return Err(format!(
            "{label} BDF {} family or version differs from the manifest",
            path.display()
        ));
    }

    let required_by_codepoint: BTreeMap<_, _> = required
        .iter()
        .copied()
        .map(|character| (u32::from(character), character))
        .collect();
    let mut glyphs = BTreeMap::new();
    let mut current: Option<BdfGlyphRecord> = None;
    for line in source.lines() {
        if line.starts_with("STARTCHAR ") {
            if current.is_some() {
                return Err(format!("{label} BDF has nested glyph records"));
            }
            current = Some(BdfGlyphRecord::default());
            continue;
        }
        let Some(record) = current.as_mut() else {
            continue;
        };
        if let Some(value) = line.strip_prefix("ENCODING ") {
            let encoding = parse_bdf_i32(value, "ENCODING", label)?;
            record.encoding = u32::try_from(encoding).ok();
        } else if let Some(value) = line.strip_prefix("DWIDTH ") {
            record.device_width = Some(parse_bdf_pair(value, "DWIDTH", label)?);
        } else if let Some(value) = line.strip_prefix("BBX ") {
            record.bounding_box = Some(parse_bdf_quad(value, "BBX", label)?);
        } else if line == "BITMAP" {
            record.reading_bitmap = true;
        } else if line == "ENDCHAR" {
            let record = current
                .take()
                .ok_or_else(|| format!("{label} BDF lost its glyph record"))?;
            if let Some(&character) = record
                .encoding
                .as_ref()
                .and_then(|encoding| required_by_codepoint.get(encoding))
            {
                let glyph = decode_bdf_glyph(character, &record, label)?;
                if glyphs.insert(character, glyph).is_some() {
                    return Err(format!(
                        "{label} BDF duplicates U+{:04X}",
                        u32::from(character)
                    ));
                }
            }
        } else if record.reading_bitmap {
            record.bitmap.push(line.to_string());
        }
    }
    if current.is_some() {
        return Err(format!("{label} BDF ends inside a glyph record"));
    }
    let actual: BTreeSet<_> = glyphs.keys().copied().collect();
    if actual != *required {
        let missing: String = required.difference(&actual).collect();
        return Err(format!(
            "{label} BDF is missing required glyphs {missing:?}"
        ));
    }
    Ok(glyphs)
}

fn decode_bdf_glyph(
    character: char,
    record: &BdfGlyphRecord,
    label: &str,
) -> Result<BdfGlyph, String> {
    let device_width = record
        .device_width
        .ok_or_else(|| format!("{label} BDF U+{:04X} has no DWIDTH", u32::from(character)))?;
    let [width, height, x_offset, y_offset] = record
        .bounding_box
        .ok_or_else(|| format!("{label} BDF U+{:04X} has no BBX", u32::from(character)))?;
    if width <= 0 || height <= 0 || width > 8 || record.bitmap.len() != height as usize {
        return Err(format!(
            "{label} BDF U+{:04X} has unsupported BBX or bitmap height",
            u32::from(character)
        ));
    }

    let width = width as usize;
    let height = height as usize;
    let unused_bits = 8 - width;
    let unused_mask = if unused_bits == 0 {
        0
    } else {
        (1u16 << unused_bits) - 1
    };
    let mut pixels = vec![false; width * height];
    for (y, encoded) in record.bitmap.iter().enumerate() {
        if encoded.len() != 2 || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "{label} BDF U+{:04X} has malformed bitmap row {encoded:?}",
                u32::from(character)
            ));
        }
        let row = u8::from_str_radix(encoded, 16).map_err(|error| {
            format!(
                "{label} BDF U+{:04X} has invalid bitmap row {encoded:?}: {error}",
                u32::from(character)
            )
        })?;
        if u16::from(row) & unused_mask != 0 {
            return Err(format!(
                "{label} BDF U+{:04X} has pixels outside its BBX width",
                u32::from(character)
            ));
        }
        for x in 0..width {
            pixels[y * width + x] = row & (1 << (7 - x)) != 0;
        }
    }
    Ok(BdfGlyph {
        device_width,
        width,
        height,
        x_offset,
        y_offset,
        pixels,
    })
}

fn parse_bdf_i32(value: &str, field: &str, label: &str) -> Result<i32, String> {
    value
        .parse()
        .map_err(|error| format!("invalid {label} BDF {field} {value:?}: {error}"))
}

fn parse_bdf_pair(value: &str, field: &str, label: &str) -> Result<[i32; 2], String> {
    let values: Vec<_> = value.split_ascii_whitespace().collect();
    if values.len() != 2 {
        return Err(format!("invalid {label} BDF {field} {value:?}"));
    }
    Ok([
        parse_bdf_i32(values[0], field, label)?,
        parse_bdf_i32(values[1], field, label)?,
    ])
}

fn parse_bdf_quad(value: &str, field: &str, label: &str) -> Result<[i32; 4], String> {
    let values: Vec<_> = value.split_ascii_whitespace().collect();
    if values.len() != 4 {
        return Err(format!("invalid {label} BDF {field} {value:?}"));
    }
    Ok([
        parse_bdf_i32(values[0], field, label)?,
        parse_bdf_i32(values[1], field, label)?,
        parse_bdf_i32(values[2], field, label)?,
        parse_bdf_i32(values[3], field, label)?,
    ])
}
