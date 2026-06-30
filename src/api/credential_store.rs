//! Local persistence for saved provider API keys.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

/// Errors from credential store operations.
#[derive(Debug, Error)]
pub enum CredentialStoreError {
    #[error("failed to read credentials: {0}")]
    Read(#[from] io::Error),
    #[error("failed to write credentials: {0}")]
    Write(io::Error),
    #[error("failed to parse credentials: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Persistence for a user-supplied OpenRouter API key.
pub trait CredentialStore: Send + Sync {
    fn saved_api_key(&self) -> Option<String>;
    fn save_api_key(&self, key: &str) -> Result<(), CredentialStoreError>;
    fn clear_api_key(&self) -> Result<(), CredentialStoreError>;
}

/// File-backed credential store under the application data directory.
pub struct FileCredentialStore {
    path: PathBuf,
}

impl FileCredentialStore {
    pub fn default_path() -> Result<PathBuf, CredentialStoreError> {
        let base = directories::ProjectDirs::from("com", "opencore", "opencore_rustroops")
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "could not resolve application data directory",
                )
            })?
            .data_dir()
            .to_path_buf();
        Ok(base.join("credentials.json"))
    }

    pub fn open() -> Result<Self, CredentialStoreError> {
        Ok(Self {
            path: Self::default_path()?,
        })
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn ensure_parent(&self) -> Result<(), CredentialStoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(CredentialStoreError::Write)?;
        }
        Ok(())
    }

    fn legacy_path(&self) -> Option<PathBuf> {
        self.path
            .parent()
            .map(|parent| parent.join("openrouter_credentials.json"))
    }

    fn read_stored_from(&self, path: &Path) -> Option<StoredCredentials> {
        if !path.exists() {
            return None;
        }
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                eprintln!(
                    "opencore: failed to read credentials from {}: {error}",
                    path.display()
                );
                return None;
            }
        };
        match serde_json::from_str(&contents) {
            Ok(stored) => Some(stored),
            Err(error) => {
                eprintln!(
                    "opencore: corrupt credentials file at {} ({error}); treating as missing",
                    path.display()
                );
                None
            }
        }
    }

    fn remove_legacy_file(&self) -> Result<(), CredentialStoreError> {
        if let Some(legacy_path) = self.legacy_path()
            && legacy_path.exists()
        {
            fs::remove_file(&legacy_path).map_err(CredentialStoreError::Write)?;
        }
        Ok(())
    }

    fn load_stored(&self) -> Option<StoredCredentials> {
        if let Some(stored) = self.read_stored_from(&self.path)
            && stored.has_usable_api_key()
        {
            return Some(stored);
        }

        let legacy_path = self.legacy_path()?;
        let stored = self.read_stored_from(&legacy_path)?;
        if !stored.has_usable_api_key() {
            return None;
        }
        if let Err(error) = self.write_stored(&stored) {
            eprintln!("opencore: failed to migrate legacy credentials: {error}");
            return Some(stored);
        }
        Some(stored)
    }

    fn write_stored(&self, stored: &StoredCredentials) -> Result<(), CredentialStoreError> {
        self.ensure_parent()?;
        let contents = serde_json::to_string_pretty(stored)?;
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, contents).map_err(CredentialStoreError::Write)?;
        restrict_file_permissions(&temp_path)?;
        fs::rename(&temp_path, &self.path).map_err(CredentialStoreError::Write)?;
        restrict_file_permissions(&self.path)?;
        self.remove_legacy_file()?;
        Ok(())
    }
}

#[cfg(unix)]
fn restrict_file_permissions(path: &Path) -> Result<(), CredentialStoreError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(CredentialStoreError::Write)
}

#[cfg(not(unix))]
fn restrict_file_permissions(_path: &Path) -> Result<(), CredentialStoreError> {
    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct StoredCredentials {
    #[serde(default)]
    openrouter_api_key: Option<String>,
}

impl StoredCredentials {
    fn has_usable_api_key(&self) -> bool {
        self.openrouter_api_key
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    }
}

impl CredentialStore for FileCredentialStore {
    fn saved_api_key(&self) -> Option<String> {
        self.load_stored()
            .and_then(|stored| stored.openrouter_api_key)
            .filter(|value| !value.trim().is_empty())
    }

    fn save_api_key(&self, key: &str) -> Result<(), CredentialStoreError> {
        let stored = StoredCredentials {
            openrouter_api_key: Some(key.trim().to_string()),
        };
        self.write_stored(&stored)
    }

    fn clear_api_key(&self) -> Result<(), CredentialStoreError> {
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(CredentialStoreError::Write)?;
        }
        self.remove_legacy_file()?;
        Ok(())
    }
}

/// In-memory credential store for unit tests.
#[derive(Debug, Default)]
pub struct InMemoryCredentialStore {
    key: Mutex<Option<String>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn saved_api_key(&self) -> Option<String> {
        self.key.lock().expect("credential lock").clone()
    }

    fn save_api_key(&self, key: &str) -> Result<(), CredentialStoreError> {
        *self.key.lock().expect("credential lock") = Some(key.trim().to_string());
        Ok(())
    }

    fn clear_api_key(&self) -> Result<(), CredentialStoreError> {
        *self.key.lock().expect("credential lock") = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn file_store_round_trips_saved_api_key() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("credentials.json");
        let store = FileCredentialStore::at(&path);
        store.save_api_key("secret-key").expect("save");
        assert_eq!(store.saved_api_key().as_deref(), Some("secret-key"));
    }

    #[test]
    fn file_store_clear_removes_saved_key() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("credentials.json");
        let store = FileCredentialStore::at(&path);
        store.save_api_key("secret-key").expect("save");
        store.clear_api_key().expect("clear");
        assert_eq!(store.saved_api_key(), None);
    }

    #[cfg(unix)]
    #[test]
    fn file_store_writes_with_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("credentials.json");
        let store = FileCredentialStore::at(&path);
        store.save_api_key("secret-key").expect("save");
        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn file_store_migrates_legacy_credentials_file() {
        let dir = TempDir::new().expect("temp dir");
        let legacy_path = dir.path().join("openrouter_credentials.json");
        let path = dir.path().join("credentials.json");
        fs::write(&legacy_path, r#"{"openrouter_api_key":"legacy-key"}"#).expect("write legacy");

        let store = FileCredentialStore::at(&path);
        assert_eq!(store.saved_api_key().as_deref(), Some("legacy-key"));
        assert!(path.exists());
        assert!(!legacy_path.exists());
    }

    #[test]
    fn file_store_falls_back_when_new_file_has_empty_key() {
        let dir = TempDir::new().expect("temp dir");
        let legacy_path = dir.path().join("openrouter_credentials.json");
        let path = dir.path().join("credentials.json");
        fs::write(&path, r#"{"openrouter_api_key":""}"#).expect("write empty new");
        fs::write(&legacy_path, r#"{"openrouter_api_key":"legacy-key"}"#).expect("write legacy");

        let store = FileCredentialStore::at(&path);
        assert_eq!(store.saved_api_key().as_deref(), Some("legacy-key"));
        assert!(!legacy_path.exists());
    }

    #[test]
    fn file_store_save_removes_legacy_file() {
        let dir = TempDir::new().expect("temp dir");
        let legacy_path = dir.path().join("openrouter_credentials.json");
        let path = dir.path().join("credentials.json");
        fs::write(&legacy_path, r#"{"openrouter_api_key":"legacy-key"}"#).expect("write legacy");

        let store = FileCredentialStore::at(&path);
        store.save_api_key("new-key").expect("save");
        assert_eq!(store.saved_api_key().as_deref(), Some("new-key"));
        assert!(!legacy_path.exists());
    }

    #[test]
    fn file_store_clear_removes_legacy_only_file() {
        let dir = TempDir::new().expect("temp dir");
        let legacy_path = dir.path().join("openrouter_credentials.json");
        let path = dir.path().join("credentials.json");
        fs::write(&legacy_path, r#"{"openrouter_api_key":"legacy-key"}"#).expect("write legacy");

        let store = FileCredentialStore::at(&path);
        store.clear_api_key().expect("clear");
        assert_eq!(store.saved_api_key(), None);
        assert!(!legacy_path.exists());
        assert!(!path.exists());
    }
}
