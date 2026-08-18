use std::{rc::Rc, time::Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Context, DragMoveEvent, InteractiveElement, IntoElement, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::shared::theme::{BackgroundToken, ForegroundToken, OpenCoreTheme};

use super::{
    BOTTOM_DEFAULT, DimTween, RIGHT_DEFAULT, SIDEBAR_DEFAULT, ShellChrome, TITLEBAR_HEIGHT,
    TabModel, clamp_bottom_height, clamp_right_width, clamp_sidebar_width, eval_tween,
    tween_finished,
};

/// Leave the native macOS traffic-light controls (x=12..66) clear.
const TITLEBAR_CONTROLS_INSET: f32 = 68.0;

/// Callback used by the shell to persist chrome changes at the application root.
pub type ShellSaveFn = Rc<dyn Fn(ShellChrome, &mut App)>;

#[derive(Clone)]
struct SidebarResize;
impl Render for SidebarResize {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone)]
struct RightResize;
impl Render for RightResize {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone)]
struct BottomResize;
impl Render for BottomResize {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone)]
struct TabDrag {
    index: usize,
    title: String,
}

impl Render for TabDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(10.0))
            .py(px(6.0))
            .bg(gpui::black().opacity(0.9))
            .text_color(gpui::white())
            .child(self.title.clone())
    }
}

/// Static holy-grail shell layout and the live state used by later interactions.
#[allow(dead_code)]
pub struct Shell {
    chrome: ShellChrome,
    tab_model: TabModel,
    left_tween: Option<DimTween>,
    right_tween: Option<DimTween>,
    bottom_tween: Option<DimTween>,
    save: ShellSaveFn,
    theme: OpenCoreTheme,
    tab_bar_scroll_handle: ScrollHandle,
}

impl Shell {
    pub fn new(chrome: ShellChrome, save: ShellSaveFn, _cx: &mut Context<Self>) -> Self {
        let tab_model = TabModel::from_chrome(&chrome);
        Self {
            chrome,
            tab_model,
            left_tween: None,
            right_tween: None,
            bottom_tween: None,
            save,
            theme: OpenCoreTheme::resolve(crate::shared::theme::ThemeMode::Dark),
            tab_bar_scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn set_theme(&mut self, theme: OpenCoreTheme) {
        self.theme = theme;
    }

    pub fn left_target(&self) -> f32 {
        Self::left_target_for(&self.chrome)
    }

    pub fn right_target(&self) -> f32 {
        Self::right_target_for(&self.chrome)
    }

    pub fn bottom_target(&self) -> f32 {
        Self::bottom_target_for(&self.chrome)
    }

    pub fn left_target_for(chrome: &ShellChrome) -> f32 {
        if chrome.left_open {
            chrome.left_width
        } else {
            0.0
        }
    }

    pub fn right_target_for(chrome: &ShellChrome) -> f32 {
        if chrome.right_open {
            chrome.right_width
        } else {
            0.0
        }
    }

    pub fn bottom_target_for(chrome: &ShellChrome) -> f32 {
        if chrome.bottom_open {
            chrome.bottom_height
        } else {
            0.0
        }
    }

    pub fn toggle_left(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        let from = eval_tween(self.left_tween.as_ref(), self.left_target(), now, reduced);
        self.left_tween = Some(toggle_panel(
            &mut self.chrome.left_open,
            from,
            self.chrome.left_width,
            now,
        ));
        self.schedule_save(cx);
        cx.notify();
    }

    pub fn toggle_right(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        let from = eval_tween(self.right_tween.as_ref(), self.right_target(), now, reduced);
        self.right_tween = Some(toggle_panel(
            &mut self.chrome.right_open,
            from,
            self.chrome.right_width,
            now,
        ));
        self.schedule_save(cx);
        cx.notify();
    }

    pub fn toggle_bottom(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        let from = eval_tween(
            self.bottom_tween.as_ref(),
            self.bottom_target(),
            now,
            reduced,
        );
        self.bottom_tween = Some(toggle_panel(
            &mut self.chrome.bottom_open,
            from,
            self.chrome.bottom_height,
            now,
        ));
        self.schedule_save(cx);
        cx.notify();
    }

    fn apply_sidebar_resize(&mut self, proposed_width: f32) {
        resize_sidebar(&mut self.chrome, proposed_width);
        self.left_tween = None;
    }

    fn apply_right_resize(&mut self, proposed_width: f32, viewport_width: f32) {
        resize_right(&mut self.chrome, proposed_width, viewport_width);
        self.right_tween = None;
    }

    fn apply_bottom_resize(&mut self, proposed_height: f32, viewport_height: f32) {
        resize_bottom(&mut self.chrome, proposed_height, viewport_height);
        self.bottom_tween = None;
    }

    fn reset_sidebar(&mut self) {
        reset_sidebar(&mut self.chrome);
        self.left_tween = None;
    }

    fn reset_right(&mut self) {
        reset_right(&mut self.chrome);
        self.right_tween = None;
    }

    fn reset_bottom(&mut self) {
        reset_bottom(&mut self.chrome);
        self.bottom_tween = None;
    }

    fn schedule_save(&self, cx: &mut Context<Self>) {
        (self.save)(self.chrome.clone(), cx);
    }

    fn sync_tab_model_to_chrome(&mut self) {
        let (tabs, active_id) = self.tab_model.to_chrome_tabs();
        self.chrome.tabs = tabs;
        self.chrome.active_tab_id = active_id;
    }

    fn select_tab(&mut self, id: &str, cx: &mut Context<Self>) {
        self.tab_model.select(id);
        self.sync_tab_model_to_chrome();
        self.schedule_save(cx);
        cx.notify();
    }

    fn close_tab(&mut self, id: &str, cx: &mut Context<Self>) {
        self.tab_model.close(id);
        self.sync_tab_model_to_chrome();
        self.schedule_save(cx);
        cx.notify();
    }

    fn add_stub_tab(&mut self, cx: &mut Context<Self>) {
        self.tab_model.add_stub();
        self.sync_tab_model_to_chrome();
        self.schedule_save(cx);
        cx.notify();
    }

    fn reorder_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        self.tab_model.reorder(from, to);
        self.sync_tab_model_to_chrome();
        self.schedule_save(cx);
        cx.notify();
    }

    fn render_tab_chip(
        &self,
        index: usize,
        tab: &crate::app::shell::ShellTabRecord,
        active: bool,
        titlebar_background: gpui::Hsla,
        label: gpui::Hsla,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = tab.id.clone();
        let select_id = id.clone();
        let close_id = id.clone();
        let on_select = cx.listener(move |shell, _, _, cx| shell.select_tab(&select_id, cx));
        let on_close = cx.listener(move |shell, _, _, cx| shell.close_tab(&close_id, cx));
        let drag = TabDrag {
            index,
            title: tab.title.clone(),
        };
        div()
            .id(format!("shell-tab-{index}"))
            .h_full()
            .min_w(px(120.0))
            .max_w(px(220.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .flex_shrink_0()
            .cursor_pointer()
            .border_b_1()
            .border_color(if active { label } else { titlebar_background })
            .when(active, |chip| chip.bg(titlebar_background.opacity(0.65)))
            .on_click(on_select)
            .on_drag(drag, move |tab, _, _, cx| cx.new(|_| tab.clone()))
            .drag_over::<TabDrag>(move |element, dragged, _, _| {
                if dragged.index == index {
                    element
                } else {
                    element.border_b_2().border_color(label)
                }
            })
            .on_drop({
                let target = index;
                cx.listener(move |shell, dragged: &TabDrag, _, cx| {
                    shell.reorder_tab(dragged.index, tab_drop_index(dragged.index, target), cx);
                })
            })
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_color(if active { gpui::white() } else { label })
                    .child(tab.title.clone()),
            )
            .child(
                Button::new(format!("shell-tab-close-{index}"))
                    .ghost()
                    .compact()
                    .label("×")
                    .on_click(on_close),
            )
    }

    fn settle_tweens(&mut self, now: Instant, reduced: bool) {
        if reduced {
            self.left_tween = None;
            self.right_tween = None;
            self.bottom_tween = None;
            return;
        }

        if self
            .left_tween
            .as_ref()
            .is_some_and(|tween| tween_finished(tween, now))
        {
            self.left_tween = None;
        }
        if self
            .right_tween
            .as_ref()
            .is_some_and(|tween| tween_finished(tween, now))
        {
            self.right_tween = None;
        }
        if self
            .bottom_tween
            .as_ref()
            .is_some_and(|tween| tween_finished(tween, now))
        {
            self.bottom_tween = None;
        }
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        self.settle_tweens(now, reduced);

        let left_width = eval_tween(self.left_tween.as_ref(), self.left_target(), now, reduced);
        let right_width = eval_tween(self.right_tween.as_ref(), self.right_target(), now, reduced);
        let bottom_height = eval_tween(
            self.bottom_tween.as_ref(),
            self.bottom_target(),
            now,
            reduced,
        );
        if self.left_tween.is_some() || self.right_tween.is_some() || self.bottom_tween.is_some() {
            window.request_animation_frame();
        }

        let background = self.theme.surface(BackgroundToken::Primary);
        let panel_background = self.theme.surface(BackgroundToken::Secondary);
        let titlebar_background = self.theme.surface(BackgroundToken::Tertiary);
        let label = self.theme.foreground(ForegroundToken::Muted);

        let (tabs, active_id) = self.tab_model.to_chrome_tabs();
        let active_title = tabs
            .iter()
            .find(|tab| tab.id == active_id)
            .map(|tab| tab.title.clone())
            .unwrap_or_else(|| "MAIN".into());

        let left = div()
            .w(px(left_width))
            .h_full()
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                stub_region("LEFT", panel_background, label)
                    .w(px(self.chrome.left_width))
                    .h_full(),
            );
        let right = div()
            .w(px(right_width))
            .h_full()
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                stub_region("RIGHT", panel_background, label)
                    .w(px(self.chrome.right_width))
                    .h_full(),
            );
        let main = div()
            .flex()
            .items_center()
            .justify_center()
            .flex_1()
            .w_full()
            .bg(background)
            .text_color(label)
            .child(format!("MAIN · {active_title}"));
        let bottom = div()
            .w_full()
            .h(px(bottom_height))
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                stub_region("BOTTOM", panel_background, label)
                    .w_full()
                    .h(px(self.chrome.bottom_height)),
            );

        let on_left_toggle = cx.listener(|shell, _, _, cx| shell.toggle_left(cx));
        let on_right_toggle = cx.listener(|shell, _, _, cx| shell.toggle_right(cx));
        let on_bottom_toggle = cx.listener(|shell, _, _, cx| shell.toggle_bottom(cx));
        let on_add_tab = cx.listener(|shell, _, _, cx| shell.add_stub_tab(cx));

        let tab_items = tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                self.render_tab_chip(
                    index,
                    tab,
                    tab.id == active_id,
                    titlebar_background,
                    label,
                    cx,
                )
            })
            .collect::<Vec<_>>();

        let on_sidebar_drag =
            cx.listener(|shell, event: &DragMoveEvent<SidebarResize>, _window, cx| {
                shell.apply_sidebar_resize(event.event.position.x.as_f32());
                shell.schedule_save(cx);
                cx.notify();
            });
        let on_right_drag = cx.listener(|shell, event: &DragMoveEvent<RightResize>, window, cx| {
            let viewport_width = window.bounds().size.width.as_f32();
            let proposed_width = viewport_width - event.event.position.x.as_f32();
            shell.apply_right_resize(proposed_width, viewport_width);
            shell.schedule_save(cx);
            cx.notify();
        });
        let on_bottom_drag =
            cx.listener(|shell, event: &DragMoveEvent<BottomResize>, window, cx| {
                let viewport_height = window.bounds().size.height.as_f32();
                let proposed_height = viewport_height - event.event.position.y.as_f32();
                shell.apply_bottom_resize(proposed_height, viewport_height);
                shell.schedule_save(cx);
                cx.notify();
            });
        let on_sidebar_reset = cx.listener(|shell, event: &gpui::ClickEvent, _, cx| {
            if event.click_count() == 2 {
                shell.reset_sidebar();
                shell.schedule_save(cx);
                cx.notify();
            }
        });
        let on_right_reset = cx.listener(|shell, event: &gpui::ClickEvent, _, cx| {
            if event.click_count() == 2 {
                shell.reset_right();
                shell.schedule_save(cx);
                cx.notify();
            }
        });
        let on_bottom_reset = cx.listener(|shell, event: &gpui::ClickEvent, _, cx| {
            if event.click_count() == 2 {
                shell.reset_bottom();
                shell.schedule_save(cx);
                cx.notify();
            }
        });

        let sidebar_seam = div().relative().w(px(0.0)).h_full().child(
            div()
                .id("shell-sidebar-resize")
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(-2.5))
                .w(px(5.0))
                .cursor_col_resize()
                .on_drag(SidebarResize, |_, _, _, cx| cx.new(|_| SidebarResize))
                .on_drag_move(on_sidebar_drag)
                .on_click(on_sidebar_reset),
        );
        let right_seam = div().relative().w(px(0.0)).h_full().child(
            div()
                .id("shell-right-resize")
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(-2.5))
                .w(px(5.0))
                .cursor_col_resize()
                .on_drag(RightResize, |_, _, _, cx| cx.new(|_| RightResize))
                .on_drag_move(on_right_drag)
                .on_click(on_right_reset),
        );
        let bottom_seam = div().relative().w_full().h(px(0.0)).child(
            div()
                .id("shell-bottom-resize")
                .absolute()
                .left_0()
                .right_0()
                .top(px(-2.5))
                .h(px(5.0))
                .cursor_row_resize()
                .on_drag(BottomResize, |_, _, _, cx| cx.new(|_| BottomResize))
                .on_drag_move(on_bottom_drag)
                .on_click(on_bottom_reset),
        );

        div()
            .relative()
            .size_full()
            .bg(background)
            .child(
                div()
                    .size_full()
                    .flex()
                    .pt(px(TITLEBAR_HEIGHT))
                    .child(left)
                    .child(sidebar_seam)
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(main)
                            .child(bottom_seam)
                            .child(bottom),
                    )
                    .child(right_seam)
                    .child(right),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(TITLEBAR_HEIGHT))
                    .bg(titlebar_background)
                    .flex()
                    .items_center()
                    .child(div().w(px(TITLEBAR_CONTROLS_INSET)).h_full())
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .child(
                                div()
                                    .id("shell-tab-strip")
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .overflow_x_scroll()
                                    .track_scroll(&self.tab_bar_scroll_handle)
                                    .children(tab_items),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(px(36.0))
                                    .bg(titlebar_background.opacity(0.86)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .right_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(px(36.0))
                                    .bg(titlebar_background.opacity(0.86)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .child(
                                Button::new("shell-left-toggle")
                                    .ghost()
                                    .compact()
                                    .label("Left")
                                    .on_click(on_left_toggle),
                            )
                            .child(
                                Button::new("shell-right-toggle")
                                    .ghost()
                                    .compact()
                                    .label("Right")
                                    .on_click(on_right_toggle),
                            )
                            .child(
                                Button::new("shell-bottom-toggle")
                                    .ghost()
                                    .compact()
                                    .label("Bottom")
                                    .on_click(on_bottom_toggle),
                            )
                            .child(
                                Button::new("shell-tab-add")
                                    .ghost()
                                    .compact()
                                    .label("+")
                                    .on_click(on_add_tab),
                            ),
                    ),
            )
    }
}

fn tab_drop_index(from: usize, target: usize) -> usize {
    if from < target {
        target.saturating_sub(1)
    } else {
        target
    }
}

fn toggle_panel(open: &mut bool, from: f32, open_size: f32, started: Instant) -> DimTween {
    *open = !*open;
    DimTween {
        from,
        to: if *open { open_size } else { 0.0 },
        started,
    }
}

fn resize_sidebar(chrome: &mut ShellChrome, proposed_width: f32) {
    chrome.left_width = clamp_sidebar_width(proposed_width);
    chrome.left_open = true;
}

fn resize_right(chrome: &mut ShellChrome, proposed_width: f32, viewport_width: f32) {
    chrome.right_width = clamp_right_width(proposed_width, viewport_width);
    chrome.right_open = true;
}

fn resize_bottom(chrome: &mut ShellChrome, proposed_height: f32, viewport_height: f32) {
    chrome.bottom_height = clamp_bottom_height(proposed_height, viewport_height);
    chrome.bottom_open = true;
}

fn reset_sidebar(chrome: &mut ShellChrome) {
    chrome.left_width = SIDEBAR_DEFAULT;
    chrome.left_open = true;
}

fn reset_right(chrome: &mut ShellChrome) {
    chrome.right_width = RIGHT_DEFAULT;
    chrome.right_open = true;
}

fn reset_bottom(chrome: &mut ShellChrome) {
    chrome.bottom_height = BOTTOM_DEFAULT;
    chrome.bottom_open = true;
}

/// GPUI does not expose a reduced-motion setting on `Window` or `App` in this revision.
fn reduced_motion(_cx: &App) -> bool {
    false
}

fn stub_region(label: &'static str, background: gpui::Hsla, foreground: gpui::Hsla) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .bg(background)
        .text_color(foreground)
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::{
        BOTTOM_DEFAULT, SIDEBAR_DEFAULT, Shell, TITLEBAR_CONTROLS_INSET, reset_bottom, reset_right,
        reset_sidebar, resize_bottom, resize_right, resize_sidebar, tab_drop_index, toggle_panel,
    };
    use crate::app::shell::{RIGHT_DEFAULT, SIDEBAR_MAX, ShellChrome};
    use std::time::Instant;

    #[test]
    fn left_target_zero_when_closed() {
        let mut chrome = ShellChrome::default();
        chrome.left_open = false;

        assert_eq!(Shell::left_target_for(&chrome), 0.0);
    }

    #[test]
    fn left_target_uses_width_when_open() {
        let mut chrome = ShellChrome::default();
        chrome.left_width = 312.0;

        assert_eq!(Shell::left_target_for(&chrome), 312.0);
    }

    #[test]
    fn right_and_bottom_targets_follow_open_flags() {
        let mut chrome = ShellChrome::default();
        chrome.right_open = true;
        chrome.bottom_open = true;

        assert_eq!(Shell::right_target_for(&chrome), chrome.right_width);
        assert_eq!(Shell::bottom_target_for(&chrome), chrome.bottom_height);

        chrome.right_open = false;
        chrome.bottom_open = false;
        assert_eq!(Shell::right_target_for(&chrome), 0.0);
        assert_eq!(Shell::bottom_target_for(&chrome), 0.0);
    }

    #[test]
    fn toggle_panel_flips_open_flag_and_sets_tween_endpoints() {
        let now = Instant::now();
        let mut open = true;

        let tween = toggle_panel(&mut open, 256.0, 256.0, now);

        assert!(!open);
        assert_eq!(tween.from, 256.0);
        assert_eq!(tween.to, 0.0);
        assert_eq!(tween.started, now);

        let tween = toggle_panel(&mut open, 48.0, 256.0, now);

        assert!(open);
        assert_eq!(tween.from, 48.0);
        assert_eq!(tween.to, 256.0);
    }

    #[test]
    fn titlebar_controls_clear_native_traffic_lights() {
        const NATIVE_TRAFFIC_LIGHT_RIGHT_EDGE: f32 = 66.0;

        assert!(TITLEBAR_CONTROLS_INSET >= NATIVE_TRAFFIC_LIGHT_RIGHT_EDGE);
    }

    #[test]
    fn sidebar_resize_clamps_and_opens_panel_without_touching_other_state() {
        let mut chrome = ShellChrome::default();
        chrome.left_open = false;
        chrome.right_open = true;
        let before_right = chrome.right_width;

        resize_sidebar(&mut chrome, 999.0);

        assert_eq!(chrome.left_width, SIDEBAR_MAX);
        assert!(chrome.left_open);
        assert_eq!(chrome.right_width, before_right);
    }

    #[test]
    fn right_resize_uses_live_viewport_width_and_opens_panel() {
        let mut chrome = ShellChrome::default();
        chrome.right_open = false;

        resize_right(&mut chrome, 900.0, 800.0);

        assert_eq!(chrome.right_width, 416.0);
        assert!(chrome.right_open);
    }

    #[test]
    fn bottom_resize_uses_live_viewport_height_and_opens_panel() {
        let mut chrome = ShellChrome::default();
        chrome.bottom_open = false;

        resize_bottom(&mut chrome, 900.0, 800.0);

        assert_eq!(chrome.bottom_height, 440.0);
        assert!(chrome.bottom_open);
    }

    #[test]
    fn resize_reset_helpers_restore_persisted_defaults_and_open_panels() {
        let mut chrome = ShellChrome::default();
        chrome.left_width = 390.0;
        chrome.right_width = 450.0;
        chrome.bottom_height = 410.0;
        chrome.left_open = false;
        chrome.right_open = false;
        chrome.bottom_open = false;

        reset_sidebar(&mut chrome);
        reset_right(&mut chrome);
        reset_bottom(&mut chrome);

        assert_eq!(chrome.left_width, SIDEBAR_DEFAULT);
        assert_eq!(chrome.right_width, RIGHT_DEFAULT);
        assert_eq!(chrome.bottom_height, BOTTOM_DEFAULT);
        assert!(chrome.left_open && chrome.right_open && chrome.bottom_open);
    }

    #[test]
    fn resize_cancels_in_flight_tween_before_live_size_takes_effect() {
        let mut shell = test_shell();
        shell.left_tween = Some(super::DimTween {
            from: 0.0,
            to: 256.0,
            started: Instant::now(),
        });

        shell.apply_sidebar_resize(300.0);

        assert_eq!(shell.chrome.left_width, 300.0);
        assert!(shell.chrome.left_open);
        assert!(shell.left_tween.is_none());
    }

    #[test]
    fn tab_model_snapshot_syncs_into_persisted_chrome() {
        let mut shell = test_shell();
        let added_id = shell.tab_model.add_stub();

        shell.sync_tab_model_to_chrome();

        assert_eq!(shell.chrome.active_tab_id, added_id);
        assert_eq!(shell.chrome.tabs, shell.tab_model.to_chrome_tabs().0);
    }

    #[test]
    fn tab_drop_index_accounts_for_removed_source_chip() {
        assert_eq!(tab_drop_index(0, 2), 1);
        assert_eq!(tab_drop_index(2, 0), 0);
        assert_eq!(tab_drop_index(1, 1), 1);
    }

    #[test]
    fn each_panel_resize_and_reset_cancels_its_own_tween() {
        let mut shell = test_shell();
        let now = Instant::now();
        shell.right_tween = Some(super::DimTween {
            from: 0.0,
            to: 320.0,
            started: now,
        });
        shell.bottom_tween = Some(super::DimTween {
            from: 0.0,
            to: 220.0,
            started: now,
        });

        shell.apply_right_resize(360.0, 1280.0);
        assert!(shell.right_tween.is_none());
        assert_eq!(shell.chrome.right_width, 360.0);
        shell.apply_bottom_resize(300.0, 800.0);
        assert!(shell.bottom_tween.is_none());
        assert_eq!(shell.chrome.bottom_height, 300.0);

        shell.left_tween = Some(super::DimTween {
            from: 300.0,
            to: 0.0,
            started: now,
        });
        shell.right_tween = Some(super::DimTween {
            from: 360.0,
            to: 0.0,
            started: now,
        });
        shell.bottom_tween = Some(super::DimTween {
            from: 300.0,
            to: 0.0,
            started: now,
        });
        shell.reset_sidebar();
        shell.reset_right();
        shell.reset_bottom();
        assert!(shell.left_tween.is_none());
        assert!(shell.right_tween.is_none());
        assert!(shell.bottom_tween.is_none());
    }

    fn test_shell() -> Shell {
        let save: super::ShellSaveFn = std::rc::Rc::new(|_, _| {});
        Shell {
            chrome: ShellChrome::default(),
            tab_model: super::TabModel::from_chrome(&ShellChrome::default()),
            left_tween: None,
            right_tween: None,
            bottom_tween: None,
            save,
            theme: crate::shared::theme::OpenCoreTheme::resolve(
                crate::shared::theme::ThemeMode::Dark,
            ),
            tab_bar_scroll_handle: gpui::ScrollHandle::new(),
        }
    }
}
