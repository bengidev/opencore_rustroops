//! Design tokens resolved from [`ThemeMode`].
//!
//! Light and dark palettes are defined from day one so screens share one visual language
//! before a theme-toggle UI exists.

use serde::{Deserialize, Serialize};

/// User-facing theme selection persisted in [`crate::shared::preferences::AppPreferences`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    /// Returns the opposite mode (for future toggle UI).
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

/// A single color token (hex string).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorToken(pub &'static str);

/// Typography role used across onboarding and shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeRole {
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
    pub background: ColorToken,
    pub foreground: ColorToken,
    pub border: ColorToken,
    pub accent: ColorToken,
    pub spacing: SpacingScale,
    pub display_title: TypeRole,
    pub tagline: TypeRole,
    pub label: TypeRole,
}

impl OpenCoreTheme {
    /// Resolves palette tokens for `mode`.
    pub fn resolve(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => LIGHT_THEME,
            ThemeMode::Dark => DARK_THEME,
        }
    }
}

const SPACING: SpacingScale = SpacingScale {
    xs: 4,
    sm: 8,
    md: 16,
    lg: 24,
    xl: 32,
};

const DISPLAY_TITLE: TypeRole = TypeRole {
    size_px: 32,
    weight: 700,
};

const TAGLINE: TypeRole = TypeRole {
    size_px: 18,
    weight: 400,
};

const LABEL: TypeRole = TypeRole {
    size_px: 14,
    weight: 500,
};

const fn theme_with_colors(
    background: &'static str,
    foreground: &'static str,
    border: &'static str,
    accent: &'static str,
) -> OpenCoreTheme {
    OpenCoreTheme {
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

const LIGHT_THEME: OpenCoreTheme = theme_with_colors("#F5F5F7", "#1D1D1F", "#D2D2D7", "#0071E3");

const DARK_THEME: OpenCoreTheme = theme_with_colors("#1C1C1E", "#F5F5F7", "#3A3A3C", "#0A84FF");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_uses_dark_background() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Dark);
        assert_eq!(theme.background.0, "#1C1C1E");
        assert_eq!(theme.foreground.0, "#F5F5F7");
    }

    #[test]
    fn light_theme_uses_light_background() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Light);
        assert_eq!(theme.background.0, "#F5F5F7");
        assert_eq!(theme.foreground.0, "#1D1D1F");
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
