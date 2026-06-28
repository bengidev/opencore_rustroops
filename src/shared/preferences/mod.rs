//! Persisted user settings as a single atomic blob.
//!
//! Uses the **Strategy** pattern: [`PreferencesStore`] abstracts file and in-memory backends.

use crate::shared::theme::ThemeMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Single preferences document (theme + onboarding completion).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppPreferences {
    pub theme_mode: ThemeMode,
    pub onboarding_completed: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::default(),
            onboarding_completed: false,
        }
    }
}

/// Errors from loading or saving preferences.
#[derive(Debug, Error)]
pub enum PreferencesError {
    #[error("failed to read preferences: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse preferences: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Persistence backend for [`AppPreferences`].
pub trait PreferencesStore {
    fn load(&self) -> Result<AppPreferences, PreferencesError>;
    fn save(&self, preferences: &AppPreferences) -> Result<(), PreferencesError>;
}

/// Writes preferences to `{data_dir}/opencore_rustroops/preferences.json`.
pub struct FilePreferencesStore {
    path: PathBuf,
}

impl FilePreferencesStore {
    /// Uses the platform application data directory.
    pub fn default_path() -> Result<PathBuf, PreferencesError> {
        let base = directories::ProjectDirs::from("com", "opencore", "opencore_rustroops")
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "could not resolve application data directory",
                )
            })?
            .data_dir()
            .to_path_buf();
        Ok(base.join("preferences.json"))
    }

    /// Opens the store at the platform default path.
    pub fn open() -> Result<Self, PreferencesError> {
        Ok(Self {
            path: Self::default_path()?,
        })
    }

    /// Opens the store at an explicit path (tests and tooling).
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn ensure_parent(&self) -> Result<(), PreferencesError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

impl PreferencesStore for FilePreferencesStore {
    fn load(&self) -> Result<AppPreferences, PreferencesError> {
        if !self.path.exists() {
            return Ok(AppPreferences::default());
        }
        let contents = fs::read_to_string(&self.path)?;
        let preferences = serde_json::from_str(&contents)?;
        Ok(preferences)
    }

    fn save(&self, preferences: &AppPreferences) -> Result<(), PreferencesError> {
        self.ensure_parent()?;
        let contents = serde_json::to_string_pretty(preferences)?;
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, &contents)?;
        fs::rename(&temp_path, &self.path)?;
        Ok(())
    }
}

/// In-memory store for unit tests (no filesystem).
#[derive(Debug, Default)]
pub struct InMemoryPreferencesStore {
    preferences: std::sync::Mutex<Option<AppPreferences>>,
}

impl InMemoryPreferencesStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PreferencesStore for InMemoryPreferencesStore {
    fn load(&self) -> Result<AppPreferences, PreferencesError> {
        let guard = self
            .preferences
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "preferences lock poisoned"))?;
        Ok(guard.clone().unwrap_or_default())
    }

    fn save(&self, preferences: &AppPreferences) -> Result<(), PreferencesError> {
        let mut guard = self
            .preferences
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "preferences lock poisoned"))?;
        *guard = Some(preferences.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn app_preferences_default_serializes_to_prd_schema() {
        let json = serde_json::to_string(&AppPreferences::default()).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(value["theme_mode"], "dark");
        assert_eq!(value["onboarding_completed"], false);
    }

    #[test]
    fn app_preferences_default_matches_schema() {
        let prefs = AppPreferences::default();
        assert_eq!(prefs.theme_mode, ThemeMode::Dark);
        assert!(!prefs.onboarding_completed);
    }

    #[test]
    fn app_preferences_round_trips_through_json() {
        let prefs = AppPreferences {
            theme_mode: ThemeMode::Light,
            onboarding_completed: true,
        };
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: AppPreferences = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, prefs);
    }

    #[test]
    fn in_memory_store_round_trips_preferences() {
        let store = InMemoryPreferencesStore::new();
        let prefs = AppPreferences {
            theme_mode: ThemeMode::Dark,
            onboarding_completed: false,
        };
        store.save(&prefs).expect("save");
        let loaded = store.load().expect("load");
        assert_eq!(loaded, prefs);
    }

    #[test]
    fn file_store_round_trips_preferences_in_temp_dir() {
        let dir = tempfile_dir();
        let path = dir.join("preferences.json");
        let store = FilePreferencesStore::at(&path);
        let prefs = AppPreferences {
            theme_mode: ThemeMode::Light,
            onboarding_completed: true,
        };
        store.save(&prefs).expect("save");
        let loaded = store.load().expect("load");
        assert_eq!(loaded, prefs);
        cleanup_dir(&dir);
    }

    #[test]
    fn file_store_returns_defaults_when_missing() {
        let dir = tempfile_dir();
        let path = dir.join("missing.json");
        let store = FilePreferencesStore::at(&path);
        let loaded = store.load().expect("load");
        assert_eq!(loaded, AppPreferences::default());
        cleanup_dir(&dir);
    }

    fn tempfile_dir() -> PathBuf {
        let base = std::env::temp_dir();
        let name = format!(
            "opencore_rustroops_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        base.join(name)
    }

    fn cleanup_dir(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }
}
