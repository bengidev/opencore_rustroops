#![allow(clippy::redundant_clone)]
//! Welcome view — immersive monochrome landing ported to GPUI.

use gpui::{
    BoxShadow, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    ParentElement, SharedString, Styled, Window, WindowControlArea, div, px, relative,
};
use gpui_component::button::{Button, ButtonVariants as _};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use crate::app::gpui_callbacks::WindowAppHandler;
use crate::app::hero::{opencore_brand_image, responsive_brand_height, show_off_brand_height};
use crate::app::viewport::WindowViewport;
use crate::shared::theme::{
    BackgroundToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::theme_toggle::theme_toggle_button;
use super::ui_state::WelcomeUiState;

const HERO_MAX_WIDTH: f32 = 680.0;
const HERO_GLOW_INSET_H: f32 = 44.0;
const HERO_GLOW_INSET_TOP: f32 = 46.0;
const HERO_GLOW_INSET_BOTTOM: f32 = 34.0;
const EDGE_INSET_H: f32 = 16.0;
const EDGE_INSET_TOP: f32 = 4.0;
const EDGE_INSET_BOTTOM: f32 = 20.0;
const ENTER_BUTTON_HEIGHT: f32 = 48.0;
const TITLEBAR_CONTROLS_INSET: f32 = 88.0;
const TITLEBAR_HEIGHT: f32 = 38.0;

fn welcome_drag_should_start(pointer_down: bool, pointer_moved: bool) -> bool {
    pointer_down && pointer_moved
}

#[derive(Clone)]
pub struct WelcomeCallbacks {
    pub on_enter: WindowAppHandler,
    pub on_toggle_theme: WindowAppHandler,
}

/// Focusable shell for welcome keyboard input (Enter to complete).
pub fn welcome_interactive_root(
    focus_handle: &FocusHandle,
    accepts_enter: bool,
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
            if welcome_drag_should_start(drag_pending.replace(false), true) {
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
            if accepts_enter && is_enter_keystroke(event) {
                on_enter(window, cx);
            }
        })
        .child(div().size_full().pt(px(TITLEBAR_HEIGHT)).child(content))
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

/// Full-screen welcome landing scene.
pub fn welcome_screen(
    theme: OpenCoreTheme,
    ui: &WelcomeUiState,
    now: Instant,
    callbacks: WelcomeCallbacks,
    persistence_error: Option<&str>,
    viewport: WindowViewport,
    content_opacity: f32,
) -> impl IntoElement {
    let background = theme.surface(BackgroundToken::Primary);
    let hero_height = responsive_brand_height(viewport);
    let show_off_height = show_off_brand_height(viewport);
    let chrome_opacity = ui.chrome_opacity(now);
    let reveal_progress = ui.reveal_progress(now);
    let chrome_interactive = ui.accepts_enter(now);

    div()
        .size_full()
        .bg(background)
        .child(
            div()
                .size_full()
                .opacity(content_opacity)
                .child(main_column(
                    theme,
                    callbacks,
                    persistence_error,
                    hero_height,
                    show_off_height,
                    chrome_opacity,
                    reveal_progress,
                    chrome_interactive,
                )),
        )
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn is_enter_keystroke(event: &KeyDownEvent) -> bool {
    let key = event.keystroke.key.as_str();
    matches!(key, "enter" | "return") && !event.is_held && !event.keystroke.modifiers.modified()
}

fn main_column(
    theme: OpenCoreTheme,
    callbacks: WelcomeCallbacks,
    persistence_error: Option<&str>,
    hero_height: f32,
    show_off_height: f32,
    chrome_opacity: f32,
    reveal_progress: f32,
    chrome_interactive: bool,
) -> impl IntoElement {
    let brand_height = lerp(show_off_height, hero_height, reveal_progress);

    let mut centered_content = div()
        .w_full()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .child(hero_brand_standalone(theme, brand_height))
        .child(hero_copy(theme, chrome_opacity));

    if let Some(message) = persistence_error {
        let muted = theme.foreground(ForegroundToken::Muted);
        let mono = SharedString::from("Space Mono");
        let message = SharedString::from(message);
        centered_content = centered_content.child(
            div()
                .w_full()
                .opacity(chrome_opacity)
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
        .child(action_row(theme, callbacks.clone(), chrome_opacity));

    div()
        .size_full()
        .flex()
        .flex_col()
        .pt(px(EDGE_INSET_TOP))
        .pb(px(EDGE_INSET_BOTTOM))
        .px(px(EDGE_INSET_H))
        .child(
            div()
                .opacity(chrome_opacity)
                .child(header_row(theme, callbacks.clone(), chrome_interactive)),
        )
        .child(div().h(px(8.)))
        .child(centered_content)
}

fn header_row(
    theme: OpenCoreTheme,
    callbacks: WelcomeCallbacks,
    chrome_interactive: bool,
) -> impl IntoElement {
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
        .child(theme_toggle_button(
            theme,
            callbacks.on_toggle_theme,
            chrome_interactive,
        ))
}

fn hero_glow(theme: OpenCoreTheme) -> impl IntoElement {
    let accent = theme.foreground(ForegroundToken::Accent);

    div()
        .absolute()
        .left(px(HERO_GLOW_INSET_H))
        .right(px(HERO_GLOW_INSET_H))
        .top(px(HERO_GLOW_INSET_TOP))
        .bottom(px(HERO_GLOW_INSET_BOTTOM))
        .shadow(vec![
            BoxShadow::new(px(0.), px(0.), accent.opacity(0.16)).blur_radius(px(72.)),
            BoxShadow::new(px(0.), px(0.), accent.opacity(0.08)).blur_radius(px(144.)),
        ])
}

fn hero_brand_standalone(theme: OpenCoreTheme, hero_height: f32) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .h(px(hero_height + 40.0))
        .flex()
        .items_center()
        .justify_center()
        .child(hero_glow(theme))
        .child(opencore_brand_image(theme, hero_height, 1.0))
}

fn hero_copy(theme: OpenCoreTheme, chrome_opacity: f32) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let grotesk = SharedString::from("Space Grotesk");
    let spacing = theme.spacing;

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
                .opacity(chrome_opacity)
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

fn action_row(
    theme: OpenCoreTheme,
    callbacks: WelcomeCallbacks,
    chrome_opacity: f32,
) -> impl IntoElement {
    let spacing = theme.spacing;
    let on_enter = callbacks.on_enter;
    div()
        .w_full()
        .opacity(chrome_opacity)
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
    use super::*;
    use gpui::Keystroke;

    #[test]
    fn welcome_titlebar_drag_starts_only_after_pointer_moves() {
        assert!(!welcome_drag_should_start(false, true));
        assert!(!welcome_drag_should_start(true, false));
        assert!(welcome_drag_should_start(true, true));
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
    fn welcome_hero_layout_constants() {
        assert_eq!(HERO_MAX_WIDTH, 680.0);
        assert_eq!(HERO_GLOW_INSET_H, 44.0);
        assert_eq!(ENTER_BUTTON_HEIGHT, 48.0);
    }
}
