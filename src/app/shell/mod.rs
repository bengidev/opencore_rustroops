//! Workspace shell — top bar, mode tabs, and center panel routing.

mod context_label;
mod mode_placeholder;
mod placeholder_view;
mod shell_state;
mod shell_view;
mod top_bar;
mod workspace_mode;

pub use shell_state::ShellState;
pub use shell_view::{ShellView, shell_screen};
pub use workspace_mode::WorkspaceMode;

use gpui::{Entity, Window};

use crate::chat::{ChatShellContext, ChatView};

/// Workspace-owned commands that delegate to the chat child for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellChatAction {
    FocusComposer,
    SwitchThread(i64),
    CreateThread,
    DeleteActiveThread,
    OpenCustomInstructions,
    OpenCredentialSettings,
}

/// Shell-owned handle for reading chat context and triggering chat actions (**Facade**).
#[derive(Clone)]
pub struct ShellChatHandle {
    pub(crate) chat_view: Entity<ChatView>,
}

impl ShellChatHandle {
    pub fn new(chat_view: Entity<ChatView>) -> Self {
        Self { chat_view }
    }

    pub fn context(&self, cx: &gpui::App) -> ChatShellContext {
        self.chat_view.read(cx).shell_context()
    }

    pub fn dispatch(&self, action: ShellChatAction, window: &mut Window, cx: &mut gpui::App) {
        self.chat_view.update(cx, |chat, cx| match action {
            ShellChatAction::FocusComposer => chat.focus_composer(window, cx),
            ShellChatAction::SwitchThread(thread_id) => chat.switch_to_thread(thread_id, cx),
            ShellChatAction::CreateThread => chat.create_thread(window, cx),
            ShellChatAction::DeleteActiveThread => chat.delete_active_thread(window, cx),
            ShellChatAction::OpenCustomInstructions => chat.open_instructions_dialog(window, cx),
            ShellChatAction::OpenCredentialSettings => chat.open_credential_settings(window, cx),
        });
    }
}

/// Legacy mount helper retained for composition-root wiring.
#[allow(dead_code)]
pub fn mount_shell(
    theme: crate::shared::theme::OpenCoreTheme,
    chat_view: Entity<ChatView>,
    cx: &mut gpui::App,
) -> Entity<ShellView> {
    shell_screen(theme, chat_view, cx)
}

#[cfg(test)]
mod tests {
    use super::context_label::active_context_label;
    use super::mode_placeholder::{CenterPanelKind, center_panel_for_mode, placeholder_for_mode};
    use super::shell_state::{ShellCommand, ShellState, reduce_shell};
    use super::workspace_mode::WorkspaceMode;
    use crate::chat::ChatShellContext;
    use crate::chat::ThreadInfo;
    use crate::shared::preferences::AppPreferences;

    #[test]
    fn completing_onboarding_routes_to_shell_screen() {
        use crate::app::{ActiveScreen, boot_screen};
        use crate::shared::preferences::AppPreferences;
        use crate::shared::theme::ThemeMode;

        let prefs = AppPreferences {
            theme_mode: ThemeMode::Dark,
            onboarding_completed: true,
        };
        assert_eq!(boot_screen(&prefs), ActiveScreen::Shell);
    }

    #[test]
    fn context_label_survives_mode_switch() {
        let context = ChatShellContext {
            active_thread_id: Some(1),
            active_thread_title: String::new(),
            threads: vec![ThreadInfo {
                id: 1,
                title: Some("Design review".into()),
                created_at: String::new(),
                model_id: String::new(),
            }],
            thread_settings: Default::default(),
            is_streaming: false,
            credentials_missing: false,
        };
        let label = active_context_label(&context);
        assert_eq!(label, "Design review");

        let state = reduce_shell(
            &ShellState::default(),
            ShellCommand::SelectMode(WorkspaceMode::Terminal),
        );
        assert_eq!(state.active_mode, WorkspaceMode::Terminal);
        assert_eq!(active_context_label(&context), label);
    }

    #[test]
    fn shell_mode_is_not_persisted_in_preferences() {
        let json = serde_json::to_string(&AppPreferences::default()).expect("serialize");
        assert!(!json.contains("workspace"));
        assert!(!json.contains("sidebar"));
    }

    #[test]
    fn editor_and_terminal_use_distinct_placeholders() {
        let editor = placeholder_for_mode(WorkspaceMode::Editor);
        let terminal = placeholder_for_mode(WorkspaceMode::Terminal);
        assert_ne!(editor, terminal);
        assert!(matches!(
            center_panel_for_mode(WorkspaceMode::Chat),
            CenterPanelKind::Chat
        ));
    }
}
