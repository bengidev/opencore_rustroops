//! Top-bar conversation context label derived from shell-readable chat state.

use crate::chat::ChatShellContext;

const UNTITLED_LABEL: &str = "New conversation";

/// Resolves the top-bar context label for the active thread.
pub fn active_context_label(context: &ChatShellContext) -> String {
    let Some(thread_id) = context.active_thread_id else {
        return UNTITLED_LABEL.into();
    };

    context
        .threads
        .iter()
        .find(|thread| thread.id == thread_id)
        .and_then(|thread| thread.title.as_ref())
        .map(|title| title.trim())
        .filter(|title| !title.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| UNTITLED_LABEL.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ThreadInfo;

    fn context_with_threads(threads: Vec<ThreadInfo>, active_id: Option<i64>) -> ChatShellContext {
        ChatShellContext {
            active_thread_id: active_id,
            active_thread_title: String::new(),
            threads,
            thread_settings: Default::default(),
            is_streaming: false,
            credentials_missing: false,
        }
    }

    #[test]
    fn untitled_thread_shows_new_conversation() {
        let context = context_with_threads(
            vec![ThreadInfo {
                id: 1,
                title: None,
                created_at: String::new(),
                model_id: String::new(),
            }],
            Some(1),
        );
        assert_eq!(active_context_label(&context), "New conversation");
    }

    #[test]
    fn titled_thread_shows_title() {
        let context = context_with_threads(
            vec![ThreadInfo {
                id: 2,
                title: Some("Rust workspace".into()),
                created_at: String::new(),
                model_id: String::new(),
            }],
            Some(2),
        );
        assert_eq!(active_context_label(&context), "Rust workspace");
    }

    #[test]
    fn label_uses_active_thread_not_first_in_list() {
        let context = context_with_threads(
            vec![
                ThreadInfo {
                    id: 1,
                    title: Some("First".into()),
                    created_at: String::new(),
                    model_id: String::new(),
                },
                ThreadInfo {
                    id: 2,
                    title: Some("Active".into()),
                    created_at: String::new(),
                    model_id: String::new(),
                },
            ],
            Some(2),
        );
        assert_eq!(active_context_label(&context), "Active");
    }

    #[test]
    fn missing_active_thread_shows_new_conversation() {
        let context = context_with_threads(vec![], None);
        assert_eq!(active_context_label(&context), "New conversation");
    }
}
