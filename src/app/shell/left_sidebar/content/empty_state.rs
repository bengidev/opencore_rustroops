//! Empty states for zero-project and zero-thread shelves.

#![allow(dead_code)]

use gpui::{IntoElement, ParentElement, SharedString, Styled, div, px, relative};
use gpui_component::{
    Icon, IconName,
    button::{Button, ButtonRounded, ButtonVariants as _},
    v_flex,
};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

pub fn sidebar_empty_state(message: &str, theme: &OpenCoreTheme) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);

    v_flex()
        .w_full()
        .items_center()
        .gap(px(SpacingToken::S3.value()))
        .px(px(8.))
        .py(px(24.))
        .child(
            div()
                .text_center()
                .font_family(mono_family())
                .text_size(px(TypeRole::LabelMd.size()))
                .line_height(relative(TypeRole::LabelMd.line_height()))
                .text_color(muted.alpha(0.6))
                .child(SharedString::from(message)),
        )
}

pub fn sidebar_add_project_button(theme: &OpenCoreTheme) -> impl IntoElement {
    let surface = theme.surface(BackgroundToken::Secondary);
    let border = theme.border_token(BorderToken::Default);
    let muted = theme.foreground(ForegroundToken::Muted);

    Button::new("left-sidebar-add-project")
        .ghost()
        .rounded(ButtonRounded::None)
        .label("Add project")
        .icon(Icon::new(IconName::Plus).text_color(muted))
        .h(px(28.))
        .text_size(px(TypeRole::LabelMd.size()))
        .font_family(mono_family())
        .text_color(muted)
        .border_1()
        .border_color(border)
        .bg(surface)
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
