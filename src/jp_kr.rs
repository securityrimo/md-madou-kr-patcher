//! Canonical JP-source to Korean ROM and BPS build pipeline.
//!
//! This entry point deliberately accepts only the exact supported JP ROM and
//! repository-owned localization assets. English ROMs, JP-to-EN patches, and
//! previously patched ROMs are not inputs to this build graph.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{bps, graphics, graphics_catalog, jp_native};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildArtifacts {
    pub rom: Vec<u8>,
    pub bps: Vec<u8>,
    pub source_sha256: String,
    pub rom_sha256: String,
    pub bps_sha256: String,
}

/// Build the current cumulative Korean ROM and its JP-source BPS patch.
///
/// The native builder validates the supported JP revision, translation
/// records, font input, typed 68000 code generation, Expected Writes, and the
/// final Mega Drive checksum. This wrapper then creates the BPS patch against
/// that same JP source and reapplies it in memory before returning either
/// artifact.
pub fn build_from_sources(jp_rom_path: &Path, assets_dir: &Path) -> Result<BuildArtifacts, String> {
    graphics_catalog::validate(assets_dir)?;
    let source =
        fs::read(jp_rom_path).map_err(|error| format!("failed to read JP source ROM: {error}"))?;
    let mut rom = jp_native::build_jp_kr(jp_rom_path, assets_dir)?;
    graphics::apply_title_menu(&source, &mut rom, assets_dir)?;
    graphics::apply_title_logo(&source, &mut rom, assets_dir)?;
    graphics::apply_compile_slogan(&source, &mut rom, assets_dir)?;
    graphics::apply_intro_pokan(&source, &mut rom, assets_dir)?;
    graphics::apply_intro_doki(&source, &mut rom, assets_dir)?;
    graphics::apply_intro_bechi(&source, &mut rom, assets_dir)?;
    graphics::apply_spell_animations(&source, &mut rom, assets_dir)?;
    graphics::apply_karaoke(&source, &mut rom, assets_dir)?;
    graphics::apply_credits_top(&source, &mut rom, assets_dir)?;
    graphics::apply_timer_remaining(&source, &mut rom, assets_dir)?;
    graphics::apply_bayoen_jin(&source, &mut rom, assets_dir)?;
    graphics::apply_mr_flea(&source, &mut rom, assets_dir)?;
    graphics::apply_demon_byun(&source, &mut rom, assets_dir)?;
    graphics::apply_panotty_wah(&source, &mut rom, assets_dir)?;
    graphics::apply_panotty_poka(&source, &mut rom, assets_dir)?;
    graphics::apply_bad_end_gaan(&source, &mut rom, assets_dir)?;
    graphics::apply_exam_seals(&source, &mut rom, assets_dir)?;
    graphics::apply_exam_card(&source, &mut rom, assets_dir)?;
    graphics::apply_escape_doors(&source, &mut rom, assets_dir)?;
    let source_after = fs::read(jp_rom_path)
        .map_err(|error| format!("failed to reread JP source ROM: {error}"))?;
    if source_after != source {
        return Err("JP source ROM changed while the build was running".to_string());
    }
    let patch = bps::create(&source, &rom)?;
    let reapplied = bps::apply(&source, &patch)
        .map_err(|error| format!("generated JP-to-KR BPS failed verification: {error}"))?;
    if reapplied != rom {
        return Err("generated JP-to-KR BPS does not reproduce the built KR ROM".to_string());
    }

    Ok(BuildArtifacts {
        source_sha256: sha256_hex(&source),
        rom_sha256: sha256_hex(&rom),
        bps_sha256: sha256_hex(&patch),
        rom,
        bps: patch,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_lowercase_and_stable() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
