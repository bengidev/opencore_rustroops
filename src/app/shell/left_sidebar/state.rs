//! Sidebar view model: search, scope, selection, and shelf partitioning.

use std::collections::{HashMap, HashSet};

use super::demo_data::{
    ALL_PROJECTS_LABEL, DEMO_DRAFT, DEMO_THREADS, DemoDraft, DemoThread, ThreadShelf, ThreadStatus,
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
    pub active_thread_id: String,
    pub selected_thread_ids: HashSet<String>,
    pub pinned_order: Vec<String>,
    pub active_order: Vec<String>,
    pub settled_order: Vec<String>,
    pub archived_order: Vec<String>,
    pub hovered_thread_id: Option<String>,
    pub footer_mode: FooterMode,
    pub footer_back_context: Option<FooterBackContext>,
    pub show_update_pill: bool,
    pub draft_visible: bool,
    pub thread_shelf_overrides: HashMap<String, ThreadShelf>,
    pub archived_thread_ids: HashSet<String>,
    pub display_title_overrides: HashMap<String, String>,
    pub renaming_thread_id: Option<String>,
}

impl SidebarViewModel {
    pub fn new(active_thread_id: impl Into<String>) -> Self {
        let active_thread_id = active_thread_id.into();
        let pinned_order = shelf_order_from_demo(ThreadShelf::Pinned);
        let active_order = shelf_order_from_demo(ThreadShelf::Active);
        let settled_order = shelf_order_from_demo(ThreadShelf::Settled);

        Self {
            search_query: String::new(),
            project_scope: None,
            pinned_expanded: true,
            settled_expanded: true,
            archived_expanded: false,
            settled_visible_limit: SETTLED_PAGE_INITIAL,
            active_thread_id,
            selected_thread_ids: HashSet::new(),
            pinned_order,
            active_order: active_order_with_draft(active_order, true),
            settled_order,
            archived_order: Vec::new(),
            hovered_thread_id: None,
            footer_mode: FooterMode::Utilities,
            footer_back_context: None,
            show_update_pill: true,
            draft_visible: true,
            thread_shelf_overrides: HashMap::new(),
            archived_thread_ids: HashSet::new(),
            display_title_overrides: HashMap::new(),
            renaming_thread_id: None,
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

    pub fn effective_shelf(&self, thread: &DemoThread) -> ThreadShelf {
        self.thread_shelf_overrides
            .get(thread.id)
            .copied()
            .unwrap_or(thread.shelf)
    }

    pub fn is_archived(&self, thread: &DemoThread) -> bool {
        self.archived_thread_ids.contains(thread.id)
    }

    pub fn display_title(&self, thread: &DemoThread) -> String {
        self.display_title_overrides
            .get(thread.id)
            .cloned()
            .unwrap_or_else(|| thread.title.to_string())
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
    }

    pub fn is_searching(&self) -> bool {
        !self.search_query.trim().is_empty()
    }

    pub fn scoped_label(&self) -> &str {
        match &self.project_scope {
            None => ALL_PROJECTS_LABEL,
            Some(key) => DEMO_THREADS
                .iter()
                .find(|t| t.project_key == key)
                .map(|t| t.project_title)
                .unwrap_or(ALL_PROJECTS_LABEL),
        }
    }

    pub fn visible_threads(&self) -> Vec<&DemoThread> {
        let query = self.search_query.trim().to_ascii_lowercase();
        DEMO_THREADS
            .iter()
            .filter(|t| !self.is_archived(t))
            .filter(|t| self.matches_scope(t))
            .filter(|t| {
                query.is_empty() || self.display_title(t).to_ascii_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn pinned_threads(&self) -> Vec<&DemoThread> {
        self.pinned_order
            .iter()
            .filter_map(|id| DEMO_THREADS.iter().find(|t| t.id == id))
            .filter(|t| !self.is_archived(t))
            .filter(|t| self.matches_scope(t))
            .filter(|t| self.effective_shelf(t) == ThreadShelf::Pinned)
            .filter(|_| !self.is_searching())
            .collect()
    }

    pub fn active_threads(&self) -> Vec<&DemoThread> {
        if self.is_searching() {
            return Vec::new();
        }
        self.threads_in_order(&self.active_order, ThreadShelf::Active)
    }

    /// Active section row ids (draft + inbox threads) in display order.
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
                    self.active_threads().iter().any(|t| t.id == id.as_str())
                }
            })
            .cloned()
            .collect()
    }

    pub fn is_draft_active(&self) -> bool {
        self.active_thread_id == DEMO_DRAFT.id
    }

    pub fn is_draft_selected(&self) -> bool {
        self.selected_thread_ids.contains(DEMO_DRAFT.id)
    }

    pub fn settled_threads(&self) -> Vec<&DemoThread> {
        if self.is_searching() {
            return Vec::new();
        }
        self.threads_in_order(&self.settled_order, ThreadShelf::Settled)
    }

    pub fn archived_threads(&self) -> Vec<&DemoThread> {
        if self.is_searching() {
            return Vec::new();
        }
        self.archived_order
            .iter()
            .filter_map(|id| DEMO_THREADS.iter().find(|t| t.id == id))
            .filter(|t| self.is_archived(t))
            .filter(|t| self.matches_scope(t))
            .collect()
    }

    pub fn search_results(&self) -> Vec<&DemoThread> {
        if !self.is_searching() {
            return Vec::new();
        }
        self.visible_threads()
    }

    pub fn settled_visible(&self) -> Vec<&DemoThread> {
        self.settled_threads()
            .into_iter()
            .take(self.settled_visible_limit)
            .collect()
    }

    pub fn settled_has_more(&self) -> bool {
        self.settled_threads().len() > self.settled_visible_limit
    }

    pub fn show_more_settled(&mut self) {
        self.settled_visible_limit += SETTLED_PAGE_SIZE;
    }

    pub fn pinned_label(&self) -> String {
        let count = self.pinned_threads().len();
        if self.pinned_expanded {
            "Pinned".to_string()
        } else {
            format!("Pinned ({count})")
        }
    }

    pub fn settled_label(&self) -> String {
        let count = self.settled_threads().len();
        if self.settled_expanded {
            "Settled".to_string()
        } else {
            format!("Settled ({count})")
        }
    }

    pub fn archived_label(&self) -> String {
        let count = self.archived_threads().len();
        if self.archived_expanded {
            "Archived".to_string()
        } else {
            format!("Archived ({count})")
        }
    }

    pub fn is_active(&self, thread: &DemoThread) -> bool {
        thread.id == self.active_thread_id
    }

    pub fn is_selected(&self, thread: &DemoThread) -> bool {
        self.selected_thread_ids.contains(thread.id)
    }

    pub fn is_renaming(&self, thread: &DemoThread) -> bool {
        self.renaming_thread_id.as_deref() == Some(thread.id)
    }

    pub fn begin_rename(&mut self, thread_id: &str) {
        self.renaming_thread_id = Some(thread_id.to_string());
    }

    pub fn commit_rename(&mut self, thread_id: &str, title: String) {
        let trimmed = title.trim();
        if !trimmed.is_empty() {
            self.display_title_overrides
                .insert(thread_id.to_string(), trimmed.to_string());
        }
        self.renaming_thread_id = None;
    }

    pub fn cancel_rename(&mut self) {
        self.renaming_thread_id = None;
    }

    pub fn renaming_title(&self, thread: &DemoThread) -> String {
        self.display_title(thread)
    }

    pub fn should_recede(&self, thread: &DemoThread) -> bool {
        if self.is_active(thread) || self.is_selected(thread) {
            return false;
        }
        if self.is_archived(thread) || self.effective_shelf(thread) == ThreadShelf::Settled {
            return true;
        }
        if thread.is_unread || thread.is_woke {
            return false;
        }
        matches!(
            thread.status,
            ThreadStatus::Ready
                | ThreadStatus::Working
                | ThreadStatus::Monitoring
                | ThreadStatus::Approval
                | ThreadStatus::Input
        )
    }

    pub fn ordered_visible_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        if self.draft_visible {
            ids.push(DEMO_DRAFT.id.to_string());
        }
        if self.pinned_expanded {
            ids.extend(self.pinned_threads().iter().map(|t| t.id.to_string()));
        }
        ids.extend(self.active_threads().iter().map(|t| t.id.to_string()));
        if self.settled_expanded {
            ids.extend(self.settled_visible().iter().map(|t| t.id.to_string()));
        }
        ids
    }

    pub fn activate_thread(&mut self, thread_id: &str) {
        self.renaming_thread_id = None;
        self.active_thread_id = thread_id.to_string();
        self.selected_thread_ids.clear();
        self.selected_thread_ids.insert(thread_id.to_string());
    }

    pub fn toggle_thread_selection(&mut self, thread_id: &str, range_select: bool) {
        if range_select {
            let ordered = self.ordered_visible_ids();
            let anchor = self
                .selected_thread_ids
                .iter()
                .next()
                .cloned()
                .or_else(|| Some(self.active_thread_id.clone()));
            if let Some(anchor_id) = anchor {
                let from = ordered.iter().position(|id| id == &anchor_id);
                let to = ordered.iter().position(|id| id == thread_id);
                if let (Some(from), Some(to)) = (from, to) {
                    let (start, end) = if from <= to { (from, to) } else { (to, from) };
                    for id in ordered.iter().take(end + 1).skip(start) {
                        self.selected_thread_ids.insert(id.clone());
                    }
                    return;
                }
            }
        }

        if self.selected_thread_ids.contains(thread_id) {
            self.selected_thread_ids.remove(thread_id);
        } else {
            self.selected_thread_ids.insert(thread_id.to_string());
        }
    }

    pub fn can_reorder_threads(&self, dragged_id: &str, target_id: &str) -> bool {
        if dragged_id == target_id {
            return false;
        }
        if self.archived_thread_ids.contains(dragged_id)
            && self.archived_thread_ids.contains(target_id)
        {
            return true;
        }
        self.shelf_for_id(dragged_id) == self.shelf_for_id(target_id)
    }

    pub fn reorder_thread(&mut self, dragged_id: &str, target_id: &str, insert_after: bool) {
        if dragged_id == target_id {
            return;
        }
        if self.archived_thread_ids.contains(dragged_id)
            && self.archived_thread_ids.contains(target_id)
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

    pub fn can_move_thread(&self, thread_id: &str, delta: isize) -> bool {
        let order = self.shelf_order_slice(thread_id);
        let pos = order.iter().position(|id| id == thread_id);
        match pos {
            Some(pos) => {
                let new_pos = pos as isize + delta;
                new_pos >= 0 && new_pos < order.len() as isize
            }
            None => false,
        }
    }

    pub fn move_thread(&mut self, thread_id: &str, delta: isize) {
        if self.archived_thread_ids.contains(thread_id) {
            let pos = self.archived_order.iter().position(|id| id == thread_id);
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
        let shelf = self.shelf_for_id(thread_id);
        let order = self.shelf_order_mut(shelf);
        let pos = order.iter().position(|id| id == thread_id);
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
        if self.active_thread_id == DEMO_DRAFT.id {
            self.active_thread_id = self
                .active_threads()
                .first()
                .map(|t| t.id.to_string())
                .unwrap_or_else(|| "active-1".to_string());
        }
        self.selected_thread_ids.remove(DEMO_DRAFT.id);
    }

    pub fn show_draft(&mut self) {
        self.draft_visible = true;
        if !self.active_order.iter().any(|id| id == DEMO_DRAFT.id) {
            self.active_order.insert(0, DEMO_DRAFT.id.to_string());
        }
    }

    pub fn activate_from_search(&mut self, thread_id: &str) {
        self.prepare_thread_reveal(thread_id);
        self.activate_thread(thread_id);
        self.clear_search();
    }

    /// Expands settled pagination when needed; shelf expand animation is handled by the panel.
    pub fn prepare_thread_reveal(&mut self, thread_id: &str) {
        if self.archived_thread_ids.contains(thread_id) {
            return;
        }
        if self.shelf_for_id(thread_id) == ThreadShelf::Settled {
            self.reveal_settled_thread(thread_id);
        }
    }

    pub fn reveal_shelf_target(&self, thread_id: &str) -> Option<RevealShelf> {
        if self.archived_thread_ids.contains(thread_id) {
            return Some(RevealShelf::Archived);
        }
        match self.shelf_for_id(thread_id) {
            ThreadShelf::Pinned => Some(RevealShelf::Pinned),
            ThreadShelf::Settled => Some(RevealShelf::Settled),
            ThreadShelf::Active => None,
        }
    }

    fn reveal_settled_thread(&mut self, thread_id: &str) {
        let settled_len = self.settled_threads().len();
        if let Some(pos) = self
            .settled_threads()
            .iter()
            .position(|t| t.id == thread_id)
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

    pub fn settle_thread(&mut self, thread_id: &str) {
        self.move_thread_to_shelf(thread_id, ThreadShelf::Settled);
    }

    pub fn unsettle_thread(&mut self, thread_id: &str) {
        if self.archived_thread_ids.contains(thread_id) {
            return;
        }
        self.move_thread_to_shelf(thread_id, ThreadShelf::Active);
    }

    pub fn pin_thread(&mut self, thread_id: &str) {
        if self.archived_thread_ids.contains(thread_id) {
            return;
        }
        self.move_thread_to_shelf(thread_id, ThreadShelf::Pinned);
    }

    pub fn unpin_thread(&mut self, thread_id: &str) {
        if self.archived_thread_ids.contains(thread_id) {
            return;
        }
        self.move_thread_to_shelf(thread_id, ThreadShelf::Active);
    }

    pub fn archive_thread(&mut self, thread_id: &str) {
        self.remove_from_shelf_orders(thread_id);
        self.archived_thread_ids.insert(thread_id.to_string());
        if !self.archived_order.iter().any(|id| id == thread_id) {
            self.archived_order.push(thread_id.to_string());
        }
        self.selected_thread_ids.remove(thread_id);
        if self.renaming_thread_id.as_deref() == Some(thread_id) {
            self.renaming_thread_id = None;
        }
        if self.active_thread_id == thread_id {
            self.active_thread_id = DEMO_DRAFT.id.to_string();
        }
    }

    pub fn unarchive_thread(&mut self, thread_id: &str) {
        self.archived_thread_ids.remove(thread_id);
        self.archived_order.retain(|id| id != thread_id);
        if !self.active_order.iter().any(|id| id == thread_id) {
            self.active_order.push(thread_id.to_string());
        }
    }

    fn move_thread_to_shelf(&mut self, thread_id: &str, shelf: ThreadShelf) {
        self.remove_from_shelf_orders(thread_id);
        self.thread_shelf_overrides
            .insert(thread_id.to_string(), shelf);
        self.append_to_shelf_order(thread_id, shelf);
    }

    fn threads_in_order(&self, order: &[String], shelf: ThreadShelf) -> Vec<&DemoThread> {
        order
            .iter()
            .filter_map(|id| DEMO_THREADS.iter().find(|t| t.id == id))
            .filter(|t| !self.is_archived(t))
            .filter(|t| self.effective_shelf(t) == shelf)
            .filter(|t| self.matches_scope(t))
            .collect()
    }

    fn shelf_for_id(&self, thread_id: &str) -> ThreadShelf {
        if thread_id == DEMO_DRAFT.id {
            return ThreadShelf::Active;
        }
        DEMO_THREADS
            .iter()
            .find(|t| t.id == thread_id)
            .map(|t| self.effective_shelf(t))
            .unwrap_or(ThreadShelf::Active)
    }

    fn shelf_order_slice(&self, thread_id: &str) -> &[String] {
        if self.archived_thread_ids.contains(thread_id) {
            return &self.archived_order;
        }
        match self.shelf_for_id(thread_id) {
            ThreadShelf::Pinned => &self.pinned_order,
            ThreadShelf::Active => &self.active_order,
            ThreadShelf::Settled => &self.settled_order,
        }
    }

    fn shelf_order_mut(&mut self, shelf: ThreadShelf) -> &mut Vec<String> {
        match shelf {
            ThreadShelf::Pinned => &mut self.pinned_order,
            ThreadShelf::Active => &mut self.active_order,
            ThreadShelf::Settled => &mut self.settled_order,
        }
    }

    fn remove_from_shelf_orders(&mut self, thread_id: &str) {
        self.pinned_order.retain(|id| id != thread_id);
        self.active_order.retain(|id| id != thread_id);
        self.settled_order.retain(|id| id != thread_id);
        self.archived_order.retain(|id| id != thread_id);
    }

    fn append_to_shelf_order(&mut self, thread_id: &str, shelf: ThreadShelf) {
        let order = self.shelf_order_mut(shelf);
        if !order.iter().any(|id| id == thread_id) {
            order.push(thread_id.to_string());
        }
    }

    fn matches_scope(&self, thread: &DemoThread) -> bool {
        match &self.project_scope {
            None => true,
            Some(key) => thread.project_key == key,
        }
    }
}

pub fn demo_draft() -> &'static DemoDraft {
    &DEMO_DRAFT
}

fn shelf_order_from_demo(shelf: ThreadShelf) -> Vec<String> {
    DEMO_THREADS
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
    fn pin_moves_thread_to_pinned_shelf() {
        let mut view = SidebarViewModel::new("active-1");
        view.pin_thread("active-2");
        assert!(view.pinned_threads().iter().any(|t| t.id == "active-2"));
        assert!(!view.active_threads().iter().any(|t| t.id == "active-2"));
    }

    #[test]
    fn unpin_moves_thread_back_to_active_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.unpin_thread("pinned-1");
        assert!(!view.pinned_threads().iter().any(|t| t.id == "pinned-1"));
        assert!(view.active_threads().iter().any(|t| t.id == "pinned-1"));
    }

    #[test]
    fn settle_moves_thread_to_settled_shelf() {
        let mut view = SidebarViewModel::new("active-1");
        view.settle_thread("active-2");
        assert!(view.settled_threads().iter().any(|t| t.id == "active-2"));
        assert!(!view.active_threads().iter().any(|t| t.id == "active-2"));
    }

    #[test]
    fn unsettle_moves_thread_back_to_active_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.unsettle_thread("settled-1");
        assert!(!view.settled_threads().iter().any(|t| t.id == "settled-1"));
        assert!(view.active_threads().iter().any(|t| t.id == "settled-1"));
    }

    #[test]
    fn settled_threads_always_recede_unless_active_or_selected() {
        let view = SidebarViewModel::new("active-1");
        let failed = DEMO_THREADS.iter().find(|t| t.id == "settled-5").unwrap();
        let woke = DEMO_THREADS.iter().find(|t| t.id == "settled-6").unwrap();
        assert!(view.should_recede(failed));
        assert!(view.should_recede(woke));
    }

    #[test]
    fn can_reorder_threads_requires_matching_shelf() {
        let view = SidebarViewModel::new("active-1");
        assert!(view.can_reorder_threads("active-1", "active-2"));
        assert!(!view.can_reorder_threads("active-1", "pinned-1"));
        assert!(view.can_reorder_threads("settled-1", "settled-2"));
        assert!(!view.can_reorder_threads("settled-1", "active-1"));
    }

    #[test]
    fn reorder_settled_moves_thread_within_settled_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.reorder_thread("settled-1", "settled-2", true);
        let order: Vec<_> = view.settled_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order[0], "settled-2");
        assert_eq!(order[1], "settled-1");
    }

    #[test]
    fn reorder_pinned_moves_thread_relative_to_target() {
        let mut view = SidebarViewModel::new("active-1");
        view.pinned_order = vec!["pinned-1".into(), "pinned-2".into()];
        view.reorder_thread("pinned-1", "pinned-2", true);
        let order: Vec<_> = view.pinned_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order, ["pinned-2", "pinned-1"]);
    }

    #[test]
    fn reorder_active_moves_thread_within_active_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.discard_draft();
        view.reorder_thread("active-1", "active-2", true);
        let order: Vec<_> = view.active_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order[0], "active-2");
        assert_eq!(order[1], "active-1");
    }

    #[test]
    fn move_thread_stays_within_settled_shelf_bounds() {
        let mut view = SidebarViewModel::new("active-1");
        view.settled_order = vec!["settled-1".into(), "settled-2".into()];
        assert!(!view.can_move_thread("settled-1", -1));
        assert!(view.can_move_thread("settled-1", 1));
        view.move_thread("settled-1", -1);
        let order: Vec<_> = view.settled_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order, ["settled-1", "settled-2"]);
        view.move_thread("settled-2", 1);
        let order: Vec<_> = view.settled_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order, ["settled-1", "settled-2"]);
        view.move_thread("settled-1", 1);
        let order: Vec<_> = view.settled_order.iter().map(|id| id.as_str()).collect();
        assert_eq!(order, ["settled-2", "settled-1"]);
    }

    #[test]
    fn archive_moves_thread_to_archived_shelf() {
        let mut view = SidebarViewModel::new("active-1");
        view.archive_thread("active-2");
        assert!(view.archived_threads().iter().any(|t| t.id == "active-2"));
        assert!(!view.active_threads().iter().any(|t| t.id == "active-2"));
    }

    #[test]
    fn unarchive_restores_thread_to_active_list() {
        let mut view = SidebarViewModel::new("active-1");
        view.archive_thread("active-2");
        view.unarchive_thread("active-2");
        assert!(view.archived_threads().is_empty());
        assert!(view.active_threads().iter().any(|t| t.id == "active-2"));
    }

    #[test]
    fn archive_removes_thread_from_lists() {
        let mut view = SidebarViewModel::new("active-1");
        view.archive_thread("active-2");
        assert!(!view.active_threads().iter().any(|t| t.id == "active-2"));
        assert!(view.visible_threads().iter().all(|t| t.id != "active-2"));
    }

    #[test]
    fn activate_from_search_clears_query_and_prepares_settled_reveal() {
        let mut view = SidebarViewModel::new("active-1");
        view.search_query = "theme".to_string();
        view.settled_expanded = false;
        view.settled_visible_limit = 1;
        view.activate_from_search("settled-1");
        assert_eq!(view.active_thread_id, "settled-1");
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
        view.commit_rename("active-1", "Renamed thread".to_string());
        assert_eq!(
            view.display_title(DEMO_THREADS.iter().find(|t| t.id == "active-1").unwrap()),
            "Renamed thread"
        );
        assert!(view.renaming_thread_id.is_none());
    }

    #[test]
    fn cancel_rename_clears_state() {
        let mut view = SidebarViewModel::new("active-1");
        view.begin_rename("active-1");
        view.cancel_rename();
        assert!(view.renaming_thread_id.is_none());
    }
}
