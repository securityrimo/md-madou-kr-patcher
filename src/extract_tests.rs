use super::*;

fn write_u32_be(rom: &mut [u8], offset: usize, value: usize) {
    rom[offset..offset + 4].copy_from_slice(&(value as u32).to_be_bytes());
}

#[test]
fn fff8_reader_stops_at_next_pointer_when_parameter_equals_terminator() {
    let data_start = EN_PTR_TABLE + 12;
    let mut rom = vec![0u8; data_start + 8];
    write_u32_be(&mut rom, EN_PTR_TABLE, data_start);
    write_u32_be(&mut rom, EN_PTR_TABLE + 4, data_start + 4);
    write_u32_be(&mut rom, EN_PTR_TABLE + 8, data_start + 8);
    rom[data_start..data_start + 8]
        .copy_from_slice(&[0xFF, 0x78, 0xFF, 0x04, 0xFF, 0x28, 0xFF, 0x38]);

    let ctrl_params = text::ctrl_with_param();
    let terminators = HashSet::from([0xFF04, 0xFF38, 0xFFFF]);
    assert_eq!(
        read_fff8_entry_words(&rom, 0, &ctrl_params, &terminators),
        Some(vec![0xFF78, 0xFF04])
    );
    assert_eq!(
        read_fff8_entry_words(&rom, 1, &ctrl_params, &terminators),
        Some(vec![0xFF28, 0xFF38])
    );
}

#[test]
fn fff8_reader_keeps_long_shared_tail_entries_open() {
    let data_start = EN_PTR_TABLE + 12;
    let mut rom = vec![0u8; data_start + 14];
    write_u32_be(&mut rom, EN_PTR_TABLE, data_start);
    write_u32_be(&mut rom, EN_PTR_TABLE + 4, data_start + 8);
    write_u32_be(&mut rom, EN_PTR_TABLE + 8, data_start + 14);
    rom[data_start..data_start + 14].copy_from_slice(&[
        0xFF, 0x10, 0x00, 0x01, 0xFF, 0x9C, 0xFF, 0x04, 0x00, 0x02, 0xFF, 0x30, 0xFF, 0x38,
    ]);

    let ctrl_params = text::ctrl_with_param();
    let terminators = HashSet::from([0xFF04, 0xFF38, 0xFFFF]);
    assert_eq!(
        read_fff8_entry_words(&rom, 0, &ctrl_params, &terminators),
        Some(vec![0xFF10, 0x0001, 0xFF9C, 0xFF04, 0x0002, 0xFF30, 0xFF38])
    );
}
