use crate::error::AppError;
use crate::store::{Storable, Store};
use chrono::Utc;
use rusqlite::{params, Result as RusqliteResult};
use serde::Serialize;
use tauri::AppHandle;

const FRECENCY_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS frecency (
    item_id TEXT PRIMARY KEY,
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_at INTEGER NOT NULL
)";
const HIDDEN_ITEMS_SCHEMA: &str =
    "CREATE TABLE IF NOT EXISTS hidden_items (item_id TEXT PRIMARY KEY)";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FrecencyData {
    pub item_id: String,
    pub use_count: i64,
    pub last_used_at: i64,
}

impl Storable for FrecencyData {
    fn from_row(row: &rusqlite::Row) -> RusqliteResult<Self> {
        Ok(FrecencyData {
            item_id: row.get(0)?,
            use_count: row.get(1)?,
            last_used_at: row.get(2)?,
        })
    }
}

pub struct FrecencyManager {
    store: Store,
}

impl FrecencyManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self, AppError> {
        let store = Store::new(app_handle, "frecency.sqlite")?;
        store.init_table(FRECENCY_SCHEMA)?;
        store.init_table(HIDDEN_ITEMS_SCHEMA)?;
        Ok(Self { store })
    }

    pub fn record_usage(&self, item_id: String) -> Result<(), AppError> {
        let now = Utc::now().timestamp();
        self.store.execute(
            "INSERT INTO frecency (item_id, use_count, last_used_at) VALUES (?, 1, ?)
             ON CONFLICT(item_id) DO UPDATE SET
                use_count = use_count + 1,
                last_used_at = excluded.last_used_at",
            params![item_id, now],
        )?;
        Ok(())
    }

    pub fn get_frecency_data(&self) -> Result<Vec<FrecencyData>, AppError> {
        self.store
            .query("SELECT item_id, use_count, last_used_at FROM frecency", [])
    }

    pub fn delete_frecency_entry(&self, item_id: String) -> Result<(), AppError> {
        self.store
            .execute("DELETE FROM frecency WHERE item_id = ?", params![item_id])?;
        Ok(())
    }

    pub fn hide_item(&self, item_id: String) -> Result<(), AppError> {
        self.store.execute(
            "INSERT OR IGNORE INTO hidden_items (item_id) VALUES (?)",
            params![item_id],
        )?;
        Ok(())
    }

    pub fn get_hidden_item_ids(&self) -> Result<Vec<String>, AppError> {
        let db = self.store.conn();
        let mut stmt = db.prepare("SELECT item_id FROM hidden_items")?;
        let ids_iter = stmt.query_map([], |row| row.get(0))?;

        ids_iter
            .collect::<RusqliteResult<Vec<String>>>()
            .map_err(|e| e.into())
    }
}
