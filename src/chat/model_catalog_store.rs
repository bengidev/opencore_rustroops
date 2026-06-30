//! SQLite cache for the OpenRouter model catalog.

use std::sync::Mutex;

use crate::api::ModelInfo;
use thiserror::Error;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedModelCatalog {
    pub models: Vec<ModelInfo>,
    pub fetched_at: Option<String>,
}

/// Errors from model catalog persistence operations.
#[derive(Debug, Error)]
pub enum ModelCatalogStoreError {
    #[error("failed to open model catalog database: {0}")]
    Open(#[from] rusqlite::Error),
    #[error("failed to serialize model catalog entry: {0}")]
    Serialize(String),
    #[error("failed to deserialize model catalog entry: {0}")]
    Deserialize(String),
}

/// Persistence backend for the model catalog cache.
pub trait ModelCatalogStore: Send + Sync {
    fn load_catalog(&self) -> Result<CachedModelCatalog, ModelCatalogStoreError>;
    fn save_catalog(
        &self,
        models: &[ModelInfo],
        fetched_at: &str,
    ) -> Result<(), ModelCatalogStoreError>;
}

/// In-memory model catalog store for unit tests.
#[derive(Debug, Default)]
pub struct InMemoryModelCatalogStore {
    catalog: Mutex<Option<CachedModelCatalog>>,
}

impl InMemoryModelCatalogStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ModelCatalogStore for InMemoryModelCatalogStore {
    fn load_catalog(&self) -> Result<CachedModelCatalog, ModelCatalogStoreError> {
        Ok(self
            .catalog
            .lock()
            .expect("catalog lock")
            .clone()
            .unwrap_or(CachedModelCatalog {
                models: Vec::new(),
                fetched_at: None,
            }))
    }

    fn save_catalog(
        &self,
        models: &[ModelInfo],
        fetched_at: &str,
    ) -> Result<(), ModelCatalogStoreError> {
        *self.catalog.lock().expect("catalog lock") = Some(CachedModelCatalog {
            models: models.to_vec(),
            fetched_at: Some(fetched_at.to_string()),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ModelInfo;
    use crate::chat::SqliteChatStore;
    use tempfile::TempDir;

    fn sample_model() -> ModelInfo {
        ModelInfo {
            id: "openai/gpt-4".into(),
            name: "GPT-4".into(),
            context_length: Some(8192),
            input_modalities: vec!["text".into()],
            output_modalities: vec!["text".into()],
            supported_parameters: vec!["temperature".into(), "max_tokens".into()],
            reasoning: None,
        }
    }

    #[test]
    fn sqlite_catalog_round_trips_models_and_fetched_at() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        let store = SqliteChatStore::at(&path).expect("open store");
        let models = vec![sample_model()];
        store
            .save_catalog(&models, "1700000000")
            .expect("save catalog");

        let loaded = store.load_catalog().expect("load catalog");
        assert_eq!(loaded.fetched_at.as_deref(), Some("1700000000"));
        assert_eq!(loaded.models, models);

        let reopened = SqliteChatStore::at(&path)
            .expect("reopen store")
            .load_catalog()
            .expect("reload catalog");
        assert_eq!(reopened, loaded);
    }
}
