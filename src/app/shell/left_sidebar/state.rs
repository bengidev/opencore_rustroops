//! Sidebar view model: search, scope, selection, and shelf partitioning.

use std::collections::{HashMap, HashSet};

use super::demo_data::{
    ALL_ATOMS_LABEL, DEMO_DRAFT, DEMO_ATOMS, DemoDraft, DemoAtom, AtomShelf, AtomStatus,
};

pub const SETTLED_PAGE_INITIAL: usize = 10;
pub const SETTLED_PAGE_SIZE: usize = 25;

/// Collapsible shelf sections that can be revealed with a height tween.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealShelf {
    Pinned,
    Settled,
    Archived,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FooterMode {
    #[default]
    Utilities,
    Back,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FooterBackContext {
    Settings,
    PullRequests,
    Usage,
}

#[derive(Clone, Debug, Default)]
pub struct SidebarViewModel {
    pub search_query: String,
    pub project_scope: Option<String>,
    pub pinned_expanded: bool,
    pub settled_expanded: bool,
    pub archived_expanded: bool,
    pub settled_visible_limit: usize,
    pub active_atom_id: String,
    pub selected_atom_ids: HashSet<String>,
    pub pinned_order: Vec<String>,
    pub active_order: Vec<String>,
    pub settled_order: Vec<String>,
    pub archived_order: Vec<String>,
    pub hovered_atom_id: Option<String>,
    pub footer_mode: FooterMode,
    pub footer_back_context: Option<FooterBackContext>,
    pub show_update_pill: bool,
    pub draft_visible: bool,
    pub atom_shelf_overrides: HashMap<String, AtomShelf>,
    pub archived_atom_ids: HashSet<String>,
    pub display_title_overrides: HashMap<String, String>,
    pub renaming_atom_id: Option<String>,
}

impl SidebarViewModel {
    pub fn new(active_atom_id: impl Into<String>) -> Self {
        let active_atom_id = active_atom_id.into();
        let pinned_order = shelf_order_from_demo(AtomShelf::Pinned);
        let active_order = shelf_order_from_demo(AtomShelf::Active);
        let settled_order = shelf_order_from_demo(AtomShelf::Settled);

        Self {
            search_query: String::new(),
            project_scope: None,
            pinned_expanded: true,
            settled_expanded: true,
            archived_expanded: false,
            settled_visible_limit: SETTLED_PAGE_INITIAL,
            active_atom_id,
            selected_atom_ids: HashSet::new(),
            pinned_order,
            active_order: active_order_with_draft(active_order, true),
            settled_order,
            archived_order: Vec::new(),
            hovered_atom_id: None,
            footer_mode: FooterMode::Utilities,
            footer_back_context: None,
            show_update_pill: true,
            draft_visible: true,
            atom_shelf_overrides: HashMap::new(),
            archived_atom_ids: HashSet::new(),
            display_title_overrides: HashMap::new(),
            renaming_atom_id: None,
        }
    }

    pub fn open_footer_utility(&mut self, context: FooterBackContext) {
        self.footer_mode = FooterMode::Back;
        self.footer_back_context = Some(context);
    }

    pub fn close_footer_utility(&mut self) {
        self.footer_mode = FooterMode::Utilities;
        self.footer_back_context = None;
    }

    pub fn effective_shelf(&self, atom: &DemoAtom) -> AtomShelf {
        self.atom_shelf_overrides
            .get(atom.id)
            .copied()
            .unwrap_or(atom.shelf)
    }

    pub fn is_archived(&self, atom: &DemoAtom) -> bool {
        self.archived_atom_ids.contains(atom.id)
    }

    pub fn display_title(&self, atom: &DemoAtom) -> String {
        self.display_title_overrides
            .get(atom.id)
            .cloned()
            .unwrap_or_else(|| atom.title.to_string())
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
    }

    pub fn is_searching(&self) -> bool {
        !self.search_query.trim().is_empty()
    }

    pub fn scoped_label(&self) -> &str {
        match &self.project_scope {
            None => ALL_ATOMS_LABEL,
            Some(key) => DEMO_ATOMS
                .iter()
                .find(|t| t.project_key == key)
                .map(|t| t.project_title)
                .unwrap_or(ALL_ATOMS_LABEL),
        }
    }

    pub fn visible_atoms(&self) -> Vec<&DemoAtom> {
        let query = self.search_query.trim().to_ascii_lowercase();
        DEMO_ATOMS
            .iter()
            .filter(|t| !self.is_archived(t))
            .filter(|t| self.matches_scope(t))
            .filter(|t| {
                query.is_empty() || self.display_title(t).to_ascii_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn pinned_atoms(&self) -> Vec<&DemoAtom> {
        self.pinned_order
            .iter()
            .filter_map(|id| DEMO_ATOMS.iter().find(|t| t.id == id))
            .filter(|t| !self.is_archived(t))
            .filter(|t| self.matches_scope(t))
            .filter(|t| self.effective_shelf(t) == AtomShelf::Pinned)
            .filter(|_| !self.is_searching())
            .collect()
    }

    pub fn active_atoms(&self) -> Vec<&DemoAtom> {
        if self.is_searching() {
            return Vec::new();
        }
        self.atoms_in_order(&self.active_order, AtomShelf::Active)
    }

    /// Active section row ids (draft + inbox atoms) in display order.
    pub fn active_section_ids(&self) -> Vec<String> {
        if self.is_searching() {
            return Vec::new();
        }
        self.active_order
            .iter()
            .filter(|id| {
                if id.as_str() == DEMO_DRAFT.id {
                    self.draft_visible
                } else {
                    self.active_atoms().iter().any(|t| t.id == id.as_str())
                }
            })
            .cloned()
            .collect()
    }

    pub fn is_draft_active(&self) -> bool {
        self.active_atom_id == DEMO_DRAFT.id
    }

    pub fn is_draft_selected(&self) -> bool {
        self.selected_atom_ids.contains(DEMO_DRAFT.id)
    }

    pub fn settled_atoms(&self) -> Vec<&DemoAtom> {
        if self.is_searching() {
            return Vec::new();
        }
        self.atoms_in_order(&self.settled_order, AtomShelf::Settled)
    }

    pub fn archived_atoms(&self) -> Vec<&DemoAtom> {
        if self.is_searching() {
            return Vec::new();
        }
        self.archived_order
            .iter()
            .filter_map(|id| DEMO_ATOMS.iter().find(|t| t.id == id))
            .filter(|t| self.is_archived(t))
            .filter(|t| self.matches_scope(t))
            .collect()
    }

    pub fn search_results(&self) -> Vec<&DemoAtom> {
        if !self.is_searching() {
            return Vec::new();
        }
        self.visible_atoms()
    }

    pub fn settled_visible(&self) -> Vec<&DemoAtom> {
        self.settled_atoms()
            .into_iter()
            .take(self.settled_visible_limit)
            .collect()
    }

    pub fn settled_has_more(&self) -> bool {
        self.settled_atoms().len() > self.settled_visible_limit
    }

    pub fn show_more_settled(&mut self) {
        self.settled_visible_limit += SETTLED_PAGE_SIZE;
    }

    pub fn pinned_label(&self) -> String {
        let count = self.pinned_atoms().len();
        if self.pinned_expanded {
            "Pinned".to_string()
        } else {
            format!("Pinned ({count})")
        }
    }

    pub fn settled_label(&self) -> String {
        let count = self.settled_atoms().len();
        if self.settled_expanded {
            "Settled".to_string()
        } else {
            format!("Settled ({count})")
        }
    }

    pub fn archived_label(&self) -> String {
        let count = self.archived_atoms().len();
        if self.archived_expanded {
            "Archived".to_string()
        } else {
            format!("Archived ({count})")
        }
    }

    pub fn is_active(&self, atom: &DemoAtom) -> bool {
        atom.id == self.active_atom_id
    }

    pub fn is_selected(&self, atom: &DemoAtom) -> bool {
        self.selected_atom_ids.contains(atom.id)
    }

    pub fn is_renaming(&self, atom: &DemoAtom) -> bool {
        self.renaming_atom_id.as_deref() == Some(atom.id)
    }

    pub fn begin_rename(&mut self, atom_id: &str) {
        self.renaming_atom_id = Some(atom_id.to_string());
    }

    pub fn commit_rename(&mut self, atom_id: &str, title: String) {
        let trimmed = title.trim();
        if !trimmed.is_empty() {
            self.display_title_overrides
                .insert(atom_id.to_string(), trimmed.to_string());
        }
        self.renaming_atom_id = None;
    }

    pub fn cancel_rename(&mut self) {
        self.renaming_atom_id = None;
    }

    pub fn renaming_title(&self, atom: &DemoAtom) -> String {
        self.display_title(atom)
    }

    pub fn should_recede(&self, atom: &DemoAtom) -> bool {
        if self.is_active(atom) || self.is_selected(atom) {
            return false;
        }
        if self.is_archived(atom) || self.effective_shelf(atom) == AtomShelf::Settled {
            return true;
        }
        if atom.is_unread || atom.is_woke {
            return false;
        }
        matches!(
            atom.status,
            AtomStatus::Ready
                | AtomStatus::Working
                | AtomStatus::Monitoring
                | AtomStatus::Approval
                | AtomStatus::Input
        )
    }

    pub fn ordered_visible_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        if self.draft_visible {
            ids.push(DEMO_DRAFT.id.to_string());
        }
        if self.pinned_expanded {
            ids.extend(self.pinned_atoms().iter().map(|t| t.id.to_string()));
        }
        ids.extend(self.active_atoms().iter().map(|t| t.id.to_string()));
        if self.settled_expanded {
            ids.extend(self.settled_visible().iter().map(|t| t.id.to_string()));
        }
        ids
    }

    pub fn activate_atom(&mut self, atom_id: &str) {
        self.renaming_atom_id = None;
        self.active_atom_id = atom_id.to_string();
        self.selected_atom_ids.clear();
        self.selected_atom_ids.insert(atom_id.to_string());
    }

    pub fn toggle_atom_selection(&mut self, atom_id: &str, range_select: bool) {
        if range_select {
            let ordered = self.ordered_visible_ids();
            let anchor = self
                .selected_atom_ids
                .iter()
                .next()
                .cloned()
                .or_else(|| Some(self.active_atom_id.clone()));
            if let Some(anchor_id) = anchor {
                let from = ordered.iter().position(|id| id == &anchor_id);
                let to = ordered.iter().position(|id| id == atom_id);
                if let (Some(from), Some(to)) = (from, to) {
                    let (start, end) = if from <= to { (from, to) } else { (to, from) };
                    for id in ordered.iter().take(end + 1).skip(start) {
                        self.selected_atom_ids.insert(id.clone());
                    }
                    return;
                }
            }
        }

        if self.selected_atom_ids.contains(atom_id) {
            self.selected_atom_ids.remove(atom_id);
        } else {
            self.selected_atom_ids.insert(atom_id.to_string());
        }
    }

    pub fn can_reorder_atoms(&self, dragged_id: &str, target_id: &str) -> bool {
        if dragged_id == target_id {
            return false;
        }
        if self.archived_atom_ids.contains(dragged_id)
            && self.archived_atom_ids.contains(target_id)
        {
            return true;
        }
        self.shelf_for_id(dragged_id) == self.shelf_for_id(target_id)
    }

    pub fn reorder_atom(&mut self, dragged_id: &str, target_id: &str, insert_after: bool) {
        if dragged_id == target_id {
            return;
        }
        if self.archived_atom_ids.contains(dragged_id)
            && self.archived_atom_ids.contains(target_id)
        {
            reorder_ids(
                &mut self.archived_order,
                dragged_id,
                target_id,
                insert_after,
            );
            return;
        }
        let dragged_shelf = self.shelf_for_id(dragged_id);
        let target_shelf = self.shelf_for_id(target_id);
        if dragged_shelf != target_shelf {
            return;
        }
        let order = self.shelf_order_mut(dragged_shelf);
        reorder_ids(order, dragged_id, target_id, insert_after);
    }

    pub fn can_move_atom(&self, atom_id: &str, delta: isize) -> bool {
        let order = self.shelf_order_slice(atom_id);
        let pos = order.iter().position(|id| id == atom_id);
        match pos {
            Some(pos) => {
                let new_pos = pos as isize + delta;
                new_pos >= 0 && new_pos < order.len() as isize
            }
            None => false,
        }
    }

    pub fn move_atom(&mut self, atom_id: &str, delta: isize) {
        if self.archived_atom_ids.contains(atom_id) {
            let pos = self.archived_order.iter().position(|id| id == atom_id);
            if let Some(pos) = pos {
                let new_pos =
                    (pos as isize + delta).clamp(0, self.archived_order.len() as isize - 1);
                if new_pos as usize != pos {
                    let id = self.archived_order.remove(pos);
                    self.archived_order.insert(new_pos as usize, id);
                }
            }
            return;
        }
        let shelf = self.shelf_for_id(atom_id);
        let order = self.shelf_order_mut(shelf);
        let pos = order.iter().position(|id| id == atom_id);
        if let Some(pos) = pos {
            let new_pos = (pos as isize + delta).clamp(0, order.len() as isize - 1);
            if new_pos as usize != pos {
                let id = order.remove(pos);
                order.insert(new_pos as usize, id);
            }
        }
    }

    pub fn discard_draft(&mut self) {
        self.draft_visible = false;
        self.active_order.retain(|id| id != DEMO_DRAFT.id);
        if self.active_atom_id == DEMO_DRAFT.id {
            self.active_atom_id = self
                .active_atoms()
                .first()
                .map(|t| t.id.to_string())
                .unwrap_or_else(|| "active-1".to_string());
        }
        self.selected_atom_ids.remove(DEMO_DRAFT.id);
    }

    pub fn show_draft(&mut self) {
        self.draft_visible = true;
        if !self.active_order.iter().any(|id| id == DEMO_DRAFT.id) {
            self.active_order.insert(0, DEMO_DRAFT.id.to_string());
        }
    }

    pub fn activate_from_search(&mut self, atom_id: &str) {
        self.prepare_atom_reveal(atom_id);
        self.activate_atom(atom_id);
        self.clear_search();
    }

    /// Expands settled pagination when needed; shelf expand animation is handled by the panel.
    pub fn prepare_atom_reveal(&mut self, atom_id: &str) {
        if self.archived_atom_ids.contains(atom_id) {
            return;
        }
        if self.shelf_for_id(atom_id) == AtomShelf::Settled {
            self.reveal_settled_atom(atom_id);
        }
    }

    pub fn reveal_shelf_target(&self, atom_id: &str) -> Option<RevealShelf> {
        if self.archived_atom_ids.contains(atom_id) {
            return Some(RevealShelf::Archived);
        }
        match self.shelf_for_id(atom_id) {
            AtomShelf::Pinned => Some(RevealShelf::Pinned),
            AtomShelf::Settled => Some(RevealShelf::Settled),
            AtomShelf::Active => None,
        }
    }

    fn reveal_settled_atom(&mut self, atom_id: &str) {
        let settled_len = self.settled_atoms().len();
        if let Some(pos) = self
            .settled_atoms()
            .iter()
            .position(|t| t.id == atom_id)
        {
            let needed = pos + 1;
            if self.settled_visible_limit < needed {
                while self.settled_visible_limit < needed && self.settled_has_more() {
                    self.settled_visible_limit += SETTLED_PAGE_SIZE;
                }
                if self.settled_visible_limit < needed {
                    self.settled_visible_limit = settled_len;
                }
            }
        }
    }

    pub fn settle_atom(&mut self, atom_id: &str) {
        self.move_atom_to_shelf(atom_id, AtomShelf::Settled);
    }

    pub fn unsettle_atom(&mut self, atom_id: &str) {
        if self.archived_atom_ids.contains(atom_id) {
            return;
        }
        self.move_atom_to_shelf(atom_id, AtomShelf::Active);
    }

    pub fn pin_atom(&mut self, atom_id: &str) {
        if self.archived_atom_ids.contains(atom_id) {
            return;
        }
        self.move_atom_to_shelf(atom_id, AtomShelf::Pinned);
    }

    pub fn unpin_atom(&mut self, atom_id: &str) {
        if self.archived_atom_ids.contains(atom_id) {
            return;
        }
        self.move_atom_to_shelf(atom_id, AtomShelf::Active);
    }

    pub fn archive_atom(&mut self, atom_id: &str) {
        self.remove_from_shelf_orders(atom_id);
        self.archived_atom_ids.insert(atom_id.to_string());
        if !self.archived_order.iter().any(|id| id == atom_id) {
            self.archived_order.push(atom_id.to_string());
        }
        self.selected_atom_ids.remove(atom_id);
        if self.renaming_atom_id.as_deref() == Some(atom_id) {
            self.renaming_atom_id = None;
        }
        if self.active_atom_id == atom_id {
            self.active_atom_id = DEMO_DRAFT.id.to_string();
        }
    }

    pub fn unarchive_atom(&mut self, atom_id: &str) {
        self.archived_atom_ids.remove(atom_id);
        self.archived_order.retain(|id| id != atom_id);
        if !self.active_order.iter().any(|id| id == atom_id) {
            self.active_order.push(atom_id.to_string());
        }
    }

    fn move_atom_to_shelf(&mut self, atom_id: &str, shelf: AtomShelf) {
        self.remove_from_shelf_orders(atom_id);
        self.atom_shelf_overrides
            .insert(atom_id.to_string(), shelf);
        self.append_to_shelf_order(atom_id, shelf);
    }

    fn atoms_in_order(&self, order: &[String], shelf: AtomShelf) -> Vec<&DemoAtom> {
        order
            .iter()
            .filter_map(|id| DEMO_ATOMS.iter().find(|t| t.id == id))
            .filter(|t| !self.is_archived(t))
            .filter(|t| self.effective_shelf(t) == shelf)
            .filter(|t| self.matches_scope(t))
            .collect()
    }

    fn shelf_for_id(&self, atom_id: &str) -> AtomShelf {
        if atom_id == DEMO_DRAFT.id {
            return AtomShelf::Active;
        }
        DEMO_ATOMS
            .iter()
            .find(|t| t.id == atom_id)
            .map(|t| self.effective_shelf(t))
            .unwrap_or(AtomShelf::Active)
    }

    fn shelf_order_slice(&self, atom_id: &str) -> &[String] {
        if self.archived_atom_ids.contains(atom_id) {
            return &self.archived_order;
        }
        match self.shelf_for_id(atom_id) {
            AtomShelf::Pinned => &self.pinned_order,
            AtomShelf::Active => &self.active_order,
            AtomShelf::Settled => &self.settled_order,
        }
    }

    fn shelf_order_mut(&mut self, shelf: AtomShelf) -> &mut Vec<String> {
        match shelf {
            AtomShelf::Pinned => &mut self.pinned_order,
            AtomShelf::Active => &mut self.active_order,
            AtomShelf::Settled => &mut self.settled_order,
        }
    }

    fn remove_from_shelf_orders(&mut self, atom_id: &str) {
        self.pinned_order.retain(|id| id != atom_id);
        self.active_order.retain(|id| id != atom_id);
        self.settled_order.retain(|id| id != atom_id);
        self.archived_order.retain(|id| id != atom_id);
    }

    fn append_to_shelf_order(&mut self, atom_id: &str, shelf: AtomShelf) {
        let order = self.shelf_order_mut(shelf);
        if !order.iter().any(|id| id == atom_id) {
            order.push(atom_id.to_string());
        }
    }

    fn matches_scope(&self, atom: &DemoAtom) -> bool {
        match &self.project_scope {
            None => true,
            Some(key) => atom.project_key == key,
        }
    }
}

pub fn demo_draft() -> &'static DemoDraft {
    &DEMO_DRAFT
}

fn shelf_order_from_demo(shelf: AtomShelf) -> Vec<String> {
    DEMO_ATOMS
        .iter()
        .filter(|t| t.shelf == shelf)
        .map(|t| t.id.to_string())
        .collect()
}

fn active_order_with_draft(active_order: Vec<String>, draft_visible: bool) -> Vec<String> {
    let mut order = active_order;
    if draft_visible && !order.iter().any(|id| id == DEMO_DRAFT.id) {
        order.insert(0, DEMO_DRAFT.id.to_string());
    }
    order
}

fn reorder_ids(order: &mut Vec<String>, dragged_id: &str, target_id: &str, insert_after: bool) {
    if dragged_id == target_id {
        return;
    }
    let from_pos = order.iter().position(|id| id == dragged_id);
    let target_pos = order.iter().position(|id| id == target_id);
    if let (Some(from), Some(target)) = (from_pos, target_pos) {
        let id = order.remove(from);
        let mut insert_at = target;
        if insert_after {
            insert_at += 1;
        }
        if from < insert_at {
            insert_at -= 1;
        }
        insert_at = insert_at.min(order.len());
        order.insert(insert_at, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_moves_atom_to_pinned_shelf() {
        let mut view = SidebarViewModel::new("active-1");
        view.pin_atom("active-2");
        assert!(view.pinned_atoms().iter().any(|t| t.id == "active-2"));
        assert!(!view.active_atoms().iter().any(|t| t.id == "active-2"));
    }

    #[test]
    fn unpin_moves_atom_back_to_active_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.unpin_atom("pinned-1");
        assert!(!view.pinned_atoms().iter().any(|t| t.id == "pinned-1"));
        assert!(view.active_atoms().iter().any(|t| t.id == "pinned-1"));
    }

    #[test]
    fn settle_moves_atom_to_settled_shelf() {
        let mut view = SidebarViewModel::new("active-1");
        view.settle_atom("active-2");
        assert!(view.settled_atoms().iter().any(|t| t.id == "active-2"));
        assert!(!view.active_atoms().iter().any(|t| t.id == "active-2"));
    }

    #[test]
    fn unsettle_moves_atom_back_to_active_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.unsettle_atom("settled-1");
        assert!(!view.settled_atoms().iter().any(|t| t.id == "settled-1"));
        assert!(view.active_atoms().iter().any(|t| t.id == "settled-1"));
    }

    #[test]
    fn settled_atoms_always_recede_unless_active_or_selected() {
        let view = SidebarViewModel::new("active-1");
        let failed = DEMO_ATOMS.iter().find(|t| t.id == "settled-5").unwrap();
        let woke = DEMO_ATOMS.iter().find(|t| t.id == "settled-6").unwrap();
        assert!(view.should_recede(failed));
        assert!(view.should_recede(woke));
    }

    #[test]
    fn can_reorder_atoms_requires_matching_shelf() {
        let view = SidebarViewModel::new("active-1");
        assert!(view.can_reorder_atoms("active-1", "active-2"));
        assert!(!view.can_reorder_atoms("active-1", "pinned-1"));
        assert!(view.can_reorder_atoms("settled-1", "settled-2"));
        assert!(!view.can_reorder_atoms("settled-1", "active-1"));
    }

    #[test]
    fn reorder_settled_moves_atom_within_settled_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.reorder_atom("settled-1", "settled-2", true);
        let order: Vec<_> = view.settled_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order[0], "settled-2");
        assert_eq!(order[1], "settled-1");
    }

    #[test]
    fn reorder_pinned_moves_atom_relative_to_target() {
        let mut view = SidebarViewModel::new("active-1");
        view.pinned_order = vec!["pinned-1".into(), "pinned-2".into()];
        view.reorder_atom("pinned-1", "pinned-2", true);
        let order: Vec<_> = view.pinned_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order, ["pinned-2", "pinned-1"]);
    }

    #[test]
    fn reorder_active_moves_atom_within_active_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.discard_draft();
        view.reorder_atom("active-1", "active-2", true);
        let order: Vec<_> = view.active_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order[0], "active-2");
        assert_eq!(order[1], "active-1");
    }

    #[test]
    fn move_atom_stays_within_settled_shelf_bounds() {
        let mut view = SidebarViewModel::new("active-1");
        view.settled_order = vec!["settled-1".into(), "settled-2".into()];
        assert!(!view.can_move_atom("settled-1", -1));
        assert!(view.can_move_atom("settled-1", 1));
        view.move_atom("settled-1", -1);
        let order: Vec<_> = view.settled_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order, ["settled-1", "settled-2"]);
        view.move_atom("settled-2", 1);
        let order: Vec<_> = view.settled_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order, ["settled-1", "settled-2"]);
        view.move_atom("settled-1", 1);
        let order: Vec<_> = view.settled_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order, ["settled-2", "settled-1"]);
    }

    #[test]
    fn archive_moves_atom_to_archived_shelf() {
        let mut view = SidebarViewModel::new("active-1");
        view.archive_atom("active-2");
        assert!(view.archived_atoms().iter().any(|t| t.id == "active-2"));
        assert!(!view.active_atoms().iter().any(|t| t.id == "active-2"));
    }

    #[test]
    fn unarchive_restores_atom_to_active_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.archive_atom("active-2");
        view.unarchive_atom("active-2");
        assert!(view.archived_atoms().is_empty());
        assert!(view.active_atoms().iter().any(|t| t.id == "active-2"));
    }

    #[test]
    fn archive_removes_atom_from_lists() {
        let mut view = SidebarViewModel::new("active-1");
        view.archive_atom("active-2");
        assert!(!view.active_atoms().iter().any(|t| t.id == "active-2"));
        assert!(view.visible_atoms().iter().all(|t| t.id != "active-2"));
    }

    #[test]
    fn activate_from_search_clears_query_and_prepares_settled_reveal() {
        let mut view = SidebarViewModel::new("active-1");
        view.search_query = "theme".to_string();
        view.settled_expanded = false;
        view.settled_visible_limit = 1;
        view.activate_from_search("settled-1");
        assert_eq!(view.active_atom_id, "settled-1");
        assert!(view.search_query.is_empty());
        assert_eq!(
            view.reveal_shelf_target("settled-1"),
            Some(RevealShelf::Settled)
        );
        assert!(view.settled_visible_limit >= 1);
        assert!(!view.settled_expanded);
    }

    #[test]
    fn commit_rename_updates_display_title() {
        let mut view = SidebarViewModel::new("active-1");
        view.begin_rename("active-1");
        view.commit_rename("active-1", "Renamed atom".to_string());
        assert_eq!(
            view.display_title(DEMO_ATOMS.iter().find(|t| t.id == "active-1").unwrap()),
            "Renamed atom"
        );
        assert!(view.renaming_atom_id.is_none());
    }

    #[test]
    fn cancel_rename_clears_state() {
        let mut view = SidebarViewModel::new("active-1");
        view.begin_rename("active-1");
        view.cancel_rename();
        assert!(view.renaming_atom_id.is_none());
    }
}
