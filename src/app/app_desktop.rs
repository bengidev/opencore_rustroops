//! **Facade** for the GPU runtime: boots preferences, opens one window, and routes
//! [`super::ActiveScreen`] without closing between onboarding and home.

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    App, AppContext, Context, FocusHandle, IntoElement, ParentElement, Render, Styled, Task,
    TitlebarOptions, WeakEntity, Window, WindowBounds, WindowOptions, div, point, px, size,
};
#[cfg(debug_assertions)]
use gpui::{InteractiveElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Point};
use gpui_component::Root;

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
use super::shell::{Shell, ShellSaveFn, TITLEBAR_HEIGHT};
use super::window_placement::center_window;

const SHELL_SAVE_DEBOUNCE: Duration = Duration::from_millis(400);

/// Composition-root view: dispatches on [`ActiveScreen`] and owns persisted state.
pub struct OpenCoreApp {
    state: AppState,
    store: Arc<FilePreferencesStore>,
    focus_handle: FocusHandle,
    onboarding_ui: Option<OnboardingUiState>,
    shell: Option<gpui::Entity<Shell>>,
    shell_save_task: Option<Task<()>>,
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
        Self {
            state,
            store,
            focus_handle: cx.focus_handle(),
            onboarding_ui,
            shell: None,
            shell_save_task: None,
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

    fn schedule_shell_save(&mut self, chrome: super::shell::ShellChrome, cx: &mut Context<Self>) {
        self.state.preferences.shell = chrome;
        self.shell_save_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(SHELL_SAVE_DEBOUNCE).await;
            let _ = view.update(cx, |app, _| app.flush_shell_save());
        }));
    }

    fn flush_shell_save(&mut self) {
        let preferences = self.state.preferences.clone();
        match self.store.save(&preferences) {
            Ok(()) => {
                self.persistence_error = None;
            }
            Err(error) => self.record_persistence_error("save shell", error),
        }
    }

    fn ensure_shell(&mut self, window: &Window, cx: &mut Context<Self>) -> gpui::Entity<Shell> {
        if let Some(shell) = self.shell.as_ref() {
            return shell.clone();
        }

        let bounds = window.bounds();
        let chrome = self
            .state
            .preferences
            .shell
            .clone()
            .sanitized(bounds.size.width.as_f32(), bounds.size.height.as_f32());
        let view = cx.entity().downgrade();
        let save: ShellSaveFn = Rc::new(move |chrome, app| {
            let _ = view.update(app, |app, cx| {
                app.schedule_shell_save(chrome, cx);
            });
        });
        let shell = cx.new(|cx| Shell::new(chrome, save, cx));
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

                div()
                    .size_full()
                    .pt(px(TITLEBAR_HEIGHT))
                    .child(onboarding_interactive_root(
                        &self.focus_handle,
                        on_enter,
                        onboarding_screen(
                            theme,
                            ui,
                            callbacks,
                            persistence_error,
                            window.bounds().size,
                        ),
                    ))
            }
            ActiveScreen::Home => {
                let shell = self.ensure_shell(window, cx);
                let _ = shell.update(cx, |shell, _| shell.set_theme(theme));
                div().size_full().child(shell)
            }
        };

        #[cfg(debug_assertions)]
        {
            // Update FAB bounds for edge damping to the current window size.
            let window_bounds = window.bounds();
            let bounds = (
                window_bounds.size.width.as_f32(),
                window_bounds.size.height.as_f32(),
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
            let _ = crate::shared::assets::AppAssets.load_fonts(cx);
            apply_nothing_theme(initial_theme_mode, cx);

            let store = store.clone();
            cx.spawn(async move |cx| {
                let bounds = cx.update(|app| window_bounds_for_state(&state, app));
                let options = WindowOptions {
                    window_bounds: Some(bounds),
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
mod shell_persistence_tests {
    use super::*;
    use crate::shared::preferences::{AppPreferences, ShellChrome};
    use crate::shared::theme::ThemeMode;
    use gpui::{AppContext, TestAppContext};
    use std::time::Duration;
    use tempfile::TempDir;

    fn test_app(
        cx: &mut TestAppContext,
        store: Arc<FilePreferencesStore>,
        preferences: AppPreferences,
    ) -> gpui::Entity<OpenCoreApp> {
        cx.new(|cx| OpenCoreApp::new(AppState::from_preferences(preferences), store, cx))
    }

    #[gpui::test]
    fn shell_snapshot_merges_immediately_and_preserves_unrelated_preferences(
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
        let mut chrome = ShellChrome::default();
        chrome.left_width = 333.0;

        app.update(cx, |app, cx| app.schedule_shell_save(chrome.clone(), cx));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(400));
        cx.run_until_parked();

        cx.read_entity(&app, |app, _| {
            assert_eq!(app.state.preferences.shell, chrome);
            assert_eq!(app.state.preferences.theme_mode, ThemeMode::Light);
            assert!(app.state.preferences.onboarding_completed);
        });
        let saved = store.load().expect("load saved preferences");
        assert_eq!(saved.shell, chrome);
        assert_eq!(saved.theme_mode, ThemeMode::Light);
        assert!(saved.onboarding_completed);
    }

    #[gpui::test]
    fn shell_debounce_writes_only_the_latest_snapshot(cx: &mut TestAppContext) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("preferences.json");
        let store = Arc::new(FilePreferencesStore::at(&path));
        let app = test_app(cx, store.clone(), AppPreferences::default());
        let mut first = ShellChrome::default();
        first.left_width = 300.0;
        let mut latest = first.clone();
        latest.left_width = 360.0;

        app.update(cx, |app, cx| app.schedule_shell_save(first, cx));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(200));
        app.update(cx, |app, cx| app.schedule_shell_save(latest.clone(), cx));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();
        assert!(!path.exists());

        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();
        let saved = store.load().expect("load saved preferences");
        assert_eq!(saved.shell, latest);
    }

    #[gpui::test]
    fn shell_debounce_records_save_errors(cx: &mut TestAppContext) {
        let dir = TempDir::new().expect("temp dir");
        let store = Arc::new(FilePreferencesStore::at(dir.path()));
        let app = test_app(cx, store, AppPreferences::default());

        app.update(cx, |app, cx| {
            app.schedule_shell_save(ShellChrome::default(), cx)
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
}

#[cfg(all(test, debug_assertions))]
mod reset_tests {
    use super::*;
    use crate::shared::preferences::ShellChrome;
    use gpui::{AppContext, TestAppContext};
    use std::time::Duration;
    use tempfile::TempDir;

    #[gpui::test]
    fn successful_reset_clears_existing_shell_entity(cx: &mut TestAppContext) {
        let store = Arc::new(FilePreferencesStore::at("/tmp/opencore-reset-test.json"));
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
        let save: ShellSaveFn = Rc::new(|_, _| {});
        let shell = cx.new(|cx| Shell::new(super::super::shell::ShellChrome::default(), save, cx));

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
        let mut stale_chrome = ShellChrome::default();
        stale_chrome.left_width = 399.0;

        app.update(cx, |app, cx| app.schedule_shell_save(stale_chrome, cx));
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
