//! Dock-based shell workspace: title bar center tabs, DockArea body, status bar toggles.

use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Edges, Entity, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants as _},
    dock::{DockArea, DockAreaState, DockEvent, DockItem, DockPlacement, PanelView},
    status_bar::StatusBar,
};

use crate::shared::theme::OpenCoreTheme;

use super::{
    DOCK_LAYOUT_VERSION, apply_default_holy_grail, center_title_bar,
    panels::CenterStubHost,
};

const MAIN_DOCK_ID: &str = "main-dock";

/// Callback used by the workspace to persist dock layout changes at the application root.
pub type DockSaveFn = Rc<dyn Fn(DockAreaState, &mut App)>;

pub struct ShellWorkspace {
    dock_area: Entity<DockArea>,
    center_host: Entity<CenterStubHost>,
    _subscriptions: Vec<Subscription>,
}

impl ShellWorkspace {
    pub fn new(
        saved: Option<DockAreaState>,
        save: DockSaveFn,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let dock_area = cx.new(|cx| {
            DockArea::new(MAIN_DOCK_ID, Some(DOCK_LAYOUT_VERSION), window, cx)
        });

        let center_host = match saved.filter(|state| state.version == Some(DOCK_LAYOUT_VERSION)) {
            Some(state) => match dock_area.update(cx, |dock, cx| dock.load(state, window, cx)) {
                Ok(()) => {
                    dock_area.update(cx, |dock, cx| {
                        dock.set_dock_collapsible(
                            Edges {
                                left: true,
                                right: true,
                                bottom: true,
                                ..Default::default()
                            },
                            window,
                            cx,
                        );
                    });
                    recover_center_host(&dock_area, cx)
                        .unwrap_or_else(|| apply_default_holy_grail(&dock_area, window, cx))
                }
                Err(error) => {
                    eprintln!("opencore: dock layout load failed: {error:?}");
                    apply_default_holy_grail(&dock_area, window, cx)
                }
            },
            None => apply_default_holy_grail(&dock_area, window, cx),
        };

        let save_for_layout = save.clone();
        let layout_subscription = cx.subscribe_in(
            &dock_area,
            window,
            move |_, dock_area, event, _, cx| {
                if matches!(event, DockEvent::LayoutChanged) {
                    let state = dock_area.read(cx).dump(cx);
                    save_for_layout(state, cx);
                }
            },
        );

        let titlebar_subscription = cx.observe(&center_host, |_, _, cx| {
            cx.notify();
        });

        Self {
            dock_area,
            center_host,
            _subscriptions: vec![layout_subscription, titlebar_subscription],
        }
    }

    pub fn set_theme(&mut self, _theme: OpenCoreTheme) {}
}

fn recover_center_host(dock_area: &Entity<DockArea>, cx: &App) -> Option<Entity<CenterStubHost>> {
    recover_center_host_from_item(dock_area.read(cx).center(), cx)
}

fn recover_center_host_from_item(item: &DockItem, cx: &App) -> Option<Entity<CenterStubHost>> {
    match item {
        DockItem::Panel { view, .. } => try_recover_center(view.as_ref(), cx),
        DockItem::Tabs { items, .. } => items
            .iter()
            .find_map(|view| try_recover_center(view.as_ref(), cx)),
        DockItem::Split { items, .. } => items
            .iter()
            .find_map(|item| recover_center_host_from_item(item, cx)),
        DockItem::Tiles { .. } => None,
    }
}

fn try_recover_center(view: &dyn PanelView, cx: &App) -> Option<Entity<CenterStubHost>> {
    if view.panel_name(cx) != "center-stub-host" {
        return None;
    }
    view.view().downcast::<CenterStubHost>().ok()
}

impl Render for ShellWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(center_title_bar(&self.center_host, cx))
            .child(div().flex_1().min_h_0().child(self.dock_area.clone()))
            .child(
                StatusBar::new()
                    .left(
                        Button::new("toggle-left-dock")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelLeft)
                            .tooltip("Toggle Left Dock")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.dock_area.update(cx, |area, cx| {
                                    area.toggle_dock(DockPlacement::Left, window, cx);
                                });
                            })),
                    )
                    .left(
                        Button::new("toggle-bottom-dock")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelBottom)
                            .tooltip("Toggle Bottom Dock")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.dock_area.update(cx, |area, cx| {
                                    area.toggle_dock(DockPlacement::Bottom, window, cx);
                                });
                            })),
                    )
                    .child(
                        Button::new("toggle-right-dock")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelRight)
                            .tooltip("Toggle Right Dock")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.dock_area.update(cx, |area, cx| {
                                    area.toggle_dock(DockPlacement::Right, window, cx);
                                });
                            })),
                    ),
            )
    }
}
