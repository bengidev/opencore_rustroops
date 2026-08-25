//! Shared theme cell for shell panels that render outside `ShellWorkspace::render`.

use std::cell::Cell;
use std::rc::Rc;

use crate::shared::theme::{OpenCoreTheme, ThemeMode};

#[derive(Clone)]
pub struct WorkspaceTheme(Rc<Cell<OpenCoreTheme>>);

thread_local! {
    static WORKSPACE_THEME: WorkspaceTheme = WorkspaceTheme::new(ThemeMode::default());
}

impl WorkspaceTheme {
    pub fn new(mode: ThemeMode) -> Self {
        Self(Rc::new(Cell::new(OpenCoreTheme::resolve(mode))))
    }

    pub fn get(&self) -> OpenCoreTheme {
        self.0.get()
    }

    pub fn set(&self, theme: OpenCoreTheme) {
        self.0.set(theme);
    }
}

impl Default for WorkspaceTheme {
    fn default() -> Self {
        Self::new(ThemeMode::default())
    }
}

/// Installs the workspace theme cell used by dock panel factories.
pub fn install_workspace_theme(theme: WorkspaceTheme) -> WorkspaceTheme {
    WORKSPACE_THEME.with(|existing| {
        existing.set(theme.get());
        existing.clone()
    })
}

/// Returns the installed workspace theme for the current thread.
pub fn workspace_theme() -> WorkspaceTheme {
    WORKSPACE_THEME.with(|theme| theme.clone())
}
