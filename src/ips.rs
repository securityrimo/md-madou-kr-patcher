/// IPS patch format apply.
///
/// Format:
///   Header: "PATCH" (5 bytes)
///   Records: 3-byte offset + 2-byte size + data (or RLE if size=0)
///   Footer: "EOF" (3 bytes)
pub fn apply(source: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    if patch.len() < 8 {
        return Err("IPS patch too small".into());
    }
    if &patch[0..5] != b"PATCH" {
        return Err("Not a valid IPS patch (missing PATCH header)".into());
    }

    let mut target = source.to_vec();
    let mut pos = 5;

    loop {
        if pos + 3 > patch.len() {
            return Err(format!("Unexpected end of IPS patch at offset {pos}"));
        }

        // Check for EOF marker
        if &patch[pos..pos + 3] == b"EOF" {
            break;
        }

        let offset = read_u24_be(patch, pos);
        pos += 3;

        if pos + 2 > patch.len() {
            return Err(format!("Unexpected end of IPS patch at offset {pos}"));
        }
        let size = read_u16_be(patch, pos) as usize;
        pos += 2;

        if size > 0 {
            // Normal record
            if pos + size > patch.len() {
                return Err(format!(
                    "IPS record at 0x{offset:06X} requests {size} bytes but patch ends"
                ));
            }
            ensure_len(&mut target, offset + size);
            target[offset..offset + size].copy_from_slice(&patch[pos..pos + size]);
            pos += size;
        } else {
            // RLE record
            if pos + 3 > patch.len() {
                return Err(format!("Unexpected end of RLE record at offset {pos}"));
            }
            let rle_size = read_u16_be(patch, pos) as usize;
            pos += 2;
            let rle_value = patch[pos];
            pos += 1;
            ensure_len(&mut target, offset + rle_size);
            for byte in &mut target[offset..offset + rle_size] {
                *byte = rle_value;
            }
        }
    }

    Ok(target)
}

fn read_u24_be(data: &[u8], pos: usize) -> usize {
    ((data[pos] as usize) << 16) | ((data[pos + 1] as usize) << 8) | (data[pos + 2] as usize)
}

fn read_u16_be(data: &[u8], pos: usize) -> u16 {
    ((data[pos] as u16) << 8) | (data[pos + 1] as u16)
}

/// Extend target buffer with zeros if needed.
fn ensure_len(target: &mut Vec<u8>, required: usize) {
    if required > target.len() {
        target.resize(required, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_patch(records: &[u8]) -> Vec<u8> {
        let mut p = b"PATCH".to_vec();
        p.extend_from_slice(records);
        p.extend_from_slice(b"EOF");
        p
    }

    #[test]
    fn test_apply_simple() {
        let source = vec![0u8; 16];
        let patch = make_patch(&[
            0x00, 0x00, 0x04, // offset 4
            0x00, 0x03, // size 3
            0xAA, 0xBB, 0xCC, // data
        ]);
        let result = apply(&source, &patch).unwrap();
        assert_eq!(result[4], 0xAA);
        assert_eq!(result[5], 0xBB);
        assert_eq!(result[6], 0xCC);
        assert_eq!(result[3], 0x00);
    }

    #[test]
    fn test_apply_rle() {
        let source = vec![0u8; 16];
        let patch = make_patch(&[
            0x00, 0x00, 0x02, // offset 2
            0x00, 0x00, // size 0 = RLE
            0x00, 0x05, // rle_size 5
            0xFF, // rle_value
        ]);
        let result = apply(&source, &patch).unwrap();
        assert_eq!(&result[2..7], &[0xFF; 5]);
        assert_eq!(result[0], 0x00);
        assert_eq!(result[7], 0x00);
    }

    #[test]
    fn test_apply_extends_file() {
        let source = vec![0u8; 4];
        let patch = make_patch(&[
            0x00, 0x00, 0x08, // offset 8
            0x00, 0x02, // size 2
            0xDE, 0xAD,
        ]);
        let result = apply(&source, &patch).unwrap();
        assert_eq!(result.len(), 10);
        assert_eq!(&result[8..10], &[0xDE, 0xAD]);
    }

    #[test]
    fn test_invalid_header() {
        let patch = b"NOTIPS".to_vec();
        assert!(apply(&[], &patch).is_err());
    }

    #[test]
    fn test_empty_patch() {
        let source = vec![0x42u8; 8];
        let patch = make_patch(&[]);
        let result = apply(&source, &patch).unwrap();
        assert_eq!(result, source);
    }
}
