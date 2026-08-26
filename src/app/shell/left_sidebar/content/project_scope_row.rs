//! Project scope filter row and new-project affordance.

use gpui::{InteractiveElement, IntoElement, ParentElement, SharedString, Styled, div, px, relative};
use gpui_component::{
    Icon, IconName, Sizable,
    button::{Button, ButtonRounded, ButtonVariants as _},
    h_flex,
};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::super::tokens::{ICON_BUTTON_SIZE, ROW_CONTENT_INSET};

pub fn sidebar_project_scope_row(project_label: &str, theme: &OpenCoreTheme) -> impl IntoElement {
    let surface = theme.surface(BackgroundToken::Secondary);
    let border = theme.border_token(BorderToken::Default);
    let primary = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();

    h_flex()
        .id("left-sidebar-project-scope")
        .w_full()
        .min_w_0()
        .gap(px(SpacingToken::S1.value()))
        .items_center()
        .overflow_hidden()
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .h(px(ICON_BUTTON_SIZE))
                .items_center()
                .gap(px(8.))
                .pl(px(ROW_CONTENT_INSET - 1.))
                .pr(px(8.))
                .border_1()
                .border_color(border)
                .bg(surface)
                .overflow_hidden()
                .child(
                    Icon::new(IconName::Folder)
                        .text_color(muted)
                        .small()
                        .flex_shrink_0(),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .font_family(mono)
                        .text_size(px(TypeRole::LabelMd.size()))
                        .line_height(relative(TypeRole::LabelMd.line_height()))
                        .text_color(primary)
                        .child(SharedString::from(project_label)),
                )
                .child(
                    Icon::new(IconName::ChevronDown)
                        .text_color(muted)
                        .small()
                        .flex_shrink_0(),
                ),
        )
        .child(
            Button::new("left-sidebar-new-project")
                .ghost()
                .rounded(ButtonRounded::None)
                .tooltip("New project")
                .icon(Icon::new(IconName::FolderOpen).text_color(primary))
                .h(px(ICON_BUTTON_SIZE))
                .w(px(ICON_BUTTON_SIZE))
                .flex_shrink_0()
                .border_1()
                .border_color(border)
                .bg(surface),
        )
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
