//! OpenCore brand lockup image.

use gpui::{IntoElement, Styled, img, px};

use crate::shared::theme::{OpenCoreTheme, ThemeMode};

pub const BRAND_IMAGE: &str = "images/opencore-brand.png";
pub const BRAND_IMAGE_INVERSE: &str = "images/opencore-brand-inverse.png";
/// Cropped asset dimensions: 848 × 202.
pub const BRAND_ASPECT: f32 = 848.0 / 202.0;

pub fn brand_width(height: f32) -> f32 {
    height * BRAND_ASPECT
}

pub(crate) fn brand_image_for(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Light => BRAND_IMAGE,
        ThemeMode::Dark => BRAND_IMAGE_INVERSE,
    }
}

pub fn opencore_brand_image(theme: OpenCoreTheme, height: f32, opacity: f32) -> impl IntoElement {
    img(brand_image_for(theme.mode))
        .h(px(height))
        .w(px(brand_width(height)))
        .opacity(opacity.clamp(0.0, 1.0))
        .flex_shrink_0()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_image_for_selects_theme_variant() {
        assert_eq!(brand_image_for(ThemeMode::Light), BRAND_IMAGE);
        assert_eq!(brand_image_for(ThemeMode::Dark), BRAND_IMAGE_INVERSE);
    }
}
