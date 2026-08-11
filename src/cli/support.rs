//! Audits, patch utilities, and legacy EN-based support commands.

use std::path::{Path, PathBuf};

use madou_kr::{align, bps, build, check, en_patch_coverage, extract, ips, jp_native, rom};

pub(super) fn cmd_audit_en_patch_coverage(
    assets_dir: &Path,
    asm_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = en_patch_coverage::audit_en_patch_coverage(assets_dir, asm_path)?;
    print!("{}", report.render());
    Ok(())
}

pub(super) fn cmd_audit_jp_fff8_ownership(
    assets_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = jp_native::audit_fff8_ownership(assets_dir)?;
    print!("{}", report.render());
    Ok(())
}

pub(super) fn cmd_audit_jp_enemy_status_consumers(
    rom_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = jp_native::audit_enemy_status_consumers(rom_path)?;
    print!("{}", report.render());
    Ok(())
}

pub(super) fn cmd_audit_jp_player_damage_consumers(
    rom_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = jp_native::audit_player_damage_consumers(rom_path)?;
    print!("{}", report.render());
    Ok(())
}

pub(super) fn cmd_build_jp_raw(
    rom_path: &Path,
    output_path: &Path,
    bps_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let output = jp_native::build_raw(rom_path)?;
    create_parent_dir(output_path)?;
    std::fs::write(output_path, &output)?;
    println!("JP-native M0 baseline saved: {}", output_path.display());

    if let Some(bps_path) = bps_path {
        let patch = bps::create(&source, &output)?;
        create_parent_dir(bps_path)?;
        std::fs::write(bps_path, &patch)?;
        println!("JP-source M0 BPS saved: {}", bps_path.display());
    }
    Ok(())
}

pub(super) fn create_parent_dir(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub(super) fn cmd_build_jp_poc(
    rom_path: &Path,
    assets_dir: &Path,
    output_path: &Path,
    bps_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read(rom_path)?;
    let output = jp_native::build_poc(rom_path, &assets_dir.join("neodgm.ttf"))?;
    std::fs::write(output_path, &output)?;
    println!("JP-native diagnostic PoC saved: {}", output_path.display());

    if let Some(bps_path) = bps_path {
        let patch = bps::create(&source, &output)?;
        std::fs::write(bps_path, &patch)?;
        println!("JP-source BPS PoC saved: {}", bps_path.display());
    }
    Ok(())
}

pub(super) fn cmd_create(
    source: &PathBuf,
    target: &PathBuf,
    output: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_data = std::fs::read(source)?;
    let target_data = std::fs::read(target)?;
    rom::validate_rom(&source_data, "source")?;
    let patch = bps::create(&source_data, &target_data)?;
    std::fs::write(output, &patch)?;
    println!(
        "Patch created: {} ({} bytes)",
        output.display(),
        patch.len()
    );
    Ok(())
}

pub(super) fn cmd_apply(
    rom_path: &PathBuf,
    patch_path: &PathBuf,
    output: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_data = std::fs::read(rom_path)?;
    let patch_data = std::fs::read(patch_path)?;
    rom::validate_rom(&source_data, "ROM")?;
    let target_data = bps::apply(&source_data, &patch_data)?;
    std::fs::write(output, &target_data)?;
    println!(
        "Patch applied: {} ({} bytes)",
        output.display(),
        target_data.len()
    );
    Ok(())
}

pub(super) fn cmd_apply_ips(
    rom_path: &PathBuf,
    patch_path: &PathBuf,
    output: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_data = std::fs::read(rom_path)?;
    let patch_data = std::fs::read(patch_path)?;
    let target_data = ips::apply(&source_data, &patch_data)?;
    rom::validate_rom(&target_data, "patched ROM")?;
    std::fs::write(output, &target_data)?;
    println!(
        "IPS patch applied: {} ({} bytes)",
        output.display(),
        target_data.len()
    );
    Ok(())
}

/// Load EN ROM, optionally by applying IPS patch to JP ROM first.
fn load_en_rom(
    rom_path: &Path,
    ips_path: Option<&Path>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let rom_data = std::fs::read(rom_path)?;
    match ips_path {
        Some(ips) => {
            println!("Applying IPS patch: {}", ips.display());
            let patch_data = std::fs::read(ips)?;
            let en_rom = ips::apply(&rom_data, &patch_data)?;
            rom::validate_rom(&en_rom, "EN ROM (after IPS)")?;
            println!(
                "  JP ROM {} → EN ROM {} bytes",
                rom_data.len(),
                en_rom.len()
            );
            Ok(en_rom)
        }
        None => {
            rom::validate_rom(&rom_data, "EN ROM")?;
            Ok(rom_data)
        }
    }
}

pub(super) fn cmd_build(
    rom_path: &Path,
    ips_path: Option<&Path>,
    assets_dir: &Path,
    output_path: &Path,
    bps_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    // If --ips provided, write EN ROM to temp file for BuildConfig
    let en_rom_data = load_en_rom(rom_path, ips_path)?;
    let en_rom_tmp;
    let en_rom_path: &Path = if ips_path.is_some() {
        en_rom_tmp = std::env::temp_dir().join("madou_kr_en_rom.md");
        std::fs::write(&en_rom_tmp, &en_rom_data)?;
        &en_rom_tmp
    } else {
        rom_path
    };

    let config = build::BuildConfig {
        en_rom_path,
        assets_dir,
        output_path,
    };

    let kr_rom = build::build_kr_rom(&config)?;

    // Save KR ROM
    std::fs::write(output_path, &kr_rom)?;
    println!(
        "KR ROM saved: {} ({} bytes)",
        output_path.display(),
        kr_rom.len()
    );

    // Optionally generate BPS patch (always against EN ROM, not JP)
    if let Some(bps_out) = bps_path {
        let patch = bps::create(&en_rom_data, &kr_rom)?;
        std::fs::write(bps_out, &patch)?;
        println!(
            "BPS patch saved: {} ({} bytes)",
            bps_out.display(),
            patch.len()
        );
    }

    // Clean up temp file
    if ips_path.is_some() {
        let _ = std::fs::remove_file(en_rom_path);
    }

    Ok(())
}

pub(super) fn cmd_check_overflow(
    rom_path: &Path,
    ips_path: Option<&Path>,
    assets_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let en_rom_data = load_en_rom(rom_path, ips_path)?;
    let en_rom_tmp = std::env::temp_dir().join("madou_kr_overflow_en.md");
    std::fs::write(&en_rom_tmp, &en_rom_data)?;
    let result = check::overflow::run(&en_rom_tmp, assets_dir)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() });
    let _ = std::fs::remove_file(&en_rom_tmp);
    result
}

pub(super) fn cmd_align(
    jp_rom_path: &Path,
    en_rom_path: &Path,
    ips_path: Option<&Path>,
    assets_dir: &Path,
    output_dir: &Path,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let jp_rom_data = std::fs::read(jp_rom_path)?;
    let en_rom_data = load_en_rom(en_rom_path, ips_path)?;
    align::run(
        &jp_rom_data,
        &en_rom_data,
        assets_dir,
        output_dir,
        chunk_size,
    )
}

pub(super) fn cmd_init(
    rom_path: &Path,
    ips_path: Option<&Path>,
    assets_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let en_rom_data = load_en_rom(rom_path, ips_path)?;
    let en_rom_tmp;
    let en_rom_path: &Path = if ips_path.is_some() {
        en_rom_tmp = std::env::temp_dir().join("madou_kr_en_rom.md");
        std::fs::write(&en_rom_tmp, &en_rom_data)?;
        &en_rom_tmp
    } else {
        rom_path
    };

    let result = extract::run(en_rom_path, assets_dir);

    if ips_path.is_some() {
        let _ = std::fs::remove_file(en_rom_path);
    }

    result
}
