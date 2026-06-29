use crate::theme::Colors;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct ColorsJson {
    alpha: String,
    special: HashMap<String, String>,
    colors: HashMap<String, String>,
}

pub fn save_and_broadcast(
    colors: &Colors,
    cache_dir: &Path,
    kitty: bool,
    hyprland: bool,
    nvim: bool,
    cava: bool,
    quickshell: bool,
) -> Result<()> {
    fs::create_dir_all(cache_dir)?;

    // JSON
    let mut special = HashMap::new();
    special.insert("background".into(), colors.background.clone());
    special.insert("foreground".into(), colors.foreground.clone());
    let colors_map: HashMap<String, String> = colors
        .ansi_colors
        .iter()
        .enumerate()
        .map(|(i, c)| (format!("color{}", i), c.clone()))
        .collect();
    let json = serde_json::to_string_pretty(&ColorsJson {
        alpha: colors.alpha.clone(),
        special,
        colors: colors_map,
    })?;
    File::create(cache_dir.join("colors.json"))?.write_all(json.as_bytes())?;

    // Kitty
    let kitty_path = cache_dir.join("colors-kitty.conf");
    let mut f = File::create(&kitty_path)?;
    writeln!(f, "background {}", colors.background)?;
    writeln!(f, "foreground {}", colors.foreground)?;
    for (i, c) in colors.ansi_colors.iter().enumerate() {
        writeln!(f, "color{} {}", i, c)?;
    }

    if kitty {
        if let Err(e) = Command::new("kitty")
            .args(["@", "set-colors", "--all", &kitty_path.to_string_lossy()])
            .status()
        {
            eprintln!("[astrium] kitty: {e}");
        }
    }

    if hyprland {
        write_and_apply_hyprland(colors, cache_dir)?;
    }

    if nvim {
        write_nvim_theme(colors, cache_dir)?;
        notify_nvim_instances(cache_dir);
    }

    if cava {
        update_cava_config(colors)?;
        reload_running_cava();
    }

    if quickshell {
        write_quickshell(colors)?;
    }

    Ok(())
}

/// Writes /tmp/qs_colors.json in the Catppuccin-named palette format that
/// ilyamiro's quickshell config (MatugenColors.qml) polls once per second.
/// No signal needed — quickshell picks up the change on its own timer.
fn write_quickshell(colors: &Colors) -> Result<()> {
    let bg = parse_hex(&colors.background);
    let fg = parse_hex(&colors.foreground);
    let ansi: Vec<(u8, u8, u8)> = colors.ansi_colors.iter().map(|h| parse_hex(h)).collect();

    let g = |i: usize| ansi.get(i).copied().unwrap_or(fg);
    let mix = |a: (u8, u8, u8), b: (u8, u8, u8), t: f32| -> String {
        let l = |x: u8, y: u8| (x as f32 * (1.0 - t) + y as f32 * t) as u8;
        format!("#{:02x}{:02x}{:02x}", l(a.0, b.0), l(a.1, b.1), l(a.2, b.2))
    };
    let black = (0u8, 0u8, 0u8);

    let mut m = std::collections::BTreeMap::new();
    m.insert("base", colors.background.clone());
    m.insert("mantle", mix(bg, black, 0.4));
    m.insert("crust", mix(bg, black, 0.6));
    m.insert("text", colors.foreground.clone());
    m.insert("subtext1", mix(fg, bg, 0.15));
    m.insert("subtext0", mix(fg, bg, 0.3));
    m.insert("surface0", mix(bg, fg, 0.10));
    m.insert("surface1", mix(bg, fg, 0.18));
    m.insert("surface2", mix(bg, fg, 0.28));
    m.insert("overlay0", mix(bg, fg, 0.40));
    m.insert("overlay1", mix(bg, fg, 0.55));
    m.insert("overlay2", mix(bg, fg, 0.70));
    // Accents: the muted ansi palette is too washed-out for a status bar, so
    // we re-saturate and normalize lightness so colors read clearly against the
    // dark base. Terminal/nvim keep the muted look; only the bar gets the punch.
    m.insert("red", hex(vivid(g(1))));
    m.insert("green", hex(vivid(g(2))));
    m.insert("yellow", hex(vivid(g(3))));
    m.insert("blue", hex(vivid(g(4))));
    m.insert("mauve", hex(vivid(g(5))));
    m.insert("teal", hex(vivid(g(6))));
    m.insert("peach", hex(vivid(g(11))));
    m.insert("pink", hex(vivid(g(13))));
    m.insert("maroon", hex(vivid(g(9))));
    m.insert("sapphire", hex(vivid(g(14))));

    let json = serde_json::to_string_pretty(&m)?;
    fs::write("/tmp/qs_colors.json", json)?;
    Ok(())
}

/// Pushes a washed-out accent toward a punchy, bar-friendly color: floor the
/// saturation so greys gain hue, and pin lightness into a readable mid-range.
fn vivid(c: (u8, u8, u8)) -> (u8, u8, u8) {
    let (h, s, l) = rgb_to_hsl(c);
    let s = s.max(0.55);
    let l = l.clamp(0.55, 0.72);
    hsl_to_rgb(h, s, l)
}

fn rgb_to_hsl(c: (u8, u8, u8)) -> (f32, f32, f32) {
    let (r, g, b) = (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / d) + 2.0)
    } else {
        60.0 * (((r - g) / d) + 4.0)
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    )
}

fn parse_hex(s: &str) -> (u8, u8, u8) {
    let s = s.trim_start_matches('#');
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
    (r, g, b)
}

fn hex(c: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// Patches the user's ~/.config/cava/config in-place: replaces the block
/// between `# >>> astrium` / `# <<< astrium` markers (creating it on first
/// run) with a freshly computed gradient. Six stops from the ansi palette
/// gives a smooth rainbow that follows the wallpaper.
fn update_cava_config(colors: &Colors) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let cfg_path = std::path::PathBuf::from(format!("{home}/.config/cava/config"));
    if let Some(p) = cfg_path.parent() {
        let _ = fs::create_dir_all(p);
    }

    // All vibrant ANSI slots — colors 1..7 and 9..15 — skipping color0/8
    // (background and grey) which would muddy the gradient. Gives cava 14
    // smoothly interpolated stops covering the full palette.
    let mut stops: Vec<&str> = Vec::new();
    for i in [1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15] {
        if let Some(c) = colors.ansi_colors.get(i) {
            stops.push(c);
        }
    }

    let mut block = format!(
        "# >>> astrium\n[color]\ngradient = 1\ngradient_count = {}\n",
        stops.len()
    );
    for (i, hex) in stops.iter().enumerate() {
        block.push_str(&format!("gradient_color_{} = '{}'\n", i + 1, hex));
    }
    block.push_str("# <<< astrium\n");

    let existing = fs::read_to_string(&cfg_path).unwrap_or_default();
    let new = replace_block(&existing, &block);
    fs::write(&cfg_path, new)?;
    Ok(())
}

fn replace_block(existing: &str, block: &str) -> String {
    const START: &str = "# >>> astrium";
    const END: &str = "# <<< astrium";
    if let (Some(s), Some(e)) = (existing.find(START), existing.find(END)) {
        let end = e + END.len();
        let mut out = String::new();
        out.push_str(&existing[..s]);
        out.push_str(block.trim_end());
        out.push_str(&existing[end..]);
        return out;
    }

    let mut out = existing.to_string();
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(block);
    out
}

/// Cava reloads its config on SIGUSR1 (since 0.7.4). We pgrep + signal all
/// running instances so colors update live without the user restarting it.
fn reload_running_cava() {
    let _ = Command::new("pkill").args(["-USR1", "-x", "cava"]).status();
}

/// Pushes an instant reload to every running nvim instance that registered
/// a socket under `cache_dir/nvim-sockets` (see init.lua). Stale sockets
/// (nvim exited without cleaning up) are removed on failed connection.
fn notify_nvim_instances(cache_dir: &Path) {
    let sock_dir = cache_dir.join("nvim-sockets");
    let entries = match fs::read_dir(&sock_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let sock = entry.path();
        let ok = Command::new("nvim")
            .args([
                "--server",
                &sock.to_string_lossy(),
                "--remote-expr",
                "v:lua.AstriumReload()",
            ])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);

        if !ok {
            let _ = fs::remove_file(&sock);
        }
    }
}

fn write_and_apply_hyprland(colors: &Colors, cache_dir: &Path) -> Result<()> {
    let active = colors
        .ansi_colors
        .get(4)
        .cloned()
        .unwrap_or_else(|| "#89b4fa".into());
    let inactive = colors
        .ansi_colors
        .get(8)
        .cloned()
        .unwrap_or_else(|| "#595959".into());

    let active_hex = active.trim_start_matches('#');
    let inactive_hex = inactive.trim_start_matches('#');
    let bg_hex = colors.background.trim_start_matches('#');

    let hypr_path = cache_dir.join("colors-hyprland.conf");
    let mut f = File::create(&hypr_path)?;
    writeln!(f, "general {{")?;
    writeln!(f, "    col.active_border = rgba({active_hex}ee)")?;
    writeln!(f, "    col.inactive_border = rgba({inactive_hex}aa)")?;
    writeln!(f, "}}")?;
    writeln!(f, "decoration {{")?;
    writeln!(f, "    shadow {{")?;
    writeln!(f, "        color = rgba({bg_hex}ee)")?;
    writeln!(f, "    }}")?;
    writeln!(f, "}}")?;

    let commands = [
        ("general:col.active_border", format!("rgba({active_hex}ee)")),
        (
            "general:col.inactive_border",
            format!("rgba({inactive_hex}aa)"),
        ),
    ];

    for (key, value) in commands {
        if let Err(e) = Command::new("hyprctl")
            .args(["keyword", key, &value])
            .status()
        {
            eprintln!("[astrium] hyprctl {key}: {e}");
        }
    }

    Ok(())
}

fn write_nvim_theme(colors: &Colors, cache_dir: &Path) -> Result<()> {
    let path = cache_dir.join("nvim-theme.lua");
    let mut f = File::create(&path)?;

    let bg = &colors.background;
    let fg = &colors.foreground;
    let c = &colors.ansi_colors;

    let get = |i: usize, default: &str| c.get(i).cloned().unwrap_or_else(|| default.to_string());

    writeln!(f, "-- Auto-generated by astrium, do not edit by hand")?;
    writeln!(f, "local M = {{}}")?;
    writeln!(f)?;
    writeln!(f, "function M.apply()")?;
    writeln!(f, "  vim.cmd('highlight clear')")?;
    writeln!(f, "  vim.o.background = 'dark'")?;
    writeln!(f)?;
    writeln!(f, "  local hl = vim.api.nvim_set_hl")?;
    writeln!(f, "  hl(0, 'Normal', {{ fg = '{fg}', bg = '{bg}' }})")?;
    writeln!(f, "  hl(0, 'NormalFloat', {{ fg = '{fg}', bg = '{bg}' }})")?;
    writeln!(
        f,
        "  hl(0, 'CursorLine', {{ bg = '{}' }})",
        get(8, "#444444")
    )?;
    writeln!(f, "  hl(0, 'LineNr', {{ fg = '{}' }})", get(8, "#666666"))?;
    writeln!(
        f,
        "  hl(0, 'Comment', {{ fg = '{}', italic = true }})",
        get(8, "#888888")
    )?;
    writeln!(f, "  hl(0, 'String', {{ fg = '{}' }})", get(2, "#a6e3a1"))?;
    writeln!(f, "  hl(0, 'Function', {{ fg = '{}' }})", get(4, "#89b4fa"))?;
    writeln!(f, "  hl(0, 'Keyword', {{ fg = '{}' }})", get(5, "#cba6f7"))?;
    writeln!(f, "  hl(0, 'Type', {{ fg = '{}' }})", get(3, "#f9e2af"))?;
    writeln!(
        f,
        "  hl(0, 'Identifier', {{ fg = '{}' }})",
        get(6, "#94e2d5")
    )?;
    writeln!(f, "  hl(0, 'Constant', {{ fg = '{}' }})", get(1, "#f38ba8"))?;
    writeln!(
        f,
        "  hl(0, 'Statement', {{ fg = '{}' }})",
        get(5, "#cba6f7")
    )?;
    writeln!(f, "  hl(0, 'Visual', {{ bg = '{}' }})", get(8, "#45475a"))?;
    writeln!(
        f,
        "  hl(0, 'Search', {{ bg = '{}', fg = '{bg}' }})",
        get(3, "#f9e2af")
    )?;
    writeln!(
        f,
        "  hl(0, 'Pmenu', {{ fg = '{fg}', bg = '{}' }})",
        get(8, "#313244")
    )?;
    writeln!(
        f,
        "  hl(0, 'PmenuSel', {{ fg = '{bg}', bg = '{}' }})",
        get(4, "#89b4fa")
    )?;

    // Neo-tree
    writeln!(
        f,
        "  hl(0, 'NeoTreeDirectoryIcon', {{ fg = '{}' }})",
        get(4, "#89b4fa")
    )?;
    writeln!(
        f,
        "  hl(0, 'NeoTreeDirectoryName', {{ fg = '{}' }})",
        get(4, "#89b4fa")
    )?;
    writeln!(f, "  hl(0, 'NeoTreeFileName', {{ fg = '{fg}' }})")?;
    writeln!(
        f,
        "  hl(0, 'NeoTreeRootName', {{ fg = '{}', bold = true }})",
        get(5, "#cba6f7")
    )?;
    writeln!(
        f,
        "  hl(0, 'NeoTreeNormal', {{ fg = '{fg}', bg = 'none' }})"
    )?;
    writeln!(
        f,
        "  hl(0, 'NeoTreeNormalNC', {{ fg = '{fg}', bg = 'none' }})"
    )?;
    writeln!(
        f,
        "  hl(0, 'NeoTreeIndentMarker', {{ fg = '{}' }})",
        get(8, "#444444")
    )?;
    writeln!(
        f,
        "  hl(0, 'NeoTreeGitModified', {{ fg = '{}' }})",
        get(3, "#f9e2af")
    )?;
    writeln!(
        f,
        "  hl(0, 'NeoTreeGitAdded', {{ fg = '{}' }})",
        get(2, "#a6e3a1")
    )?;

    writeln!(f, "end")?;
    writeln!(f)?;
    writeln!(f, "return M")?;

    Ok(())
}
