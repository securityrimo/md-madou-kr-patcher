use super::*;

#[test]
fn word_branch_resolves_from_the_end_of_the_instruction() {
    let bytes = assemble(&[
        Inst::Branch {
            condition: BranchCondition::CarrySet,
            width: BranchWidth::Word,
            target: "target",
        },
        Inst::Nop,
        Inst::Label("target"),
        Inst::Rts,
    ])
    .unwrap();

    assert_eq!(bytes, [0x65, 0x00, 0x00, 0x04, 0x4E, 0x71, 0x4E, 0x75]);
}

#[test]
fn byte_branch_resolves_from_the_end_of_the_instruction() {
    let bytes = assemble(&[
        Inst::Branch {
            condition: BranchCondition::CarryClear,
            width: BranchWidth::Byte,
            target: "target",
        },
        Inst::Nop,
        Inst::Label("target"),
        Inst::Rts,
    ])
    .unwrap();

    assert_eq!(bytes, [0x64, 0x02, 0x4E, 0x71, 0x4E, 0x75]);
}

#[test]
fn absolute_dbf_uses_the_instruction_address_as_its_origin() {
    let bytes = assemble_at(
        0x0010_0020,
        &[Inst::DbfAbsolute {
            register: DataReg::D7,
            target: 0x0010_0008,
        }],
    )
    .unwrap();

    assert_eq!(bytes, [0x51, 0xCF, 0xFF, 0xE6]);
}

#[test]
fn invalid_immediate_shift_count_is_rejected() {
    let error = assemble(&[Inst::AslWordImmediate {
        count: 0,
        destination: DataReg::D0,
    }])
    .unwrap_err();

    assert!(error.contains("1..=8"));
}

#[test]
fn invalid_addq_immediate_is_rejected() {
    let error = assemble(&[Inst::AddqWordImmediateToDisplacementAddress {
        immediate: 0,
        displacement: 0x0010,
        destination: AddressReg::A0,
    }])
    .unwrap_err();

    assert!(error.contains("1..=8"));
}

#[test]
fn absolute_long_address_is_encoded_in_big_endian_order() {
    let bytes = assemble(&[Inst::JmpAbsoluteLong(0x0012_3456)]).unwrap();

    assert_eq!(bytes, [0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56]);
}

#[test]
fn absolute_short_word_addressing_preserves_the_low_word() {
    let bytes = assemble(&[
        Inst::MoveWordImmediateToAbsoluteShort {
            immediate: 0x1234,
            address: 0xFEDC,
        },
        Inst::MoveWordAbsoluteShortToData {
            address: 0xFEDC,
            destination: DataReg::D0,
        },
    ])
    .unwrap();

    assert_eq!(
        bytes,
        [0x31, 0xFC, 0x12, 0x34, 0xFE, 0xDC, 0x30, 0x38, 0xFE, 0xDC]
    );
}

#[test]
fn representative_addressing_modes_compose_in_instruction_order() {
    let bytes = assemble(&[
        Inst::MoveByteImmediateToAbsoluteLong {
            immediate: 0x7F,
            address: 0x0012_3456,
        },
        Inst::LeaAbsoluteLong {
            address: 0x0023_4568,
            destination: AddressReg::A2,
        },
        Inst::AddaWordData {
            source: DataReg::D0,
            destination: AddressReg::A2,
        },
        Inst::Rts,
    ])
    .unwrap();

    assert_eq!(
        bytes,
        [
            0x13, 0xFC, 0x00, 0x7F, 0x00, 0x12, 0x34, 0x56, 0x45, 0xF9, 0x00, 0x23, 0x45, 0x68,
            0xD4, 0xC0, 0x4E, 0x75,
        ]
    );
}
