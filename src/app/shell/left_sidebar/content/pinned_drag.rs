//! Drag-and-drop payload and drop-indicator state for atom row reordering.

use super::super::demo_data::AtomShelf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomDragScope {
    Pinned,
    Active,
    Settled,
    Archived,
}

impl AtomDragScope {
    pub fn from_atom(_atom_id: &str, effective_shelf: AtomShelf, is_archived: bool) -> Self {
        if is_archived {
            return Self::Archived;
        }
        match effective_shelf {
            AtomShelf::Pinned => Self::Pinned,
            AtomShelf::Active => Self::Active,
            AtomShelf::Settled => Self::Settled,
        }
    }

    pub fn allows_drop(self, target: Self) -> bool {
        self == target
    }
}

#[derive(Clone, Debug)]
pub struct PinnedAtomDrag {
    pub atom_id: String,
    pub scope: AtomDragScope,
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
    pub fn for_atom(&self, atom_id: &str) -> PinnedRowDragUi {
        let (drop_above, drop_below) = match &self.drop_target {
            Some((id, insert_after)) if id == atom_id => (!*insert_after, *insert_after),
            _ => (false, false),
        };
        PinnedRowDragUi {
            is_source: self.dragging_id.as_deref() == Some(atom_id),
            drop_above,
            drop_below,
        }
    }
}
