//! Sidebar view model: search, scope, selection, and shelf partitioning.

use std::collections::HashSet;

use super::demo_data::{
    ALL_PROJECTS_LABEL, DEMO_DRAFT, DEMO_THREADS, DemoDraft, DemoThread, ThreadShelf,
    ThreadStatus,
};

pub const SETTLED_PAGE_INITIAL: usize = 10;
pub const SETTLED_PAGE_SIZE: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FooterMode {
    #[default]
    Utilities,
    Back,
}

#[derive(Clone, Debug, Default)]
pub struct SidebarViewModel {
    pub search_query: String,
    pub project_scope: Option<String>,
    pub snoozed_expanded: bool,
    pub settled_expanded: bool,
    pub settled_visible_limit: usize,
    pub active_thread_id: String,
    pub selected_thread_ids: HashSet<String>,
    pub pinned_order: Vec<String>,
    pub hovered_thread_id: Option<String>,
    pub footer_mode: FooterMode,
    pub show_update_pill: bool,
    pub draft_visible: bool,
}

impl SidebarViewModel {
    pub fn new(active_thread_id: impl Into<String>) -> Self {
        let active_thread_id = active_thread_id.into();
        let pinned_order = DEMO_THREADS
            .iter()
            .filter(|t| t.shelf == ThreadShelf::Pinned)
            .map(|t| t.id.to_string())
            .collect();

        Self {
            search_query: String::new(),
            project_scope: None,
            snoozed_expanded: true,
            settled_expanded: true,
            settled_visible_limit: SETTLED_PAGE_INITIAL,
            active_thread_id,
            selected_thread_ids: HashSet::new(),
            pinned_order,
            hovered_thread_id: None,
            footer_mode: FooterMode::Utilities,
            show_update_pill: true,
            draft_visible: true,
        }
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
            .filter(|t| self.matches_scope(t))
            .filter(|t| query.is_empty() || t.title.to_ascii_lowercase().contains(&query))
            .collect()
    }

    pub fn pinned_threads(&self) -> Vec<&DemoThread> {
        self.pinned_order
            .iter()
            .filter_map(|id| DEMO_THREADS.iter().find(|t| t.id == id))
            .filter(|t| self.matches_scope(t))
            .filter(|_| !self.is_searching())
            .collect()
    }

    pub fn active_threads(&self) -> Vec<&DemoThread> {
        if self.is_searching() {
            return Vec::new();
        }
        DEMO_THREADS
            .iter()
            .filter(|t| t.shelf == ThreadShelf::Active)
            .filter(|t| self.matches_scope(t))
            .collect()
    }

    pub fn snoozed_threads(&self) -> Vec<&DemoThread> {
        if self.is_searching() {
            return Vec::new();
        }
        DEMO_THREADS
            .iter()
            .filter(|t| t.shelf == ThreadShelf::Snoozed)
            .filter(|t| self.matches_scope(t))
            .collect()
    }

    pub fn settled_threads(&self) -> Vec<&DemoThread> {
        if self.is_searching() {
            return Vec::new();
        }
        DEMO_THREADS
            .iter()
            .filter(|t| t.shelf == ThreadShelf::Settled)
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

    pub fn snoozed_label(&self) -> String {
        let count = self.snoozed_threads().len();
        if self.snoozed_expanded {
            "Snoozed".to_string()
        } else {
            format!("Snoozed ({count})")
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

    pub fn is_active(&self, thread: &DemoThread) -> bool {
        thread.id == self.active_thread_id
    }

    pub fn is_selected(&self, thread: &DemoThread) -> bool {
        self.selected_thread_ids.contains(thread.id)
    }

    pub fn is_hovered(&self, thread: &DemoThread) -> bool {
        self.hovered_thread_id.as_deref() == Some(thread.id)
    }

    pub fn should_recede(&self, thread: &DemoThread) -> bool {
        if self.is_active(thread) || self.is_selected(thread) {
            return false;
        }
        if thread.is_unread || thread.is_woke {
            return false;
        }
        matches!(
            thread.status,
            ThreadStatus::Ready | ThreadStatus::Working | ThreadStatus::Monitoring
                | ThreadStatus::Approval | ThreadStatus::Input
        )
    }

    pub fn ordered_visible_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        if self.draft_visible {
            ids.push(DEMO_DRAFT.id.to_string());
        }
        ids.extend(self.pinned_threads().iter().map(|t| t.id.to_string()));
        ids.extend(self.active_threads().iter().map(|t| t.id.to_string()));
        if self.snoozed_expanded {
            ids.extend(self.snoozed_threads().iter().map(|t| t.id.to_string()));
        }
        if self.settled_expanded {
            ids.extend(self.settled_visible().iter().map(|t| t.id.to_string()));
        }
        ids
    }

    pub fn activate_thread(&mut self, thread_id: &str) {
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
                    let (start, end) = if from <= to {
                        (from, to)
                    } else {
                        (to, from)
                    };
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

    pub fn move_pinned(&mut self, thread_id: &str, delta: isize) {
        let pos = self.pinned_order.iter().position(|id| id == thread_id);
        if let Some(pos) = pos {
            let new_pos = (pos as isize + delta).clamp(0, self.pinned_order.len() as isize - 1);
            if new_pos as usize != pos {
                let id = self.pinned_order.remove(pos);
                self.pinned_order.insert(new_pos as usize, id);
            }
        }
    }

    pub fn discard_draft(&mut self) {
        self.draft_visible = false;
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
