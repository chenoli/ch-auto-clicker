//! Native side of the themes (ARCHITECTURE §8.2, TASKS T9.1–T9.2): the window background follows
//! the theme so a light theme never flashes dark while a window opens or resizes.

use ch_core::model::{Settings, ThemeId};
use tauri::window::Color;
use tauri::{AppHandle, Manager, Runtime, Theme};

use crate::tokens::{DEFAULT_THEME, NativePalette, themes};

/// The theme actually shown: with "Seguir o Windows", light → Papel and dark → Grafite.
pub fn resolve(settings: &Settings, system: Option<Theme>) -> ThemeId {
    if settings.follow_system_theme {
        return if system == Some(Theme::Light) {
            ThemeId::Papel
        } else {
            ThemeId::Grafite
        };
    }
    settings.theme
}

pub const fn palette(id: ThemeId) -> NativePalette {
    match id {
        ThemeId::Grafite => themes::GRAFITE,
        ThemeId::Blueprint => themes::BLUEPRINT,
        ThemeId::Fosforo => themes::FOSFORO,
        ThemeId::Bancada => themes::BANCADA,
        ThemeId::Papel => themes::PAPEL,
        ThemeId::Contraste => themes::CONTRASTE,
    }
}

/// The design system's default theme (`tokens.json`).
fn default_palette() -> NativePalette {
    themes::ALL
        .into_iter()
        .find(|p| p.id == DEFAULT_THEME)
        .unwrap_or(themes::GRAFITE)
}

/// `#RRGGBB` → opaque color; falls back to the default theme's background on malformed input.
pub fn color(hex: &str) -> Color {
    parse(hex)
        .or_else(|| parse(default_palette().bg))
        .unwrap_or(Color(0, 0, 0, 0xFF))
}

fn parse(hex: &str) -> Option<Color> {
    let channel = |i: usize| {
        hex.get(i..i + 2)
            .and_then(|c| u8::from_str_radix(c, 16).ok())
    };
    match (hex.len(), channel(1), channel(3), channel(5)) {
        (7, Some(r), Some(g), Some(b)) => Some(Color(r, g, b, 0xFF)),
        _ => None,
    }
}

/// The Windows app theme, as the main window sees it.
pub fn system_theme<R: Runtime>(app: &AppHandle<R>) -> Option<Theme> {
    app.get_webview_window("main").and_then(|w| w.theme().ok())
}

/// Background for new windows.
pub fn background<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> Color {
    color(palette(resolve(settings, system_theme(app))).bg)
}

/// Repaints the native background of every open window. Overlays keep their own dark backdrop
/// and the HUD must stay transparent.
pub fn apply<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    let bg = background(app, settings);
    for (label, window) in app.webview_windows() {
        if !label.starts_with("overlay-") && label != crate::hud::LABEL {
            let _ = window.set_background_color(Some(bg));
        }
    }
}

#[cfg(test)]
mod tests {
    use ch_core::model::Locale;

    use super::*;

    #[test]
    fn follows_windows_only_when_asked() {
        let mut s = Settings::new_default(Locale::PtBr);
        s.theme = ThemeId::Blueprint;
        s.follow_system_theme = false;
        assert_eq!(resolve(&s, Some(Theme::Light)), ThemeId::Blueprint);
        s.follow_system_theme = true;
        assert_eq!(resolve(&s, Some(Theme::Light)), ThemeId::Papel);
        assert_eq!(resolve(&s, Some(Theme::Dark)), ThemeId::Grafite);
        assert_eq!(resolve(&s, None), ThemeId::Grafite);
    }

    #[test]
    fn every_theme_has_a_parsable_background() {
        for id in [
            ThemeId::Grafite,
            ThemeId::Blueprint,
            ThemeId::Fosforo,
            ThemeId::Bancada,
            ThemeId::Papel,
            ThemeId::Contraste,
        ] {
            let p = palette(id);
            assert_eq!(p.id, serde_json::to_value(id).expect("json"));
            assert_eq!(color(p.bg).3, 0xFF);
        }
        assert_eq!(color("#F5F2EC"), Color(0xF5, 0xF2, 0xEC, 0xFF));
        assert_eq!(color("nope"), Color(0x14, 0x14, 0x12, 0xFF));
        assert_eq!(color("#GG0000"), Color(0x14, 0x14, 0x12, 0xFF));
    }
}
