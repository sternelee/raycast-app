use std::fs;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use tauri::Manager;
use zip::result::ZipError;
use zip::ZipArchive;

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HeuristicViolation {
    command_name: String,
    reason: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum InstallResult {
    Success,
    RequiresConfirmation { violations: Vec<HeuristicViolation> },
}

trait IncompatibilityHeuristic {
    fn check(&self, command_title: &str, file_content: &str) -> Option<HeuristicViolation>;
}

struct AppleScriptHeuristic;
impl IncompatibilityHeuristic for AppleScriptHeuristic {
    fn check(&self, command_title: &str, file_content: &str) -> Option<HeuristicViolation> {
        if file_content.contains("runAppleScript") {
            Some(HeuristicViolation {
                command_name: command_title.to_string(),
                reason: "Possible usage of AppleScript (runAppleScript)".to_string(),
            })
        } else {
            None
        }
    }
}

fn get_extension_dir(app: &tauri::AppHandle, slug: &str) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Failed to get app local data dir".to_string())?;
    Ok(data_dir.join("plugins").join(slug))
}

async fn download_archive(url: &str) -> Result<bytes::Bytes, String> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download extension: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download extension: status code {}",
            response.status()
        ));
    }

    response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response bytes: {}", e))
}

fn find_common_prefix(file_names: &[PathBuf]) -> Option<PathBuf> {
    if file_names.len() <= 1 {
        return None;
    }
    file_names
        .get(0)
        .and_then(|p| p.components().next())
        .and_then(|first_component| {
            if file_names
                .iter()
                .all(|path| path.starts_with(first_component))
            {
                Some(PathBuf::from(first_component.as_os_str()))
            } else {
                None
            }
        })
}

fn get_commands_from_package_json(
    archive: &mut ZipArchive<Cursor<bytes::Bytes>>,
    prefix: &Option<PathBuf>,
) -> Result<Vec<(String, String)>, String> {
    let package_json_path = if let Some(ref p) = prefix {
        p.join("package.json")
    } else {
        PathBuf::from("package.json")
    };

    let mut pkg_file = match archive.by_name(&package_json_path.to_string_lossy()) {
        Ok(file) => file,
        Err(ZipError::FileNotFound) => return Ok(vec![]),
        Err(e) => return Err(e.to_string()),
    };

    let mut pkg_str = String::new();
    pkg_file
        .read_to_string(&mut pkg_str)
        .map_err(|e| e.to_string())?;

    let pkg_json: serde_json::Value =
        serde_json::from_str(&pkg_str).map_err(|_| "Failed to parse package.json".to_string())?;

    let commands = match pkg_json.get("commands").and_then(|c| c.as_array()) {
        Some(cmds) => cmds,
        None => return Ok(vec![]),
    };

    Ok(commands
        .iter()
        .filter_map(|command| {
            let command_name = command.get("name")?.as_str()?;
            let command_title = command
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or(command_name)
                .to_string();

            let src_path = format!("{}.js", command_name);
            let command_file_path_in_archive = if let Some(ref p) = prefix {
                p.join(src_path)
            } else {
                PathBuf::from(src_path)
            };

            Some((
                command_file_path_in_archive.to_string_lossy().into_owned(),
                command_title,
            ))
        })
        .collect())
}

fn run_heuristic_checks(archive_data: &bytes::Bytes) -> Result<Vec<HeuristicViolation>, String> {
    let heuristics: Vec<Box<dyn IncompatibilityHeuristic + Send + Sync>> =
        vec![Box::new(AppleScriptHeuristic)];
    if heuristics.is_empty() {
        return Ok(vec![]);
    }

    let mut archive =
        ZipArchive::new(Cursor::new(archive_data.clone())).map_err(|e| e.to_string())?;
    let file_names: Vec<PathBuf> = archive.file_names().map(PathBuf::from).collect();
    let prefix = find_common_prefix(&file_names);

    let commands_to_check = get_commands_from_package_json(&mut archive, &prefix)?;
    let mut violations = Vec::new();

    for (path_in_archive, command_title) in commands_to_check {
        if let Ok(mut command_file) = archive.by_name(&path_in_archive) {
            let mut content = String::new();
            if command_file.read_to_string(&mut content).is_ok() {
                for heuristic in &heuristics {
                    if let Some(violation) = heuristic.check(&command_title, &content) {
                        violations.push(violation);
                    }
                }
            }
        }
    }
    Ok(violations)
}

fn extract_archive(archive_data: &bytes::Bytes, target_dir: &Path) -> Result<(), String> {
    if target_dir.exists() {
        fs::remove_dir_all(target_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(target_dir).map_err(|e| e.to_string())?;

    let mut archive =
        ZipArchive::new(Cursor::new(archive_data.clone())).map_err(|e| e.to_string())?;
    let file_names: Vec<PathBuf> = archive.file_names().map(PathBuf::from).collect();
    let prefix_to_strip = find_common_prefix(&file_names);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let enclosed_path = match file.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => continue,
        };

        let final_path_part = if let Some(ref prefix) = prefix_to_strip {
            enclosed_path
                .strip_prefix(prefix)
                .unwrap_or(&enclosed_path)
                .to_path_buf()
        } else {
            enclosed_path
        };

        if final_path_part.as_os_str().is_empty() {
            continue;
        }

        let outpath = target_dir.join(final_path_part);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn install_extension(
    app: tauri::AppHandle,
    download_url: String,
    slug: String,
    force: bool,
) -> Result<InstallResult, String> {
    let extension_dir = get_extension_dir(&app, &slug)?;
    let content = download_archive(&download_url).await?;

    if !force {
        let violations = run_heuristic_checks(&content)?;
        if !violations.is_empty() {
            return Ok(InstallResult::RequiresConfirmation { violations });
        }
    }

    extract_archive(&content, &extension_dir)?;

    Ok(InstallResult::Success)
}
