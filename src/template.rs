//! User-defined `{{var}}` template rendering, so any app can be themed.

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

/// Single-pass `{{key}}` substitution. Inner whitespace is trimmed
/// (`{{ base }}` == `{{base}}`); unknown keys are copied through verbatim.
/// Slice-based so multi-byte UTF-8 in templates is preserved.
fn substitute(src: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];

        match after.find("}}") {
            Some(end) => {
                let key = after[..end].trim();
                match vars.get(key) {
                    Some(val) => out.push_str(val),
                    None => {
                        out.push_str("{{");
                        out.push_str(&after[..end]);
                        out.push_str("}}");
                    }
                }
                rest = &after[end + 2..];
            }
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }

    out.push_str(rest);
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

    #[test]
    fn preserves_multibyte_utf8() {
        assert_eq!(substitute("─ цвет {{base}} ─", &vars()), "─ цвет #1e1e2e ─");
    }

    #[test]
    fn handles_unterminated_braces() {
        assert_eq!(substitute("{{base", &vars()), "{{base");
    }
}
