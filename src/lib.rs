//! Extract a Material You palette from a wallpaper and broadcast it to the
//! desktop (kitty, Hyprland, neovim, cava, quickshell).

pub mod broadcast;
pub mod color;
pub mod config;
pub mod template;
pub mod theme;

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Full pipeline: set the wallpaper via awww, extract the accent, build the
/// palette, write all enabled outputs.
pub fn apply(image_path: &Path, cfg: &config::Config, cache_dir: &Path) -> Result<()> {
    apply_with(image_path, cfg, cache_dir, true)
}

/// As `apply`, but `set_wallpaper = false` skips the awww call — for callers
/// that already displayed the image and only want the palette regenerated.
pub fn apply_with(
    image_path: &Path,
    cfg: &config::Config,
    cache_dir: &Path,
    set_wallpaper: bool,
) -> Result<()> {
    if set_wallpaper {
        let _ = Command::new("awww")
            .args(["img", &image_path.to_string_lossy()])
            .status();
    }

    let source = color::extract_source_color(image_path)?;
    let colors = theme::build_colors(source, cfg);

    broadcast::save_and_broadcast(
        &colors,
        cache_dir,
        cfg.outputs.kitty,
        cfg.outputs.hyprland,
        cfg.outputs.nvim,
        cfg.outputs.cava,
        cfg.outputs.quickshell,
    )?;

    render_user_templates(&colors, cfg);
    Ok(())
}

/// Render every `[[templates]]` entry; a broken one is logged and skipped so
/// it never aborts the rest of the retheme.
fn render_user_templates(colors: &theme::Colors, cfg: &config::Config) {
    if cfg.templates.is_empty() {
        return;
    }
    let vars = template::build_vars(colors);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let expand = |p: &str| -> PathBuf {
        if let Some(rest) = p.strip_prefix("~/") {
            PathBuf::from(format!("{home}/{rest}"))
        } else {
            PathBuf::from(p)
        }
    };

    for t in &cfg.templates {
        let input = expand(&t.input);
        let output = expand(&t.output);
        if let Err(e) = template::render(&input, &output, &vars) {
            eprintln!("[astrium] template {} -> {}: {e:?}", t.input, t.output);
        }
    }
}

/// Extract a palette and write every artifact into `out_dir` with zero side
/// effects (no awww/kitty/hyprctl, no /tmp). Pure and sandbox-safe — this is
/// what lets Nix bake a palette into a derivation at build time.
pub fn generate(image_path: &Path, out_dir: &Path, cfg: &config::Config) -> Result<()> {
    let source = color::extract_source_color(image_path)?;
    let colors = theme::build_colors(source, cfg);
    broadcast::write_static(&colors, out_dir)?;
    render_user_templates(&colors, cfg);
    Ok(())
}

/// The wallpaper awww is currently displaying, if any.
pub fn current_wallpaper() -> Option<PathBuf> {
    let out = Command::new("awww").arg("query").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    // Lines: `<output>: …, currently displaying: image: /path/to.jpg`
    text.lines()
        .find_map(|line| line.split("image: ").nth(1))
        .map(|s| PathBuf::from(s.trim().trim_end_matches(',').trim()))
}
