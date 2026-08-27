//! Amber-tinted draft row for unsent composer sessions.

use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, ParentElement, SharedString, Styled,
    Window, div, px, relative,
};
use gpui_component::{
    Icon, IconName, Sizable,
    button::{Button, ButtonRounded, ButtonVariants as _},
    h_flex, v_flex,
};

use crate::shared::theme::{ForegroundToken, OpenCoreTheme, TypeRole};

use super::super::demo_data::{DemoDraft, DEMO_PROJECTS};
use super::super::surfaces::{draft_bg, draft_bg_hover, project_favicon_color};
use super::super::tokens::{CONTENT_INSET, FAVICON_SIZE, ROW_CONTENT_INSET, ROW_RADIUS};

pub fn sidebar_draft_row(
    draft: &DemoDraft,
    is_active: bool,
    theme: &OpenCoreTheme,
    on_activate: impl Fn(&mut Window, &mut App) + 'static,
    on_discard: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let amber: gpui::Hsla = gpui::rgb(crate::shared::theme::WARNING_AMBER).into();
    let mono = mono_family();
    let favicon_hue = DEMO_PROJECTS
        .iter()
        .find(|p| p.key == draft.project_key)
        .map(|p| p.favicon_hue)
        .unwrap_or(0x888888);

    let bg = if is_active {
        super::super::surfaces::row_active_bg(theme)
    } else {
        draft_bg(theme)
    };

    div()
        .id(format!("left-sidebar-draft-{}", draft.id))
        .w_full()
        .min_w_0()
        .cursor_pointer()
        .rounded(px(ROW_RADIUS))
        .bg(bg)
        .hover(|style| style.bg(draft_bg_hover(theme)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| on_activate(window, cx))
        .child(
            div()
                .w_full()
                .min_w_0()
                .px(px(ROW_CONTENT_INSET))
                .py(px(CONTENT_INSET))
                .child(
                    v_flex()
                        .w_full()
                        .min_w_0()
                        .gap(px(4.))
                        .child(
                            h_flex()
                                .w_full()
                                .min_w_0()
                                .items_center()
                                .gap(px(6.))
                                .child(
                                    Icon::new(IconName::File)
                                        .text_color(amber.alpha(0.85))
                                        .small()
                                        .flex_shrink_0(),
                                )
                                .child(project_favicon(favicon_hue))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .font_family(mono.clone())
                                        .text_size(px(TypeRole::LabelMd.size()))
                                        .line_height(relative(TypeRole::LabelMd.line_height()))
                                        .text_color(secondary)
                                        .child(draft.project_title),
                                )
                                .child(
                                    Button::new("left-sidebar-discard-draft")
                                        .ghost()
                                        .rounded(ButtonRounded::None)
                                        .tooltip("Discard draft")
                                        .icon(Icon::new(IconName::Close).text_color(muted))
                                        .h(px(20.))
                                        .w(px(20.))
                                        .on_click(move |_, window, cx| on_discard(window, cx)),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .font_family(mono)
                                .text_size(px(TypeRole::LabelMd.size()))
                                .line_height(relative(TypeRole::LabelMd.line_height()))
                                .text_color(primary.alpha(0.9))
                                .child(draft.preview),
                        ),
                ),
        )
}

fn project_favicon(hue: u32) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .w(px(FAVICON_SIZE))
        .h(px(FAVICON_SIZE))
        .rounded(px(3.))
        .bg(project_favicon_color(hue).alpha(0.25))
        .border_1()
        .border_color(project_favicon_color(hue).alpha(0.5))
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
