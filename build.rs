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
const PATCH_MARKER_ZERO_SIZE: &str = "opencore_allow_zero_dock_size";
const PATCH_MARKER_ANIMATED_CLIP: &str = "opencore_dock_animated_clip";
const PATCH_MARKER_BOTTOM_DOCK_ANIMATING: &str = "opencore_bottom_dock_animating";
const PATCH_MARKER_TITLE_BAR_DRAG: &str = "opencore_title_bar_drag_spacer";
const PATCH_MARKER_TITLE_BAR_LEADING: &str = "opencore_title_bar_leading_click_guard";
const PATCH_MARKER_TITLE_BAR_TRAILING: &str = "opencore_title_bar_trailing";

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
        patched |= patch_dock_zero_size(&dock_dir.join("dock.rs"));
        patched |= patch_dock_animated_clip(&dock_dir.join("dock.rs"));
        patched |= patch_dock_area_bottom_animating(&dock_dir.join("mod.rs"));
        patched |= patch_dock_area_skip_closed_bottom(&dock_dir.join("mod.rs"));
        patched |= patch_title_bar_drag_spacer(&src_dir.join("title_bar.rs"));
        patched |= patch_title_bar_leading_click_guard(&src_dir.join("title_bar.rs"));
        patched |= patch_title_bar_trailing(&src_dir.join("title_bar.rs"));
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

fn patch_dock_zero_size(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_ZERO_SIZE) {
        return false;
    }

    let needle = "    pub fn set_size(&mut self, size: Pixels, _: &mut Window, cx: &mut Context<Self>) {\n        self.size = size.max(PANEL_MIN_SIZE);\n        cx.notify();\n    }\n";
    let replacement = format!(
        "    pub fn set_size(&mut self, size: Pixels, _: &mut Window, cx: &mut Context<Self>) {{\n        // {PATCH_MARKER_ZERO_SIZE}\n        self.size = if size <= px(0.) {{ px(0.) }} else {{ size.max(PANEL_MIN_SIZE) }};\n        cx.notify();\n    }}\n"
    );

    if !content.contains(needle) {
        panic!(
            "gpui-component dock.rs changed; update build.rs zero-size patch for {}",
            path.display()
        );
    }

    let patched = content.replace(needle, &replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_dock_animated_clip(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_ANIMATED_CLIP) {
        return patch_dock_animated_clip_doc_comment(path)
            | patch_dock_animated_clip_from_state(path)
            | patch_dock_display_size_closed(path)
            | patch_dock_resize_sync_clip(path);
    }

    let struct_needle = "    /// Whether the Dock is resizing\n    resizing: bool,\n}";
    let struct_replacement = format!(
        "    /// Whether the Dock is resizing\n    resizing: bool,\n    // {PATCH_MARKER_ANIMATED_CLIP}\n    /// Outer clip width/height during show/hide tweens. `None` = use [`Self::size`].\n    animated_size: Option<gpui::Pixels>,\n}}"
    );

    let init_needle = "            resizing: false,\n        }\n    }\n\n    pub fn left(";
    let init_replacement = format!(
        "            resizing: false,\n            // {PATCH_MARKER_ANIMATED_CLIP}\n            animated_size: None,\n        }}\n    }}\n\n    pub fn left("
    );

    let from_state_needle = "            collapsible: true,\n            resizing: false,\n        }\n    }\n\n    fn subscribe_panel_events(";
    let from_state_replacement = format!(
        "            collapsible: true,\n            resizing: false,\n            // {PATCH_MARKER_ANIMATED_CLIP}\n            animated_size: None,\n        }}\n    }}\n\n    fn subscribe_panel_events("
    );

    let set_size_needle = "    pub fn set_size(&mut self, size: Pixels, _: &mut Window, cx: &mut Context<Self>) {\n        // opencore_allow_zero_dock_size\n        self.size = if size <= px(0.) { px(0.) } else { size.max(PANEL_MIN_SIZE) };\n        cx.notify();\n    }\n\n    /// Set the open state of the Dock.\n";
    let set_size_replacement = format!(
        "    pub fn set_size(&mut self, size: Pixels, _: &mut Window, cx: &mut Context<Self>) {{\n        // opencore_allow_zero_dock_size\n        self.size = if size <= px(0.) {{ px(0.) }} else {{ size.max(PANEL_MIN_SIZE) }};\n        cx.notify();\n    }}\n\n    /// Outer clip size during show/hide tweens (animated outer clip, fixed inner panel).\n    pub fn set_animated_size(&mut self, size: Option<Pixels>, cx: &mut Context<Self>) {{\n        self.animated_size = size;\n        cx.notify();\n    }}\n\n    pub fn clear_animated_size(&mut self, cx: &mut Context<Self>) {{\n        if self.animated_size.is_some() {{\n            self.animated_size = None;\n            cx.notify();\n        }}\n    }}\n\n    pub fn display_size(&self) -> Pixels {{\n        // {PATCH_MARKER_ANIMATED_CLIP}\n        if !self.open {{\n            return self.animated_size.unwrap_or(px(0.));\n        }}\n        self.animated_size.unwrap_or(self.size)\n    }}\n\n    /// Set the open state of the Dock.\n"
    );

    let render_needle = "impl Render for Dock {\n    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {\n        // opencore_hide_closed_bottom_dock\n        if !self.open {\n            return div();\n        }\n\n        let cache_style = StyleRefinement::default().absolute().size_full();\n\n        div()\n            .relative()\n            .overflow_hidden()\n            .map(|this| match self.placement {\n                DockPlacement::Left | DockPlacement::Right => this.h_flex().h_full().w(self.size),\n                DockPlacement::Bottom => this.w_full().h(self.size),\n                DockPlacement::Center => unreachable!(),\n            })\n            .map(|this| match &self.panel {\n                DockItem::Split { view, .. } => this.child(view.clone()),\n                DockItem::Tabs { view, .. } => this.child(view.clone()),\n                DockItem::Panel { view, .. } => this.child(view.clone().view().cached(cache_style)),\n                // Not support to render Tiles and Tile into Dock\n                DockItem::Tiles { .. } => this,\n            })\n            .child(self.render_resize_handle(window, cx))\n            .child(DockElement {\n                view: cx.entity().clone(),\n            })\n    }\n}\n";
    let render_replacement = "impl Render for Dock {\n    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {\n        let outer = self.display_size();\n        // opencore_hide_closed_bottom_dock\n        if !self.open && outer <= px(0.) {\n            return div();\n        }\n\n        let cache_style = StyleRefinement::default().absolute().size_full();\n        let inner = self.size;\n\n        div()\n            .relative()\n            .flex_none()\n            .overflow_hidden()\n            .map(|this| match self.placement {\n                DockPlacement::Left | DockPlacement::Right => this.h_flex().h_full().w(outer),\n                DockPlacement::Bottom => this.w_full().h(outer),\n                DockPlacement::Center => unreachable!(),\n            })\n            .child(\n                div()\n                    .relative()\n                    .overflow_hidden()\n                    .map(|this| match self.placement {\n                        DockPlacement::Left | DockPlacement::Right => this.h_flex().h_full().w(inner),\n                        DockPlacement::Bottom => this.w_full().h(inner),\n                        DockPlacement::Center => unreachable!(),\n                    })\n                    .map(|this| match &self.panel {\n                        DockItem::Split { view, .. } => this.child(view.clone()),\n                        DockItem::Tabs { view, .. } => this.child(view.clone()),\n                        DockItem::Panel { view, .. } => this.child(view.clone().view().cached(cache_style)),\n                        // Not support to render Tiles and Tile into Dock\n                        DockItem::Tiles { .. } => this,\n                    })\n                    .child(self.render_resize_handle(window, cx))\n                    .child(DockElement {\n                        view: cx.entity().clone(),\n                    }),\n            )\n    }\n}\n".to_string();

    if !content.contains(struct_needle) {
        panic!(
            "gpui-component dock.rs changed; update build.rs animated-clip struct patch for {}",
            path.display()
        );
    }
    if !content.contains(init_needle) {
        panic!(
            "gpui-component dock.rs changed; update build.rs animated-clip init patch for {}",
            path.display()
        );
    }
    if !content.contains(from_state_needle) {
        panic!(
            "gpui-component dock.rs changed; update build.rs animated-clip from_state patch for {}",
            path.display()
        );
    }
    if !content.contains(set_size_needle) {
        panic!(
            "gpui-component dock.rs changed; update build.rs animated-clip methods patch for {}",
            path.display()
        );
    }
    if !content.contains(render_needle) {
        panic!(
            "gpui-component dock.rs changed; update build.rs animated-clip render patch for {}",
            path.display()
        );
    }

    let patched = content
        .replace(struct_needle, &struct_replacement)
        .replace(init_needle, &init_replacement)
        .replace(from_state_needle, &from_state_replacement)
        .replace(set_size_needle, &set_size_replacement)
        .replace(render_needle, &render_replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_dock_animated_clip_doc_comment(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    let needle = "/// Outer clip size during show/hide tweens (comet `pane_container` pattern).";
    if !content.contains(needle) {
        return false;
    }

    let replacement =
        "/// Outer clip size during show/hide tweens (animated outer clip, fixed inner panel).";
    let patched = content.replace(needle, replacement);

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

fn patch_dock_display_size_closed(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    let marker = "opencore_dock_display_size_closed";
    if content.contains(marker) {
        return false;
    }

    let needle = "    pub fn display_size(&self) -> Pixels {\n        self.animated_size.unwrap_or(self.size)\n    }\n";
    let replacement = format!(
        "    pub fn display_size(&self) -> Pixels {{\n        // {marker}\n        if !self.open {{\n            return self.animated_size.unwrap_or(px(0.));\n        }}\n        self.animated_size.unwrap_or(self.size)\n    }}\n"
    );

    if !content.contains(needle) {
        return false;
    }

    let patched = content.replace(needle, &replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_dock_resize_sync_clip(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    let marker = "opencore_dock_resize_sync_clip";
    if content.contains(marker) {
        return false;
    }

    let needle = "        self.animated_size.unwrap_or(self.size)\n    }\n\n    /// Set the open state of the Dock.\n";
    let replacement = format!(
        "        // {marker}\n        if self.resizing {{\n            return self.size;\n        }}\n        self.animated_size.unwrap_or(self.size)\n    }}\n\n    /// Set the open state of the Dock.\n"
    );

    if !content.contains(needle) {
        return false;
    }

    let patched = content.replace(needle, &replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_dock_animated_clip_from_state(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    let marker = "opencore_dock_animated_clip_from_state";
    if content.contains(marker) {
        return false;
    }

    let needle = "            collapsible: true,\n            resizing: false,\n        }\n    }\n\n    fn subscribe_panel_events(";
    let replacement = format!(
        "            collapsible: true,\n            resizing: false,\n            // {marker}\n            animated_size: None,\n        }}\n    }}\n\n    fn subscribe_panel_events("
    );

    if !content.contains(needle) {
        return false;
    }

    let patched = content.replace(needle, &replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_dock_area_bottom_animating(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_BOTTOM_DOCK_ANIMATING) {
        return patch_dock_area_bottom_animating_px(path);
    }

    let needle = "                                                    .filter(|dock| dock.read(cx).is_open())\n";
    let replacement = format!(
        "                                                    .filter(|dock| {{\n                                                        let dock = dock.read(cx);\n                                                        // {PATCH_MARKER_BOTTOM_DOCK_ANIMATING}\n                                                        dock.is_open() || dock.display_size() > gpui::px(0.)\n                                                    }})\n"
    );

    if !content.contains(needle) {
        return false;
    }

    let patched = content.replace(needle, &replacement);

    std::fs::write(path, patched).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    true
}

fn patch_dock_area_bottom_animating_px(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    let marker = "opencore_bottom_dock_animating_px";
    if content.contains(marker) {
        return false;
    }

    let needle = "dock.is_open() || dock.display_size() > px(0.)";
    let replacement = format!("dock.is_open() || dock.display_size() > gpui::px(0.) // {marker}");

    if !content.contains(needle) {
        return false;
    }

    let patched = content.replace(needle, &replacement);

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

fn patch_title_bar_trailing(path: &std::path::Path) -> bool {
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });

    if content.contains(PATCH_MARKER_TITLE_BAR_TRAILING) {
        return false;
    }

    let struct_needle = "    children: SmallVec<[AnyElement; 1]>,\n    on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,\n";
    let struct_replacement = format!(
        "    children: SmallVec<[AnyElement; 1]>,\n    // {PATCH_MARKER_TITLE_BAR_TRAILING}\n    trailing_children: SmallVec<[AnyElement; 1]>,\n    on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,\n"
    );

    let init_needle =
        "            children: SmallVec::new(),\n            on_close_window: None,\n";
    let init_replacement = format!(
        "            children: SmallVec::new(),\n            // {PATCH_MARKER_TITLE_BAR_TRAILING}\n            trailing_children: SmallVec::new(),\n            on_close_window: None,\n"
    );

    let method_needle = "    /// Add custom for close window event, default is None, then click X button will call `window.remove_window()`.\n    /// Linux only, this will do nothing on other platforms.\n    pub fn on_close_window(\n";
    let method_replacement = "    /// Trailing title-bar controls (right side, before platform window controls).\n    pub fn trailing(mut self, element: impl IntoElement) -> Self {\n        self.trailing_children.push(element.into_any_element());\n        self\n    }\n\n    /// Add custom for close window event, default is None, then click X button will call `window.remove_window()`.\n    /// Linux only, this will do nothing on other platforms.\n    pub fn on_close_window(\n".to_string();

    let render_needle = "                )\n                .child(WindowControls {\n                    on_close_window: self.on_close_window,\n                }),\n        )\n    }\n}\n";
    let render_replacement = format!(
        "                )\n                .when(!self.trailing_children.is_empty(), |this| {{\n                    this.child(\n                        h_flex()\n                            .id(\"bar-trailing\")\n                            // {PATCH_MARKER_TITLE_BAR_TRAILING}\n                            .occlude()\n                            .h_full()\n                            .items_center()\n                            .flex_shrink_0()\n                            .gap_1()\n                            .pr_2()\n                            .on_double_click(|_, _, cx| cx.stop_propagation())\n                            .children(self.trailing_children),\n                    )\n                }})\n                .child(WindowControls {{\n                    on_close_window: self.on_close_window,\n                }}),\n        )\n    }}\n}}\n"
    );

    if !content.contains(struct_needle) {
        panic!(
            "gpui-component title_bar.rs changed; update build.rs trailing struct patch for {}",
            path.display()
        );
    }
    if !content.contains(init_needle) {
        panic!(
            "gpui-component title_bar.rs changed; update build.rs trailing init patch for {}",
            path.display()
        );
    }
    if !content.contains(method_needle) {
        panic!(
            "gpui-component title_bar.rs changed; update build.rs trailing method patch for {}",
            path.display()
        );
    }
    if !content.contains(render_needle) {
        panic!(
            "gpui-component title_bar.rs changed; update build.rs trailing render patch for {}",
            path.display()
        );
    }

    let patched = content
        .replace(struct_needle, &struct_replacement)
        .replace(init_needle, &init_replacement)
        .replace(method_needle, &method_replacement)
        .replace(render_needle, &render_replacement);

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
