//! Workspace shell state and command dispatch (**State** + **Command** patterns).

use gpui::{Context, Entity, Window};

use crate::chat::ChatView;
use crate::shared::theme::OpenCoreTheme;

use super::shell_state::{ShellCommand, ShellState, WorkspaceMode, reduce_shell};
use super::{ShellChatAction, ShellChatHandle};

/// Owns ephemeral shell state and delegates chat actions to the chat entity.
pub struct ShellView {
    state: ShellState,
    chat: ShellChatHandle,
    theme: OpenCoreTheme,
}

impl ShellView {
    pub fn new(chat_view: Entity<ChatView>, theme: OpenCoreTheme) -> Self {
        Self {
            state: ShellState::default(),
            chat: ShellChatHandle::new(chat_view),
            theme,
        }
    }

    pub fn theme(&self) -> OpenCoreTheme {
        self.theme
    }

    pub fn set_theme(&mut self, theme: OpenCoreTheme) {
        self.theme = theme;
    }

    pub fn state(&self) -> &ShellState {
        &self.state
    }

    pub fn chat_handle(&self) -> &ShellChatHandle {
        &self.chat
    }

    pub fn apply_command(&mut self, command: ShellCommand, window: &mut Window, cx: &mut Context<Self>) {
        self.state = reduce_shell(&self.state, command);
        if matches!(command, ShellCommand::SelectMode(WorkspaceMode::Chat)) {
            self.chat
                .dispatch(ShellChatAction::FocusComposer, window, cx);
        }
        cx.notify();
    }

    pub fn dispatch_chat_action(
        &mut self,
        action: ShellChatAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.chat.dispatch(action, window, cx);
    }
}
