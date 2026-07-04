//! Coming-soon center panels for non-chat workspace modes.

use gpui::{IntoElement, ParentElement, Styled, div, px};
use gpui_component::v_flex;

use crate::shared::theme::{ForegroundToken, LegacyTypeRole, OpenCoreTheme};

use super::shell_state::WorkspaceMode;

/// Copy shown in a mode placeholder panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModePlaceholderCopy {
    pub title: &'static str,
    pub description: &'static str,
}

/// Returns distinct placeholder copy for stub modes.
pub fn mode_placeholder_copy(mode: WorkspaceMode) -> Option<ModePlaceholderCopy> {
    match mode {
        WorkspaceMode::Editor => Some(ModePlaceholderCopy {
            title: "Editor",
            description: "Code editing and file navigation are coming soon.",
        }),
        WorkspaceMode::Terminal => Some(ModePlaceholderCopy {
            title: "Terminal",
            description: "An integrated terminal workspace is coming soon.",
        }),
        WorkspaceMode::Chat => None,
    }
}

pub fn render_mode_placeholder(mode: WorkspaceMode, theme: OpenCoreTheme) -> impl IntoElement {
    let copy = mode_placeholder_copy(mode).expect("chat mode has no placeholder");
    let foreground = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let label = theme.label;

    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .child(title_line(copy.title, foreground, label))
        .child(description_line(copy.description, muted, label))
}

fn title_line(title: &str, color: gpui::Hsla, label: LegacyTypeRole) -> impl IntoElement {
    div()
        .text_color(color)
        .text_size(px(label.size_px as f32))
        .child(title.to_string())
}

fn description_line(text: &str, color: gpui::Hsla, label: LegacyTypeRole) -> impl IntoElement {
    div()
        .max_w(px(420.))
        .text_center()
        .text_color(color)
        .text_size(px((label.size_px - 1) as f32))
        .child(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_and_terminal_placeholders_are_distinct() {
        let editor = mode_placeholder_copy(WorkspaceMode::Editor).expect("editor copy");
        let terminal = mode_placeholder_copy(WorkspaceMode::Terminal).expect("terminal copy");
        assert_ne!(editor.title, terminal.title);
        assert_ne!(editor.description, terminal.description);
    }

    #[test]
    fn chat_mode_has_no_placeholder_copy() {
        assert!(mode_placeholder_copy(WorkspaceMode::Chat).is_none());
    }
}
