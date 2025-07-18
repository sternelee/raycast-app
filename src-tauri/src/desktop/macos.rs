use crate::app::App;
use crate::error::AppError;
use plist::Value;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

pub fn scan_and_parse_apps() -> Result<(Vec<App>, HashMap<PathBuf, SystemTime>), AppError> {
    let app_dirs = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];

    let app_files: Vec<PathBuf> = app_dirs
        .par_iter()
        .filter(|dir| dir.exists())
        .flat_map(|dir| find_app_bundles(dir))
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

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter_map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => Some('_'),
            _ if c.is_control() => None,
            _ => Some(c),
        })
        .collect()
}

fn get_icon_cache_dir() -> Result<PathBuf, AppError> {
    let cache_dir = dirs::cache_dir()
        .ok_or(AppError::DirectoryNotFound)?
        .join("dev.byteatatime.raycast/icons");
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

fn convert_icns_to_png(icns_path: &Path, png_path: &Path) -> Result<(), AppError> {
    let output = Command::new("sips")
        .arg("-s")
        .arg("format")
        .arg("png")
        .arg(icns_path)
        .arg("--out")
        .arg(png_path)
        .output()?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::ConversionError(format!(
            "sips command failed: {}",
            error_msg
        )));
    }

    Ok(())
}

fn parse_app_bundle(bundle_path: &Path) -> Option<App> {
    let info_plist_path = bundle_path.join("Contents/Info.plist");
    if !info_plist_path.exists() {
        return None;
    }

    let plist_value = Value::from_file(&info_plist_path).ok()?;
    let plist_dict = plist_value.as_dictionary()?;

    let app_name = plist_dict.get("CFBundleName").and_then(Value::as_string)?;
    let app_exec = plist_dict.get("CFBundleExecutable").and_then(Value::as_string)?;
    let app_icon_file = plist_dict.get("CFBundleIconFile").and_then(Value::as_string);

    let icon_path = app_icon_file.and_then(|icon_file_name| {
        let mut icns_path = bundle_path.join("Contents/Resources").join(icon_file_name);
        if icns_path.extension().is_none() {
            icns_path.set_extension("icns");
        }

        if !icns_path.exists() {
            return None;
        }

        let icon_cache_dir = get_icon_cache_dir().ok()?;
        let sanitized_name = sanitize_filename(app_name);
        let png_path = icon_cache_dir.join(format!("{}.png", sanitized_name));

        let should_convert = if png_path.exists() {
            let png_meta = fs::metadata(&png_path).ok()?;
            let bundle_meta = fs::metadata(bundle_path).ok()?;
            bundle_meta.modified().ok()? > png_meta.modified().ok()?
        } else {
            true
        };

        if should_convert {
            if convert_icns_to_png(&icns_path, &png_path).is_err() {
                // If conversion fails, we can't provide a valid icon path.
                return None;
            }
        }

        png_path.to_str().map(String::from)
    });

    let exec_path = bundle_path.join("Contents/MacOS").join(app_exec);

    Some(
        App::new(app_name.to_string())
            .with_exec(exec_path.to_str().map(String::from))
            .with_icon_path(icon_path),
    )
}