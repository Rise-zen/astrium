//! astrium — extract a Material You palette from a wallpaper and broadcast
//! it to the rest of the desktop (kitty, Hyprland, neovim, cava, quickshell).
//!
//! The CLI in `main.rs` is thin: this library does all the work so other
//! Rust apps (or future watch daemons) can drive the same pipeline without
//! re-parsing argv.

pub mod broadcast;
pub mod color;
pub mod config;
pub mod theme;

use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Run the full pipeline for one image: tell awww to display it, extract the
/// dominant accent, build a palette, write all enabled output files.
///
/// `cache_dir` is where `colors.json` and the per-target files (kitty conf,
/// hyprland conf, nvim lua, etc.) are written. The quickshell output also
/// writes `/tmp/qs_colors.json` so the bar picks it up via its polling.
pub fn apply(image_path: &Path, cfg: &config::Config, cache_dir: &Path) -> Result<()> {
    // Drive the wallpaper too — astrium is the single entrypoint for "swap
    // wallpaper + retheme everything", so callers don't have to remember to
    // call awww separately.
    let _ = Command::new("awww")
        .args(["img", &image_path.to_string_lossy()])
        .status();

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
    )
}

/// Re-theme using whatever wallpaper awww is currently displaying.
/// Returns the path so the watch loop can de-dupe consecutive identical calls.
pub fn current_wallpaper() -> Option<PathBuf> {
    let out = Command::new("awww").arg("query").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    // `awww query` lines look like:
    //   eDP-1: 1920x1080, scale: 1, currently displaying: image: /path/to.jpg
    text.lines()
        .find_map(|line| line.split("image: ").nth(1))
        .map(|s| PathBuf::from(s.trim().trim_end_matches(',').trim()))
}
