//! Historical JP-native milestone builders.
//!
//! Every milestone shares one output/BPS transaction. The typed builder
//! function is the only stage-specific behavior.

use std::path::Path;

use madou_kr::{bps, jp_native};

use super::support::create_parent_dir;

macro_rules! jp_poc_command {
    ($command:ident, $builder:ident, $rom_message:literal, $bps_message:literal) => {
        pub(super) fn $command(
            rom_path: &Path,
            assets_dir: &Path,
            output_path: &Path,
            bps_path: Option<&Path>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let source = std::fs::read(rom_path)?;
            let output = jp_native::$builder(rom_path, assets_dir)?;
            create_parent_dir(output_path)?;
            std::fs::write(output_path, &output)?;
            println!($rom_message, output_path.display());

            if let Some(bps_path) = bps_path {
                let patch = bps::create(&source, &output)?;
                create_parent_dir(bps_path)?;
                std::fs::write(bps_path, &patch)?;
                println!($bps_message, bps_path.display());
            }
            Ok(())
        }
    };
}

jp_poc_command!(
    cmd_build_jp_font_poc,
    build_font_poc,
    "JP-native M1 font PoC saved: {}",
    "JP-source M1 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_text_poc,
    build_text_poc,
    "JP-native M2 text PoC saved: {}",
    "JP-source M2 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_menu_poc,
    build_menu_poc,
    "JP-native M3 menu PoC saved: {}",
    "JP-source M3 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dialog_poc,
    build_dialog_poc,
    "JP-native M4 dialog PoC saved: {}",
    "JP-source M4 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_item_quoted_poc,
    build_item_quoted_poc,
    "JP-native M5 quoted-item PoC saved: {}",
    "JP-source M5 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_item_desc_poc,
    build_item_desc_poc,
    "JP-native M6 item-description PoC saved: {}",
    "JP-source M6 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_item_use_poc,
    build_item_use_poc,
    "JP-native M7 ordinary item-use PoC saved: {}",
    "JP-source M7 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_item_use2_poc,
    build_item_use2_poc,
    "JP-native M8 battle item-use PoC saved: {}",
    "JP-source M8 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_recovery_poc,
    build_recovery_poc,
    "JP-native M9 recovery-result PoC saved: {}",
    "JP-source M9 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_health_poc,
    build_health_poc,
    "JP-native M10 health-status PoC saved: {}",
    "JP-source M10 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_mp_remaining_poc,
    build_mp_remaining_poc,
    "JP-native M11 remaining-MP PoC saved: {}",
    "JP-source M11 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_enemy_action_poc,
    build_enemy_action_poc,
    "JP-native M12 Puyo battle-action PoC saved: {}",
    "JP-source M12 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_enemy_reaction_poc,
    build_enemy_reaction_poc,
    "JP-native M13 enemy-reaction PoC saved: {}",
    "JP-source M13 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_enemy_battle2_poc,
    build_enemy_battle2_poc,
    "JP-native M14 second enemy battle-text PoC saved: {}",
    "JP-source M14 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_enemy_battle3_poc,
    build_enemy_battle3_poc,
    "JP-native M15 third enemy battle-text PoC saved: {}",
    "JP-source M15 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_enemy_battle4_poc,
    build_enemy_battle4_poc,
    "JP-native M16 fourth enemy battle-text PoC saved: {}",
    "JP-source M16 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event1_poc,
    build_dungeon_event1_poc,
    "JP-native M17 first dungeon-event PoC saved: {}",
    "JP-source M17 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_choice1_poc,
    build_dungeon_choice1_poc,
    "JP-native M18 first dungeon choice and button-event PoC saved: {}",
    "JP-source M18 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event2_poc,
    build_dungeon_event2_poc,
    "JP-native M19 second dungeon-event PoC saved: {}",
    "JP-source M19 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event3_poc,
    build_dungeon_event3_poc,
    "JP-native M20 third dungeon-event PoC saved: {}",
    "JP-source M20 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event4_poc,
    build_dungeon_event4_poc,
    "JP-native M21 fourth dungeon-event PoC saved: {}",
    "JP-source M21 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event5_poc,
    build_dungeon_event5_poc,
    "JP-native M22 fifth dungeon-event PoC saved: {}",
    "JP-source M22 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event6_poc,
    build_dungeon_event6_poc,
    "JP-native M23 sixth dungeon-event PoC saved: {}",
    "JP-source M23 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event7_poc,
    build_dungeon_event7_poc,
    "JP-native M24 seventh dungeon-event PoC saved: {}",
    "JP-source M24 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event8_poc,
    build_dungeon_event8_poc,
    "JP-native M25 eighth dungeon-event PoC saved: {}",
    "JP-source M25 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event9_poc,
    build_dungeon_event9_poc,
    "JP-native M26 ninth dungeon-event PoC saved: {}",
    "JP-source M26 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event10_poc,
    build_dungeon_event10_poc,
    "JP-native M27 tenth dungeon-event PoC saved: {}",
    "JP-source M27 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event11_poc,
    build_dungeon_event11_poc,
    "JP-native M28 eleventh dungeon-event PoC saved: {}",
    "JP-source M28 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event12_poc,
    build_dungeon_event12_poc,
    "JP-native M29 twelfth dungeon-event PoC saved: {}",
    "JP-source M29 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event13_poc,
    build_dungeon_event13_poc,
    "JP-native M30 thirteenth dungeon-event PoC saved: {}",
    "JP-source M30 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event14_poc,
    build_dungeon_event14_poc,
    "JP-native M31 fourteenth dungeon-event PoC saved: {}",
    "JP-source M31 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event15_poc,
    build_dungeon_event15_poc,
    "JP-native M32 fifteenth dungeon-event PoC saved: {}",
    "JP-source M32 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event16_poc,
    build_dungeon_event16_poc,
    "JP-native M33 sixteenth dungeon-event PoC saved: {}",
    "JP-source M33 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event17_poc,
    build_dungeon_event17_poc,
    "JP-native M34 seventeenth dungeon-event PoC saved: {}",
    "JP-source M34 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event18_poc,
    build_dungeon_event18_poc,
    "JP-native M35 eighteenth dungeon-event PoC saved: {}",
    "JP-source M35 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event19_poc,
    build_dungeon_event19_poc,
    "JP-native M36 nineteenth dungeon-event PoC saved: {}",
    "JP-source M36 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event20_poc,
    build_dungeon_event20_poc,
    "JP-native M37 twentieth dungeon-event PoC saved: {}",
    "JP-source M37 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event21_poc,
    build_dungeon_event21_poc,
    "JP-native M38 twenty-first dungeon-event PoC saved: {}",
    "JP-source M38 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event22_poc,
    build_dungeon_event22_poc,
    "JP-native M39 twenty-second dungeon-event PoC saved: {}",
    "JP-source M39 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event23_poc,
    build_dungeon_event23_poc,
    "JP-native M40 twenty-third dungeon-event PoC saved: {}",
    "JP-source M40 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event24_poc,
    build_dungeon_event24_poc,
    "JP-native M41 twenty-fourth dungeon-event PoC saved: {}",
    "JP-source M41 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_dungeon_event25_poc,
    build_dungeon_event25_poc,
    "JP-native M42 twenty-fifth dungeon-event PoC saved: {}",
    "JP-source M42 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_shop1_poc,
    build_shop1_poc,
    "JP-native M43 first shop PoC saved: {}",
    "JP-source M43 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_shop2_poc,
    build_shop2_poc,
    "JP-native M44 second shop PoC saved: {}",
    "JP-source M44 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_shop3_poc,
    build_shop3_poc,
    "JP-native M45 third shop PoC saved: {}",
    "JP-source M45 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_shop4_poc,
    build_shop4_poc,
    "JP-native M46 fourth shop PoC saved: {}",
    "JP-source M46 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_intro1_poc,
    build_intro1_poc,
    "JP-native M47 first automatic-prologue PoC saved: {}",
    "JP-source M47 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_intro2_poc,
    build_intro2_poc,
    "JP-native M48 second automatic-intro PoC saved: {}",
    "JP-source M48 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_intro3_poc,
    build_intro3_poc,
    "JP-native M49 automatic-intro final fall-effect PoC saved: {}",
    "JP-source M49 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_ending1_poc,
    build_ending1_poc,
    "JP-native M50 first ending escape-and-rescue PoC saved: {}",
    "JP-source M50 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_ending2_poc,
    build_ending2_poc,
    "JP-native M51 ending aftermath-and-score PoC saved: {}",
    "JP-source M51 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_ending3_poc,
    build_ending3_poc,
    "JP-native M52 ending outcome-and-gift PoC saved: {}",
    "JP-source M52 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_system1_poc,
    build_system1_poc,
    "JP-native M55 save, floor-label, and encounter system PoC saved: {}",
    "JP-source M55 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_system2_poc,
    build_system2_poc,
    "JP-native M56 chest, item, note, and door system PoC saved: {}",
    "JP-source M56 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_system3_poc,
    build_system3_poc,
    "JP-native M57 door, stair, spell-tutorial, and hazard system PoC saved: {}",
    "JP-source M57 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_system4_poc,
    build_system4_poc,
    "JP-native M59 exhausted-wall, floating-stone, and dark-passage PoC saved: {}",
    "JP-source M59 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_monster_names_poc,
    build_monster_names_poc,
    "JP-native M60 fixed monster-name PoC saved: {}",
    "JP-source M60 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_damage_voice_poc,
    build_damage_voice_poc,
    "JP-native M61 voiced-damage PoC saved: {}",
    "JP-source M61 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_early_puyo_battle_poc,
    build_early_puyo_battle_poc,
    "JP-native M62 early Puyo battle PoC saved: {}",
    "JP-source M62 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_encounter_intro_poc,
    build_encounter_intro_poc,
    "JP-native M63 encounter-introduction PoC saved: {}",
    "JP-source M63 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_damage_novoice_poc,
    build_damage_novoice_poc,
    "JP-native M64 non-voiced damage PoC saved: {}",
    "JP-source M64 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_spell_msg_poc,
    build_spell_msg_poc,
    "JP-native M65 spell-message PoC saved: {}",
    "JP-source M65 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_event_alias_poc,
    build_event_alias_poc,
    "JP-native M66 standalone-event/shared-prefix PoC saved: {}",
    "JP-source M66 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_spell_command_poc,
    build_spell_command_poc,
    "JP-native M67 spell-command PoC saved: {}",
    "JP-source M67 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_item_event_poc,
    build_item_event_poc,
    "JP-native M68 special item-event PoC saved: {}",
    "JP-source M68 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_enemy_hp_poc,
    build_enemy_hp_poc,
    "JP-native M69 enemy-health status PoC saved: {}",
    "JP-source M69 BPS saved: {}"
);

jp_poc_command!(
    cmd_build_jp_enemy_damage_poc,
    build_enemy_damage_poc,
    "JP-native M70 enemy-damage response PoC saved: {}",
    "JP-source M70 BPS saved: {}"
);
