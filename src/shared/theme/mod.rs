//! Design tokens resolved from [`ThemeMode`].
//!
//! Monochrome palette for welcome and shell surfaces.

mod theme_nothing_gpui;
mod theme_transition;

use gpui::Hsla;
use serde::{Deserialize, Serialize};

pub use theme_nothing_gpui::apply_nothing_theme;
pub use theme_transition::{
    THEME_TRANSITION_DURATION, ThemeTransition, ease_out_resize, ease_out_strong, mix_light_for,
};

pub const ACCENT_RED: u32 = 0xD7_19_21;
pub const SUCCESS_GREEN: u32 = 0x4A_9E_5C;
pub const WARNING_AMBER: u32 = 0xD4_A8_43;

/// User-facing theme selection persisted in [`crate::shared::preferences::AppPreferences`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

impl ThemeMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundToken {
    Primary,
    Secondary,
    Tertiary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForegroundToken {
    Primary,
    Secondary,
    Muted,
    Accent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderToken {
    Default,
    Strong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionToken {
    Strong,
    StrongText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacingToken {
    S1,
    S3,
    S4,
}

impl SpacingToken {
    pub fn value(self) -> f32 {
        match self {
            Self::S1 => 4.0,
            Self::S3 => 12.0,
            Self::S4 => 16.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRole {
    DisplayMd,
    LabelMd,
    MonoSm,
}

impl TypeRole {
    pub fn size(self) -> f32 {
        match self {
            Self::DisplayMd => 32.0,
            Self::LabelMd => 13.0,
            Self::MonoSm => 12.0,
        }
    }

    pub fn line_height(self) -> f32 {
        match self {
            Self::DisplayMd => 1.12,
            Self::LabelMd => 1.15,
            Self::MonoSm => 1.20,
        }
    }
}

/// Linear RGBA components in `[0.0, 1.0]` for canvas drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ThemeRgba {
    pub fn from_hsla(h: Hsla) -> Self {
        let c = h.to_rgb();
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }
}

/// Typography role used across shell zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyTypeRole {
    pub size_px: u16,
    pub weight: u16,
}

/// Spacing scale in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpacingScale {
    pub xs: u16,
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
    pub xl: u16,
    pub xxl: u16,
    pub xxxl: u16,
}

/// Resolved design tokens for the active [`ThemeMode`].
///
/// Color methods blend dark (`mix_light = 0`) and light (`mix_light = 1`) palettes
/// during a theme transition. [`Self::mode`] is the target mode (icons/labels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenCoreTheme {
    pub mode: ThemeMode,
    pub spacing: SpacingScale,
    pub label: LegacyTypeRole,
    mix_light: f32,
}

impl OpenCoreTheme {
    pub fn resolve(mode: ThemeMode) -> Self {
        Self::blended(mode, mix_light_for(mode))
    }

    /// Target `mode` for chrome; `mix_light` for interpolated colors.
    pub fn blended(mode: ThemeMode, mix_light: f32) -> Self {
        OpenCoreTheme {
            mode,
            spacing: SPACING,
            label: LABEL,
            mix_light: mix_light.clamp(0.0, 1.0),
        }
    }

    pub fn foreground(&self, token: ForegroundToken) -> Hsla {
        lerp_hsla(
            foreground_hsla(ThemeMode::Dark, token),
            foreground_hsla(ThemeMode::Light, token),
            self.mix_light,
        )
    }

    pub fn surface(&self, token: BackgroundToken) -> Hsla {
        lerp_hsla(
            surface_hsla(ThemeMode::Dark, token),
            surface_hsla(ThemeMode::Light, token),
            self.mix_light,
        )
    }

    pub fn border_token(&self, token: BorderToken) -> Hsla {
        lerp_hsla(
            border_hsla(ThemeMode::Dark, token),
            border_hsla(ThemeMode::Light, token),
            self.mix_light,
        )
    }

    pub fn action(&self, token: ActionToken) -> Hsla {
        lerp_hsla(
            action_hsla(ThemeMode::Dark, token),
            action_hsla(ThemeMode::Light, token),
            self.mix_light,
        )
    }

    pub fn control_radius(&self) -> f32 {
        0.0
    }

    pub fn rgba_foreground(&self, token: ForegroundToken) -> ThemeRgba {
        ThemeRgba::from_hsla(self.foreground(token))
    }

    pub fn rgba_surface(&self, token: BackgroundToken) -> ThemeRgba {
        ThemeRgba::from_hsla(self.surface(token))
    }

    pub fn rgba_border(&self, token: BorderToken) -> ThemeRgba {
        ThemeRgba::from_hsla(self.border_token(token))
    }

    pub fn rgba_action(&self, token: ActionToken) -> ThemeRgba {
        ThemeRgba::from_hsla(self.action(token))
    }
}

const SPACING: SpacingScale = SpacingScale {
    xs: 4,
    sm: 8,
    md: 16,
    lg: 24,
    xl: 32,
    xxl: 48,
    xxxl: 64,
};

const LABEL: LegacyTypeRole = LegacyTypeRole {
    size_px: 14,
    weight: 500,
};

fn foreground_hsla(mode: ThemeMode, token: ForegroundToken) -> Hsla {
    rgba_to_hsla(match mode {
        ThemeMode::Light => match token {
            ForegroundToken::Primary => rgbf(26.0 / 255.0, 26.0 / 255.0, 26.0 / 255.0),
            ForegroundToken::Secondary => rgbf(102.0 / 255.0, 102.0 / 255.0, 102.0 / 255.0),
            ForegroundToken::Muted => rgbf(153.0 / 255.0, 153.0 / 255.0, 153.0 / 255.0),
            ForegroundToken::Accent => rgbf(0.09, 0.09, 0.09),
        },
        ThemeMode::Dark => match token {
            ForegroundToken::Primary => rgbf(232.0 / 255.0, 232.0 / 255.0, 232.0 / 255.0),
            ForegroundToken::Secondary => rgbf(153.0 / 255.0, 153.0 / 255.0, 153.0 / 255.0),
            ForegroundToken::Muted => rgbf(102.0 / 255.0, 102.0 / 255.0, 102.0 / 255.0),
            ForegroundToken::Accent => rgbf(0.90, 0.90, 0.90),
        },
    })
}

fn surface_hsla(mode: ThemeMode, token: BackgroundToken) -> Hsla {
    rgba_to_hsla(match mode {
        ThemeMode::Light => match token {
            BackgroundToken::Primary => rgbf(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0),
            BackgroundToken::Secondary => rgbf(1.0, 1.0, 1.0),
            BackgroundToken::Tertiary => rgbf(240.0 / 255.0, 240.0 / 255.0, 240.0 / 255.0),
        },
        ThemeMode::Dark => match token {
            BackgroundToken::Primary => rgbf(0.0, 0.0, 0.0),
            BackgroundToken::Secondary => rgbf(17.0 / 255.0, 17.0 / 255.0, 17.0 / 255.0),
            BackgroundToken::Tertiary => rgbf(26.0 / 255.0, 26.0 / 255.0, 26.0 / 255.0),
        },
    })
}

fn border_hsla(mode: ThemeMode, token: BorderToken) -> Hsla {
    rgba_to_hsla(match mode {
        ThemeMode::Light => match token {
            BorderToken::Default => rgbf(232.0 / 255.0, 232.0 / 255.0, 232.0 / 255.0),
            BorderToken::Strong => rgbf(204.0 / 255.0, 204.0 / 255.0, 204.0 / 255.0),
        },
        ThemeMode::Dark => match token {
            BorderToken::Default => rgbf(34.0 / 255.0, 34.0 / 255.0, 34.0 / 255.0),
            BorderToken::Strong => rgbf(51.0 / 255.0, 51.0 / 255.0, 51.0 / 255.0),
        },
    })
}

fn action_hsla(mode: ThemeMode, token: ActionToken) -> Hsla {
    rgba_to_hsla(match mode {
        ThemeMode::Light => match token {
            ActionToken::Strong => rgbf(0.04, 0.04, 0.04),
            ActionToken::StrongText => rgbf(0.98, 0.98, 0.98),
        },
        ThemeMode::Dark => match token {
            ActionToken::Strong => rgbf(0.98, 0.98, 0.98),
            ActionToken::StrongText => rgbf(0.04, 0.04, 0.04),
        },
    })
}

fn rgbf(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (r, g, b)
}

fn rgba_to_hsla((r, g, b): (f32, f32, f32)) -> Hsla {
    let ri = (r * 255.0) as u32;
    let gi = (g * 255.0) as u32;
    let bi = (b * 255.0) as u32;
    gpui::rgb((ri << 16) | (gi << 8) | bi).into()
}

fn lerp_hsla(a: Hsla, b: Hsla, t: f32) -> Hsla {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    Hsla {
        h: a.h + (b.h - a.h) * t,
        s: a.s + (b.s - a.s) * t,
        l: a.l + (b.l - a.l) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_uses_dark_background() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Dark);
        let bg = theme.surface(BackgroundToken::Primary);
        assert_eq!(bg, rgba_to_hsla(rgbf(0.0, 0.0, 0.0)));
    }

    #[test]
    fn nothing_dark_page_is_oled_black() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Dark);
        assert_eq!(
            theme.surface(BackgroundToken::Primary),
            rgba_to_hsla(rgbf(0.0, 0.0, 0.0))
        );
    }

    #[test]
    fn nothing_light_page_is_off_white() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Light);
        // #F5F5F5
        assert_eq!(
            theme.surface(BackgroundToken::Primary),
            rgba_to_hsla(rgbf(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0))
        );
    }

    #[test]
    fn nothing_dark_text_primary_near_e8() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Dark);
        assert_eq!(
            theme.foreground(ForegroundToken::Primary),
            rgba_to_hsla(rgbf(232.0 / 255.0, 232.0 / 255.0, 232.0 / 255.0))
        );
    }

    #[test]
    fn light_theme_uses_light_background() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Light);
        let bg = theme.surface(BackgroundToken::Primary);
        assert_eq!(
            bg,
            rgba_to_hsla(rgbf(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0))
        );
    }

    #[test]
    fn theme_mode_toggles_between_light_and_dark() {
        assert_eq!(ThemeMode::Dark.toggle(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.toggle(), ThemeMode::Dark);
    }

    #[test]
    fn default_theme_mode_is_dark() {
        assert_eq!(ThemeMode::default(), ThemeMode::Dark);
    }

    #[test]
    fn light_and_dark_share_layout_tokens() {
        let light = OpenCoreTheme::resolve(ThemeMode::Light);
        let dark = OpenCoreTheme::resolve(ThemeMode::Dark);
        assert_eq!(light.spacing, dark.spacing);
        assert_eq!(light.label, dark.label);
    }

    #[test]
    fn control_radius_is_zero_px() {
        let dark = OpenCoreTheme::resolve(ThemeMode::Dark);
        let light = OpenCoreTheme::resolve(ThemeMode::Light);
        assert_eq!(dark.control_radius(), 0.0);
        assert_eq!(light.control_radius(), 0.0);
    }

    #[test]
    fn nothing_status_constants_match_spec() {
        assert_eq!(ACCENT_RED, 0xD7_19_21);
        assert_eq!(SUCCESS_GREEN, 0x4A_9E_5C);
        assert_eq!(WARNING_AMBER, 0xD4_A8_43);
    }

    #[test]
    fn blended_endpoints_match_resolved_palettes() {
        let dark = OpenCoreTheme::resolve(ThemeMode::Dark);
        let light = OpenCoreTheme::resolve(ThemeMode::Light);
        assert_eq!(
            OpenCoreTheme::blended(ThemeMode::Light, 0.0).surface(BackgroundToken::Primary),
            dark.surface(BackgroundToken::Primary)
        );
        assert_eq!(
            OpenCoreTheme::blended(ThemeMode::Dark, 1.0).surface(BackgroundToken::Primary),
            light.surface(BackgroundToken::Primary)
        );
    }

    #[test]
    fn blended_midpoint_sits_between_palettes() {
        let dark = OpenCoreTheme::resolve(ThemeMode::Dark).surface(BackgroundToken::Primary);
        let light = OpenCoreTheme::resolve(ThemeMode::Light).surface(BackgroundToken::Primary);
        let mid = OpenCoreTheme::blended(ThemeMode::Light, 0.5).surface(BackgroundToken::Primary);
        assert!(mid.l > dark.l && mid.l < light.l);
    }
}
