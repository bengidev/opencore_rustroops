//! Coming-soon center panels for non-chat workspace modes.

use super::workspace_mode::WorkspaceMode;

/// Copy shown in Editor / Terminal placeholder panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModePlaceholder {
    pub heading: &'static str,
    pub description: &'static str,
}

/// Which center surface the shell should mount for a mode (**State** routing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterPanelKind {
    Chat,
    Placeholder(ModePlaceholder),
}

pub fn center_panel_for_mode(mode: WorkspaceMode) -> CenterPanelKind {
    match mode {
        WorkspaceMode::Chat => CenterPanelKind::Chat,
        WorkspaceMode::Editor => {
            CenterPanelKind::Placeholder(placeholder_for_mode(WorkspaceMode::Editor))
        }
        WorkspaceMode::Terminal => {
            CenterPanelKind::Placeholder(placeholder_for_mode(WorkspaceMode::Terminal))
        }
    }
}

pub fn placeholder_for_mode(mode: WorkspaceMode) -> ModePlaceholder {
    match mode {
        WorkspaceMode::Editor => ModePlaceholder {
            heading: "Editor",
            description: "A multi-file code editor will live here. Chat remains fully available via the Chat tab.",
        },
        WorkspaceMode::Terminal => ModePlaceholder {
            heading: "Terminal",
            description: "An integrated terminal will live here. Chat remains fully available via the Chat tab.",
        },
        WorkspaceMode::Chat => ModePlaceholder {
            heading: "Chat",
            description: "Active conversations render in this panel.",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_mode_mounts_chat_panel() {
        assert_eq!(
            center_panel_for_mode(WorkspaceMode::Chat),
            CenterPanelKind::Chat
        );
    }

    #[test]
    fn editor_and_terminal_placeholders_are_distinct() {
        let editor = placeholder_for_mode(WorkspaceMode::Editor);
        let terminal = placeholder_for_mode(WorkspaceMode::Terminal);
        assert_ne!(editor.heading, terminal.heading);
        assert_ne!(editor.description, terminal.description);
    }

    #[test]
    fn non_chat_modes_use_placeholder_panels() {
        for mode in [WorkspaceMode::Editor, WorkspaceMode::Terminal] {
            match center_panel_for_mode(mode) {
                CenterPanelKind::Placeholder(placeholder) => {
                    assert_eq!(placeholder.heading, mode.label());
                }
                CenterPanelKind::Chat => panic!("expected placeholder for {mode:?}"),
            }
        }
    }
}
