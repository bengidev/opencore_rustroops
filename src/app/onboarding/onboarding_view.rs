#![allow(clippy::redundant_clone)]
//! Onboarding view — immersive monochrome landing ported to GPUI.

use gpui::{
    FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, SharedString,
    Styled, div, px, relative,
};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::app::gpui_callbacks::WindowAppHandler;
use crate::shared::theme::{
    BackgroundToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::onboarding_theme_toggle::theme_toggle_button;
use super::onboarding_ui_state::OnboardingUiState;

const HERO_MAX_WIDTH: f32 = 600.0;
const ASCII_HERO_HEIGHT: f32 = 300.0;
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
        .child(hero_block(theme, ui.last_frame()))
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

fn hero_block(theme: OpenCoreTheme, frame: &str) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let grotesk = SharedString::from("Space Grotesk");
    let spacing = theme.spacing;

    let mut ascii_col = div()
        .w_full()
        .h(px(ASCII_HERO_HEIGHT))
        .flex()
        .flex_col()
        .items_center()
        .justify_center();
    for line in frame.lines() {
        ascii_col = ascii_col.child(
            div()
                .font_family(SharedString::from("Space Mono"))
                .text_size(px(9.))
                .text_color(primary)
                .child(line.to_string()),
        );
    }

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
                .child(ascii_col)
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
}
