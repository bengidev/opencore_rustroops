//! **Facade** for the GPU runtime: boots preferences, opens one window, and routes
//! [`super::ActiveScreen`] without closing between onboarding and shell.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    App, AppContext, Context, FocusHandle, IntoElement, ParentElement, Render, Styled, WeakEntity,
    Window, WindowBounds, WindowOptions, div, px, size,
};
use gpui_component::Root;
use gpui_component::Theme;

use crate::shared::preferences::{FilePreferencesStore, PreferencesError, PreferencesStore};
use crate::shared::theme::{OpenCoreTheme, ThemeMode};

use super::AppError;
use super::app_state::{ActiveScreen, AppState};
use super::onboarding::{
    OnboardingCallbacks, OnboardingCommand, OnboardingOutcome, OnboardingUiState,
    onboarding_interactive_root, onboarding_screen, reduce_onboarding,
};
use super::shell::{ShellCallbacks, shell_screen};
use super::window_placement::center_window;

/// Composition-root view: dispatches on [`ActiveScreen`] and owns persisted state.
pub struct OpenCoreApp {
    state: AppState,
    store: Arc<FilePreferencesStore>,
    focus_handle: FocusHandle,
    onboarding_ui: Option<OnboardingUiState>,
    animation_scheduled: bool,
    persistence_error: Option<String>,
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
            animation_scheduled: false,
            persistence_error: None,
        }
    }

    fn theme(&self) -> OpenCoreTheme {
        OpenCoreTheme::resolve(self.state.theme_mode())
    }

    fn sync_component_theme(&self, cx: &mut App) {
        sync_gpui_component_theme(self.state.theme_mode(), cx);
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
        self.persistence_error = Some(format!("Could not save settings ({error})"));
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
        let next = self.state.theme_mode().toggle();
        match self.state.set_theme_mode(self.store.as_ref(), next) {
            Ok(()) => {
                self.persistence_error = None;
                self.sync_component_theme(cx);
                cx.notify();
            }
            Err(error) => {
                self.record_persistence_error("save theme", error);
                cx.notify();
            }
        }
    }

    fn reset_dev_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.state.reset_persistent_data(self.store.as_ref()) {
            Ok(()) => {
                self.onboarding_ui = Some(OnboardingUiState::new());
                self.animation_scheduled = false;
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

    fn schedule_animation(&mut self, cx: &mut Context<Self>) {
        if self.animation_scheduled || self.onboarding_ui.is_none() {
            return;
        }
        self.animation_scheduled = true;
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
                let still_onboarding = entity
                    .update(cx, |app, cx| {
                        if let Some(ui) = app.onboarding_ui.as_mut() {
                            ui.tick(Instant::now());
                            cx.notify();
                            true
                        } else {
                            app.animation_scheduled = false;
                            false
                        }
                    })
                    .unwrap_or(false);
                if !still_onboarding {
                    break;
                }
            }
        })
        .detach();
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
        let on_orb_pressed = {
            let view = view.clone();
            Rc::new(move |cx: &mut App| {
                let _ = view.update(cx, |app, cx| {
                    if let Some(ui) = app.onboarding_ui.as_mut() {
                        ui.orb_pressed();
                        cx.notify();
                    }
                });
            })
        };
        let on_orb_released = {
            let view = view.clone();
            Rc::new(move |cx: &mut App| {
                let _ = view.update(cx, |app, cx| {
                    if let Some(ui) = app.onboarding_ui.as_mut() {
                        ui.orb_released();
                        cx.notify();
                    }
                });
            })
        };

        Self {
            on_enter,
            on_toggle_theme,
            on_orb_pressed,
            on_orb_released,
        }
    }
}

impl Render for OpenCoreApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_resize_intent(window, cx);

        match self.state.active_screen {
            ActiveScreen::Onboarding => {
                self.schedule_animation(cx);
                let theme = self.theme();
                let ui = self
                    .onboarding_ui
                    .get_or_insert_with(OnboardingUiState::new);
                let callbacks = OnboardingCallbacks::from_app(cx.entity().downgrade());
                let persistence_error = self.persistence_error.as_deref();
                let on_enter = callbacks.on_enter.clone();

                div().size_full().child(onboarding_interactive_root(
                    &self.focus_handle,
                    on_enter,
                    onboarding_screen(theme, ui, callbacks, persistence_error),
                ))
            }
            ActiveScreen::Shell => {
                let view = cx.entity().downgrade();
                let mut callbacks = ShellCallbacks::new();
                #[cfg(debug_assertions)]
                {
                    callbacks.on_reset_dev = Some(Rc::new({
                        let view = view.clone();
                        move |window: &mut Window, cx: &mut App| {
                            let _ = view.update(cx, |app, cx| {
                                app.reset_dev_data(window, cx);
                            });
                        }
                    }));
                }
                div()
                    .size_full()
                    .child(shell_screen(self.theme(), callbacks))
            }
        }
    }
}

fn window_bounds_for_state(state: &AppState, cx: &App) -> WindowBounds {
    let (width, height) = state.initial_window_size();
    WindowBounds::centered(size(px(width as f32), px(height as f32)), cx)
}

fn sync_gpui_component_theme(mode: ThemeMode, cx: &mut App) {
    use gpui_component::theme::ThemeMode as ComponentThemeMode;

    let component_mode = match mode {
        ThemeMode::Light => ComponentThemeMode::Light,
        ThemeMode::Dark => ComponentThemeMode::Dark,
    };
    Theme::change(component_mode, None, cx);
}

/// Boots preferences and runs the desktop event loop until the window closes.
pub fn run_desktop() -> Result<(), AppError> {
    let store = Arc::new(FilePreferencesStore::open()?);
    let preferences = store.load()?;
    let state = AppState::from_preferences(preferences);
    let initial_theme_mode = state.theme_mode();

    gpui_platform::application()
        .with_assets(gpui_component_assets::Assets)
        .run(move |cx| {
            gpui_component::init(cx);
            sync_gpui_component_theme(initial_theme_mode, cx);

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
        ONBOARDING_WINDOW_HEIGHT, ONBOARDING_WINDOW_WIDTH, SHELL_WINDOW_HEIGHT, SHELL_WINDOW_WIDTH,
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
    fn initial_window_size_is_shell_dimensions_when_complete() {
        let state = AppState::from_preferences(AppPreferences {
            theme_mode: ThemeMode::Dark,
            onboarding_completed: true,
        });
        assert_eq!(
            state.initial_window_size(),
            (SHELL_WINDOW_WIDTH, SHELL_WINDOW_HEIGHT)
        );
    }

    #[test]
    fn take_pending_window_resize_clears_intent() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences::default());
        state.complete_onboarding(&store).expect("complete");
        assert!(state.pending_window_resize.is_some());
        let intent = state.take_pending_window_resize().expect("intent");
        assert_eq!(intent.width, SHELL_WINDOW_WIDTH);
        assert!(state.pending_window_resize.is_none());
    }
}
