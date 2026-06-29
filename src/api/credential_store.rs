//! Local persistence for saved provider API keys.

use std::fs;
use std::io;
use std::path::PathBuf;
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
        Ok(base.join("openrouter_credentials.json"))
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
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct StoredCredentials {
    #[serde(default)]
    openrouter_api_key: Option<String>,
}

impl CredentialStore for FileCredentialStore {
    fn saved_api_key(&self) -> Option<String> {
        if !self.path.exists() {
            return None;
        }
        let contents = fs::read_to_string(&self.path).ok()?;
        let stored: StoredCredentials = serde_json::from_str(&contents).ok()?;
        stored
            .openrouter_api_key
            .filter(|value| !value.trim().is_empty())
    }

    fn save_api_key(&self, key: &str) -> Result<(), CredentialStoreError> {
        self.ensure_parent()?;
        let stored = StoredCredentials {
            openrouter_api_key: Some(key.trim().to_string()),
        };
        let contents = serde_json::to_string_pretty(&stored)?;
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, contents).map_err(CredentialStoreError::Write)?;
        fs::rename(&temp_path, &self.path).map_err(CredentialStoreError::Write)?;
        Ok(())
    }

    fn clear_api_key(&self) -> Result<(), CredentialStoreError> {
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(CredentialStoreError::Write)?;
        }
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
        let path = dir.path().join("openrouter_credentials.json");
        let store = FileCredentialStore::at(&path);
        store.save_api_key("secret-key").expect("save");
        assert_eq!(store.saved_api_key().as_deref(), Some("secret-key"));
    }

    #[test]
    fn file_store_clear_removes_saved_key() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("openrouter_credentials.json");
        let store = FileCredentialStore::at(&path);
        store.save_api_key("secret-key").expect("save");
        store.clear_api_key().expect("clear");
        assert_eq!(store.saved_api_key(), None);
    }
}
