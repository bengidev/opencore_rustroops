//! SQLite persistence for the multi-thread conversation history.

use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::HashMap;

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
/// A thread row loaded from SQLite for the picker list.
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    pub id: i64,
    pub title: Option<String>,
    pub created_at: String,
    pub model_id: String,
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
    fn list_threads(&self) -> Result<Vec<ThreadInfo>, ChatStoreError>;
    fn create_thread(&self, settings: &ThreadSettings) -> Result<i64, ChatStoreError>;
    fn delete_thread(&self, thread_id: i64) -> Result<(), ChatStoreError>;
    fn set_thread_title(&self, thread_id: i64, title: &str) -> Result<(), ChatStoreError>;
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

        if !columns.iter().any(|c| c == "title") {
            connection.execute("ALTER TABLE threads ADD COLUMN title TEXT", [])?;
            columns.push("title".into());
        }
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
            .query_row("SELECT id FROM threads ORDER BY id DESC LIMIT 1", [], |row| {
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
    fn list_threads(&self) -> Result<Vec<ThreadInfo>, ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        let mut statement = connection.prepare(
            "SELECT id, title, created_at, model_id FROM threads ORDER BY id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ThreadInfo {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                model_id: row.get::<_, Option<String>>(3)?.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(ChatStoreError::from)
    }

    fn create_thread(&self, settings: &ThreadSettings) -> Result<i64, ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection.execute(
            "INSERT INTO threads (created_at, model_id, temperature, max_tokens, reasoning_effort, speed_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                timestamp_now(),
                settings.model_id,
                settings.generation.temperature.map(f64::from),
                settings.generation.max_tokens.map(i64::from),
                settings.generation.reasoning_effort,
                settings.generation.speed_mode.as_str(),
            ],
        )?;
        Ok(connection.last_insert_rowid())
    }

    fn delete_thread(&self, thread_id: i64) -> Result<(), ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection.execute("DELETE FROM messages WHERE thread_id = ?1", params![thread_id])?;
        connection.execute("DELETE FROM threads WHERE id = ?1", params![thread_id])?;
        Ok(())
    }

    fn set_thread_title(&self, thread_id: i64, title: &str) -> Result<(), ChatStoreError> {
        let connection = self.connection.lock().expect("chat db lock");
        connection.execute(
            "UPDATE threads SET title = ?1 WHERE id = ?2",
            params![title, thread_id],
        )?;
        Ok(())
    }
}

/// In-memory chat store for unit tests.
#[derive(Debug)]
pub struct InMemoryChatStore {
    messages: Mutex<HashMap<i64, Vec<StoredMessage>>>,
    threads: Mutex<Vec<ThreadInfo>>,
    next_thread_id: Mutex<i64>,
    next_message_id: Mutex<i64>,
    thread_settings: Mutex<HashMap<i64, ThreadSettings>>,
}

impl InMemoryChatStore {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(HashMap::new()),
            threads: Mutex::new(Vec::new()),
            next_thread_id: Mutex::new(1),
            next_message_id: Mutex::new(1),
            thread_settings: Mutex::new(HashMap::new()),
        }
    }
}

impl ChatStore for InMemoryChatStore {
    fn ensure_thread(&self) -> Result<i64, ChatStoreError> {
        {
            let threads = self.threads.lock().expect("threads lock");
            if let Some(latest) = threads.last() {
                return Ok(latest.id);
            }
        }
        // Threads list is empty — create a default thread
        let mut next = self.next_thread_id.lock().expect("id lock");
        let id = *next;
        *next = id + 1;
        let thread = ThreadInfo {
            id,
            title: None,
            created_at: timestamp_now(),
            model_id: DEFAULT_MODEL.to_string(),
        };
        self.threads.lock().expect("threads lock").push(thread);
        self.thread_settings
            .lock()
            .expect("settings lock")
            .insert(id, ThreadSettings::default());
        self.messages
            .lock()
            .expect("messages lock")
            .insert(id, Vec::new());
        Ok(id)
    }

    fn load_messages(&self, thread_id: i64) -> Result<Vec<StoredMessage>, ChatStoreError> {
        Ok(self
            .messages
            .lock()
            .expect("messages lock")
            .get(&thread_id)
            .cloned()
            .unwrap_or_default())
    }

    fn load_thread_settings(&self, thread_id: i64) -> Result<ThreadSettings, ChatStoreError> {
        Ok(self
            .thread_settings
            .lock()
            .expect("settings lock")
            .get(&thread_id)
            .cloned()
            .unwrap_or_default())
    }

    fn save_thread_settings(
        &self,
        thread_id: i64,
        settings: &ThreadSettings,
    ) -> Result<(), ChatStoreError> {
        self.thread_settings
            .lock()
            .expect("settings lock")
            .insert(thread_id, settings.clone());
        Ok(())
    }

    fn insert_message(
        &self,
        thread_id: i64,
        role: MessageRole,
        content: &str,
    ) -> Result<i64, ChatStoreError> {
        let mut next_id = self.next_message_id.lock().expect("id lock");
        let id = *next_id;
        *next_id += 1;
        drop(next_id);
        self.messages
            .lock()
            .expect("messages lock")
            .entry(thread_id)
            .or_default()
            .push(StoredMessage {
                id,
                role,
                content: content.to_string(),
            });
        Ok(id)
    }

    fn update_message_content(&self, message_id: i64, content: &str) -> Result<(), ChatStoreError> {
        let mut all_messages = self.messages.lock().expect("messages lock");
        for msgs in all_messages.values_mut() {
            if let Some(msg) = msgs.iter_mut().find(|m| m.id == message_id) {
                msg.content = content.to_string();
                return Ok(());
            }
        }
        Err(ChatStoreError::NotInitialized)
    }

    fn delete_message(&self, message_id: i64) -> Result<(), ChatStoreError> {
        let mut all_messages = self.messages.lock().expect("messages lock");
        for msgs in all_messages.values_mut() {
            msgs.retain(|m| m.id != message_id);
        }
        Ok(())
    }

    fn list_threads(&self) -> Result<Vec<ThreadInfo>, ChatStoreError> {
        Ok(self.threads.lock().expect("threads lock").clone())
    }

    fn create_thread(&self, settings: &ThreadSettings) -> Result<i64, ChatStoreError> {
        let id = {
            let mut next = self.next_thread_id.lock().expect("id lock");
            let id = *next;
            *next += 1;
            id
        };
        let thread = ThreadInfo {
            id,
            title: None,
            created_at: timestamp_now(),
            model_id: settings.model_id.clone(),
        };
        self.threads.lock().expect("threads lock").push(thread);
        self.thread_settings
            .lock()
            .expect("settings lock")
            .insert(id, settings.clone());
        self.messages
            .lock()
            .expect("messages lock")
            .insert(id, Vec::new());
        Ok(id)
    }

    fn delete_thread(&self, thread_id: i64) -> Result<(), ChatStoreError> {
        self.threads
            .lock()
            .expect("threads lock")
            .retain(|t| t.id != thread_id);
        self.thread_settings
            .lock()
            .expect("settings lock")
            .remove(&thread_id);
        self.messages
            .lock()
            .expect("messages lock")
            .remove(&thread_id);
        Ok(())
    }

    fn set_thread_title(&self, thread_id: i64, title: &str) -> Result<(), ChatStoreError> {
        let mut threads = self.threads.lock().expect("threads lock");
        if let Some(thread) = threads.iter_mut().find(|t| t.id == thread_id) {
            thread.title = Some(title.to_string());
        }
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

        // Assistant starts streaming — first token "Hel" arrives and is persisted
        let assistant_id = store
            .insert_message(thread_id, MessageRole::Assistant, "Hel")
            .expect("assistant msg");

        // More content arrives in-memory before stop is clicked.
        // Production path: Error arm calls persist_streaming_assistant →
        // update_message_content with the accumulated content.
        store
            .update_message_content(assistant_id, "Hello, world!")
            .expect("update with accumulated content");

        // Verify persisted content reflects the accumulated state
        let messages = store.load_messages(thread_id).expect("load");
        let assistant_msg = messages
            .iter()
            .find(|m| m.role == MessageRole::Assistant)
            .expect("assistant message");
        assert_eq!(assistant_msg.content, "Hello, world!");
    }

    #[test]
    fn sqlite_store_creates_and_lists_threads() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        let store = SqliteChatStore::at(&path).expect("open store");

        let settings_a = ThreadSettings {
            model_id: "openai/gpt-4o".into(),
            generation: GenerationSettings::default(),
        };
        let settings_b = ThreadSettings {
            model_id: "anthropic/claude-3.5-sonnet".into(),
            generation: GenerationSettings::default(),
        };
        let settings_c = ThreadSettings {
            model_id: "google/gemini-pro".into(),
            generation: GenerationSettings::default(),
        };

        let id_a = store.create_thread(&settings_a).expect("create thread a");
        let id_b = store.create_thread(&settings_b).expect("create thread b");
        let id_c = store.create_thread(&settings_c).expect("create thread c");

        let threads = store.list_threads().expect("list threads");
        assert_eq!(threads.len(), 3);

        // DESC order: most recent (highest id) first
        assert_eq!(threads[0].id, id_c);
        assert_eq!(threads[1].id, id_b);
        assert_eq!(threads[2].id, id_a);

        // Each thread has a unique id and a non-empty timestamp
        let ids: Vec<i64> = threads.iter().map(|t| t.id).collect();
        let mut unique_ids = ids.clone();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len(), "ids must be unique");

        for thread in &threads {
            assert!(!thread.created_at.is_empty(), "created_at must not be empty");
        }

        // Verify model_ids
        assert_eq!(threads[2].model_id, "openai/gpt-4o");
        assert_eq!(threads[1].model_id, "anthropic/claude-3.5-sonnet");
        assert_eq!(threads[0].model_id, "google/gemini-pro");
    }

    #[test]
    fn sqlite_store_inherits_settings_from_source() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        let store = SqliteChatStore::at(&path).expect("open store");

        let source_settings = ThreadSettings {
            model_id: "anthropic/claude-3.5-sonnet".into(),
            generation: GenerationSettings {
                temperature: Some(0.7),
                max_tokens: Some(4096),
                reasoning_effort: Some("high".into()),
                speed_mode: SpeedMode::Fast,
            },
        };
        let _source_id = store.create_thread(&source_settings).expect("create source");
        let new_id = store.create_thread(&source_settings).expect("create new");

        let loaded = store.load_thread_settings(new_id).expect("load settings");
        assert_eq!(loaded, source_settings);
    }

    #[test]
    fn sqlite_store_delete_thread_removes_messages() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        let store = SqliteChatStore::at(&path).expect("open store");

        let settings = ThreadSettings::default();
        let thread_id = store.create_thread(&settings).expect("create thread");

        store
            .insert_message(thread_id, MessageRole::User, "First")
            .expect("insert first");
        store
            .insert_message(thread_id, MessageRole::Assistant, "Second")
            .expect("insert second");

        let messages = store.load_messages(thread_id).expect("load messages");
        assert_eq!(messages.len(), 2);

        store.delete_thread(thread_id).expect("delete thread");

        let threads = store.list_threads().expect("list threads");
        assert!(!threads.iter().any(|t| t.id == thread_id), "deleted thread must not appear");

        let remaining = store.load_messages(thread_id).expect("load after delete");
        assert_eq!(remaining.len(), 0, "messages must be cascaded on thread delete");
    }

    #[test]
    fn sqlite_store_set_thread_title() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        let store = SqliteChatStore::at(&path).expect("open store");

        let settings = ThreadSettings::default();
        let thread_id = store.create_thread(&settings).expect("create thread");

        store.set_thread_title(thread_id, "My Chat").expect("set title");
        let threads = store.list_threads().expect("list threads");
        let thread = threads.iter().find(|t| t.id == thread_id).expect("find thread");
        assert_eq!(thread.title.as_deref(), Some("My Chat"));

        store.set_thread_title(thread_id, "Updated Chat").expect("update title");
        let threads = store.list_threads().expect("list threads");
        let thread = threads.iter().find(|t| t.id == thread_id).expect("find thread");
        assert_eq!(thread.title.as_deref(), Some("Updated Chat"));
    }

    #[test]
    fn sqlite_store_multiple_threads_round_trip() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("chat.db");
        let store = SqliteChatStore::at(&path).expect("open store");

        let settings = ThreadSettings::default();
        let thread_a = store.create_thread(&settings).expect("create thread a");

        store
            .insert_message(thread_a, MessageRole::User, "A-msg1")
            .expect("insert a1");
        store
            .insert_message(thread_a, MessageRole::Assistant, "A-msg2")
            .expect("insert a2");

        let thread_b = store.create_thread(&settings).expect("create thread b");

        store
            .insert_message(thread_b, MessageRole::User, "B-msg1")
            .expect("insert b1");

        let msgs_a = store.load_messages(thread_a).expect("load a messages");
        assert_eq!(msgs_a.len(), 2, "thread A must have 2 messages");
        let msgs_b = store.load_messages(thread_b).expect("load b messages");
        assert_eq!(msgs_b.len(), 1, "thread B must have 1 message");

        // Save non-default settings on thread A and verify they round-trip
        let custom_settings = ThreadSettings {
            model_id: "anthropic/claude-opus-4".into(),
            generation: GenerationSettings {
                temperature: Some(0.3),
                max_tokens: Some(8192),
                reasoning_effort: Some("low".into()),
                speed_mode: SpeedMode::Normal,
            },
        };
        store
            .save_thread_settings(thread_a, &custom_settings)
            .expect("save custom settings");

        let loaded_a = store.load_thread_settings(thread_a).expect("load a settings");
        assert_eq!(loaded_a, custom_settings, "thread A must have custom settings");

        // Thread B should still have default settings
        let loaded_b = store.load_thread_settings(thread_b).expect("load b settings");
        assert_eq!(loaded_b, ThreadSettings::default(), "thread B must have default settings");
    }

    #[test]
    fn in_memory_creates_and_lists_threads() {
        let store = InMemoryChatStore::new();

        let settings_a = ThreadSettings {
            model_id: "openai/gpt-4o".into(),
            generation: GenerationSettings::default(),
        };
        let settings_b = ThreadSettings {
            model_id: "anthropic/claude-3.5-sonnet".into(),
            generation: GenerationSettings::default(),
        };
        let settings_c = ThreadSettings {
            model_id: "google/gemini-pro".into(),
            generation: GenerationSettings::default(),
        };

        let id_a = store.create_thread(&settings_a).expect("create thread a");
        let id_b = store.create_thread(&settings_b).expect("create thread b");
        let id_c = store.create_thread(&settings_c).expect("create thread c");

        let threads = store.list_threads().expect("list threads");
        assert_eq!(threads.len(), 3);

        // InMemoryChatStore returns threads in insertion order (ascending id)
        assert_eq!(threads[0].id, id_a, "in-memory: first inserted thread first");
        assert_eq!(threads[1].id, id_b, "in-memory: second inserted thread second");
        assert_eq!(threads[2].id, id_c, "in-memory: third inserted thread third");

        // Each thread has a unique id and a non-empty timestamp
        let ids: Vec<i64> = threads.iter().map(|t| t.id).collect();
        let mut unique_ids = ids.clone();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len(), "ids must be unique");

        for thread in &threads {
            assert!(!thread.created_at.is_empty(), "created_at must not be empty");
        }

        // Model IDs stored correctly
        let thread_a = threads.iter().find(|t| t.id == id_a).expect("find a");
        assert_eq!(thread_a.model_id, "openai/gpt-4o");
        let thread_b = threads.iter().find(|t| t.id == id_b).expect("find b");
        assert_eq!(thread_b.model_id, "anthropic/claude-3.5-sonnet");
        let thread_c = threads.iter().find(|t| t.id == id_c).expect("find c");
        assert_eq!(thread_c.model_id, "google/gemini-pro");
    }
}
