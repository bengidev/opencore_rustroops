#![allow(clippy::redundant_clone)]
//! Onboarding view — immersive monochrome landing ported to GPUI.

use gpui::{
    BoxShadow, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    ParentElement, SharedString, Styled, Window, WindowControlArea, div, px, relative,
};
use gpui_component::button::{Button, ButtonVariants as _};
use std::cell::Cell;
use std::rc::Rc;

use crate::app::gpui_callbacks::WindowAppHandler;
use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::onboarding_theme_toggle::theme_toggle_button;
use super::onboarding_ui_state::OnboardingUiState;

const HERO_MAX_WIDTH: f32 = 680.0;
const HERO_GLOW_INSET_H: f32 = 44.0;
const HERO_GLOW_INSET_TOP: f32 = 46.0;
const HERO_GLOW_INSET_BOTTOM: f32 = 34.0;
const ASCII_TEXT_SIZE: f32 = 9.0;
const ASCII_BOX_SIZE: f32 = 320.0;
const EDGE_INSET_H: f32 = 16.0;
const EDGE_INSET_V: f32 = 20.0;
const ENTER_BUTTON_HEIGHT: f32 = 48.0;
const HERO_MIN_SIZE: f32 = 220.0;
const TITLEBAR_CONTROLS_INSET: f32 = 68.0;
const TITLEBAR_HEIGHT: f32 = 38.0;

fn onboarding_drag_should_start(pointer_down: bool, pointer_moved: bool) -> bool {
    pointer_down && pointer_moved
}

fn responsive_hero_size(available_width: f32, available_height: f32) -> f32 {
    let width_limit = (available_width - EDGE_INSET_H * 2.0).max(HERO_MIN_SIZE);
    let height_limit = (available_height - 260.0).max(HERO_MIN_SIZE);
    width_limit.min(height_limit).min(ASCII_BOX_SIZE)
}

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
    let drag_pending = Rc::new(Cell::new(false));
    let on_drag_down = {
        let drag_pending = drag_pending.clone();
        move |_: &gpui::MouseDownEvent, _window: &mut Window, _cx: &mut gpui::App| {
            drag_pending.set(true);
        }
    };
    let on_drag_up = {
        let drag_pending = drag_pending.clone();
        move |_: &gpui::MouseUpEvent, _window: &mut Window, _cx: &mut gpui::App| {
            drag_pending.set(false);
        }
    };
    let on_drag_move = {
        let drag_pending = drag_pending.clone();
        move |_: &gpui::MouseMoveEvent, window: &mut Window, _cx: &mut gpui::App| {
            if onboarding_drag_should_start(drag_pending.replace(false), true) {
                window.start_window_move();
            }
        }
    };

    div()
        .relative()
        .size_full()
        .tab_index(0)
        .track_focus(focus_handle)
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if is_enter_keystroke(event) {
                on_enter(window, cx);
            }
        })
        .child(div().size_full().pt(px(TITLEBAR_HEIGHT)).child(content))
        // Keep the drag hitbox above the full-screen content wrapper. The
        // wrapper is padded visually, but still owns the titlebar band for
        // hit-testing unless this strip is the frontmost child.
        .child(
            div()
                .absolute()
                .top_0()
                .left(px(TITLEBAR_CONTROLS_INSET))
                .right_0()
                .h(px(TITLEBAR_HEIGHT))
                .window_control_area(WindowControlArea::Drag)
                .on_mouse_down(MouseButton::Left, on_drag_down)
                .on_mouse_up(MouseButton::Left, on_drag_up)
                .on_mouse_move(on_drag_move),
        )
}

/// Full-screen onboarding scene matching the reference layout.
pub fn onboarding_screen(
    theme: OpenCoreTheme,
    ui: &OnboardingUiState,
    callbacks: OnboardingCallbacks,
    persistence_error: Option<&str>,
    window_size: gpui::Size<gpui::Pixels>,
) -> impl IntoElement {
    let background = theme.surface(BackgroundToken::Primary);

    div().size_full().bg(background).child(main_column(
        theme,
        ui,
        callbacks,
        persistence_error,
        responsive_hero_size(window_size.width.as_f32(), window_size.height.as_f32()),
    ))
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
    hero_size: f32,
) -> impl IntoElement {
    let mut centered_content = div()
        .w_full()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .child(hero_block(theme, ui, hero_size));

    if let Some(message) = persistence_error {
        let muted = theme.foreground(ForegroundToken::Muted);
        let mono = SharedString::from("Space Mono");
        let message = SharedString::from(message);
        centered_content = centered_content.child(
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

    centered_content = centered_content
        .child(div().h(px(60.0)))
        .child(action_row(theme, callbacks.clone()));

    div()
        .size_full()
        .flex()
        .flex_col()
        .p(px(EDGE_INSET_V))
        .px(px(EDGE_INSET_H))
        .child(header_row(theme, callbacks.clone()))
        .child(div().h(px(SpacingToken::S4.value())))
        .child(centered_content)
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

fn hero_block(theme: OpenCoreTheme, ui: &OnboardingUiState, hero_size: f32) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let ascii_color = theme.foreground(ForegroundToken::Primary);
    let grotesk = SharedString::from("Space Grotesk");
    let spacing = theme.spacing;

    let hero_ascii = div()
        .relative()
        .w_full()
        .h(px(hero_size + 40.0))
        .flex()
        .items_center()
        .justify_center()
        .child(hero_glow(theme))
        .child(ascii_box(theme, ui.last_frame(), ascii_color, hero_size));

    div()
        .w_full()
        .flex()
        .justify_center()
        .child(
            div()
                .w_full()
                .max_w(px(HERO_MAX_WIDTH))
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

fn ascii_box(
    theme: OpenCoreTheme,
    frame: &str,
    text_color: gpui::Hsla,
    size: f32,
) -> impl IntoElement {
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
        .w(px(size))
        .h(px(size))
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
                .h(px(ENTER_BUTTON_HEIGHT))
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

    #[test]
    fn onboarding_titlebar_drag_starts_only_after_pointer_moves() {
        assert!(!onboarding_drag_should_start(false, true));
        assert!(!onboarding_drag_should_start(true, false));
        assert!(onboarding_drag_should_start(true, true));
    }

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
    fn responsive_hero_size_fits_narrow_windows() {
        assert_eq!(responsive_hero_size(440.0, 360.0), 220.0);
        assert_eq!(responsive_hero_size(1200.0, 900.0), ASCII_BOX_SIZE);
    }

    #[test]
    fn ascii_hero_layout_constants() {
        assert_eq!(HERO_MAX_WIDTH, 680.0);
        assert_eq!(HERO_GLOW_INSET_H, 44.0);
        assert_eq!(HERO_GLOW_INSET_TOP, 46.0);
        assert_eq!(HERO_GLOW_INSET_BOTTOM, 34.0);
        assert_eq!(ASCII_TEXT_SIZE, 9.0);
        assert_eq!(ASCII_BOX_SIZE, 320.0);
        assert_eq!(ENTER_BUTTON_HEIGHT, 48.0);
        assert_eq!(COLS, 74);
        assert_eq!(ROWS, 44);
    }
}
