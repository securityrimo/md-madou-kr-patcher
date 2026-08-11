/// Encode a u64 value as a BPS variable-length integer.
pub fn encode(buf: &mut Vec<u8>, mut data: u64) {
    loop {
        let x = (data & 0x7F) as u8;
        data >>= 7;
        if data == 0 {
            buf.push(0x80 | x);
            break;
        }
        buf.push(x);
        data -= 1;
    }
}

/// Decode a BPS variable-length integer from a byte slice.
/// Returns (value, bytes_consumed).
pub fn decode(data: &[u8]) -> Result<(u64, usize), &'static str> {
    let mut result: u64 = 0;
    let mut shift: u64 = 1;
    for (i, &byte) in data.iter().enumerate() {
        result += u64::from(byte & 0x7F) * shift;
        if byte & 0x80 != 0 {
            return Ok((result, i + 1));
        }
        shift <<= 7;
        result += shift;
    }
    Err("unexpected end of VLI data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_zero() {
        let mut buf = Vec::new();
        encode(&mut buf, 0);
        let (val, consumed) = decode(&buf).unwrap();
        assert_eq!(val, 0);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_roundtrip_small() {
        for n in 0..128 {
            let mut buf = Vec::new();
            encode(&mut buf, n);
            let (val, _) = decode(&buf).unwrap();
            assert_eq!(val, n, "roundtrip failed for {n}");
        }
    }

    #[test]
    fn test_roundtrip_large() {
        let values = [127, 128, 255, 256, 1000, 65535, 0x100000, 0xFFFFFFFF];
        for &n in &values {
            let mut buf = Vec::new();
            encode(&mut buf, n);
            let (val, _) = decode(&buf).unwrap();
            assert_eq!(val, n, "roundtrip failed for {n}");
        }
    }

    #[test]
    fn test_single_byte_range() {
        let mut buf = Vec::new();
        encode(&mut buf, 0);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0x80);

        buf.clear();
        encode(&mut buf, 127);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0xFF);
    }

    #[test]
    fn test_two_byte_range() {
        let mut buf = Vec::new();
        encode(&mut buf, 128);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_sequential_decode() {
        let mut buf = Vec::new();
        encode(&mut buf, 42);
        encode(&mut buf, 1000);
        encode(&mut buf, 0);

        let (v1, c1) = decode(&buf).unwrap();
        assert_eq!(v1, 42);
        let (v2, c2) = decode(&buf[c1..]).unwrap();
        assert_eq!(v2, 1000);
        let (v3, _) = decode(&buf[c1 + c2..]).unwrap();
        assert_eq!(v3, 0);
    }
}
