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

const LIGHT_THEME: OpenCoreTheme = OpenCoreTheme {
    background: ColorToken("#F5F5F7"),
    foreground: ColorToken("#1D1D1F"),
    border: ColorToken("#D2D2D7"),
    accent: ColorToken("#0071E3"),
    spacing: SpacingScale {
        xs: 4,
        sm: 8,
        md: 16,
        lg: 24,
        xl: 32,
    },
    display_title: TypeRole {
        size_px: 32,
        weight: 700,
    },
    tagline: TypeRole {
        size_px: 18,
        weight: 400,
    },
    label: TypeRole {
        size_px: 14,
        weight: 500,
    },
};

const DARK_THEME: OpenCoreTheme = OpenCoreTheme {
    background: ColorToken("#1C1C1E"),
    foreground: ColorToken("#F5F5F7"),
    border: ColorToken("#3A3A3C"),
    accent: ColorToken("#0A84FF"),
    spacing: SpacingScale {
        xs: 4,
        sm: 8,
        md: 16,
        lg: 24,
        xl: 32,
    },
    display_title: TypeRole {
        size_px: 32,
        weight: 700,
    },
    tagline: TypeRole {
        size_px: 18,
        weight: 400,
    },
    label: TypeRole {
        size_px: 14,
        weight: 500,
    },
};

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
}
