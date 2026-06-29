use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub outputs: OutputConfig,
    /// User-defined templates rendered on every retheme. Each entry reads
    /// `input`, substitutes `{{var}}` placeholders, and writes `output`.
    #[serde(default)]
    pub templates: Vec<TemplateConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TemplateConfig {
    /// Path to the template file (supports a leading `~`).
    pub input: String,
    /// Where to write the rendered result (supports a leading `~`).
    pub output: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ThemeConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_bg_darken")]
    pub bg_darken: f32,
    #[serde(default = "default_fg_mute")]
    pub fg_mute: f32,
    #[serde(default = "default_ansi_mute")]
    pub ansi_mute: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OutputConfig {
    #[serde(default = "default_true")]
    pub kitty: bool,
    #[serde(default = "default_true")]
    pub hyprland: bool,
    #[serde(default = "default_true")]
    pub nvim: bool,
    #[serde(default = "default_true")]
    pub cava: bool,
    #[serde(default = "default_true")]
    pub quickshell: bool,
}

fn default_mode() -> String {
    "dark".to_string()
}
fn default_bg_darken() -> f32 {
    0.4
}
fn default_fg_mute() -> f32 {
    0.7
}
fn default_ansi_mute() -> f32 {
    0.55
}
fn default_true() -> bool {
    true
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            bg_darken: default_bg_darken(),
            fg_mute: default_fg_mute(),
            ansi_mute: default_ansi_mute(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            kitty: true,
            hyprland: true,
            nvim: true,
            cava: true,
            quickshell: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let path = PathBuf::from(format!("{}/.config/astrium/config.toml", home));

        fs::read_to_string(&path)
            .ok()
            .and_then(|c| toml::from_str(&c).ok())
            .unwrap_or_default()
    }
}
