//! Flat search result row with project context.

use gpui::{
    App, ClickEvent, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px, relative,
};
use gpui_component::{h_flex, v_flex};

use crate::shared::theme::{ForegroundToken, OpenCoreTheme, TypeRole};

use super::super::demo_data::{DEMO_PROJECTS, DemoAtom};
use super::super::state::SidebarViewModel;
use super::super::surfaces::{project_favicon_color, row_active_bg, row_hover_bg};
use super::super::tokens::{FAVICON_SIZE, ROW_CONTENT_INSET, ROW_HEIGHT_SLIM, ROW_RADIUS};

pub fn sidebar_search_result_row(
    atom: &DemoAtom,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    on_activate: impl Fn(String, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let is_active = view.is_active(atom);
    let title = view.display_title(atom);
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();
    let favicon_hue = DEMO_PROJECTS
        .iter()
        .find(|p| p.key == atom.project_key)
        .map(|p| p.favicon_hue)
        .unwrap_or(0x888888);

    let bg = if is_active {
        row_active_bg(theme)
    } else {
        theme.surface(crate::shared::theme::BackgroundToken::Primary)
    };

    let atom_id = atom.id.to_string();

    div()
        .id(format!("left-sidebar-search-{}", atom.id))
        .w_full()
        .min_w_0()
        .h(px(ROW_HEIGHT_SLIM))
        .flex()
        .items_center()
        .rounded(px(ROW_RADIUS))
        .bg(bg)
        .cursor_pointer()
        .hover(|style| style.bg(row_hover_bg(theme)))
        .on_click(move |_: &ClickEvent, window, cx| on_activate(atom_id.clone(), window, cx))
        .child(
            h_flex()
                .w_full()
                .min_w_0()
                .h_full()
                .px(px(ROW_CONTENT_INSET))
                .gap(px(8.))
                .items_center()
                .overflow_hidden()
                .child(project_favicon(favicon_hue))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .font_family(mono.clone())
                                .text_size(px(TypeRole::LabelMd.size()))
                                .line_height(relative(TypeRole::LabelMd.line_height()))
                                .text_color(primary)
                                .child(title),
                        )
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .font_family(mono)
                                .text_size(px(TypeRole::MonoSm.size()))
                                .text_color(secondary.alpha(0.8))
                                .child(atom.project_title),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .font_family(mono_family())
                        .text_size(px(TypeRole::LabelMd.size()))
                        .text_color(muted)
                        .child(atom.time_label),
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
