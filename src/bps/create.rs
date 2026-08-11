use crate::bps::vli;

const BPS_MAGIC: &[u8; 4] = b"BPS1";

pub fn create(source: &[u8], target: &[u8]) -> Result<Vec<u8>, String> {
    let mut patch = Vec::new();

    // Header
    patch.extend_from_slice(BPS_MAGIC);
    vli::encode(&mut patch, source.len() as u64);
    vli::encode(&mut patch, target.len() as u64);
    vli::encode(&mut patch, 0); // no metadata

    // Generate actions
    let actions = generate_actions(source, target);
    for action in &actions {
        encode_action(&mut patch, action);
    }

    // Checksums (little-endian CRC32)
    let source_crc = crc32fast::hash(source);
    let target_crc = crc32fast::hash(target);
    patch.extend_from_slice(&source_crc.to_le_bytes());
    patch.extend_from_slice(&target_crc.to_le_bytes());
    let patch_crc = crc32fast::hash(&patch);
    patch.extend_from_slice(&patch_crc.to_le_bytes());

    Ok(patch)
}

#[derive(Debug)]
enum Action {
    SourceRead(usize),
    TargetRead(Vec<u8>),
}

fn generate_actions(source: &[u8], target: &[u8]) -> Vec<Action> {
    let mut actions: Vec<Action> = Vec::new();
    let mut pos = 0;

    while pos < target.len() {
        if pos < source.len() && source[pos] == target[pos] {
            let start = pos;
            while pos < target.len() && pos < source.len() && source[pos] == target[pos] {
                pos += 1;
            }
            actions.push(Action::SourceRead(pos - start));
        } else {
            let start = pos;
            while pos < target.len() && (pos >= source.len() || source[pos] != target[pos]) {
                pos += 1;
            }
            actions.push(Action::TargetRead(target[start..pos].to_vec()));
        }
    }

    actions
}

fn encode_action(patch: &mut Vec<u8>, action: &Action) {
    match action {
        Action::SourceRead(length) => {
            let data = (*length as u64 - 1) << 2;
            vli::encode(patch, data);
        }
        Action::TargetRead(bytes) => {
            let data = ((bytes.len() as u64 - 1) << 2) | 1;
            vli::encode(patch, data);
            patch.extend_from_slice(bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bps;

    #[test]
    fn test_create_identical() {
        let source = vec![0u8; 256];
        let target = source.clone();
        let patch = create(&source, &target).unwrap();
        assert_eq!(&patch[..4], b"BPS1");
        assert!(patch.len() >= 16);
    }

    #[test]
    fn test_create_single_byte_diff() {
        let source = vec![0u8; 16];
        let mut target = source.clone();
        target[8] = 0xFF;
        let patch = create(&source, &target).unwrap();
        assert_eq!(&patch[..4], b"BPS1");
    }

    #[test]
    fn test_create_roundtrip() {
        let source = b"Hello, World!".to_vec();
        let target = b"Hello, Rust!!".to_vec();
        let patch = create(&source, &target).unwrap();
        let result = bps::apply(&source, &patch).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_create_size_change() {
        let source = vec![0u8; 100];
        let target = vec![0xFFu8; 200];
        let patch = create(&source, &target).unwrap();
        let result = bps::apply(&source, &patch).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_create_empty_to_content() {
        let source = vec![];
        let target = b"new content".to_vec();
        let patch = create(&source, &target).unwrap();
        let result = bps::apply(&source, &patch).unwrap();
        assert_eq!(result, target);
    }
}
