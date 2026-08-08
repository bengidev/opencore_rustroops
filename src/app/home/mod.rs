//! Nothing-styled Hello World home (gpui.rs-inspired).

use gpui::{IntoElement, ParentElement, Styled, div, px};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, TypeRole,
};

/// Full-screen Nothing-styled Hello World home screen.
pub fn home_screen(theme: OpenCoreTheme) -> impl IntoElement {
    let page = theme.surface(BackgroundToken::Primary);
    let display = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let label = theme.foreground(ForegroundToken::Muted);

    div()
        .size_full()
        .bg(page)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(theme.spacing.md as f32))
        .child(
            div()
                .text_size(px(48.))
                .font_weight(gpui::FontWeight::LIGHT)
                .text_color(display)
                .child("Hello, World!"),
        )
        .child(
            div()
                .text_size(px(TypeRole::MonoSm.size()))
                .text_color(secondary)
                .child("OpenCore · GPUI"),
        )
        .child(swatch_row(theme))
        .child(
            div()
                .mt(px(theme.spacing.xl as f32))
                .text_size(px(11.))
                .text_color(label)
                .child("HOME"),
        )
}

/// Design break: R G B Y K W swatches using theme border tokens.
fn swatch_row(theme: OpenCoreTheme) -> impl IntoElement {
    let border = theme.border_token(BorderToken::Default);
    let radius = px(theme.control_radius().min(8.0));

    // R G B Y K W
    let colors: [u32; 6] = [
        0xD7_19_21, // R (Nothing accent red as red chip)
        0x4A_9E_5C, // G
        0x5B_9B_F6, // B
        0xD4_A8_43, // Y
        0x00_00_00, // K
        0xFF_FF_FF, // W
    ];
    let mut row = div().flex().gap(px(8.));
    for c in colors {
        row = row.child(
            div()
                .size(px(32.))
                .rounded(radius)
                .border_1()
                .border_color(border)
                .bg(gpui::rgb(c)),
        );
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::theme::{BackgroundToken, OpenCoreTheme, ThemeMode};

    #[test]
    fn home_screen_builds_for_both_themes() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let _ = home_screen(OpenCoreTheme::resolve(mode));
            let bg = OpenCoreTheme::resolve(mode).surface(BackgroundToken::Primary);
            let _ = bg; // theme resolves; building element must not panic
        }
    }
}
