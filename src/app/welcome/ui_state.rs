//! Interactive welcome UI state (keyboard focus).

use gpui::{App, FocusHandle, Window};

pub struct WelcomeUiState {
    focus_claimed: bool,
}

impl WelcomeUiState {
    pub fn new() -> Self {
        Self {
            focus_claimed: false,
        }
    }

    /// Requests keyboard focus once per welcome session.
    pub fn ensure_initial_focus(
        &mut self,
        window: &mut Window,
        handle: &FocusHandle,
        cx: &mut App,
    ) {
        if self.focus_claimed {
            return;
        }
        if handle.is_focused(window) {
            self.focus_claimed = true;
        } else {
            window.focus(handle, cx);
        }
    }
}
