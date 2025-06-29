use crate::error::AppError;
use rusqlite::Connection;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Manager};

pub struct Store {
    db: Mutex<Connection>,
}

impl Store {
    pub fn new(app_handle: &AppHandle, db_filename: &str) -> Result<Self, AppError> {
        let data_dir = app_handle
            .path()
            .app_local_data_dir()
            .map_err(|_| AppError::DirectoryNotFound)?;
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)?;
        }
        let db_path = data_dir.join(db_filename);
        let db = Connection::open(db_path)?;

        Ok(Self { db: Mutex::new(db) })
    }

    pub fn init_table(&self, schema: &str) -> Result<(), AppError> {
        self.db.lock().unwrap().execute(schema, [])?;
        Ok(())
    }

    pub fn conn(&self) -> MutexGuard<Connection> {
        self.db.lock().unwrap()
    }
}
