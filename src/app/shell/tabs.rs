use crate::shared::preferences::shell_chrome::{ShellChrome, ShellTabRecord};

#[derive(Debug, Clone, PartialEq)]
pub struct TabModel {
    tabs: Vec<ShellTabRecord>,
    active_id: String,
}

impl TabModel {
    pub fn from_chrome(chrome: &ShellChrome) -> Self {
        Self {
            tabs: chrome.tabs.clone(),
            active_id: chrome.active_tab_id.clone(),
        }
    }

    pub fn select(&mut self, id: &str) {
        if self.tabs.iter().any(|tab| tab.id == id) {
            self.active_id = id.to_owned();
        }
    }

    pub fn rename(&mut self, id: &str, title: impl AsRef<str>) {
        let trimmed = title.as_ref().trim();
        if trimmed.is_empty() {
            return;
        }
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
            return;
        };
        tab.title = trimmed.to_owned();
    }

    pub fn close(&mut self, id: &str) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let was_active = self.active_id == id;
        self.tabs.remove(index);

        if self.tabs.is_empty() {
            self.active_id.clear();
            return;
        }

        if was_active {
            let neighbor_index = index.min(self.tabs.len() - 1);
            self.active_id = self.tabs[neighbor_index].id.clone();
        }
    }

    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }

        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
    }

    pub fn add_stub(&mut self) -> String {
        let mut number = self.tabs.len() + 1;
        let id = loop {
            let candidate = format!("tab-{number}");
            if !self.tabs.iter().any(|tab| tab.id == candidate) {
                break candidate;
            }
            number += 1;
        };
        self.tabs.push(ShellTabRecord {
            id: id.clone(),
            title: "New Tab".into(),
        });
        self.active_id = id.clone();
        id
    }

    pub fn to_chrome_tabs(&self) -> (Vec<ShellTabRecord>, String) {
        (self.tabs.clone(), self.active_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_two() -> TabModel {
        TabModel {
            tabs: vec![
                ShellTabRecord {
                    id: "a".into(),
                    title: "A".into(),
                },
                ShellTabRecord {
                    id: "b".into(),
                    title: "B".into(),
                },
            ],
            active_id: "a".into(),
        }
    }

    #[test]
    fn from_chrome_copies_tabs_and_active_id() {
        let chrome = ShellChrome {
            tabs: vec![
                ShellTabRecord {
                    id: "x".into(),
                    title: "X".into(),
                },
                ShellTabRecord {
                    id: "y".into(),
                    title: "Y".into(),
                },
            ],
            active_tab_id: "y".into(),
            ..ShellChrome::default()
        };

        let model = TabModel::from_chrome(&chrome);

        assert_eq!(model.tabs, chrome.tabs);
        assert_eq!(model.active_id, chrome.active_tab_id);
    }

    #[test]
    fn select_changes_active_tab_for_known_id() {
        let mut model = model_two();

        model.select("b");

        assert_eq!(model.active_id, "b");
    }

    #[test]
    fn select_unknown_id_is_noop() {
        let mut model = model_two();

        model.select("missing");

        assert_eq!(model.active_id, "a");
    }

    #[test]
    fn close_active_selects_neighbor() {
        let mut m = model_two();
        m.close("a");
        assert_eq!(m.active_id, "b");
        assert_eq!(m.tabs.len(), 1);
    }

    #[test]
    fn close_active_last_index_selects_previous_neighbor() {
        let mut m = model_two();
        m.select("b");

        m.close("b");

        assert_eq!(m.active_id, "a");
        assert_eq!(m.tabs.len(), 1);
    }

    #[test]
    fn close_inactive_tab_preserves_active_tab() {
        let mut m = model_two();

        m.close("b");

        assert_eq!(m.active_id, "a");
        assert_eq!(
            m.tabs,
            vec![ShellTabRecord {
                id: "a".into(),
                title: "A".into()
            }]
        );
    }

    #[test]
    fn close_unknown_tab_is_noop() {
        let mut m = model_two();

        m.close("missing");

        assert_eq!(m, model_two());
    }

    #[test]
    fn close_last_tab_leaves_strip_empty() {
        let mut m = model_two();
        m.close("a");
        m.close("b");
        assert!(m.tabs.is_empty());
        assert!(m.active_id.is_empty());
    }

    #[test]
    fn reorder_moves_tab() {
        let mut m = model_two();
        m.reorder(0, 1);
        assert_eq!(m.tabs[0].id, "b");
        assert_eq!(m.tabs[1].id, "a");
    }

    #[test]
    fn reorder_out_of_bounds_is_noop() {
        let mut m = model_two();
        m.reorder(0, 2);
        assert_eq!(m, model_two());
    }

    #[test]
    fn rename_updates_known_tab_title() {
        let mut m = model_two();
        m.rename("a", "Alpha");
        assert_eq!(m.tabs[0].title, "Alpha");
        assert_eq!(m.active_id, "a");
    }

    #[test]
    fn rename_trims_whitespace() {
        let mut m = model_two();
        m.rename("b", "  Beta  ");
        assert_eq!(m.tabs[1].title, "Beta");
    }

    #[test]
    fn rename_empty_or_whitespace_keeps_previous_title() {
        let mut m = model_two();
        m.rename("a", "   ");
        assert_eq!(m.tabs[0].title, "A");
        m.rename("a", "");
        assert_eq!(m.tabs[0].title, "A");
    }

    #[test]
    fn rename_unknown_id_is_noop() {
        let mut m = model_two();
        m.rename("missing", "Nope");
        assert_eq!(m, model_two());
    }

    #[test]
    fn add_stub_appends_and_activates() {
        let mut m = model_two();
        let id = m.add_stub();
        assert_eq!(m.active_id, id);
        assert_eq!(m.tabs.len(), 3);
        assert_eq!(m.tabs.last().map(|tab| tab.id.as_str()), Some(id.as_str()));
    }

    #[test]
    fn to_chrome_tabs_copies_tabs_and_active_id() {
        let model = model_two();

        let (tabs, active_id) = model.to_chrome_tabs();

        assert_eq!(tabs, model.tabs);
        assert_eq!(active_id, model.active_id);
    }
}
