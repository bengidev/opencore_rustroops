//! Shell mount point — fullscreen chat with optional debug chrome.

use gpui::{Entity, IntoElement, ParentElement, Styled, div};

use crate::chat::ChatView;
use crate::shared::theme::OpenCoreTheme;

/// Mounts the fullscreen chat surface in the shell.
pub fn shell_screen(theme: OpenCoreTheme, chat_view: Entity<ChatView>) -> impl IntoElement {
    let _ = theme;
    div().size_full().child(chat_view)
}
