//! SQLite persistence for the implicit single-thread chat history.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::api::MessageRole;
use rusqlite::{Connection, params};
use thiserror::Error;

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

/// Persistence backend for chat threads and messages.
pub trait ChatStore: Send + Sync {
    fn ensure_thread(&self) -> Result<i64, ChatStoreError>;
    fn load_messages(&self, thread_id: i64) -> Result<Vec<StoredMessage>, ChatStoreError>;
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
        Ok(store)
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
            ",
        )?;
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
            "INSERT INTO threads (id, created_at) VALUES (1, ?1)",
            params![timestamp_now()],
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
}

impl InMemoryChatStore {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
            thread_id: Mutex::new(None),
            next_id: Mutex::new(1),
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
}
