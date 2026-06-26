use crate::color::{darken, mute};
use crate::config::Config;
use material_colors::color::Argb;
use material_colors::theme::ThemeBuilder;

pub struct Colors {
    pub alpha: String,
    pub background: String,
    pub foreground: String,
    pub ansi_colors: Vec<String>,
}

pub fn build_colors(source: Argb, config: &Config) -> Colors {
    let theme = ThemeBuilder::with_source(source).build();
    let s = if config.theme.mode == "light" {
        theme.schemes.light
    } else {
        theme.schemes.dark
    };

    let bg = darken(s.surface, config.theme.bg_darken);
    let fg = mute(s.on_surface, config.theme.fg_mute);

    let roles = [
        s.surface, s.primary_container, s.primary, s.secondary,
        s.tertiary, s.on_primary, s.secondary_container,
        s.on_surface_variant, s.outline, s.inverse_primary,
        s.outline_variant, s.tertiary_container, s.on_primary_container,
        s.on_secondary_container, s.on_tertiary_container, s.on_surface,
    ];

    let f = config.theme.ansi_mute;
    let ansi_colors = roles.iter().enumerate()
        .map(|(i, &role)| {
            if i == 0 { darken(role, config.theme.bg_darken) }
            else { mute(role, f) }
        })
        .collect();

    Colors {
        alpha: "100".to_string(),
        background: bg,
        foreground: fg,
        ansi_colors,
    }
}
