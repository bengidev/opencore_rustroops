//! T3-style sidebar footer: settings, pull requests, usage utility cluster.

use gpui::{IntoElement, ParentElement, SharedString, Styled, px};
use gpui_component::{
    Icon, IconName,
    button::{Button, ButtonRounded, ButtonVariants as _},
    h_flex,
};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::super::tokens::{CONTENT_INSET, ICON_BUTTON_SIZE};

pub fn sidebar_chrome_footer(theme: &OpenCoreTheme) -> impl IntoElement {
    let surface = theme.surface(BackgroundToken::Primary);
    let border = theme.border_token(BorderToken::Default);
    let muted = theme.foreground(ForegroundToken::Muted);

    h_flex()
        .w_full()
        .flex_shrink_0()
        .items_center()
        .gap(px(SpacingToken::S1.value()))
        .p(px(CONTENT_INSET))
        .border_t_1()
        .border_color(border)
        .bg(surface)
        .child(utility_icon_button(
            "left-sidebar-settings",
            IconName::Settings,
            "Settings",
            muted,
            border,
            surface,
        ))
        .child(utility_icon_button(
            "left-sidebar-prs",
            IconName::Github,
            "Pull Requests",
            muted,
            border,
            surface,
        ))
        .child(utility_icon_button(
            "left-sidebar-usage",
            IconName::ChartPie,
            "Usage",
            muted,
            border,
            surface,
        ))
}

fn utility_icon_button(
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    icon_color: gpui::Hsla,
    border: gpui::Hsla,
    surface: gpui::Hsla,
) -> impl IntoElement {
    Button::new(id)
        .ghost()
        .rounded(ButtonRounded::None)
        .tooltip(tooltip)
        .icon(Icon::new(icon).text_color(icon_color))
        .h(px(ICON_BUTTON_SIZE))
        .w(px(ICON_BUTTON_SIZE))
        .text_size(px(TypeRole::MonoSm.size()))
        .font_family(mono_family())
        .border_1()
        .border_color(border)
        .bg(surface)
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
