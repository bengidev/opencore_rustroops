//! Drag-and-drop payload and drop-indicator state for thread row reordering.

use super::super::demo_data::ThreadShelf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadDragScope {
    Pinned,
    Active,
    Settled,
    Archived,
}

impl ThreadDragScope {
    pub fn from_thread(_thread_id: &str, effective_shelf: ThreadShelf, is_archived: bool) -> Self {
        if is_archived {
            return Self::Archived;
        }
        match effective_shelf {
            ThreadShelf::Pinned => Self::Pinned,
            ThreadShelf::Active => Self::Active,
            ThreadShelf::Settled => Self::Settled,
        }
    }

    pub fn allows_drop(self, target: Self) -> bool {
        self == target
    }
}

#[derive(Clone, Debug)]
pub struct PinnedThreadDrag {
    pub thread_id: String,
    pub scope: ThreadDragScope,
    pub title: gpui::SharedString,
    pub preview_bg: gpui::Hsla,
    pub preview_text: gpui::Hsla,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PinnedRowDragUi {
    pub is_source: bool,
    pub drop_above: bool,
    pub drop_below: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PinnedDragState {
    pub dragging_id: Option<String>,
    pub drop_target: Option<(String, bool)>,
}

impl PinnedDragState {
    pub fn for_thread(&self, thread_id: &str) -> PinnedRowDragUi {
        let (drop_above, drop_below) = match &self.drop_target {
            Some((id, insert_after)) if id == thread_id => (!*insert_after, *insert_after),
            _ => (false, false),
        };
        PinnedRowDragUi {
            is_source: self.dragging_id.as_deref() == Some(thread_id),
            drop_above,
            drop_below,
        }
    }
}
