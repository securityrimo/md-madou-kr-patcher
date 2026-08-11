use std::path::Path;

use madou_kr::graphics;

pub(super) fn cmd_write_spell_animation_master(
    assets_dir: &Path,
    animation_id: &str,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary =
        graphics::write_spell_animation_font_master(assets_dir, animation_id, output_path)?;
    println!("Korean spell-animation normalized cell master saved");
    println!("  ID: {}", summary.animation_id);
    println!("  PNG: {}", output_path.display());
    println!(
        "  frames/cells: {}/{}; geometry: {}x{}; alpha bounds: {}x{}+{}+{}",
        summary.frames,
        summary.visible_cells,
        summary.width,
        summary.height,
        summary.alpha_width,
        summary.alpha_height,
        summary.alpha_x,
        summary.alpha_y,
    );
    Ok(())
}
