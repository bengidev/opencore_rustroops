//! Theme toggle chip using gpui-component Lucide icons (`IconName::Sun` / `IconName::Moon`).

use std::rc::Rc;

use gpui::{
    div, px, App, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled,
    Window,
};
use gpui_component::{Icon, IconName, Sizable};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, ThemeMode,
};

pub fn theme_toggle_button(
    theme: OpenCoreTheme,
    on_press: Rc<dyn Fn(&mut Window, &mut App)>,
) -> impl IntoElement {
    let (icon, label) = match theme.mode {
        ThemeMode::Dark => (IconName::Sun, "Light"),
        ThemeMode::Light => (IconName::Moon, "Dark"),
    };
    let chip_bg = theme.surface(BackgroundToken::Tertiary);
    let border = theme.border_token(BorderToken::Default);
    let foreground = theme.foreground(ForegroundToken::Primary);
    let radius = px(theme.control_radius());

    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(10.))
        .py(px(8.))
        .rounded(radius)
        .border_1()
        .border_color(border)
        .bg(chip_bg)
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(foreground)
        .cursor_pointer()
        .child(Icon::new(icon).small().text_color(foreground))
        .child(label)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| on_press(window, cx))
}
