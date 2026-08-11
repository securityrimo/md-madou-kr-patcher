//! Native dynamic-display consumer contracts and fixed-width layout validation.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::build::text::Token;
use crate::m68k::{
    AddressReg, BranchCondition, BranchWidth, DataReg, Inst, assemble_at as assemble_m68k_at,
};

use super::{JP_MONSTER_NAME_BUFFER, JP_NATIVE_HALF_WIDTH_CHARS, JP_TEXT_REDIRECT_RETURN};

const JP_TEXT_OPCODE_HANDLER_TABLE: usize = 0x048D96;
pub(super) const JP_FF78_HANDLER_SLOT: usize = JP_TEXT_OPCODE_HANDLER_TABLE + 0x78;
pub(super) const JP_KR_FF78_FIXED_SLOT_HANDLER: usize = 0x33_E200;
const JP_DYNAMIC_DISPLAY_SHARED_HANDLER: usize = 0x0004_90BC;
const JP_DYNAMIC_DISPLAY_BUFFER_LOOP: usize = 0x0004_90C4;
const JP_DYNAMIC_DISPLAY_BUFFER_LOOP_END: usize = 0x0004_90DC;

fn expect_instruction(
    source: &[u8],
    offset: usize,
    instruction: Inst,
    label: &str,
) -> Result<(), String> {
    let expected = assemble_m68k_at(offset as u32, &[instruction])?;
    let actual = source
        .get(offset..offset + expected.len())
        .ok_or_else(|| format!("{label}: instruction at 0x{offset:06X} exceeds the JP ROM"))?;
    if actual != expected {
        return Err(format!(
            "{label}: typed 68000 instruction mismatch at 0x{offset:06X}"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DynamicDisplayControl {
    OneWord,
    TwoWords,
    FiveWords,
    SevenWords,
}

pub(super) const JP_DYNAMIC_DISPLAY_CONTROLS: [DynamicDisplayControl; 4] = [
    DynamicDisplayControl::OneWord,
    DynamicDisplayControl::TwoWords,
    DynamicDisplayControl::FiveWords,
    DynamicDisplayControl::SevenWords,
];

impl DynamicDisplayControl {
    pub(super) const fn from_code(code: u16) -> Option<Self> {
        match code {
            0xFF44 => Some(Self::OneWord),
            0xFF48 => Some(Self::TwoWords),
            0xFF4C => Some(Self::FiveWords),
            0xFF78 => Some(Self::SevenWords),
            _ => None,
        }
    }

    pub(super) const fn code(self) -> u16 {
        match self {
            Self::OneWord => 0xFF44,
            Self::TwoWords => 0xFF48,
            Self::FiveWords => 0xFF4C,
            Self::SevenWords => 0xFF78,
        }
    }

    pub(super) const fn handler(self) -> usize {
        match self {
            Self::OneWord => 0x0004_90BA,
            Self::TwoWords => 0x0004_90B2,
            Self::FiveWords => 0x0004_90B6,
            Self::SevenWords => 0x0004_90AE,
        }
    }

    pub(super) const fn loop_counter(self) -> i8 {
        match self {
            Self::OneWord => 0,
            Self::TwoWords => 1,
            Self::FiveWords => 4,
            Self::SevenWords => 6,
        }
    }

    pub(super) const fn visible_words(self) -> usize {
        self.loop_counter() as usize + 1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DynamicDisplayUseSpec {
    pub(super) id: &'static str,
    pub(super) control: DynamicDisplayControl,
    pub(super) occurrences: usize,
}

pub(super) const JP_DYNAMIC_DISPLAY_USE_SPECS: [DynamicDisplayUseSpec; 34] = [
    DynamicDisplayUseSpec {
        id: "script_0001",
        control: DynamicDisplayControl::OneWord,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0002",
        control: DynamicDisplayControl::OneWord,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0005",
        control: DynamicDisplayControl::TwoWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0006",
        control: DynamicDisplayControl::OneWord,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0007",
        control: DynamicDisplayControl::TwoWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0008",
        control: DynamicDisplayControl::OneWord,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0023",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0024",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0025",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0027",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0030",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0322",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0450",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0451",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 2,
    },
    DynamicDisplayUseSpec {
        id: "script_0454",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_0849",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1333",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1000",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1001",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1031",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1034",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1181",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1188",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1197",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1199",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1205",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1206",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1207",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1214",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1215",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1221",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1222",
        control: DynamicDisplayControl::FiveWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1231",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
    DynamicDisplayUseSpec {
        id: "script_1237",
        control: DynamicDisplayControl::SevenWords,
        occurrences: 1,
    },
];
pub(super) const JP_DYNAMIC_DISPLAY_EXCLUDED_ASSET_IDS: [&str; 3] = [
    "plural_amigo_defeated",
    "plural_capsule_prompt",
    "plural_capsule_released",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DynamicTrailingTileReplacement {
    KrGlyph(char),
    Blank,
    Control(u16),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DynamicTrailingTileSpec {
    pub(super) id: &'static str,
    pub(super) control: u16,
    pub(super) jp_tile: u16,
    pub(super) occurrences: usize,
    pub(super) replacement: DynamicTrailingTileReplacement,
}

pub(super) const M7_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 3] = [
    DynamicTrailingTileSpec {
        id: "script_0322",
        control: 0xFF78,
        jp_tile: 0x003C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Control(0xFF30),
    },
    DynamicTrailingTileSpec {
        id: "script_0450",
        control: 0xFF78,
        jp_tile: 0x00A0,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Control(0xFF30),
    },
    DynamicTrailingTileSpec {
        id: "script_0451",
        control: 0xFF78,
        jp_tile: 0x0029,
        occurrences: 2,
        replacement: DynamicTrailingTileReplacement::Control(0xFF30),
    },
];

pub(super) const M24_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 1] =
    [DynamicTrailingTileSpec {
        id: "script_0849",
        control: 0xFF78,
        jp_tile: 0x003C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Control(0xFF30),
    }];

pub(super) const M54_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 1] =
    [DynamicTrailingTileSpec {
        id: "script_1333",
        control: 0xFF78,
        jp_tile: 0x003C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Control(0xFF30),
    }];

pub(super) const M32_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 1] =
    [DynamicTrailingTileSpec {
        id: "script_1000",
        control: 0xFF4C,
        jp_tile: 0xFF30,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    }];

pub(super) const M43_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 2] = [
    DynamicTrailingTileSpec {
        id: "script_1181",
        control: 0xFF4C,
        jp_tile: 0x000E,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    },
    DynamicTrailingTileSpec {
        id: "script_1188",
        control: 0xFF4C,
        jp_tile: 0x000E,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    },
];

pub(super) const M44_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 4] = [
    DynamicTrailingTileSpec {
        id: "script_1197",
        control: 0xFF4C,
        jp_tile: 0x0025,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('임'),
    },
    DynamicTrailingTileSpec {
        id: "script_1199",
        control: 0xFF4C,
        jp_tile: 0x0024,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    },
    DynamicTrailingTileSpec {
        id: "script_1205",
        control: 0xFF4C,
        jp_tile: 0x0025,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('임'),
    },
    DynamicTrailingTileSpec {
        id: "script_1207",
        control: 0xFF4C,
        jp_tile: 0x0024,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    },
];

pub(super) const M45_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 2] = [
    DynamicTrailingTileSpec {
        id: "script_1214",
        control: 0xFF4C,
        jp_tile: 0x000E,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    },
    DynamicTrailingTileSpec {
        id: "script_1221",
        control: 0xFF4C,
        jp_tile: 0x00AD,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    },
];

pub(super) const M46_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 1] =
    [DynamicTrailingTileSpec {
        id: "script_1231",
        control: 0xFF78,
        jp_tile: 0x00A0,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Blank,
    }];

pub(super) const M55_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 6] = [
    DynamicTrailingTileSpec {
        id: "script_0001",
        control: 0xFF44,
        jp_tile: 0x0025,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    },
    DynamicTrailingTileSpec {
        id: "script_0002",
        control: 0xFF44,
        jp_tile: 0x0025,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('에'),
    },
    DynamicTrailingTileSpec {
        id: "script_0005",
        control: 0xFF48,
        jp_tile: 0x014C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('층'),
    },
    DynamicTrailingTileSpec {
        id: "script_0006",
        control: 0xFF44,
        jp_tile: 0x014C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('층'),
    },
    DynamicTrailingTileSpec {
        id: "script_0007",
        control: 0xFF48,
        jp_tile: 0x014C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('층'),
    },
    DynamicTrailingTileSpec {
        id: "script_0008",
        control: 0xFF44,
        jp_tile: 0x014C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::KrGlyph('층'),
    },
];

pub(super) const M56_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 5] = [
    DynamicTrailingTileSpec {
        id: "script_0023",
        control: 0xFF78,
        jp_tile: 0x003C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Blank,
    },
    DynamicTrailingTileSpec {
        id: "script_0024",
        control: 0xFF78,
        jp_tile: 0x003C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Blank,
    },
    DynamicTrailingTileSpec {
        id: "script_0025",
        control: 0xFF78,
        jp_tile: 0x003C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Blank,
    },
    DynamicTrailingTileSpec {
        id: "script_0027",
        control: 0xFF78,
        jp_tile: 0x003C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Blank,
    },
    DynamicTrailingTileSpec {
        id: "script_0030",
        control: 0xFF78,
        jp_tile: 0x003C,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Blank,
    },
];

pub(super) const M8_DYNAMIC_TRAILING_TILE_SPECS: [DynamicTrailingTileSpec; 1] =
    [DynamicTrailingTileSpec {
        id: "script_0454",
        control: 0xFF78,
        jp_tile: 0x0029,
        occurrences: 1,
        replacement: DynamicTrailingTileReplacement::Blank,
    }];

pub(super) fn validate_dynamic_display_consumer_contract(source: &[u8]) -> Result<(), String> {
    for consumer in JP_DYNAMIC_DISPLAY_CONTROLS {
        let table_offset = JP_TEXT_OPCODE_HANDLER_TABLE + usize::from(consumer.code() & 0x00FF);
        let pointer = source
            .get(table_offset..table_offset + 4)
            .ok_or_else(|| {
                format!(
                    "dynamic display control 0x{:04X} handler pointer is outside the JP ROM",
                    consumer.code()
                )
            })?
            .try_into()
            .expect("four-byte handler pointer slice");
        if u32::from_be_bytes(pointer) as usize != consumer.handler() {
            return Err(format!(
                "dynamic display control 0x{:04X} no longer dispatches to 0x{:06X}",
                consumer.code(),
                consumer.handler()
            ));
        }

        expect_instruction(
            source,
            consumer.handler(),
            Inst::Moveq {
                immediate: consumer.loop_counter(),
                destination: DataReg::D7,
            },
            &format!(
                "dynamic display control 0x{:04X} fixed buffer bound",
                consumer.code()
            ),
        )?;
        if consumer.handler() + 2 != JP_DYNAMIC_DISPLAY_SHARED_HANDLER {
            expect_instruction(
                source,
                consumer.handler() + 2,
                Inst::BranchAbsolute {
                    condition: BranchCondition::Always,
                    width: BranchWidth::Byte,
                    target: JP_DYNAMIC_DISPLAY_SHARED_HANDLER as u32,
                },
                &format!(
                    "dynamic display control 0x{:04X} shared buffer loop branch",
                    consumer.code()
                ),
            )?;
        }
    }

    expect_instruction(
        source,
        JP_DYNAMIC_DISPLAY_SHARED_HANDLER,
        Inst::MoveAddressLongToDisplacementAddress {
            source: AddressReg::A1,
            displacement: 0x0040,
            destination: AddressReg::A0,
        },
        "dynamic display trailing-word cursor preservation",
    )?;
    expect_instruction(
        source,
        JP_DYNAMIC_DISPLAY_SHARED_HANDLER + 4,
        Inst::LeaAbsoluteShort {
            address: JP_MONSTER_NAME_BUFFER,
            destination: AddressReg::A3,
        },
        "dynamic display buffer source",
    )?;
    expect_instruction(
        source,
        JP_DYNAMIC_DISPLAY_BUFFER_LOOP,
        Inst::MoveWordPostincrementAddressToData {
            source: AddressReg::A3,
            destination: DataReg::D0,
        },
        "dynamic display fixed buffer read",
    )?;
    expect_instruction(
        source,
        JP_DYNAMIC_DISPLAY_BUFFER_LOOP_END,
        Inst::DbfAbsolute {
            register: DataReg::D7,
            target: JP_DYNAMIC_DISPLAY_BUFFER_LOOP as u32,
        },
        "dynamic display fixed buffer loop",
    )?;
    expect_instruction(
        source,
        JP_DYNAMIC_DISPLAY_BUFFER_LOOP_END + 4,
        Inst::BranchAbsolute {
            condition: BranchCondition::Always,
            width: BranchWidth::Word,
            target: JP_TEXT_REDIRECT_RETURN,
        },
        "dynamic display trailing-word redispatch",
    )?;
    Ok(())
}

pub(super) fn assemble_ff78_fixed_slot_handler(half_space_code: u16) -> Result<Vec<u8>, String> {
    assemble_m68k_at(
        JP_KR_FF78_FIXED_SLOT_HANDLER as u32,
        &[
            Inst::MoveLongDataToPredecrementAddress {
                source: DataReg::D6,
                destination: AddressReg::A7,
            },
            Inst::Moveq {
                immediate: 0,
                destination: DataReg::D6,
            },
            Inst::Moveq {
                immediate: DynamicDisplayControl::SevenWords.loop_counter(),
                destination: DataReg::D7,
            },
            Inst::MoveAddressLongToDisplacementAddress {
                source: AddressReg::A1,
                displacement: 0x0040,
                destination: AddressReg::A0,
            },
            Inst::LeaAbsoluteShort {
                address: JP_MONSTER_NAME_BUFFER,
                destination: AddressReg::A3,
            },
            Inst::Label("ff78_word_loop"),
            Inst::MoveWordPostincrementAddressToData {
                source: AddressReg::A3,
                destination: DataReg::D0,
            },
            Inst::CmpiWordImmediate {
                immediate: half_space_code,
                destination: DataReg::D0,
            },
            Inst::Branch {
                condition: BranchCondition::NotEqual,
                width: BranchWidth::Word,
                target: "ff78_render_word",
            },
            Inst::AddiWordImmediate {
                immediate: 1,
                destination: DataReg::D6,
            },
            Inst::Label("ff78_render_word"),
            Inst::JsrAbsoluteLong(0x0004_91D6),
            Inst::MoveByteDisplacementAddressToData {
                displacement: 0x0052,
                source: AddressReg::A0,
                destination: DataReg::D0,
            },
            Inst::Branch {
                condition: BranchCondition::Equal,
                width: BranchWidth::Word,
                target: "ff78_skip_wait",
            },
            Inst::JsrAbsoluteLong(0x0006_3470),
            Inst::Label("ff78_skip_wait"),
            Inst::MoveWordDisplacementAddressToDisplacementAddress {
                source_displacement: 0x002A,
                source: AddressReg::A0,
                destination_displacement: 0x0026,
                destination: AddressReg::A0,
            },
            Inst::Dbf {
                register: DataReg::D7,
                target: "ff78_word_loop",
            },
            Inst::CmpiWordImmediate {
                immediate: 0,
                destination: DataReg::D6,
            },
            Inst::Branch {
                condition: BranchCondition::Equal,
                width: BranchWidth::Word,
                target: "ff78_done",
            },
            Inst::Label("ff78_compensate_slot"),
            Inst::AddqWordImmediateToDisplacementAddress {
                immediate: 2,
                displacement: 0x004C,
                destination: AddressReg::A0,
            },
            Inst::SubiWordImmediate {
                immediate: 1,
                destination: DataReg::D6,
            },
            Inst::Branch {
                condition: BranchCondition::NotEqual,
                width: BranchWidth::Word,
                target: "ff78_compensate_slot",
            },
            Inst::Label("ff78_done"),
            Inst::MoveLongPostincrementAddressToData {
                source: AddressReg::A7,
                destination: DataReg::D6,
            },
            Inst::JmpAbsoluteLong(JP_TEXT_REDIRECT_RETURN),
        ],
    )
}

pub(super) fn validate_ff78_fixed_slot_handler(
    rom: &[u8],
    half_space_code: u16,
) -> Result<(), String> {
    let pointer = rom
        .get(JP_FF78_HANDLER_SLOT..JP_FF78_HANDLER_SLOT + 4)
        .ok_or("FF78 handler pointer is outside the ROM")?;
    if pointer != (JP_KR_FF78_FIXED_SLOT_HANDLER as u32).to_be_bytes() {
        return Err("FF78 no longer dispatches to the KR fixed-slot handler".into());
    }
    let handler = assemble_ff78_fixed_slot_handler(half_space_code)?;
    if rom.get(JP_KR_FF78_FIXED_SLOT_HANDLER..JP_KR_FF78_FIXED_SLOT_HANDLER + handler.len())
        != Some(handler.as_slice())
    {
        return Err("typed FF78 fixed-slot handler reassembly differs".into());
    }
    Ok(())
}

pub(super) fn localize_dynamic_trailing_tiles(
    id: &str,
    tokens: &mut [Token],
    charmap: &BTreeMap<char, u16>,
) -> Result<(), String> {
    for spec in M7_DYNAMIC_TRAILING_TILE_SPECS
        .iter()
        .chain(M8_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M24_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M54_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M32_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M43_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M44_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M45_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M46_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M55_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .chain(M56_DYNAMIC_TRAILING_TILE_SPECS.iter())
        .filter(|spec| spec.id == id)
    {
        let replacement_code = match spec.replacement {
            DynamicTrailingTileReplacement::KrGlyph(glyph) => *charmap
                .get(&glyph)
                .ok_or_else(|| format!("{id}: KR dynamic suffix glyph {glyph:?} is absent"))?,
            DynamicTrailingTileReplacement::Blank => 0x0000,
            DynamicTrailingTileReplacement::Control(code) => code,
        };
        let mut replacements = 0usize;
        for token in tokens.iter_mut() {
            let Token::CtrlParam(control, tile) = token else {
                continue;
            };
            if *control != spec.control {
                continue;
            }
            if *tile != spec.jp_tile {
                return Err(format!(
                    "{id}: dynamic control 0x{control:04X} has tile 0x{tile:04X}, expected JP tile 0x{:04X}",
                    spec.jp_tile
                ));
            }
            *tile = replacement_code;
            replacements += 1;
        }
        if replacements != spec.occurrences {
            return Err(format!(
                "{id}: expected {} dynamic control 0x{:04X} occurrence(s), found {replacements}",
                spec.occurrences, spec.control
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_dynamic_display_population(
    id: &str,
    tokens: &[Token],
) -> Result<(), String> {
    let mut expected = BTreeMap::new();
    for spec in JP_DYNAMIC_DISPLAY_USE_SPECS
        .iter()
        .filter(|spec| spec.id == id)
    {
        *expected.entry(spec.control.code()).or_insert(0usize) += spec.occurrences;
    }

    let mut actual = BTreeMap::new();
    for token in tokens {
        match token {
            Token::CtrlParam(code, _) if DynamicDisplayControl::from_code(*code).is_some() => {
                *actual.entry(*code).or_insert(0usize) += 1;
            }
            Token::Ctrl(code) if DynamicDisplayControl::from_code(*code).is_some() => {
                return Err(format!(
                    "{id}: dynamic display control 0x{code:04X} is missing its trailing display word"
                ));
            }
            _ => {}
        }
    }

    if actual != expected {
        return Err(format!(
            "{id}: dynamic display population differs: expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

pub(super) fn validate_dynamic_display_asset_ledger(translation_dir: &Path) -> Result<(), String> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(translation_dir)
        .map_err(|error| format!("failed to read {}: {error}", translation_dir.display()))?
    {
        let path = entry
            .map_err(|error| format!("failed to read translation directory entry: {error}"))?
            .path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("script_") && name.ends_with(".json"))
        {
            paths.push(path);
        }
    }
    paths.sort();

    let mut actual = BTreeMap::new();
    let mut excluded = BTreeSet::new();
    for path in paths {
        let data = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let root: serde_json::Value = serde_json::from_str(&data)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        let entries = root
            .get("entries")
            .and_then(|value| value.as_array())
            .ok_or_else(|| format!("{}: missing entries array", path.display()))?;
        for entry in entries {
            let Some(id) = entry.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(ko) = entry.get("ko").and_then(|value| value.as_str()) else {
                continue;
            };
            for token in crate::build::text::parse_display_text(ko) {
                let Token::CtrlParam(code, _) = token else {
                    continue;
                };
                if DynamicDisplayControl::from_code(code).is_none() {
                    continue;
                }
                if JP_DYNAMIC_DISPLAY_EXCLUDED_ASSET_IDS.contains(&id) {
                    excluded.insert(id.to_string());
                } else {
                    *actual.entry((id.to_string(), code)).or_insert(0usize) += 1;
                }
            }
        }
    }

    let expected = JP_DYNAMIC_DISPLAY_USE_SPECS
        .iter()
        .map(|spec| ((spec.id.to_string(), spec.control.code()), spec.occurrences))
        .collect::<BTreeMap<_, _>>();
    if actual != expected {
        return Err(format!(
            "dynamic display asset population differs: expected {expected:?}, found {actual:?}"
        ));
    }

    let expected_excluded = JP_DYNAMIC_DISPLAY_EXCLUDED_ASSET_IDS
        .iter()
        .map(|id| id.to_string())
        .collect::<BTreeSet<_>>();
    if excluded != expected_excluded {
        return Err(format!(
            "dynamic display excluded asset population differs: expected {expected_excluded:?}, found {excluded:?}"
        ));
    }
    Ok(())
}

pub(super) fn validate_fixed_width_layout(
    tokens: &[Token],
    id: &str,
    max_glyphs_per_line: usize,
    max_lines_per_page: Option<usize>,
) -> Result<(), String> {
    let mut line = 1usize;
    let max_half_cells = max_glyphs_per_line * 2;
    let mut half_cells = 0usize;
    let mut page_start = 0usize;
    let mut line_start = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Ctrl(0xFF30) => {
                line += 1;
                half_cells = 0;
                line_start = index + 1;
            }
            Token::Ctrl(0xFF10 | 0xFF14 | 0xFF18 | 0xFF1C | 0xFF34 | 0xFF38 | 0xFFB4 | 0xFFB8) => {
                line = 1;
                half_cells = 0;
                page_start = index + 1;
                line_start = index + 1;
            }
            Token::KrChar(_)
            | Token::EnChar(_)
            | Token::Tile(_)
            | Token::LayoutPad
            | Token::Raw(_) => {
                validate_fixed_width_page_line(
                    tokens,
                    id,
                    index,
                    line,
                    page_start,
                    max_lines_per_page,
                )?;
                half_cells += fixed_width_token_half_cells(token);
                if half_cells > max_half_cells {
                    return Err(format!(
                        "{id}: fixed-width line {line} exceeds {max_glyphs_per_line} cells"
                    ));
                }
                if half_cells == max_half_cells && is_terminal_half_width_punctuation(token) {
                    return Err(format!(
                        "{id}: fixed-width line {line} leaves no half-cell right margin after terminal punctuation: {:?}",
                        fixed_width_page_preview(&tokens[line_start..=index])
                    ));
                }
            }
            Token::SourceRowFinalize { .. } => {
                // This source-proven operation is generated after semantic
                // layout validation. It clears the rest of the physical row
                // and restores its border without extending KR content.
                validate_fixed_width_page_line(
                    tokens,
                    id,
                    index,
                    line,
                    page_start,
                    max_lines_per_page,
                )?;
            }
            Token::CtrlParam(code, trailing_word)
                if let Some(consumer) = DynamicDisplayControl::from_code(*code) =>
            {
                validate_fixed_width_page_line(
                    tokens,
                    id,
                    index,
                    line,
                    page_start,
                    max_lines_per_page,
                )?;
                add_fixed_width_half_cells(
                    &mut half_cells,
                    consumer.visible_words() * 2,
                    id,
                    line,
                    max_glyphs_per_line,
                    &format!(
                        "dynamic control 0x{code:04X} {}-word buffer",
                        consumer.visible_words()
                    ),
                )?;
                if *trailing_word < 0xFF00 {
                    add_fixed_width_half_cells(
                        &mut half_cells,
                        dynamic_trailing_word_half_cells(*trailing_word),
                        id,
                        line,
                        max_glyphs_per_line,
                        &format!("dynamic control 0x{code:04X} trailing glyph"),
                    )?;
                } else {
                    match *trailing_word {
                        0xFF30 => {
                            if tokens.get(index + 1) == Some(&Token::Ctrl(0xFF30)) {
                                return Err(format!(
                                    "{id}: dynamic control 0x{code:04X} trailing word already breaks the line before an explicit NL"
                                ));
                            }
                            line += 1;
                            half_cells = 0;
                            line_start = index + 1;
                        }
                        0xFF04 | 0xFFFF => {}
                        0xFF10 | 0xFF14 | 0xFF18 | 0xFF1C | 0xFF34 | 0xFF38 | 0xFFB4 | 0xFFB8 => {
                            line = 1;
                            half_cells = 0;
                            page_start = index + 1;
                            line_start = index + 1;
                        }
                        _ => {
                            return Err(format!(
                                "{id}: dynamic control 0x{code:04X} has unsupported trailing control 0x{trailing_word:04X}"
                            ));
                        }
                    }
                }
            }
            Token::Ctrl(_) | Token::CtrlParam(_, _) => {}
        }
    }
    Ok(())
}

pub(super) fn dynamic_trailing_word_half_cells(trailing_word: u16) -> usize {
    match trailing_word {
        0 => 1,
        0x0001..=0xFEFF => 2,
        _ => 0,
    }
}

pub(super) fn fixed_width_owned_row_half_cells(max_glyphs_per_line: usize) -> usize {
    // The semantic limit reserves one final half-cell as the right margin.
    // In-place overwrites own that margin too: clearing only the eight
    // full-width content cells leaves the old rightmost 8px tile visible.
    max_glyphs_per_line * 2 + 1
}

fn validate_fixed_width_page_line(
    tokens: &[Token],
    id: &str,
    index: usize,
    line: usize,
    page_start: usize,
    max_lines_per_page: Option<usize>,
) -> Result<(), String> {
    let Some(max_lines) = max_lines_per_page else {
        return Ok(());
    };
    if line <= max_lines {
        return Ok(());
    }

    let page_end = tokens[index..]
        .iter()
        .position(is_fixed_width_page_reset)
        .map(|relative| index + relative)
        .unwrap_or(tokens.len());
    Err(format!(
        "{id}: fixed-width page exceeds {max_lines} lines: {:?}",
        fixed_width_page_preview(&tokens[page_start..page_end])
    ))
}

fn is_fixed_width_page_reset(token: &Token) -> bool {
    matches!(
        token,
        Token::Ctrl(0xFF10 | 0xFF14 | 0xFF18 | 0xFF1C | 0xFF34 | 0xFF38 | 0xFFB4 | 0xFFB8)
    )
}

fn fixed_width_page_preview(tokens: &[Token]) -> String {
    let mut preview = String::new();
    for token in tokens {
        match token {
            Token::KrChar(ch) | Token::EnChar(ch) => preview.push(*ch),
            Token::Ctrl(0xFF30) => preview.push('\n'),
            Token::CtrlParam(code, _) => {
                preview.push_str(&format!("{{{code:04X}}}"));
            }
            Token::Tile(code) | Token::Raw(code) => {
                preview.push_str(&format!("[{code:04X}]"));
            }
            Token::LayoutPad => preview.push_str("{pad}"),
            Token::SourceRowFinalize { clear_half_cells } => {
                preview.push_str(&format!("{{finalize:{clear_half_cells}}}"));
            }
            Token::Ctrl(_) => {}
        }
    }
    preview
}

pub(super) fn fixed_width_token_half_cells(token: &Token) -> usize {
    match token {
        Token::EnChar(ch) if JP_NATIVE_HALF_WIDTH_CHARS.contains(ch) => 1,
        Token::KrChar(_) | Token::EnChar(_) | Token::Tile(_) | Token::LayoutPad | Token::Raw(_) => {
            2
        }
        Token::Ctrl(_) | Token::CtrlParam(_, _) | Token::SourceRowFinalize { .. } => 0,
    }
}

fn is_terminal_half_width_punctuation(token: &Token) -> bool {
    matches!(token, Token::EnChar('.' | '?' | '!' | ',' | '*' | '~'))
}

pub(super) fn add_fixed_width_half_cells(
    half_cells: &mut usize,
    added: usize,
    id: &str,
    line: usize,
    max_glyphs_per_line: usize,
    source: &str,
) -> Result<(), String> {
    *half_cells += added;
    if *half_cells > max_glyphs_per_line * 2 {
        return Err(format!(
            "{id}: fixed-width line {line} exceeds {max_glyphs_per_line} cells after {source}"
        ));
    }
    Ok(())
}
