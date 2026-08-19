use std::{rc::Rc, time::Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton,
    MouseMoveEvent, ParentElement, Render, ScrollHandle, StatefulInteractiveElement, Styled,
    Subscription, Window, WindowControlArea, canvas, div, linear_color_stop, linear_gradient, px,
    relative,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Escape, Input, InputEvent, InputState, SelectAll};
use gpui_component::{Selectable, Sizable};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, TypeRole,
};

use super::{
    BOTTOM_DEFAULT, DimTween, RIGHT_DEFAULT, SIDEBAR_DEFAULT, ShellChrome, TITLEBAR_HEIGHT,
    TabModel, clamp_bottom_height, clamp_right_width, clamp_sidebar_width, eval_tween,
    tween_finished,
};

/// macOS `hiddenInset` chrome: traffic lights sit near x=14; Comet budgets 88px
/// for the top-left cluster so tabs never bleed into the control area.
const TITLEBAR_CONTROLS_INSET: f32 = 88.0;
const TAB_CHIP_MIN_WIDTH: f32 = 64.0;
const TAB_CHIP_MAX_WIDTH: f32 = 280.0;
const TAB_CHIP_GAP: f32 = 4.0;
const TAB_FADE_WIDTH: f32 = 36.0;
const SHELL_TEXT: TypeRole = TypeRole::MonoSm;

#[derive(Clone, Copy)]
struct TabChipColors {
    titlebar: gpui::Hsla,
    active_surface: gpui::Hsla,
    primary: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TabFadeState {
    left: bool,
    right: bool,
}

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
    surface: gpui::Hsla,
    foreground: gpui::Hsla,
    border: gpui::Hsla,
}

impl Render for TabDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(8.0))
            .py(px(4.0))
            .text_size(px(SHELL_TEXT.size()))
            .line_height(relative(SHELL_TEXT.line_height()))
            .border_1()
            .border_color(self.border)
            .bg(self.surface)
            .text_color(self.foreground)
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
    tab_fade_state: TabFadeState,
    titlebar_drag_pending: bool,
    renaming_tab_id: Option<String>,
    rename_input: Option<gpui::Entity<InputState>>,
    /// When false, ignore Input Blur so focus-before-mount / deferred focus
    /// setup cannot immediately commit and abort the rename session.
    rename_commit_on_blur: bool,
    _rename_subscriptions: Vec<Subscription>,
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
            tab_fade_state: TabFadeState::default(),
            titlebar_drag_pending: false,
            renaming_tab_id: None,
            rename_input: None,
            rename_commit_on_blur: false,
            _rename_subscriptions: Vec::new(),
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

    /// Return the right-pane target constrained by the current viewport.
    ///
    /// The persisted width remains untouched so a resize back to a sensible
    /// viewport can restore the user's chosen dimension.
    pub fn right_target_for_viewport(chrome: &ShellChrome, viewport_width: f32) -> f32 {
        if chrome.right_open {
            clamp_right_width(chrome.right_width, viewport_width)
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

    /// Return the bottom-drawer target constrained by the current viewport.
    ///
    /// The persisted height remains untouched so a resize back to a sensible
    /// viewport can restore the user's chosen dimension.
    pub fn bottom_target_for_viewport(chrome: &ShellChrome, viewport_height: f32) -> f32 {
        if chrome.bottom_open {
            clamp_bottom_height(chrome.bottom_height, viewport_height)
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

    /// Toggle the right pane using its persisted/static width.
    pub fn toggle_right(&mut self, cx: &mut Context<Self>) {
        self.toggle_right_at_viewport(f32::INFINITY, cx);
    }

    fn toggle_right_in(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.toggle_right_at_viewport(window.bounds().size.width.as_f32(), cx);
    }

    fn toggle_right_at_viewport(&mut self, viewport_width: f32, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        let open_size = clamp_right_width(self.chrome.right_width, viewport_width);
        let from = effective_dimension(
            self.right_tween.as_ref(),
            Self::right_target_for_viewport(&self.chrome, viewport_width),
            open_size,
            now,
            reduced,
        );
        self.right_tween = Some(toggle_panel(
            &mut self.chrome.right_open,
            from,
            open_size,
            now,
        ));
        self.schedule_save(cx);
        cx.notify();
    }

    /// Toggle the bottom drawer using its persisted/static height.
    pub fn toggle_bottom(&mut self, cx: &mut Context<Self>) {
        self.toggle_bottom_at_viewport(f32::INFINITY, cx);
    }

    fn toggle_bottom_in(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.toggle_bottom_at_viewport(window.bounds().size.height.as_f32(), cx);
    }

    fn toggle_bottom_at_viewport(&mut self, viewport_height: f32, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        let open_size = clamp_bottom_height(self.chrome.bottom_height, viewport_height);
        let from = effective_dimension(
            self.bottom_tween.as_ref(),
            Self::bottom_target_for_viewport(&self.chrome, viewport_height),
            open_size,
            now,
            reduced,
        );
        self.bottom_tween = Some(toggle_panel(
            &mut self.chrome.bottom_open,
            from,
            open_size,
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
        if self.renaming_tab_id.as_deref() == Some(id) {
            self.clear_rename_session();
        }
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

    fn begin_rename(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(title) = self
            .tab_model
            .to_chrome_tabs()
            .0
            .into_iter()
            .find(|tab| tab.id == id)
            .map(|tab| tab.title)
        else {
            return;
        };

        // Select without an early notify so the first paint already includes the input.
        self.tab_model.select(id);
        self.sync_tab_model_to_chrome();
        self.renaming_tab_id = Some(id.to_owned());
        self.rename_commit_on_blur = false;

        let input = cx.new(|cx| InputState::new(window, cx).default_value(title));
        self._rename_subscriptions =
            vec![cx.subscribe(&input, |shell, _, event: &InputEvent, cx| {
                match event {
                    // Grow the chip as the draft changes (Input is size_full).
                    InputEvent::Change => cx.notify(),
                    InputEvent::PressEnter { .. } => shell.commit_rename(cx),
                    InputEvent::Blur if shell.rename_commit_on_blur => shell.commit_rename(cx),
                    _ => {}
                }
            })];
        self.rename_input = Some(input);
        self.schedule_save(cx);
        cx.notify();

        // Focus only after the Input is in the tree. Focusing earlier emits Blur
        // immediately and was aborting rename before keystrokes could apply.
        // Select-all on the *next frame* so the Input key_context is mounted —
        // dispatching SelectAll in the same defer (pre-paint) was a no-op, and
        // typing prepended instead of replacing.
        cx.defer_in(window, |shell, window, cx| {
            let Some(input) = shell.rename_input.clone() else {
                return;
            };
            input.update(cx, |state, cx| state.focus(window, cx));
            shell.rename_commit_on_blur = true;
            window.on_next_frame(|window, cx| {
                window.dispatch_action(Box::new(SelectAll), cx);
            });
        });
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let (Some(id), Some(input)) = (self.renaming_tab_id.clone(), self.rename_input.as_ref())
        else {
            self.clear_rename_session();
            cx.notify();
            return;
        };
        let value = input.read(cx).value();

        self.tab_model.rename(&id, value);
        self.sync_tab_model_to_chrome();
        self.schedule_save(cx);
        self.clear_rename_session();
        cx.notify();
    }

    fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.clear_rename_session();
        cx.notify();
    }

    fn clear_rename_session(&mut self) {
        self.renaming_tab_id = None;
        self.rename_input = None;
        self.rename_commit_on_blur = false;
        self._rename_subscriptions.clear();
    }

    fn render_tab_chip(
        &self,
        index: usize,
        tab: &crate::app::shell::ShellTabRecord,
        active: bool,
        colors: TabChipColors,
        tab_count: usize,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let TabChipColors { primary, muted, .. } = colors;
        let id = tab.id.clone();
        let renaming = tab_is_renaming(self.renaming_tab_id.as_deref(), &id);
        let rename_input = self.rename_input.clone().filter(|_| renaming);
        let rename_width = rename_input
            .as_ref()
            .map(|input| tab_chip_width_for_title(input.read(cx).value().as_ref()))
            .unwrap_or_else(|| tab_chip_width_for_title(&tab.title));
        let rename_id = id.clone();
        let close_id = id.clone();
        let middle_close_id = id;
        let on_chip_click = cx.listener(move |shell, event: &gpui::ClickEvent, window, cx| {
            cx.stop_propagation();
            if event.click_count() == 2 {
                shell.begin_rename(&rename_id, window, cx);
                return;
            }
            shell.select_tab(&rename_id, cx);
        });
        let on_close = cx.listener(move |shell, _event: &gpui::ClickEvent, _, cx| {
            stop_close_click_propagation(cx);
            shell.close_tab(&close_id, cx);
        });
        let on_middle_close = cx.listener(move |shell, _, window, cx| {
            window.prevent_default();
            shell.close_tab(&middle_close_id, cx);
        });
        let drag = TabDrag {
            index,
            title: tab.title.clone(),
            surface: colors.active_surface,
            foreground: primary,
            border: colors.border,
        };
        titlebar_chip(format!("shell-tab-{index}"), active, colors)
            .tab_index(index as isize)
            .focus_visible(|style| style.border_1().border_color(colors.border))
            .gap(px(6.0))
            .on_mouse_down(MouseButton::Middle, on_middle_close)
            .on_click(on_chip_click)
            .when(renaming, |chip| {
                // Explicit width: Input is size_full and otherwise collapses to
                // min_w, clipping/garbling keystrokes so they look like no-ops.
                chip.w(px(rename_width))
                    .on_action(cx.listener(|shell, _: &Escape, _, cx| shell.cancel_rename(cx)))
            })
            .when(!renaming, |chip| {
                chip.on_drag(drag, move |tab, _, _, cx| cx.new(|_| tab.clone()))
            })
            .drag_over::<TabDrag>(move |element, dragged, _, _| {
                if dragged.index == index {
                    element
                } else {
                    element.border_b_2().border_color(primary)
                }
            })
            .on_drop({
                let target = index;
                cx.listener(move |shell, dragged: &TabDrag, _, cx| {
                    shell.reorder_tab(
                        dragged.index,
                        tab_drop_index(dragged.index, target, tab_count),
                        cx,
                    );
                })
            })
            .when_some(rename_input, |chip, input| {
                chip.child(
                    Input::new(&input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false)
                        .with_size(gpui_component::Size::XSmall)
                        .flex_1()
                        .w_full()
                        .min_w(px(0.0)),
                )
            })
            .when(!renaming, |chip| {
                chip.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(if active { primary } else { muted })
                        .child(tab.title.clone()),
                )
            })
            .when(!renaming, |chip| {
                chip.child(
                    div()
                        .invisible()
                        .group_hover("shell-tab", |style| style.visible())
                        .child(
                            Button::new(format!("shell-tab-close-{index}"))
                                .ghost()
                                .compact()
                                .label("×")
                                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                })
                                .on_click(on_close),
                        ),
                )
            })
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

        let viewport = window.bounds().size;
        let viewport_width = viewport.width.as_f32();
        let viewport_height = viewport.height.as_f32();
        let left_width = eval_tween(self.left_tween.as_ref(), self.left_target(), now, reduced);
        let right_cap = clamp_right_width(self.chrome.right_width, viewport_width);
        let right_width = effective_dimension(
            self.right_tween.as_ref(),
            Self::right_target_for_viewport(&self.chrome, viewport_width),
            right_cap,
            now,
            reduced,
        );
        let bottom_cap = clamp_bottom_height(self.chrome.bottom_height, viewport_height);
        let bottom_height = effective_dimension(
            self.bottom_tween.as_ref(),
            Self::bottom_target_for_viewport(&self.chrome, viewport_height),
            bottom_cap,
            now,
            reduced,
        );
        if self.left_tween.is_some() || self.right_tween.is_some() || self.bottom_tween.is_some() {
            window.request_animation_frame();
        }

        let background = self.theme.surface(BackgroundToken::Primary);
        let panel_background = self.theme.surface(BackgroundToken::Secondary);
        let titlebar_background = self.theme.surface(BackgroundToken::Tertiary);
        let foreground_primary = self.theme.foreground(ForegroundToken::Primary);
        let foreground_secondary = self.theme.foreground(ForegroundToken::Secondary);
        let foreground_muted = self.theme.foreground(ForegroundToken::Muted);
        let handle_border = self.theme.border_token(BorderToken::Strong);
        let tab_chip_colors = TabChipColors {
            titlebar: titlebar_background,
            active_surface: panel_background,
            primary: foreground_primary,
            muted: foreground_muted,
            border: handle_border,
        };
        let tab_fade_state = self.tab_fade_state;

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
                stub_region("LEFT", panel_background, foreground_muted)
                    .w(px(self.chrome.left_width))
                    .h_full(),
            );
        let right = div()
            .w(px(right_width))
            .h_full()
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                stub_region("RIGHT", panel_background, foreground_muted)
                    .w(px(self.chrome.right_width))
                    .h_full(),
            );
        let main = div()
            .flex()
            .items_center()
            .justify_center()
            .flex_1()
            .min_w(px(0.0))
            .bg(background)
            .text_size(px(SHELL_TEXT.size()))
            .line_height(relative(SHELL_TEXT.line_height()))
            .text_color(foreground_secondary)
            .child(format!("MAIN · {active_title}"));
        let bottom = div()
            .w_full()
            .h(px(bottom_height))
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                stub_region("BOTTOM", panel_background, foreground_muted)
                    .w_full()
                    .h(px(self.chrome.bottom_height)),
            );

        let on_left_toggle = cx.listener(|shell, _, _, cx| shell.toggle_left(cx));
        let on_right_toggle = cx.listener(|shell, _, window, cx| shell.toggle_right_in(window, cx));
        let on_bottom_toggle =
            cx.listener(|shell, _, window, cx| shell.toggle_bottom_in(window, cx));
        let on_add_tab = cx.listener(|shell, _, _, cx| shell.add_stub_tab(cx));
        let shell_entity = cx.entity();

        let on_titlebar_mouse_down =
            cx.listener(|shell, _, _, _| shell.titlebar_drag_pending = true);
        let on_titlebar_mouse_up =
            cx.listener(|shell, _, _, _| shell.titlebar_drag_pending = false);
        let on_titlebar_mouse_down_out =
            cx.listener(|shell, _, _, _| shell.titlebar_drag_pending = false);
        let on_titlebar_mouse_up_out =
            cx.listener(|shell, _, _, _| shell.titlebar_drag_pending = false);
        let on_titlebar_mouse_move = cx.listener(|shell, event: &MouseMoveEvent, window, _| {
            if shell.titlebar_drag_pending && event.pressed_button == Some(MouseButton::Left) {
                shell.titlebar_drag_pending = false;
                window.start_window_move();
            }
        });

        let tab_count = tabs.len();
        let tab_items = tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                self.render_tab_chip(
                    index,
                    tab,
                    tab.id == active_id,
                    tab_chip_colors,
                    tab_count,
                    cx,
                )
            })
            .collect::<Vec<_>>();

        let on_trailing_tab_drop = cx.listener(move |shell, dragged: &TabDrag, _, cx| {
            shell.reorder_tab(
                dragged.index,
                tab_drop_index(dragged.index, tab_count, tab_count),
                cx,
            );
        });
        let fade_state_canvas = canvas(
            |_, _, _| (),
            move |_, _, _, cx| {
                shell_entity.update(cx, |shell, cx| {
                    let offset_x = shell.tab_bar_scroll_handle.offset().x.as_f32();
                    let max_offset_x = shell.tab_bar_scroll_handle.max_offset().x.as_f32();
                    if update_tab_fade_state(&mut shell.tab_fade_state, offset_x, max_offset_x) {
                        cx.notify();
                    }
                });
            },
        )
        .absolute()
        .size_full();

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

        let sidebar_seam = div().relative().w(px(0.0)).h_full().flex_shrink_0().child(
            div()
                .id("shell-sidebar-resize")
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(-5.0))
                .w(px(5.0))
                .occlude()
                .border_r_1()
                .border_color(handle_border.opacity(resize_handle_border_opacity(false)))
                .hover(|style| {
                    style.border_color(handle_border.opacity(resize_handle_border_opacity(true)))
                })
                .cursor_col_resize()
                .on_drag(SidebarResize, |_, _, _, cx| cx.new(|_| SidebarResize))
                .on_drag_move(on_sidebar_drag)
                .on_click(on_sidebar_reset),
        );
        let right_seam = div().relative().w(px(0.0)).h_full().flex_shrink_0().child(
            div()
                .id("shell-right-resize")
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(0.0))
                .w(px(5.0))
                .occlude()
                .border_l_1()
                .border_color(handle_border.opacity(resize_handle_border_opacity(false)))
                .hover(|style| {
                    style.border_color(handle_border.opacity(resize_handle_border_opacity(true)))
                })
                .cursor_col_resize()
                .on_drag(RightResize, |_, _, _, cx| cx.new(|_| RightResize))
                .on_drag_move(on_right_drag)
                .on_click(on_right_reset),
        );
        let bottom_seam = div().relative().w_full().h(px(0.0)).flex_shrink_0().child(
            div()
                .id("shell-bottom-resize")
                .absolute()
                .left_0()
                .right_0()
                .top(px(-5.0))
                .h(px(5.0))
                .occlude()
                .border_b_1()
                .border_color(handle_border.opacity(resize_handle_border_opacity(false)))
                .hover(|style| {
                    style.border_color(handle_border.opacity(resize_handle_border_opacity(true)))
                })
                .cursor_row_resize()
                .on_drag(BottomResize, |_, _, _, cx| cx.new(|_| BottomResize))
                .on_drag_move(on_bottom_drag)
                .on_click(on_bottom_reset),
        );

        // Tabs stay immediately right of traffic lights regardless of left panel width.
        let titlebar_leading_inset = TITLEBAR_CONTROLS_INSET;

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
                            .min_w(px(0.0))
                            .h_full()
                            .overflow_hidden()
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
                    .child(
                        div()
                            .id("shell-titlebar-drag")
                            .debug_selector(|| "shell-titlebar-drag".into())
                            .size_full()
                            .window_control_area(WindowControlArea::Drag)
                            .on_mouse_down(MouseButton::Left, on_titlebar_mouse_down)
                            .on_mouse_up(MouseButton::Left, on_titlebar_mouse_up)
                            .on_mouse_down_out(on_titlebar_mouse_down_out)
                            .on_mouse_up_out(MouseButton::Left, on_titlebar_mouse_up_out)
                            .on_mouse_move(on_titlebar_mouse_move)
                            .on_click(|event, window, _| {
                                if event.click_count() == 2 {
                                    window.titlebar_double_click();
                                }
                            })
                            .child(
                                div()
                                    .size_full()
                                    .flex()
                                    .items_center()
                                    .child(
                                        div()
                                            .id("shell-titlebar-leading")
                                            .debug_selector(|| "shell-titlebar-leading".into())
                                            .flex_shrink_0()
                                            .w(px(titlebar_leading_inset))
                                            .h_full(),
                                    )
                                    .child(
                                        div()
                                            .relative()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .h_full()
                                            .overflow_hidden()
                                            .child(
                                                div()
                                                    .id("shell-tab-strip")
                                                    .relative()
                                                    .h_full()
                                                    .overflow_hidden()
                                                    .child(
                                                        div()
                                                            .id("shell-tab-scroll-content")
                                                            .debug_selector(|| {
                                                                "shell-tab-scroll-content".into()
                                                            })
                                                            .h_full()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(TAB_CHIP_GAP))
                                                            .overflow_x_scroll()
                                                            .track_scroll(
                                                                &self.tab_bar_scroll_handle,
                                                            )
                                                            .children(tab_items)
                                                            .child(
                                                                div()
                                                                    .id("shell-tab-trailing-drop")
                                                                    .h_full()
                                                                    .w(px(32.0))
                                                                    .flex_shrink_0()
                                                                    .block_mouse_except_scroll()
                                                                    .on_mouse_down(
                                                                        MouseButton::Left,
                                                                        |_, window, cx| {
                                                                            window
                                                                                .prevent_default();
                                                                            cx.stop_propagation();
                                                                        },
                                                                    )
                                                                    .on_drop(on_trailing_tab_drop),
                                                            ),
                                                    )
                                                    .child(fade_state_canvas)
                                                    .when(tab_fade_state.left, |strip| {
                                                        strip.child(
                                                            div()
                                                                .absolute()
                                                                .left_0()
                                                                .top_0()
                                                                .bottom_0()
                                                                .w(px(TAB_FADE_WIDTH))
                                                                .bg(linear_gradient(
                                                                    90.0,
                                                                    linear_color_stop(
                                                                        titlebar_background,
                                                                        0.0,
                                                                    ),
                                                                    linear_color_stop(
                                                                        titlebar_background
                                                                            .opacity(0.0),
                                                                        1.0,
                                                                    ),
                                                                )),
                                                        )
                                                    })
                                                    .when(tab_fade_state.right, |strip| {
                                                        strip.child(
                                                            div()
                                                                .absolute()
                                                                .right_0()
                                                                .top_0()
                                                                .bottom_0()
                                                                .w(px(TAB_FADE_WIDTH))
                                                                .bg(linear_gradient(
                                                                    90.0,
                                                                    linear_color_stop(
                                                                        titlebar_background
                                                                            .opacity(0.0),
                                                                        0.0,
                                                                    ),
                                                                    linear_color_stop(
                                                                        titlebar_background,
                                                                        1.0,
                                                                    ),
                                                                )),
                                                        )
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("shell-titlebar-controls")
                                            .debug_selector(|| "shell-titlebar-controls".into())
                                            .flex()
                                            .flex_shrink_0()
                                            .items_center()
                                            .gap(px(4.0))
                                            .h_full()
                                            .occlude()
                                            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                                window.prevent_default();
                                                cx.stop_propagation();
                                            })
                                            .child(
                                                titlebar_control_button(
                                                    "shell-left-toggle",
                                                    "Left",
                                                    self.chrome.left_open,
                                                )
                                                .on_click(on_left_toggle),
                                            )
                                            .child(
                                                titlebar_control_button(
                                                    "shell-right-toggle",
                                                    "Right",
                                                    self.chrome.right_open,
                                                )
                                                .on_click(on_right_toggle),
                                            )
                                            .child(
                                                titlebar_control_button(
                                                    "shell-bottom-toggle",
                                                    "Bottom",
                                                    self.chrome.bottom_open,
                                                )
                                                .on_click(on_bottom_toggle),
                                            )
                                            .child(
                                                titlebar_control_button(
                                                    "shell-tab-add",
                                                    "+",
                                                    false,
                                                )
                                                .on_click(on_add_tab),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("shell-traffic-light-guard")
                            .debug_selector(|| "shell-traffic-light-guard".into())
                            .absolute()
                            .top_0()
                            .left_0()
                            .w(px(TITLEBAR_CONTROLS_INSET))
                            .h(px(TITLEBAR_HEIGHT))
                            .bg(titlebar_background),
                    ),
            )
    }
}

fn titlebar_control_button(id: &'static str, label: &'static str, selected: bool) -> Button {
    Button::new(id)
        .ghost()
        .compact()
        .xsmall()
        .text_size(px(SHELL_TEXT.size()))
        .line_height(relative(SHELL_TEXT.line_height()))
        .selected(selected)
        .label(label)
}

fn titlebar_chip(
    id: impl Into<std::string::String>,
    active: bool,
    colors: TabChipColors,
) -> gpui::Stateful<gpui::Div> {
    let TabChipColors {
        titlebar,
        active_surface,
        primary,
        muted,
        border: _,
    } = colors;
    div()
        .id(id.into())
        .group("shell-tab")
        .h_full()
        .min_w(px(TAB_CHIP_MIN_WIDTH))
        .max_w(px(TAB_CHIP_MAX_WIDTH))
        .px(px(8.0))
        .text_size(px(SHELL_TEXT.size()))
        .line_height(relative(SHELL_TEXT.line_height()))
        .flex()
        .items_center()
        .flex_shrink_0()
        .cursor_pointer()
        .block_mouse_except_scroll()
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .active(|style| style.opacity(press_feedback_opacity(true)))
        .border_b_1()
        .border_color(if active { primary } else { titlebar })
        .when(active, |chip| chip.bg(active_surface))
        .text_color(if active { primary } else { muted })
}

fn tab_chip_width_for_title_len(estimated_text_px: f32) -> f32 {
    estimated_text_px.clamp(TAB_CHIP_MIN_WIDTH, TAB_CHIP_MAX_WIDTH)
}

/// Estimate chip width for a title so the rename Input has a real box to paint
/// into. `Input` uses `size_full`; without an explicit chip width it collapses
/// to the min floor and typed characters clip/garble instead of showing.
fn tab_chip_width_for_title(title: &str) -> f32 {
    // MonoSm is 12px; ~0.62em approximates monospaced glyph advance.
    const CHAR_EM: f32 = 0.62;
    // Chip horizontal padding (8px each side) + room for caret while typing.
    const CHIP_PAD_AND_CARET: f32 = 16.0 + 12.0;
    let text_px = title.chars().count() as f32 * SHELL_TEXT.size() * CHAR_EM;
    tab_chip_width_for_title_len(text_px + CHIP_PAD_AND_CARET)
}

fn tab_is_renaming(renaming_tab_id: Option<&str>, tab_id: &str) -> bool {
    renaming_tab_id == Some(tab_id)
}

fn tab_drop_index(from: usize, target: usize, tab_count: usize) -> usize {
    if target >= tab_count {
        return tab_count.saturating_sub(1);
    }
    if from < target {
        target.saturating_sub(1)
    } else {
        target
    }
}

fn tab_fade_visibility(offset_x: f32, max_offset_x: f32) -> (bool, bool) {
    const EDGE_EPSILON: f32 = 0.5;

    if max_offset_x <= EDGE_EPSILON {
        return (false, false);
    }

    (
        offset_x < -EDGE_EPSILON,
        offset_x > -max_offset_x + EDGE_EPSILON,
    )
}

fn update_tab_fade_state(state: &mut TabFadeState, offset_x: f32, max_offset_x: f32) -> bool {
    let (left, right) = tab_fade_visibility(offset_x, max_offset_x);
    let next = TabFadeState { left, right };
    if *state == next {
        false
    } else {
        *state = next;
        true
    }
}

fn stop_close_click_propagation(cx: &mut App) {
    cx.stop_propagation();
}

fn toggle_panel(open: &mut bool, from: f32, open_size: f32, started: Instant) -> DimTween {
    *open = !*open;
    DimTween {
        from,
        to: if *open { open_size } else { 0.0 },
        started,
    }
}

/// Evaluate a panel dimension while keeping an in-flight tween inside the
/// current viewport cap. The persisted open size is not rewritten when the
/// viewport becomes smaller.
fn effective_dimension(
    tween: Option<&DimTween>,
    target: f32,
    cap: f32,
    now: Instant,
    reduced_motion: bool,
) -> f32 {
    eval_tween(tween, target, now, reduced_motion).clamp(0.0, cap.max(0.0))
}

/// Dragging a seam below this size collapses the panel instead of leaving a
/// near-invisible strip. The last usable size is kept so toggle-open restores
/// a visible panel.
const PANEL_COLLAPSE_THRESHOLD: f32 = 48.0;

fn resize_sidebar(chrome: &mut ShellChrome, proposed_width: f32) {
    let width = clamp_sidebar_width(proposed_width);
    if width < PANEL_COLLAPSE_THRESHOLD {
        chrome.left_open = false;
        if chrome.left_width < PANEL_COLLAPSE_THRESHOLD {
            chrome.left_width = SIDEBAR_DEFAULT;
        }
        return;
    }
    chrome.left_width = width;
    chrome.left_open = true;
}

fn resize_right(chrome: &mut ShellChrome, proposed_width: f32, viewport_width: f32) {
    let width = clamp_right_width(proposed_width, viewport_width);
    if width < PANEL_COLLAPSE_THRESHOLD {
        chrome.right_open = false;
        if chrome.right_width < PANEL_COLLAPSE_THRESHOLD {
            chrome.right_width = RIGHT_DEFAULT;
        }
        return;
    }
    chrome.right_width = width;
    chrome.right_open = true;
}

fn resize_bottom(chrome: &mut ShellChrome, proposed_height: f32, viewport_height: f32) {
    let height = clamp_bottom_height(proposed_height, viewport_height);
    if height < PANEL_COLLAPSE_THRESHOLD {
        chrome.bottom_open = false;
        if chrome.bottom_height < PANEL_COLLAPSE_THRESHOLD {
            chrome.bottom_height = BOTTOM_DEFAULT;
        }
        return;
    }
    chrome.bottom_height = height;
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

const PRESS_FEEDBACK_OPACITY: f32 = 0.84;

fn press_feedback_opacity(pressed: bool) -> f32 {
    if pressed { PRESS_FEEDBACK_OPACITY } else { 1.0 }
}

const HANDLE_BORDER_IDLE_OPACITY: f32 = 0.28;
const HANDLE_BORDER_HOVER_OPACITY: f32 = 1.0;

fn resize_handle_border_opacity(hovered: bool) -> f32 {
    if hovered {
        HANDLE_BORDER_HOVER_OPACITY
    } else {
        HANDLE_BORDER_IDLE_OPACITY
    }
}

fn stub_region(label: &'static str, background: gpui::Hsla, foreground: gpui::Hsla) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .bg(background)
        .text_size(px(SHELL_TEXT.size()))
        .line_height(relative(SHELL_TEXT.line_height()))
        .text_color(foreground)
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::{
        BOTTOM_DEFAULT, SIDEBAR_DEFAULT, Shell, TAB_CHIP_MAX_WIDTH, TAB_CHIP_MIN_WIDTH,
        TAB_FADE_WIDTH, TITLEBAR_CONTROLS_INSET, TabFadeState, press_feedback_opacity,
        reset_bottom, reset_right, reset_sidebar, resize_bottom, resize_handle_border_opacity,
        resize_right, resize_sidebar, tab_chip_width_for_title, tab_chip_width_for_title_len,
        tab_drop_index, tab_fade_visibility, tab_is_renaming, toggle_panel, update_tab_fade_state,
    };
    use crate::app::shell::{RIGHT_DEFAULT, ShellChrome};
    use gpui::{
        AppContext as _, Bounds, Context, InteractiveElement, IntoElement, Modifiers, MouseButton,
        ParentElement, Render, StatefulInteractiveElement, Styled, TestAppContext,
        VisualTestContext, Window, WindowBounds, WindowOptions, div, point, px, size,
    };
    use std::time::Instant;

    struct CloseRoutingProbe {
        parent_clicks: usize,
        close_clicks: usize,
    }

    impl Render for CloseRoutingProbe {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("probe-parent")
                .w(px(100.0))
                .h(px(100.0))
                .on_click(cx.listener(|probe, _, _, _| probe.parent_clicks += 1))
                .child(
                    div()
                        .id("probe-close")
                        .absolute()
                        .left_0()
                        .top_0()
                        .w(px(20.0))
                        .h(px(20.0))
                        .on_click(cx.listener(|probe, _, _, cx| {
                            probe.close_clicks += 1;
                            super::stop_close_click_propagation(cx);
                        })),
                )
        }
    }

    #[gpui::test]
    fn close_click_stops_parent_selection_event(cx: &mut TestAppContext) {
        let window_handle = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| {
                cx.new(|_| CloseRoutingProbe {
                    parent_clicks: 0,
                    close_clicks: 0,
                })
            })
            .unwrap()
        });

        cx.run_until_parked();
        let mut window = VisualTestContext::from_window(*window_handle, cx);
        window.simulate_click(point(px(10.0), px(10.0)), Modifiers::default());
        window_handle
            .update(cx, |probe, _, _| {
                assert_eq!(probe.close_clicks, 1);
                assert_eq!(probe.parent_clicks, 0);
            })
            .unwrap();
    }

    #[gpui::test]
    fn first_tab_clears_traffic_light_inset(cx: &mut TestAppContext) {
        let window_handle = cx.update(|cx| {
            gpui_component::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(1280.0), px(800.0)),
                })),
                ..Default::default()
            };
            cx.open_window(options, |_, cx| cx.new(|_| test_shell()))
                .unwrap()
        });

        cx.run_until_parked();
        let mut window = VisualTestContext::from_window(*window_handle, cx);
        let leading_bounds = window
            .debug_bounds("shell-titlebar-leading")
            .expect("leading titlebar band should be laid out");
        let tab_bounds = window
            .debug_bounds("shell-tab-scroll-content")
            .expect("tab scroll content should be laid out");
        assert_eq!(leading_bounds.size.width.as_f32(), TITLEBAR_CONTROLS_INSET);
        assert!(
            tab_bounds.origin.x.as_f32() >= TITLEBAR_CONTROLS_INSET - 0.5,
            "tab strip overlaps traffic lights"
        );
        assert_eq!(tab_bounds.origin.x.as_f32(), TITLEBAR_CONTROLS_INSET);
    }

    #[gpui::test]
    fn shell_exposes_a_real_empty_titlebar_drag_region(cx: &mut TestAppContext) {
        let window_handle = cx.update(|cx| {
            gpui_component::init(cx);
            cx.open_window(Default::default(), |_, cx| cx.new(|_| test_shell()))
                .unwrap()
        });

        cx.run_until_parked();
        let mut window = VisualTestContext::from_window(*window_handle, cx);

        assert!(window.debug_bounds("shell-titlebar-drag").is_some());
    }

    #[gpui::test]
    fn tab_chip_does_not_arm_titlebar_drag(cx: &mut TestAppContext) {
        let window_handle = cx.update(|cx| {
            gpui_component::init(cx);
            cx.open_window(Default::default(), |_, cx| cx.new(|_| test_shell()))
                .unwrap()
        });

        cx.run_until_parked();
        let mut window = VisualTestContext::from_window(*window_handle, cx);
        let tab_bounds = window
            .debug_bounds("shell-tab-scroll-content")
            .expect("tab scroll content should be laid out");
        window.simulate_mouse_down(
            tab_bounds.origin + point(px(12.0), px(12.0)),
            MouseButton::Left,
            Modifiers::default(),
        );

        window_handle
            .update(cx, |shell, _, _| assert!(!shell.titlebar_drag_pending))
            .unwrap();
    }

    #[gpui::test]
    fn titlebar_toggle_area_does_not_arm_titlebar_drag(cx: &mut TestAppContext) {
        let window_handle = cx.update(|cx| {
            gpui_component::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(640.0), px(320.0)),
                })),
                ..Default::default()
            };
            cx.open_window(options, |_, cx| cx.new(|_| test_shell()))
                .unwrap()
        });

        cx.run_until_parked();
        let mut window = VisualTestContext::from_window(*window_handle, cx);
        let toggle_bounds = window
            .debug_bounds("shell-titlebar-controls")
            .expect("titlebar controls should be inside the explicit test window");
        window.simulate_mouse_down(
            toggle_bounds.center(),
            MouseButton::Left,
            Modifiers::default(),
        );

        window_handle
            .update(cx, |shell, _, _| assert!(!shell.titlebar_drag_pending))
            .unwrap();
    }

    #[test]
    fn press_feedback_uses_a_brief_near_imperceptible_opacity_flash() {
        assert_eq!(press_feedback_opacity(false), 1.0);
        assert_eq!(press_feedback_opacity(true), 0.84);
    }

    #[test]
    fn resize_handle_hover_uses_a_stronger_border_than_idle() {
        assert_eq!(resize_handle_border_opacity(false), 0.28);
        assert_eq!(resize_handle_border_opacity(true), 1.0);
        assert!(resize_handle_border_opacity(true) > resize_handle_border_opacity(false));
    }

    #[test]
    fn tab_fade_state_refreshes_after_layout_and_only_notifies_on_change() {
        let mut state = TabFadeState::default();

        assert!(update_tab_fade_state(&mut state, -40.0, 120.0));
        assert_eq!(
            state,
            TabFadeState {
                left: true,
                right: true
            }
        );
        assert!(!update_tab_fade_state(&mut state, -40.0, 120.0));
        assert!(update_tab_fade_state(&mut state, -120.0, 120.0));
        assert_eq!(
            state,
            TabFadeState {
                left: true,
                right: false
            }
        );
    }

    #[test]
    fn tab_fade_width_matches_binding() {
        assert_eq!(TAB_FADE_WIDTH, 36.0);
    }

    #[test]
    fn tab_chip_width_clamps_to_soft_floor_and_ceiling() {
        assert_eq!(tab_chip_width_for_title_len(10.0), TAB_CHIP_MIN_WIDTH);
        assert_eq!(tab_chip_width_for_title_len(120.0), 120.0);
        assert_eq!(tab_chip_width_for_title_len(999.0), TAB_CHIP_MAX_WIDTH);
    }

    #[test]
    fn tab_chip_width_for_title_grows_with_typed_text() {
        let short = tab_chip_width_for_title("New Tab");
        let longer = tab_chip_width_for_title("testNew Tab");
        assert!(short >= TAB_CHIP_MIN_WIDTH);
        assert!(longer > short);
        assert!(longer <= TAB_CHIP_MAX_WIDTH);
        assert_eq!(
            tab_chip_width_for_title(""),
            TAB_CHIP_MIN_WIDTH,
            "empty draft still keeps a usable chip"
        );
        assert_eq!(
            tab_chip_width_for_title(&"x".repeat(200)),
            TAB_CHIP_MAX_WIDTH
        );
    }

    #[test]
    fn left_target_zero_when_closed() {
        let chrome = ShellChrome {
            left_open: false,
            ..Default::default()
        };

        assert_eq!(Shell::left_target_for(&chrome), 0.0);
    }

    #[test]
    fn left_target_uses_width_when_open() {
        let chrome = ShellChrome {
            left_width: 312.0,
            ..Default::default()
        };

        assert_eq!(Shell::left_target_for(&chrome), 312.0);
    }

    #[test]
    fn right_and_bottom_targets_follow_open_flags() {
        let open_chrome = ShellChrome {
            right_open: true,
            bottom_open: true,
            ..Default::default()
        };

        assert_eq!(
            Shell::right_target_for(&open_chrome),
            open_chrome.right_width
        );
        assert_eq!(
            Shell::bottom_target_for(&open_chrome),
            open_chrome.bottom_height
        );

        let closed_chrome = ShellChrome {
            right_open: false,
            bottom_open: false,
            ..Default::default()
        };
        assert_eq!(Shell::right_target_for(&closed_chrome), 0.0);
        assert_eq!(Shell::bottom_target_for(&closed_chrome), 0.0);
    }

    #[test]
    fn live_right_target_caps_without_mutating_persisted_width() {
        let chrome = ShellChrome {
            right_open: true,
            right_width: 400.0,
            ..Default::default()
        };

        assert_eq!(Shell::right_target_for_viewport(&chrome, 300.0), 300.0);
        assert_eq!(Shell::right_target_for_viewport(&chrome, 1280.0), 400.0);
        assert_eq!(chrome.right_width, 400.0);
    }

    #[test]
    fn live_bottom_target_caps_without_mutating_persisted_height() {
        let chrome = ShellChrome {
            bottom_open: true,
            bottom_height: 400.0,
            ..Default::default()
        };

        assert_eq!(Shell::bottom_target_for_viewport(&chrome, 100.0), 100.0);
        assert_eq!(Shell::bottom_target_for_viewport(&chrome, 800.0), 400.0);
        assert_eq!(chrome.bottom_height, 400.0);
    }

    #[test]
    fn live_cap_clamps_an_in_flight_tween_after_resize() {
        let started = Instant::now();
        let tween = super::DimTween {
            from: 400.0,
            to: 400.0,
            started,
        };

        assert_eq!(
            super::effective_dimension(Some(&tween), 156.0, 156.0, started, false),
            156.0
        );
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

        const {
            assert!(TITLEBAR_CONTROLS_INSET >= NATIVE_TRAFFIC_LIGHT_RIGHT_EDGE);
        }
    }

    #[test]
    fn mutated_shell_chrome_round_trips_through_json() {
        let mut chrome = ShellChrome {
            left_width: 333.0,
            right_open: true,
            ..Default::default()
        };
        chrome.tabs.push(super::super::ShellTabRecord {
            id: "tab-2".into(),
            title: "Second".into(),
        });
        chrome.active_tab_id = "tab-2".into();

        let json = serde_json::to_string(&chrome).expect("serialize");
        let restored: ShellChrome = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored, chrome);
    }

    #[test]
    fn sidebar_resize_accepts_any_non_negative_width_and_opens_panel() {
        let mut chrome = ShellChrome {
            left_open: false,
            right_open: true,
            ..Default::default()
        };
        let before_right = chrome.right_width;

        resize_sidebar(&mut chrome, 999.0);

        assert_eq!(chrome.left_width, 999.0);
        assert!(chrome.left_open);
        assert_eq!(chrome.right_width, before_right);
    }

    #[test]
    fn sidebar_resize_near_end_collapses_and_keeps_last_usable_width() {
        let mut chrome = ShellChrome {
            left_width: 256.0,
            left_open: true,
            ..Default::default()
        };

        resize_sidebar(&mut chrome, 120.0);
        assert_eq!(chrome.left_width, 120.0);
        assert!(chrome.left_open);

        resize_sidebar(&mut chrome, 20.0);
        assert!(!chrome.left_open);
        assert_eq!(chrome.left_width, 120.0);
    }

    #[test]
    fn right_resize_uses_live_viewport_width_and_opens_panel() {
        let mut chrome = ShellChrome {
            right_open: false,
            ..Default::default()
        };

        resize_right(&mut chrome, 900.0, 800.0);

        assert_eq!(chrome.right_width, 800.0);
        assert!(chrome.right_open);
    }

    #[test]
    fn right_resize_near_end_collapses_and_keeps_last_usable_width() {
        let mut chrome = ShellChrome {
            right_width: 320.0,
            right_open: true,
            ..Default::default()
        };

        resize_right(&mut chrome, 160.0, 1280.0);
        resize_right(&mut chrome, 10.0, 1280.0);

        assert!(!chrome.right_open);
        assert_eq!(chrome.right_width, 160.0);
    }

    #[test]
    fn bottom_resize_uses_live_viewport_height_and_opens_panel() {
        let mut chrome = ShellChrome {
            bottom_open: false,
            ..Default::default()
        };

        resize_bottom(&mut chrome, 900.0, 800.0);

        assert_eq!(chrome.bottom_height, 800.0);
        assert!(chrome.bottom_open);
    }

    #[test]
    fn bottom_resize_near_end_collapses_and_restores_default_if_size_was_tiny() {
        let mut chrome = ShellChrome {
            bottom_height: 30.0,
            bottom_open: true,
            ..Default::default()
        };

        resize_bottom(&mut chrome, 10.0, 800.0);

        assert!(!chrome.bottom_open);
        assert_eq!(chrome.bottom_height, BOTTOM_DEFAULT);
    }

    #[test]
    fn resize_reset_helpers_restore_persisted_defaults_and_open_panels() {
        let mut chrome = ShellChrome {
            left_width: 390.0,
            right_width: 450.0,
            bottom_height: 410.0,
            left_open: false,
            right_open: false,
            bottom_open: false,
            ..Default::default()
        };

        reset_sidebar(&mut chrome);
        reset_right(&mut chrome);
        reset_bottom(&mut chrome);

        assert_eq!(chrome.left_width, SIDEBAR_DEFAULT);
        assert_eq!(chrome.right_width, RIGHT_DEFAULT);
        assert_eq!(chrome.bottom_height, BOTTOM_DEFAULT);
        assert!(chrome.left_open && chrome.right_open && chrome.bottom_open);
    }

    #[test]
    fn reset_defaults_use_live_caps_only_for_rendered_targets() {
        let mut chrome = ShellChrome {
            right_width: 450.0,
            bottom_height: 410.0,
            ..Default::default()
        };

        reset_right(&mut chrome);
        reset_bottom(&mut chrome);

        assert_eq!(chrome.right_width, RIGHT_DEFAULT);
        assert_eq!(chrome.bottom_height, BOTTOM_DEFAULT);
        assert_eq!(Shell::right_target_for_viewport(&chrome, 300.0), 300.0);
        assert_eq!(Shell::bottom_target_for_viewport(&chrome, 100.0), 100.0);
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
    fn closing_sole_tab_clears_tab_strip() {
        let mut shell = test_shell();
        assert_eq!(shell.chrome.tabs.len(), 1);
        let sole_id = shell.chrome.active_tab_id.clone();

        shell.tab_model.close(&sole_id);
        shell.sync_tab_model_to_chrome();

        assert!(shell.chrome.tabs.is_empty());
        assert!(shell.chrome.active_tab_id.is_empty());
    }

    #[gpui::test]
    fn closing_renamed_tab_clears_rename_session(cx: &mut TestAppContext) {
        let window_handle = cx.update(|cx| {
            gpui_component::init(cx);
            cx.open_window(Default::default(), |_, cx| cx.new(|_| test_shell()))
                .unwrap()
        });
        let renamed_id = window_handle
            .update(cx, |shell, _, _| {
                let id = shell.chrome.active_tab_id.clone();
                shell.renaming_tab_id = Some(id.clone());
                id
            })
            .unwrap();

        window_handle
            .update(cx, |shell, _, cx| shell.close_tab(&renamed_id, cx))
            .unwrap();

        window_handle
            .update(cx, |shell, _, _| {
                assert!(shell.renaming_tab_id.is_none());
                assert!(shell.rename_input.is_none());
                assert!(shell._rename_subscriptions.is_empty());
            })
            .unwrap();
    }

    #[test]
    fn clear_rename_session_discards_rename_state() {
        let mut shell = test_shell();
        shell.renaming_tab_id = Some("tab-1".into());

        shell.clear_rename_session();

        assert!(shell.renaming_tab_id.is_none());
        assert!(shell.rename_input.is_none());
        assert!(shell._rename_subscriptions.is_empty());
    }

    #[test]
    fn tab_rename_state_only_matches_the_active_rename_session() {
        assert!(tab_is_renaming(Some("tab-1"), "tab-1"));
        assert!(!tab_is_renaming(Some("tab-1"), "tab-2"));
        assert!(!tab_is_renaming(None, "tab-1"));
    }

    #[test]
    fn tab_drop_index_accounts_for_removed_source_chip() {
        assert_eq!(tab_drop_index(0, 2, 2), 1);
        assert_eq!(tab_drop_index(2, 0, 3), 0);
        assert_eq!(tab_drop_index(1, 1, 3), 1);
    }

    #[test]
    fn tab_drop_index_supports_trailing_drop_target() {
        assert_eq!(tab_drop_index(0, 3, 3), 2);
        assert_eq!(tab_drop_index(2, 3, 3), 2);
    }

    #[test]
    fn tab_fade_visibility_tracks_scroll_edges_and_overflow() {
        assert_eq!(tab_fade_visibility(0.0, 0.0), (false, false));
        assert_eq!(tab_fade_visibility(0.0, 120.0), (false, true));
        assert_eq!(tab_fade_visibility(-40.0, 120.0), (true, true));
        assert_eq!(tab_fade_visibility(-120.0, 120.0), (true, false));
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
            tab_fade_state: super::TabFadeState::default(),
            titlebar_drag_pending: false,
            renaming_tab_id: None,
            rename_input: None,
            rename_commit_on_blur: false,
            _rename_subscriptions: Vec::new(),
        }
    }

    #[test]
    fn rename_blur_commit_starts_disarmed() {
        let shell = test_shell();
        assert!(!shell.rename_commit_on_blur);
    }

    #[test]
    fn clear_rename_session_disarms_blur_commit() {
        let mut shell = test_shell();
        shell.rename_commit_on_blur = true;
        shell.renaming_tab_id = Some("tab-1".into());

        shell.clear_rename_session();

        assert!(!shell.rename_commit_on_blur);
        assert!(shell.renaming_tab_id.is_none());
    }
}
