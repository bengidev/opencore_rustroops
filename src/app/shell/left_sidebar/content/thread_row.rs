//! Thread row surfaces: card layout for inbox/pinned, slim layout for history.

use gpui::{
    AnyElement, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, div, px,
    relative,
};
use gpui_component::{Icon, IconName, Sizable, h_flex, v_flex};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, TypeRole,
    SUCCESS_GREEN,
};

use super::super::demo_data::DemoThread;
use super::super::tokens::{CONTENT_INSET, ROW_CONTENT_INSET, ROW_HEIGHT_CARD, ROW_HEIGHT_SLIM};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadRowVariant {
    Card,
    Slim,
}

pub fn sidebar_thread_row(
    thread: &DemoThread,
    variant: ThreadRowVariant,
    theme: &OpenCoreTheme,
) -> AnyElement {
    match variant {
        ThreadRowVariant::Card => card_row(thread, theme).into_any_element(),
        ThreadRowVariant::Slim => slim_row(thread, theme).into_any_element(),
    }
}

fn row_surface(
    thread: &DemoThread,
    theme: &OpenCoreTheme,
    height: f32,
    content: impl IntoElement,
) -> impl IntoElement {
    let (bg, text) = row_colors(thread, theme);
    let border = theme.border_token(BorderToken::Default);

    let row = div()
        .id(format!("left-sidebar-thread-{}", thread.id))
        .w_full()
        .min_w_0()
        .h(px(height))
        .flex()
        .items_center()
        .overflow_hidden()
        .rounded(px(0.))
        .bg(bg)
        .text_color(text);

    if thread.is_active {
        row.border_1().border_color(border).child(content)
    } else {
        row.child(content)
    }
}

fn row_colors(thread: &DemoThread, theme: &OpenCoreTheme) -> (gpui::Hsla, gpui::Hsla) {
    if thread.is_active {
        (
            theme.surface(BackgroundToken::Tertiary),
            theme.foreground(ForegroundToken::Primary),
        )
    } else {
        (
            theme.surface(BackgroundToken::Primary),
            theme.foreground(ForegroundToken::Primary),
        )
    }
}

fn card_row(thread: &DemoThread, theme: &OpenCoreTheme) -> impl IntoElement {
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();

    row_surface(
        thread,
        theme,
        ROW_HEIGHT_CARD,
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
                                    .font_family(mono.clone())
                                    .text_size(px(TypeRole::LabelMd.size()))
                                    .line_height(relative(TypeRole::LabelMd.line_height()))
                                    .text_color(secondary)
                                    .child(thread.project_title),
                            )
                            .child(status_or_time(thread, theme, true)),
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
                            .text_color(theme.foreground(ForegroundToken::Primary))
                            .child(thread.title),
                    )
                    .child(branch_meta_row(thread, theme)),
            ),
    )
}

fn slim_row(thread: &DemoThread, theme: &OpenCoreTheme) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();

    row_surface(
        thread,
        theme,
        ROW_HEIGHT_SLIM,
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
                    .child(thread.title),
            )
            .child(status_or_time(thread, theme, false)),
    )
}

fn status_or_time(
    thread: &DemoThread,
    theme: &OpenCoreTheme,
    show_status: bool,
) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();

    if show_status && thread.status_label.is_some() {
        h_flex()
            .flex_shrink_0()
            .gap(px(4.))
            .items_center()
            .child(
                Icon::new(IconName::LoaderCircle)
                    .text_color(muted)
                    .small()
                    .flex_shrink_0(),
            )
            .child(
                div()
                    .font_family(mono)
                    .text_size(px(TypeRole::LabelMd.size()))
                    .text_color(muted)
                    .child(thread.status_label.unwrap_or_default()),
            )
    } else {
        div()
            .flex_shrink_0()
            .font_family(mono)
            .text_size(px(TypeRole::LabelMd.size()))
            .text_color(muted)
            .child(thread.time_label)
    }
}

fn branch_meta_row(thread: &DemoThread, theme: &OpenCoreTheme) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let green = gpui::rgb(SUCCESS_GREEN).into();
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
        .children(pr_badge(thread, muted, mono.clone()))
        .children(diff_stats(thread, green, red, mono))
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
