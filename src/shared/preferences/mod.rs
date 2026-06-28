//! Persisted user settings as a single atomic blob.
//!
//! [`PreferencesStore`] abstracts file and in-memory backends.

use crate::shared::theme::ThemeMode;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Single preferences document (theme + onboarding completion).
///
/// Unknown JSON fields are ignored on load and dropped on the next save.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPreferences {
    pub theme_mode: ThemeMode,
    pub onboarding_completed: bool,
}

/// Errors from loading or saving preferences.
#[derive(Debug, Error)]
pub enum PreferencesError {
    #[error("failed to read preferences: {0}")]
    Read(#[from] io::Error),
    #[error("failed to write preferences: {0}")]
    Write(io::Error),
    #[error("failed to parse preferences: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Persistence backend for [`AppPreferences`].
pub trait PreferencesStore {
    fn load(&self) -> Result<AppPreferences, PreferencesError>;
    fn save(&self, preferences: &AppPreferences) -> Result<(), PreferencesError>;
}

/// Writes preferences to the platform application data directory.
///
/// On macOS: `~/Library/Application Support/com.opencore.opencore_rustroops/preferences.json`
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
            fs::create_dir_all(parent).map_err(PreferencesError::Write)?;
        }
        Ok(())
    }

    fn backup_corrupt_file(&self) -> Result<(), PreferencesError> {
        if !self.path.exists() {
            return Ok(());
        }
        let backup_path = self.path.with_extension("corrupt");
        let _ = fs::remove_file(&backup_path);
        fs::rename(&self.path, &backup_path).map_err(PreferencesError::Write)?;
        Ok(())
    }
}

impl PreferencesStore for FilePreferencesStore {
    fn load(&self) -> Result<AppPreferences, PreferencesError> {
        if !self.path.exists() {
            return Ok(AppPreferences::default());
        }
        let contents = fs::read_to_string(&self.path)?;
        match serde_json::from_str(&contents) {
            Ok(preferences) => Ok(preferences),
            Err(parse_error) => {
                eprintln!(
                    "opencore_rustroops: corrupt preferences reset to defaults ({parse_error})"
                );
                self.backup_corrupt_file()?;
                Ok(AppPreferences::default())
            }
        }
    }

    fn save(&self, preferences: &AppPreferences) -> Result<(), PreferencesError> {
        self.ensure_parent()?;
        let contents = serde_json::to_string_pretty(preferences)?;
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, &contents).map_err(PreferencesError::Write)?;
        fs::rename(&temp_path, &self.path).map_err(PreferencesError::Write)?;
        Ok(())
    }
}

/// In-memory store for unit tests (no filesystem).
#[derive(Debug, Default)]
pub struct InMemoryPreferencesStore {
    preferences: RefCell<Option<AppPreferences>>,
}

impl InMemoryPreferencesStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PreferencesStore for InMemoryPreferencesStore {
    fn load(&self) -> Result<AppPreferences, PreferencesError> {
        Ok(self.preferences.borrow().clone().unwrap_or_default())
    }

    fn save(&self, preferences: &AppPreferences) -> Result<(), PreferencesError> {
        *self.preferences.borrow_mut() = Some(preferences.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
    fn app_preferences_deserializes_with_missing_fields() {
        let restored: AppPreferences =
            serde_json::from_str(r#"{"onboarding_completed":true}"#).expect("deserialize");
        assert_eq!(restored.theme_mode, ThemeMode::Dark);
        assert!(restored.onboarding_completed);
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
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("preferences.json");
        let store = FilePreferencesStore::at(&path);
        let prefs = AppPreferences {
            theme_mode: ThemeMode::Light,
            onboarding_completed: true,
        };
        store.save(&prefs).expect("save");
        let loaded = store.load().expect("load");
        assert_eq!(loaded, prefs);
    }

    #[test]
    fn file_store_returns_defaults_when_missing() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("missing.json");
        let store = FilePreferencesStore::at(&path);
        let loaded = store.load().expect("load");
        assert_eq!(loaded, AppPreferences::default());
    }

    #[test]
    fn file_store_resets_to_defaults_on_corrupt_json() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("preferences.json");
        fs::write(&path, "{not valid json").expect("write corrupt file");
        let store = FilePreferencesStore::at(&path);
        let loaded = store.load().expect("load");
        assert_eq!(loaded, AppPreferences::default());
        assert!(!path.exists());
        assert!(path.with_extension("corrupt").exists());
    }
}
