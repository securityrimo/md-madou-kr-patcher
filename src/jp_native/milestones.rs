//! JP-native milestone orchestration and source-consumer admission gates.
//!
//! Public builders retain the historical M2-M70 API while domain modules own
//! the individual stage groups and their admission checks.

mod dungeon;
mod early;
mod late;
mod scenes_system;

pub use dungeon::{
    build_dungeon_choice1_poc, build_dungeon_event1_poc, build_dungeon_event2_poc,
    build_dungeon_event3_poc, build_dungeon_event4_poc, build_dungeon_event5_poc,
    build_dungeon_event6_poc, build_dungeon_event7_poc, build_dungeon_event8_poc,
    build_dungeon_event9_poc, build_dungeon_event10_poc, build_dungeon_event11_poc,
    build_dungeon_event12_poc, build_dungeon_event13_poc, build_dungeon_event14_poc,
    build_dungeon_event15_poc, build_dungeon_event16_poc, build_dungeon_event17_poc,
    build_dungeon_event18_poc, build_dungeon_event19_poc, build_dungeon_event20_poc,
    build_dungeon_event21_poc, build_dungeon_event22_poc, build_dungeon_event23_poc,
    build_dungeon_event24_poc, build_dungeon_event25_poc,
};
pub use early::{
    build_dialog_poc, build_enemy_action_poc, build_enemy_battle2_poc, build_enemy_battle3_poc,
    build_enemy_battle4_poc, build_enemy_reaction_poc, build_health_poc, build_item_desc_poc,
    build_item_quoted_poc, build_item_use_poc, build_item_use2_poc, build_menu_poc,
    build_mp_remaining_poc, build_recovery_poc, build_text_poc,
};
pub use late::{
    build_damage_novoice_poc, build_damage_voice_poc, build_early_puyo_battle_poc,
    build_encounter_intro_poc, build_enemy_damage_poc, build_enemy_hp_poc, build_event_alias_poc,
    build_item_event_poc, build_jp_kr, build_monster_names_poc, build_spell_command_poc,
    build_spell_msg_poc,
};
pub use scenes_system::{
    build_ending1_poc, build_ending2_poc, build_ending3_poc, build_intro1_poc, build_intro2_poc,
    build_intro3_poc, build_shop1_poc, build_shop2_poc, build_shop3_poc, build_shop4_poc,
    build_system1_poc, build_system2_poc, build_system3_poc, build_system4_poc,
};

pub(super) use late::m70_text_specs;
pub(super) use scenes_system::{
    m58_direct_jsr_offsets, m58_expect_instruction, m58_read_word,
    validate_m58_unconsumed_system_rewards,
};
