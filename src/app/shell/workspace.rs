//! Dock-based shell workspace: title bar dock toggles and DockArea body.

use std::rc::Rc;

use gpui::{
    App, AppContext, ClickEvent, Context, Edges, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, Styled, Subscription, Window, div,
};
use gpui_component::{
    IconName, InteractiveElementExt as _, Sizable, TitleBar,
    button::{Button, ButtonVariants as _},
    dock::{DockArea, DockAreaState, DockEvent, DockItem, DockPlacement, PanelStyle},
    h_flex,
};

use crate::shared::theme::OpenCoreTheme;

use super::{DOCK_LAYOUT_VERSION, apply_default_holy_grail};

const MAIN_DOCK_ID: &str = "main-dock";

/// Callback used by the workspace to persist dock layout changes at the application root.
pub type DockSaveFn = Rc<dyn Fn(DockAreaState, &mut App)>;

pub struct ShellWorkspace {
    dock_area: Entity<DockArea>,
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
                // Always show tab bars so single-tab groups still expose tab-bar drop targets.
                .panel_style(PanelStyle::TabBar)
        });

        let reset_to_default = match saved {
            None => {
                apply_default_holy_grail(&dock_area, window, cx);
                true
            }
            Some(state) if state.version != Some(DOCK_LAYOUT_VERSION) => {
                eprintln!(
                    "opencore: dock layout version mismatch (saved {:?}, expected {DOCK_LAYOUT_VERSION}); resetting to default",
                    state.version,
                );
                apply_default_holy_grail(&dock_area, window, cx);
                true
            }
            Some(state) => match dock_area.update(cx, |dock, cx| dock.load(state, window, cx)) {
                Ok(()) => {
                    dock_area.update(cx, |dock, cx| {
                        dock.set_toggle_button_visible(false, cx);
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
                    if center_has_main_stub(&dock_area, cx) {
                        false
                    } else {
                        apply_default_holy_grail(&dock_area, window, cx);
                        true
                    }
                }
                Err(error) => {
                    eprintln!("opencore: dock layout load failed: {error:?}");
                    apply_default_holy_grail(&dock_area, window, cx);
                    true
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

        Self {
            dock_area,
            _subscriptions: vec![layout_subscription],
        }
    }

    pub fn set_theme(&mut self, _theme: OpenCoreTheme) {}
}

/// True when center contains at least one registered main stub panel.
///
/// Empty/`Default` `DockAreaState` still loads as a tab of `InvalidPanel`, so a
/// raw panel-count check is not enough to decide whether the layout is usable.
fn center_has_main_stub(dock_area: &Entity<DockArea>, cx: &App) -> bool {
    item_has_main_stub(dock_area.read(cx).center(), cx)
}

fn item_has_main_stub(item: &DockItem, cx: &App) -> bool {
    match item {
        DockItem::Panel { view, .. } => view.panel_name(cx) == "main-stub",
        DockItem::Tabs { items, .. } => items.iter().any(|view| view.panel_name(cx) == "main-stub"),
        DockItem::Split { items, .. } => items.iter().any(|item| item_has_main_stub(item, cx)),
        DockItem::Tiles { .. } => false,
    }
}

/// Title-bar dock toggle that expands/collapses a dock without zooming the window.
///
/// TitleBar treats the strip as a drag/double-click zoom region. Wrapping the
/// button in an occluding hit target and ignoring the second click of a
/// double-click keeps a double-tap as a single dock toggle.
fn title_bar_dock_toggle(
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    placement: DockPlacement,
    cx: &Context<ShellWorkspace>,
) -> impl IntoElement {
    div()
        .id(id)
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_double_click(|_, _, cx| cx.stop_propagation())
        .child(
            Button::new(id)
                .ghost()
                .xsmall()
                .icon(icon)
                .tooltip(tooltip)
                .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    // First click of a double-click already toggled; skip the second.
                    if event.click_count() > 1 {
                        return;
                    }
                    this.dock_area.update(cx, |area, cx| {
                        area.toggle_dock(placement, window, cx);
                    });
                })),
        )
}

impl Render for ShellWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                TitleBar::new()
                    .child(title_bar_dock_toggle(
                        "toggle-left-dock",
                        IconName::PanelLeft,
                        "Toggle Left Dock",
                        DockPlacement::Left,
                        cx,
                    ))
                    .child(
                        h_flex()
                            .gap_1()
                            .child(title_bar_dock_toggle(
                                "toggle-bottom-dock",
                                IconName::PanelBottom,
                                "Toggle Bottom Dock",
                                DockPlacement::Bottom,
                                cx,
                            ))
                            .child(title_bar_dock_toggle(
                                "toggle-right-dock",
                                IconName::PanelRight,
                                "Toggle Right Dock",
                                DockPlacement::Right,
                                cx,
                            )),
                    ),
            )
            .child(div().flex_1().min_h_0().child(self.dock_area.clone()))
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
        use crate::app::shell::{
            EDGE_DOCK_TAB_COUNT, dock_item_enables_dnd, dock_item_panel_count,
        };

        let dock = dock_area.read(cx);
        assert!(dock.is_dock_open(DockPlacement::Left, cx));
        assert!(!dock.is_dock_open(DockPlacement::Right, cx));
        assert!(!dock.is_dock_open(DockPlacement::Bottom, cx));
        assert_eq!(
            dock.left_dock().map(|dock| dock.read(cx).size()),
            Some(px(SIDEBAR_DEFAULT))
        );
        assert_eq!(dock.dump(cx).version, Some(DOCK_LAYOUT_VERSION));

        assert!(
            dock_item_enables_dnd(dock.center()),
            "center must be Split-wrapped for DnD"
        );
        assert!(
            dock_item_panel_count(dock.center()) >= EDGE_DOCK_TAB_COUNT,
            "center expected ≥{EDGE_DOCK_TAB_COUNT} panels for DnD"
        );

        for dock_entity in [dock.left_dock(), dock.right_dock(), dock.bottom_dock()]
            .into_iter()
            .flatten()
        {
            let panel = dock_entity.read(cx).panel();
            assert!(
                dock_item_enables_dnd(panel),
                "edge dock must be Split-wrapped for DnD"
            );
            let count = dock_item_panel_count(panel);
            assert!(
                count >= EDGE_DOCK_TAB_COUNT,
                "edge dock expected ≥{EDGE_DOCK_TAB_COUNT} panels for DnD, got {count}"
            );
        }
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

        let persisted = saved_layout
            .borrow()
            .clone()
            .expect("save callback should run");
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
    fn dock_load_compatible_but_empty_center_resets_default(cx: &mut TestAppContext) {
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
