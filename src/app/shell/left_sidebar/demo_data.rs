//! Static demo projections for left-sidebar interface scaffolding.

#![allow(dead_code)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadShelf {
    Pinned,
    Active,
    Snoozed,
    Settled,
}

#[derive(Clone, Debug)]
pub struct DemoProject {
    pub key: &'static str,
    pub display_name: &'static str,
}

#[derive(Clone, Debug)]
pub struct DemoThread {
    pub id: &'static str,
    pub title: &'static str,
    pub project_key: &'static str,
    pub project_title: &'static str,
    pub branch: Option<&'static str>,
    pub shelf: ThreadShelf,
    pub time_label: &'static str,
    pub status_label: Option<&'static str>,
    pub pr_number: Option<u32>,
    pub diff_insertions: Option<u32>,
    pub diff_deletions: Option<u32>,
    pub pinned: bool,
    pub is_active: bool,
}

pub const DEMO_PROJECTS: [DemoProject; 2] = [
    DemoProject {
        key: "opencore",
        display_name: "opencore_rustroops",
    },
    DemoProject {
        key: "t3code",
        display_name: "t3code",
    },
];

pub const DEMO_THREADS: [DemoThread; 7] = [
    DemoThread {
        id: "pinned-1",
        title: "Fix dock layout persistence",
        project_key: "opencore",
        project_title: "opencore_rustroops",
        branch: Some("feat/shell-dock"),
        shelf: ThreadShelf::Pinned,
        time_label: "2h",
        status_label: Some("Working"),
        pr_number: Some(42),
        diff_insertions: Some(128),
        diff_deletions: Some(24),
        pinned: true,
        is_active: false,
    },
    DemoThread {
        id: "active-1",
        title: "Implement left sidebar UI",
        project_key: "opencore",
        project_title: "opencore_rustroops",
        branch: Some("feat/left-sidebar"),
        shelf: ThreadShelf::Active,
        time_label: "now",
        status_label: None,
        pr_number: None,
        diff_insertions: None,
        diff_deletions: None,
        pinned: false,
        is_active: true,
    },
    DemoThread {
        id: "active-2",
        title: "Theme transition polish",
        project_key: "opencore",
        project_title: "opencore_rustroops",
        branch: Some("main"),
        shelf: ThreadShelf::Active,
        time_label: "18m",
        status_label: Some("Working"),
        pr_number: None,
        diff_insertions: Some(12),
        diff_deletions: Some(3),
        pinned: false,
        is_active: false,
    },
    DemoThread {
        id: "snoozed-1",
        title: "Review gpui-component dock APIs",
        project_key: "t3code",
        project_title: "t3code",
        branch: Some("research/dock"),
        shelf: ThreadShelf::Snoozed,
        time_label: "tomorrow 9a",
        status_label: None,
        pr_number: None,
        diff_insertions: None,
        diff_deletions: None,
        pinned: false,
        is_active: false,
    },
    DemoThread {
        id: "settled-1",
        title: "Welcome view port",
        project_key: "opencore",
        project_title: "opencore_rustroops",
        branch: Some("main"),
        shelf: ThreadShelf::Settled,
        time_label: "1d",
        status_label: None,
        pr_number: Some(38),
        diff_insertions: None,
        diff_deletions: None,
        pinned: false,
        is_active: false,
    },
    DemoThread {
        id: "settled-2",
        title: "Shell workspace title bar",
        project_key: "opencore",
        project_title: "opencore_rustroops",
        branch: Some("feat/shell"),
        shelf: ThreadShelf::Settled,
        time_label: "3d",
        status_label: None,
        pr_number: None,
        diff_insertions: None,
        diff_deletions: None,
        pinned: false,
        is_active: false,
    },
    DemoThread {
        id: "settled-3",
        title: "Preferences persistence",
        project_key: "opencore",
        project_title: "opencore_rustroops",
        branch: None,
        shelf: ThreadShelf::Settled,
        time_label: "1w",
        status_label: None,
        pr_number: None,
        diff_insertions: None,
        diff_deletions: None,
        pinned: false,
        is_active: false,
    },
];

pub const SCOPED_PROJECT_LABEL: &str = "All projects";
