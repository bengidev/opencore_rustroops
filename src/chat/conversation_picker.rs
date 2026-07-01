//! Header conversation picker backed by persisted thread list.

use gpui::{App, Entity, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use gpui_component::Sizable;
use gpui_component::select::{SearchableVec, Select, SelectState};

use super::chat_store::ThreadInfo;

/// Select list entry wrapping [`ThreadInfo`].
#[derive(Debug, Clone)]
pub struct ThreadSelectEntry {
    pub info: ThreadInfo,
}

impl gpui_component::select::SelectItem for ThreadSelectEntry {
    type Value = i64;

    fn title(&self) -> SharedString {
        SharedString::from(self.info.title.as_deref().unwrap_or("New Chat"))
    }

    fn value(&self) -> &Self::Value {
        &self.info.id
    }

    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.info
            .title
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&query)
    }
}

pub fn entries_from_threads(threads: &[ThreadInfo]) -> SearchableVec<ThreadSelectEntry> {
    SearchableVec::new(
        threads
            .iter()
            .cloned()
            .map(|info| ThreadSelectEntry { info })
            .collect::<Vec<_>>(),
    )
}

pub fn selected_index_for_thread(
    threads: &[ThreadInfo],
    thread_id: i64,
) -> Option<gpui_component::IndexPath> {
    threads
        .iter()
        .position(|t| t.id == thread_id)
        .map(|index| gpui_component::IndexPath::default().row(index))
}

pub fn sync_thread_select(
    select: &Entity<SelectState<SearchableVec<ThreadSelectEntry>>>,
    threads: &[ThreadInfo],
    thread_id: i64,
    window: &mut Window,
    cx: &mut App,
) {
    select.update(cx, |state, cx| {
        state.set_items(entries_from_threads(threads), window, cx);
        state.set_selected_value(&thread_id, window, cx);
    });
}

/// Header thread picker rendered as a select element.
pub fn render_thread_select(
    select: &Entity<SelectState<SearchableVec<ThreadSelectEntry>>>,
) -> impl IntoElement {
    div().flex_shrink_0().min_w(px(120.)).max_w(px(240.)).child(
        Select::new(select)
            .placeholder("New Chat")
            .appearance(false)
            .small()
            .menu_width(px(320.)),
    )
}
