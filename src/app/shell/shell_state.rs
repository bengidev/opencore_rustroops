//! Shell command reducer — ephemeral UI state separate from persisted preferences.

use super::workspace_mode::WorkspaceMode;

/// In-memory shell UI state (not written to [`crate::shared::preferences::AppPreferences`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    pub active_mode: WorkspaceMode,
    pub sidebar_collapsed: bool,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            active_mode: WorkspaceMode::Chat,
            sidebar_collapsed: false,
        }
    }
}

/// Shell-local commands handled by the reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    SelectMode(WorkspaceMode),
    ToggleSidebar,
}

/// Pure reducer for shell transitions (**Command** pattern).
pub fn reduce_shell(state: &ShellState, command: ShellCommand) -> ShellState {
    let mut next = state.clone();
    match command {
        ShellCommand::SelectMode(mode) => next.active_mode = mode,
        ShellCommand::ToggleSidebar => next.sidebar_collapsed = !next.sidebar_collapsed,
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::preferences::AppPreferences;

    #[test]
    fn default_shell_state_starts_in_chat_mode() {
        let state = ShellState::default();
        assert_eq!(state.active_mode, WorkspaceMode::Chat);
    }

    #[test]
    fn select_mode_switches_center_panel() {
        let state = ShellState::default();
        let next = reduce_shell(&state, ShellCommand::SelectMode(WorkspaceMode::Editor));
        assert_eq!(next.active_mode, WorkspaceMode::Editor);
    }

    #[test]
    fn toggle_sidebar_flips_collapsed_flag() {
        let state = ShellState::default();
        let collapsed = reduce_shell(&state, ShellCommand::ToggleSidebar);
        assert!(collapsed.sidebar_collapsed);
        let expanded = reduce_shell(&collapsed, ShellCommand::ToggleSidebar);
        assert!(!expanded.sidebar_collapsed);
    }

    #[test]
    fn preferences_document_has_no_workspace_mode_field() {
        let json = serde_json::to_string(&AppPreferences::default()).expect("serialize");
        assert!(!json.contains("workspace_mode"));
        assert!(!json.contains("active_mode"));
        assert!(!json.contains("sidebar_collapsed"));
    }
}
