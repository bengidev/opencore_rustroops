//! SQLite persistence for the implicit single-thread chat history.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::api::{DEFAULT_MODEL, GenerationSettings, MessageRole, ModelInfo, SpeedMode};
use rusqlite::{Connection, params};
use thiserror::Error;

use super::model_catalog_store::{CachedModelCatalog, ModelCatalogStore, ModelCatalogStoreError};

/// A message row loaded from SQLite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMessage {
    pub id: i64,
    pub role: MessageRole,
    pub content: String,
}

/// Errors from chat persistence operations.
#[derive(Debug, Error)]
pub enum ChatStoreError {
    #[error("failed to open chat database: {0}")]
    Open(#[from] rusqlite::Error),
    #[error("chat database is not initialized")]
    NotInitialized,
    #[error("unknown message role in database: {0}")]
    InvalidRole(String),
}

/// Per-thread model and generation settings persisted in SQLite.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreadSettings {
    pub model_id: String,
    pub generation: GenerationSettings,
}

impl Default for ThreadSettings {
    fn default() -> Self {
        Self {
            model_id: DEFAULT_MODEL.to_string(),
            generation: GenerationSettings::default(),
        }
    }
}

/// Persistence backend for chat threads and messages.
pub trait ChatStore: Send + Sync {
    fn ensure_thread(&self) -> Result<i64, ChatStoreError>;
    fn load_messages(&self, thread_id: i64) -> Result<Vec<StoredMessage>, ChatStoreError>;
    fn load_thread_settings(&self, thread_id: i64) -> Result<ThreadSettings, ChatStoreError>;
    fn save_thread_settings(
        &self,
        thread_id: i64,
        settings: &ThreadSettings,
    ) -> Result<(), ChatStoreError>;
    fn insert_message(
        &self,
        thread_id: i64,
        role: MessageRole,
        content: &str,
    ) -> Result<i64, ChatStoreError>;
    fn update_message_content(&self, message_id: i64, content: &str) -> Result<(), ChatStoreError>;
    fn delete_message(&self, message_id: i64) -> Result<(), ChatStoreError>;
}

/// SQLite-backed chat store under the application data directory.
pub struct SqliteChatStore {
    connection: Mutex<Connection>,
}

impl SqliteChatStore {
    pub fn default_path() -> Result<PathBuf, ChatStoreError> {
        let base = directories::ProjectDirs::from("com", "opencore", "opencore_rustroops")
            .ok_or_else(|| {
                rusqlite::Error::InvalidPath(PathBuf::from("application data directory"))
            })?
            .data_dir()
            .to_path_buf();
        Ok(base.join("chat.db"))
    }

    pub fn open() -> Result<Self, ChatStoreError> {
        let path = Self::default_path()?;
        Self::open_at(path)
    }

    /// Opens the store, recreating the database file if it is corrupt or unreadable.
    pub fn open_at(path: PathBuf) -> Result<Self, ChatStoreError> {
        match Self::at(path.clone()) {
            Ok(store) => Ok(store),
            Err(error) => {
                eprintln!("opencore: chat database unusable ({error}); recreating");
                Self::recreate_at(path)
            }
        }
    }

    fn recreate_at(path: PathBuf) -> Result<Self, ChatStoreError> {
        if path.exists() {
            let backup = path.with_extension("corrupt");
            let _ = std::fs::remove_file(&backup);
            std::fs::rename(&path, &backup)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        Self::at(path)
    }

    pub fn at(path: impl Into<PathBuf>) -> Result<Self, ChatStoreError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }

        let connection = Connection::open(path)?;
        let store = Self {
            connection: Mutex::new(connection),
        };
        store.initialize_schema()?;
        store.migrate_schema()?;
        Ok(store)
    }

    fn migrate_schema(&self) -> Result<(), ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        let mut columns = connection
            .prepare("PRAGMA table_info(threads)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;

        if !columns.iter().any(|column| column == "model_id") {
            connection.execute("ALTER TABLE threads ADD COLUMN model_id TEXT", [])?;
            columns.push("model_id".into());
        }
        if !columns.iter().any(|column| column == "temperature") {
            connection.execute("ALTER TABLE threads ADD COLUMN temperature REAL", [])?;
            columns.push("temperature".into());
        }
        if !columns.iter().any(|column| column == "max_tokens") {
            connection.execute("ALTER TABLE threads ADD COLUMN max_tokens INTEGER", [])?;
            columns.push("max_tokens".into());
        }
        if !columns.iter().any(|column| column == "reasoning_effort") {
            connection.execute("ALTER TABLE threads ADD COLUMN reasoning_effort TEXT", [])?;
            columns.push("reasoning_effort".into());
        }
        if !columns.iter().any(|column| column == "speed_mode") {
            connection.execute("ALTER TABLE threads ADD COLUMN speed_mode TEXT", [])?;
        }

        connection.execute(
            "UPDATE threads SET model_id = ?1 WHERE model_id IS NULL",
            params![DEFAULT_MODEL],
        )?;
        Ok(())
    }

    fn initialize_schema(&self) -> Result<(), ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS threads (
                id INTEGER PRIMARY KEY,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                thread_id INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(thread_id) REFERENCES threads(id)
            );

            CREATE TABLE IF NOT EXISTS model_catalog (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS model_catalog_meta (
                fetched_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }
}

impl ModelCatalogStore for SqliteChatStore {
    fn load_catalog(&self) -> Result<CachedModelCatalog, ModelCatalogStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        let fetched_at: Option<String> = connection
            .query_row(
                "SELECT fetched_at FROM model_catalog_meta LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        let mut statement = connection.prepare("SELECT data FROM model_catalog ORDER BY id ASC")?;
        let rows = statement.query_map([], |row| {
            let data: String = row.get(0)?;
            serde_json::from_str(&data).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(
                    ModelCatalogStoreError::Deserialize(error.to_string()),
                ))
            })
        })?;

        let models = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(ModelCatalogStoreError::from)?;

        Ok(CachedModelCatalog { models, fetched_at })
    }

    fn save_catalog(
        &self,
        models: &[ModelInfo],
        fetched_at: &str,
    ) -> Result<(), ModelCatalogStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        let transaction = connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM model_catalog", [])?;
        transaction.execute("DELETE FROM model_catalog_meta", [])?;

        for model in models {
            let data = serde_json::to_string(model)
                .map_err(|error| ModelCatalogStoreError::Serialize(error.to_string()))?;
            transaction.execute(
                "INSERT INTO model_catalog (id, data) VALUES (?1, ?2)",
                params![model.id, data],
            )?;
        }

        transaction.execute(
            "INSERT INTO model_catalog_meta (fetched_at) VALUES (?1)",
            params![fetched_at],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

impl ChatStore for SqliteChatStore {
    fn ensure_thread(&self) -> Result<i64, ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        let existing: Option<i64> = connection
            .query_row("SELECT id FROM threads ORDER BY id LIMIT 1", [], |row| {
                row.get(0)
            })
            .ok();

        if let Some(thread_id) = existing {
            return Ok(thread_id);
        }

        connection.execute(
            "INSERT INTO threads (id, created_at, model_id) VALUES (1, ?1, ?2)",
            params![timestamp_now(), DEFAULT_MODEL],
        )?;
        Ok(1)
    }

    fn load_messages(&self, thread_id: i64) -> Result<Vec<StoredMessage>, ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        let mut statement = connection.prepare(
            "SELECT id, role, content FROM messages WHERE thread_id = ?1 ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![thread_id], |row| {
            let role: String = row.get(1)?;
            let role = parse_role(&role)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            Ok(StoredMessage {
                id: row.get(0)?,
                role,
                content: row.get(2)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(ChatStoreError::from)
    }

    fn load_thread_settings(&self, thread_id: i64) -> Result<ThreadSettings, ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection
            .query_row(
                "SELECT model_id, temperature, max_tokens, reasoning_effort, speed_mode FROM threads WHERE id = ?1",
                params![thread_id],
                |row| {
                    let model_id: Option<String> = row.get(0)?;
                    let temperature: Option<f64> = row.get(1)?;
                    let max_tokens: Option<i64> = row.get(2)?;
                    let reasoning_effort: Option<String> = row.get(3)?;
                    let speed_mode: Option<String> = row.get(4)?;
                    Ok(ThreadSettings {
                        model_id: model_id.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
                        generation: GenerationSettings {
                            temperature: temperature.map(|value| value as f32),
                            max_tokens: max_tokens.and_then(|value| u32::try_from(value).ok()),
                            reasoning_effort,
                            speed_mode: SpeedMode::from_persisted(speed_mode.as_deref()),
                        },
                    })
                },
            )
            .map_err(ChatStoreError::from)
    }

    fn save_thread_settings(
        &self,
        thread_id: i64,
        settings: &ThreadSettings,
    ) -> Result<(), ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection.execute(
            "UPDATE threads SET model_id = ?1, temperature = ?2, max_tokens = ?3, reasoning_effort = ?4, speed_mode = ?5 WHERE id = ?6",
            params![
                settings.model_id,
                settings.generation.temperature.map(f64::from),
                settings.generation.max_tokens.map(i64::from),
                settings.generation.reasoning_effort,
                settings.generation.speed_mode.as_str(),
                thread_id,
            ],
        )?;
        Ok(())
    }

    fn insert_message(
        &self,
        thread_id: i64,
        role: MessageRole,
        content: &str,
    ) -> Result<i64, ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection.execute(
            "INSERT INTO messages (thread_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![thread_id, role_as_str(role), content, timestamp_now()],
        )?;
        Ok(connection.last_insert_rowid())
    }

    fn update_message_content(&self, message_id: i64, content: &str) -> Result<(), ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection.execute(
            "UPDATE messages SET content = ?1 WHERE id = ?2",
            params![content, message_id],
        )?;
        Ok(())
    }

    fn delete_message(&self, message_id: i64) -> Result<(), ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection.execute("DELETE FROM messages WHERE id = ?1", params![message_id])?;
        Ok(())
    }
}

/// In-memory chat store for unit tests.
#[derive(Debug, Default)]
pub struct InMemoryChatStore {
    messages: Mutex<Vec<StoredMessage>>,
    thread_id: Mutex<Option<i64>>,
    next_id: Mutex<i64>,
    thread_settings: Mutex<ThreadSettings>,
}

impl InMemoryChatStore {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
            thread_id: Mutex::new(None),
            next_id: Mutex::new(1),
            thread_settings: Mutex::new(ThreadSettings::default()),
        }
    }
}

impl ChatStore for InMemoryChatStore {
    fn ensure_thread(&self) -> Result<i64, ChatStoreError> {
        let mut thread_id = self.thread_id.lock().expect("thread lock");
        if let Some(id) = *thread_id {
            return Ok(id);
        }
        *thread_id = Some(1);
        Ok(1)
    }

    fn load_messages(&self, thread_id: i64) -> Result<Vec<StoredMessage>, ChatStoreError> {
        let _ = thread_id;
        Ok(self.messages.lock().expect("messages lock").clone())
    }

    fn load_thread_settings(&self, thread_id: i64) -> Result<ThreadSettings, ChatStoreError> {
        let _ = thread_id;
        Ok(self.thread_settings.lock().expect("settings lock").clone())
    }

    fn save_thread_settings(
        &self,
        thread_id: i64,
        settings: &ThreadSettings,
    ) -> Result<(), ChatStoreError> {
        let _ = thread_id;
        *self.thread_settings.lock().expect("settings lock") = settings.clone();
        Ok(())
    }

    fn insert_message(
        &self,
        thread_id: i64,
        role: MessageRole,
        content: &str,
    ) -> Result<i64, ChatStoreError> {
        let _ = thread_id;
        let mut next_id = self.next_id.lock().expect("id lock");
        let id = *next_id;
        *next_id += 1;
        self.messages
            .lock()
            .expect("messages lock")
            .push(StoredMessage {
                id,
                role,
                content: content.to_string(),
            });
        Ok(id)
    }

    fn update_message_content(&self, message_id: i64, content: &str) -> Result<(), ChatStoreError> {
        let mut messages = self.messages.lock().expect("messages lock");
        let message = messages
            .iter_mut()
            .find(|message| message.id == message_id)
            .ok_or(ChatStoreError::NotInitialized)?;
        message.content = content.to_string();
        Ok(())
    }

    fn delete_message(&self, message_id: i64) -> Result<(), ChatStoreError> {
        let mut messages = self.messages.lock().expect("messages lock");
        messages.retain(|message| message.id != message_id);
        Ok(())
    }
}

fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

fn role_as_str(role: MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

fn parse_role(role: &str) -> Result<MessageRole, ChatStoreError> {
    match role {
        "system" => Ok(MessageRole::System),
        "user" => Ok(MessageRole::User),
        "assistant" => Ok(MessageRole::Assistant),
        other => Err(ChatStoreError::InvalidRole(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sqlite_store_round_trips_messages_for_implicit_thread() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        let store = SqliteChatStore::at(&path).expect("open store");
        let thread_id = store.ensure_thread().expect("thread");
        let user_id = store
            .insert_message(thread_id, MessageRole::User, "Hello")
            .expect("insert user");
        let assistant_id = store
            .insert_message(thread_id, MessageRole::Assistant, "Hi")
            .expect("insert assistant");
        store
            .update_message_content(assistant_id, "Hi there")
            .expect("update");

        let messages = store.load_messages(thread_id).expect("load");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, user_id);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi there");
    }

    #[test]
    fn sqlite_store_reopens_with_existing_history() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        {
            let store = SqliteChatStore::at(&path).expect("open store");
            let thread_id = store.ensure_thread().expect("thread");
            store
                .insert_message(thread_id, MessageRole::User, "Persist me")
                .expect("insert");
        }

        let store = SqliteChatStore::at(&path).expect("reopen store");
        let thread_id = store.ensure_thread().expect("thread");
        let messages = store.load_messages(thread_id).expect("load");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Persist me");
    }

    #[test]
    fn sqlite_store_round_trips_thread_settings() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        let store = SqliteChatStore::at(&path).expect("open store");
        let thread_id = store.ensure_thread().expect("thread");
        let settings = ThreadSettings {
            model_id: "anthropic/claude-3.5-sonnet".into(),
            generation: GenerationSettings {
                temperature: Some(0.7),
                max_tokens: Some(4096),
                reasoning_effort: Some("high".into()),
                speed_mode: SpeedMode::Fast,
            },
        };
        store
            .save_thread_settings(thread_id, &settings)
            .expect("save settings");
        let loaded = store
            .load_thread_settings(thread_id)
            .expect("load settings");
        assert_eq!(loaded, settings);
    }

    #[test]
    fn stop_persists_partial_accumulated_content() {
        let store = InMemoryChatStore::new();
        let thread_id = store.ensure_thread().expect("thread");
        let _user_id = store
            .insert_message(thread_id, MessageRole::User, "hello")
            .expect("user msg");

        // Simulate: assistant starts as pending, first token arrives → DB insert with partial
        let first_content = "Hel";
        let assistant_id = store
            .insert_message(thread_id, MessageRole::Assistant, first_content)
            .expect("assistant msg");

        // More tokens arrive in-memory (DB still has "Hel")
        let accumulated = "Hello, world!";

        // On stop: persist the full accumulated content (what the Error arm now does)
        store
            .update_message_content(assistant_id, accumulated)
            .expect("persist stop content");

        let messages = store.load_messages(thread_id).expect("load");
        let assistant_msg = messages
            .iter()
            .find(|m| m.role == MessageRole::Assistant)
            .expect("assistant message");
        assert_eq!(assistant_msg.content, "Hello, world!");
        assert!(messages.len() >= 2);
    }
}

