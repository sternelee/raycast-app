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

    #[cfg(test)]
    pub fn new_for_test() -> Result<Self, AppError> {
        let store = Store::new_in_memory()?;
        store.init_table(FRECENCY_SCHEMA)?;
        store.init_table(HIDDEN_ITEMS_SCHEMA)?;
        Ok(Self { store })
    }

    pub fn record_usage(&self, item_id: String) -> Result<(), AppError> {
        let now = Utc::now().timestamp_nanos_opt().unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_record_usage_new_item() {
        let manager = FrecencyManager::new_for_test().unwrap();
        let item_id = "new_item".to_string();

        manager.record_usage(item_id.clone()).unwrap();

        let data = manager.get_frecency_data().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].item_id, item_id);
        assert_eq!(data[0].use_count, 1);
        assert!(data[0].last_used_at > 0);
    }

    #[test]
    fn test_record_usage_existing_item() {
        let manager = FrecencyManager::new_for_test().unwrap();
        let item_id = "existing_item".to_string();

        manager.record_usage(item_id.clone()).unwrap();
        let data1 = manager.get_frecency_data().unwrap();
        let time1 = data1[0].last_used_at;

        thread::sleep(Duration::from_millis(10));

        manager.record_usage(item_id.clone()).unwrap();
        let data2 = manager.get_frecency_data().unwrap();
        let time2 = data2[0].last_used_at;

        assert_eq!(data2.len(), 1);
        assert_eq!(data2[0].use_count, 2);
        assert!(time2 > time1, "last_used_at should be updated");
    }

    #[test]
    fn test_get_frecency_data_empty() {
        let manager = FrecencyManager::new_for_test().unwrap();
        let data = manager.get_frecency_data().unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_delete_frecency_entry() {
        let manager = FrecencyManager::new_for_test().unwrap();
        let item_id = "to_delete".to_string();
        manager.record_usage(item_id.clone()).unwrap();
        assert_eq!(manager.get_frecency_data().unwrap().len(), 1);

        manager.delete_frecency_entry(item_id).unwrap();
        assert!(manager.get_frecency_data().unwrap().is_empty());
    }

    #[test]
    fn test_delete_non_existent_entry() {
        let manager = FrecencyManager::new_for_test().unwrap();
        let result = manager.delete_frecency_entry("non_existent".to_string());
        assert!(result.is_ok());
        assert!(manager.get_frecency_data().unwrap().is_empty());
    }

    #[test]
    fn test_hide_item_and_get_ids() {
        let manager = FrecencyManager::new_for_test().unwrap();
        let item1 = "hidden1".to_string();
        let item2 = "hidden2".to_string();

        assert!(manager.get_hidden_item_ids().unwrap().is_empty());

        manager.hide_item(item1.clone()).unwrap();
        let hidden = manager.get_hidden_item_ids().unwrap();
        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0], item1);

        manager.hide_item(item2.clone()).unwrap();
        let hidden = manager.get_hidden_item_ids().unwrap();
        assert_eq!(hidden.len(), 2);
        assert!(hidden.contains(&item1));
        assert!(hidden.contains(&item2));
    }

    #[test]
    fn test_hide_item_is_idempotent() {
        let manager = FrecencyManager::new_for_test().unwrap();
        let item1 = "hidden1".to_string();

        manager.hide_item(item1.clone()).unwrap();
        assert_eq!(manager.get_hidden_item_ids().unwrap().len(), 1);

        manager.hide_item(item1.clone()).unwrap();
        assert_eq!(manager.get_hidden_item_ids().unwrap().len(), 1);
    }
}
