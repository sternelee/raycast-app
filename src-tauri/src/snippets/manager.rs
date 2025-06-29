use crate::error::AppError;
use crate::snippets::types::Snippet;
use crate::store::{Storable, Store};
use chrono::{DateTime, Utc};
use rusqlite::params;
use std::sync::Arc;
use tauri::AppHandle;

const SNIPPETS_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    keyword TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)";

#[derive(Clone)]
pub struct SnippetManager {
    store: Arc<Store>,
}

impl Storable for Snippet {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let created_at_ts: i64 = row.get(4)?;
        let updated_at_ts: i64 = row.get(5)?;
        let last_used_at_ts: i64 = row.get(7)?;
        Ok(Snippet {
            id: row.get(0)?,
            name: row.get(1)?,
            keyword: row.get(2)?,
            content: row.get(3)?,
            created_at: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_default(),
            updated_at: DateTime::from_timestamp(updated_at_ts, 0).unwrap_or_default(),
            times_used: row.get(6)?,
            last_used_at: DateTime::from_timestamp(last_used_at_ts, 0).unwrap_or_default(),
        })
    }
}

impl SnippetManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self, AppError> {
        let store = Store::new(app_handle, "snippets.sqlite")?;
        store.init_table(SNIPPETS_SCHEMA)?;

        // Handle simple schema migrations in a block to drop the lock
        {
            let db = store.conn();
            let mut stmt = db.prepare("PRAGMA table_info(snippets)")?;
            let columns: Vec<String> = stmt
                .query_map([], |row| row.get(1))?
                .collect::<Result<Vec<_>, _>>()?;

            if !columns.contains(&"times_used".to_string()) {
                db.execute(
                    "ALTER TABLE snippets ADD COLUMN times_used INTEGER NOT NULL DEFAULT 0",
                    [],
                )?;
            }
            if !columns.contains(&"last_used_at".to_string()) {
                db.execute(
                    "ALTER TABLE snippets ADD COLUMN last_used_at INTEGER NOT NULL DEFAULT 0",
                    [],
                )?;
            }
        }

        Ok(Self {
            store: Arc::new(store),
        })
    }

    pub fn create_snippet(
        &self,
        name: String,
        keyword: String,
        content: String,
    ) -> Result<i64, AppError> {
        let now = Utc::now().timestamp();
        self.store.execute(
            "INSERT INTO snippets (name, keyword, content, created_at, updated_at, times_used, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?4, 0, 0)",
            params![name, keyword, content, now],
        )?;
        Ok(self.store.last_insert_rowid())
    }

    pub fn list_snippets(&self, search_term: Option<String>) -> Result<Vec<Snippet>, AppError> {
        let mut query = "SELECT id, name, keyword, content, created_at, updated_at, times_used, last_used_at FROM snippets".to_string();

        if let Some(term) = search_term {
            if !term.is_empty() {
                query.push_str(" WHERE name LIKE ?1 OR keyword LIKE ?1 OR content LIKE ?1");
                query.push_str(" ORDER BY updated_at DESC");
                let search_param = format!("%{}%", term);
                return self.store.query(&query, params![search_param]);
            }
        }

        query.push_str(" ORDER BY updated_at DESC");
        self.store.query(&query, [])
    }

    pub fn update_snippet(
        &self,
        id: i64,
        name: String,
        keyword: String,
        content: String,
    ) -> Result<(), AppError> {
        let now = Utc::now().timestamp();
        self.store.execute(
            "UPDATE snippets SET name = ?1, keyword = ?2, content = ?3, updated_at = ?4 WHERE id = ?5",
            params![name, keyword, content, now, id],
        )?;
        Ok(())
    }

    pub fn delete_snippet(&self, id: i64) -> Result<(), AppError> {
        self.store
            .execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn snippet_was_used(&self, id: i64) -> Result<(), AppError> {
        let now = Utc::now().timestamp();
        self.store.execute(
            "UPDATE snippets SET times_used = times_used + 1, last_used_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn find_snippet_by_keyword(&self, keyword: &str) -> Result<Option<Snippet>, AppError> {
        self.store.query_row(
            "SELECT id, name, keyword, content, created_at, updated_at, times_used, last_used_at FROM snippets WHERE keyword = ?1",
            params![keyword],
        )
    }

    pub fn find_snippet_by_name(&self, name: &str) -> Result<Option<Snippet>, AppError> {
        self.store.query_row(
            "SELECT id, name, keyword, content, created_at, updated_at, times_used, last_used_at FROM snippets WHERE name = ?1 ORDER BY updated_at DESC LIMIT 1",
            params![name],
        )
    }
}
