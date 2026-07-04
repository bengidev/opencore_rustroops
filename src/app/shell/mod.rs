//! Shell mount point — fullscreen chat plus shell-ready action seam.

mod workspace_mode;

pub use workspace_mode::WorkspaceMode;

mod shell_state;
pub use shell_state::ShellState;

use gpui::{Entity, IntoElement, ParentElement, Styled, Window, div};

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

/// Shell-owned handle for reading chat context and triggering chat actions.
#[derive(Clone)]
pub struct ShellChatHandle {
    chat_view: Entity<ChatView>,
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

/// Mounts the fullscreen chat surface in the shell.
pub fn shell_screen(chat_view: Entity<ChatView>) -> impl IntoElement {
    let handle = ShellChatHandle::new(chat_view);
    div().size_full().child(handle.chat_view)
}
