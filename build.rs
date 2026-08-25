//! Applies small patches to gpui-component for opencore shell dock behavior:
//! - hide empty panel toolbar ellipsis menus on stub panels
//! - allow dragging the last tab in a group (merge into other tab bars)
//! - fully hide the bottom dock when collapsed (title bar toggle replaces the tab strip)
//! - confine title-bar drag/zoom to a trailing spacer so leading controls stay clickable

const PATCH_MARKER_TOOLBAR: &str = "opencore_hide_empty_panel_toolbar";
const PATCH_MARKER_DRAG: &str = "opencore_allow_last_panel_drag";
const PATCH_MARKER_BOTTOM_HIDE: &str = "opencore_hide_closed_bottom_dock";
const PATCH_MARKER_DOCK_AREA_SKIP: &str = "opencore_skip_closed_bottom_dock";
const PATCH_MARKER_TAB_PANEL_COLLAPSED_BOTTOM: &str = "opencore_hide_collapsed_bottom_tab_bar";
const PATCH_MARKER_TITLE_BAR_DRAG: &str = "opencore_title_bar_drag_spacer";
const PATCH_MARKER_TITLE_BAR_LEADING: &str = "opencore_title_bar_leading_click_guard";

fn main() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("cargo metadata failed");

    let mut patched = false;

    if let Some(src_dir) = find_gpui_component_src_dir(&metadata) {
        let dock_dir = src_dir.join("dock");
        patched |= patch_tab_panel_toolbar(&dock_dir.join("tab_panel.rs"));
        patched |= patch_tab_panel_draggable(&dock_dir.join("tab_panel.rs"));
        patched |= patch_tab_panel_hide_collapsed_bottom(&dock_dir.join("tab_panel.rs"));
        patched |= patch_dock_hide_closed_bottom(&dock_dir.join("dock.rs"));
        patched |= patch_dock_area_skip_closed_bottom(&dock_dir.join("mod.rs"));
        patched |= patch_title_bar_drag_spacer(&src_dir.join("title_bar.rs"));
        patched |= patch_title_bar_leading_click_guard(&src_dir.join("title_bar.rs"));
    }

    if patched {
        for path in patched_paths(&metadata) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
        println!(
            "cargo:warning=gpui-component was patched; run `cargo build` once more if the app still shows old dock behavior"
        );
    }
}

fn patched_paths(metadata: &cargo_metadata::Metadata) -> Vec<std::path::PathBuf> {
    find_gpui_component_src_dir(metadata)
        .map(|src_dir| {
            let dock_dir = src_dir.join("dock");
            [
                dock_dir.join("tab_panel.rs"),
                dock_dir.join("dock.rs"),
                dock_dir.join("mod.rs"),
                src_dir.join("title_bar.rs"),
            ]
            .into_iter()
            .collect()
        })
        .unwrap_or_default()
}

fn find_gpui_component_src_dir(metadata: &cargo_metadata::Metadata) -> Option<std::path::PathBuf> {
    metadata
        .packages
        .iter()
        .find(|package| package.name == "gpui-component")
        .map(|package| {
            package
                .manifest_path
                .parent()
                .expect("gpui-component manifest path")
                .join("src")
                .into_std_path_buf()
        })
}

fn patch_tab_panel_toolbar(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_TOOLBAR) {
        return false;
    }

    let needle = "        let zoomable_toolbar_visible = state.zoomable.map_or(false, |v| v.toolbar_visible());\n";
    let Some(index) = content.find(needle) else {
        panic!(
            "gpui-component tab_panel.rs changed; update build.rs toolbar patch for {}",
            path.display()
        );
    };

    let insert_at = index + needle.len();
    let patch = format!(
        "\n        // {PATCH_MARKER_TOOLBAR}\n        let menu_visible = state.zoomable.map_or(false, |v| v.menu_visible());\n        if !menu_visible\n            && !state.closable\n            && !zoomable_toolbar_visible\n            && !zoomed\n            && self.toolbar_buttons(window, cx).is_none()\n        {{\n            return div();\n        }}\n"
    );

    let patched = format!(
        "{}{}{}",
        &content[..insert_at],
        patch,
        &content[insert_at..]
    );

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_tab_panel_draggable(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_DRAG) {
        return false;
    }

    let needle = "    fn draggable(&self, cx: &App) -> bool {\n        !self.is_locked(cx) && !self.is_last_panel(cx)\n    }\n";
    let replacement = format!(
        "    fn draggable(&self, cx: &App) -> bool {{\n        // {PATCH_MARKER_DRAG}\n        !self.is_locked(cx)\n    }}\n"
    );

    if !content.contains(needle) {
        panic!(
            "gpui-component tab_panel.rs changed; update build.rs draggable patch for {}",
            path.display()
        );
    }

    let patched = content.replace(needle, &replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_tab_panel_hide_collapsed_bottom(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_TAB_PANEL_COLLAPSED_BOTTOM) {
        return false;
    }

    let needle = "    fn render_title_bar(\n        &mut self,\n        state: &TabState,\n        window: &mut Window,\n        cx: &mut Context<Self>,\n    ) -> impl IntoElement {\n        let view = cx.entity().clone();\n";
    let replacement = format!(
        "    fn render_title_bar(\n        &mut self,\n        state: &TabState,\n        window: &mut Window,\n        cx: &mut Context<Self>,\n    ) -> impl IntoElement {{\n        // {PATCH_MARKER_TAB_PANEL_COLLAPSED_BOTTOM}\n        if self.collapsed {{\n            if let Some(dock_area) = self.dock_area.upgrade() {{\n                let entity_id = cx.entity().entity_id();\n                let dock_area = dock_area.read(cx);\n                if dock_area.toggle_button_panels.bottom == Some(entity_id)\n                    && !dock_area.is_dock_open(DockPlacement::Bottom, cx)\n                {{\n                    return div().into_any_element();\n                }}\n            }}\n        }}\n\n        let view = cx.entity().clone();\n"
    );

    if !content.contains(needle) {
        panic!(
            "gpui-component tab_panel.rs changed; update build.rs collapsed-bottom patch for {}",
            path.display()
        );
    }

    let patched = content.replace(needle, &replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_dock_hide_closed_bottom(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_BOTTOM_HIDE) {
        return false;
    }

    let open_guard = "        if !self.open && !self.placement.is_bottom() {\n            return div();\n        }\n";
    let open_guard_replacement = format!(
        "        // {PATCH_MARKER_BOTTOM_HIDE}\n        if !self.open {{\n            return div();\n        }}\n"
    );

    let collapsed_strip = "            // Bottom Dock should keep the title bar, then user can click the Toggle button\n            .when(!self.open && self.placement.is_bottom(), |this| {\n                this.h(px(29.))\n            })\n";

    if !content.contains(open_guard) {
        panic!(
            "gpui-component dock.rs changed; update build.rs bottom-hide patch for {}",
            path.display()
        );
    }

    let mut patched = content.replace(open_guard, &open_guard_replacement);
    if patched.contains(collapsed_strip) {
        patched = patched.replace(collapsed_strip, "");
    }

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_dock_area_skip_closed_bottom(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_DOCK_AREA_SKIP) {
        return false;
    }

    let needle = "                                            // Bottom Dock\n                                            .when_some(self.bottom_dock.clone(), |this, dock| {\n                                                this.child(dock)\n                                            }),\n";
    let replacement = format!(
        "                                            // {PATCH_MARKER_DOCK_AREA_SKIP}\n                                            .when_some(\n                                                self.bottom_dock\n                                                    .as_ref()\n                                                    .filter(|dock| dock.read(cx).is_open())\n                                                    .cloned(),\n                                                |this, dock| {{\n                                                    this.child(\n                                                        div().flex().flex_none().w_full().child(dock),\n                                                    )\n                                                }},\n                                            ),\n"
    );

    if !content.contains(needle) {
        panic!(
            "gpui-component dock mod.rs changed; update build.rs dock-area patch for {}",
            path.display()
        );
    }

    let patched = content.replace(needle, &replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_title_bar_drag_spacer(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_TITLE_BAR_DRAG) {
        return false;
    }

    let needle = r#"                .when(is_linux, |this| {
                    this.on_double_click(|_, window, _| window.zoom_window())
                })
                .when(is_macos, |this| {
                    this.on_double_click(|_, window, _| window.titlebar_double_click())
                })
                .on_mouse_down_out(window.listener_for(&state, |state, _, _, _| {
                    state.should_move = false;
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(&state, |state, _, _, _| {
                        state.should_move = true;
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&state, |state, _, _, _| {
                        state.should_move = false;
                    }),
                )
                .on_mouse_move(window.listener_for(&state, |state, _, window, _| {
                    if state.should_move {
                        state.should_move = false;
                        window.start_window_move();
                    }
                }))
                .child(
                    h_flex()
                        .id("bar")
                        .h_full()
                        .justify_between()
                        .flex_shrink_0()
                        .flex_1()
                        .when(!is_web, |this| {
                            this.window_control_area(WindowControlArea::Drag)
                                .when(window.is_fullscreen(), |this| this.pl_3())
                                .when(is_linux && is_client_decorated, |this| {
                                    this.child(
                                        div()
                                            .top_0()
                                            .left_0()
                                            .absolute()
                                            .size_full()
                                            .h_full()
                                            .on_mouse_down(
                                                MouseButton::Right,
                                                move |ev, window, _| {
                                                    window.show_window_menu(ev.position)
                                                },
                                            ),
                                    )
                                })
                        })
                        .children(self.children),
                )"#;

    let replacement = r#"                .child(
                    h_flex()
                        .id("bar")
                        .h_full()
                        .flex_shrink_0()
                        .flex_1()
                        // opencore_title_bar_drag_spacer
                        .child(
                            h_flex()
                                .id("bar-leading")
                                .h_full()
                                .items_center()
                                .flex_shrink_0()
                                .gap_1()
                                .children(self.children),
                        )
                        .child(
                            div()
                                .id("title-bar-drag")
                                .flex_1()
                                .h_full()
                                .min_w_0()
                                .when(is_linux, |this| {
                                    this.on_double_click(|_, window, _| window.zoom_window())
                                })
                                .when(is_macos, |this| {
                                    this.on_double_click(|_, window, _| window.titlebar_double_click())
                                })
                                .on_mouse_down_out(window.listener_for(&state, |state, _, _, _| {
                                    state.should_move = false;
                                }))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    window.listener_for(&state, |state, _, _, _| {
                                        state.should_move = true;
                                    }),
                                )
                                .on_mouse_up(
                                    MouseButton::Left,
                                    window.listener_for(&state, |state, _, _, _| {
                                        state.should_move = false;
                                    }),
                                )
                                .on_mouse_move(window.listener_for(&state, |state, _, window, _| {
                                    if state.should_move {
                                        state.should_move = false;
                                        window.start_window_move();
                                    }
                                }))
                                .when(!is_web, |this| {
                                    this.window_control_area(WindowControlArea::Drag)
                                        .when(window.is_fullscreen(), |this| this.pl_3())
                                        .when(is_linux && is_client_decorated, |this| {
                                            this.child(
                                                div()
                                                    .top_0()
                                                    .left_0()
                                                    .absolute()
                                                    .size_full()
                                                    .h_full()
                                                    .on_mouse_down(
                                                        MouseButton::Right,
                                                        move |ev, window, _| {
                                                            window.show_window_menu(ev.position)
                                                        },
                                                    ),
                                            )
                                        })
                                }),
                        ),
                )"#;

    if !content.contains(needle) {
        panic!(
            "gpui-component title_bar.rs changed; update build.rs title-bar patch for {}",
            path.display()
        );
    }

    let patched = content.replace(needle, replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_title_bar_leading_click_guard(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_TITLE_BAR_LEADING) {
        return false;
    }

    let needle = r#"                            h_flex()
                                .id("bar-leading")
                                .h_full()
                                .items_center()
                                .flex_shrink_0()
                                .gap_1()
                                .children(self.children),"#;

    let replacement = r#"                            h_flex()
                                .id("bar-leading")
                                // opencore_title_bar_leading_click_guard
                                .occlude()
                                .h_full()
                                .items_center()
                                .flex_shrink_0()
                                .gap_1()
                                .on_double_click(|_, _, cx| cx.stop_propagation())
                                .children(self.children),"#;

    if !content.contains(needle) {
        panic!(
            "gpui-component title_bar.rs changed; update build.rs leading-click patch for {}",
            path.display()
        );
    }

    let patched = content.replace(needle, replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}
