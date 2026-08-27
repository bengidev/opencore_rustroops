//! Sidebar chrome header with wordmark and optional build channel pill.

use gpui::{
    InteractiveElement, IntoElement, ParentElement, SharedString, Styled, div, px, relative,
};
use gpui_component::h_flex;

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::super::tokens::{CONTENT_INSET, DOCK_RESIZE_GUTTER, HEADER_HEIGHT};

pub fn sidebar_chrome_header(theme: &OpenCoreTheme) -> impl IntoElement {
    let page = theme.surface(BackgroundToken::Primary);
    let border = theme.border_token(BorderToken::Default);
    let primary = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();

    h_flex()
        .id("left-sidebar-header")
        .w_full()
        .flex_shrink_0()
        .h(px(HEADER_HEIGHT))
        .items_center()
        .gap(px(SpacingToken::S1.value()))
        .pl(px(CONTENT_INSET))
        .pr(px(CONTENT_INSET + DOCK_RESIZE_GUTTER))
        .border_b_1()
        .border_color(border)
        .bg(page)
        .child(
            div()
                .font_family(mono.clone())
                .text_size(px(TypeRole::LabelMd.size()))
                .line_height(relative(TypeRole::LabelMd.line_height()))
                .text_color(primary)
                .child("OpenCore"),
        )
        .child(
            div()
                .font_family(mono)
                .text_size(px(TypeRole::LabelMd.size()))
                .line_height(relative(TypeRole::LabelMd.line_height()))
                .text_color(muted)
                .child("Rustroops"),
        )
        .child(
            div()
                .ml(px(4.))
                .px(px(6.))
                .py(px(2.))
                .rounded(px(999.))
                .bg(theme.surface(BackgroundToken::Secondary))
                .border_1()
                .border_color(border)
                .font_family(mono_family())
                .text_size(px(TypeRole::MonoSm.size()))
                .text_color(muted)
                .child("Dev"),
        )
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
