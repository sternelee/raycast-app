use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use plist::Value;
use rayon::prelude::*;
use crate::app::App;
use crate::error::AppError;

use std::time::SystemTime;

pub fn scan_and_parse_apps() -> Result<(Vec<App>, HashMap<PathBuf, SystemTime>), AppError> {
    let app_dirs = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];

    let app_files: Vec<PathBuf> = app_dirs
        .par_iter()
        .filter(|dir| dir.exists())
        .flat_map(|dir| find_app_bundles(&dir))
        .collect();

    let apps: Vec<App> = app_files
        .par_iter()
        .filter_map(|file_path| parse_app_bundle(file_path))
        .collect();
    
    let dir_mod_times = get_directory_modification_times(app_dirs.clone())?;

    Ok((apps, dir_mod_times))
}

fn get_directory_modification_times(
    app_dirs: Vec<PathBuf>,
) -> Result<HashMap<PathBuf, SystemTime>, AppError> {
    Ok(app_dirs
        .into_iter()
        .filter_map(|dir| {
            fs::metadata(&dir)
                .and_then(|m| m.modified())
                .ok()
                .map(|mod_time| (dir, mod_time))
        })
        .collect())
}

fn find_app_bundles(path: &Path) -> Vec<PathBuf> {
    let mut app_bundles = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.extension().map_or(false, |ext| ext == "app") {
                app_bundles.push(path);
            }
        }
    }
    app_bundles
}

use crate::cache::AppCache;
use icns::{IconFamily, IconType};
use image::ImageFormat;

fn parse_app_bundle(bundle_path: &Path) -> Option<App> {
    let info_plist_path = bundle_path.join("Contents/Info.plist");
    if !info_plist_path.exists() {
        return None;
    }

    let plist_value = Value::from_file(&info_plist_path).ok()?;
    let plist_dict = plist_value.as_dictionary()?;

    let app_name = plist_dict.get("CFBundleName").and_then(Value::as_string)?;
    let app_exec = plist_dict.get("CFBundleExecutable").and_then(Value::as_string)?;

    let exec_path = bundle_path.join("Contents/MacOS").join(app_exec);

    Some(
        App::new(app_name.to_string())
            .with_exec(exec_path.to_str().map(String::from))
            .with_icon_path(None),
    )
}

