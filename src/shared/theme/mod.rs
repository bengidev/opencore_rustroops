//! Design tokens resolved from [`ThemeMode`].
//!
//! Monochrome palette ported from the reference onboarding implementation.

use gpui::Hsla;
use serde::{Deserialize, Serialize};

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
}

/// Resolved design tokens for the active [`ThemeMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenCoreTheme {
    pub mode: ThemeMode,
    pub spacing: SpacingScale,
    pub label: LegacyTypeRole,
}

impl OpenCoreTheme {
    pub fn resolve(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => LIGHT_THEME,
            ThemeMode::Dark => DARK_THEME,
        }
    }

    pub fn foreground(&self, token: ForegroundToken) -> Hsla {
        rgba_to_hsla(match self.mode {
            ThemeMode::Light => match token {
                ForegroundToken::Primary => rgbf(0.04, 0.04, 0.04),
                ForegroundToken::Secondary => rgbf(0.32, 0.32, 0.32),
                ForegroundToken::Muted => rgbf(0.64, 0.64, 0.64),
                ForegroundToken::Accent => rgbf(0.09, 0.09, 0.09),
            },
            ThemeMode::Dark => match token {
                ForegroundToken::Primary => rgbf(0.98, 0.98, 0.98),
                ForegroundToken::Secondary => rgbf(0.64, 0.64, 0.64),
                ForegroundToken::Muted => rgbf(0.45, 0.45, 0.45),
                ForegroundToken::Accent => rgbf(0.90, 0.90, 0.90),
            },
        })
    }

    pub fn surface(&self, token: BackgroundToken) -> Hsla {
        rgba_to_hsla(match self.mode {
            ThemeMode::Light => match token {
                BackgroundToken::Primary => rgbf(0.98, 0.98, 0.98),
                BackgroundToken::Secondary => rgbf(0.96, 0.96, 0.96),
                BackgroundToken::Tertiary => rgbf(0.94, 0.94, 0.94),
            },
            ThemeMode::Dark => match token {
                BackgroundToken::Primary => rgbf(0.0, 0.0, 0.0),
                BackgroundToken::Secondary => rgbf(0.04, 0.04, 0.04),
                BackgroundToken::Tertiary => rgbf(0.10, 0.10, 0.10),
            },
        })
    }

    pub fn border_token(&self, token: BorderToken) -> Hsla {
        rgba_to_hsla(match self.mode {
            ThemeMode::Light => match token {
                BorderToken::Default => rgbf(0.90, 0.90, 0.90),
                BorderToken::Strong => rgbf(0.83, 0.83, 0.83),
            },
            ThemeMode::Dark => match token {
                BorderToken::Default => rgbf(0.15, 0.15, 0.15),
                BorderToken::Strong => rgbf(0.25, 0.25, 0.25),
            },
        })
    }

    pub fn action(&self, token: ActionToken) -> Hsla {
        rgba_to_hsla(match self.mode {
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

    pub fn control_radius(&self) -> f32 {
        8.0
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
};

const LABEL: LegacyTypeRole = LegacyTypeRole {
    size_px: 14,
    weight: 500,
};

const fn theme_with_mode(mode: ThemeMode) -> OpenCoreTheme {
    OpenCoreTheme {
        mode,
        spacing: SPACING,
        label: LABEL,
    }
}

const LIGHT_THEME: OpenCoreTheme = theme_with_mode(ThemeMode::Light);
const DARK_THEME: OpenCoreTheme = theme_with_mode(ThemeMode::Dark);

fn rgbf(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (r, g, b)
}

fn rgba_to_hsla((r, g, b): (f32, f32, f32)) -> Hsla {
    let ri = (r * 255.0) as u32;
    let gi = (g * 255.0) as u32;
    let bi = (b * 255.0) as u32;
    gpui::rgb((ri << 16) | (gi << 8) | bi).into()
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
    fn light_theme_uses_light_background() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Light);
        let bg = theme.surface(BackgroundToken::Primary);
        assert_eq!(bg, rgba_to_hsla(rgbf(0.98, 0.98, 0.98)));
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
}
