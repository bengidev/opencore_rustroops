//! **Composite** shell chrome: three labeled workspace zones (Editor, Chat, Terminal)
//! arranged as a flex split for future feature mounting.

use std::rc::Rc;

use gpui::{
    div, px, relative, App, Div, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Styled, Window,
};

use gpui_component::h_flex;

use crate::shared::theme::{ColorToken, OpenCoreTheme};

/// Optional shell callbacks (dev reset is debug-only).
pub struct ShellCallbacks {
    #[cfg(debug_assertions)]
    pub on_reset_dev: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl ShellCallbacks {
    pub fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            on_reset_dev: None,
        }
    }
}

impl Default for ShellCallbacks {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub shell layout — Editor and Chat side-by-side, Terminal full-width below.
pub fn shell_screen(theme: OpenCoreTheme, callbacks: ShellCallbacks) -> impl IntoElement {
    let spacing = theme.spacing;
    let background = color_from_token(theme.background);
    let foreground = color_from_token(theme.foreground);
    let border = color_from_token(theme.border);
    let label = theme.label;

    let mut root = div()
        .size_full()
        .flex()
        .flex_col()
        .gap(px(spacing.sm as f32))
        .p(px(spacing.md as f32))
        .bg(background);

    #[cfg(debug_assertions)]
    if let Some(on_reset) = callbacks.on_reset_dev {
        root = root.child(dev_reset_bar(border, foreground, on_reset));
    }

    root.child(
        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(spacing.sm as f32))
            .child(
                h_flex()
                    .flex_1()
                    .gap(px(spacing.sm as f32))
                    .child(zone_panel("Editor", border, foreground, label))
                    .child(zone_panel("Chat", border, foreground, label)),
            )
            .child(zone_panel("Terminal", border, foreground, label).h(relative(0.35))),
    )
}

#[cfg(debug_assertions)]
fn dev_reset_bar(
    border: Hsla,
    foreground: Hsla,
    on_reset: Rc<dyn Fn(&mut Window, &mut App)>,
) -> Div {
    div()
        .w_full()
        .flex()
        .justify_end()
        .child(
            div()
                .px(px(12.))
                .py(px(8.))
                .rounded_md()
                .border_1()
                .border_color(border)
                .text_size(px(11.))
                .text_color(foreground.opacity(0.75))
                .cursor_pointer()
                .child("Reset dev data")
                .on_mouse_down(MouseButton::Left, move |_, window, cx| on_reset(window, cx)),
        )
}

fn zone_panel(
    label: &'static str,
    border: Hsla,
    foreground: Hsla,
    type_role: crate::shared::theme::LegacyTypeRole,
) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(border)
        .text_size(px(type_role.size_px as f32))
        .font_weight(gpui::FontWeight(type_role.weight as f32))
        .text_color(foreground.opacity(0.9))
        .child(label)
}

fn color_from_token(token: ColorToken) -> Hsla {
    let hex = token.0.trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).unwrap_or(0);
    gpui::rgb(value).into()
}
