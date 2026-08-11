//! Read-only ownership and native-consumer audits for JP-side text paths.

use super::*;

pub(super) const FFF8_INDEX_COUNT: usize = 1273;
pub(super) const FFF8_ABSENT_INDICES: [u16; 6] = [10, 192, 740, 877, 894, 1010];
pub(super) const FFF8_NATIVE_UNCONSUMED_INDICES: [u16; 2] = [33, 34];
pub(super) const FFF8_NATIVE_SHARED_TAIL_INDICES: [u16; 5] = [206, 207, 1065, 1162, 1199];
pub(super) const FFF8_NATIVE_CONTROL_ONLY_INDICES: [u16; 7] = [469, 473, 485, 499, 522, 850, 1191];
pub(super) const FFF8_NATIVE_DYNAMIC_BUFFER_INDICES: [u16; 3] = [739, 741, 1160];
const FFF8_SPECIAL_ASSET_IDENTITIES: [(u16, &str, &str); 24] = [
    (33, "script_0043", "system"),
    (34, "script_0044", "system"),
    (40, "script_1325", "fff8_unconsumed_tail"),
    (41, "script_1326", "fff8_unconsumed_tail"),
    (206, "script_0228", "event"),
    (207, "script_0231", "event"),
    (469, "script_0740", "dungeon"),
    (473, "script_0771", "dungeon"),
    (485, "script_0500", "dungeon"),
    (499, "script_0515", "dungeon"),
    (522, "script_0540", "dungeon"),
    (702, "script_1327", "dungeon"),
    (739, "script_0785", "dungeon"),
    (741, "script_0812", "dungeon"),
    (764, "script_0839", "dungeon"),
    (765, "script_1332", "dungeon"),
    (850, "script_0922", "dungeon"),
    (899, "script_1340", "fff8_control_only"),
    (1065, "script_1152", "dungeon"),
    (1160, "script_1237", "shop"),
    (1162, "script_1239", "intro_ending"),
    (1191, "script_1268", "intro_ending"),
    (1199, "script_1276", "intro_ending"),
    (1226, "script_1303", "intro_ending"),
];

/// The JP-side owner of one legacy EN FFF8 index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fff8Ownership {
    StableRedirect,
    NativeItemName,
    NativeQuotedItem,
    NativeItemDescription,
    NativeItemUse,
    NativeBattleItemUse,
    NativeMonsterName,
    NativeUnconsumedSlot,
    NativeSharedTail,
    NativeControlOnly,
    NativeDynamicBuffer,
    EnUnconsumedTail,
    EnControlDrop,
    AbsentSlot,
}

impl Fff8Ownership {
    pub fn label(self) -> &'static str {
        match self {
            Self::StableRedirect => "stable_redirect",
            Self::NativeItemName => "native_item_name",
            Self::NativeQuotedItem => "native_quoted_item",
            Self::NativeItemDescription => "native_item_description",
            Self::NativeItemUse => "native_item_use",
            Self::NativeBattleItemUse => "native_battle_item_use",
            Self::NativeMonsterName => "native_monster_name",
            Self::NativeUnconsumedSlot => "native_unconsumed_slot",
            Self::NativeSharedTail => "native_shared_tail",
            Self::NativeControlOnly => "native_control_only",
            Self::NativeDynamicBuffer => "native_dynamic_buffer",
            Self::EnUnconsumedTail => "en_unconsumed_tail",
            Self::EnControlDrop => "en_control_drop",
            Self::AbsentSlot => "absent_slot",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fff8OwnershipRow {
    pub index: u16,
    pub id: Option<String>,
    pub section: Option<String>,
    pub ownership: Fff8Ownership,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fff8OwnershipReport {
    pub rows: Vec<Fff8OwnershipRow>,
}

impl Fff8OwnershipReport {
    pub fn counts(&self) -> BTreeMap<Fff8Ownership, usize> {
        let mut counts = BTreeMap::new();
        for row in &self.rows {
            *counts.entry(row.ownership).or_default() += 1;
        }
        counts
    }

    pub fn render(&self) -> String {
        let counts = self.counts();
        let mut output = format!(
            "FFF8 ownership audit: {}/{} classified, 0 unclassified\n",
            self.rows.len(),
            FFF8_INDEX_COUNT
        );
        for ownership in FFF8_OWNERSHIP_ORDER {
            use std::fmt::Write;
            writeln!(
                &mut output,
                "  {:28} {}",
                ownership.label(),
                counts.get(&ownership).copied().unwrap_or(0)
            )
            .expect("writing to a String cannot fail");
        }
        for ownership in [Fff8Ownership::EnUnconsumedTail, Fff8Ownership::AbsentSlot] {
            use std::fmt::Write;
            let indices = self
                .rows
                .iter()
                .filter(|row| row.ownership == ownership)
                .map(|row| row.index.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(&mut output, "  {} indices: {indices}", ownership.label())
                .expect("writing to a String cannot fail");
        }
        output
    }
}

const FFF8_OWNERSHIP_ORDER: [Fff8Ownership; 14] = [
    Fff8Ownership::StableRedirect,
    Fff8Ownership::NativeItemName,
    Fff8Ownership::NativeQuotedItem,
    Fff8Ownership::NativeItemDescription,
    Fff8Ownership::NativeItemUse,
    Fff8Ownership::NativeBattleItemUse,
    Fff8Ownership::NativeMonsterName,
    Fff8Ownership::NativeUnconsumedSlot,
    Fff8Ownership::NativeSharedTail,
    Fff8Ownership::NativeControlOnly,
    Fff8Ownership::NativeDynamicBuffer,
    Fff8Ownership::EnUnconsumedTail,
    Fff8Ownership::EnControlDrop,
    Fff8Ownership::AbsentSlot,
];

#[derive(Debug)]
struct Fff8AssetEntry {
    id: String,
    section: String,
}

/// Classify every legacy EN FFF8 index by its JP-native owner.
///
/// This is deliberately fail-closed: new, duplicate, missing, or reclassified
/// alignment rows make the audit fail instead of silently entering a fallback
/// bucket.
pub fn audit_fff8_ownership(assets_dir: &Path) -> Result<Fff8OwnershipReport, String> {
    let inventory = load_fff8_asset_inventory(&assets_dir.join("translation"))?;
    if inventory.len() != 1267 {
        return Err(format!(
            "FFF8 alignment contains {} unique indices, expected 1267",
            inventory.len()
        ));
    }

    let mut stable_specs = BTreeMap::new();
    for spec in m70_text_specs() {
        let index = u16::try_from(spec.legacy_fff8_idx)
            .map_err(|_| format!("{}: FFF8 index exceeds 16 bits", spec.id))?;
        if stable_specs.insert(index, spec).is_some() {
            return Err(format!("FFF8 index {index}: duplicate stable owner"));
        }
    }
    if stable_specs.len() != 1008 {
        return Err(format!(
            "M70 stable catalog contains {} unique FFF8 indices, expected 1008",
            stable_specs.len()
        ));
    }

    let mut rows = Vec::with_capacity(FFF8_INDEX_COUNT);
    for index in 0..FFF8_INDEX_COUNT as u16 {
        let asset = inventory.get(&index);
        let ownership = if let Some(spec) = stable_specs.get(&index) {
            let asset = asset.ok_or_else(|| {
                format!("FFF8 index {index}: stable owner {} has no asset", spec.id)
            })?;
            if asset.id != spec.id {
                return Err(format!(
                    "FFF8 index {index}: stable owner is {}, asset is {}",
                    spec.id, asset.id
                ));
            }
            if let Some(section) = spec.section
                && asset.section != section
            {
                return Err(format!(
                    "FFF8 index {index}: stable section is {section}, asset section is {}",
                    asset.section
                ));
            }
            Fff8Ownership::StableRedirect
        } else if FFF8_ABSENT_INDICES.contains(&index) {
            if asset.is_some() {
                return Err(format!(
                    "FFF8 index {index}: expected an absent slot, found an asset"
                ));
            }
            Fff8Ownership::AbsentSlot
        } else {
            let asset =
                asset.ok_or_else(|| format!("FFF8 index {index}: unclassified missing asset"))?;
            if let Some((_, expected_id, expected_section)) = FFF8_SPECIAL_ASSET_IDENTITIES
                .iter()
                .find(|(expected_index, _, _)| *expected_index == index)
                && (asset.id != *expected_id || asset.section != *expected_section)
            {
                return Err(format!(
                    "FFF8 index {index}: expected {expected_id} in {expected_section}, found {} in {}",
                    asset.id, asset.section
                ));
            }
            match (index, asset.section.as_str()) {
                (208..=247, "item_name") => Fff8Ownership::NativeItemName,
                (248..=287, "item_quoted") => Fff8Ownership::NativeQuotedItem,
                (288..=325, "item_desc") => Fff8Ownership::NativeItemDescription,
                (421, "event") if asset.id == JP_ITEM_DESC_SHARED_EVENT_ID => {
                    Fff8Ownership::NativeItemDescription
                }
                (326..=376, "item_use") => Fff8Ownership::NativeItemUse,
                (377..=420, "item_use2") => Fff8Ownership::NativeBattleItemUse,
                (1248..=1272, "monster") => Fff8Ownership::NativeMonsterName,
                (_, _) if FFF8_NATIVE_UNCONSUMED_INDICES.contains(&index) => {
                    Fff8Ownership::NativeUnconsumedSlot
                }
                (_, _) if FFF8_NATIVE_SHARED_TAIL_INDICES.contains(&index) => {
                    Fff8Ownership::NativeSharedTail
                }
                (_, _) if FFF8_NATIVE_CONTROL_ONLY_INDICES.contains(&index) => {
                    Fff8Ownership::NativeControlOnly
                }
                (_, _) if FFF8_NATIVE_DYNAMIC_BUFFER_INDICES.contains(&index) => {
                    Fff8Ownership::NativeDynamicBuffer
                }
                (40 | 41, "fff8_unconsumed_tail") => Fff8Ownership::EnUnconsumedTail,
                (_, "fff8_control_only") => Fff8Ownership::EnControlDrop,
                _ => {
                    return Err(format!(
                        "FFF8 index {index}: unclassified asset {} in section {}",
                        asset.id, asset.section
                    ));
                }
            }
        };
        rows.push(Fff8OwnershipRow {
            index,
            id: asset.map(|entry| entry.id.clone()),
            section: asset.map(|entry| entry.section.clone()),
            ownership,
        });
    }

    let report = Fff8OwnershipReport { rows };
    let expected_counts = BTreeMap::from([
        (Fff8Ownership::StableRedirect, 1008),
        (Fff8Ownership::NativeItemName, 40),
        (Fff8Ownership::NativeQuotedItem, 40),
        (Fff8Ownership::NativeItemDescription, 39),
        (Fff8Ownership::NativeItemUse, 51),
        (Fff8Ownership::NativeBattleItemUse, 44),
        (Fff8Ownership::NativeMonsterName, 25),
        (Fff8Ownership::NativeUnconsumedSlot, 2),
        (Fff8Ownership::NativeSharedTail, 5),
        (Fff8Ownership::NativeControlOnly, 7),
        (Fff8Ownership::NativeDynamicBuffer, 3),
        (Fff8Ownership::EnUnconsumedTail, 2),
        (Fff8Ownership::EnControlDrop, 1),
        (Fff8Ownership::AbsentSlot, 6),
    ]);
    if report.counts() != expected_counts {
        return Err(format!(
            "FFF8 ownership population drifted: {:?}",
            report.counts()
        ));
    }
    Ok(report)
}

fn load_fff8_asset_inventory(
    translation_dir: &Path,
) -> Result<BTreeMap<u16, Fff8AssetEntry>, String> {
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

    let mut inventory = BTreeMap::new();
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
            let Some(raw_index) = entry.get("fff8_idx").and_then(|value| value.as_i64()) else {
                continue;
            };
            if raw_index < 0 {
                continue;
            }
            let index = u16::try_from(raw_index).map_err(|_| {
                format!("{}: FFF8 index {raw_index} exceeds 16 bits", path.display())
            })?;
            if usize::from(index) >= FFF8_INDEX_COUNT {
                return Err(format!(
                    "{}: FFF8 index {index} exceeds catalog end {}",
                    path.display(),
                    FFF8_INDEX_COUNT - 1
                ));
            }
            let id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("{}: FFF8 index {index} has no ID", path.display()))?
                .to_string();
            let section = entry
                .get("section")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("{id}: FFF8 entry has no section"))?
                .to_string();
            if let Some(previous) = inventory.insert(index, Fff8AssetEntry { id, section }) {
                return Err(format!(
                    "FFF8 index {index}: duplicate assets {} and {}",
                    previous.id, inventory[&index].id
                ));
            }
        }
    }
    Ok(inventory)
}

pub(super) fn validate_m70_enemy_damage_boundaries(
    source: &[u8],
    assets_dir: &Path,
) -> Result<(), String> {
    let entries = load_stable_text_entries(
        &assets_dir.join("translation"),
        &M70_ENEMY_DAMAGE_TEXT_SPECS,
    )?;
    let mut expected_offset = M70_ENEMY_DAMAGE_TEXT_SPECS[0].offset;

    for (entry, spec) in entries.iter().zip(M70_ENEMY_DAMAGE_TEXT_SPECS) {
        if spec.offset != expected_offset {
            return Err(format!(
                "M70 enemy-damage entry {} starts at 0x{:06X}, expected 0x{expected_offset:06X}",
                spec.id, spec.offset
            ));
        }
        let words = validate_jp_text_source(
            source,
            &entry.id,
            spec.offset,
            &entry.jp,
            "M70 enemy-damage response",
        )?;
        expected_offset += words.len() * 2;
    }

    if expected_offset != 0x0A_2A74 {
        return Err(format!(
            "M70 enemy-damage batch ends at 0x{expected_offset:06X}, expected 0x0A2A74"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct EnemyStatusConsumerReport {
    pub pointer_array: u16,
    pub enemy_damage_table: usize,
    pub enemy_damage_entries: usize,
    pub enemy_hp_table: usize,
    pub enemy_hp_entries: usize,
    pub array_init: usize,
    pub object_binding: usize,
}

impl EnemyStatusConsumerReport {
    pub fn render(self) -> String {
        format!(
            concat!(
                "JP enemy-status consumer audit\n",
                "pointer_array=FF{:04X}\n",
                "enemy_damage=0x{:06X} entries={}\n",
                "enemy_hp=0x{:06X} entries={}\n",
                "array_init=0x{:06X}\n",
                "object_binding=0x{:06X}\n",
                "classified=2/2 unbound=0\n"
            ),
            self.pointer_array,
            self.enemy_damage_table,
            self.enemy_damage_entries,
            self.enemy_hp_table,
            self.enemy_hp_entries,
            self.array_init,
            self.object_binding,
        )
    }
}

/// Audit the JP battle object's direct bindings to the M69/M70 native tables.
pub fn audit_enemy_status_consumers(
    jp_rom_path: &Path,
) -> Result<EnemyStatusConsumerReport, String> {
    let source =
        fs::read(jp_rom_path).map_err(|error| format!("failed to read JP ROM: {error}"))?;
    validate_jp_source(&source)?;
    validate_m72_enemy_status_consumers(&source)?;
    Ok(EnemyStatusConsumerReport {
        pointer_array: JP_ENEMY_STATUS_POINTER_ARRAY,
        enemy_damage_table: JP_ENEMY_DAMAGE_TABLE,
        enemy_damage_entries: M70_ENEMY_DAMAGE_TEXT_SPECS.len(),
        enemy_hp_table: JP_ENEMY_HP_TABLE,
        enemy_hp_entries: M69_ENEMY_HP_TEXT_SPECS.len(),
        array_init: JP_ENEMY_STATUS_ARRAY_INIT,
        object_binding: JP_ENEMY_STATUS_OBJECT_BINDING,
    })
}

pub(super) fn validate_m72_enemy_status_consumers(source: &[u8]) -> Result<(), String> {
    m58_expect_instruction(
        source,
        JP_ENEMY_STATUS_ARRAY_INIT,
        Inst::LeaAbsoluteShort {
            address: JP_ENEMY_STATUS_POINTER_ARRAY,
            destination: AddressReg::A1,
        },
        "M72 enemy-status pointer-array initializer",
    )?;
    m58_expect_instruction(
        source,
        JP_ENEMY_STATUS_ARRAY_INIT + 4,
        Inst::MoveLongImmediateToPostincrementAddress {
            immediate: JP_ENEMY_DAMAGE_TABLE as u32,
            destination: AddressReg::A1,
        },
        "M72 enemy-damage table binding",
    )?;
    m58_expect_instruction(
        source,
        JP_ENEMY_STATUS_ARRAY_INIT + 10,
        Inst::MoveLongImmediateToPostincrementAddress {
            immediate: JP_ENEMY_HP_TABLE as u32,
            destination: AddressReg::A1,
        },
        "M72 enemy-health table binding",
    )?;
    m58_expect_instruction(
        source,
        JP_ENEMY_STATUS_OBJECT_BINDING,
        Inst::MoveLongImmediateToDisplacementAddress {
            immediate: 0x00FF_0000 | u32::from(JP_ENEMY_STATUS_POINTER_ARRAY),
            displacement: 0x0040,
            destination: AddressReg::A1,
        },
        "M72 battle-object pointer-array binding",
    )?;

    validate_m72_relative_table(
        source,
        JP_ENEMY_DAMAGE_TABLE,
        &M70_ENEMY_DAMAGE_TEXT_SPECS,
        "enemy-damage",
    )?;
    validate_m72_relative_table(
        source,
        JP_ENEMY_HP_TABLE,
        &M69_ENEMY_HP_TEXT_SPECS,
        "enemy-health",
    )?;

    for (address, expected_operand) in [
        (JP_ENEMY_DAMAGE_TABLE, JP_ENEMY_STATUS_ARRAY_INIT + 6),
        (JP_ENEMY_HP_TABLE, JP_ENEMY_STATUS_ARRAY_INIT + 12),
        (
            0x00FF_0000 | usize::from(JP_ENEMY_STATUS_POINTER_ARRAY),
            JP_ENEMY_STATUS_OBJECT_BINDING + 2,
        ),
    ] {
        let address = (address as u32).to_be_bytes();
        let refs = source[..JP_CODE_SCAN_END]
            .windows(address.len())
            .enumerate()
            .filter_map(|(offset, bytes)| (bytes == address).then_some(offset))
            .collect::<Vec<_>>();
        if refs != [expected_operand] {
            return Err(format!(
                "M72 battle consumer xrefs changed: expected 0x{expected_operand:06X}, found {refs:?}"
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_m72_relative_table(
    source: &[u8],
    table: usize,
    specs: &[StableTextSpec],
    label: &str,
) -> Result<(), String> {
    for (slot, spec) in specs.iter().enumerate() {
        let relative = usize::from(m58_read_word(
            source,
            table + slot * 2,
            "M72 enemy-status table pointer",
        )?);
        let target = table + relative;
        if target != spec.offset {
            return Err(format!(
                "M72 {label} slot {slot} resolves to 0x{target:06X}, not {} at 0x{:06X}",
                spec.id, spec.offset
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerDamageConsumerReport {
    pub pointer_array: u32,
    pub voice_table: usize,
    pub novoice_table: usize,
    pub promoted_novoice_entries: usize,
    pub normal_init: usize,
    pub camus_cutscene_init: usize,
    pub amigo_battle_init: usize,
    pub object_binding: usize,
}

impl PlayerDamageConsumerReport {
    pub fn render(self) -> String {
        format!(
            concat!(
                "JP player-damage consumer audit\n",
                "pointer_array={:06X}\n",
                "voice=0x{:06X}\n",
                "novoice=0x{:06X} promoted_entries={}\n",
                "normal_init=0x{:06X}\n",
                "camus_cutscene_init=0x{:06X}\n",
                "amigo_battle_init=0x{:06X}\n",
                "object_binding=0x{:06X}\n",
                "novoice_paths=2 classified=2 unbound=0\n"
            ),
            self.pointer_array,
            self.voice_table,
            self.novoice_table,
            self.promoted_novoice_entries,
            self.normal_init,
            self.camus_cutscene_init,
            self.amigo_battle_init,
            self.object_binding,
        )
    }
}

/// Audit the normal, Camus-cutscene, and A-capsule Amigo damage-table bindings.
pub fn audit_player_damage_consumers(
    jp_rom_path: &Path,
) -> Result<PlayerDamageConsumerReport, String> {
    let source =
        fs::read(jp_rom_path).map_err(|error| format!("failed to read JP ROM: {error}"))?;
    validate_jp_source(&source)?;
    validate_m75_player_damage_path_classification(&source)?;
    Ok(PlayerDamageConsumerReport {
        pointer_array: JP_PLAYER_MESSAGE_POINTER_ARRAY,
        voice_table: JP_DAMAGE_VOICE_TABLE,
        novoice_table: JP_DAMAGE_NOVOICE_TABLE,
        promoted_novoice_entries: M64_DAMAGE_NOVOICE_TEXT_SPECS.len(),
        normal_init: JP_PLAYER_DAMAGE_NORMAL_INIT,
        camus_cutscene_init: JP_PLAYER_DAMAGE_CAMUS_CUTSCENE_INIT,
        amigo_battle_init: JP_PLAYER_DAMAGE_AMIGO_BATTLE_INIT,
        object_binding: JP_PLAYER_MESSAGE_OBJECT_BINDING,
    })
}

pub(super) fn validate_m74_player_damage_consumers(source: &[u8]) -> Result<(), String> {
    m58_expect_instruction(
        source,
        JP_PLAYER_DAMAGE_NORMAL_INIT,
        Inst::LeaAbsoluteLong {
            address: JP_PLAYER_DAMAGE_POINTER_VECTOR as u32,
            destination: AddressReg::A1,
        },
        "M74 normal player-message vector source",
    )?;
    m58_expect_instruction(
        source,
        JP_PLAYER_DAMAGE_NORMAL_INIT + 6,
        Inst::LeaAbsoluteLong {
            address: JP_PLAYER_MESSAGE_POINTER_ARRAY,
            destination: AddressReg::A2,
        },
        "M74 normal player-message vector destination",
    )?;
    for index in 0..8 {
        m58_expect_instruction(
            source,
            JP_PLAYER_DAMAGE_NORMAL_INIT + 12 + index * 2,
            Inst::MoveLongPostincrementAddressToPostincrementAddress {
                source: AddressReg::A1,
                destination: AddressReg::A2,
            },
            "M74 normal player-message vector copy",
        )?;
    }

    let voice_pointer = source
        .get(JP_PLAYER_DAMAGE_POINTER_VECTOR..JP_PLAYER_DAMAGE_POINTER_VECTOR + 4)
        .ok_or_else(|| "M74 player-message vector is outside the JP ROM".to_owned())?;
    if voice_pointer != (JP_DAMAGE_VOICE_TABLE as u32).to_be_bytes() {
        return Err("M74 normal player-damage vector no longer starts with dmg_voice".into());
    }

    for (offset, label) in [
        (
            JP_PLAYER_DAMAGE_CAMUS_CUTSCENE_INIT,
            "M74/M75 Camus non-voiced player-damage initializer",
        ),
        (
            JP_PLAYER_DAMAGE_AMIGO_BATTLE_INIT,
            "M74/M75 Amigo non-voiced player-damage initializer",
        ),
    ] {
        m58_expect_instruction(
            source,
            offset,
            Inst::MoveLongImmediateToAbsoluteLong {
                immediate: JP_DAMAGE_NOVOICE_TABLE as u32,
                address: JP_PLAYER_MESSAGE_POINTER_ARRAY,
            },
            label,
        )?;
    }

    m58_expect_instruction(
        source,
        JP_PLAYER_MESSAGE_OBJECT_BINDING,
        Inst::MoveLongImmediateToDisplacementAddress {
            immediate: JP_PLAYER_MESSAGE_POINTER_ARRAY,
            displacement: 0x0040,
            destination: AddressReg::A1,
        },
        "M74 player-message object pointer-array binding",
    )?;
    validate_m72_relative_table(
        source,
        JP_DAMAGE_NOVOICE_TABLE,
        &M64_DAMAGE_NOVOICE_TEXT_SPECS,
        "player-damage-novoice",
    )?;

    let table_bytes = (JP_DAMAGE_NOVOICE_TABLE as u32).to_be_bytes();
    let refs = source[..JP_CODE_SCAN_END]
        .windows(table_bytes.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == table_bytes).then_some(offset))
        .collect::<Vec<_>>();
    let expected_refs = [
        JP_PLAYER_DAMAGE_CAMUS_CUTSCENE_INIT + 2,
        JP_PLAYER_DAMAGE_AMIGO_BATTLE_INIT + 2,
    ];
    if refs != expected_refs {
        return Err(format!(
            "M74 dmg_novoice code xrefs changed: expected {expected_refs:?}, found {refs:?}"
        ));
    }
    Ok(())
}

pub(super) fn validate_m75_player_damage_path_classification(source: &[u8]) -> Result<(), String> {
    validate_m74_player_damage_consumers(source)?;

    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_SAVE_ARLE,
        Inst::JsrProgramCounterDisplacement(0x0004_C320),
        "M75 Camus battle Arle-stat save",
    )?;
    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_SAVE_ARLE + 4,
        Inst::LeaAbsoluteLong {
            address: JP_CAMUS_BATTLE_STATS_SOURCE,
            destination: AddressReg::A2,
        },
        "M75 Camus stand-in stat source",
    )?;
    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_SAVE_ARLE + 10,
        Inst::LeaAbsoluteShort {
            address: JP_PLAYER_BATTLE_STATS,
            destination: AddressReg::A3,
        },
        "M75 Camus player-slot stat destination",
    )?;
    for (offset, displacement, is_word) in [
        (0x0004_751C, 0, false),
        (0x0004_7520, 4, false),
        (0x0004_7524, 8, true),
        (0x0004_7528, 2, false),
    ] {
        let instruction = if is_word {
            Inst::MoveWordDisplacementAddressToPostincrementAddress {
                displacement,
                source: AddressReg::A2,
                destination: AddressReg::A3,
            }
        } else {
            Inst::MoveLongDisplacementAddressToPostincrementAddress {
                displacement,
                source: AddressReg::A2,
                destination: AddressReg::A3,
            }
        };
        m58_expect_instruction(source, offset, instruction, "M75 Camus stand-in stat copy")?;
    }
    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_DIALOGUE_SELECT,
        Inst::Moveq {
            immediate: 0x5C,
            destination: DataReg::D0,
        },
        "M75 Camus battle dialogue selector",
    )?;
    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_DIALOGUE_SELECT + 2,
        Inst::LeaAbsoluteLong {
            address: JP_MONSTER_BATTLE_DIALOGUE_POINTER_VECTOR,
            destination: AddressReg::A2,
        },
        "M75 Camus battle dialogue table",
    )?;
    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_DIALOGUE_SELECT + 8,
        Inst::MoveLongIndexedAddressToAbsoluteLong {
            displacement: 0,
            base: AddressReg::A2,
            index: DataReg::D0,
            address: JP_PLAYER_MESSAGE_POINTER_ARRAY + 8,
        },
        "M75 Camus battle dialogue binding",
    )?;
    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_ENEMY_SELECT,
        Inst::MoveByteImmediateToData {
            immediate: 0x17,
            destination: DataReg::D0,
        },
        "M75 Camus fixed Mysterious Person enemy selector",
    )?;
    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_ENEMY_SELECT + 8,
        Inst::MoveByteDataToAbsoluteShort {
            source: DataReg::D0,
            address: 0x85CC,
        },
        "M75 Camus fixed enemy binding",
    )?;
    m58_expect_instruction(
        source,
        JP_CAMUS_BATTLE_RESTORE_ARLE,
        Inst::JsrProgramCounterDisplacement(0x0004_C342),
        "M75 Camus battle Arle-stat restore",
    )?;

    m58_expect_instruction(
        source,
        JP_AMIGO_CAPTURE_MONSTER,
        Inst::MoveByteAbsoluteShortToAbsoluteShort {
            source: 0x8003,
            destination: JP_AMIGO_CAPTURED_MONSTER_ID,
        },
        "M75 A-capsule captured-monster identity",
    )?;
    m58_expect_instruction(
        source,
        JP_AMIGO_CAPTURE_MONSTER + 6,
        Inst::MoveAddressAbsoluteShortToAddress {
            address: JP_AMIGO_CAPTURED_STATS_POINTER,
            destination: AddressReg::A2,
        },
        "M75 A-capsule captured-stat source",
    )?;
    m58_expect_instruction(
        source,
        JP_AMIGO_CAPTURE_MONSTER + 10,
        Inst::LeaAbsoluteShort {
            address: JP_AMIGO_CAPTURED_STATS,
            destination: AddressReg::A3,
        },
        "M75 A-capsule captured-stat destination",
    )?;
    for (offset, source_displacement, destination_displacement, is_word) in [
        (0x0000_B612, 0, 0, true),
        (0x0000_B618, 2, 2, false),
        (0x0000_B61E, 8, 8, true),
        (0x0000_B624, 2, 10, false),
    ] {
        let instruction = if is_word {
            Inst::MoveWordDisplacementAddressToDisplacementAddress {
                source_displacement,
                source: AddressReg::A2,
                destination_displacement,
                destination: AddressReg::A3,
            }
        } else {
            Inst::MoveLongDisplacementAddressToDisplacementAddress {
                source_displacement,
                source: AddressReg::A2,
                destination_displacement,
                destination: AddressReg::A3,
            }
        };
        m58_expect_instruction(
            source,
            offset,
            instruction,
            "M75 A-capsule captured-stat copy",
        )?;
    }
    m58_expect_instruction(
        source,
        JP_AMIGO_BATTLE_ENTRY,
        Inst::MoveByteAbsoluteShortToData {
            address: JP_AMIGO_CAPTURED_MONSTER_ID,
            destination: DataReg::D0,
        },
        "M75 Amigo battle captured-monster selector",
    )?;
    m58_expect_instruction(
        source,
        JP_AMIGO_BATTLE_STATS_SWAP - 4,
        Inst::JsrProgramCounterDisplacement(0x0004_C320),
        "M75 Amigo battle Arle-stat save",
    )?;
    m58_expect_instruction(
        source,
        JP_AMIGO_BATTLE_STATS_SWAP,
        Inst::LeaAbsoluteShort {
            address: JP_AMIGO_CAPTURED_STATS,
            destination: AddressReg::A2,
        },
        "M75 Amigo battle captured-stat source",
    )?;
    m58_expect_instruction(
        source,
        JP_AMIGO_BATTLE_STATS_SWAP + 4,
        Inst::LeaAbsoluteShort {
            address: JP_PLAYER_BATTLE_STATS,
            destination: AddressReg::A3,
        },
        "M75 Amigo battle player-slot stat destination",
    )?;
    for (offset, is_word) in [
        (JP_AMIGO_BATTLE_STATS_SWAP + 8, false),
        (JP_AMIGO_BATTLE_STATS_SWAP + 10, false),
        (JP_AMIGO_BATTLE_STATS_SWAP + 12, true),
        (JP_AMIGO_BATTLE_STATS_SWAP + 14, false),
    ] {
        let instruction = if is_word {
            Inst::MoveWordPostincrementAddressToPostincrementAddress {
                source: AddressReg::A2,
                destination: AddressReg::A3,
            }
        } else {
            Inst::MoveLongPostincrementAddressToPostincrementAddress {
                source: AddressReg::A2,
                destination: AddressReg::A3,
            }
        };
        m58_expect_instruction(
            source,
            offset,
            instruction,
            "M75 Amigo player-slot stat copy",
        )?;
    }
    m58_expect_instruction(
        source,
        JP_AMIGO_BATTLE_DIALOGUE_SELECT,
        Inst::MoveByteAbsoluteShortToData {
            address: JP_AMIGO_CAPTURED_MONSTER_ID,
            destination: DataReg::D0,
        },
        "M75 Amigo battle dialogue selector",
    )?;
    m58_expect_instruction(
        source,
        JP_AMIGO_BATTLE_DIALOGUE_SELECT + 10,
        Inst::LeaAbsoluteLong {
            address: JP_MONSTER_BATTLE_DIALOGUE_POINTER_VECTOR,
            destination: AddressReg::A2,
        },
        "M75 Amigo battle dialogue table",
    )?;
    m58_expect_instruction(
        source,
        JP_AMIGO_BATTLE_DIALOGUE_SELECT + 16,
        Inst::MoveLongIndexedAddressToAbsoluteLong {
            displacement: 0,
            base: AddressReg::A2,
            index: DataReg::D0,
            address: JP_PLAYER_MESSAGE_POINTER_ARRAY + 8,
        },
        "M75 Amigo battle dialogue binding",
    )?;
    Ok(())
}
