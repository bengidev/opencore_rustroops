#![allow(clippy::redundant_clone)]
//! Onboarding view — immersive monochrome landing ported to GPUI.

use gpui::{
    BoxShadow, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    SharedString, Styled, div, px, relative,
};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::app::gpui_callbacks::WindowAppHandler;
use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::onboarding_theme_toggle::theme_toggle_button;
use super::onboarding_ui_state::OnboardingUiState;

const HERO_MAX_WIDTH: f32 = 680.0;
const ASCII_HERO_HEIGHT: f32 = 360.0;
const HERO_GLOW_INSET_H: f32 = 44.0;
const HERO_GLOW_INSET_TOP: f32 = 46.0;
const HERO_GLOW_INSET_BOTTOM: f32 = 34.0;
const ASCII_TEXT_SIZE: f32 = 9.0;
const ASCII_BOX_SIZE: f32 = 320.0;
const EDGE_INSET_H: f32 = 16.0;
const EDGE_INSET_V: f32 = 20.0;

#[derive(Clone)]
pub struct OnboardingCallbacks {
    pub on_enter: WindowAppHandler,
    pub on_toggle_theme: WindowAppHandler,
}

/// Focusable shell for onboarding keyboard input (Enter to complete).
pub fn onboarding_interactive_root(
    focus_handle: &FocusHandle,
    on_enter: WindowAppHandler,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .size_full()
        .tab_index(0)
        .track_focus(focus_handle)
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if is_enter_keystroke(event) {
                on_enter(window, cx);
            }
        })
        .child(content)
}

/// Full-screen onboarding scene matching the reference layout.
pub fn onboarding_screen(
    theme: OpenCoreTheme,
    ui: &OnboardingUiState,
    callbacks: OnboardingCallbacks,
    persistence_error: Option<&str>,
) -> impl IntoElement {
    let background = theme.surface(BackgroundToken::Primary);

    div()
        .size_full()
        .bg(background)
        .child(main_column(theme, ui, callbacks, persistence_error))
}

fn is_enter_keystroke(event: &KeyDownEvent) -> bool {
    let key = event.keystroke.key.as_str();
    matches!(key, "enter" | "return") && !event.is_held && !event.keystroke.modifiers.modified()
}

fn main_column(
    theme: OpenCoreTheme,
    ui: &OnboardingUiState,
    callbacks: OnboardingCallbacks,
    persistence_error: Option<&str>,
) -> impl IntoElement {
    let mut column = div()
        .size_full()
        .flex()
        .flex_col()
        .p(px(EDGE_INSET_V))
        .px(px(EDGE_INSET_H))
        .child(header_row(theme, callbacks.clone()))
        .child(div().h(px(SpacingToken::S4.value())))
        .child(hero_block(theme, ui))
        .child(div().flex_grow(1.));

    if let Some(message) = persistence_error {
        let muted = theme.foreground(ForegroundToken::Muted);
        let mono = SharedString::from("Space Mono");
        let message = SharedString::from(message);
        column = column.child(
            div()
                .w_full()
                .text_center()
                .text_size(px(TypeRole::MonoSm.size()))
                .font_family(mono)
                .text_color(muted)
                .pb(px(SpacingToken::S1.value()))
                .child(message),
        );
    }

    column.child(action_row(theme, callbacks))
}

fn header_row(theme: OpenCoreTheme, callbacks: OnboardingCallbacks) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = SharedString::from("Space Mono");
    let grotesk = SharedString::from("Space Grotesk");

    div()
        .w_full()
        .flex()
        .items_center()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(TypeRole::LabelMd.size()))
                        .font_family(grotesk)
                        .text_color(primary)
                        .child("OpenCore"),
                )
                .child(
                    div()
                        .text_size(px(TypeRole::MonoSm.size()))
                        .font_family(mono)
                        .text_color(muted)
                        .child("LOCAL AI WORKSPACE"),
                ),
        )
        .child(div().flex_grow(1.))
        .child(theme_toggle_button(theme, callbacks.on_toggle_theme))
}

fn hero_glow(theme: OpenCoreTheme) -> impl IntoElement {
    let accent = theme.foreground(ForegroundToken::Accent);

    div()
        .absolute()
        .left(px(HERO_GLOW_INSET_H))
        .right(px(HERO_GLOW_INSET_H))
        .top(px(HERO_GLOW_INSET_TOP))
        .bottom(px(HERO_GLOW_INSET_BOTTOM))
        .rounded(px(160.))
        .shadow(vec![
            BoxShadow::new(px(0.), px(0.), accent.opacity(0.16)).blur_radius(px(72.)),
            BoxShadow::new(px(0.), px(0.), accent.opacity(0.08)).blur_radius(px(144.)),
        ])
}

fn hero_block(theme: OpenCoreTheme, ui: &OnboardingUiState) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let ascii_color = theme.foreground(ForegroundToken::Primary);
    let grotesk = SharedString::from("Space Grotesk");
    let spacing = theme.spacing;

    let hero_ascii = div()
        .relative()
        .w_full()
        .h(px(ASCII_HERO_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .child(hero_glow(theme))
        .child(ascii_box(theme, ui.last_frame(), ascii_color));

    div()
        .w_full()
        .flex()
        .justify_center()
        .child(
            div()
                .w(px(HERO_MAX_WIDTH))
                .flex()
                .flex_col()
                .items_center()
                .child(hero_ascii)
                .child(div().h(px(spacing.lg as f32)))
                .child(
                    div()
                        .w_full()
                        .text_center()
                        .text_size(px(TypeRole::DisplayMd.size()))
                        .font_family(grotesk.clone())
                        .text_color(primary)
                        .child("Your local AI command workspace"),
                )
                .child(div().h(px(spacing.sm as f32)))
                .child(
                    div()
                        .w_full()
                        .max_w(px(HERO_MAX_WIDTH))
                        .text_center()
                        .text_size(px(TypeRole::MonoSm.size()))
                        .line_height(relative(TypeRole::MonoSm.line_height()))
                        .font_family(grotesk)
                        .text_color(secondary)
                        .child("OpenCore combines chat, terminal, editing, and Rust-native performance in one permissioned desktop environment. To leave the crowded cloud, polluted by leaks and unconsciousness, to return to a workspace that stays on your machine."),
                ),
        )
}

fn ascii_box(theme: OpenCoreTheme, frame: &str, text_color: gpui::Hsla) -> impl IntoElement {
    let border = theme.border_token(BorderToken::Strong).opacity(0.40);
    let surface = theme.surface(BackgroundToken::Secondary).opacity(0.50);

    let mut ascii = div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .overflow_hidden();

    for line in frame.lines() {
        ascii = ascii.child(
            div()
                .font_family(mono_family())
                .text_size(px(ASCII_TEXT_SIZE))
                .line_height(relative(1.0))
                .text_color(text_color)
                .child(line.to_string()),
        );
    }

    div()
        .relative()
        .w(px(ASCII_BOX_SIZE))
        .h(px(ASCII_BOX_SIZE))
        .child(ascii)
        .rounded(px(8.))
        .border_1()
        .border_color(border)
        .bg(surface)
        .p(px(6.))
        .overflow_hidden()
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}

fn action_row(theme: OpenCoreTheme, callbacks: OnboardingCallbacks) -> impl IntoElement {
    let spacing = theme.spacing;
    let on_enter = callbacks.on_enter;
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .pb(px(spacing.sm as f32))
        .child(
            Button::new("enter-opencore")
                .primary()
                .label("Enter OpenCore")
                .on_click(move |_, window, cx| {
                    on_enter(window, cx);
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::super::ascii_galaxy::{COLS, ROWS};
    use super::*;
    use gpui::Keystroke;

    fn enter_key_event(is_held: bool) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: Keystroke::parse("enter").expect("enter keystroke"),
            is_held,
            prefer_character_input: false,
        }
    }

    #[test]
    fn enter_keystroke_ignores_key_autorepeat() {
        assert!(is_enter_keystroke(&enter_key_event(false)));
        assert!(!is_enter_keystroke(&enter_key_event(true)));
    }

    #[test]
    fn ascii_hero_layout_constants() {
        assert_eq!(HERO_MAX_WIDTH, 680.0);
        assert_eq!(ASCII_HERO_HEIGHT, 360.0);
        assert_eq!(HERO_GLOW_INSET_H, 44.0);
        assert_eq!(HERO_GLOW_INSET_TOP, 46.0);
        assert_eq!(HERO_GLOW_INSET_BOTTOM, 34.0);
        assert_eq!(ASCII_TEXT_SIZE, 9.0);
        assert_eq!(ASCII_BOX_SIZE, 320.0);
        assert_eq!(COLS, 74);
        assert_eq!(ROWS, 44);
    }
}
