//! Left dock panel — t3code thread sidebar interface (demo data).

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
    div, px,
};
use gpui_component::{
    dock::{Panel, PanelEvent},
    input::InputState,
    v_flex,
};

use crate::app::shell::workspace_theme::WorkspaceTheme;
use crate::shared::theme::{
    BackgroundToken, BorderToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::chrome::sidebar_chrome_footer;
use super::content::{
    ShelfTone, ThreadRowVariant, sidebar_project_scope_row, sidebar_search_row,
    sidebar_shelf_header, sidebar_thread_row,
};
use super::demo_data::{DEMO_THREADS, SCOPED_PROJECT_LABEL, ThreadShelf};
use super::tokens::{CONTENT_INSET, DOCK_RESIZE_GUTTER};

const PANEL_TITLE: &str = "THREADS";

pub struct LeftSidebarPanel {
    focus_handle: FocusHandle,
    theme: WorkspaceTheme,
    search: Entity<InputState>,
    snoozed_expanded: bool,
    settled_expanded: bool,
}

impl LeftSidebarPanel {
    pub fn new(window: &mut Window, theme: WorkspaceTheme, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));

        Self {
            focus_handle: cx.focus_handle(),
            theme,
            search,
            snoozed_expanded: true,
            settled_expanded: true,
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
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.get();
        let page = theme.surface(BackgroundToken::Primary);
        let border = theme.border_token(BorderToken::Default);

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
            .child(fixed_controls(&self.search, &theme))
            .child(
                div()
                    .id("left-sidebar-scroll")
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .child(thread_list(&theme, self.snoozed_expanded, self.settled_expanded)),
            )
            .child(sidebar_chrome_footer(&theme))
            .border_r_1()
            .border_color(border)
    }
}

fn fixed_controls(search: &Entity<InputState>, theme: &OpenCoreTheme) -> impl IntoElement {
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
        .child(sidebar_search_row(search, theme))
        .child(sidebar_project_scope_row(SCOPED_PROJECT_LABEL, theme))
}

fn thread_list(
    theme: &OpenCoreTheme,
    snoozed_expanded: bool,
    settled_expanded: bool,
) -> impl IntoElement {
    let snoozed_count = DEMO_THREADS
        .iter()
        .filter(|t| t.shelf == ThreadShelf::Snoozed)
        .count();
    let settled_count = DEMO_THREADS
        .iter()
        .filter(|t| t.shelf == ThreadShelf::Settled)
        .count();

    let snoozed_label = if snoozed_expanded {
        "Snoozed".to_string()
    } else {
        format!("Snoozed ({snoozed_count})")
    };
    let settled_label = if settled_expanded {
        "Settled".to_string()
    } else {
        format!("Settled ({settled_count})")
    };

    v_flex()
        .id("left-sidebar-thread-list")
        .w_full()
        .min_w_0()
        .gap(px(1.))
        .pl(px(CONTENT_INSET))
        .pr(px(CONTENT_INSET + DOCK_RESIZE_GUTTER))
        .pb(px(CONTENT_INSET))
        .children(
            DEMO_THREADS
                .iter()
                .filter(|t| t.shelf == ThreadShelf::Pinned)
                .map(|thread| sidebar_thread_row(thread, ThreadRowVariant::Card, theme)),
        )
        .children(
            DEMO_THREADS
                .iter()
                .filter(|t| t.shelf == ThreadShelf::Active)
                .map(|thread| sidebar_thread_row(thread, ThreadRowVariant::Card, theme)),
        )
        .children(if snoozed_count > 0 {
            Some(sidebar_shelf_header(
                &snoozed_label,
                snoozed_expanded,
                ShelfTone::Snoozed,
                theme,
            ))
        } else {
            None
        })
        .children(
            snoozed_expanded
                .then(|| {
                    DEMO_THREADS
                        .iter()
                        .filter(|t| t.shelf == ThreadShelf::Snoozed)
                        .map(|thread| sidebar_thread_row(thread, ThreadRowVariant::Slim, theme))
                        .collect::<Vec<_>>()
                })
                .into_iter()
                .flatten(),
        )
        .children(if settled_count > 0 {
            Some(sidebar_shelf_header(
                &settled_label,
                settled_expanded,
                ShelfTone::Settled,
                theme,
            ))
        } else {
            None
        })
        .children(
            settled_expanded
                .then(|| {
                    DEMO_THREADS
                        .iter()
                        .filter(|t| t.shelf == ThreadShelf::Settled)
                        .map(|thread| sidebar_thread_row(thread, ThreadRowVariant::Slim, theme))
                        .collect::<Vec<_>>()
                })
                .into_iter()
                .flatten(),
        )
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
