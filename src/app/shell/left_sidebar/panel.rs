//! Left dock panel — thread sidebar (demo data).

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, ScrollAnchor, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};
use gpui_component::{
    dock::{Panel, PanelEvent},
    input::{InputEvent, InputState},
    v_flex,
};

use crate::app::shell::workspace_theme::WorkspaceTheme;
use crate::shared::theme::{
    BackgroundToken, BorderToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::chrome::sidebar_chrome_footer;
use super::content::{
    DraftRowDragActions, PinnedDragState, PinnedRowDragUi, ShelfTone, ThreadRowActions,
    ThreadRowVariant, sidebar_add_project_button,
    sidebar_draft_row, sidebar_empty_state, sidebar_project_scope_row,
    sidebar_search_result_row, sidebar_search_row, sidebar_shelf_body, sidebar_section_header,
    sidebar_shelf_header,
    sidebar_show_more_button, sidebar_thread_row,
};
use super::demo_data::{DEMO_DRAFT, DEMO_THREADS};
use super::shelf_tween::{
    eval_shelf_tween, shelf_content_height_card, shelf_content_height_slim, shelf_expand_progress,
    ShelfHeightTween,
};
use super::state::{demo_draft, FooterBackContext, RevealShelf, SidebarViewModel};
use super::tokens::{CONTENT_INSET, DOCK_RESIZE_GUTTER};

const PANEL_TITLE: &str = "THREADS";

pub struct LeftSidebarPanel {
    focus_handle: FocusHandle,
    theme: WorkspaceTheme,
    search: Entity<InputState>,
    rename_input: Entity<InputState>,
    view: SidebarViewModel,
    thread_list_scroll_handle: ScrollHandle,
    thread_scroll_anchors: HashMap<String, ScrollAnchor>,
    pinned_height_tween: Option<ShelfHeightTween>,
    settled_height_tween: Option<ShelfHeightTween>,
    archived_height_tween: Option<ShelfHeightTween>,
    pinned_dragging_id: Option<String>,
    pinned_drop_target: Option<(String, bool)>,
    /// Follow shelf expand with scroll-to-bottom until the height tween finishes.
    pending_scroll_to_bottom: bool,
    _search_subscription: Subscription,
    _rename_subscription: Subscription,
}

impl LeftSidebarPanel {
    pub fn new(window: &mut Window, theme: WorkspaceTheme, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        let rename_input = cx.new(|cx| InputState::new(window, cx).placeholder("Thread name"));
        let _panel = cx.entity();
        let search_subscription = cx.subscribe_in(&search, window, move |this, _, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                let query = this.search.read(cx).value().to_string();
                if this.view.search_query != query {
                    this.view.search_query = query;
                    cx.notify();
                }
            }
        });
        let rename_subscription = cx.subscribe_in(
            &rename_input,
            window,
            move |this, _, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.commit_rename(window, cx);
                }
            },
        );

        Self {
            focus_handle: cx.focus_handle(),
            theme,
            search,
            rename_input,
            view: SidebarViewModel::new("active-1"),
            thread_list_scroll_handle: ScrollHandle::new(),
            thread_scroll_anchors: HashMap::new(),
            pinned_height_tween: None,
            settled_height_tween: None,
            archived_height_tween: None,
            pinned_dragging_id: None,
            pinned_drop_target: None,
            pending_scroll_to_bottom: false,
            _search_subscription: search_subscription,
            _rename_subscription: rename_subscription,
        }
    }

    fn shelf_clip_height(
        tween: Option<ShelfHeightTween>,
        expanded: bool,
        full_height: f32,
        now: Instant,
    ) -> f32 {
        let target = if expanded { full_height } else { 0.0 };
        eval_shelf_tween(tween, target, now)
    }

    fn shelf_show_content(
        tween: Option<ShelfHeightTween>,
        expanded: bool,
        clip_height: f32,
        now: Instant,
    ) -> bool {
        expanded || clip_height > f32::EPSILON || tween.is_some_and(|t| t.is_active(now))
    }

    fn finish_shelf_tween(tween_slot: &mut Option<ShelfHeightTween>, now: Instant) -> bool {
        let Some(tween) = *tween_slot else {
            return false;
        };
        if tween.is_active(now) {
            return true;
        }
        *tween_slot = None;
        false
    }

    fn tick_shelf_tweens(&mut self, now: Instant) -> bool {
        Self::finish_shelf_tween(&mut self.pinned_height_tween, now)
            | Self::finish_shelf_tween(&mut self.settled_height_tween, now)
            | Self::finish_shelf_tween(&mut self.archived_height_tween, now)
    }

    fn shelf_expand_tween_active(&self, now: Instant) -> bool {
        self.settled_height_tween.is_some_and(|t| t.is_active(now))
            || self.archived_height_tween.is_some_and(|t| t.is_active(now))
    }

    fn mark_scroll_to_bottom_on_expand(&mut self) {
        self.pending_scroll_to_bottom = true;
    }

    fn maintain_scroll_to_bottom_on_expand(
        &mut self,
        now: Instant,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.pending_scroll_to_bottom {
            return;
        }
        self.thread_list_scroll_handle.scroll_to_bottom();
        if !self.shelf_expand_tween_active(now) {
            self.pending_scroll_to_bottom = false;
            self.defer_finalize_scroll_to_bottom(window, cx);
        }
    }

    fn defer_finalize_scroll_to_bottom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let panel = cx.entity();
        window.defer(cx, move |window, cx| {
            panel.update(cx, |panel, cx| {
                let max = panel.thread_list_scroll_handle.max_offset();
                panel
                    .thread_list_scroll_handle
                    .set_offset(gpui::Point::new(px(0.), -max.y));
                panel.thread_list_scroll_handle.scroll_to_bottom();
                cx.notify();
            });
            let panel = panel.clone();
            window.defer(cx, move |_, cx| {
                panel.update(cx, |panel, cx| {
                    let max = panel.thread_list_scroll_handle.max_offset();
                    panel
                        .thread_list_scroll_handle
                        .set_offset(gpui::Point::new(px(0.), -max.y));
                    cx.notify();
                });
            });
        });
    }

    fn start_shelf_height_animation(
        tween_slot: &mut Option<ShelfHeightTween>,
        expanded_flag: &mut bool,
        expanded: bool,
        current_tween: Option<ShelfHeightTween>,
        full_height: f32,
        now: Instant,
    ) -> bool {
        let from = Self::shelf_clip_height(current_tween, *expanded_flag, full_height, now);
        if *expanded_flag == expanded
            && !current_tween.is_some_and(|t| t.is_active(now))
            && if expanded {
                (from - full_height).abs() <= 1.0
            } else {
                from <= f32::EPSILON
            }
        {
            return false;
        }
        *expanded_flag = expanded;
        let to = if expanded { full_height } else { 0.0 };
        *tween_slot = Some(ShelfHeightTween::new(from, to, now));
        true
    }

    fn expand_reveal_shelf(&mut self, shelf: RevealShelf, now: Instant, cx: &mut Context<Self>) {
        let changed = match shelf {
            RevealShelf::Pinned => {
                let full_height = self.pinned_shelf_full_height();
                let current_tween = self.pinned_height_tween;
                Self::start_shelf_height_animation(
                    &mut self.pinned_height_tween,
                    &mut self.view.pinned_expanded,
                    true,
                    current_tween,
                    full_height,
                    now,
                )
            }
            RevealShelf::Settled => {
                let full_height = self.settled_shelf_full_height();
                let current_tween = self.settled_height_tween;
                Self::start_shelf_height_animation(
                    &mut self.settled_height_tween,
                    &mut self.view.settled_expanded,
                    true,
                    current_tween,
                    full_height,
                    now,
                )
            }
            RevealShelf::Archived => {
                let full_height = self.archived_shelf_full_height();
                let current_tween = self.archived_height_tween;
                Self::start_shelf_height_animation(
                    &mut self.archived_height_tween,
                    &mut self.view.archived_expanded,
                    true,
                    current_tween,
                    full_height,
                    now,
                )
            }
        };
        if changed {
            if matches!(shelf, RevealShelf::Settled | RevealShelf::Archived) {
                self.mark_scroll_to_bottom_on_expand();
            }
            cx.notify();
        }
    }

    fn activate_from_search_animated(
        &mut self,
        thread_id: &str,
        now: Instant,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shelf = self.view.reveal_shelf_target(thread_id);
        self.view.prepare_thread_reveal(thread_id);
        if let Some(shelf) = shelf {
            self.expand_reveal_shelf(shelf, now, cx);
        }
        self.view.activate_thread(thread_id);
        self.view.clear_search();
        self.clear_search_input(window, cx);
        cx.notify();
    }

    fn pinned_shelf_full_height(&self) -> f32 {
        shelf_content_height_card(self.view.pinned_threads().len())
    }

    fn toggle_pinned_shelf(&mut self, now: Instant, cx: &mut Context<Self>) {
        let full_height = self.pinned_shelf_full_height();
        let target = !self.view.pinned_expanded;
        let current_tween = self.pinned_height_tween;
        if Self::start_shelf_height_animation(
            &mut self.pinned_height_tween,
            &mut self.view.pinned_expanded,
            target,
            current_tween,
            full_height,
            now,
        ) {
            cx.notify();
        }
    }

    fn settled_shelf_full_height(&self) -> f32 {
        let show_more = self.view.settled_expanded && self.view.settled_has_more();
        shelf_content_height_slim(self.view.settled_visible().len(), show_more)
    }

    fn archived_shelf_full_height(&self) -> f32 {
        shelf_content_height_slim(self.view.archived_threads().len(), false)
    }

    fn toggle_settled_shelf(&mut self, now: Instant, cx: &mut Context<Self>) {
        let full_height = self.settled_shelf_full_height();
        let target = !self.view.settled_expanded;
        let current_tween = self.settled_height_tween;
        if Self::start_shelf_height_animation(
            &mut self.settled_height_tween,
            &mut self.view.settled_expanded,
            target,
            current_tween,
            full_height,
            now,
        ) {
            if target {
                self.mark_scroll_to_bottom_on_expand();
            }
            cx.notify();
        }
    }

    fn toggle_archived_shelf(&mut self, now: Instant, cx: &mut Context<Self>) {
        let full_height = self.archived_shelf_full_height();
        let target = !self.view.archived_expanded;
        let current_tween = self.archived_height_tween;
        if Self::start_shelf_height_animation(
            &mut self.archived_height_tween,
            &mut self.view.archived_expanded,
            target,
            current_tween,
            full_height,
            now,
        ) {
            if target {
                self.mark_scroll_to_bottom_on_expand();
            }
            cx.notify();
        }
    }

    fn pinned_drag_state(&self) -> PinnedDragState {
        PinnedDragState {
            dragging_id: self.pinned_dragging_id.clone(),
            drop_target: self.pinned_drop_target.clone(),
        }
    }

    fn sync_search_query(&mut self, cx: &mut App) {
        let query = self.search.read(cx).value().to_string();
        if self.view.search_query != query {
            self.view.search_query = query;
        }
    }

    fn thread_actions(&self, cx: &mut Context<Self>) -> ThreadRowActions {
        let panel = cx.entity();
        ThreadRowActions {
            on_activate: Rc::new({
                let panel = panel.clone();
                move |id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.activate_thread(&id);
                        cx.notify();
                    });
                }
            }),
            on_select: Rc::new({
                let panel = panel.clone();
                move |id: String, range, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        if range {
                            panel.view.toggle_thread_selection(&id, true);
                        } else {
                            panel.view.toggle_thread_selection(&id, false);
                        }
                        cx.notify();
                    });
                }
            }),
            on_hover: Rc::new({
                let panel = panel.clone();
                move |id: Option<String>, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.hovered_thread_id = id;
                        cx.notify();
                    });
                }
            }),
            on_move_thread: Rc::new({
                let panel = panel.clone();
                move |id: String, delta, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.move_thread(&id, delta);
                        cx.notify();
                    });
                }
            }),
            on_pin: Rc::new({
                let panel = panel.clone();
                move |id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        let now = Instant::now();
                        panel.view.pin_thread(&id);
                        panel.expand_reveal_shelf(RevealShelf::Pinned, now, cx);
                    });
                }
            }),
            on_unpin: Rc::new({
                let panel = panel.clone();
                move |id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.unpin_thread(&id);
                        cx.notify();
                    });
                }
            }),
            on_settle: Rc::new({
                let panel = panel.clone();
                move |id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        let now = Instant::now();
                        panel.view.settle_thread(&id);
                        panel.expand_reveal_shelf(RevealShelf::Settled, now, cx);
                    });
                }
            }),
            on_unsettle: Rc::new({
                let panel = panel.clone();
                move |id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.unsettle_thread(&id);
                        cx.notify();
                    });
                }
            }),
            on_rename: Rc::new({
                let panel = panel.clone();
                move |id: String, window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.begin_rename_thread(&id, window, cx);
                    });
                }
            }),
            on_archive: Rc::new({
                let panel = panel.clone();
                move |id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        let now = Instant::now();
                        panel.view.archive_thread(&id);
                        panel.expand_reveal_shelf(RevealShelf::Archived, now, cx);
                    });
                }
            }),
            on_unarchive: Rc::new({
                let panel = panel.clone();
                move |id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.unarchive_thread(&id);
                        cx.notify();
                    });
                }
            }),
            on_pinned_drag_start: Rc::new({
                let panel = panel.clone();
                move |id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.pinned_dragging_id = Some(id);
                        panel.pinned_drop_target = None;
                        cx.notify();
                    });
                }
            }),
            on_pinned_drag_over: Rc::new({
                let panel = panel.clone();
                move |target_id: String, insert_after: bool, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        let dragged_id = panel.pinned_dragging_id.clone();
                        if let Some(dragged_id) = dragged_id {
                            if !panel.view.can_reorder_threads(&dragged_id, &target_id) {
                                if panel.pinned_drop_target.is_some() {
                                    panel.pinned_drop_target = None;
                                    cx.notify();
                                }
                                return;
                            }
                        }
                        let changed = panel.pinned_drop_target.as_ref()
                            != Some(&(target_id.clone(), insert_after));
                        if changed {
                            panel.pinned_drop_target = Some((target_id, insert_after));
                            cx.notify();
                        }
                    });
                }
            }),
            on_pinned_drop: Rc::new({
                let panel = panel.clone();
                move |dragged_id: String, target_id: String, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        if !panel.view.can_reorder_threads(&dragged_id, &target_id) {
                            panel.pinned_dragging_id = None;
                            panel.pinned_drop_target = None;
                            cx.notify();
                            return;
                        }
                        let insert_after = panel
                            .pinned_drop_target
                            .as_ref()
                            .filter(|(id, _)| id == &target_id)
                            .map(|(_, after)| *after)
                            .unwrap_or(false);
                        panel.view.reorder_thread(&dragged_id, &target_id, insert_after);
                        panel.pinned_dragging_id = None;
                        panel.pinned_drop_target = None;
                        cx.notify();
                    });
                }
            }),
        }
    }

    fn clear_search_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |search, cx| {
            search.set_value("", window, cx);
        });
        self.view.clear_search();
    }

    fn thread_scroll_anchor(
        anchors: &mut HashMap<String, ScrollAnchor>,
        scroll_handle: &ScrollHandle,
        thread_id: &str,
    ) -> ScrollAnchor {
        anchors
            .entry(thread_id.to_string())
            .or_insert_with(|| ScrollAnchor::for_handle(scroll_handle.clone()))
            .clone()
    }

    fn scroll_to_thread(&self, thread_id: &str, window: &mut Window, _cx: &mut App) {
        if let Some(anchor) = self.thread_scroll_anchors.get(thread_id) {
            anchor.scroll_to(window, _cx);
        }
    }

    fn begin_rename_thread(&mut self, thread_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let thread = super::demo_data::DEMO_THREADS.iter().find(|t| t.id == thread_id);
        if thread.is_none() {
            return;
        }
        let title = self.view.renaming_title(thread.unwrap());
        self.view.begin_rename(thread_id);
        self.rename_input.update(cx, |input, cx| {
            input.set_value(title, window, cx);
        });
        let panel = cx.entity();
        let thread_id = thread_id.to_string();
        window.defer(cx, move |window, cx| {
            panel.update(cx, |panel, cx| {
                if panel.view.renaming_thread_id.as_deref() != Some(&thread_id) {
                    return;
                }
                panel.rename_input.update(cx, |input, cx| {
                    input.focus(window, cx);
                });
            });
        });
        cx.notify();
    }

    fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(thread_id) = self.view.renaming_thread_id.clone() {
            let title = self.rename_input.read(cx).value().to_string();
            self.view.commit_rename(&thread_id, title);
            cx.notify();
            let _ = window;
        }
    }
}

impl EventEmitter<PanelEvent> for LeftSidebarPanel {}

impl Focusable for LeftSidebarPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for LeftSidebarPanel {
    fn panel_name(&self) -> &'static str {
        "left-stub"
    }

    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .font_family(mono_family())
            .text_size(px(TypeRole::MonoSm.size()))
            .child(PANEL_TITLE)
    }

    fn tab_name(&self, _: &App) -> Option<gpui::SharedString> {
        Some(PANEL_TITLE.into())
    }

    fn closable(&self, _: &App) -> bool {
        false
    }

    fn zoomable(&self, _: &App) -> Option<gpui_component::dock::PanelControl> {
        None
    }
}

impl Render for LeftSidebarPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_search_query(cx);
        let now = Instant::now();
        let needs_frame = self.tick_shelf_tweens(now);
        self.maintain_scroll_to_bottom_on_expand(now, window, cx);
        if self.pinned_dragging_id.is_some() && !cx.has_active_drag() {
            self.pinned_dragging_id = None;
            self.pinned_drop_target = None;
        }
        if needs_frame {
            cx.defer_in(window, |_, _, cx| cx.notify());
        }
        let theme = self.theme.get();
        let page = theme.surface(BackgroundToken::Primary);
        let border = theme.border_token(BorderToken::Default);
        let view = &self.view;
        let panel = cx.entity();
        let actions = self.thread_actions(cx);
        let rename_input = &self.rename_input;
        let drag_state = self.pinned_drag_state();
        let thread_list_el = thread_list(
            self.pinned_height_tween,
            self.settled_height_tween,
            self.archived_height_tween,
            drag_state,
            view,
            &theme,
            &actions,
            rename_input,
            panel.clone(),
            now,
            &self.thread_list_scroll_handle,
            &mut self.thread_scroll_anchors,
            window,
            cx,
        );

        div()
            .id("left-sidebar-panel")
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(page)
            .track_focus(&self.focus_handle)
            .child(sidebar_fixed_controls(
                &self.search,
                view,
                &theme,
                panel.clone(),
            ))
            .child(
                div()
                    .id("left-sidebar-scroll")
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .track_scroll(&self.thread_list_scroll_handle)
                    .child(thread_list_el),
            )
            .child(sidebar_chrome_footer(
                view.footer_mode,
                view.show_update_pill,
                &theme,
                {
                    let panel = panel.clone();
                    move |window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.close_footer_utility();
                            cx.notify();
                        });
                        let _ = window;
                    }
                },
                {
                    let panel = panel.clone();
                    move |window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.open_footer_utility(FooterBackContext::Settings);
                            cx.notify();
                        });
                        let _ = window;
                    }
                },
                {
                    let panel = panel.clone();
                    move |window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.open_footer_utility(FooterBackContext::PullRequests);
                            cx.notify();
                        });
                        let _ = window;
                    }
                },
                {
                    let panel = panel.clone();
                    move |window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.open_footer_utility(FooterBackContext::Usage);
                            cx.notify();
                        });
                        let _ = window;
                    }
                },
            ))
            .border_r_1()
            .border_color(border)
    }
}

fn sidebar_fixed_controls(
    search: &Entity<InputState>,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    panel: Entity<LeftSidebarPanel>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .flex_shrink_0()
        .gap(px(SpacingToken::S1.value()))
        .pt(px(CONTENT_INSET))
        .pb(px(CONTENT_INSET))
        .pl(px(CONTENT_INSET))
        .pr(px(CONTENT_INSET + DOCK_RESIZE_GUTTER))
        .border_b_1()
        .border_color(theme.border_token(BorderToken::Default))
        .child(sidebar_search_row(
            search,
            view.search_query.as_str(),
            theme,
            {
                let panel = panel.clone();
                move |window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.search.update(cx, |search, cx| {
                            search.set_value("", window, cx);
                        });
                        panel.view.search_query.clear();
                        cx.notify();
                    });
                }
            },
            {
                let panel = panel.clone();
                move |_window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.show_draft();
                            cx.notify();
                        });
                }
            },
        ))
        .child(sidebar_project_scope_row(
            view.scoped_label(),
            theme,
            {
                let panel = panel.clone();
                move |scope, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.project_scope = scope;
                        cx.notify();
                    });
                }
            },
            {
                let panel = panel.clone();
                move |_window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.show_draft();
                            cx.notify();
                        });
                }
            },
        ))
}

fn thread_row_with_scroll(
    thread: &super::demo_data::DemoThread,
    variant: ThreadRowVariant,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &ThreadRowActions,
    rename_input: &Entity<InputState>,
    row_drag: Option<PinnedRowDragUi>,
    scroll_handle: &ScrollHandle,
    scroll_anchors: &mut HashMap<String, ScrollAnchor>,
) -> gpui::AnyElement {
    sidebar_thread_row(
        thread,
        variant,
        view,
        theme,
        actions,
        Some(rename_input),
        row_drag,
        Some(LeftSidebarPanel::thread_scroll_anchor(
            scroll_anchors,
            scroll_handle,
            thread.id,
        )),
    )
}

fn thread_list(
    pinned_height_tween: Option<ShelfHeightTween>,
    settled_height_tween: Option<ShelfHeightTween>,
    archived_height_tween: Option<ShelfHeightTween>,
    drag_state: PinnedDragState,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &ThreadRowActions,
    rename_input: &Entity<InputState>,
    panel_entity: Entity<LeftSidebarPanel>,
    now: Instant,
    scroll_handle: &ScrollHandle,
    scroll_anchors: &mut HashMap<String, ScrollAnchor>,
    _window: &mut Window,
    _cx: &mut Context<LeftSidebarPanel>,
) -> gpui::AnyElement {
    if view.is_searching() {
        return search_results_list(view, theme, panel_entity).into_any_element();
    }

    let pinned_threads = view.pinned_threads();
    let pinned_count = pinned_threads.len();
    let settled_visible = view.settled_visible();
    let archived_threads = view.archived_threads();
    let settled_count = view.settled_threads().len();
    let settled_visible_count = settled_visible.len();
    let archived_count = archived_threads.len();
    let settled_show_more = view.settled_expanded && view.settled_has_more();
    let pinned_full_height = shelf_content_height_card(pinned_count);
    let settled_full_height =
        shelf_content_height_slim(settled_visible_count, settled_show_more);
    let archived_full_height = shelf_content_height_slim(archived_count, false);
    let pinned_clip = LeftSidebarPanel::shelf_clip_height(
        pinned_height_tween,
        view.pinned_expanded,
        pinned_full_height,
        now,
    );
    let settled_clip = LeftSidebarPanel::shelf_clip_height(
        settled_height_tween,
        view.settled_expanded,
        settled_full_height,
        now,
    );
    let archived_clip = LeftSidebarPanel::shelf_clip_height(
        archived_height_tween,
        view.archived_expanded,
        archived_full_height,
        now,
    );
    let pinned_show = LeftSidebarPanel::shelf_show_content(
        pinned_height_tween,
        view.pinned_expanded,
        pinned_clip,
        now,
    );
    let settled_show = LeftSidebarPanel::shelf_show_content(
        settled_height_tween,
        view.settled_expanded,
        settled_clip,
        now,
    );
    let archived_show = LeftSidebarPanel::shelf_show_content(
        archived_height_tween,
        view.archived_expanded,
        archived_clip,
        now,
    );
    let active_section_ids = view.active_section_ids();
    let show_empty = active_section_ids.is_empty()
        && pinned_count == 0
        && settled_count == 0
        && archived_count == 0;
    let draft_drag = DraftRowDragActions {
        on_drag_start: actions.on_pinned_drag_start.clone(),
        on_drag_over: actions.on_pinned_drag_over.clone(),
        on_drop: actions.on_pinned_drop.clone(),
    };
    let pinned_label = view.pinned_label();
    let settled_label = view.settled_label();
    let archived_label = view.archived_label();

    v_flex()
        .id("left-sidebar-thread-list")
        .w_full()
        .min_w_0()
        .gap(px(1.))
        .pl(px(CONTENT_INSET))
        .pr(px(CONTENT_INSET + DOCK_RESIZE_GUTTER))
        .pb(px(CONTENT_INSET))
        .children(if pinned_count > 0 {
            Some(sidebar_shelf_header(
                &pinned_label,
                shelf_expand_progress(pinned_clip, pinned_full_height),
                ShelfTone::Pinned,
                theme,
                {
                    let panel = panel_entity.clone();
                    move |_window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.toggle_pinned_shelf(Instant::now(), cx);
                        });
                    }
                },
            ))
        } else {
            None
        })
        .child(
            sidebar_shelf_body(
                "pinned",
                pinned_clip,
                pinned_show,
                pinned_threads
                    .iter()
                    .map(|thread| {
                        let row_drag = Some(drag_state.for_thread(thread.id));
                        thread_row_with_scroll(
                            thread,
                            ThreadRowVariant::Card,
                            view,
                            theme,
                            actions,
                            rename_input,
                            row_drag,
                            scroll_handle,
                            scroll_anchors,
                        )
                        .into_any_element()
                    })
                    .collect::<Vec<_>>(),
            ),
        )
        .child(sidebar_section_header("Active", ShelfTone::Active, theme))
        .children(
            active_section_rows(
                view,
                theme,
                actions,
                &draft_drag,
                &drag_state,
                rename_input,
                panel_entity.clone(),
                scroll_handle,
                scroll_anchors,
            ),
        )
        .children(if show_empty {
            Some(
                v_flex()
                    .child(sidebar_empty_state("No threads yet", theme))
                    .child(sidebar_add_project_button(theme, {
                        let panel = panel_entity.clone();
                        move |_window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.show_draft();
                            cx.notify();
                        });
                        }
                    }))
                    .into_any_element(),
            )
        } else {
            None
        })
        .children(if settled_count > 0 {
            Some(sidebar_shelf_header(
                &settled_label,
                shelf_expand_progress(settled_clip, settled_full_height),
                ShelfTone::Settled,
                theme,
                {
                    let panel = panel_entity.clone();
                    move |_window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.toggle_settled_shelf(Instant::now(), cx);
                        });
                    }
                },
            ))
        } else {
            None
        })
        .child(
            sidebar_shelf_body(
                "settled",
                settled_clip,
                settled_show,
                settled_visible
                    .iter()
                    .map(|thread| {
                        let row_drag = Some(drag_state.for_thread(thread.id));
                        thread_row_with_scroll(
                            thread,
                            ThreadRowVariant::Slim,
                            view,
                            theme,
                            actions,
                            rename_input,
                            row_drag,
                            scroll_handle,
                            scroll_anchors,
                        )
                        .into_any_element()
                    })
                    .chain(
                        settled_show_more
                            .then(|| {
                                sidebar_show_more_button(theme, {
                                    let panel = panel_entity.clone();
                                    move |_window, cx| {
                                        panel.update(cx, |panel, cx| {
                                            panel.view.show_more_settled();
                                            cx.notify();
                                        });
                                    }
                                })
                                .into_any_element()
                            })
                            .into_iter(),
                    )
                    .collect::<Vec<_>>(),
            ),
        )
        .children(if archived_count > 0 {
            Some(sidebar_shelf_header(
                &archived_label,
                shelf_expand_progress(archived_clip, archived_full_height),
                ShelfTone::Archived,
                theme,
                {
                    let panel = panel_entity.clone();
                    move |_window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.toggle_archived_shelf(Instant::now(), cx);
                        });
                    }
                },
            ))
        } else {
            None
        })
        .child(
            sidebar_shelf_body(
                "archived",
                archived_clip,
                archived_show,
                archived_threads
                    .iter()
                    .map(|thread| {
                        let row_drag = Some(drag_state.for_thread(thread.id));
                        thread_row_with_scroll(
                            thread,
                            ThreadRowVariant::Slim,
                            view,
                            theme,
                            actions,
                            rename_input,
                            row_drag,
                            scroll_handle,
                            scroll_anchors,
                        )
                        .into_any_element()
                    })
                    .collect::<Vec<_>>(),
            ),
        )
        .into_any_element()
}

fn search_results_list(
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    panel: Entity<LeftSidebarPanel>,
) -> impl IntoElement {
    let results = view.search_results();
    let panel_for_activate = panel.clone();
    v_flex()
        .id("left-sidebar-search-results")
        .w_full()
        .min_w_0()
        .gap(px(1.))
        .pl(px(CONTENT_INSET))
        .pr(px(CONTENT_INSET + DOCK_RESIZE_GUTTER))
        .pb(px(CONTENT_INSET))
        .children(if results.is_empty() {
            Some(sidebar_empty_state("No threads found", theme))
        } else {
            None
        })
        .children(
            results
                .iter()
                .map(|thread| {
                    let panel = panel_for_activate.clone();
                    sidebar_search_result_row(
                        thread,
                        view,
                        theme,
                        move |id, window, cx| {
                            let panel_defer = panel.clone();
                            let scroll_id = id.clone();
                            panel.update(cx, |panel, cx| {
                                panel.activate_from_search_animated(&id, Instant::now(), window, cx);
                            });
                            window.defer(cx, move |window, cx| {
                                panel_defer.update(cx, |panel, cx| {
                                    panel.scroll_to_thread(&scroll_id, window, cx);
                                    cx.notify();
                                });
                            });
                        },
                    )
                })
                .collect::<Vec<_>>(),
        )
}

fn active_section_rows(
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &ThreadRowActions,
    draft_drag: &DraftRowDragActions,
    drag_state: &PinnedDragState,
    rename_input: &Entity<InputState>,
    panel_entity: Entity<LeftSidebarPanel>,
    scroll_handle: &ScrollHandle,
    scroll_anchors: &mut HashMap<String, ScrollAnchor>,
) -> Vec<gpui::AnyElement> {
    view.active_section_ids()
        .iter()
        .map(|id| {
            if id == DEMO_DRAFT.id {
                let row_drag = drag_state.for_thread(DEMO_DRAFT.id);
                sidebar_draft_row(
                    demo_draft(),
                    view.is_draft_active(),
                    view.is_draft_selected(),
                    theme,
                    row_drag,
                    draft_drag,
                    {
                        let panel = panel_entity.clone();
                        move |_window, cx| {
                            panel.update(cx, |panel, cx| {
                                panel.view.activate_thread(DEMO_DRAFT.id);
                                cx.notify();
                            });
                        }
                    },
                    {
                        let panel = panel_entity.clone();
                        move |range, _window, cx| {
                            panel.update(cx, |panel, cx| {
                                if range {
                                    panel.view.toggle_thread_selection(DEMO_DRAFT.id, true);
                                } else {
                                    panel.view.toggle_thread_selection(DEMO_DRAFT.id, false);
                                }
                                cx.notify();
                            });
                        }
                    },
                    {
                        let panel = panel_entity.clone();
                        move |_window, cx| {
                            panel.update(cx, |panel, cx| {
                                panel.view.discard_draft();
                                cx.notify();
                            });
                        }
                    },
                )
                .into_any_element()
            } else {
                let thread = DEMO_THREADS
                    .iter()
                    .find(|t| t.id == id)
                    .expect("active section id must map to demo thread");
                let row_drag = Some(drag_state.for_thread(thread.id));
                thread_row_with_scroll(
                    thread,
                    ThreadRowVariant::Card,
                    view,
                    theme,
                    actions,
                    rename_input,
                    row_drag,
                    scroll_handle,
                    scroll_anchors,
                )
            }
        })
        .collect()
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
