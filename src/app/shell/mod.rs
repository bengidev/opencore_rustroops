//! Shell mount point — fullscreen chat.

use gpui::{Entity, IntoElement, ParentElement, Styled, div};

use crate::chat::ChatView;

/// Mounts the fullscreen chat surface in the shell.
pub fn shell_screen(chat_view: Entity<ChatView>) -> impl IntoElement {
    div().size_full().child(chat_view)
}
