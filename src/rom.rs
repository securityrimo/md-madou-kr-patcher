const SUPPORTED_SIZES: [usize; 2] = [0x200000, 0x400000]; // JP original, EN-expanded
const HEADER_OFFSET: usize = 0x100;

pub fn validate_rom(data: &[u8], label: &str) -> Result<(), String> {
    if !SUPPORTED_SIZES.contains(&data.len()) {
        return Err(format!(
            "{label}: expected a 2 MiB JP or 4 MiB expanded ROM, got {} bytes",
            data.len()
        ));
    }

    let header = &data[HEADER_OFFSET..HEADER_OFFSET + 16];
    let header_str = String::from_utf8_lossy(header);
    if !header_str.starts_with("SEGA MEGA DRIVE") && !header_str.starts_with("SEGA GENESIS") {
        return Err(format!(
            "{label}: not a Mega Drive ROM (header: {header_str:?})"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_size_too_small() {
        let rom = vec![0u8; 1024];
        assert!(validate_rom(&rom, "test").is_err());
    }

    #[test]
    fn test_validate_valid_header() {
        let mut rom = vec![0u8; 0x400000];
        let header = b"SEGA MEGA DRIVE ";
        rom[0x100..0x100 + header.len()].copy_from_slice(header);
        assert!(validate_rom(&rom, "test").is_ok());
    }

    #[test]
    fn test_validate_jp_sized_header() {
        let mut rom = vec![0u8; 0x200000];
        let header = b"SEGA MEGA DRIVE ";
        rom[0x100..0x100 + header.len()].copy_from_slice(header);
        assert!(validate_rom(&rom, "test").is_ok());
    }

    #[test]
    fn test_validate_genesis_header() {
        let mut rom = vec![0u8; 0x400000];
        let header = b"SEGA GENESIS    ";
        rom[0x100..0x100 + header.len()].copy_from_slice(header);
        assert!(validate_rom(&rom, "test").is_ok());
    }
}
