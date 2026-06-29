//! User-defined template rendering.
//!
//! astrium ships built-in outputs (kitty, hyprland, nvim, cava, quickshell),
//! but those only cover the author's own setup. The template engine lets any
//! user theme any app: point astrium at an input file containing placeholders
//! like `{{base}}` or `{{color5}}` and an output path, and it renders the file
//! on every wallpaper change. Same idea as pywal/matugen templates, minimal
//! syntax, no dependencies.

use crate::theme::Colors;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Build the full variable map exposed to templates. Every entry is available
/// as `{{name}}` (with leading `#` for colors) and `{{name.strip}}` (hex digits
/// only, handy for CSS `rgba()` or Hyprland `rgba(RRGGBBAA)`).
pub fn build_vars(colors: &Colors) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    let mut insert = |key: &str, hex: &str| {
        vars.insert(key.to_string(), hex.to_string());
        vars.insert(
            format!("{key}.strip"),
            hex.trim_start_matches('#').to_string(),
        );
    };

    insert("background", &colors.background);
    insert("foreground", &colors.foreground);
    insert("cursor", &colors.foreground);
    for (i, c) in colors.ansi_colors.iter().enumerate() {
        insert(&format!("color{i}"), c);
    }
    vars.insert("alpha".to_string(), colors.alpha.clone());

    vars
}

/// Render one template: read `input`, substitute every `{{var}}`, write
/// `output`. Unknown placeholders are left untouched so a typo is visible in
/// the rendered file rather than silently blanked.
pub fn render(input: &Path, output: &Path, vars: &HashMap<String, String>) -> Result<()> {
    let src = fs::read_to_string(input)
        .with_context(|| format!("reading template {}", input.display()))?;
    let rendered = substitute(&src, vars);

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(output, rendered)
        .with_context(|| format!("writing rendered output {}", output.display()))?;
    Ok(())
}

/// Single-pass `{{key}}` substitution. Whitespace inside the braces is trimmed,
/// so `{{ base }}` and `{{base}}` are equivalent.
fn substitute(src: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(end) = src[i + 2..].find("}}") {
                let key = src[i + 2..i + 2 + end].trim();
                if let Some(val) = vars.get(key) {
                    out.push_str(val);
                    i = i + 2 + end + 2;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("base".to_string(), "#1e1e2e".to_string());
        m.insert("base.strip".to_string(), "1e1e2e".to_string());
        m
    }

    #[test]
    fn substitutes_known_keys() {
        assert_eq!(substitute("bg = {{base}}", &vars()), "bg = #1e1e2e");
    }

    #[test]
    fn trims_inner_whitespace() {
        assert_eq!(substitute("{{ base }}", &vars()), "#1e1e2e");
    }

    #[test]
    fn strip_variant_drops_hash() {
        assert_eq!(
            substitute("rgba({{base.strip}}ff)", &vars()),
            "rgba(1e1e2eff)"
        );
    }

    #[test]
    fn leaves_unknown_keys_intact() {
        assert_eq!(substitute("{{nope}}", &vars()), "{{nope}}");
    }
}
