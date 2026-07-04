//! Pure shell helpers (**Strategy** inputs for workspace chrome).

use crate::chat::ChatShellContext;

/// Fallback label when a conversation has no title.
pub const UNTITLED_CONVERSATION_LABEL: &str = "New conversation";

/// Resolves the top-bar context label from an optional thread title.
pub fn thread_context_label(title: Option<&str>) -> &str {
    title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(UNTITLED_CONVERSATION_LABEL)
}

/// Reads the active thread title from shell context for top-bar display.
pub fn context_label_from_shell_context(ctx: &ChatShellContext) -> &str {
    let from_thread_list = ctx.active_thread_id.and_then(|id| {
        ctx.threads
            .iter()
            .find(|thread| thread.id == id)
            .and_then(|thread| thread.title.as_deref())
    });
    if let Some(title) = from_thread_list {
        return thread_context_label(Some(title));
    }

    let fallback = ctx.active_thread_title.as_str();
    if fallback == "New Chat" {
        thread_context_label(None)
    } else {
        thread_context_label(Some(fallback))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ThreadInfo;

    #[test]
    fn thread_context_label_uses_new_conversation_when_title_missing() {
        assert_eq!(thread_context_label(None), "New conversation");
    }

    #[test]
    fn thread_context_label_uses_new_conversation_when_title_blank() {
        assert_eq!(thread_context_label(Some("")), "New conversation");
        assert_eq!(thread_context_label(Some("   ")), "New conversation");
    }

    #[test]
    fn thread_context_label_preserves_non_empty_title() {
        assert_eq!(thread_context_label(Some("Design review")), "Design review");
    }

    #[test]
    fn context_label_from_shell_context_uses_active_thread_title_fallback() {
        let ctx = ChatShellContext {
            active_thread_id: Some(1),
            active_thread_title: "Renamed thread".into(),
            threads: vec![],
            thread_settings: Default::default(),
            is_streaming: false,
            credentials_missing: false,
        };
        assert_eq!(context_label_from_shell_context(&ctx), "Renamed thread");
    }

    #[test]
    fn context_label_from_shell_context_uses_active_thread_title() {
        let ctx = ChatShellContext {
            active_thread_id: Some(2),
            active_thread_title: "ignored".into(),
            threads: vec![
                ThreadInfo {
                    id: 1,
                    title: Some("First".into()),
                    created_at: "2026-01-01".into(),
                    model_id: "model".into(),
                },
                ThreadInfo {
                    id: 2,
                    title: Some("Active".into()),
                    created_at: "2026-01-02".into(),
                    model_id: "model".into(),
                },
            ],
            thread_settings: Default::default(),
            is_streaming: false,
            credentials_missing: false,
        };
        assert_eq!(context_label_from_shell_context(&ctx), "Active");
    }

    #[test]
    fn context_label_from_shell_context_falls_back_when_untitled() {
        let ctx = ChatShellContext {
            active_thread_id: Some(1),
            active_thread_title: "New Chat".into(),
            threads: vec![ThreadInfo {
                id: 1,
                title: None,
                created_at: "2026-01-01".into(),
                model_id: "model".into(),
            }],
            thread_settings: Default::default(),
            is_streaming: false,
            credentials_missing: false,
        };
        assert_eq!(context_label_from_shell_context(&ctx), "New conversation");
    }
}
