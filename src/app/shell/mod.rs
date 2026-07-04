//! Workspace shell mount point — top bar, left sidebar slot, and mode routing.

mod shell_helpers;
mod shell_placeholders;
mod shell_state;
mod shell_view;
mod workspace_layout;

pub use shell_view::ShellView;
pub use workspace_layout::render_workspace_shell;

use gpui::Entity;

use crate::chat::ChatView;

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

/// Shell-owned handle for reading chat context and triggering chat actions.
#[derive(Clone)]
pub struct ShellChatHandle {
    chat_view: Entity<ChatView>,
}

impl ShellChatHandle {
    pub fn new(chat_view: Entity<ChatView>) -> Self {
        Self { chat_view }
    }

    pub fn chat_view(&self) -> Entity<ChatView> {
        self.chat_view.clone()
    }

    pub fn context(&self, cx: &gpui::App) -> crate::chat::ChatShellContext {
        self.chat_view.read(cx).shell_context()
    }

    pub fn dispatch(&self, action: ShellChatAction, window: &mut gpui::Window, cx: &mut gpui::App) {
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
