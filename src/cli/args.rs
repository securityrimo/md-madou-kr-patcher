//! Clap command-line schema.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "madou-kr")]
#[command(about = "Madou Monogatari I Korean patch tool")]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Subcommand)]
pub(super) enum Commands {
    /// Create a BPS patch from source and target ROMs
    Create {
        /// Source ROM (JP original or English v1.1)
        #[arg(long)]
        source: PathBuf,
        /// Target ROM (Korean patched)
        #[arg(long)]
        target: PathBuf,
        /// Output BPS patch file
        #[arg(long)]
        output: PathBuf,
    },
    /// Apply a BPS patch to a source ROM
    Apply {
        /// Source ROM (JP original or English v1.1)
        #[arg(long)]
        rom: PathBuf,
        /// BPS patch file
        #[arg(long)]
        patch: PathBuf,
        /// Output patched ROM
        #[arg(long)]
        output: PathBuf,
    },
    /// Apply an IPS patch to a source ROM
    ApplyIps {
        /// Source ROM (JP)
        #[arg(long)]
        rom: PathBuf,
        /// IPS patch file
        #[arg(long)]
        patch: PathBuf,
        /// Output patched ROM
        #[arg(long)]
        output: PathBuf,
    },
    /// Legacy EN-based comparison build; use build-jp-kr for canonical outputs
    Build {
        /// ROM path (EN v1.1, or JP if --ips is provided)
        #[arg(long)]
        rom: PathBuf,
        /// IPS patch to apply to JP ROM first (produces EN ROM in-memory)
        #[arg(long)]
        ips: Option<PathBuf>,
        /// Assets directory (translation/, charmap.json, neodgm.ttf)
        #[arg(long)]
        assets: PathBuf,
        /// Output KR ROM path
        #[arg(long)]
        output: PathBuf,
        /// Also generate BPS patch file
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the canonical KR ROM and JP-to-KR BPS from source inputs only
    BuildJpKr {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB Korean ROM path
        #[arg(long)]
        output: PathBuf,
        /// Output BPS patch path; the exact JP ROM is always the patch source
        #[arg(long)]
        bps: PathBuf,
    },
    /// Render a static JP-source title-menu QA preview (not runtime proof)
    PreviewTitleMenu {
        /// Exact JP ROM containing the original title graphics pack
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render a static JP-source Korean title-logo QA preview (not runtime proof)
    PreviewTitleLogo {
        /// Exact JP ROM containing the original title tile and map packs
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP-source Korean Compile-slogan pixel fit (not runtime proof)
    PreviewCompileSlogan {
        /// Exact JP ROM containing the original Compile splash packs
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP-source Korean automatic-prologue `콩!` effect
    PreviewIntroPokan {
        /// Exact JP ROM containing the original prologue effect packs
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP-source Korean automatic-prologue `두근!` effect
    PreviewIntroDoki {
        /// Exact JP ROM containing the original prologue effect packs
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP-source Korean final-prologue `꽈당` impact
    PreviewIntroBechi {
        /// Exact JP ROM containing the original final-prologue effect packs
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render both five-frame JP/Korean spell animations in one contact sheet
    PreviewSpellAnimations {
        /// Exact JP ROM containing the Bayoen and Cockadoodle transfer groups
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Write one pinned-font spell animation as a normalized binary cell master
    WriteSpellAnimationMaster {
        /// Source assets directory containing graphics_text/ and the pinned font
        #[arg(long)]
        assets: PathBuf,
        /// Exact manifest animation ID
        #[arg(long)]
        animation: String,
        /// Output 640x112 RGBA PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render all nine JP/Korean Madou Ondo lyric surfaces side by side
    PreviewKaraoke {
        /// Exact JP ROM containing the original karaoke pattern and map group
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render all ten original JP/Korean ending-credit headings side by side
    PreviewCreditsTop {
        /// Exact JP ROM containing the ten original ending-credit page packs
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the integrated fixed-cell JP/Korean timed credit names
    PreviewCreditsTimed {
        /// Exact JP ROM containing the original ending-credit page packs
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render JP `あと [digits] 秒` and Korean `앞으로 [digits]초` side by side
    PreviewTimerRemaining {
        /// Exact JP ROM containing the original timer pattern pack and sprite table
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render JP `じ〜ん` and Korean `찡…` side by side
    PreviewBayoenJin {
        /// Exact JP ROM containing the battle-main effect pack
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the three JP/Korean Mr. Flea battle tags
    PreviewMrFlea {
        /// Exact JP ROM containing the shared enemy and amigo pack
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render JP `びゅんっ` and Korean `휙!` side by side
    PreviewDemonByun {
        /// Exact JP ROM containing the demon-escape effect pack
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP-source Korean Panotty `와!` battle effect
    PreviewPanottyWah {
        /// Exact JP ROM containing the shared enemy and amigo effect pack
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP and Korean Panotty defeat cries side by side
    PreviewPanottyFueen {
        /// Exact JP ROM containing the shared enemy and amigo effect pack
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP-source Korean Panotty `퍽` battle effect
    PreviewPanottyPoka {
        /// Exact JP ROM containing the shared enemy and amigo main pack
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP-source Korean bad-ending `뜨악` effect
    PreviewBadEndGaan {
        /// Exact JP ROM containing the original bad-ending transfer group
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the exact JP-source Korean graduation-exam seals
    PreviewExamSeals {
        /// Exact JP ROM containing the original exam-result transfer group
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render the JP and Korean graduation-exam result cards side by side
    PreviewExamCard {
        /// Exact JP ROM containing the original exam-result transfer group
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Render JP `ひ・じ・ば` and Korean `ㅎ・ㅈ・ㅂ` escape-door marks
    PreviewEscapeDoors {
        /// Exact JP ROM containing the original ending-escape door-mark pack
        #[arg(long)]
        rom: PathBuf,
        /// Source assets directory containing graphics_text/
        #[arg(long)]
        assets: PathBuf,
        /// Output PNG path
        #[arg(long)]
        output: PathBuf,
    },
    /// Build a diagnostic Hangul PoC directly from the exact JP ROM
    BuildJpPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output diagnostic ROM path
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Expand the exact JP ROM to a 4 MiB JP-native M0 baseline
    BuildJpRaw {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Output 4 MiB JP-native baseline ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M1 full fixed-font consumer proof
    BuildJpFontPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M1 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M2 stable-text redirect proof
    BuildJpTextPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M2 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M3 item-name and dynamic-buffer proof
    BuildJpMenuPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M3 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M4 runtime-proven opening-dialog proof
    BuildJpDialogPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M4 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M5 quoted-item text proof
    BuildJpItemQuotedPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M5 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M6 item-description text proof
    BuildJpItemDescPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M6 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M7 ordinary item-use text proof
    BuildJpItemUsePoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M7 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M8 battle item-use text proof
    BuildJpItemUse2Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M8 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M9 recovery-result text proof
    BuildJpRecoveryPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M9 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M10 health-status text proof
    BuildJpHealthPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M10 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M11 remaining-MP status text proof
    BuildJpMpRemainingPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M11 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M12 Puyo battle-action text proof
    BuildJpEnemyActionPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M12 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M13 enemy action and reaction text proof
    BuildJpEnemyReactionPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M13 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M14 second enemy battle-text batch
    BuildJpEnemyBattle2Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M14 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M15 third enemy battle-text batch
    BuildJpEnemyBattle3Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M15 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M16 fourth enemy battle-text batch
    BuildJpEnemyBattle4Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M16 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M17 first dungeon-event batch
    BuildJpDungeonEvent1Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M17 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M18 first dungeon choice and button-event batch
    BuildJpDungeonChoice1Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M18 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M19 second dungeon-event batch
    BuildJpDungeonEvent2Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M19 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M20 third dungeon-event batch
    BuildJpDungeonEvent3Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M20 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M21 fourth dungeon-event batch
    BuildJpDungeonEvent4Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M21 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M22 fifth dungeon-event batch
    BuildJpDungeonEvent5Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M22 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M23 sixth dungeon-event batch
    BuildJpDungeonEvent6Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M23 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M24 seventh dungeon-event batch
    BuildJpDungeonEvent7Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M24 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M25 eighth dungeon-event batch
    BuildJpDungeonEvent8Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M25 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M26 ninth dungeon-event batch
    BuildJpDungeonEvent9Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M26 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M27 tenth dungeon-event batch
    BuildJpDungeonEvent10Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M27 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M28 eleventh dungeon-event batch
    BuildJpDungeonEvent11Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M28 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M29 twelfth dungeon-event batch
    BuildJpDungeonEvent12Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M29 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M30 thirteenth dungeon-event batch
    BuildJpDungeonEvent13Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M30 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M31 fourteenth dungeon-event batch
    BuildJpDungeonEvent14Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M31 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M32 fifteenth dungeon-event batch
    BuildJpDungeonEvent15Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M32 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M33 sixteenth dungeon-event batch
    BuildJpDungeonEvent16Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M33 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M34 seventeenth dungeon-event batch
    BuildJpDungeonEvent17Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M34 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M35 eighteenth dungeon-event batch
    BuildJpDungeonEvent18Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M35 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M36 nineteenth dungeon-event batch
    BuildJpDungeonEvent19Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M36 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M37 twentieth dungeon-event batch
    BuildJpDungeonEvent20Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M37 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M38 twenty-first dungeon-event batch
    BuildJpDungeonEvent21Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M38 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M39 twenty-second dungeon-event batch
    BuildJpDungeonEvent22Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M39 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M40 twenty-third dungeon-event batch
    BuildJpDungeonEvent23Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M40 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M41 twenty-fourth dungeon-event batch
    BuildJpDungeonEvent24Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M41 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M42 twenty-fifth dungeon-event batch
    BuildJpDungeonEvent25Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M42 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M43 first shop batch
    BuildJpShop1Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M43 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M44 second shop batch
    BuildJpShop2Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M44 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M45 third shop batch
    BuildJpShop3Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M45 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M46 fourth shop batch
    BuildJpShop4Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M46 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M47 first automatic-prologue batch
    BuildJpIntro1Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M47 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M48 second automatic-intro batch
    BuildJpIntro2Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M48 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M49 automatic-intro final fall-effect batch
    BuildJpIntro3Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M49 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M50 first ending escape-and-rescue batch
    BuildJpEnding1Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M50 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M51 ending aftermath-and-score batch
    BuildJpEnding2Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M51 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M52 ending outcome-and-gift batch
    BuildJpEnding3Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M52 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M55 save, floor-label, and encounter system batch
    BuildJpSystem1Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M55 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M56 chest, item, note, and door system batch
    BuildJpSystem2Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M56 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M57 door, stair, spell-tutorial, and hazard system batch
    BuildJpSystem3Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M57 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M59 exhausted-wall, floating-stone, and dark-passage batch
    BuildJpSystem4Poc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M59 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M60 six-word monster-name table
    BuildJpMonsterNamesPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M60 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M61 voiced-damage text batch
    BuildJpDamageVoicePoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M61 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M62 early Puyo battle text batch
    BuildJpEarlyPuyoBattlePoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M62 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M63 monster-encounter introduction batch
    BuildJpEncounterIntroPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M63 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M64 non-voiced damage batch
    BuildJpDamageNovoicePoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M64 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M65 spell-message batch
    BuildJpSpellMsgPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M65 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M66 standalone-event and shared-prefix boundary
    BuildJpEventAliasPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M66 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M67 spell-command table
    BuildJpSpellCommandPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M67 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M68 special item-event texts
    BuildJpItemEventPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M68 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M69 enemy-health status ladder
    BuildJpEnemyHpPoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M69 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Build the JP-native M70 enemy-damage response ladder
    BuildJpEnemyDamagePoc {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
        /// Assets directory containing translation/ and neodgm.ttf
        #[arg(long)]
        assets: PathBuf,
        /// Output 4 MiB JP-native M70 ROM
        #[arg(long)]
        output: PathBuf,
        /// Also generate a BPS patch against the JP ROM
        #[arg(long)]
        bps: Option<PathBuf>,
    },
    /// Audit ownership of every legacy EN FFF8 index against JP-native consumers
    AuditJpFff8Ownership {
        /// Assets directory containing translation/
        #[arg(long)]
        assets: PathBuf,
    },
    /// Audit the JP battle object's direct M69/M70 status-table bindings
    AuditJpEnemyStatusConsumers {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
    },
    /// Audit normal, Camus-cutscene, and A-capsule Amigo damage-table bindings
    AuditJpPlayerDamageConsumers {
        /// Exact 2 MiB JP ROM (SHA-256 is validated)
        #[arg(long)]
        rom: PathBuf,
    },
    /// Audit all original-ROM installation sites in the pinned JP-to-EN reference
    AuditEnPatchCoverage {
        /// Assets directory containing en_patch_coverage.json
        #[arg(long)]
        assets: PathBuf,
        /// Optional pinned madou1md.asm path for exact SHA-256 and site comparison
        #[arg(long)]
        asm: Option<PathBuf>,
    },
    /// Check control code integrity (EN vs KR)
    CheckCtrl {
        /// Assets directory
        #[arg(long)]
        assets: PathBuf,
    },
    /// Check text overflow (pixel width)
    CheckOverflow {
        /// ROM path (EN v1.1, or JP if --ips is provided)
        #[arg(long)]
        rom: PathBuf,
        /// IPS patch to apply to JP ROM first (produces EN ROM in-memory)
        #[arg(long)]
        ips: Option<PathBuf>,
        /// Assets directory
        #[arg(long)]
        assets: PathBuf,
    },
    /// Generate derived assets (charmap.json, en_reference.json, text_en.json) from EN ROM
    Init {
        /// ROM path (EN v1.1, or JP if --ips is provided)
        #[arg(long)]
        rom: PathBuf,
        /// IPS patch to apply to JP ROM first (produces EN ROM in-memory)
        #[arg(long)]
        ips: Option<PathBuf>,
        /// Assets output directory
        #[arg(long)]
        assets: PathBuf,
    },
    /// Generate JP-EN-KR complete text alignment (chunked JSON files)
    Align {
        /// JP ROM path
        #[arg(long)]
        jp_rom: PathBuf,
        /// EN ROM path (or JP ROM if --ips provided)
        #[arg(long)]
        en_rom: PathBuf,
        /// IPS patch (optional, if en_rom is actually JP ROM)
        #[arg(long)]
        ips: Option<PathBuf>,
        /// Assets directory
        #[arg(long)]
        assets: PathBuf,
        /// Output directory for chunked JSON files
        #[arg(long)]
        output: PathBuf,
        /// Entries per JSON file
        #[arg(long, default_value = "32")]
        chunk_size: usize,
    },
}
