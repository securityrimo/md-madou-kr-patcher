use crate::m68k::{AddressReg, BranchCondition, BranchWidth, DataReg, Inst, assemble};

pub const HOOK_POINT: usize = 0x3004D8;
pub const HOOK_ADDR: u32 = 0x0033F000;
pub const RETURN_ADDR: u32 = 0x00300510;
pub const KR_WIDTH_TABLE: u32 = 0x0033F100;
pub const KR_FONT_BASE: u32 = 0x00340000;
pub const KR_INDEX_START: u16 = 0x0100;

pub const NOP_RANGE_START: usize = 0x3004DE;
pub const NOP_RANGE_END: usize = 0x300510;

fn width_read_program() -> Vec<Inst> {
    vec![
        Inst::MoveByteDisplacementAddressToData {
            displacement: 0x000C,
            source: AddressReg::A0,
            destination: DataReg::D2,
        },
        Inst::MoveByteDisplacementAddressToData {
            displacement: 0x0006,
            source: AddressReg::A0,
            destination: DataReg::D3,
        },
        Inst::SubByteData {
            source: DataReg::D3,
            destination: DataReg::D2,
        },
        Inst::MoveByteDataToDisplacementAddress {
            source: DataReg::D2,
            displacement: 0x000D,
            destination: AddressReg::A0,
        },
        Inst::MoveByteDisplacementAddressToData {
            displacement: 0x0000,
            source: AddressReg::A1,
            destination: DataReg::D2,
        },
        Inst::MoveByteDataToDisplacementAddress {
            source: DataReg::D2,
            displacement: 0x0006,
            destination: AddressReg::A0,
        },
        Inst::MoveByteDisplacementAddressToData {
            displacement: 0x0001,
            source: AddressReg::A1,
            destination: DataReg::D1,
        },
        Inst::MoveLongDataToPredecrementAddress {
            source: DataReg::D1,
            destination: AddressReg::A7,
        },
    ]
}

pub fn assemble_hook() -> Result<Vec<u8>, String> {
    let mut program = vec![
        Inst::AndiLongImmediate {
            immediate: 0x0000FFFF,
            destination: DataReg::D0,
        },
        Inst::CmpiWordImmediate {
            immediate: KR_INDEX_START,
            destination: DataReg::D0,
        },
        Inst::Branch {
            condition: BranchCondition::CarryClear,
            width: BranchWidth::Byte,
            target: "korean_path",
        },
        Inst::LslLongImmediate {
            count: 1,
            destination: DataReg::D0,
        },
        Inst::MoveAddressLongImmediate {
            address: 0x00320D46,
            destination: AddressReg::A1,
        },
        Inst::AddaLongData {
            source: DataReg::D0,
            destination: AddressReg::A1,
        },
    ];
    program.extend(width_read_program());
    program.extend([
        Inst::LslLongImmediate {
            count: 8,
            destination: DataReg::D0,
        },
        Inst::LslLongImmediate {
            count: 1,
            destination: DataReg::D0,
        },
        Inst::MoveAddressLongImmediate {
            address: 0x00300D46,
            destination: AddressReg::A1,
        },
        Inst::AddaLongData {
            source: DataReg::D0,
            destination: AddressReg::A1,
        },
        Inst::JmpAbsoluteLong(RETURN_ADDR),
        Inst::Label("korean_path"),
        Inst::SubiWordImmediate {
            immediate: KR_INDEX_START,
            destination: DataReg::D0,
        },
        Inst::LslLongImmediate {
            count: 1,
            destination: DataReg::D0,
        },
        Inst::MoveAddressLongImmediate {
            address: KR_WIDTH_TABLE,
            destination: AddressReg::A1,
        },
        Inst::AddaLongData {
            source: DataReg::D0,
            destination: AddressReg::A1,
        },
    ]);
    program.extend(width_read_program());
    program.extend([
        Inst::LslLongImmediate {
            count: 8,
            destination: DataReg::D0,
        },
        Inst::LslLongImmediate {
            count: 1,
            destination: DataReg::D0,
        },
        Inst::MoveAddressLongImmediate {
            address: KR_FONT_BASE,
            destination: AddressReg::A1,
        },
        Inst::AddaLongData {
            source: DataReg::D0,
            destination: AddressReg::A1,
        },
        Inst::JmpAbsoluteLong(RETURN_ADDR),
    ]);

    assemble(&program)
}

pub fn jmp_to_hook() -> Result<Vec<u8>, String> {
    assemble(&[Inst::JmpAbsoluteLong(HOOK_ADDR)])
}

pub fn apply_hook(rom: &mut [u8]) -> Result<(), String> {
    let hook_code = assemble_hook()?;
    let hook_addr = HOOK_ADDR as usize;
    rom[hook_addr..hook_addr + hook_code.len()].copy_from_slice(&hook_code);

    let jmp = jmp_to_hook()?;
    rom[HOOK_POINT..HOOK_POINT + jmp.len()].copy_from_slice(&jmp);

    let nop_count = (NOP_RANGE_END - NOP_RANGE_START) / 2;
    let nop_fill = assemble(&vec![Inst::Nop; nop_count])?;
    rom[NOP_RANGE_START..NOP_RANGE_END].copy_from_slice(&nop_fill);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_fits_reserved_region() {
        let code = assemble_hook().unwrap();
        assert!(
            code.len() <= 128,
            "hook code too large: {} bytes",
            code.len()
        );
    }

    #[test]
    fn hook_is_byte_identical_to_previous_en_pipeline() {
        let code = assemble_hook().unwrap();
        assert_eq!(
            &code[..12],
            &[
                0x02, 0x80, 0x00, 0x00, 0xFF, 0xFF, 0x0C, 0x40, 0x01, 0x00, 0x64, 0x38
            ]
        );
    }

    #[test]
    fn jmp_instruction_targets_hook() {
        assert_eq!(jmp_to_hook().unwrap(), [0x4E, 0xF9, 0x00, 0x33, 0xF0, 0x00]);
    }

    #[test]
    fn apply_hook_writes_typed_code_and_nops() {
        let mut rom = vec![0u8; 0x400000];
        apply_hook(&mut rom).unwrap();
        assert_eq!(&rom[HOOK_POINT..HOOK_POINT + 2], &[0x4E, 0xF9]);
        for addr in (NOP_RANGE_START..NOP_RANGE_END).step_by(2) {
            assert_eq!(&rom[addr..addr + 2], &[0x4E, 0x71]);
        }
        let hook_addr = HOOK_ADDR as usize;
        assert_ne!(&rom[hook_addr..hook_addr + 6], &[0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn en_and_kr_paths_both_return() {
        let code = assemble_hook().unwrap();
        let ret_bytes = RETURN_ADDR.to_be_bytes();
        assert_eq!(
            code.windows(4)
                .filter(|window| *window == ret_bytes)
                .count(),
            2
        );
    }
}
