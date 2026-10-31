use gpui::{Window, WindowBackgroundAppearance};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
pub struct Appearance {
    pub glass: bool,
    pub tint: u8,
    pub surface: u32,
    pub accent: u32,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            glass: true,
            tint: 110,
            surface: 0x0c1417,
            accent: 0x92c8e5,
        }
    }
}
impl Appearance {
    fn path() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .map(|root| PathBuf::from(root).join("ProjectLauncher/appearance.json"))
    }
    pub fn load() -> Self {
        let mut settings: Self = Self::path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        settings.tint = settings.tint.clamp(25, 235);
        settings.surface &= 0xffffff;
        settings.accent &= 0xffffff;
        if !dark_surface(settings.surface) {
            settings.surface = Self::default().surface;
        }
        settings
    }
    pub fn apply(self, window: &Window) {
        window.set_background_appearance(if self.glass {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        });
    }
    pub fn fill(self, rgb: u32) -> gpui::Rgba {
        let mut tinted = 0;
        for shift in [16, 8, 0] {
            let channel = (((rgb >> shift) & 255) + 3 * ((self.surface >> shift) & 255)) / 4;
            tinted |= channel << shift;
        }
        gpui::rgba((tinted << 8) | if self.glass { self.tint as u32 } else { 255 })
    }
    pub fn apply_theme(self, cx: &mut gpui::App) {
        let colors = &mut gpui_component::Theme::global_mut(cx).colors;
        colors.background = gpui::rgba(0x00000000).into();
        colors.primary = gpui::rgb(self.accent).into();
        colors.primary_hover = colors.primary;
        colors.primary_active = colors.primary;
        colors.ring = colors.primary;
        colors.primary_foreground = gpui::rgb(if dark_surface(self.accent) {
            0xffffff
        } else {
            0x101010
        })
        .into();
        colors.selection = gpui::rgba((self.accent << 8) | 70).into();
    }
}

pub fn parse_color(value: &str) -> Result<u32, String> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if value.len() != 6 || !value.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Use a six-digit hex color, such as #183044.".into());
    }
    u32::from_str_radix(value, 16).map_err(|_| "Invalid color.".into())
}
pub fn dark_surface(color: u32) -> bool {
    let channel = |shift: u32| {
        let value = ((color >> shift) & 255u32) as f64 / 255.;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0) < 0.18
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_defaults_and_partial_settings_are_valid() {
        let settings: Appearance = serde_json::from_str(r#"{"glass":false}"#).unwrap();
        assert!(!settings.glass);
        assert_eq!(settings.tint, 110);
        let saved = serde_json::to_vec(&Appearance::default()).unwrap();
        assert!(serde_json::from_slice::<Appearance>(&saved).unwrap().glass);
    }
    #[test]
    fn custom_colors_validate_and_old_settings_still_load() {
        assert_eq!(parse_color(" #18aBcD ").unwrap(), 0x18abcd);
        for invalid in ["red", "#abc", "#12345678", "#abcdef<script>"] {
            assert!(parse_color(invalid).is_err());
        }
        assert!(dark_surface(0x183044));
        assert!(!dark_surface(0xffffff));
        let previous: Appearance = serde_json::from_str(r#"{"glass":true,"tint":160}"#).unwrap();
        assert_eq!(previous.surface, Appearance::default().surface);
        let solid = Appearance {
            glass: false,
            ..Default::default()
        };
        assert_eq!(solid.fill(0).a, 1.);
    }
}
