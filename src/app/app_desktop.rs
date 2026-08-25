//! **Facade** for the GPU runtime: boots preferences, opens one window, and routes
//! [`super::ActiveScreen`] without closing between onboarding and home.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    App, AppContext, Context, FocusHandle, IntoElement, ParentElement, Render, Styled, Task,
    WeakEntity, Window, WindowBounds, WindowOptions, div, px, size,
};
#[cfg(debug_assertions)]
use gpui::{InteractiveElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Point};
#[cfg(all(debug_assertions, target_os = "linux"))]
use gpui::{TitlebarOptions, point};
use gpui_component::Root;
use gpui_component::dock::DockAreaState;

use crate::shared::preferences::{FilePreferencesStore, PreferencesError, PreferencesStore};
use crate::shared::theme::{OpenCoreTheme, ThemeTransition, apply_nothing_theme};

use super::AppError;
use super::app_state::{ActiveScreen, AppState};
#[cfg(debug_assertions)]
use super::dev_reset::{DevResetCallbacks, DevResetState, dev_reset_fab};
use super::onboarding::{
    OnboardingCallbacks, OnboardingCommand, OnboardingOutcome, OnboardingUiState,
    onboarding_interactive_root, onboarding_screen, reduce_onboarding,
};
use super::shell::{DockSaveFn, ShellWorkspace, register_shell_panels};
use super::viewport::WindowViewport;
use super::window_placement::center_window;

const SHELL_SAVE_DEBOUNCE: Duration = Duration::from_millis(400);

#[derive(Debug, Default)]
struct PendingDockSave {
    latest: DockAreaState,
    dirty: bool,
}

impl PendingDockSave {
    fn set_latest(&mut self, layout: DockAreaState) {
        self.latest = layout;
        self.dirty = true;
    }

    fn take_dirty(&mut self) -> Option<DockAreaState> {
        if self.dirty {
            self.dirty = false;
            Some(self.latest.clone())
        } else {
            None
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    #[cfg(debug_assertions)]
    fn clear(&mut self) {
        self.dirty = false;
    }
}

fn flush_pending_shell_save(
    pending: &Rc<RefCell<PendingDockSave>>,
    store: &FilePreferencesStore,
    context: &str,
) {
    let Some(layout) = pending.borrow_mut().take_dirty() else {
        return;
    };
    let mut preferences = match store.load() {
        Ok(preferences) => preferences,
        Err(error) => {
            eprintln!("opencore: {context}: {error}");
            pending.borrow_mut().set_latest(layout);
            return;
        }
    };
    preferences.dock_layout = Some(layout.clone());
    if let Err(error) = store.save(&preferences) {
        eprintln!("opencore: {context}: {error}");
        pending.borrow_mut().set_latest(layout);
    }
}

/// Composition-root view: dispatches on [`ActiveScreen`] and owns persisted state.
pub struct OpenCoreApp {
    state: AppState,
    store: Arc<FilePreferencesStore>,
    focus_handle: FocusHandle,
    onboarding_ui: Option<OnboardingUiState>,
    shell: Option<gpui::Entity<ShellWorkspace>>,
    shell_save_task: Option<Task<()>>,
    pending_shell_save: Rc<RefCell<PendingDockSave>>,
    _shutdown_subscription: gpui::Subscription,
    _window_closed_subscription: gpui::Subscription,
    theme_transition: Option<ThemeTransition>,
    persistence_error: Option<String>,
    #[cfg(debug_assertions)]
    dev_reset_state: DevResetState,
}

impl OpenCoreApp {
    fn new(state: AppState, store: Arc<FilePreferencesStore>, cx: &mut Context<Self>) -> Self {
        let onboarding_ui = if state.active_screen == ActiveScreen::Onboarding {
            Some(OnboardingUiState::new())
        } else {
            None
        };
        let pending_shell_save = Rc::new(RefCell::new(PendingDockSave::default()));
        let pending_for_shutdown = pending_shell_save.clone();
        let store_for_shutdown = store.clone();
        // Register directly on App rather than through Context::on_app_quit:
        // GPUI clears window-owned entities before running quit futures, while
        // this shared state remains available to flush the latest dirty shell.
        let shutdown_subscription = App::on_app_quit(cx, move |_app| {
            let pending_for_shutdown = pending_for_shutdown.clone();
            let store_for_shutdown = store_for_shutdown.clone();
            async move {
                flush_pending_shell_save(
                    &pending_for_shutdown,
                    &store_for_shutdown,
                    "save shell on shutdown",
                );
            }
        });
        let pending_for_window_close = pending_shell_save.clone();
        let store_for_window_close = store.clone();
        let window_closed_subscription = App::on_window_closed(cx, move |_app, _window_id| {
            flush_pending_shell_save(
                &pending_for_window_close,
                &store_for_window_close,
                "save shell on window close",
            );
        });

        Self {
            state,
            store,
            focus_handle: cx.focus_handle(),
            onboarding_ui,
            shell: None,
            shell_save_task: None,
            pending_shell_save,
            _shutdown_subscription: shutdown_subscription,
            _window_closed_subscription: window_closed_subscription,
            theme_transition: None,
            persistence_error: None,
            #[cfg(debug_assertions)]
            dev_reset_state: DevResetState::default(),
        }
    }

    fn visual_theme(&self, now: Instant) -> OpenCoreTheme {
        let target = self.state.theme_mode();
        match self.theme_transition {
            Some(tx) if tx.is_active(now) => OpenCoreTheme::blended(target, tx.mix_light(now)),
            _ => OpenCoreTheme::resolve(target),
        }
    }

    fn settle_theme_transition(&mut self, now: Instant) {
        if self.theme_transition.is_some_and(|tx| !tx.is_active(now)) {
            self.theme_transition = None;
        }
    }

    fn sync_component_theme(&self, cx: &mut App) {
        apply_nothing_theme(self.state.theme_mode(), cx);
    }

    fn apply_resize_intent(&mut self, window: &mut Window, cx: &App) {
        if let Some(intent) = self.state.take_pending_window_resize() {
            let new_size = size(px(intent.width as f32), px(intent.height as f32));
            window.resize(new_size);
            center_window(window, new_size, cx);
        }
    }

    fn finish_screen_transition(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_resize_intent(window, cx);
        cx.notify();
    }

    fn ensure_onboarding_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ui) = self.onboarding_ui.as_mut() {
            ui.ensure_initial_focus(window, &self.focus_handle, cx);
        }
    }

    fn record_persistence_error(&mut self, context: &str, error: PreferencesError) {
        eprintln!("opencore: {context}: {error}");
        self.persistence_error = Some(format!("[ERROR: Could not save settings ({error})]"));
    }

    fn schedule_dock_layout_save(&mut self, layout: DockAreaState, cx: &mut Context<Self>) {
        self.state.preferences.dock_layout = Some(layout.clone());
        self.pending_shell_save.borrow_mut().set_latest(layout);
        self.shell_save_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(SHELL_SAVE_DEBOUNCE).await;
            let _ = view.update(cx, |app, _| app.flush_shell_save());
        }));
    }

    fn flush_shell_save(&mut self) {
        if !self.pending_shell_save.borrow().dirty {
            self.shell_save_task = None;
            return;
        }

        let layout = self.pending_shell_save.borrow().latest.clone();
        self.state.preferences.dock_layout = Some(layout);
        match self.store.save(&self.state.preferences) {
            Ok(()) => {
                self.pending_shell_save.borrow_mut().dirty = false;
                self.persistence_error = None;
            }
            Err(error) => {
                self.pending_shell_save.borrow_mut().mark_dirty();
                self.record_persistence_error("save dock layout", error);
            }
        }
        self.shell_save_task = None;
    }

    fn ensure_shell(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<ShellWorkspace> {
        if let Some(shell) = self.shell.as_ref() {
            return shell.clone();
        }

        let saved = self.state.preferences.dock_layout.clone();
        let view = cx.entity().downgrade();
        // Defer: ShellWorkspace::new may invoke `save` while OpenCoreApp is still
        // inside render/update (ensure_shell). Nested `view.update` panics.
        let save: DockSaveFn = Rc::new(move |layout, app| {
            let view = view.clone();
            app.defer(move |app| {
                let _ = view.update(app, |app, cx| {
                    app.schedule_dock_layout_save(layout, cx);
                });
            });
        });
        let shell = cx.new(|cx| ShellWorkspace::new(saved, save, window, cx));
        self.shell = Some(shell.clone());
        shell
    }

    fn apply_onboarding_command(
        &mut self,
        command: OnboardingCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let outcome = reduce_onboarding(command);
        match self
            .state
            .apply_onboarding_outcome(outcome, self.store.as_ref())
        {
            Ok(()) => {
                self.persistence_error = None;
                if outcome != OnboardingOutcome::Pending {
                    self.onboarding_ui = None;
                    self.finish_screen_transition(window, cx);
                }
            }
            Err(error) => {
                self.record_persistence_error("complete onboarding", error);
                cx.notify();
            }
        }
    }

    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let from = self.state.theme_mode();
        let next = from.toggle();
        match self.state.set_theme_mode(self.store.as_ref(), next) {
            Ok(()) => {
                self.persistence_error = None;
                self.theme_transition = Some(match self.theme_transition {
                    Some(mut tx) if tx.is_active(now) => {
                        tx.retarget(next, now);
                        tx
                    }
                    _ => ThemeTransition::start(from, next, now),
                });
                self.sync_component_theme(cx);
                cx.notify();
            }
            Err(error) => {
                self.record_persistence_error("save theme", error);
                cx.notify();
            }
        }
    }

    /// Resets persisted preferences to defaults and routes back to onboarding.
    ///
    /// Called by the debug reset overlay.
    #[cfg(debug_assertions)]
    fn reset_dev_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.reset_dev_data_state() {
            Ok(()) => {
                self.ensure_onboarding_focus(window, cx);
                self.finish_screen_transition(window, cx);
            }
            Err(error) => {
                self.record_persistence_error("reset dev data", error);
                cx.notify();
            }
        }
    }

    #[cfg(debug_assertions)]
    fn reset_dev_data_state(&mut self) -> Result<(), PreferencesError> {
        self.state.reset_persistent_data(self.store.as_ref())?;
        self.shell_save_task.take();
        self.pending_shell_save.borrow_mut().clear();
        self.shell = None;
        self.onboarding_ui = Some(OnboardingUiState::new());
        self.persistence_error = None;
        Ok(())
    }
}

impl OnboardingCallbacks {
    pub fn from_app(view: WeakEntity<OpenCoreApp>) -> Self {
        let on_enter = {
            let view = view.clone();
            Rc::new(move |window: &mut Window, cx: &mut App| {
                let _ = view.update(cx, |app, cx| {
                    app.apply_onboarding_command(OnboardingCommand::EnterPressed, window, cx);
                });
            })
        };
        let on_toggle_theme = {
            let view = view.clone();
            Rc::new(move |_: &mut Window, cx: &mut App| {
                let _ = view.update(cx, |app, cx| {
                    app.toggle_theme(cx);
                });
            })
        };

        Self {
            on_enter,
            on_toggle_theme,
        }
    }
}

#[cfg(debug_assertions)]
impl DevResetCallbacks {
    /// Constructs overlay callbacks that drive `OpenCoreApp`'s `DevResetState`.
    ///
    /// - `on_activate` (click without drag) calls `reset_dev_data`.
    /// - `on_drag_start` records the mouse-down position and current FAB origin.
    /// - `on_drag_move` updates the FAB origin using `damp_translation`.
    /// - `on_drag_end` checks click-vs-drag; if click, calls `on_activate`.
    pub fn from_app(view: WeakEntity<OpenCoreApp>, bounds: (f32, f32)) -> Self {
        let on_activate = {
            let view = view.clone();
            Rc::new(move |window: &mut Window, cx: &mut App| {
                let _ = view.update(cx, |app, cx| {
                    app.reset_dev_data(window, cx);
                });
            })
        };

        let on_drag_start = {
            let view = view.clone();
            Rc::new(
                move |event: &MouseDownEvent, _window: &mut Window, cx: &mut App| {
                    let _ = view.update(cx, |app, cx| {
                        app.dev_reset_state.dragging = true;
                        app.dev_reset_state.pointer_start = Some(event.position);
                        app.dev_reset_state.origin_at_drag_start = Some(app.dev_reset_state.origin);
                        cx.notify();
                    });
                },
            )
        };

        let (win_w, win_h) = bounds;
        let on_drag_move = {
            let view = view.clone();
            Rc::new(
                move |event: &MouseMoveEvent, _window: &mut Window, cx: &mut App| {
                    let _ = view.update(cx, |app, cx| {
                        let st = &mut app.dev_reset_state;
                        if !st.dragging {
                            return;
                        }
                        let (start, origin0) = match (st.pointer_start, st.origin_at_drag_start) {
                            (Some(s), Some(o)) => (s, o),
                            _ => return,
                        };
                        let dx = event.position.x.as_f32() - start.x.as_f32();
                        let dy = event.position.y.as_f32() - start.y.as_f32();
                        let proposed_x = origin0.x.as_f32() + dx;
                        let proposed_y = origin0.y.as_f32() + dy;
                        // Clamp with damping: FAB stays fully inside the window.
                        let min_x = 0.0;
                        let max_x = (win_w - super::dev_reset::FAB_WIDTH).max(0.0);
                        let min_y = 0.0;
                        let max_y = (win_h - super::dev_reset::FAB_HEIGHT).max(0.0);
                        st.origin = Point {
                            x: px(super::dev_reset::damp_translation(
                                origin0.x.as_f32(),
                                min_x,
                                max_x,
                                proposed_x,
                            )),
                            y: px(super::dev_reset::damp_translation(
                                origin0.y.as_f32(),
                                min_y,
                                max_y,
                                proposed_y,
                            )),
                        };
                        cx.notify();
                    });
                },
            )
        };

        let on_drag_end = {
            let view = view.clone();
            Rc::new(
                move |event: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                    let _ = view.update(cx, |app, cx| {
                        let st = &mut app.dev_reset_state;
                        let was_dragging = st.dragging;
                        st.dragging = false;
                        let is_click = match st.pointer_start {
                            Some(start) => {
                                let dx = event.position.x.as_f32() - start.x.as_f32();
                                let dy = event.position.y.as_f32() - start.y.as_f32();
                                super::dev_reset::is_click_not_drag(
                                    dx,
                                    dy,
                                    super::dev_reset::CLICK_THRESHOLD,
                                )
                            }
                            None => false,
                        };
                        st.pointer_start = None;
                        st.origin_at_drag_start = None;
                        cx.notify();
                        // Only activate on a click that started a drag (mouse down on FAB).
                        if was_dragging && is_click {
                            app.reset_dev_data(window, cx);
                        }
                    });
                },
            )
        };

        Self {
            on_activate,
            on_drag_start,
            on_drag_move,
            on_drag_end,
        }
    }
}

impl Render for OpenCoreApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_resize_intent(window, cx);

        let now = Instant::now();
        self.settle_theme_transition(now);
        let theme = self.visual_theme(now);
        if should_request_frame(&self.onboarding_ui, self.theme_transition.as_ref(), now) {
            window.request_animation_frame();
        }

        let content = match self.state.active_screen {
            ActiveScreen::Onboarding => {
                let _ = self
                    .onboarding_ui
                    .get_or_insert_with(OnboardingUiState::new);
                if let Some(ui) = self.onboarding_ui.as_mut() {
                    ui.tick(now);
                }
                let ui = self.onboarding_ui.as_ref().expect("inserted");
                let callbacks = OnboardingCallbacks::from_app(cx.entity().downgrade());
                let persistence_error = self.persistence_error.as_deref();
                let on_enter = callbacks.on_enter.clone();

                div().size_full().min_w_0().min_h_0().child(onboarding_interactive_root(
                    &self.focus_handle,
                    on_enter,
                    onboarding_screen(
                        theme,
                        ui,
                        callbacks,
                        persistence_error,
                        WindowViewport::from_window(window),
                    ),
                ))
            }
            ActiveScreen::Home => {
                let shell = self.ensure_shell(window, cx);
                shell.update(cx, |shell, _| shell.set_theme(theme));
                div().size_full().min_w_0().min_h_0().child(shell)
            }
        };

        #[cfg(debug_assertions)]
        {
            // Update FAB bounds for edge damping to the current window size.
            let window_bounds = window.viewport_size();
            let bounds = (
                window_bounds.width.as_f32(),
                window_bounds.height.as_f32(),
            );
            let callbacks = DevResetCallbacks::from_app(cx.entity().downgrade(), bounds);
            // Snapshot the state so the element borrows don't clash with `&mut self`.
            let state_snapshot = self.dev_reset_state.clone();
            let on_drag_move = callbacks.on_drag_move.clone();
            let on_drag_end = callbacks.on_drag_end.clone();

            div()
                .size_full()
                .relative()
                .child(content)
                .child(dev_reset_fab(theme, &state_snapshot, &callbacks))
                .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
                    (on_drag_move)(event, window, cx);
                })
                .on_mouse_up(
                    MouseButton::Left,
                    move |event: &MouseUpEvent, window, cx| {
                        (on_drag_end)(event, window, cx);
                    },
                )
        }

        #[cfg(not(debug_assertions))]
        {
            content
        }
    }
}

fn should_request_onboarding_animation(onboarding_ui: &Option<OnboardingUiState>) -> bool {
    onboarding_ui.is_some()
}

fn should_request_frame(
    onboarding_ui: &Option<OnboardingUiState>,
    theme_transition: Option<&ThemeTransition>,
    now: Instant,
) -> bool {
    should_request_onboarding_animation(onboarding_ui)
        || theme_transition.is_some_and(|tx| tx.is_active(now))
}

fn window_bounds_for_state(state: &AppState, cx: &App) -> WindowBounds {
    let (width, height) = state.initial_window_size();
    WindowBounds::centered(size(px(width as f32), px(height as f32)), cx)
}

/// Boots preferences and runs the desktop event loop until the window closes.
pub fn run_desktop() -> Result<(), AppError> {
    let store = Arc::new(FilePreferencesStore::open()?);
    let preferences = store.load()?;
    let state = AppState::from_preferences(preferences);
    let initial_theme_mode = state.theme_mode();

    gpui_platform::application()
        .with_assets(crate::shared::assets::AppAssets)
        .run(move |cx| {
            gpui_component::init(cx);
            register_shell_panels(cx);
            let _ = crate::shared::assets::AppAssets.load_fonts(cx);
            apply_nothing_theme(initial_theme_mode, cx);

            let store = store.clone();
            cx.spawn(async move |cx| {
                let bounds = cx.update(|app| window_bounds_for_state(&state, app));
                let options = WindowOptions {
                    window_bounds: Some(bounds),
                    #[cfg(not(target_os = "linux"))]
                    titlebar: Some(gpui_component::TitleBar::title_bar_options()),
                    #[cfg(target_os = "linux")]
                    titlebar: Some(TitlebarOptions {
                        title: None,
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(12.0), px(11.0))),
                    }),
                    ..Default::default()
                };

                let starts_onboarding = state.active_screen == ActiveScreen::Onboarding;
                cx.open_window(options, |window, cx| {
                    let view = cx.new(|cx| OpenCoreApp::new(state, store, cx));
                    if starts_onboarding {
                        view.update(cx, |app, cx| {
                            app.ensure_onboarding_focus(window, cx);
                        });
                    }
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("failed to open window");
            })
            .detach();
        });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::app_state::{
        HOME_WINDOW_HEIGHT, HOME_WINDOW_WIDTH, ONBOARDING_WINDOW_HEIGHT, ONBOARDING_WINDOW_WIDTH,
    };
    use crate::shared::preferences::{AppPreferences, InMemoryPreferencesStore};
    use crate::shared::theme::ThemeMode;

    #[test]
    fn initial_window_size_is_onboarding_dimensions_when_incomplete() {
        let state = AppState::from_preferences(AppPreferences::default());
        assert_eq!(
            state.initial_window_size(),
            (ONBOARDING_WINDOW_WIDTH, ONBOARDING_WINDOW_HEIGHT)
        );
    }

    #[test]
    fn initial_window_size_is_home_dimensions_when_complete() {
        let state = AppState::from_preferences(AppPreferences {
            theme_mode: ThemeMode::Dark,
            onboarding_completed: true,
            ..Default::default()
        });
        assert_eq!(
            state.initial_window_size(),
            (HOME_WINDOW_WIDTH, HOME_WINDOW_HEIGHT)
        );
    }

    #[test]
    fn take_pending_window_resize_clears_intent() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences::default());
        state.complete_onboarding(&store).expect("complete");
        assert!(state.pending_window_resize.is_some());
        let intent = state.take_pending_window_resize().expect("intent");
        assert_eq!(intent.width, HOME_WINDOW_WIDTH);
        assert!(state.pending_window_resize.is_none());
    }
}

#[cfg(test)]
mod animation_gate_tests {
    use super::*;

    #[test]
    fn onboarding_animation_gate_follows_ui_presence() {
        assert!(should_request_onboarding_animation(&Some(
            OnboardingUiState::new()
        )));
        assert!(!should_request_onboarding_animation(&None));
    }

    #[test]
    fn frame_gate_follows_theme_transition() {
        let now = Instant::now();
        let tx = ThemeTransition::start(
            crate::shared::theme::ThemeMode::Dark,
            crate::shared::theme::ThemeMode::Light,
            now,
        );
        assert!(should_request_frame(&None, Some(&tx), now));
        assert!(!should_request_frame(
            &None,
            Some(&tx),
            now + crate::shared::theme::THEME_TRANSITION_DURATION
        ));
        assert!(!should_request_frame(&None, None, now));
    }
}

#[cfg(test)]
mod dock_layout_persistence_tests {
    use super::*;
    use crate::shared::preferences::AppPreferences;
    use crate::shared::theme::ThemeMode;
    use gpui::{AppContext, TestAppContext};
    use gpui_component::dock::DockAreaState;
    use std::time::Duration;
    use tempfile::TempDir;

    fn test_app(
        cx: &mut TestAppContext,
        store: Arc<FilePreferencesStore>,
        preferences: AppPreferences,
    ) -> gpui::Entity<OpenCoreApp> {
        cx.new(|cx| OpenCoreApp::new(AppState::from_preferences(preferences), store, cx))
    }

    fn marker_layout(version_marker: usize) -> DockAreaState {
        DockAreaState {
            version: Some(version_marker),
            ..Default::default()
        }
    }

    #[gpui::test]
    fn dock_layout_snapshot_merges_immediately_and_preserves_unrelated_preferences(
        cx: &mut TestAppContext,
    ) {
        let dir = TempDir::new().expect("temp dir");
        let store = Arc::new(FilePreferencesStore::at(
            dir.path().join("preferences.json"),
        ));
        let preferences = AppPreferences {
            theme_mode: ThemeMode::Light,
            onboarding_completed: true,
            ..Default::default()
        };
        let app = test_app(cx, store.clone(), preferences);
        let layout = marker_layout(333);

        app.update(cx, |app, cx| {
            app.schedule_dock_layout_save(layout.clone(), cx)
        });
        cx.run_until_parked();

        cx.read_entity(&app, |app, _| {
            assert_eq!(app.state.preferences.theme_mode, ThemeMode::Light);
            assert!(app.state.preferences.onboarding_completed);
            assert_eq!(app.state.preferences.dock_layout, Some(layout.clone()));
        });

        cx.executor().advance_clock(Duration::from_millis(400));
        cx.run_until_parked();
        let saved = store.load().expect("load saved preferences");
        assert_eq!(saved.theme_mode, ThemeMode::Light);
        assert!(saved.onboarding_completed);
        assert_eq!(saved.dock_layout, Some(layout));
    }

    #[gpui::test]
    fn dock_layout_debounce_writes_only_the_latest_snapshot(cx: &mut TestAppContext) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("preferences.json");
        let store = Arc::new(FilePreferencesStore::at(&path));
        let app = test_app(cx, store.clone(), AppPreferences::default());
        let first = marker_layout(300);
        let latest = marker_layout(360);

        app.update(cx, |app, cx| app.schedule_dock_layout_save(first, cx));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(200));
        app.update(cx, |app, cx| {
            app.schedule_dock_layout_save(latest.clone(), cx)
        });
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();
        assert!(!path.exists());

        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();
        let saved = store.load().expect("load saved preferences");
        assert_eq!(saved.dock_layout, Some(latest));
    }

    #[gpui::test]
    fn dock_layout_debounce_records_save_errors(cx: &mut TestAppContext) {
        let dir = TempDir::new().expect("temp dir");
        let store = Arc::new(FilePreferencesStore::at(dir.path()));
        let app = test_app(cx, store, AppPreferences::default());

        app.update(cx, |app, cx| {
            app.schedule_dock_layout_save(marker_layout(1), cx)
        });
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(400));
        cx.run_until_parked();

        cx.read_entity(&app, |app, _| {
            assert!(
                app.persistence_error
                    .as_deref()
                    .is_some_and(|error| error.contains("Could not save settings"))
            );
        });
    }

    #[gpui::test]
    fn dock_layout_shutdown_flushes_dirty_latest_snapshot_before_debounce(cx: &mut TestAppContext) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("preferences.json");
        let store = Arc::new(FilePreferencesStore::at(&path));
        let app = test_app(cx, store.clone(), AppPreferences::default());
        let latest = marker_layout(372);

        app.update(cx, |app, cx| {
            app.schedule_dock_layout_save(latest.clone(), cx)
        });
        cx.run_until_parked();

        cx.update(|gpui| gpui.shutdown());

        let saved = store.load().expect("load shutdown-flushed preferences");
        assert_eq!(saved.dock_layout, Some(latest));
    }
}

#[cfg(all(test, debug_assertions))]
mod reset_tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};
    use std::time::Duration;
    use tempfile::TempDir;

    #[gpui::test]
    fn successful_reset_clears_existing_shell_entity(cx: &mut TestAppContext) {
        cx.update(|app| {
            gpui_component::init(app);
            register_shell_panels(app);
        });
        let dir = TempDir::new().expect("temp dir");
        let store = Arc::new(FilePreferencesStore::at(
            dir.path().join("preferences.json"),
        ));
        let app = cx.new(|cx| {
            OpenCoreApp::new(
                AppState::from_preferences(crate::shared::preferences::AppPreferences {
                    onboarding_completed: true,
                    ..Default::default()
                }),
                store,
                cx,
            )
        });
        let save: DockSaveFn = Rc::new(|_, _| {});
        let (shell, _) =
            cx.add_window_view(|window, cx| ShellWorkspace::new(None, save, window, cx));

        app.update(cx, |app, _| app.shell = Some(shell));
        app.update(cx, |app, _| {
            app.reset_dev_data_state().expect("reset persistent data");
        });

        assert!(cx.read_entity(&app, |app, _| app.shell.is_none()));
    }

    #[gpui::test]
    fn successful_reset_cancels_pending_shell_save(cx: &mut TestAppContext) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("preferences.json");
        let store = Arc::new(FilePreferencesStore::at(&path));
        let app = cx.new(|cx| {
            OpenCoreApp::new(
                AppState::from_preferences(crate::shared::preferences::AppPreferences {
                    onboarding_completed: true,
                    ..Default::default()
                }),
                store.clone(),
                cx,
            )
        });
        let stale_layout = DockAreaState {
            version: Some(399),
            ..Default::default()
        };

        app.update(cx, |app, cx| {
            app.schedule_dock_layout_save(stale_layout, cx)
        });
        cx.run_until_parked();
        app.update(cx, |app, _| {
            app.reset_dev_data_state().expect("reset persistent data");
        });
        assert!(cx.read_entity(&app, |app, _| app.shell_save_task.is_none()));
        cx.executor().advance_clock(Duration::from_millis(400));
        cx.run_until_parked();

        let saved = store.load().expect("load reset preferences");
        assert_eq!(saved, crate::shared::preferences::AppPreferences::default());
    }
}
