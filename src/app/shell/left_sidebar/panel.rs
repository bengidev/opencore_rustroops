//! Left dock panel — t3code thread sidebar interface (demo data).

use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Window, div, px,
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

use super::chrome::{sidebar_chrome_footer, sidebar_chrome_header};
use super::content::{
    ShelfTone, ThreadRowActions, ThreadRowVariant, sidebar_add_project_button,
    sidebar_draft_row, sidebar_empty_state, sidebar_pinned_divider, sidebar_project_scope_row,
    sidebar_search_result_row, sidebar_search_row, sidebar_shelf_header, sidebar_show_more_button,
    sidebar_thread_row,
};
use super::demo_data::DEMO_DRAFT;
use super::state::{demo_draft, FooterMode, SidebarViewModel};
use super::tokens::{CONTENT_INSET, DOCK_RESIZE_GUTTER};

const PANEL_TITLE: &str = "THREADS";

pub struct LeftSidebarPanel {
    focus_handle: FocusHandle,
    theme: WorkspaceTheme,
    search: Entity<InputState>,
    view: SidebarViewModel,
    _search_subscription: Subscription,
}

impl LeftSidebarPanel {
    pub fn new(window: &mut Window, theme: WorkspaceTheme, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        let panel = cx.entity();
        let search_subscription = cx.subscribe_in(&search, window, move |this, _, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                let query = this.search.read(cx).value().to_string();
                if this.view.search_query != query {
                    this.view.search_query = query;
                    cx.notify();
                }
            }
        });

        Self {
            focus_handle: cx.focus_handle(),
            theme,
            search,
            view: SidebarViewModel::new("active-1"),
            _search_subscription: search_subscription,
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
            on_move_pinned: Rc::new({
                let panel = panel.clone();
                move |id: String, delta, _window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.move_pinned(&id, delta);
                        cx.notify();
                    });
                }
            }),
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_search_query(cx);
        let theme = self.theme.get();
        let page = theme.surface(BackgroundToken::Primary);
        let border = theme.border_token(BorderToken::Default);
        let view = &self.view;
        let panel = cx.entity();
        let actions = self.thread_actions(cx);

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
            .child(sidebar_chrome_header(&theme))
            .child(fixed_controls(
                &self.search,
                view.search_query.as_str(),
                view.scoped_label(),
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
                    .child(thread_list(view, &theme, &actions, panel.clone(), cx)),
            )
            .child(sidebar_chrome_footer(
                view.footer_mode,
                view.show_update_pill,
                &theme,
                {
                    let panel = panel.clone();
                    move |window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.footer_mode = FooterMode::Utilities;
                            cx.notify();
                        });
                        let _ = window;
                    }
                },
                {
                    let panel = panel.clone();
                    move |window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.footer_mode = FooterMode::Back;
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

fn fixed_controls(
    search: &Entity<InputState>,
    query: &str,
    scoped_label: &str,
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
            query,
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
                        panel.view.draft_visible = true;
                        cx.notify();
                    });
                }
            },
        ))
        .child(sidebar_project_scope_row(
            scoped_label,
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
                        panel.view.draft_visible = true;
                        cx.notify();
                    });
                }
            },
        ))
}

fn thread_list(
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &ThreadRowActions,
    panel: Entity<LeftSidebarPanel>,
    cx: &mut Context<LeftSidebarPanel>,
) -> gpui::AnyElement {
    if view.is_searching() {
        return search_results_list(view, theme).into_any_element();
    }

    let pinned = view.pinned_threads();
    let active = view.active_threads();
    let snoozed_count = view.snoozed_threads().len();
    let settled_count = view.settled_threads().len();
    let show_empty = pinned.is_empty() && active.is_empty() && snoozed_count == 0 && settled_count == 0;
    let snoozed_label = view.snoozed_label();
    let settled_label = view.settled_label();

    v_flex()
        .id("left-sidebar-thread-list")
        .w_full()
        .min_w_0()
        .gap(px(1.))
        .pl(px(CONTENT_INSET))
        .pr(px(CONTENT_INSET + DOCK_RESIZE_GUTTER))
        .pb(px(CONTENT_INSET))
        .children(if view.draft_visible {
            Some(draft_block(view, theme, panel.clone(), cx))
        } else {
            None
        })
        .children(
            pinned
                .iter()
                .map(|thread| {
                    sidebar_thread_row(thread, ThreadRowVariant::Card, view, theme, actions)
                })
                .collect::<Vec<_>>(),
        )
        .children(if !pinned.is_empty() {
            Some(sidebar_pinned_divider(theme))
        } else {
            None
        })
        .children(
            active
                .iter()
                .map(|thread| {
                    sidebar_thread_row(thread, ThreadRowVariant::Card, view, theme, actions)
                })
                .collect::<Vec<_>>(),
        )
        .children(if show_empty {
            Some(
                v_flex()
                    .child(sidebar_empty_state("No threads yet", theme))
                    .child(sidebar_add_project_button(theme, {
                        let panel = panel.clone();
                        move |_window, cx| {
                            panel.update(cx, |panel, cx| {
                                panel.view.draft_visible = true;
                                cx.notify();
                            });
                        }
                    }))
                    .into_any_element(),
            )
        } else {
            None
        })
        .children(if snoozed_count > 0 {
            Some(sidebar_shelf_header(
                &snoozed_label,
                view.snoozed_expanded,
                ShelfTone::Snoozed,
                theme,
                {
                    let panel = panel.clone();
                    move |_window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.snoozed_expanded = !panel.view.snoozed_expanded;
                            cx.notify();
                        });
                    }
                },
            ))
        } else {
            None
        })
        .children(
            view.snoozed_expanded
                .then(|| {
                    view.snoozed_threads()
                        .iter()
                        .map(|thread| {
                            sidebar_thread_row(thread, ThreadRowVariant::Slim, view, theme, actions)
                        })
                        .collect::<Vec<_>>()
                })
                .into_iter()
                .flatten(),
        )
        .children(if settled_count > 0 {
            Some(sidebar_shelf_header(
                &settled_label,
                view.settled_expanded,
                ShelfTone::Settled,
                theme,
                {
                    let panel = panel.clone();
                    move |_window, cx| {
                        panel.update(cx, |panel, cx| {
                            panel.view.settled_expanded = !panel.view.settled_expanded;
                            cx.notify();
                        });
                    }
                },
            ))
        } else {
            None
        })
        .children(
            view.settled_expanded
                .then(|| {
                    view.settled_visible()
                        .iter()
                        .map(|thread| {
                            sidebar_thread_row(thread, ThreadRowVariant::Slim, view, theme, actions)
                        })
                        .collect::<Vec<_>>()
                })
                .into_iter()
                .flatten(),
        )
        .children(if view.settled_expanded && view.settled_has_more() {
            Some(sidebar_show_more_button(theme, {
                let panel = panel.clone();
                move |_window, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.view.show_more_settled();
                        cx.notify();
                    });
                }
            }))
        } else {
            None
        })
        .into_any_element()
}

fn search_results_list(
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
) -> impl IntoElement {
    let results = view.search_results();
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
                .map(|thread| sidebar_search_result_row(thread, view.is_active(thread), theme))
                .collect::<Vec<_>>(),
        )
}

fn draft_block(
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    panel: Entity<LeftSidebarPanel>,
    _cx: &mut Context<LeftSidebarPanel>,
) -> gpui::AnyElement {
    let draft = demo_draft();
    let is_active = view.active_thread_id == DEMO_DRAFT.id;
    sidebar_draft_row(
        draft,
        is_active,
        theme,
        {
            let panel = panel.clone();
            move |_window, cx| {
                panel.update(cx, |panel, cx| {
                    panel.view.activate_thread(DEMO_DRAFT.id);
                    cx.notify();
                });
            }
        },
        {
            let panel = panel.clone();
            move |_window, cx| {
                panel.update(cx, |panel, cx| {
                    panel.view.discard_draft();
                    cx.notify();
                });
            }
        },
    )
    .into_any_element()
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
