//! Command routing only; command implementations live in responsibility modules.

use clap::Parser;
use std::process;

use madou_kr::check;

use super::args::{Cli, Commands};
use super::generators::*;
use super::jp_poc::*;
use super::previews::*;
use super::support::*;

pub(crate) fn run() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Create {
            source,
            target,
            output,
        } => cmd_create(&source, &target, &output),
        Commands::Apply { rom, patch, output } => cmd_apply(&rom, &patch, &output),
        Commands::ApplyIps { rom, patch, output } => cmd_apply_ips(&rom, &patch, &output),
        Commands::Build {
            rom,
            ips: ips_patch,
            assets,
            output,
            bps,
        } => cmd_build(&rom, ips_patch.as_deref(), &assets, &output, bps.as_deref()),
        Commands::BuildJpKr {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_kr(&rom, &assets, &output, &bps),
        Commands::PreviewTitleMenu {
            rom,
            assets,
            output,
        } => cmd_preview_title_menu(&rom, &assets, &output),
        Commands::PreviewTitleLogo {
            rom,
            assets,
            output,
        } => cmd_preview_title_logo(&rom, &assets, &output),
        Commands::PreviewCompileSlogan {
            rom,
            assets,
            output,
        } => cmd_preview_compile_slogan(&rom, &assets, &output),
        Commands::PreviewIntroPokan {
            rom,
            assets,
            output,
        } => cmd_preview_intro_pokan(&rom, &assets, &output),
        Commands::PreviewIntroDoki {
            rom,
            assets,
            output,
        } => cmd_preview_intro_doki(&rom, &assets, &output),
        Commands::PreviewIntroBechi {
            rom,
            assets,
            output,
        } => cmd_preview_intro_bechi(&rom, &assets, &output),
        Commands::PreviewSpellAnimations {
            rom,
            assets,
            output,
        } => cmd_preview_spell_animations(&rom, &assets, &output),
        Commands::WriteSpellAnimationMaster {
            assets,
            animation,
            output,
        } => cmd_write_spell_animation_master(&assets, &animation, &output),
        Commands::PreviewKaraoke {
            rom,
            assets,
            output,
        } => cmd_preview_karaoke(&rom, &assets, &output),
        Commands::PreviewCreditsTop {
            rom,
            assets,
            output,
        } => cmd_preview_credits_top(&rom, &assets, &output),
        Commands::PreviewCreditsTimed {
            rom,
            assets,
            output,
        } => cmd_preview_credits_timed(&rom, &assets, &output),
        Commands::PreviewTimerRemaining {
            rom,
            assets,
            output,
        } => cmd_preview_timer_remaining(&rom, &assets, &output),
        Commands::PreviewBayoenJin {
            rom,
            assets,
            output,
        } => cmd_preview_bayoen_jin(&rom, &assets, &output),
        Commands::PreviewMrFlea {
            rom,
            assets,
            output,
        } => cmd_preview_mr_flea(&rom, &assets, &output),
        Commands::PreviewDemonByun {
            rom,
            assets,
            output,
        } => cmd_preview_demon_byun(&rom, &assets, &output),
        Commands::PreviewPanottyWah {
            rom,
            assets,
            output,
        } => cmd_preview_panotty_wah(&rom, &assets, &output),
        Commands::PreviewPanottyFueen {
            rom,
            assets,
            output,
        } => cmd_preview_panotty_fueen(&rom, &assets, &output),
        Commands::PreviewPanottyPoka {
            rom,
            assets,
            output,
        } => cmd_preview_panotty_poka(&rom, &assets, &output),
        Commands::PreviewBadEndGaan {
            rom,
            assets,
            output,
        } => cmd_preview_bad_end_gaan(&rom, &assets, &output),
        Commands::PreviewExamSeals {
            rom,
            assets,
            output,
        } => cmd_preview_exam_seals(&rom, &assets, &output),
        Commands::PreviewExamCard {
            rom,
            assets,
            output,
        } => cmd_preview_exam_card(&rom, &assets, &output),
        Commands::PreviewEscapeDoors {
            rom,
            assets,
            output,
        } => cmd_preview_escape_doors(&rom, &assets, &output),
        Commands::BuildJpPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpRaw { rom, output, bps } => {
            cmd_build_jp_raw(&rom, &output, bps.as_deref())
        }
        Commands::BuildJpFontPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_font_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpTextPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_text_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpMenuPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_menu_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDialogPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dialog_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpItemQuotedPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_item_quoted_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpItemDescPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_item_desc_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpItemUsePoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_item_use_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpItemUse2Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_item_use2_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpRecoveryPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_recovery_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpHealthPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_health_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpMpRemainingPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_mp_remaining_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnemyActionPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_enemy_action_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnemyReactionPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_enemy_reaction_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnemyBattle2Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_enemy_battle2_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnemyBattle3Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_enemy_battle3_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnemyBattle4Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_enemy_battle4_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent1Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event1_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonChoice1Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_choice1_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent2Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event2_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent3Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event3_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent4Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event4_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent5Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event5_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent6Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event6_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent7Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event7_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent8Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event8_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent9Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event9_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent10Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event10_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent11Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event11_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent12Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event12_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent13Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event13_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent14Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event14_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent15Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event15_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent16Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event16_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent17Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event17_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent18Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event18_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent19Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event19_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent20Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event20_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent21Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event21_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent22Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event22_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent23Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event23_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent24Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event24_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDungeonEvent25Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_dungeon_event25_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpShop1Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_shop1_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpShop2Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_shop2_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpShop3Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_shop3_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpShop4Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_shop4_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpIntro1Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_intro1_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpIntro2Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_intro2_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpIntro3Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_intro3_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnding1Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_ending1_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnding2Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_ending2_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnding3Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_ending3_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpSystem1Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_system1_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpSystem2Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_system2_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpSystem3Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_system3_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpSystem4Poc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_system4_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpMonsterNamesPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_monster_names_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDamageVoicePoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_damage_voice_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEarlyPuyoBattlePoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_early_puyo_battle_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEncounterIntroPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_encounter_intro_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpDamageNovoicePoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_damage_novoice_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpSpellMsgPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_spell_msg_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEventAliasPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_event_alias_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpSpellCommandPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_spell_command_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpItemEventPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_item_event_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnemyHpPoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_enemy_hp_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::BuildJpEnemyDamagePoc {
            rom,
            assets,
            output,
            bps,
        } => cmd_build_jp_enemy_damage_poc(&rom, &assets, &output, bps.as_deref()),
        Commands::AuditJpFff8Ownership { assets } => cmd_audit_jp_fff8_ownership(&assets),
        Commands::AuditJpEnemyStatusConsumers { rom } => cmd_audit_jp_enemy_status_consumers(&rom),
        Commands::AuditJpPlayerDamageConsumers { rom } => {
            cmd_audit_jp_player_damage_consumers(&rom)
        }
        Commands::AuditEnPatchCoverage { assets, asm } => {
            cmd_audit_en_patch_coverage(&assets, asm.as_deref())
        }
        Commands::CheckCtrl { assets } => check::ctrl_codes::run(&assets).map_err(|e| e.into()),
        Commands::CheckOverflow {
            rom,
            ips: ips_patch,
            assets,
        } => cmd_check_overflow(&rom, ips_patch.as_deref(), &assets),
        Commands::Init {
            rom,
            ips: ips_patch,
            assets,
        } => cmd_init(&rom, ips_patch.as_deref(), &assets),
        Commands::Align {
            jp_rom,
            en_rom,
            ips: ips_patch,
            assets,
            output,
            chunk_size,
        } => cmd_align(
            &jp_rom,
            &en_rom,
            ips_patch.as_deref(),
            &assets,
            &output,
            chunk_size,
        ),
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
