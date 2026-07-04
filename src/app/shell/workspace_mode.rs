//! Ephemeral workspace mode — which center panel the shell shows.

/// Active center workspace panel (not persisted in preferences).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_chat() {
        assert_eq!(WorkspaceMode::default(), WorkspaceMode::Chat);
    }

    #[test]
    fn all_modes_have_distinct_labels() {
        let labels: Vec<_> = WorkspaceMode::ALL.iter().map(|m| m.label()).collect();
        assert_eq!(labels, vec!["Editor", "Chat", "Terminal"]);
    }
}
