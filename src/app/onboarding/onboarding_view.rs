//! Onboarding view — immersive monochrome landing ported to GPUI.

use gpui::{
    FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, SharedString, Styled, canvas, div, px, relative,
};

use crate::app::gpui_callbacks::{AppHandler, WindowAppHandler};

use crate::shared::theme::{
    ActionToken, BackgroundToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::onboarding_draw::Painter;
use super::onboarding_galaxy_orb::GalaxyOrb;
use super::onboarding_scene_backdrop::SceneBackdrop;
use super::onboarding_theme_toggle::theme_toggle_button;
use super::onboarding_ui_state::OnboardingUiState;

const HERO_MAX_WIDTH: f32 = 600.0;
const ORB_HEIGHT: f32 = 300.0;
const EDGE_INSET_H: f32 = 16.0;
const EDGE_INSET_V: f32 = 20.0;

#[derive(Clone)]
pub struct OnboardingCallbacks {
    pub on_enter: WindowAppHandler,
    pub on_toggle_theme: WindowAppHandler,
    pub on_orb_pressed: AppHandler,
    pub on_orb_released: AppHandler,
}

/// Full-screen onboarding scene matching the reference layout.
pub fn onboarding_screen(
    theme: OpenCoreTheme,
    ui: &OnboardingUiState,
    callbacks: OnboardingCallbacks,
    persistence_error: Option<&str>,
    focus_handle: &FocusHandle,
) -> impl IntoElement {
    let background = theme.surface(BackgroundToken::Primary);
    let backdrop = SceneBackdrop::new(theme, ui.started_at, ui.now);
    let on_enter_key = callbacks.on_enter.clone();

    div()
        .size_full()
        .tab_index(0)
        .track_focus(focus_handle)
        .bg(background)
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if is_enter_keystroke(event) {
                on_enter_key(window, cx);
            }
        })
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(backdrop_canvas(backdrop)),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(main_column(theme, ui, callbacks, persistence_error)),
        )
}

fn is_enter_keystroke(event: &KeyDownEvent) -> bool {
    let key = event.keystroke.key.as_str();
    matches!(key, "enter" | "return") && !event.is_held && !event.keystroke.modifiers.modified()
}

fn backdrop_canvas(backdrop: SceneBackdrop) -> impl IntoElement {
    canvas(
        move |bounds, _, _| (bounds, backdrop),
        move |scene_bounds, (_, backdrop), window, _| {
            backdrop.paint(&mut Painter::new(window), scene_bounds);
        },
    )
    .size_full()
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
        .child(hero_block(theme, ui, callbacks.clone()))
        .child(div().flex_grow(1.));

    if let Some(message) = persistence_error {
        let muted = theme.foreground(ForegroundToken::Muted);
        let message = SharedString::from(message);
        column = column.child(
            div()
                .w_full()
                .text_center()
                .text_size(px(11.))
                .text_color(muted)
                .pb(px(4.))
                .child(message),
        );
    }

    column.child(action_row(theme, callbacks))
}

fn header_row(theme: OpenCoreTheme, callbacks: OnboardingCallbacks) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);

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
                        .text_color(primary)
                        .child("OpenCore"),
                )
                .child(
                    div()
                        .text_size(px(9.))
                        .text_color(muted)
                        .child("LOCAL AI WORKSPACE"),
                ),
        )
        .child(div().flex_grow(1.))
        .child(theme_toggle_button(theme, callbacks.on_toggle_theme))
}

fn hero_block(
    theme: OpenCoreTheme,
    ui: &OnboardingUiState,
    callbacks: OnboardingCallbacks,
) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let mono = SharedString::from("Menlo");
    let orb = GalaxyOrb::with_dynamics(
        theme,
        ui.started_at,
        ui.now,
        ui.displayed_speed,
        ui.displayed_zoom,
    );
    let on_pressed = callbacks.on_orb_pressed.clone();
    let on_released = callbacks.on_orb_released.clone();

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
                .child(
                    div()
                        .w_full()
                        .h(px(ORB_HEIGHT))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, _, cx| {
                            on_pressed(cx);
                        })
                        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                            on_released(cx);
                        })
                        .child(orb_canvas(orb)),
                )
                .child(div().h(px(28.)))
                .child(
                    div()
                        .w_full()
                        .text_center()
                        .text_size(px(TypeRole::DisplayMd.size()))
                        .text_color(primary)
                        .child("Your local AI command workspace"),
                )
                .child(div().h(px(10.)))
                .child(
                    div()
                        .w_full()
                        .max_w(px(HERO_MAX_WIDTH))
                        .text_center()
                        .text_size(px(TypeRole::MonoSm.size()))
                        .line_height(relative(TypeRole::MonoSm.line_height()))
                        .font_family(mono)
                        .text_color(secondary)
                        .child("OpenCore combines chat, terminal, editing, and Rust-native performance in one permissioned desktop environment. To leave the crowded cloud, polluted by leaks and unconsciousness, to return to a workspace that stays on your machine."),
                ),
        )
}

fn orb_canvas(orb: GalaxyOrb) -> impl IntoElement {
    canvas(
        move |bounds, _, _| (bounds, orb),
        move |scene_bounds, (_, orb), window, _| {
            orb.paint(&mut Painter::new(window), scene_bounds);
        },
    )
    .w_full()
    .h_full()
}

fn action_row(theme: OpenCoreTheme, callbacks: OnboardingCallbacks) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .pb(px(8.))
        .child(primary_button(theme, "Enter OpenCore", callbacks.on_enter))
}

fn primary_button(
    theme: OpenCoreTheme,
    label: &'static str,
    on_press: WindowAppHandler,
) -> impl IntoElement {
    let bg = theme.action(ActionToken::Strong);
    let text = theme.action(ActionToken::StrongText);
    let radius = px(theme.control_radius());

    div()
        .px(px(28.))
        .py(px(14.))
        .rounded(radius)
        .bg(bg)
        .text_size(px(13.))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(text)
        .cursor_pointer()
        .child(label)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| on_press(window, cx))
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
