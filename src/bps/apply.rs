use crate::bps::vli;

const BPS_MAGIC: &[u8; 4] = b"BPS1";

pub fn apply(source: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    if patch.len() < 16 {
        return Err("patch too small".into());
    }
    if &patch[..4] != BPS_MAGIC {
        return Err("not a BPS patch (invalid magic)".into());
    }

    // Verify patch CRC
    let patch_body = &patch[..patch.len() - 4];
    let stored_patch_crc = u32::from_le_bytes(patch[patch.len() - 4..].try_into().unwrap());
    let actual_patch_crc = crc32fast::hash(patch_body);
    if stored_patch_crc != actual_patch_crc {
        return Err(format!(
            "patch CRC mismatch (stored: {stored_patch_crc:08X}, actual: {actual_patch_crc:08X})"
        ));
    }

    // Parse header
    let mut pos = 4;
    let (source_size, n) = vli::decode(&patch[pos..]).map_err(|e| e.to_string())?;
    pos += n;
    let (target_size, n) = vli::decode(&patch[pos..]).map_err(|e| e.to_string())?;
    pos += n;
    if target_size > 64 * 1024 * 1024 {
        return Err(format!("target size too large: {}", target_size));
    }
    let (metadata_size, n) = vli::decode(&patch[pos..]).map_err(|e| e.to_string())?;
    pos += n;
    pos += metadata_size as usize;
    if pos > patch.len() {
        return Err("metadata extends beyond patch".to_string());
    }

    if source.len() as u64 != source_size {
        return Err(format!(
            "source size mismatch (expected: {source_size}, actual: {})",
            source.len()
        ));
    }

    // Verify source CRC
    let footer_start = patch.len() - 12;
    let stored_source_crc =
        u32::from_le_bytes(patch[footer_start..footer_start + 4].try_into().unwrap());
    let actual_source_crc = crc32fast::hash(source);
    if stored_source_crc != actual_source_crc {
        return Err(format!(
            "source CRC mismatch - wrong ROM? (expected: {stored_source_crc:08X}, actual: {actual_source_crc:08X})"
        ));
    }

    // Apply actions
    let mut target = vec![0u8; target_size as usize];
    let mut output_offset: usize = 0;
    let mut source_relative_offset: i64 = 0;
    let mut target_relative_offset: i64 = 0;
    let action_end = footer_start;

    while pos < action_end {
        let (data, n) = vli::decode(&patch[pos..]).map_err(|e| e.to_string())?;
        pos += n;

        let action = data & 3;
        let length = ((data >> 2) + 1) as usize;

        match action {
            0 => {
                // SourceRead
                for i in 0..length {
                    let src_idx = output_offset + i;
                    target[output_offset + i] = if src_idx < source.len() {
                        source[src_idx]
                    } else {
                        0
                    };
                }
                output_offset += length;
            }
            1 => {
                // TargetRead
                if pos + length > action_end {
                    return Err("TargetRead extends beyond patch data".to_string());
                }
                target[output_offset..output_offset + length]
                    .copy_from_slice(&patch[pos..pos + length]);
                pos += length;
                output_offset += length;
            }
            2 => {
                // SourceCopy
                let (offset_data, n) = vli::decode(&patch[pos..]).map_err(|e| e.to_string())?;
                pos += n;
                let sign = if offset_data & 1 != 0 { -1i64 } else { 1i64 };
                let abs_offset = (offset_data >> 1) as i64;
                source_relative_offset += sign * abs_offset;
                for _ in 0..length {
                    let src_idx = source_relative_offset as usize;
                    if src_idx >= source.len() {
                        return Err(format!("SourceCopy out of bounds: {}", src_idx));
                    }
                    target[output_offset] = source[src_idx];
                    output_offset += 1;
                    source_relative_offset += 1;
                }
            }
            3 => {
                // TargetCopy
                let (offset_data, n) = vli::decode(&patch[pos..]).map_err(|e| e.to_string())?;
                pos += n;
                let sign = if offset_data & 1 != 0 { -1i64 } else { 1i64 };
                let abs_offset = (offset_data >> 1) as i64;
                target_relative_offset += sign * abs_offset;
                for _ in 0..length {
                    let tgt_idx = target_relative_offset as usize;
                    if tgt_idx >= output_offset {
                        return Err(format!("TargetCopy out of bounds: {}", tgt_idx));
                    }
                    target[output_offset] = target[tgt_idx];
                    output_offset += 1;
                    target_relative_offset += 1;
                }
            }
            _ => unreachable!(),
        }
        if output_offset > target.len() {
            return Err("output exceeds target size".to_string());
        }
    }

    // Verify target CRC
    let stored_target_crc = u32::from_le_bytes(
        patch[footer_start + 4..footer_start + 8]
            .try_into()
            .unwrap(),
    );
    let actual_target_crc = crc32fast::hash(&target);
    if stored_target_crc != actual_target_crc {
        return Err(format!(
            "target CRC mismatch (expected: {stored_target_crc:08X}, actual: {actual_target_crc:08X})"
        ));
    }

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bps;

    #[test]
    fn test_apply_validates_magic() {
        let result = apply(&[], b"XXXX1234567890ab");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid magic"));
    }

    #[test]
    fn test_apply_validates_source_crc() {
        let source = b"hello".to_vec();
        let target = b"world".to_vec();
        let patch = bps::create(&source, &target).unwrap();
        let wrong_source = b"wrong".to_vec();
        let result = apply(&wrong_source, &patch);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_roundtrip_simple() {
        let source = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mut target = source.clone();
        target[3] = 99;
        target[7] = 88;
        let patch = bps::create(&source, &target).unwrap();
        let result = apply(&source, &patch).unwrap();
        assert_eq!(result, target);
    }
}
