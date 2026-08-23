//! Dock stub panels and panel registry for the shell workspace migration.

use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, Styled, WeakEntity, Window, div,
};
use gpui_component::dock::{
    DockArea, Panel, PanelEvent, PanelInfo, PanelRegistry, PanelState, register_panel,
};
use serde::{Deserialize, Serialize};

/// Returns the tab index to select after closing `closed` when `len_after` tabs remain.
pub fn next_active_after_close(
    active: usize,
    closed: usize,
    len_after: usize,
) -> Option<usize> {
    if len_after == 0 {
        return None;
    }
    if active < closed {
        Some(active.min(len_after - 1))
    } else if active == closed {
        Some(closed.min(len_after - 1))
    } else {
        Some((active - 1).min(len_after - 1))
    }
}

macro_rules! stub_panel {
    ($struct_name:ident, $panel_name:literal, $label:literal) => {
        pub struct $struct_name {
            focus_handle: FocusHandle,
        }

        impl $struct_name {
            pub fn new(cx: &mut Context<Self>) -> Self {
                Self {
                    focus_handle: cx.focus_handle(),
                }
            }
        }

        impl EventEmitter<PanelEvent> for $struct_name {}

        impl Focusable for $struct_name {
            fn focus_handle(&self, _: &App) -> FocusHandle {
                self.focus_handle.clone()
            }
        }

        impl Panel for $struct_name {
            fn panel_name(&self) -> &'static str {
                $panel_name
            }

            fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                $label
            }
        }

        impl Render for $struct_name {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .track_focus(&self.focus_handle)
                    .child($label)
            }
        }
    };
}

stub_panel!(LeftStubPanel, "left-stub", "LEFT");
stub_panel!(RightStubPanel, "right-stub", "RIGHT");
stub_panel!(BottomStubPanel, "bottom-stub", "BOTTOM");

pub struct MainStubPanel {
    focus_handle: FocusHandle,
    tab_title: SharedString,
}

impl MainStubPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tab_title: "MAIN".into(),
        }
    }

    pub fn with_title(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tab_title: title.into(),
        }
    }

    pub fn tab_title(&self) -> &SharedString {
        &self.tab_title
    }
}

impl EventEmitter<PanelEvent> for MainStubPanel {}

impl Focusable for MainStubPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for MainStubPanel {
    fn panel_name(&self) -> &'static str {
        "main-stub"
    }

    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.tab_title.clone()
    }

    fn tab_name(&self, _: &App) -> Option<SharedString> {
        Some(self.tab_title.clone())
    }
}

impl Render for MainStubPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.focus_handle)
            .child(self.tab_title.clone())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CenterStubHostState {
    active: usize,
}

pub struct CenterStubHost {
    focus_handle: FocusHandle,
    active_ix: usize,
    tabs: Vec<Entity<MainStubPanel>>,
}

impl CenterStubHost {
    pub fn with_initial_tab(cx: &mut Context<Self>) -> Self {
        let tab = cx.new(|cx| MainStubPanel::new(cx));
        Self {
            focus_handle: cx.focus_handle(),
            active_ix: 0,
            tabs: vec![tab],
        }
    }

    pub fn select(&mut self, ix: usize) {
        if ix < self.tabs.len() {
            self.active_ix = ix;
        }
    }

    pub fn add_tab(&mut self, cx: &mut Context<Self>) {
        let n = self.tabs.len() + 1;
        let tab = cx.new(|cx| MainStubPanel::with_title(format!("MAIN {n}"), cx));
        self.tabs.push(tab);
        self.active_ix = self.tabs.len() - 1;
    }

    pub fn close_tab(&mut self, ix: usize) {
        if ix >= self.tabs.len() {
            return;
        }
        self.tabs.remove(ix);
        self.active_ix = next_active_after_close(self.active_ix, ix, self.tabs.len()).unwrap_or(0);
    }

    pub fn tab_titles(&self, cx: &App) -> Vec<SharedString> {
        self.tabs
            .iter()
            .map(|tab| tab.read(cx).tab_title().clone())
            .collect()
    }

    fn from_panel_state(
        dock_area: WeakEntity<DockArea>,
        panel_state: &PanelState,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let active = match &panel_state.info {
            PanelInfo::Panel(value) => serde_json::from_value::<CenterStubHostState>(value.clone())
                .map(|state| state.active)
                .unwrap_or(0),
            _ => 0,
        };

        let tabs = if panel_state.children.is_empty() {
            vec![cx.new(|cx| MainStubPanel::new(cx))]
        } else {
            panel_state
                .children
                .iter()
                .map(|child| {
                    let view = PanelRegistry::build_panel(
                        &child.panel_name,
                        dock_area.clone(),
                        child,
                        &child.info,
                        window,
                        cx,
                    );
                    Entity::<MainStubPanel>::from(&*view)
                })
                .collect()
        };

        let active_ix = if tabs.is_empty() {
            0
        } else {
            active.min(tabs.len() - 1)
        };

        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            active_ix,
            tabs,
        })
    }
}

impl EventEmitter<PanelEvent> for CenterStubHost {}

impl Focusable for CenterStubHost {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for CenterStubHost {
    fn panel_name(&self) -> &'static str {
        "center-stub-host"
    }

    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        ""
    }

    fn closable(&self, _: &App) -> bool {
        false
    }

    fn dump(&self, cx: &App) -> PanelState {
        let mut state = PanelState::new(self);
        for tab in &self.tabs {
            state.add_child(tab.read(cx).dump(cx));
        }
        state.info = PanelInfo::panel(
            serde_json::to_value(CenterStubHostState {
                active: self.active_ix,
            })
            .unwrap_or_default(),
        );
        state
    }
}

impl Render for CenterStubHost {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.tabs.get(self.active_ix).cloned();
        div()
            .size_full()
            .track_focus(&self.focus_handle)
            .when_some(active, |this, tab| this.child(tab))
    }
}

pub fn register_shell_panels(cx: &mut App) {
    register_panel(cx, "left-stub", |_, _, _, _, cx| {
        Box::new(cx.new(|cx| LeftStubPanel::new(cx)))
    });
    register_panel(cx, "right-stub", |_, _, _, _, cx| {
        Box::new(cx.new(|cx| RightStubPanel::new(cx)))
    });
    register_panel(cx, "bottom-stub", |_, _, _, _, cx| {
        Box::new(cx.new(|cx| BottomStubPanel::new(cx)))
    });
    register_panel(cx, "main-stub", |_, _, _, _, cx| {
        Box::new(cx.new(|cx| MainStubPanel::new(cx)))
    });
    register_panel(cx, "center-stub-host", |dock_area, panel_state, info, window, cx| {
        let host = match info {
            PanelInfo::Panel(value) if !value.is_null() => {
                CenterStubHost::from_panel_state(dock_area, panel_state, window, cx)
            }
            _ if !panel_state.children.is_empty() => {
                CenterStubHost::from_panel_state(dock_area, panel_state, window, cx)
            }
            _ => cx.new(|cx| CenterStubHost::with_initial_tab(cx)),
        };
        Box::new(host)
    });
}

#[cfg(test)]
mod tests {
    use super::next_active_after_close;

    #[test]
    fn center_stub_host_add_select_close_updates_active() {
        assert_eq!(next_active_after_close(0, 0, 2), Some(0));
        assert_eq!(next_active_after_close(2, 2, 2), Some(1));
        assert_eq!(next_active_after_close(0, 0, 0), None);
    }
}
