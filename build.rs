//! Applies small patches to gpui-component for opencore shell dock behavior:
//! - hide empty panel toolbar ellipsis menus on stub panels
//! - allow dragging the last tab in a group (merge into other tab bars)
//! - fully hide the bottom dock when collapsed (title bar toggle replaces the tab strip)

const PATCH_MARKER_TOOLBAR: &str = "opencore_hide_empty_panel_toolbar";
const PATCH_MARKER_DRAG: &str = "opencore_allow_last_panel_drag";
const PATCH_MARKER_BOTTOM_HIDE: &str = "opencore_hide_closed_bottom_dock";
const PATCH_MARKER_DOCK_AREA_SKIP: &str = "opencore_skip_closed_bottom_dock";
const PATCH_MARKER_TAB_PANEL_COLLAPSED_BOTTOM: &str = "opencore_hide_collapsed_bottom_tab_bar";

fn main() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("cargo metadata failed");

    let mut patched = false;

    if let Some(dock_dir) = find_gpui_component_dock_dir(&metadata) {
        patched |= patch_tab_panel_toolbar(&dock_dir.join("tab_panel.rs"));
        patched |= patch_tab_panel_draggable(&dock_dir.join("tab_panel.rs"));
        patched |= patch_tab_panel_hide_collapsed_bottom(&dock_dir.join("tab_panel.rs"));
        patched |= patch_dock_hide_closed_bottom(&dock_dir.join("dock.rs"));
        patched |= patch_dock_area_skip_closed_bottom(&dock_dir.join("mod.rs"));
    }

    if patched {
        invalidate_gpui_component_artifacts(&metadata);
        println!(
            "cargo:warning=gpui-component was patched; rebuilding dependency — run `cargo build` once more if the app still shows old dock behavior"
        );
    }
}

fn find_gpui_component_dock_dir(metadata: &cargo_metadata::Metadata) -> Option<std::path::PathBuf> {
    metadata
        .packages
        .iter()
        .find(|package| package.name == "gpui-component")
        .map(|package| {
            package
                .manifest_path
                .parent()
                .expect("gpui-component manifest path")
                .join("src/dock")
                .into_std_path_buf()
        })
}

/// Drop cached gpui-component build artifacts so the next compile picks up patched sources.
fn invalidate_gpui_component_artifacts(metadata: &cargo_metadata::Metadata) {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let target = metadata.target_directory.as_std_path();
    let dirs = [
        target.join(&profile).join("deps"),
        target.join(&profile).join(".fingerprint"),
        target.join(&profile).join("build"),
    ];

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.contains("gpui_component") && !name.contains("gpui-component") {
                continue;
            }

            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path).ok();
            } else {
                std::fs::remove_file(&path).ok();
            }
        }
    }
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
