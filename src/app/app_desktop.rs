//! **Facade** for the GPU runtime: boots preferences, opens one window, and routes
//! [`super::ActiveScreen`] without closing between onboarding and home.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    App, AppContext, Context, FocusHandle, IntoElement, ParentElement, Render, Styled, WeakEntity,
    Window, WindowBounds, WindowOptions, div, px, size,
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
use super::home::home_screen;
use super::onboarding::{
    OnboardingCallbacks, OnboardingCommand, OnboardingOutcome, OnboardingUiState,
    onboarding_interactive_root, onboarding_screen, reduce_onboarding,
};
use super::window_placement::center_window;

/// Composition-root view: dispatches on [`ActiveScreen`] and owns persisted state.
pub struct OpenCoreApp {
    state: AppState,
    store: Arc<FilePreferencesStore>,
    focus_handle: FocusHandle,
    onboarding_ui: Option<OnboardingUiState>,
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
        match self.state.reset_persistent_data(self.store.as_ref()) {
            Ok(()) => {
                self.onboarding_ui = Some(OnboardingUiState::new());
                self.persistence_error = None;
                self.ensure_onboarding_focus(window, cx);
                self.finish_screen_transition(window, cx);
            }
            Err(error) => {
                self.record_persistence_error("reset dev data", error);
                cx.notify();
            }
        }
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

                div().size_full().child(onboarding_interactive_root(
                    &self.focus_handle,
                    on_enter,
                    onboarding_screen(theme, ui, callbacks, persistence_error),
                ))
            }
            ActiveScreen::Home => div().size_full().child(home_screen(theme)),
        };

        #[cfg(debug_assertions)]
        {
            // Update FAB bounds for edge damping to the current window size.
            let (win_w, win_h) = self.state.initial_window_size();
            let bounds = (win_w as f32, win_h as f32);
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
