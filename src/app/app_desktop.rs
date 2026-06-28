//! **Facade** for the GPU runtime: boots preferences, opens one window, and routes
//! [`super::ActiveScreen`] without closing between onboarding and shell.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    div, px, size, App, AppContext, Context, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions,
};
use gpui_component::Root;
use gpui_component::Theme;

use crate::shared::preferences::{FilePreferencesStore, PreferencesStore};
use crate::shared::theme::{OpenCoreTheme, ThemeMode};

use super::onboarding::{
    onboarding_screen, reduce_onboarding, OnboardingCallbacks, OnboardingCommand,
    OnboardingUiState,
};
use super::shell::{shell_screen, ShellCallbacks};
use super::app_state::{ActiveScreen, AppState};
use super::window_placement::center_window;
use super::AppError;

/// Composition-root view: dispatches on [`ActiveScreen`] and owns persisted state.
pub struct OpenCoreApp {
    state: AppState,
    store: Arc<FilePreferencesStore>,
    onboarding_ui: Option<OnboardingUiState>,
    animation_scheduled: bool,
}

impl OpenCoreApp {
    fn new(state: AppState, store: Arc<FilePreferencesStore>) -> Self {
        let onboarding_ui = if state.active_screen == ActiveScreen::Onboarding {
            Some(OnboardingUiState::new())
        } else {
            None
        };
        Self {
            state,
            store,
            onboarding_ui,
            animation_scheduled: false,
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

    fn complete_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .state
            .complete_onboarding(self.store.as_ref())
            .is_ok()
        {
            self.onboarding_ui = None;
            self.apply_resize_intent(window, cx);
            cx.notify();
        }
    }

    fn reset_dev_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .state
            .reset_persistent_data(self.store.as_ref())
            .is_ok()
        {
            self.onboarding_ui = Some(OnboardingUiState::new());
            self.animation_scheduled = false;
            self.apply_resize_intent(window, cx);
            cx.notify();
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

impl Render for OpenCoreApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_resize_intent(window, cx);

        match self.state.active_screen {
            ActiveScreen::Onboarding => {
                self.schedule_animation(cx);
                let theme = self.theme();
                let ui = self
                    .onboarding_ui
                    .get_or_insert_with(OnboardingUiState::new)
                    .clone();
                let view = cx.entity().downgrade();

                let on_enter = Rc::new({
                    let view = view.clone();
                    move |window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |app, cx| {
                            app.complete_onboarding(window, cx);
                        });
                    }
                });
                let on_skip = {
                    let view = view.clone();
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |app, cx| {
                            let outcome = reduce_onboarding(OnboardingCommand::Skipped);
                            if app
                                .state
                                .apply_onboarding_outcome(outcome, app.store.as_ref())
                                .is_ok()
                            {
                                app.onboarding_ui = None;
                                app.apply_resize_intent(window, cx);
                                cx.notify();
                            }
                        });
                    })
                };
                let on_toggle_theme = {
                    let view = view.clone();
                    Rc::new(move |_: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |app, cx| {
                            let next = app.state.theme_mode().toggle();
                            if app
                                .state
                                .set_theme_mode(app.store.as_ref(), next)
                                .is_ok()
                            {
                                app.sync_component_theme(cx);
                                cx.notify();
                            }
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

                div().size_full().child(onboarding_screen(
                    theme,
                    &ui,
                    OnboardingCallbacks {
                        on_enter,
                        on_skip,
                        on_toggle_theme,
                        on_orb_pressed,
                        on_orb_released,
                    },
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
                div().size_full().child(shell_screen(self.theme(), callbacks))
            }
        }
    }
}

fn window_bounds_for_state(state: &AppState, cx: &App) -> WindowBounds {
    let (width, height) = state.initial_window_size();
    WindowBounds::centered(
        size(px(width as f32), px(height as f32)),
        cx,
    )
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

                cx.open_window(options, |window, cx| {
                    let view = cx.new(|_| OpenCoreApp::new(state, store));
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
