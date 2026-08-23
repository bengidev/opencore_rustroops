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
    DOCK_LAYOUT_VERSION, apply_default_holy_grail, center_title_bar, panels::CenterStubHost,
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
        let dock_area =
            cx.new(|cx| DockArea::new(MAIN_DOCK_ID, Some(DOCK_LAYOUT_VERSION), window, cx));

        let (center_host, reset_to_default) = match saved {
            None => (apply_default_holy_grail(&dock_area, window, cx), true),
            Some(state) if state.version != Some(DOCK_LAYOUT_VERSION) => {
                eprintln!(
                    "opencore: dock layout version mismatch (saved {:?}, expected {DOCK_LAYOUT_VERSION}); resetting to default",
                    state.version,
                );
                (apply_default_holy_grail(&dock_area, window, cx), true)
            }
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
                    match recover_center_host(&dock_area, cx) {
                        Some(host) => (host, false),
                        None => (apply_default_holy_grail(&dock_area, window, cx), true),
                    }
                }
                Err(error) => {
                    eprintln!("opencore: dock layout load failed: {error:?}");
                    (apply_default_holy_grail(&dock_area, window, cx), true)
                }
            },
        };

        if reset_to_default {
            save(dock_area.read(cx).dump(cx), cx);
        }

        let save_for_layout = save.clone();
        let layout_subscription =
            cx.subscribe_in(&dock_area, window, move |_, dock_area, event, _, cx| {
                if matches!(event, DockEvent::LayoutChanged) {
                    let state = dock_area.read(cx).dump(cx);
                    save_for_layout(state, cx);
                }
            });

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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{App, AppContext, TestAppContext, px};
    use gpui_component::dock::{DockAreaState, DockPlacement};

    use crate::app::shell::{DOCK_LAYOUT_VERSION, SIDEBAR_DEFAULT, register_shell_panels};

    use super::{DockSaveFn, ShellWorkspace};

    fn init_shell_panels(cx: &mut TestAppContext) {
        cx.update(|app| {
            gpui_component::init(app);
            register_shell_panels(app);
        });
    }

    fn noop_save() -> DockSaveFn {
        Rc::new(|_, _| {})
    }

    fn assert_default_holy_grail_layout(
        dock_area: &gpui::Entity<gpui_component::dock::DockArea>,
        cx: &App,
    ) {
        let dock = dock_area.read(cx);
        assert!(dock.is_dock_open(DockPlacement::Left, cx));
        assert!(!dock.is_dock_open(DockPlacement::Right, cx));
        assert!(!dock.is_dock_open(DockPlacement::Bottom, cx));
        assert_eq!(
            dock.left_dock().map(|dock| dock.read(cx).size()),
            Some(px(SIDEBAR_DEFAULT))
        );
        assert_eq!(dock.dump(cx).version, Some(DOCK_LAYOUT_VERSION));
    }

    #[gpui::test]
    fn dock_load_version_mismatch_resets_default(cx: &mut TestAppContext) {
        init_shell_panels(cx);
        let saved = DockAreaState {
            version: Some(0),
            ..Default::default()
        };

        let (workspace, _) = cx.add_window_view(|window, cx| {
            ShellWorkspace::new(Some(saved), noop_save(), window, cx)
        });

        cx.read_entity(&workspace, |workspace, cx| {
            assert_default_holy_grail_layout(&workspace.dock_area, cx);
        });
    }

    #[gpui::test]
    fn dock_reset_persists_default_layout(cx: &mut TestAppContext) {
        init_shell_panels(cx);
        let saved_layout = Rc::new(RefCell::new(None));
        let save: DockSaveFn = {
            let saved_layout = saved_layout.clone();
            Rc::new(move |layout, _| {
                *saved_layout.borrow_mut() = Some(layout);
            })
        };
        let saved = DockAreaState {
            version: Some(0),
            ..Default::default()
        };

        let _ = cx.add_window_view(|window, cx| ShellWorkspace::new(Some(saved), save, window, cx));

        let persisted = saved_layout.borrow().clone().expect("save callback should run");
        assert_eq!(persisted.version, Some(DOCK_LAYOUT_VERSION));
    }

    #[gpui::test]
    fn dock_load_none_uses_default_layout(cx: &mut TestAppContext) {
        init_shell_panels(cx);

        let (workspace, _) =
            cx.add_window_view(|window, cx| ShellWorkspace::new(None, noop_save(), window, cx));

        cx.read_entity(&workspace, |workspace, cx| {
            assert_default_holy_grail_layout(&workspace.dock_area, cx);
        });
    }

    #[gpui::test]
    fn dock_load_compatible_but_unrecoverable_resets_default(cx: &mut TestAppContext) {
        init_shell_panels(cx);
        let saved = DockAreaState {
            version: Some(DOCK_LAYOUT_VERSION),
            ..Default::default()
        };

        let (workspace, _) = cx.add_window_view(|window, cx| {
            ShellWorkspace::new(Some(saved), noop_save(), window, cx)
        });

        cx.read_entity(&workspace, |workspace, cx| {
            assert_default_holy_grail_layout(&workspace.dock_area, cx);
        });
    }

    #[gpui::test]
    fn dock_load_failure_resets_default(cx: &mut TestAppContext) {
        init_shell_panels(cx);

        let (reference, _) =
            cx.add_window_view(|window, cx| ShellWorkspace::new(None, noop_save(), window, cx));

        let mut corrupt = cx.read_entity(&reference, |workspace, cx| {
            workspace.dock_area.read(cx).dump(cx)
        });
        corrupt.center.panel_name = "nonexistent-panel".into();

        let (workspace, _) = cx.add_window_view(|window, cx| {
            ShellWorkspace::new(Some(corrupt), noop_save(), window, cx)
        });

        cx.read_entity(&workspace, |workspace, cx| {
            assert_default_holy_grail_layout(&workspace.dock_area, cx);
        });
    }
}
