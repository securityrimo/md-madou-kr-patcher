//! Canonical JP-to-KR build and deterministic static preview commands.

use std::path::Path;

use madou_kr::{graphics, jp_kr};

use super::support::create_parent_dir;

pub(super) fn cmd_build_jp_kr(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
    bps_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if output_path == rom_path || bps_path == rom_path {
        return Err("refusing to overwrite the JP source ROM".into());
    }
    if output_path == bps_path {
        return Err("KR ROM and BPS outputs must use different paths".into());
    }

    let artifacts = jp_kr::build_from_sources(rom_path, assets_dir)?;
    create_parent_dir(output_path)?;
    create_parent_dir(bps_path)?;
    std::fs::write(output_path, &artifacts.rom)?;
    std::fs::write(bps_path, &artifacts.bps)?;

    println!("JP-to-KR source-only build complete");
    println!("  source SHA-256: {}", artifacts.source_sha256);
    println!(
        "  KR ROM: {} ({} bytes, SHA-256 {})",
        output_path.display(),
        artifacts.rom.len(),
        artifacts.rom_sha256,
    );
    println!(
        "  BPS: {} ({} bytes, SHA-256 {})",
        bps_path.display(),
        artifacts.bps.len(),
        artifacts.bps_sha256,
    );
    println!("  BPS reapply: byte-identical");
    Ok(())
}

pub(super) fn cmd_preview_title_menu(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_title_menu_preview(&source, assets_dir, output_path)?;
    println!("JP-source title-menu static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  entries: {}; unique glyphs: {}; tile range: 0x{:03X}..0x{:03X}",
        summary.entries, summary.unique_glyphs, summary.first_new_tile, summary.next_free_tile,
    );
    println!("  evidence class: static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_title_logo(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_title_logo_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean title-logo static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  regions: {}; allocated tiles: {}; highest tile: 0x{:03X}",
        summary.regions, summary.allocated_tiles, summary.highest_tile,
    );
    println!("  evidence class: static main-plane QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_compile_slogan(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_compile_slogan_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean Compile-slogan static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  rewritten tiles: {}; protected decoded bytes: {}; pack bytes: {}",
        summary.rewritten_tiles,
        summary.decoded_bytes - summary.rewritten_tiles * 32,
        summary.pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_intro_pokan(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_intro_pokan_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean automatic-prologue `콩!` static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; output tiles: {}; protected pixels: {}; map/pattern packs: {}/{} bytes",
        summary.source_pattern_tiles,
        summary.output_pattern_tiles,
        summary.protected_pixels,
        summary.map_pack_bytes,
        summary.pattern_pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_intro_doki(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_intro_doki_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean automatic-prologue `두근!` static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; rewritten tiles: {}; protected pattern bytes: {}; protected companion bytes: {}; pack bytes: {}",
        summary.source_pattern_tiles,
        summary.rewritten_tiles,
        summary.protected_pattern_bytes,
        summary.companion_decoded_bytes,
        summary.pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_intro_bechi(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_intro_bechi_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean final automatic-prologue impact static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; rewritten tiles: {}; protected pattern/companion bytes: {}/{}; protected burst pixels: {}; pack bytes: {}",
        summary.source_pattern_tiles,
        summary.rewritten_tiles,
        summary.protected_pattern_bytes,
        summary.protected_companion_bytes,
        summary.protected_decoration_pixels,
        summary.pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_spell_animations(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_spell_animations_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean Bayoen and Cockadoodle five-frame preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  animations/frames: {}/{}; protected patterns: {} bytes; source/target maps: {}/{} bytes; visible Korean tiles: {}; map packs: {} bytes",
        summary.animations,
        summary.frames,
        summary.source_pattern_bytes,
        summary.source_map_bytes,
        summary.target_map_bytes,
        summary.visible_target_tiles,
        summary.pack_bytes,
    );
    println!("  rows: Bayoen JP/KR, Cockadoodle JP/KR");
    println!("  evidence class: deterministic static QA preview, not conditioned runtime");
    Ok(())
}

pub(super) fn cmd_preview_karaoke(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_karaoke_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean Madou Ondo nine-line static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  lines: {}; Korean glyphs: {}; protected pattern bytes: {}; rewritten pattern tiles/map bytes: {}/{}",
        summary.lines,
        summary.unique_glyphs,
        summary.source_pattern_bytes - summary.rewritten_pattern_tiles * 32,
        summary.rewritten_pattern_tiles,
        summary.rewritten_map_bytes,
    );
    println!("  columns: JP source / Korean target");
    println!("  evidence class: deterministic static QA preview, not conditioned runtime");
    Ok(())
}

pub(super) fn cmd_preview_credits_top(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_credits_top_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean original ending-credit heading preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  headings: {}; Korean glyphs: {}; protected/rewritten decoded bytes: {}/{}; verified loader bytes: {}",
        summary.headings,
        summary.unique_glyphs,
        summary.protected_bytes,
        summary.rewritten_tiles * 32,
        summary.verified_loader_bytes,
    );
    println!("  columns: JP source / Korean target");
    println!("  evidence class: deterministic static QA preview, not ending runtime");
    Ok(())
}

pub(super) fn cmd_preview_credits_timed(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_credits_timed_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean fixed-cell ending-credit name preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  timed lines/relocated: {}/{}; Korean glyphs across cumulative credits: {}; overwritten/appended timed tiles: {}/{}; direct-map/generic-table/native-frame-table/remaining-table/remaining-pack/typed executable bytes: {}/{}/{}/{}/{}/{}; verified direct/generic/native-frame/remaining consumer bytes: {}/{}/{}/{}",
        summary.timed_lines,
        summary.relocated_timed_lines,
        summary.unique_glyphs,
        summary.overwritten_timed_tiles,
        summary.appended_timed_tiles,
        summary.map_bank_bytes,
        summary.generic_table_bank_bytes,
        summary.native_frame_table_bank_bytes,
        summary.remaining_table_bank_bytes,
        summary.remaining_pack_bank_bytes,
        summary.written_executable_bytes,
        summary.verified_timed_consumer_bytes,
        summary.verified_generic_consumer_bytes,
        summary.verified_native_frame_consumer_bytes,
        summary.verified_remaining_consumer_bytes,
    );
    println!("  columns: JP source / Korean target");
    println!("  evidence class: deterministic static QA preview, not ending runtime");
    Ok(())
}

pub(super) fn cmd_preview_timer_remaining(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_timer_remaining_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean remaining-time static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  aliases/source/rewritten/protected-digit tiles: {}/{}/{}/{}; table/pack bytes: {}/{}; typed executable bytes: {}; verified consumer bytes: {}",
        summary.alias_headers,
        summary.source_tiles,
        summary.rewritten_tiles,
        summary.protected_digit_tiles,
        summary.table_bytes,
        summary.pack_bank_bytes,
        summary.written_executable_bytes,
        summary.verified_consumer_bytes,
    );
    println!("  columns: JP source / Korean target");
    println!("  evidence class: deterministic static QA preview, not conditioned runtime");
    Ok(())
}

pub(super) fn cmd_preview_bayoen_jin(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_bayoen_jin_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean `じ〜ん` / `찡…` static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; rewritten tiles: {}; protected bytes: {}; visible Korean pixels: {}; pack bytes: {}",
        summary.source_tiles,
        summary.rewritten_tiles,
        summary.protected_bytes,
        summary.visible_pixels,
        summary.pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not conditioned runtime");
    Ok(())
}

pub(super) fn cmd_preview_mr_flea(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_mr_flea_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean Mr. Flea battle-tag static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source/output tiles: {}/{}; rewritten: {} (appended {}); protected source bytes: {}; typed code bytes: {}; semantic data patches: {}; pack bytes: {}",
        summary.source_tiles,
        summary.output_tiles,
        summary.rewritten_tiles,
        summary.appended_tiles,
        summary.protected_source_bytes,
        summary.typed_code_bytes,
        summary.semantic_data_patches,
        summary.pack_bytes,
    );
    println!("  rows: HERE, DEFENDING, BATANKYU; JP left, KR right");
    println!("  evidence class: deterministic static QA preview, not conditioned runtime");
    Ok(())
}

pub(super) fn cmd_preview_demon_byun(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_demon_byun_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean `びゅんっ` / `휙!` static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; rewritten consumer tiles: {} (mapped {}, cleared residual {}); protected bytes: {}; consumer sprites: {}; pack bytes: {}",
        summary.source_tiles,
        summary.rewritten_tiles,
        summary.mapped_tiles,
        summary.cleared_residual_tiles,
        summary.protected_source_bytes,
        summary.consumer_sprites,
        summary.pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not conditioned runtime");
    Ok(())
}

pub(super) fn cmd_preview_panotty_wah(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_panotty_wah_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean Panotty `와!` static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; rewritten tiles: {}; protected bytes: {}; consumer headers: {}; pack bytes: {}",
        summary.source_tiles,
        summary.rewritten_tiles,
        summary.protected_bytes,
        summary.consumer_headers,
        summary.pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_panotty_fueen(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_panotty_fueen_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean Panotty `ふえ〜ん` / `흐엥` static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; rewritten tiles: {} (fueen {}, wah {}); visible Korean fueen pixels: {}; protected bytes: {}; consumer headers: {}; pack bytes: {}",
        summary.source_tiles,
        summary.rewritten_tiles,
        summary.fueen_rewritten_tiles,
        summary.wah_rewritten_tiles,
        summary.fueen_visible_pixels,
        summary.protected_bytes,
        summary.consumer_headers,
        summary.pack_bytes,
    );
    println!(
        "  evidence class: deterministic static QA preview; direct enemy/amigo defeat remains conditioned runtime"
    );
    Ok(())
}

pub(super) fn cmd_preview_panotty_poka(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_panotty_poka_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean Panotty `퍽` static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; rewritten tiles: {}; protected bytes: {}; consumer headers: {}; pack bytes: {}",
        summary.source_tiles,
        summary.rewritten_tiles,
        summary.protected_bytes,
        summary.consumer_headers,
        summary.pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_bad_end_gaan(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_bad_end_gaan_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean bad-ending `뜨악` static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; appended tiles: {}; protected bytes: {}; companion transfers: {}; target sprites: {}; pack bytes: {}",
        summary.source_tiles,
        summary.appended_tiles,
        summary.protected_bytes,
        summary.companion_transfers,
        summary.target_sprites,
        summary.pack_bytes,
    );
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_exam_seals(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_exam_seals_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean graduation-exam seal static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  source tiles: {}; rewritten tiles: {}; protected tiles: {}; companion transfers: {}; source sprite records: {}; pack bytes: {}",
        summary.source_tiles,
        summary.rewritten_tiles,
        summary.protected_tiles,
        summary.companion_transfers,
        summary.source_sprite_records,
        summary.pack_bytes,
    );
    println!("  pass: round `합격` seal; fail: diagonal `불합격` seal without an outer circle");
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_exam_card(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_exam_card_preview(&source, assets_dir, output_path)?;
    println!("JP-source Korean graduation-exam result-card static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  blocks: {}; source text pixels: {}; protected pixels: {}; source tiles: {}; appended tiles: {}; changed map cells: {}; pattern pack bytes: {}; map pack bytes: {}; typed code bytes: {}",
        summary.blocks,
        summary.source_text_pixels,
        summary.protected_pixels,
        summary.source_pattern_tiles,
        summary.appended_pattern_tiles,
        summary.changed_map_cells,
        summary.pattern_pack_bytes,
        summary.map_pack_bytes,
        summary.typed_code_bytes,
    );
    println!("  left: JP `100てんです`; right: KR `100점입니다`");
    println!("  evidence class: deterministic static QA preview, not runtime consumption");
    Ok(())
}

pub(super) fn cmd_preview_escape_doors(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let summary = graphics::write_escape_doors_preview(&source, assets_dir, output_path)?;
    println!("JP/Korean ending-escape door-mark static preview saved");
    println!("  PNG: {}", output_path.display());
    println!(
        "  marks: {}; source/rewritten/protected tiles: {}/{}/{}; source sprite records: {}; verified consumer bytes: {}; executable bytes: {}; pack bytes: {}",
        summary.marks,
        summary.source_tiles,
        summary.rewritten_tiles,
        summary.protected_tiles,
        summary.source_sprite_records,
        summary.verified_consumer_bytes,
        summary.written_executable_bytes,
        summary.pack_bytes,
    );
    println!("  rows: JP ひ・じ・ば / Korean ㅎ・ㅈ・ㅂ");
    println!(
        "  evidence class: deterministic static QA preview; spell logic and natural ending escape remain unchanged/unverified"
    );
    Ok(())
}
