//! JP-source title prompt layout and typed consumer patches.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::m68k::{AddressReg, Inst, assemble};

use super::{parse_hex, read_u16, source_range};

const EXPECTED_STRUCTURE_BANK_OFFSET: usize = 0x26_0000;
const STRUCTURE_BYTES: usize = 10;
const TILE_WORD_OFFSET: usize = 6;
const TILE_INDEX_MASK: u16 = 0x07FF;

#[derive(Debug, Deserialize)]
struct TitlePromptManifest {
    structure_bank_offset: String,
    logical_prompts: Vec<LogicalPromptSource>,
    fragments: Vec<PromptFragmentSource>,
}

#[derive(Debug, Deserialize)]
struct LogicalPromptSource {
    id: String,
    ko: String,
    fragment_ids: Vec<String>,
    base_x_instruction_offset: String,
    original_base_x: String,
    base_x: String,
}

#[derive(Debug, Deserialize)]
struct PromptFragmentSource {
    id: String,
    ko_layout: String,
    original_structure_offset: String,
    original_slots: usize,
    #[serde(default)]
    pointer_instruction_offset: Option<String>,
    #[serde(default)]
    continues_after: Option<String>,
    count_instruction_offset: String,
}

#[derive(Debug)]
pub(super) struct TitlePromptPlan {
    pub(super) structure_bank_offset: usize,
    pub(super) logical_prompts: Vec<LogicalPrompt>,
    pub(super) fragments: Vec<PromptFragment>,
}

#[derive(Debug)]
pub(super) struct LogicalPrompt {
    pub(super) id: String,
    pub(super) ko: String,
    fragment_ids: Vec<String>,
    base_x_instruction_offset: usize,
    original_base_x: u16,
    base_x: u16,
}

#[derive(Debug)]
pub(super) struct PromptFragment {
    pub(super) id: String,
    pub(super) ko_layout: String,
    original_structure_offset: usize,
    original_slots: usize,
    pointer_instruction_offset: Option<usize>,
    continues_after: Option<String>,
    count_instruction_offset: usize,
}

#[derive(Debug)]
pub(super) struct TitlePromptBuild {
    pub(super) structure_bank_offset: usize,
    pub(super) structure_bank: Vec<u8>,
    pub(super) code_patches: Vec<CodePatch>,
}

#[derive(Debug)]
pub(super) struct CodePatch {
    pub(super) id: String,
    pub(super) offset: usize,
    pub(super) expected: Vec<u8>,
    pub(super) replacement: Vec<u8>,
}

pub(super) fn read_title_prompt_plan(assets_dir: &Path) -> Result<TitlePromptPlan, String> {
    let path = assets_dir.join("graphics_text/title_prompt.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read title-prompt source {}: {error}",
            path.display()
        )
    })?;
    let manifest: TitlePromptManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid title-prompt source {}: {error}", path.display()))?;
    let structure_bank_offset = parse_hex(&manifest.structure_bank_offset)?;
    if structure_bank_offset != EXPECTED_STRUCTURE_BANK_OFFSET {
        return Err(format!(
            "title-prompt structure bank drifted: expected 0x{EXPECTED_STRUCTURE_BANK_OFFSET:06X}, got 0x{structure_bank_offset:06X}"
        ));
    }

    let mut fragment_ids = BTreeSet::new();
    let mut fragments = Vec::with_capacity(manifest.fragments.len());
    for source in manifest.fragments {
        if !fragment_ids.insert(source.id.clone()) {
            return Err(format!("duplicate title-prompt fragment {}", source.id));
        }
        let slots = source.ko_layout.chars().count();
        if slots == 0 {
            return Err(format!("title-prompt fragment {} is empty", source.id));
        }
        if source.original_slots == 0 {
            return Err(format!(
                "title-prompt fragment {} has no JP source slots",
                source.id
            ));
        }
        if source.pointer_instruction_offset.is_some() == source.continues_after.is_some() {
            return Err(format!(
                "title-prompt fragment {} must declare exactly one of pointer_instruction_offset or continues_after",
                source.id
            ));
        }
        fragments.push(PromptFragment {
            id: source.id,
            ko_layout: source.ko_layout,
            original_structure_offset: parse_hex(&source.original_structure_offset)?,
            original_slots: source.original_slots,
            pointer_instruction_offset: source
                .pointer_instruction_offset
                .as_deref()
                .map(parse_hex)
                .transpose()?,
            continues_after: source.continues_after,
            count_instruction_offset: parse_hex(&source.count_instruction_offset)?,
        });
    }

    let fragment_by_id: BTreeMap<_, _> = fragments
        .iter()
        .map(|fragment| (fragment.id.as_str(), fragment))
        .collect();
    let mut logical_ids = BTreeSet::new();
    let mut referenced_fragments = BTreeSet::new();
    let mut logical_prompts = Vec::with_capacity(manifest.logical_prompts.len());
    for source in manifest.logical_prompts {
        if !logical_ids.insert(source.id.clone()) {
            return Err(format!("duplicate logical title prompt {}", source.id));
        }
        if source.fragment_ids.is_empty() {
            return Err(format!(
                "logical title prompt {} has no fragments",
                source.id
            ));
        }
        let composed = source
            .fragment_ids
            .iter()
            .map(|id| {
                let fragment = fragment_by_id.get(id.as_str()).ok_or_else(|| {
                    format!("logical title prompt {} references unknown {id}", source.id)
                })?;
                if !referenced_fragments.insert(id.clone()) {
                    return Err(format!(
                        "title-prompt fragment {id} is referenced more than once"
                    ));
                }
                Ok(fragment.ko_layout.as_str())
            })
            .collect::<Result<Vec<_>, String>>()?
            .join(" ");
        if composed != source.ko {
            return Err(format!(
                "logical title prompt {} composes to {composed:?}, not {:?}",
                source.id, source.ko
            ));
        }
        logical_prompts.push(LogicalPrompt {
            id: source.id,
            ko: source.ko,
            fragment_ids: source.fragment_ids,
            base_x_instruction_offset: parse_hex(&source.base_x_instruction_offset)?,
            original_base_x: parse_u16_hex(&source.original_base_x)?,
            base_x: parse_u16_hex(&source.base_x)?,
        });
    }
    if referenced_fragments != fragment_ids {
        let unreferenced: Vec<_> = fragment_ids
            .difference(&referenced_fragments)
            .cloned()
            .collect();
        return Err(format!(
            "unreferenced title-prompt fragments: {unreferenced:?}"
        ));
    }

    Ok(TitlePromptPlan {
        structure_bank_offset,
        logical_prompts,
        fragments,
    })
}

pub(super) fn build_title_prompt(
    source: &[u8],
    plan: &TitlePromptPlan,
    tile_by_char: &BTreeMap<char, u16>,
    blank_tile: u16,
) -> Result<TitlePromptBuild, String> {
    let template = canonical_structure_template(source, plan)?;
    let mut structure_bank = Vec::new();
    let mut new_offset_by_fragment = BTreeMap::new();

    for fragment in &plan.fragments {
        validate_original_structures(source, fragment, &template)?;
        let new_offset = plan.structure_bank_offset + structure_bank.len();
        new_offset_by_fragment.insert(fragment.id.as_str(), new_offset);
        for ch in fragment.ko_layout.chars() {
            let tile = if ch.is_whitespace() {
                blank_tile
            } else {
                *tile_by_char.get(&ch).ok_or_else(|| {
                    format!(
                        "title-prompt fragment {} glyph {ch:?} was not allocated",
                        fragment.id
                    )
                })?
            };
            let mut structure = template;
            let source_word = read_u16(&structure, TILE_WORD_OFFSET, &fragment.id)?;
            let replacement_word = (source_word & !TILE_INDEX_MASK) | tile;
            structure[TILE_WORD_OFFSET..TILE_WORD_OFFSET + 2]
                .copy_from_slice(&replacement_word.to_be_bytes());
            structure_bank.extend_from_slice(&structure);
        }
    }

    let mut code_patches = Vec::new();
    for fragment in &plan.fragments {
        let new_slots = fragment.ko_layout.chars().count();
        let expected_count =
            count_instruction(u16_from_usize(fragment.original_slots, &fragment.id)?)?;
        let replacement_count = count_instruction(u16_from_usize(new_slots, &fragment.id)?)?;
        validate_instruction(
            source,
            fragment.count_instruction_offset,
            &expected_count,
            &format!("{} JP draw count", fragment.id),
        )?;
        if expected_count != replacement_count {
            code_patches.push(CodePatch {
                id: format!("{} draw count", fragment.id),
                offset: fragment.count_instruction_offset,
                expected: expected_count,
                replacement: replacement_count,
            });
        }

        let new_offset = *new_offset_by_fragment
            .get(fragment.id.as_str())
            .expect("every fragment was assigned a structure offset");
        if let Some(pointer_offset) = fragment.pointer_instruction_offset {
            let expected_pointer = pointer_instruction(fragment.original_structure_offset)?;
            let replacement_pointer = pointer_instruction(new_offset)?;
            validate_instruction(
                source,
                pointer_offset,
                &expected_pointer,
                &format!("{} JP structure pointer", fragment.id),
            )?;
            code_patches.push(CodePatch {
                id: format!("{} structure pointer", fragment.id),
                offset: pointer_offset,
                expected: expected_pointer,
                replacement: replacement_pointer,
            });
        } else {
            let predecessor_id = fragment
                .continues_after
                .as_deref()
                .expect("plan validation requires a continuation predecessor");
            let predecessor = plan
                .fragments
                .iter()
                .find(|candidate| candidate.id == predecessor_id)
                .ok_or_else(|| {
                    format!(
                        "{} continues after unknown fragment {predecessor_id}",
                        fragment.id
                    )
                })?;
            let original_expected = predecessor.original_structure_offset
                + predecessor.original_slots * STRUCTURE_BYTES;
            if fragment.original_structure_offset != original_expected {
                return Err(format!(
                    "{} JP structure is not contiguous after {predecessor_id}",
                    fragment.id
                ));
            }
            let predecessor_new = *new_offset_by_fragment
                .get(predecessor_id)
                .expect("predecessor was assigned a structure offset");
            let predecessor_new_end =
                predecessor_new + predecessor.ko_layout.chars().count() * STRUCTURE_BYTES;
            if new_offset != predecessor_new_end {
                return Err(format!(
                    "{} generated structure is not contiguous after {predecessor_id}",
                    fragment.id
                ));
            }
        }
    }

    for logical in &plan.logical_prompts {
        for pair in logical.fragment_ids.windows(2) {
            let left = new_offset_by_fragment
                .get(pair[0].as_str())
                .ok_or_else(|| format!("{} has unknown fragment {}", logical.id, pair[0]))?;
            let left_fragment = plan
                .fragments
                .iter()
                .find(|fragment| fragment.id == pair[0])
                .expect("logical prompt fragments were validated");
            let right = new_offset_by_fragment
                .get(pair[1].as_str())
                .ok_or_else(|| format!("{} has unknown fragment {}", logical.id, pair[1]))?;
            if left + left_fragment.ko_layout.chars().count() * STRUCTURE_BYTES != *right {
                return Err(format!(
                    "{} fragments {} and {} are not generated contiguously",
                    logical.id, pair[0], pair[1]
                ));
            }
        }

        let expected_x = x_instruction(logical.original_base_x)?;
        let replacement_x = x_instruction(logical.base_x)?;
        validate_instruction(
            source,
            logical.base_x_instruction_offset,
            &expected_x,
            &format!("{} JP base X", logical.id),
        )?;
        if expected_x != replacement_x {
            code_patches.push(CodePatch {
                id: format!("{} base X", logical.id),
                offset: logical.base_x_instruction_offset,
                expected: expected_x,
                replacement: replacement_x,
            });
        }
    }

    code_patches.sort_by_key(|patch| patch.offset);
    for pair in code_patches.windows(2) {
        if pair[0].offset + pair[0].replacement.len() > pair[1].offset {
            return Err(format!(
                "title-prompt code patches {} and {} overlap",
                pair[0].id, pair[1].id
            ));
        }
    }

    Ok(TitlePromptBuild {
        structure_bank_offset: plan.structure_bank_offset,
        structure_bank,
        code_patches,
    })
}

fn canonical_structure_template(
    source: &[u8],
    plan: &TitlePromptPlan,
) -> Result<[u8; STRUCTURE_BYTES], String> {
    let first = plan
        .fragments
        .first()
        .ok_or_else(|| "title-prompt plan has no fragments".to_string())?;
    let bytes = source_range(
        source,
        first.original_structure_offset,
        STRUCTURE_BYTES,
        "JP title-prompt structure template",
    )?;
    let template: [u8; STRUCTURE_BYTES] = bytes
        .try_into()
        .expect("source_range returned the requested structure size");
    if read_u16(&template, 0, "JP title-prompt template count")? != 1
        || read_u16(&template, 2, "JP title-prompt template Y")? != 0x0078
        || read_u16(&template, 4, "JP title-prompt template size/link")? != 0x0500
        || read_u16(&template, 8, "JP title-prompt template X")? != 0x0078
    {
        return Err("JP title-prompt structure template fields drifted".to_string());
    }
    Ok(template)
}

fn validate_original_structures(
    source: &[u8],
    fragment: &PromptFragment,
    template: &[u8; STRUCTURE_BYTES],
) -> Result<(), String> {
    let bytes = source_range(
        source,
        fragment.original_structure_offset,
        fragment.original_slots * STRUCTURE_BYTES,
        &format!("{} JP structures", fragment.id),
    )?;
    for (slot_index, slot) in bytes.chunks_exact(STRUCTURE_BYTES).enumerate() {
        if slot[..TILE_WORD_OFFSET] != template[..TILE_WORD_OFFSET]
            || slot[TILE_WORD_OFFSET + 2..] != template[TILE_WORD_OFFSET + 2..]
        {
            return Err(format!(
                "{} JP structure slot {} differs outside its tile word",
                fragment.id, slot_index
            ));
        }
    }
    Ok(())
}

fn count_instruction(slots: u16) -> Result<Vec<u8>, String> {
    assemble(&[Inst::MoveWordImmediateToDisplacementAddress {
        immediate: slots,
        displacement: 0x002C,
        destination: AddressReg::A0,
    }])
}

fn x_instruction(base_x: u16) -> Result<Vec<u8>, String> {
    assemble(&[Inst::MoveWordImmediateToDisplacementAddress {
        immediate: base_x,
        displacement: 0x0012,
        destination: AddressReg::A0,
    }])
}

fn pointer_instruction(offset: usize) -> Result<Vec<u8>, String> {
    let address = u32::try_from(offset)
        .map_err(|_| format!("title-prompt pointer 0x{offset:X} exceeds 32 bits"))?;
    assemble(&[Inst::MoveAddressLongImmediate {
        address,
        destination: AddressReg::A2,
    }])
}

fn validate_instruction(
    source: &[u8],
    offset: usize,
    expected: &[u8],
    label: &str,
) -> Result<(), String> {
    let actual = source_range(source, offset, expected.len(), label)?;
    if actual != expected {
        return Err(format!(
            "{label} at 0x{offset:06X} does not match typed 68000 source"
        ));
    }
    Ok(())
}

fn parse_u16_hex(value: &str) -> Result<u16, String> {
    let parsed = parse_hex(value)?;
    u16::try_from(parsed).map_err(|_| format!("hex value {value:?} exceeds u16"))
}

fn u16_from_usize(value: usize, label: &str) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("{label} slot count {value} exceeds u16"))
}
