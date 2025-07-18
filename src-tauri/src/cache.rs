use crate::{app::App, desktop, error::AppError};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppCache {
    apps: Vec<App>,
    dir_mod_times: HashMap<PathBuf, SystemTime>,
}

impl AppCache {
    pub fn get_cache_path() -> Result<PathBuf, AppError> {
        let cache_dir = env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".cache")))
            .map_err(|_| AppError::DirectoryNotFound)?;

        let app_cache_dir = cache_dir.join("raycast-linux");
        fs::create_dir_all(&app_cache_dir)?;
        Ok(app_cache_dir.join("apps.bincode"))
    }

    pub fn read_from_file(path: &Path) -> Result<AppCache, AppError> {
        let file_content = fs::read(path)?;
        let (decoded, _) =
            bincode::serde::decode_from_slice(&file_content, bincode::config::standard())?;
        Ok(decoded)
    }

    pub fn write_to_file(&self, path: &Path) -> Result<(), AppError> {
        let encoded = bincode::serde::encode_to_vec(self, bincode::config::standard())?;
        fs::write(path, encoded)?;
        Ok(())
    }

    pub fn is_stale(&self, new_dir_mod_times: &HashMap<PathBuf, SystemTime>) -> bool {
        for (dir, cached_mod_time) in &self.dir_mod_times {
            if let Some(current_mod_time) = new_dir_mod_times.get(dir) {
                if current_mod_time > cached_mod_time {
                    return true;
                }
            } else {
                // Directory in cache no longer exists
                return true;
            }
        }
        // Check for new directories not in cache
        new_dir_mod_times.len() != self.dir_mod_times.len()
    }

    pub fn get_apps() -> Result<Vec<App>, AppError> {
        let cache_path = Self::get_cache_path()?;

        if let Ok(cached_data) = Self::read_from_file(&cache_path) {
            // Perform a quick check without a full rescan
            if let Ok((_, new_dir_mod_times)) = desktop::scan_and_parse_apps() {
                if !cached_data.is_stale(&new_dir_mod_times) {
                    return Ok(cached_data.apps);
                }
            }
        }

        Self::refresh_and_get_apps()
    }

    pub fn refresh_and_get_apps() -> Result<Vec<App>, AppError> {
        let (apps, dir_mod_times) = desktop::scan_and_parse_apps()?;
        let cache_data = AppCache {
            apps: apps.clone(),
            dir_mod_times,
        };

        if let Ok(cache_path) = Self::get_cache_path() {
            if let Err(e) = cache_data.write_to_file(&cache_path) {
                eprintln!("Failed to write to app cache: {:?}", e);
            }
        }

        Ok(apps)
    }

    pub fn refresh_background() {
        if let Err(e) = Self::refresh_and_get_apps() {
            eprintln!("Error refreshing app cache in background: {:?}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;

    fn setup_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "raycast_test_cache_{}_{}",
            name,
            rand::random::<u32>()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_cache_file_roundtrip() {
        let temp_dir = setup_temp_dir("roundtrip");
        let cache_path = temp_dir.join("test_cache.bincode");

        let mut dir_mod_times = HashMap::new();
        dir_mod_times.insert(PathBuf::from("/test/path"), SystemTime::now());

        let original_cache = AppCache {
            apps: vec![App::new("TestApp".to_string())],
            dir_mod_times,
        };

        original_cache.write_to_file(&cache_path).unwrap();

        let read_cache = AppCache::read_from_file(&cache_path).unwrap();

        assert_eq!(original_cache.apps.len(), read_cache.apps.len());
        assert_eq!(original_cache.apps[0].name, read_cache.apps[0].name);
        assert_eq!(original_cache.dir_mod_times, read_cache.dir_mod_times);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    fn get_mock_app_directories(mock_dir: PathBuf) -> Vec<PathBuf> {
        vec![mock_dir]
    }

    fn is_stale_mock(cache: &AppCache, mock_dir: PathBuf) -> bool {
        get_mock_app_directories(mock_dir).into_iter().any(|dir| {
            let current_mod_time = fs::metadata(&dir).ok().and_then(|m| m.modified().ok());
            let cached_mod_time = cache.dir_mod_times.get(&dir);
            match (current_mod_time, cached_mod_time) {
                (Some(current), Some(cached)) => current > *cached,
                _ => true,
            }
        })
    }

    #[test]
    fn test_is_stale_logic() {
        let temp_dir = setup_temp_dir("is_stale");

        let mod_time_before = fs::metadata(&temp_dir).unwrap().modified().unwrap();
        let mut dir_mod_times = HashMap::new();
        dir_mod_times.insert(temp_dir.clone(), mod_time_before);
        let cache = AppCache {
            apps: vec![],
            dir_mod_times,
        };
        assert!(!is_stale_mock(&cache, temp_dir.clone()));

        thread::sleep(Duration::from_millis(10));
        let mut file = fs::File::create(temp_dir.join("test.txt")).unwrap();
        file.write_all(b"Hello, world!").unwrap();
        drop(file);
        assert!(is_stale_mock(&cache, temp_dir.clone()));

        let mod_time_after = fs::metadata(&temp_dir).unwrap().modified().unwrap();
        let mut new_dir_mod_times = HashMap::new();
        new_dir_mod_times.insert(temp_dir.clone(), mod_time_after);
        let cache_updated = AppCache {
            apps: vec![],
            dir_mod_times: new_dir_mod_times,
        };
        assert!(!is_stale_mock(&cache_updated, temp_dir.clone()));

        let cache_missing_entry = AppCache {
            apps: vec![],
            dir_mod_times: HashMap::new(),
        };
        assert!(is_stale_mock(&cache_missing_entry, temp_dir.clone()));

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
