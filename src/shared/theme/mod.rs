//! Design tokens resolved from [`ThemeMode`].
//!
//! Monochrome palette ported from the reference onboarding implementation.

use gpui::Hsla;
use serde::{Deserialize, Serialize};

/// User-facing theme selection persisted in [`crate::shared::preferences::AppPreferences`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
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

impl Default for ThemeMode {
    fn default() -> Self {
        Self::Dark
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

/// A single color token (hex string) for shell compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorToken(pub &'static str);

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
    pub background: ColorToken,
    pub foreground: ColorToken,
    pub border: ColorToken,
    pub accent: ColorToken,
    pub spacing: SpacingScale,
    pub display_title: LegacyTypeRole,
    pub tagline: LegacyTypeRole,
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
}

const SPACING: SpacingScale = SpacingScale {
    xs: 4,
    sm: 8,
    md: 16,
    lg: 24,
    xl: 32,
};

const DISPLAY_TITLE: LegacyTypeRole = LegacyTypeRole {
    size_px: 32,
    weight: 700,
};

const TAGLINE: LegacyTypeRole = LegacyTypeRole {
    size_px: 12,
    weight: 400,
};

const LABEL: LegacyTypeRole = LegacyTypeRole {
    size_px: 14,
    weight: 500,
};

const fn theme_with_mode(mode: ThemeMode) -> OpenCoreTheme {
    let (background, foreground, border, accent) = match mode {
        ThemeMode::Light => ("#FAFAFA", "#0A0A0A", "#E6E6E6", "#171717"),
        ThemeMode::Dark => ("#000000", "#FAFAFA", "#262626", "#E6E6E6"),
    };
    OpenCoreTheme {
        mode,
        background: ColorToken(background),
        foreground: ColorToken(foreground),
        border: ColorToken(border),
        accent: ColorToken(accent),
        spacing: SPACING,
        display_title: DISPLAY_TITLE,
        tagline: TAGLINE,
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
        assert_eq!(theme.background.0, "#000000");
        assert_eq!(theme.foreground.0, "#FAFAFA");
    }

    #[test]
    fn light_theme_uses_light_background() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Light);
        assert_eq!(theme.background.0, "#FAFAFA");
        assert_eq!(theme.foreground.0, "#0A0A0A");
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
        assert_eq!(light.display_title, dark.display_title);
    }
}
