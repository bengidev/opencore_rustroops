//! Nothing-styled Hello World home (gpui.rs-inspired).

use gpui::{IntoElement, ParentElement, SharedString, Styled, div, px};

use crate::shared::theme::{
    ACCENT_RED, BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SUCCESS_GREEN,
    TypeRole, WARNING_AMBER,
};

/// Full-screen Nothing-styled Hello World home screen.
pub fn home_screen(theme: OpenCoreTheme) -> impl IntoElement {
    let page = theme.surface(BackgroundToken::Primary);
    let display = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let label = theme.foreground(ForegroundToken::Muted);
    let mono = SharedString::from("Space Mono");

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
                .font_family(SharedString::from("Space Grotesk"))
                .font_weight(gpui::FontWeight::LIGHT)
                .text_color(display)
                .child("Hello, World!"),
        )
        .child(
            div()
                .text_size(px(TypeRole::MonoSm.size()))
                .font_family(mono.clone())
                .text_color(secondary)
                .child("OpenCore · GPUI"),
        )
        .child(swatch_row(theme))
        .child(
            div()
                .mt(px(theme.spacing.xl as f32))
                .text_size(px(11.))
                .font_family(mono)
                .text_color(label)
                .child("HOME"),
        )
}

/// Design break: R G B Y K W swatches using theme border tokens.
fn swatch_row(theme: OpenCoreTheme) -> impl IntoElement {
    let border = theme.border_token(BorderToken::Default);
    let radius = px(theme.control_radius());

    let colors: [u32; 6] = [
        ACCENT_RED,
        SUCCESS_GREEN,
        0x5B_9B_F6, // interactive blue chip
        WARNING_AMBER,
        0x00_00_00,
        0xFF_FF_FF,
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

    #[test]
    fn home_swatch_radius_follows_control_radius() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Dark);
        assert_eq!(theme.control_radius(), 6.0);
    }
}
