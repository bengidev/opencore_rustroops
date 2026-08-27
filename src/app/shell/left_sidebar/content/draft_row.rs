//! Draft row for unsent composer sessions — matches active card row interaction.

use gpui::{
    App, AppContext, ClickEvent, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder as _, px, relative,
};
use gpui_component::{
    Icon, IconName,
    button::{Button, ButtonRounded, ButtonVariants as _},
    h_flex, v_flex,
};

use crate::shared::theme::{BackgroundToken, ForegroundToken, OpenCoreTheme, TypeRole};

use super::super::demo_data::{DEMO_PROJECTS, DemoDraft};
use super::super::surfaces::{
    draft_bg, draft_bg_hover, project_favicon_color, row_active_bg, row_hover_bg, row_selected_bg,
};
use super::super::tokens::{FAVICON_SIZE, ROW_CONTENT_INSET, ROW_HEIGHT_CARD};
use super::callbacks::{ThreadDragOverCallback, ThreadDropCallback, ThreadIdCallback};
use super::pinned_drag::{PinnedRowDragUi, PinnedThreadDrag, ThreadDragScope};

pub struct DraftRowDragActions {
    pub on_drag_start: ThreadIdCallback,
    pub on_drag_over: ThreadDragOverCallback,
    pub on_drop: ThreadDropCallback,
}

pub fn sidebar_draft_row(
    draft: &DemoDraft,
    is_active: bool,
    is_selected: bool,
    theme: &OpenCoreTheme,
    row_drag: PinnedRowDragUi,
    drag_actions: &DraftRowDragActions,
    on_activate: impl Fn(&mut Window, &mut App) + 'static,
    on_select: impl Fn(bool, &mut Window, &mut App) + 'static,
    on_discard: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();
    let favicon_hue = DEMO_PROJECTS
        .iter()
        .find(|p| p.key == draft.project_key)
        .map(|p| p.favicon_hue)
        .unwrap_or(0x888888);

    let (bg, text) = if is_active {
        (row_active_bg(theme), primary)
    } else if is_selected {
        (row_selected_bg(theme), primary)
    } else {
        (draft_bg(theme), primary.alpha(0.9))
    };

    let draft_id = draft.id.to_string();
    let preview = draft.preview;
    let project_title = draft.project_title;
    let drop_line_color: gpui::Hsla = gpui::rgb(0x3B82F6).into();
    let drag_hover_bg = row_hover_bg(theme);
    let on_drag_start = drag_actions.on_drag_start.clone();
    let on_drag_over = drag_actions.on_drag_over.clone();
    let on_drop = drag_actions.on_drop.clone();
    let drag_scope = ThreadDragScope::Active;

    div()
        .id(format!("left-sidebar-draft-{}", draft.id))
        .w_full()
        .min_w_0()
        .relative()
        .flex()
        .h(px(ROW_HEIGHT_CARD))
        .items_start()
        .overflow_x_hidden()
        .bg(bg)
        .text_color(text)
        .cursor_pointer()
        .when(row_drag.drop_above, |row| {
            row.child(
                div()
                    .absolute()
                    .top(px(0.))
                    .left(px(0.))
                    .right(px(0.))
                    .h(px(2.))
                    .bg(drop_line_color),
            )
        })
        .when(row_drag.drop_below, |row| {
            row.child(
                div()
                    .absolute()
                    .bottom(px(0.))
                    .left(px(0.))
                    .right(px(0.))
                    .h(px(2.))
                    .bg(drop_line_color),
            )
        })
        .when(!is_active && !is_selected, |row| {
            row.hover(|style| style.bg(draft_bg_hover(theme)))
        })
        .when(row_drag.is_source, |row| row.opacity(0.45))
        .on_drag(
            PinnedThreadDrag {
                thread_id: draft_id.clone(),
                scope: drag_scope,
                title: preview.into(),
                preview_bg: theme.surface(BackgroundToken::Secondary),
                preview_text: primary,
            },
            {
                let on_drag_start = on_drag_start.clone();
                move |drag, _, window, cx| {
                    cx.stop_propagation();
                    on_drag_start(drag.thread_id.clone(), window, cx);
                    cx.new(|_| DraftDragPreview {
                        title: drag.title.clone(),
                        bg: drag.preview_bg,
                        text: drag.preview_text,
                    })
                }
            },
        )
        .can_drop({
            let self_id = draft_id.clone();
            move |value, _, _| {
                value
                    .downcast_ref::<PinnedThreadDrag>()
                    .is_some_and(|drag| {
                        drag.thread_id != self_id && drag.scope.allows_drop(drag_scope)
                    })
            }
        })
        .drag_over::<PinnedThreadDrag>({
            let self_id = draft_id.clone();
            move |style, drag, _, _| {
                if drag.thread_id == self_id || !drag.scope.allows_drop(drag_scope) {
                    return style;
                }
                style.bg(drag_hover_bg)
            }
        })
        .on_drag_move::<PinnedThreadDrag>({
            let target_id = draft_id.clone();
            let on_drag_over = on_drag_over.clone();
            move |event, window, cx| {
                let drag = event.drag(cx);
                if drag.thread_id == target_id || !drag.scope.allows_drop(drag_scope) {
                    return;
                }
                let pointer = event.event.position;
                let bounds = event.bounds;
                if pointer.x < bounds.origin.x
                    || pointer.x > bounds.origin.x + bounds.size.width
                    || pointer.y < bounds.origin.y
                    || pointer.y > bounds.origin.y + bounds.size.height
                {
                    return;
                }
                let midpoint = bounds.size.height / 2.0;
                let offset_y = pointer.y - bounds.origin.y;
                let insert_after = offset_y > midpoint;
                on_drag_over(target_id.clone(), insert_after, window, cx);
            }
        })
        .on_drop({
            let target_id = draft_id.clone();
            let on_drop = on_drop.clone();
            move |drag: &PinnedThreadDrag, window, cx| {
                if drag.thread_id == target_id || !drag.scope.allows_drop(drag_scope) {
                    return;
                }
                on_drop(drag.thread_id.clone(), target_id.clone(), window, cx);
            }
        })
        .on_click({
            let on_select = on_select;
            let on_activate = on_activate;
            move |event: &ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    return;
                }
                if event.modifiers().shift {
                    on_select(true, window, cx);
                } else {
                    on_activate(window, cx);
                }
            }
        })
        .child(
            div()
                .w_full()
                .min_w_0()
                .px(px(ROW_CONTENT_INSET))
                .py(px(6.))
                .child(
                    v_flex()
                        .w_full()
                        .min_w_0()
                        .gap(px(3.))
                        .child(
                            h_flex()
                                .w_full()
                                .min_w_0()
                                .items_center()
                                .gap(px(6.))
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
                                        .child(project_title),
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
                                .text_color(text)
                                .child(preview),
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

struct DraftDragPreview {
    title: SharedString,
    bg: gpui::Hsla,
    text: gpui::Hsla,
}

impl gpui::Render for DraftDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(240.))
            .px(px(10.))
            .py(px(6.))
            .bg(self.bg)
            .opacity(0.92)
            .shadow_md()
            .font_family(mono_family())
            .text_size(px(TypeRole::LabelMd.size()))
            .line_height(relative(TypeRole::LabelMd.line_height()))
            .text_color(self.text)
            .child(self.title.clone())
    }
}
