//! Sidebar footer: utilities, back navigation, and update pill stub.

use gpui::{
    App, IntoElement, ParentElement, SharedString, Styled, Window, div, px,
};
use gpui_component::{
    Icon, IconName,
    button::{Button, ButtonRounded, ButtonVariants as _},
    h_flex,
};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
    SUCCESS_GREEN,
};

use super::super::state::FooterMode;
use super::super::tokens::{CONTENT_INSET, ICON_BUTTON_SIZE};

pub fn sidebar_chrome_footer(
    mode: FooterMode,
    show_update_pill: bool,
    theme: &OpenCoreTheme,
    on_back: impl Fn(&mut Window, &mut App) + 'static,
    on_settings: impl Fn(&mut Window, &mut App) + 'static,
    on_prs: impl Fn(&mut Window, &mut App) + 'static,
    on_usage: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
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
        .children(if show_update_pill {
            Some(update_pill(theme, border, surface))
        } else {
            None
        })
        .children(match mode {
            FooterMode::Utilities => Some(
                utilities_cluster(muted, border, surface, on_settings, on_prs, on_usage),
            ),
            FooterMode::Back => Some(back_button(muted, border, surface, on_back)),
        })
}

fn utilities_cluster(
    muted: gpui::Hsla,
    border: gpui::Hsla,
    surface: gpui::Hsla,
    on_settings: impl Fn(&mut Window, &mut App) + 'static,
    on_prs: impl Fn(&mut Window, &mut App) + 'static,
    on_usage: impl Fn(&mut Window, &mut App) + 'static,
) -> gpui::AnyElement {
    h_flex()
        .gap(px(SpacingToken::S1.value()))
        .child(utility_icon_button(
            "left-sidebar-settings",
            IconName::Settings,
            "Settings",
            muted,
            border,
            surface,
            on_settings,
        ))
        .child(utility_icon_button(
            "left-sidebar-prs",
            IconName::Github,
            "Pull Requests",
            muted,
            border,
            surface,
            on_prs,
        ))
        .child(utility_icon_button(
            "left-sidebar-usage",
            IconName::ChartPie,
            "Usage",
            muted,
            border,
            surface,
            on_usage,
        ))
        .into_any_element()
}

fn back_button(
    muted: gpui::Hsla,
    border: gpui::Hsla,
    surface: gpui::Hsla,
    on_back: impl Fn(&mut Window, &mut App) + 'static,
) -> gpui::AnyElement {
    Button::new("left-sidebar-back")
        .ghost()
        .rounded(ButtonRounded::None)
        .label("Back")
        .icon(Icon::new(IconName::ArrowLeft).text_color(muted))
        .h(px(ICON_BUTTON_SIZE))
        .text_size(px(TypeRole::LabelMd.size()))
        .font_family(mono_family())
        .text_color(muted)
        .border_1()
        .border_color(border)
        .bg(surface)
        .on_click(move |_, window, cx| on_back(window, cx))
        .into_any_element()
}

fn update_pill(theme: &OpenCoreTheme, border: gpui::Hsla, surface: gpui::Hsla) -> gpui::AnyElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let dot: gpui::Hsla = gpui::rgb(SUCCESS_GREEN).into();

    h_flex()
        .mr(px(4.))
        .items_center()
        .gap(px(6.))
        .px(px(8.))
        .h(px(ICON_BUTTON_SIZE))
        .border_1()
        .border_color(border)
        .bg(surface)
        .rounded(px(4.))
        .child(
            div()
                .w(px(6.))
                .h(px(6.))
                .rounded(px(999.))
                .bg(dot),
        )
        .child(
            div()
                .font_family(mono_family())
                .text_size(px(TypeRole::MonoSm.size()))
                .text_color(muted)
                .child("Update"),
        )
        .into_any_element()
}

fn utility_icon_button(
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    icon_color: gpui::Hsla,
    border: gpui::Hsla,
    surface: gpui::Hsla,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
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
        .on_click(move |_, window, cx| on_click(window, cx))
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
