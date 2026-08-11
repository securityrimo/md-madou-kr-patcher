//! Game-facing Motorola 68000 patch DSL lowered through `retro-typed-isa`.
//!
//! Patch call sites keep their domain-specific vocabulary here, while the
//! shared MC68000 profile owns instruction validity, byte encoding, strict
//! decoding, and complete stream placement.

use std::collections::HashMap;

use m68000::{
    AddressSize, BranchDisplacement, BranchKind, CodeLocation, Condition, DataDirection,
    EffectiveAddress, Immediate, IndexRegister, IndexSize, Instruction, OperandSize, Register,
    ShiftCount, ShiftDirection, ShiftOperation,
};

pub use m68000::{AddressRegister as AddressReg, DataRegister as DataReg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchCondition {
    Always,
    Equal,
    NotEqual,
    CarryClear,
    CarrySet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchWidth {
    Byte,
    Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inst {
    Moveq {
        immediate: i8,
        destination: DataReg,
    },
    AndiLongImmediate {
        immediate: u32,
        destination: DataReg,
    },
    AndiWordImmediate {
        immediate: u16,
        destination: DataReg,
    },
    AndiWordImmediateToStatus {
        immediate: u16,
    },
    OriWordImmediateToStatus {
        immediate: u16,
    },
    AndWordImmediate {
        immediate: u16,
        destination: DataReg,
    },
    AddiWordImmediate {
        immediate: u16,
        destination: DataReg,
    },
    CmpiWordImmediate {
        immediate: u16,
        destination: DataReg,
    },
    CmpaLongImmediate {
        immediate: u32,
        destination: AddressReg,
    },
    CmpiWordImmediateToAddressIndirect {
        immediate: u16,
        source: AddressReg,
    },
    SubiWordImmediate {
        immediate: u16,
        destination: DataReg,
    },
    MoveByteImmediateToAbsoluteLong {
        immediate: u8,
        address: u32,
    },
    MoveByteImmediateToData {
        immediate: u8,
        destination: DataReg,
    },
    MoveByteImmediateToDisplacementAddress {
        immediate: u8,
        displacement: u16,
        destination: AddressReg,
    },
    MoveByteAbsoluteShortToData {
        address: u16,
        destination: DataReg,
    },
    MoveByteDataToAbsoluteShort {
        source: DataReg,
        address: u16,
    },
    MoveByteAbsoluteShortToAbsoluteShort {
        source: u16,
        destination: u16,
    },
    MoveWordImmediateToAbsoluteLong {
        immediate: u16,
        address: u32,
    },
    MoveWordImmediateToAbsoluteShort {
        immediate: u16,
        address: u16,
    },
    MoveWordImmediateToData {
        immediate: u16,
        destination: DataReg,
    },
    MoveLongImmediateToData {
        immediate: u32,
        destination: DataReg,
    },
    MoveWordImmediateToDisplacementAddress {
        immediate: u16,
        displacement: u16,
        destination: AddressReg,
    },
    MoveByteAbsoluteLongToData {
        address: u32,
        destination: DataReg,
    },
    MoveWordAbsoluteLongToData {
        address: u32,
        destination: DataReg,
    },
    MoveWordAbsoluteShortToData {
        address: u16,
        destination: DataReg,
    },
    MoveAddressLongImmediate {
        address: u32,
        destination: AddressReg,
    },
    MoveLongImmediateToPostincrementAddress {
        immediate: u32,
        destination: AddressReg,
    },
    MoveLongImmediateToDisplacementAddress {
        immediate: u32,
        displacement: u16,
        destination: AddressReg,
    },
    MoveLongImmediateToAbsoluteLong {
        immediate: u32,
        address: u32,
    },
    MoveAddressAbsoluteLongToAddress {
        address: u32,
        destination: AddressReg,
    },
    MoveAddressAbsoluteShortToAddress {
        address: u16,
        destination: AddressReg,
    },
    MoveAddressLongIndexedWordToAddress {
        base: AddressReg,
        index: DataReg,
        destination: AddressReg,
    },
    MoveWordPostincrementAddressToData {
        source: AddressReg,
        destination: DataReg,
    },
    MoveWordDisplacementAddressToData {
        displacement: u16,
        source: AddressReg,
        destination: DataReg,
    },
    MoveWordDataToData {
        source: DataReg,
        destination: DataReg,
    },
    MoveLongPostincrementAddressToData {
        source: AddressReg,
        destination: DataReg,
    },
    MoveWordIndexedAddressToData {
        displacement: i8,
        base: AddressReg,
        index: DataReg,
        destination: DataReg,
    },
    MoveWordPostincrementAddressToPostincrementAddress {
        source: AddressReg,
        destination: AddressReg,
    },
    MoveLongPostincrementAddressToPostincrementAddress {
        source: AddressReg,
        destination: AddressReg,
    },
    MoveWordDisplacementAddressToPostincrementAddress {
        displacement: u16,
        source: AddressReg,
        destination: AddressReg,
    },
    MoveLongDisplacementAddressToPostincrementAddress {
        displacement: u16,
        source: AddressReg,
        destination: AddressReg,
    },
    MoveWordDisplacementAddressToDisplacementAddress {
        source_displacement: u16,
        source: AddressReg,
        destination_displacement: u16,
        destination: AddressReg,
    },
    MoveLongDisplacementAddressToDisplacementAddress {
        source_displacement: u16,
        source: AddressReg,
        destination_displacement: u16,
        destination: AddressReg,
    },
    MoveLongIndexedAddressToAbsoluteLong {
        displacement: i8,
        base: AddressReg,
        index: DataReg,
        address: u32,
    },
    MoveWordStatusToPredecrementAddress {
        destination: AddressReg,
    },
    MoveWordPostincrementAddressToStatus {
        source: AddressReg,
    },
    MoveAddressLongToDisplacementAddress {
        source: AddressReg,
        displacement: u16,
        destination: AddressReg,
    },
    LeaAbsoluteLong {
        address: u32,
        destination: AddressReg,
    },
    LeaAbsoluteShort {
        address: u16,
        destination: AddressReg,
    },
    LeaIndexedWord {
        displacement: i8,
        base: AddressReg,
        index: DataReg,
        destination: AddressReg,
    },
    LeaProgramCounterIndexedWord {
        displacement: i8,
        index: DataReg,
        destination: AddressReg,
    },
    ClearWordPostincrementAddress {
        destination: AddressReg,
    },
    ClearByteDisplacementAddress {
        displacement: u16,
        destination: AddressReg,
    },
    MoveByteDisplacementAddressToData {
        displacement: u16,
        source: AddressReg,
        destination: DataReg,
    },
    MoveByteDataToDisplacementAddress {
        source: DataReg,
        displacement: u16,
        destination: AddressReg,
    },
    MoveLongDataToPredecrementAddress {
        source: DataReg,
        destination: AddressReg,
    },
    SubByteData {
        source: DataReg,
        destination: DataReg,
    },
    SubqByteImmediate {
        immediate: u8,
        destination: DataReg,
    },
    SubWordDisplacementAddress {
        displacement: u16,
        source: AddressReg,
        destination: DataReg,
    },
    AslWordImmediate {
        count: u8,
        destination: DataReg,
    },
    AslLongImmediate {
        count: u8,
        destination: DataReg,
    },
    LslLongImmediate {
        count: u8,
        destination: DataReg,
    },
    LslWordImmediate {
        count: u8,
        destination: DataReg,
    },
    LsrWordImmediate {
        count: u8,
        destination: DataReg,
    },
    AddaWordData {
        source: DataReg,
        destination: AddressReg,
    },
    AddaWordPostincrementAddress {
        source: AddressReg,
        destination: AddressReg,
    },
    AddaLongData {
        source: DataReg,
        destination: AddressReg,
    },
    AddWordData {
        source: DataReg,
        destination: DataReg,
    },
    AddWordAddressIndirect {
        source: AddressReg,
        destination: DataReg,
    },
    AddqWordImmediateToDisplacementAddress {
        immediate: u8,
        displacement: u16,
        destination: AddressReg,
    },
    Dbf {
        register: DataReg,
        target: &'static str,
    },
    DbfAbsolute {
        register: DataReg,
        target: u32,
    },
    Branch {
        condition: BranchCondition,
        width: BranchWidth,
        target: &'static str,
    },
    BranchAbsolute {
        condition: BranchCondition,
        width: BranchWidth,
        target: u32,
    },
    JmpAbsoluteLong(u32),
    JsrAbsoluteLong(u32),
    JsrProgramCounterDisplacement(u32),
    Rts,
    Nop,
    Label(&'static str),
}

fn data(register: DataReg) -> EffectiveAddress {
    EffectiveAddress::DataRegister(register)
}

fn address_displacement(register: AddressReg, displacement: u16) -> EffectiveAddress {
    EffectiveAddress::AddressDisplacement {
        register,
        displacement: displacement as i16,
    }
}

fn word_index(register: DataReg) -> IndexRegister {
    IndexRegister {
        register: Register::Data(register),
        size: IndexSize::Word,
    }
}

fn branch_kind(condition: BranchCondition) -> BranchKind {
    match condition {
        BranchCondition::Always => BranchKind::Always,
        BranchCondition::Equal => BranchKind::Conditional(Condition::Equal),
        BranchCondition::NotEqual => BranchKind::Conditional(Condition::NotEqual),
        BranchCondition::CarryClear => BranchKind::Conditional(Condition::CarryClear),
        BranchCondition::CarrySet => BranchKind::Conditional(Condition::CarrySet),
    }
}

fn instruction_address(origin: u32, pc: usize) -> Result<u32, String> {
    let pc = u32::try_from(pc).map_err(|_| format!("68000 program offset is too large: {pc}"))?;
    origin
        .checked_add(pc)
        .ok_or_else(|| format!("68000 program address overflows at origin 0x{origin:08X}"))
}

fn relative_displacement(
    origin: u32,
    pc: usize,
    target: u32,
    _description: &str,
) -> Result<i64, String> {
    let instruction = instruction_address(origin, pc)?;
    Ok(i64::from(target) - (i64::from(instruction) + 2))
}

fn branch_displacement(
    origin: u32,
    pc: usize,
    target: u32,
    width: BranchWidth,
    description: &str,
) -> Result<BranchDisplacement, String> {
    let displacement = relative_displacement(origin, pc, target, description)?;
    match width {
        BranchWidth::Byte => {
            let displacement = i8::try_from(displacement).map_err(|_| {
                format!("68000 byte branch to {description} is out of range: {displacement}")
            })?;
            if displacement == 0 {
                return Err(format!(
                    "68000 byte branch to {description} encodes reserved displacement 0"
                ));
            }
            Ok(BranchDisplacement::Byte(displacement))
        }
        BranchWidth::Word => i16::try_from(displacement)
            .map(BranchDisplacement::Word)
            .map_err(|_| {
                format!("68000 word branch to {description} is out of range: {displacement}")
            }),
    }
}

fn label_address(
    origin: u32,
    labels: &HashMap<&'static str, usize>,
    label: &'static str,
) -> Result<u32, String> {
    let offset = labels
        .get(label)
        .copied()
        .ok_or_else(|| format!("undefined 68000 label: {label}"))?;
    instruction_address(origin, offset)
}

fn lower_instruction(
    inst: &Inst,
    origin: u32,
    pc: usize,
    labels: &HashMap<&'static str, usize>,
) -> Result<Option<Instruction>, String> {
    let instruction = match inst {
        Inst::Moveq {
            immediate,
            destination,
        } => Instruction::MoveQuick {
            immediate: *immediate,
            destination: *destination,
        },
        Inst::AndiLongImmediate {
            immediate,
            destination,
        } => Instruction::AndImmediate {
            size: OperandSize::Long,
            immediate: Immediate::Long(*immediate),
            destination: data(*destination),
        },
        Inst::AndiWordImmediate {
            immediate,
            destination,
        } => Instruction::AndImmediate {
            size: OperandSize::Word,
            immediate: Immediate::Word(*immediate),
            destination: data(*destination),
        },
        Inst::AndiWordImmediateToStatus { immediate } => {
            Instruction::AndImmediateToStatus(*immediate)
        }
        Inst::OriWordImmediateToStatus { immediate } => {
            Instruction::OrImmediateToStatus(*immediate)
        }
        Inst::AndWordImmediate {
            immediate,
            destination,
        } => Instruction::And {
            size: OperandSize::Word,
            direction: DataDirection::ToDataRegister,
            data_register: *destination,
            effective_address: EffectiveAddress::Immediate(Immediate::Word(*immediate)),
        },
        Inst::AddiWordImmediate {
            immediate,
            destination,
        } => Instruction::AddImmediate {
            size: OperandSize::Word,
            immediate: Immediate::Word(*immediate),
            destination: data(*destination),
        },
        Inst::CmpiWordImmediate {
            immediate,
            destination,
        } => Instruction::CompareImmediate {
            size: OperandSize::Word,
            immediate: Immediate::Word(*immediate),
            destination: data(*destination),
        },
        Inst::CmpaLongImmediate {
            immediate,
            destination,
        } => Instruction::CompareAddress {
            size: AddressSize::Long,
            source: EffectiveAddress::Immediate(Immediate::Long(*immediate)),
            destination: *destination,
        },
        Inst::CmpiWordImmediateToAddressIndirect { immediate, source } => {
            Instruction::CompareImmediate {
                size: OperandSize::Word,
                immediate: Immediate::Word(*immediate),
                destination: EffectiveAddress::AddressIndirect(*source),
            }
        }
        Inst::SubiWordImmediate {
            immediate,
            destination,
        } => Instruction::SubtractImmediate {
            size: OperandSize::Word,
            immediate: Immediate::Word(*immediate),
            destination: data(*destination),
        },
        Inst::MoveByteImmediateToAbsoluteLong { immediate, address } => Instruction::Move {
            size: OperandSize::Byte,
            source: EffectiveAddress::Immediate(Immediate::Byte(*immediate)),
            destination: EffectiveAddress::AbsoluteLong(*address),
        },
        Inst::MoveByteImmediateToData {
            immediate,
            destination,
        } => Instruction::Move {
            size: OperandSize::Byte,
            source: EffectiveAddress::Immediate(Immediate::Byte(*immediate)),
            destination: data(*destination),
        },
        Inst::MoveByteImmediateToDisplacementAddress {
            immediate,
            displacement,
            destination,
        } => Instruction::Move {
            size: OperandSize::Byte,
            source: EffectiveAddress::Immediate(Immediate::Byte(*immediate)),
            destination: address_displacement(*destination, *displacement),
        },
        Inst::MoveByteAbsoluteShortToData {
            address,
            destination,
        } => Instruction::Move {
            size: OperandSize::Byte,
            source: EffectiveAddress::AbsoluteWord(*address),
            destination: data(*destination),
        },
        Inst::MoveByteDataToAbsoluteShort { source, address } => Instruction::Move {
            size: OperandSize::Byte,
            source: data(*source),
            destination: EffectiveAddress::AbsoluteWord(*address),
        },
        Inst::MoveByteAbsoluteShortToAbsoluteShort {
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Byte,
            source: EffectiveAddress::AbsoluteWord(*source),
            destination: EffectiveAddress::AbsoluteWord(*destination),
        },
        Inst::MoveWordImmediateToAbsoluteLong { immediate, address } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::Immediate(Immediate::Word(*immediate)),
            destination: EffectiveAddress::AbsoluteLong(*address),
        },
        Inst::MoveWordImmediateToAbsoluteShort { immediate, address } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::Immediate(Immediate::Word(*immediate)),
            destination: EffectiveAddress::AbsoluteWord(*address),
        },
        Inst::MoveWordImmediateToData {
            immediate,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::Immediate(Immediate::Word(*immediate)),
            destination: data(*destination),
        },
        Inst::MoveLongImmediateToData {
            immediate,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::Immediate(Immediate::Long(*immediate)),
            destination: data(*destination),
        },
        Inst::MoveWordImmediateToDisplacementAddress {
            immediate,
            displacement,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::Immediate(Immediate::Word(*immediate)),
            destination: address_displacement(*destination, *displacement),
        },
        Inst::MoveByteAbsoluteLongToData {
            address,
            destination,
        } => Instruction::Move {
            size: OperandSize::Byte,
            source: EffectiveAddress::AbsoluteLong(*address),
            destination: data(*destination),
        },
        Inst::MoveWordAbsoluteLongToData {
            address,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::AbsoluteLong(*address),
            destination: data(*destination),
        },
        Inst::MoveWordAbsoluteShortToData {
            address,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::AbsoluteWord(*address),
            destination: data(*destination),
        },
        Inst::MoveAddressLongImmediate {
            address,
            destination,
        } => Instruction::MoveAddress {
            size: AddressSize::Long,
            source: EffectiveAddress::Immediate(Immediate::Long(*address)),
            destination: *destination,
        },
        Inst::MoveLongImmediateToPostincrementAddress {
            immediate,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::Immediate(Immediate::Long(*immediate)),
            destination: EffectiveAddress::AddressPostincrement(*destination),
        },
        Inst::MoveLongImmediateToDisplacementAddress {
            immediate,
            displacement,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::Immediate(Immediate::Long(*immediate)),
            destination: address_displacement(*destination, *displacement),
        },
        Inst::MoveLongImmediateToAbsoluteLong { immediate, address } => Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::Immediate(Immediate::Long(*immediate)),
            destination: EffectiveAddress::AbsoluteLong(*address),
        },
        Inst::MoveAddressAbsoluteLongToAddress {
            address,
            destination,
        } => Instruction::MoveAddress {
            size: AddressSize::Long,
            source: EffectiveAddress::AbsoluteLong(*address),
            destination: *destination,
        },
        Inst::MoveAddressAbsoluteShortToAddress {
            address,
            destination,
        } => Instruction::MoveAddress {
            size: AddressSize::Long,
            source: EffectiveAddress::AbsoluteWord(*address),
            destination: *destination,
        },
        Inst::MoveAddressLongIndexedWordToAddress {
            base,
            index,
            destination,
        } => Instruction::MoveAddress {
            size: AddressSize::Long,
            source: EffectiveAddress::AddressIndex {
                register: *base,
                index: word_index(*index),
                displacement: 0,
            },
            destination: *destination,
        },
        Inst::MoveWordPostincrementAddressToData {
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::AddressPostincrement(*source),
            destination: data(*destination),
        },
        Inst::MoveWordDisplacementAddressToData {
            displacement,
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: address_displacement(*source, *displacement),
            destination: data(*destination),
        },
        Inst::MoveWordDataToData {
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: data(*source),
            destination: data(*destination),
        },
        Inst::MoveLongPostincrementAddressToData {
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::AddressPostincrement(*source),
            destination: data(*destination),
        },
        Inst::MoveWordIndexedAddressToData {
            displacement,
            base,
            index,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::AddressIndex {
                register: *base,
                index: word_index(*index),
                displacement: *displacement,
            },
            destination: data(*destination),
        },
        Inst::MoveWordPostincrementAddressToPostincrementAddress {
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: EffectiveAddress::AddressPostincrement(*source),
            destination: EffectiveAddress::AddressPostincrement(*destination),
        },
        Inst::MoveLongPostincrementAddressToPostincrementAddress {
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::AddressPostincrement(*source),
            destination: EffectiveAddress::AddressPostincrement(*destination),
        },
        Inst::MoveWordDisplacementAddressToPostincrementAddress {
            displacement,
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: address_displacement(*source, *displacement),
            destination: EffectiveAddress::AddressPostincrement(*destination),
        },
        Inst::MoveLongDisplacementAddressToPostincrementAddress {
            displacement,
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: address_displacement(*source, *displacement),
            destination: EffectiveAddress::AddressPostincrement(*destination),
        },
        Inst::MoveWordDisplacementAddressToDisplacementAddress {
            source_displacement,
            source,
            destination_displacement,
            destination,
        } => Instruction::Move {
            size: OperandSize::Word,
            source: address_displacement(*source, *source_displacement),
            destination: address_displacement(*destination, *destination_displacement),
        },
        Inst::MoveLongDisplacementAddressToDisplacementAddress {
            source_displacement,
            source,
            destination_displacement,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: address_displacement(*source, *source_displacement),
            destination: address_displacement(*destination, *destination_displacement),
        },
        Inst::MoveLongIndexedAddressToAbsoluteLong {
            displacement,
            base,
            index,
            address,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::AddressIndex {
                register: *base,
                index: word_index(*index),
                displacement: *displacement,
            },
            destination: EffectiveAddress::AbsoluteLong(*address),
        },
        Inst::MoveWordStatusToPredecrementAddress { destination } => Instruction::MoveFromStatus {
            destination: EffectiveAddress::AddressPredecrement(*destination),
        },
        Inst::MoveWordPostincrementAddressToStatus { source } => Instruction::MoveToStatus {
            source: EffectiveAddress::AddressPostincrement(*source),
        },
        Inst::MoveAddressLongToDisplacementAddress {
            source,
            displacement,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::AddressRegister(*source),
            destination: address_displacement(*destination, *displacement),
        },
        Inst::LeaAbsoluteLong {
            address,
            destination,
        } => Instruction::LoadEffectiveAddress {
            source: EffectiveAddress::AbsoluteLong(*address),
            destination: *destination,
        },
        Inst::LeaAbsoluteShort {
            address,
            destination,
        } => Instruction::LoadEffectiveAddress {
            source: EffectiveAddress::AbsoluteWord(*address),
            destination: *destination,
        },
        Inst::LeaIndexedWord {
            displacement,
            base,
            index,
            destination,
        } => Instruction::LoadEffectiveAddress {
            source: EffectiveAddress::AddressIndex {
                register: *base,
                index: word_index(*index),
                displacement: *displacement,
            },
            destination: *destination,
        },
        Inst::LeaProgramCounterIndexedWord {
            displacement,
            index,
            destination,
        } => Instruction::LoadEffectiveAddress {
            source: EffectiveAddress::ProgramCounterIndex {
                index: word_index(*index),
                displacement: *displacement,
            },
            destination: *destination,
        },
        Inst::ClearWordPostincrementAddress { destination } => Instruction::Clear {
            size: OperandSize::Word,
            destination: EffectiveAddress::AddressPostincrement(*destination),
        },
        Inst::ClearByteDisplacementAddress {
            displacement,
            destination,
        } => Instruction::Clear {
            size: OperandSize::Byte,
            destination: address_displacement(*destination, *displacement),
        },
        Inst::MoveByteDisplacementAddressToData {
            displacement,
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Byte,
            source: address_displacement(*source, *displacement),
            destination: data(*destination),
        },
        Inst::MoveByteDataToDisplacementAddress {
            source,
            displacement,
            destination,
        } => Instruction::Move {
            size: OperandSize::Byte,
            source: data(*source),
            destination: address_displacement(*destination, *displacement),
        },
        Inst::MoveLongDataToPredecrementAddress {
            source,
            destination,
        } => Instruction::Move {
            size: OperandSize::Long,
            source: data(*source),
            destination: EffectiveAddress::AddressPredecrement(*destination),
        },
        Inst::SubByteData {
            source,
            destination,
        } => Instruction::Subtract {
            size: OperandSize::Byte,
            direction: DataDirection::ToDataRegister,
            data_register: *destination,
            effective_address: data(*source),
        },
        Inst::SubqByteImmediate {
            immediate,
            destination,
        } => Instruction::SubtractQuick {
            size: OperandSize::Byte,
            immediate: *immediate,
            destination: data(*destination),
        },
        Inst::SubWordDisplacementAddress {
            displacement,
            source,
            destination,
        } => Instruction::Subtract {
            size: OperandSize::Word,
            direction: DataDirection::ToDataRegister,
            data_register: *destination,
            effective_address: address_displacement(*source, *displacement),
        },
        Inst::AslWordImmediate { count, destination } => Instruction::ShiftRegister {
            operation: ShiftOperation::Arithmetic,
            direction: ShiftDirection::Left,
            size: OperandSize::Word,
            count: ShiftCount::Immediate(*count),
            destination: *destination,
        },
        Inst::AslLongImmediate { count, destination } => Instruction::ShiftRegister {
            operation: ShiftOperation::Arithmetic,
            direction: ShiftDirection::Left,
            size: OperandSize::Long,
            count: ShiftCount::Immediate(*count),
            destination: *destination,
        },
        Inst::LslLongImmediate { count, destination } => Instruction::ShiftRegister {
            operation: ShiftOperation::Logical,
            direction: ShiftDirection::Left,
            size: OperandSize::Long,
            count: ShiftCount::Immediate(*count),
            destination: *destination,
        },
        Inst::LslWordImmediate { count, destination } => Instruction::ShiftRegister {
            operation: ShiftOperation::Logical,
            direction: ShiftDirection::Left,
            size: OperandSize::Word,
            count: ShiftCount::Immediate(*count),
            destination: *destination,
        },
        Inst::LsrWordImmediate { count, destination } => Instruction::ShiftRegister {
            operation: ShiftOperation::Logical,
            direction: ShiftDirection::Right,
            size: OperandSize::Word,
            count: ShiftCount::Immediate(*count),
            destination: *destination,
        },
        Inst::AddaWordData {
            source,
            destination,
        } => Instruction::AddAddress {
            size: AddressSize::Word,
            source: data(*source),
            destination: *destination,
        },
        Inst::AddaWordPostincrementAddress {
            source,
            destination,
        } => Instruction::AddAddress {
            size: AddressSize::Word,
            source: EffectiveAddress::AddressPostincrement(*source),
            destination: *destination,
        },
        Inst::AddaLongData {
            source,
            destination,
        } => Instruction::AddAddress {
            size: AddressSize::Long,
            source: data(*source),
            destination: *destination,
        },
        Inst::AddWordData {
            source,
            destination,
        } => Instruction::Add {
            size: OperandSize::Word,
            direction: DataDirection::ToDataRegister,
            data_register: *destination,
            effective_address: data(*source),
        },
        Inst::AddWordAddressIndirect {
            source,
            destination,
        } => Instruction::Add {
            size: OperandSize::Word,
            direction: DataDirection::ToDataRegister,
            data_register: *destination,
            effective_address: EffectiveAddress::AddressIndirect(*source),
        },
        Inst::AddqWordImmediateToDisplacementAddress {
            immediate,
            displacement,
            destination,
        } => Instruction::AddQuick {
            size: OperandSize::Word,
            immediate: *immediate,
            destination: address_displacement(*destination, *displacement),
        },
        Inst::Dbf { register, target } => {
            let target_address = label_address(origin, labels, target)?;
            let displacement = relative_displacement(origin, pc, target_address, target)?;
            Instruction::DecrementBranch {
                condition: Condition::False,
                register: *register,
                displacement: i16::try_from(displacement).map_err(|_| {
                    format!("68000 DBF to {target} is out of range: {displacement}")
                })?,
            }
        }
        Inst::DbfAbsolute { register, target } => {
            let displacement = relative_displacement(origin, pc, *target, "absolute target")?;
            Instruction::DecrementBranch {
                condition: Condition::False,
                register: *register,
                displacement: i16::try_from(displacement).map_err(|_| {
                    format!(
                        "68000 DBF absolute target 0x{target:08X} is out of range: {displacement}"
                    )
                })?,
            }
        }
        Inst::Branch {
            condition,
            width,
            target,
        } => {
            let target_address = label_address(origin, labels, target)?;
            Instruction::Branch {
                kind: branch_kind(*condition),
                displacement: branch_displacement(origin, pc, target_address, *width, target)?,
            }
        }
        Inst::BranchAbsolute {
            condition,
            width,
            target,
        } => Instruction::Branch {
            kind: branch_kind(*condition),
            displacement: branch_displacement(origin, pc, *target, *width, "absolute target")?,
        },
        Inst::JmpAbsoluteLong(address) => Instruction::Jump {
            target: EffectiveAddress::AbsoluteLong(*address),
        },
        Inst::JsrAbsoluteLong(address) => Instruction::JumpSubroutine {
            target: EffectiveAddress::AbsoluteLong(*address),
        },
        Inst::JsrProgramCounterDisplacement(target) => {
            let displacement =
                relative_displacement(origin, pc, *target, "PC-relative JSR target")?;
            Instruction::JumpSubroutine {
                target: EffectiveAddress::ProgramCounterDisplacement(
                    i16::try_from(displacement).map_err(|_| {
                        format!(
                            "68000 PC-relative JSR target 0x{target:08X} is out of range: {displacement}"
                        )
                    })?,
                ),
            }
        }
        Inst::Rts => Instruction::ReturnFromSubroutine,
        Inst::Nop => Instruction::Nop,
        Inst::Label(_) => return Ok(None),
    };
    Ok(Some(instruction))
}

fn instruction_size(inst: &Inst) -> Result<usize, String> {
    match inst {
        Inst::Label(_) => Ok(0),
        Inst::Branch {
            width: BranchWidth::Byte,
            ..
        }
        | Inst::BranchAbsolute {
            width: BranchWidth::Byte,
            ..
        } => Ok(2),
        Inst::Branch {
            width: BranchWidth::Word,
            ..
        }
        | Inst::BranchAbsolute {
            width: BranchWidth::Word,
            ..
        }
        | Inst::Dbf { .. }
        | Inst::DbfAbsolute { .. }
        | Inst::JsrProgramCounterDisplacement(_) => Ok(4),
        _ => {
            let instruction = lower_instruction(inst, 0, 0, &HashMap::new())?
                .expect("non-label instruction lowers to a shared instruction");
            m68000::encode_bytes(&instruction)
                .map(|bytes| bytes.len())
                .map_err(|error| format!("shared MC68000 encoding failed for {inst:?}: {error}"))
        }
    }
}

pub fn assemble(program: &[Inst]) -> Result<Vec<u8>, String> {
    assemble_at(0, program)
}

pub fn assemble_at(origin: u32, program: &[Inst]) -> Result<Vec<u8>, String> {
    let mut labels = HashMap::new();
    let mut offset = 0usize;
    for inst in program {
        if let Inst::Label(name) = inst
            && labels.insert(*name, offset).is_some()
        {
            return Err(format!("duplicate 68000 label: {name}"));
        }
        offset = offset
            .checked_add(instruction_size(inst)?)
            .ok_or_else(|| "68000 program size overflows usize".to_owned())?;
    }

    let mut output = Vec::with_capacity(offset);
    let mut pc = 0usize;
    for inst in program {
        let Some(instruction) = lower_instruction(inst, origin, pc, &labels)? else {
            continue;
        };
        let encoded = m68000::encode_bytes(&instruction)
            .map_err(|error| format!("shared MC68000 encoding failed for {inst:?}: {error}"))?;
        pc = pc
            .checked_add(encoded.len())
            .ok_or_else(|| "68000 program size overflows usize".to_owned())?;
        output.extend_from_slice(&encoded);
    }

    if pc != offset {
        return Err(format!(
            "shared MC68000 size drift: planned {offset} bytes, encoded {pc} bytes"
        ));
    }
    m68000::verify_placed_program(&output, CodeLocation::new(origin))
        .map_err(|error| format!("shared MC68000 verification failed: {error}"))?;
    Ok(output)
}

#[cfg(test)]
#[path = "m68k_tests.rs"]
mod tests;
