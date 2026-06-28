//! **Composite** shell chrome: three labeled workspace zones (Editor, Chat, Terminal)
//! arranged as a flex split for future feature mounting.

use gpui::{
    Div, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled, div, px,
    relative,
};

use gpui_component::h_flex;

use crate::app::gpui_callbacks::WindowAppHandler;
use crate::shared::theme::{BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme};

/// Optional shell callbacks (dev reset is debug-only).
pub struct ShellCallbacks {
    #[cfg(debug_assertions)]
    pub on_reset_dev: Option<WindowAppHandler>,
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
    let background = theme.surface(BackgroundToken::Primary);
    let foreground = theme.foreground(ForegroundToken::Primary);
    let border = theme.border_token(BorderToken::Default);
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
fn dev_reset_bar(border: Hsla, foreground: Hsla, on_reset: WindowAppHandler) -> Div {
    div().w_full().flex().justify_end().child(
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
