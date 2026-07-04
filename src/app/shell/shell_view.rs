//! Workspace shell GPUI entity — top bar, left sidebar placement, center workspace.

use gpui::{
    AppContext, ClickEvent, Context, Entity, IntoElement, ParentElement, Render, SharedString,
    Styled, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::v_flex;

use crate::chat::ChatView;
use crate::shared::theme::{BackgroundToken, OpenCoreTheme};

use super::context_label::active_context_label;
use super::mode_placeholder::{CenterPanelKind, center_panel_for_mode, placeholder_for_mode};
use super::placeholder_view::render_mode_placeholder;
use super::shell_state::{ShellCommand, ShellState, reduce_shell};
use super::top_bar::shell_top_bar_row;
use super::workspace_mode::WorkspaceMode;
use super::{ShellChatAction, ShellChatHandle};

/// Workspace shell owning ephemeral layout state and mounting chat as a child.
pub struct ShellView {
    theme: OpenCoreTheme,
    state: ShellState,
    chat_handle: ShellChatHandle,
}

impl ShellView {
    pub fn new(theme: OpenCoreTheme, chat_handle: ShellChatHandle, cx: &mut Context<Self>) -> Self {
        let chat_entity = chat_handle.chat_view.clone();
        let _subscription = cx.observe(&chat_entity, |_, _, cx| {
            cx.notify();
        });
        Self {
            theme,
            state: ShellState::default(),
            chat_handle,
        }
    }

    pub fn set_theme(&mut self, theme: OpenCoreTheme) {
        self.theme = theme;
    }

    pub fn state(&self) -> &ShellState {
        &self.state
    }

    fn apply_command(&mut self, command: ShellCommand, cx: &mut Context<Self>) {
        self.state = reduce_shell(&self.state, command);
        cx.notify();
    }

    fn on_toggle_sidebar(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.apply_command(ShellCommand::ToggleSidebar, cx);
    }

    fn on_select_editor(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.apply_command(ShellCommand::SelectMode(WorkspaceMode::Editor), cx);
    }

    fn on_select_chat(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.apply_command(ShellCommand::SelectMode(WorkspaceMode::Chat), cx);
    }

    fn on_select_terminal(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.apply_command(ShellCommand::SelectMode(WorkspaceMode::Terminal), cx);
    }

    fn on_open_instructions(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.chat_handle
            .dispatch(ShellChatAction::OpenCustomInstructions, window, cx);
    }

    fn on_open_credentials(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.chat_handle
            .dispatch(ShellChatAction::OpenCredentialSettings, window, cx);
    }
}

impl Render for ShellView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let context = self.chat_handle.context(cx);
        let context_label = SharedString::from(active_context_label(&context));
        let active_mode = self.state.active_mode;
        let sidebar_collapsed = self.state.sidebar_collapsed;
        let background = theme.surface(BackgroundToken::Primary);

        let center_panel = match center_panel_for_mode(active_mode) {
            CenterPanelKind::Chat => div()
                .flex_1()
                .min_h_0()
                .min_w(px(0.))
                .child(self.chat_handle.chat_view.clone())
                .into_any_element(),
            CenterPanelKind::Placeholder(_) => {
                let placeholder = placeholder_for_mode(active_mode);
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w(px(0.))
                    .child(render_mode_placeholder(theme, placeholder))
                    .into_any_element()
            }
        };

        v_flex()
            .size_full()
            .min_h_0()
            .bg(background)
            .child(shell_top_bar_row(
                theme,
                context_label,
                active_mode,
                cx.listener(Self::on_toggle_sidebar),
                cx.listener(Self::on_select_editor),
                cx.listener(Self::on_select_chat),
                cx.listener(Self::on_select_terminal),
                cx.listener(Self::on_open_instructions),
                cx.listener(Self::on_open_credentials),
            ))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .flex_row()
                    .when(!sidebar_collapsed, |row| {
                        row.child(div().flex_shrink_0().w(px(0.)).h_full())
                    })
                    .child(center_panel),
            )
    }
}

/// Mounts the workspace shell around the chat entity.
pub fn shell_screen(
    theme: OpenCoreTheme,
    chat_view: Entity<ChatView>,
    cx: &mut gpui::App,
) -> Entity<ShellView> {
    let handle = ShellChatHandle::new(chat_view);
    cx.new(|cx| ShellView::new(theme, handle, cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_state_defaults_to_chat_mode() {
        let state = ShellState::default();
        assert_eq!(state.active_mode, WorkspaceMode::Chat);
    }
}
