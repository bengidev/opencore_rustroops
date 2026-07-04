//! Ephemeral shell UI state (**State** pattern) kept separate from persisted preferences.

/// Active center workspace mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspaceMode {
    Editor,
    #[default]
    Chat,
    Terminal,
}

impl WorkspaceMode {
    pub const ALL: [Self; 3] = [Self::Editor, Self::Chat, Self::Terminal];

    pub fn label(self) -> &'static str {
        match self {
            Self::Editor => "Editor",
            Self::Chat => "Chat",
            Self::Terminal => "Terminal",
        }
    }
}

/// Session-local shell state; not written to [`AppPreferences`].
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

/// Commands routed through the shell reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    SelectMode(WorkspaceMode),
    ToggleSidebar,
}

/// Reduces shell commands into the next ephemeral state.
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
    fn default_shell_state_starts_in_chat_with_sidebar_expanded() {
        let state = ShellState::default();
        assert_eq!(state.active_mode, WorkspaceMode::Chat);
        assert!(!state.sidebar_collapsed);
    }

    #[test]
    fn select_mode_updates_ephemeral_state_only() {
        let state = ShellState::default();
        let next = reduce_shell(&state, ShellCommand::SelectMode(WorkspaceMode::Editor));
        assert_eq!(next.active_mode, WorkspaceMode::Editor);
        assert_eq!(state.active_mode, WorkspaceMode::Chat);
    }

    #[test]
    fn select_mode_keeps_context_mode_independent() {
        let state = ShellState {
            active_mode: WorkspaceMode::Terminal,
            sidebar_collapsed: true,
        };
        let next = reduce_shell(&state, ShellCommand::SelectMode(WorkspaceMode::Chat));
        assert_eq!(next.active_mode, WorkspaceMode::Chat);
        assert!(next.sidebar_collapsed);
    }

    #[test]
    fn toggle_sidebar_flips_collapsed_flag_only() {
        let state = ShellState::default();
        let collapsed = reduce_shell(&state, ShellCommand::ToggleSidebar);
        assert!(collapsed.sidebar_collapsed);
        assert_eq!(collapsed.active_mode, WorkspaceMode::Chat);
        let expanded = reduce_shell(&collapsed, ShellCommand::ToggleSidebar);
        assert!(!expanded.sidebar_collapsed);
    }

    #[test]
    fn workspace_mode_is_not_part_of_app_preferences_json() {
        let json =
            serde_json::to_string(&AppPreferences::default()).expect("serialize preferences");
        assert!(!json.contains("active_mode"));
        assert!(!json.contains("workspace_mode"));
        assert!(!json.contains("sidebar_collapsed"));
    }
}
