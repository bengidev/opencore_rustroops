//! Dock stub panels and panel registry for the shell workspace.

use gpui::{
    App, AppContext, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
};
use gpui_component::dock::{Panel, PanelEvent, register_panel};

macro_rules! stub_panel {
    ($struct_name:ident, $panel_name:literal, $label:literal) => {
        pub struct $struct_name {
            focus_handle: FocusHandle,
            title: SharedString,
        }

        impl $struct_name {
            pub fn new(cx: &mut Context<Self>) -> Self {
                Self {
                    focus_handle: cx.focus_handle(),
                    title: $label.into(),
                }
            }

            pub fn with_title(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
                Self {
                    focus_handle: cx.focus_handle(),
                    title: title.into(),
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
                self.title.clone()
            }

            fn tab_name(&self, _: &App) -> Option<SharedString> {
                Some(self.title.clone())
            }

            fn closable(&self, _: &App) -> bool {
                false
            }

            fn zoomable(&self, _: &App) -> Option<gpui_component::dock::PanelControl> {
                None
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
                    .child(self.title.clone())
            }
        }
    };
}

stub_panel!(LeftStubPanel, "left-stub", "LEFT");
stub_panel!(RightStubPanel, "right-stub", "RIGHT");
stub_panel!(BottomStubPanel, "bottom-stub", "BOTTOM");
stub_panel!(MainStubPanel, "main-stub", "MAIN");

pub fn register_shell_panels(cx: &mut App) {
    register_panel(cx, "left-stub", |_, _, _, _, cx| {
        Box::new(cx.new(LeftStubPanel::new))
    });
    register_panel(cx, "right-stub", |_, _, _, _, cx| {
        Box::new(cx.new(RightStubPanel::new))
    });
    register_panel(cx, "bottom-stub", |_, _, _, _, cx| {
        Box::new(cx.new(BottomStubPanel::new))
    });
    register_panel(cx, "main-stub", |_, _, _, _, cx| {
        Box::new(cx.new(MainStubPanel::new))
    });
}
