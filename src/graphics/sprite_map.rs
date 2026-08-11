//! Checked helpers for original Mega Drive sprite-frame data.
//!
//! Credit objects use a table of big-endian frame offsets followed by counted
//! eight-byte sprite records.  This module keeps that non-executable format
//! explicit so page compilers can preserve unrelated frames, rebuild only
//! declared frames, and render static source/target proof from the same bytes
//! that the game consumes.

use super::{MD_TILE_BYTES, source_range};

const TILE_INDEX_MASK: u16 = 0x07FF;
const H_FLIP: u16 = 0x0800;
const V_FLIP: u16 = 0x1000;
const MAX_VRAM_TILES: usize = 0x800;
const SPRITE_RECORD_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpriteRecord {
    pub(super) y: u16,
    pub(super) size_and_link: u16,
    pub(super) tile_and_attributes: u16,
    pub(super) x: u16,
}

impl SpriteRecord {
    pub(super) fn width_tiles(&self) -> usize {
        usize::from((self.size_and_link >> 10) & 0x03) + 1
    }

    pub(super) fn height_tiles(&self) -> usize {
        usize::from((self.size_and_link >> 8) & 0x03) + 1
    }

    pub(super) fn tile_index(&self) -> usize {
        usize::from(self.tile_and_attributes & TILE_INDEX_MASK)
    }

    pub(super) fn encode(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.y.to_be_bytes());
        output.extend_from_slice(&self.size_and_link.to_be_bytes());
        output.extend_from_slice(&self.tile_and_attributes.to_be_bytes());
        output.extend_from_slice(&self.x.to_be_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpriteFrame {
    pub(super) records: Vec<SpriteRecord>,
}

impl SpriteFrame {
    pub(super) fn encode(&self) -> Result<Vec<u8>, String> {
        let count = u16::try_from(self.records.len())
            .map_err(|_| "sprite frame has more than 65535 records".to_string())?;
        let mut output = Vec::with_capacity(2 + self.records.len() * SPRITE_RECORD_BYTES);
        output.extend_from_slice(&count.to_be_bytes());
        for record in &self.records {
            record.encode(&mut output);
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FrameSurface {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) pixels: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(super) struct VirtualVram {
    tiles: Vec<Option<[u8; MD_TILE_BYTES]>>,
}

impl VirtualVram {
    pub(super) fn new() -> Self {
        Self {
            tiles: vec![None; MAX_VRAM_TILES],
        }
    }

    pub(super) fn load(
        &mut self,
        destination: u16,
        payload: &[u8],
        label: &str,
    ) -> Result<(), String> {
        if !usize::from(destination).is_multiple_of(MD_TILE_BYTES)
            || !payload.len().is_multiple_of(MD_TILE_BYTES)
        {
            return Err(format!("{label} VRAM transfer is not tile-aligned"));
        }
        let first_tile = usize::from(destination) / MD_TILE_BYTES;
        let tile_count = payload.len() / MD_TILE_BYTES;
        if first_tile + tile_count > self.tiles.len() {
            return Err(format!("{label} VRAM transfer exceeds pattern memory"));
        }
        for (index, tile) in payload.chunks_exact(MD_TILE_BYTES).enumerate() {
            let mut bytes = [0u8; MD_TILE_BYTES];
            bytes.copy_from_slice(tile);
            self.tiles[first_tile + index] = Some(bytes);
        }
        Ok(())
    }

    fn tile(&self, index: usize, label: &str) -> Result<&[u8; MD_TILE_BYTES], String> {
        self.tiles
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| format!("{label} references unloaded VRAM tile 0x{index:03X}"))
    }
}

pub(super) fn parse_frame_table(
    table: &[u8],
    frame_count: usize,
    label: &str,
) -> Result<Vec<SpriteFrame>, String> {
    let header_bytes = frame_count
        .checked_mul(2)
        .ok_or_else(|| format!("{label} frame-offset table overflowed"))?;
    let header = source_range(table, 0, header_bytes, "sprite frame-offset table")?;
    let offsets = header
        .chunks_exact(2)
        .map(|pair| usize::from(u16::from_be_bytes([pair[0], pair[1]])))
        .collect::<Vec<_>>();
    if offsets.first().copied() != Some(header_bytes)
        || offsets.iter().any(|&offset| offset < header_bytes)
        || offsets.windows(2).any(|pair| pair[0] > pair[1])
        || offsets.last().is_none_or(|&offset| offset >= table.len())
    {
        return Err(format!("{label} frame offsets are not a strict table"));
    }

    let mut frames = Vec::with_capacity(frame_count);
    for index in 0..frame_count {
        let start = offsets[index];
        let end = offsets.get(index + 1).copied().unwrap_or(table.len());
        let frame = source_range(table, start, end - start, &format!("{label} frame {index}"))?;
        if frame.is_empty() {
            frames.push(SpriteFrame {
                records: Vec::new(),
            });
            continue;
        }
        if frame.len() < 2 {
            return Err(format!("{label} frame {index} has no record count"));
        }
        let count = usize::from(u16::from_be_bytes([frame[0], frame[1]]));
        if frame.len() != 2 + count * SPRITE_RECORD_BYTES {
            return Err(format!(
                "{label} frame {index} has {} bytes for {count} records",
                frame.len()
            ));
        }
        let records = frame[2..]
            .chunks_exact(SPRITE_RECORD_BYTES)
            .map(|record| SpriteRecord {
                y: u16::from_be_bytes([record[0], record[1]]),
                size_and_link: u16::from_be_bytes([record[2], record[3]]),
                tile_and_attributes: u16::from_be_bytes([record[4], record[5]]),
                x: u16::from_be_bytes([record[6], record[7]]),
            })
            .collect();
        frames.push(SpriteFrame { records });
    }
    Ok(frames)
}

pub(super) fn render_frame(
    vram: &VirtualVram,
    frame: &SpriteFrame,
    label: &str,
) -> Result<FrameSurface, String> {
    if frame.records.is_empty() {
        return Ok(FrameSurface {
            width: 8,
            height: 8,
            pixels: vec![0; 8 * 8],
        });
    }
    let min_x = frame
        .records
        .iter()
        .map(|record| usize::from(record.x))
        .min()
        .unwrap();
    let min_y = frame
        .records
        .iter()
        .map(|record| usize::from(record.y))
        .min()
        .unwrap();
    let max_x = frame
        .records
        .iter()
        .map(|record| usize::from(record.x) + record.width_tiles() * 8)
        .max()
        .unwrap();
    let max_y = frame
        .records
        .iter()
        .map(|record| usize::from(record.y) + record.height_tiles() * 8)
        .max()
        .unwrap();
    let width = max_x - min_x;
    let height = max_y - min_y;
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return Err(format!("{label} frame bounds are invalid"));
    }
    let mut pixels = vec![0u8; width * height];
    for record in &frame.records {
        let width_tiles = record.width_tiles();
        let height_tiles = record.height_tiles();
        let flip_x = record.tile_and_attributes & H_FLIP != 0;
        let flip_y = record.tile_and_attributes & V_FLIP != 0;
        for tile_x in 0..width_tiles {
            for tile_y in 0..height_tiles {
                let source_tile_x = if flip_x {
                    width_tiles - 1 - tile_x
                } else {
                    tile_x
                };
                let source_tile_y = if flip_y {
                    height_tiles - 1 - tile_y
                } else {
                    tile_y
                };
                let tile_index = record.tile_index() + source_tile_x * height_tiles + source_tile_y;
                let tile = vram.tile(tile_index, label)?;
                for local_y in 0..8 {
                    for local_x in 0..8 {
                        let source_x = if flip_x { 7 - local_x } else { local_x };
                        let source_y = if flip_y { 7 - local_y } else { local_y };
                        let byte = tile[source_y * 4 + source_x / 2];
                        let pixel = if source_x.is_multiple_of(2) {
                            byte >> 4
                        } else {
                            byte & 0x0F
                        };
                        if pixel == 0 {
                            continue;
                        }
                        let x = usize::from(record.x) - min_x + tile_x * 8 + local_x;
                        let y = usize::from(record.y) - min_y + tile_y * 8 + local_y;
                        pixels[y * width + x] = pixel;
                    }
                }
            }
        }
    }
    Ok(FrameSurface {
        width,
        height,
        pixels,
    })
}
