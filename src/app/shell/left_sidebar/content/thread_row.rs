//! Thread row surfaces: card layout for inbox/pinned, slim layout for history.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, InteractiveElement, IntoElement, MouseButton, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px, relative,
    prelude::FluentBuilder as _,
};
use gpui_component::{
    Icon, IconName, Sizable,
    h_flex,
    menu::{ContextMenuExt as _, PopupMenuItem},
    v_flex,
};

use crate::shared::theme::{
    BackgroundToken, ForegroundToken, OpenCoreTheme, TypeRole,
};

use super::super::demo_data::{DemoThread, DEMO_PROJECTS, ThreadStatus};
use super::super::state::SidebarViewModel;
use super::super::surfaces::{
    pr_open_color, project_favicon_color, row_active_bg, row_hover_bg, row_selected_bg,
    status_color,
};
use super::super::tokens::{
    CONTENT_INSET, FAVICON_SIZE, ROW_CONTENT_INSET, ROW_HEIGHT_CARD, ROW_HEIGHT_SLIM, ROW_RADIUS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadRowVariant {
    Card,
    Slim,
}

pub struct ThreadRowActions {
    pub on_activate: Rc<dyn Fn(String, &mut Window, &mut App)>,
    pub on_select: Rc<dyn Fn(String, bool, &mut Window, &mut App)>,
    pub on_hover: Rc<dyn Fn(Option<String>, &mut Window, &mut App)>,
    pub on_move_pinned: Rc<dyn Fn(String, isize, &mut Window, &mut App)>,
}

pub fn sidebar_thread_row(
    thread: &DemoThread,
    variant: ThreadRowVariant,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &ThreadRowActions,
) -> AnyElement {
    match variant {
        ThreadRowVariant::Card => card_row(thread, view, theme, actions).into_any_element(),
        ThreadRowVariant::Slim => slim_row(thread, view, theme, actions).into_any_element(),
    }
}

pub fn sidebar_pinned_divider(theme: &OpenCoreTheme) -> impl IntoElement {
    let border = theme.border_token(crate::shared::theme::BorderToken::Default);
    div()
        .id("left-sidebar-pinned-divider")
        .w_full()
        .my(px(super::super::tokens::PINNED_DIVIDER_MARGIN))
        .h(px(1.))
        .bg(border.alpha(0.6))
}

pub fn sidebar_show_more_button(
    theme: &OpenCoreTheme,
    on_show_more: impl Fn(&mut gpui::Window, &mut App) + 'static,
) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
  div()
        .id("left-sidebar-show-more-settled")
        .w_full()
        .py(px(8.))
        .text_center()
        .cursor_pointer()
        .font_family(mono_family())
        .text_size(px(TypeRole::LabelMd.size()))
        .text_color(muted)
        .hover(|style| style.text_color(theme.foreground(ForegroundToken::Primary)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| on_show_more(window, cx))
        .child("Show more")
}

fn row_surface(
    thread: &DemoThread,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    height: f32,
    variant_key: &str,
    actions: &ThreadRowActions,
    content: impl IntoElement,
) -> impl IntoElement {
    let is_active = view.is_active(thread);
    let is_selected = view.is_selected(thread);
    let recede = view.should_recede(thread);

    let (bg, text) = if is_active {
        (row_active_bg(theme), theme.foreground(ForegroundToken::Primary))
    } else if is_selected {
        (row_selected_bg(theme), theme.foreground(ForegroundToken::Primary))
    } else {
        (
            theme.surface(BackgroundToken::Primary),
            if recede {
                theme.foreground(ForegroundToken::Muted).alpha(0.75)
            } else {
                theme.foreground(ForegroundToken::Primary)
            },
        )
    };

    let thread_id = thread.id.to_string();
    let on_select = actions.on_select.clone();
    let on_activate = actions.on_activate.clone();
    let on_hover = actions.on_hover.clone();
    let on_move_pinned = actions.on_move_pinned.clone();
    let pinned = thread.pinned;
    let menu_title = thread.title.to_string();

    div()
        .id(format!("left-sidebar-thread-{variant_key}-{}", thread.id))
        .w_full()
        .min_w_0()
        .h(px(height))
        .flex()
        .items_center()
        .overflow_hidden()
        .rounded(px(ROW_RADIUS))
        .bg(bg)
        .text_color(text)
        .cursor_pointer()
        .when(!is_active && !is_selected, |row| {
            row.hover(|style| style.bg(row_hover_bg(theme)))
        })
        .when(recede && !is_active && !is_selected, |row| {
            row.opacity(0.85)
        })
        .on_hover({
            let on_hover = on_hover.clone();
            let hover_id = thread_id.clone();
            move |hovered, window, cx| {
                on_hover(
                    if *hovered { Some(hover_id.clone()) } else { None },
                    window,
                    cx,
                );
            }
        })
        .on_click({
            let on_select = on_select.clone();
            let on_activate = on_activate.clone();
            let click_id = thread_id.clone();
            move |event: &ClickEvent, window, cx| {
                if event.modifiers().shift {
                    on_select(click_id.clone(), true, window, cx);
                } else {
                    on_activate(click_id.clone(), window, cx);
                }
            }
        })
        .context_menu({
            let on_move_pinned = on_move_pinned.clone();
            let menu_id = thread_id.clone();
            move |menu, _window, _cx| {
                let mut menu = menu.label(menu_title.clone());
                if pinned {
                    let up_id = menu_id.clone();
                    let down_id = menu_id.clone();
                    let on_move_up = on_move_pinned.clone();
                    let on_move_down = on_move_pinned.clone();
                    menu = menu.item(
                        PopupMenuItem::new("Move up").on_click(move |_, window, cx| {
                            on_move_up(up_id.clone(), -1, window, cx);
                        }),
                    );
                    menu = menu.item(
                        PopupMenuItem::new("Move down").on_click(move |_, window, cx| {
                            on_move_down(down_id.clone(), 1, window, cx);
                        }),
                    );
                }
                menu.item(PopupMenuItem::new("Settle"))
                    .item(PopupMenuItem::new("Snooze"))
                    .separator()
                    .item(PopupMenuItem::new("Rename"))
                    .item(PopupMenuItem::new("Archive"))
            }
        })
        .child(content)
}

fn card_row(
    thread: &DemoThread,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &ThreadRowActions,
) -> impl IntoElement {
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();
    let recede = view.should_recede(thread);
    let favicon_hue = DEMO_PROJECTS
        .iter()
        .find(|p| p.key == thread.project_key)
        .map(|p| p.favicon_hue)
        .unwrap_or(0x888888);

    row_surface(
        thread,
        view,
        theme,
        ROW_HEIGHT_CARD,
        "card",
        actions,
        div()
            .w_full()
            .min_w_0()
            .h_full()
            .overflow_hidden()
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
                            .children(pin_marker(thread, muted))
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
                                    .text_color(if recede {
                                        secondary.alpha(0.75)
                                    } else {
                                        secondary
                                    })
                                    .child(thread.project_title),
                            )
                            .child(status_or_time(thread, view, theme, true)),
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
                            .text_color(if recede {
                                theme.foreground(ForegroundToken::Secondary)
                            } else {
                                theme.foreground(ForegroundToken::Primary).alpha(0.9)
                            })
                            .child(thread.title),
                    )
                    .child(branch_meta_row(thread, theme)),
            ),
    )
}

fn slim_row(
    thread: &DemoThread,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &ThreadRowActions,
) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();
    let recede = view.should_recede(thread);

    row_surface(
        thread,
        view,
        theme,
        ROW_HEIGHT_SLIM,
        "slim",
        actions,
        h_flex()
            .w_full()
            .min_w_0()
            .h_full()
            .px(px(10.))
            .gap(px(10.))
            .items_center()
            .overflow_hidden()
            .child(
                Icon::new(IconName::Inbox)
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
                    .text_color(if recede {
                        muted.alpha(0.7)
                    } else {
                        theme.foreground(ForegroundToken::Primary)
                    })
                    .child(thread.title),
            )
            .child(status_or_time(thread, view, theme, false)),
    )
}

fn pin_marker(thread: &DemoThread, muted: gpui::Hsla) -> Option<gpui::AnyElement> {
    if !thread.pinned {
        return None;
    }
    Some(
        Icon::new(IconName::Star)
            .text_color(muted.alpha(0.7))
            .small()
            .flex_shrink_0()
            .into_any_element(),
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

fn status_or_time(
    thread: &DemoThread,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    show_status: bool,
) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();
    let dimmed = view.should_recede(thread) && !view.is_active(thread);

    if show_status {
        if let Some(label) = thread.status.label() {
            let color = status_color(thread.status, theme, dimmed);
            return h_flex()
                .flex_shrink_0()
                .gap(px(4.))
                .items_center()
                .children(status_icon(thread.status, color))
                .child(
                    div()
                        .font_family(mono)
                        .text_size(px(TypeRole::LabelMd.size()))
                        .text_color(color)
                        .child(label),
                )
                .into_any_element();
        }
    }

    div()
        .flex_shrink_0()
        .font_family(mono)
        .text_size(px(TypeRole::LabelMd.size()))
        .text_color(muted)
        .child(thread.time_label)
        .into_any_element()
}

fn status_icon(status: ThreadStatus, color: gpui::Hsla) -> Option<gpui::AnyElement> {
    match status {
        ThreadStatus::Working => Some(
            Icon::new(IconName::LoaderCircle)
                .text_color(color)
                .small()
                .flex_shrink_0()
                .into_any_element(),
        ),
        ThreadStatus::Woke => Some(
            Icon::new(IconName::Bell)
                .text_color(color)
                .small()
                .flex_shrink_0()
                .into_any_element(),
        ),
        _ => None,
    }
}

fn branch_meta_row(thread: &DemoThread, theme: &OpenCoreTheme) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let green = pr_open_color();
    let red = theme.foreground(ForegroundToken::Accent);
    let mono = mono_family();

    h_flex()
        .w_full()
        .min_w_0()
        .items_center()
        .gap(px(6.))
        .overflow_hidden()
        .text_size(px(TypeRole::LabelMd.size()))
        .child(
            thread.branch.map_or_else(
                || div().flex_1().min_w_0().into_any_element(),
                |branch| {
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .font_family(mono.clone())
                        .text_color(muted)
                        .child(branch)
                        .into_any_element()
                },
            ),
        )
        .children(terminal_indicator(thread, muted))
        .children(pr_badge(thread, muted, mono.clone()))
        .children(diff_stats(thread, green, red, mono))
}

fn terminal_indicator(thread: &DemoThread, muted: gpui::Hsla) -> Vec<gpui::AnyElement> {
    if thread.terminal_process_count == 0 {
        return Vec::new();
    }
    vec![
        Icon::new(IconName::SquareTerminal)
            .text_color(muted)
            .small()
            .flex_shrink_0()
            .into_any_element(),
    ]
}

fn pr_badge(thread: &DemoThread, color: gpui::Hsla, mono: SharedString) -> Vec<gpui::AnyElement> {
    thread
        .pr_number
        .map(|number| {
            div()
                .flex_shrink_0()
                .font_family(mono)
                .text_color(color)
                .child(format!("#{number}"))
                .into_any_element()
        })
        .into_iter()
        .collect()
}

fn diff_stats(
    thread: &DemoThread,
    green: gpui::Hsla,
    red: gpui::Hsla,
    mono: SharedString,
) -> Vec<gpui::AnyElement> {
    if thread.diff_insertions.is_none() && thread.diff_deletions.is_none() {
        return Vec::new();
    }

    let mut elements = Vec::new();
    if let Some(n) = thread.diff_insertions {
        elements.push(
            div()
                .flex_shrink_0()
                .font_family(mono.clone())
                .text_color(green)
                .child(format!("+{n}"))
                .into_any_element(),
        );
    }
    if let Some(n) = thread.diff_deletions {
        elements.push(
            div()
                .flex_shrink_0()
                .font_family(mono)
                .text_color(red)
                .child(format!("−{n}"))
                .into_any_element(),
        );
    }
    elements
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
