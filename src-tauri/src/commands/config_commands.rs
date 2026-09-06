use crate::db;
use crate::state::AppState;
use base64::Engine;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

#[tauri::command]
pub fn read_config(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;

    let mod_info = data.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;

    let base_dir = if mod_info.enabled {
        PathBuf::from(&mod_info.game_path)
    } else {
        PathBuf::from(&mod_info.disabled_path)
    };

    let config_path = if let Some(ref custom) = mod_info.config_path {
        get_full_mod_file_path(mod_info, custom).unwrap_or_else(|_| base_dir.join(custom))
    } else {
        let found = find_json_config(&base_dir).or_else(|| find_lua_config(&base_dir));
        match found {
            Some(p) => p,
            None => return Ok(serde_json::json!({ "content": null, "path": null, "configType": null })),
        }
    };

    if !config_path.exists() {
        return Ok(serde_json::json!({ "content": null, "path": config_path.to_string_lossy(), "configType": null }));
    }

    let content = fs::read_to_string(&config_path).map_err(|e| format!("Cannot read config: {}", e))?;
    let ext = config_path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();

    Ok(serde_json::json!({
        "content": content,
        "path": config_path.to_string_lossy(),
        "configType": ext,
    }))
}

#[tauri::command]
pub fn save_config(mod_id: String, content: String, state: State<AppState>) -> Result<Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();

    let mod_index = data.mods.iter().position(|m| m.id == mod_id).ok_or("Mod not found")?;
    let mod_info = &data.mods[mod_index];

    let base_dir = if mod_info.enabled {
        PathBuf::from(&mod_info.game_path)
    } else {
        PathBuf::from(&mod_info.disabled_path)
    };

    let config_path = if let Some(ref custom) = mod_info.config_path {
        get_full_mod_file_path(mod_info, custom).unwrap_or_else(|_| base_dir.join(custom))
    } else {
        let found = find_json_config(&base_dir).or_else(|| find_lua_config(&base_dir));
        match found {
            Some(p) => p,
            None => return Err("No config file found for this mod".to_string()),
        }
    };

    create_rotated_backup(&config_path);

    fs::write(&config_path, &content).map_err(|e| format!("Cannot write config: {}", e))?;

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn set_mod_config(mod_id: String, config_path: Option<String>, state: State<AppState>) -> Result<Value, String> {
    set_mod_configs(mod_id, config_path.map(|p| vec![p]), state)
}

#[tauri::command]
pub fn set_mod_configs(mod_id: String, config_paths: Option<Vec<String>>, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_index = data.mods.iter().position(|m| m.id == mod_id).ok_or("Mod not found")?;
    let primary = config_paths.as_ref().and_then(|paths| paths.first().cloned());
    data.mods[mod_index].config_path = primary;
    data.mods[mod_index].config_paths = config_paths;
    data.mods[mod_index].config_type = Some("manual".to_string());
    let result = serde_json::to_value(&data.mods[mod_index]).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    Ok(result)
}

pub fn get_mod_shared_dir(mod_info: &crate::models::ModInfo) -> Option<PathBuf> {
    let base_dir = get_mod_base_dir(mod_info);
    let parent = base_dir.parent()?;
    let shared_root = if parent.file_name() == Some(std::ffi::OsStr::new("Mods")) {
        parent.join("shared")
    } else {
        let alt = parent.join("Mods").join("shared");
        if alt.is_dir() { alt } else { return None; }
    };

    if !shared_root.is_dir() {
        return None;
    }

    let folder_name = crate::profiles::get_mod_folder_name(mod_info);
    let clean_folder = folder_name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
    let clean_name = mod_info.name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");

    let exact = shared_root.join(&folder_name);
    if exact.is_dir() {
        return Some(exact);
    }
    let by_name = shared_root.join(&mod_info.name);
    if by_name.is_dir() {
        return Some(by_name);
    }

    if let Ok(entries) = fs::read_dir(&shared_root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let sub_name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                let clean_sub = sub_name.replace(|c: char| !c.is_alphanumeric(), "");
                if (!clean_folder.is_empty() && clean_sub == clean_folder)
                    || (!clean_name.is_empty() && clean_sub == clean_name)
                {
                    return Some(p);
                }
            }
        }
    }

    None
}

fn get_mod_base_dir(mod_info: &crate::models::ModInfo) -> PathBuf {
    let game_path = PathBuf::from(&mod_info.game_path);
    let disabled_path = PathBuf::from(&mod_info.disabled_path);

    if mod_info.enabled {
        if game_path.exists() && !mod_info.game_path.is_empty() {
            game_path
        } else if disabled_path.exists() && !mod_info.disabled_path.is_empty() {
            disabled_path
        } else {
            game_path
        }
    } else {
        if disabled_path.exists() && !mod_info.disabled_path.is_empty() {
            disabled_path
        } else if game_path.exists() && !mod_info.game_path.is_empty() {
            game_path
        } else {
            disabled_path
        }
    }
}

pub fn get_full_mod_file_path(mod_info: &crate::models::ModInfo, file_path: &str) -> Result<PathBuf, String> {
    let normalized = file_path.replace('\\', "/");

    // 1. Check if it targets shared/
    if normalized.starts_with("shared/") || normalized.starts_with("[Shared]/") {
        if let Some(shared_dir) = get_mod_shared_dir(mod_info) {
            let subfolder = shared_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
            let prefix_with_folder = format!("shared/{}/", subfolder);
            if let Some(rel) = normalized.strip_prefix(&prefix_with_folder) {
                return Ok(shared_dir.join(rel));
            } else if let Some(rel) = normalized.strip_prefix("shared/") {
                if let Some(parent) = shared_dir.parent() {
                    let direct = parent.join(rel);
                    if direct.exists() {
                        return Ok(direct);
                    }
                }
                return Ok(shared_dir.join(rel));
            } else if let Some(rel) = normalized.strip_prefix("[Shared]/") {
                return Ok(shared_dir.join(rel));
            }
        } else {
            let base_dir = get_mod_base_dir(mod_info);
            if let Some(parent) = base_dir.parent() {
                let shared_root = parent.join("shared");
                if let Some(rel) = normalized.strip_prefix("shared/") {
                    return Ok(shared_root.join(rel));
                }
            }
        }
    }

    // 2. Check if it's an explicit absolute path or matches one of config_paths
    if let Some(ref c_paths) = mod_info.config_paths {
        for cp in c_paths {
            let cp_norm = cp.replace('\\', "/");
            if cp_norm == normalized || cp_norm.ends_with(&format!("/{}", normalized)) {
                let p = PathBuf::from(cp);
                if p.is_absolute() && p.exists() {
                    return Ok(p);
                }
            }
        }
    }

    if mod_info.mod_type == crate::models::ModType::Hybrid {
        let path_obj = Path::new(file_path);
        let components: Vec<&str> = path_obj.iter().map(|c| c.to_str().unwrap_or_default()).collect();
        if !components.is_empty() {
            let prefix = components[0];
            let relative_part = path_obj.strip_prefix(prefix)
                .map_err(|e| format!("Failed to strip prefix: {}", e))?;

            // Collect all possible roots
            let mut roots = Vec::new();
            let base_path1 = get_mod_base_dir(mod_info);
            if base_path1.exists() {
                roots.push(base_path1);
            }
            for extra_str in &mod_info.extra_files {
                let base_path_extra = PathBuf::from(extra_str);
                if base_path_extra.exists() && !roots.contains(&base_path_extra) {
                    roots.push(base_path_extra);
                }
            }

            for root in roots {
                let root_str = root.to_string_lossy().to_string().replace('\\', "/").to_lowercase();
                let is_palschema = root_str.contains("/palschema/") || root.join("raw").exists() || root.join("items").exists() || root.join("blueprints").exists();
                let tag = if is_palschema { "PalSchema" } else { "UE4SS" };
                let folder_name = root.file_name().unwrap_or_default().to_string_lossy().to_string();

                if prefix == format!("[{}] {}", tag, folder_name) || prefix == folder_name {
                    return Ok(root.join(relative_part));
                }
            }
        }
        return Err("Invalid hybrid file path prefix".to_string());
    }

    let base_dir = get_mod_base_dir(mod_info);
    Ok(base_dir.join(file_path))
}

#[tauri::command]
pub fn list_mod_files(mod_id: String, state: State<AppState>) -> Result<Vec<String>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;

    let mut files = Vec::new();

    // 1. Scan shared/ directory if it exists for this mod
    if let Some(shared_dir) = get_mod_shared_dir(mod_info) {
        let subfolder = shared_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        let mut shared_files = Vec::new();
        let _ = walk_dir(&shared_dir, &mut shared_files, &shared_dir);
        for sf in shared_files {
            let entry = format!("shared/{}/{}", subfolder, sf);
            if !files.contains(&entry) {
                files.push(entry);
            }
        }
    }

    if mod_info.mod_type == crate::models::ModType::Hybrid {
        // Collect all hybrid directory roots (primary and extras)
        let mut roots = Vec::new();
        let base_path1 = get_mod_base_dir(mod_info);
        if base_path1.exists() && base_path1.is_dir() {
            roots.push(base_path1);
        }
        for extra_str in &mod_info.extra_files {
            let base_path_extra = PathBuf::from(extra_str);
            if base_path_extra.exists() && base_path_extra.is_dir() && !roots.contains(&base_path_extra) {
                roots.push(base_path_extra);
            }
        }

        for root in roots {
            let root_str = root.to_string_lossy().to_string().replace('\\', "/").to_lowercase();
            let is_palschema = root_str.contains("/palschema/") || root.join("raw").exists() || root.join("items").exists() || root.join("blueprints").exists();
            let tag = if is_palschema { "PalSchema" } else { "UE4SS" };
            let folder_name = root.file_name().unwrap_or_default().to_string_lossy().to_string();

            let mut root_files = Vec::new();
            walk_dir(&root, &mut root_files, &root).map_err(|e| e.to_string())?;
            for f in root_files {
                files.push(format!("[{}] {}/{}", tag, folder_name, f));
            }
        }
    } else {
        let base_path = get_mod_base_dir(mod_info);
        walk_dir(&base_path, &mut files, &base_path).map_err(|e| e.to_string())?;
    }

    // 2. Add custom config_paths if not already represented
    if let Some(ref c_paths) = mod_info.config_paths {
        for cp in c_paths {
            let norm = cp.replace('\\', "/");
            if !files.iter().any(|f| f.eq_ignore_ascii_case(&norm) || norm.ends_with(f.as_str())) {
                files.push(norm);
            }
        }
    }

    Ok(files)
}


fn walk_dir(dir: &Path, files: &mut Vec<String>, base: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, files, base)?;
        } else {
            if let Ok(relative) = path.strip_prefix(base) {
                files.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn read_mod_file(mod_id: String, file_path: String, state: State<AppState>) -> Result<Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;

    let full_path = get_full_mod_file_path(mod_info, &file_path)?;

    if !full_path.exists() {
        return Ok(serde_json::json!({ "content": null, "path": file_path, "configType": null }));
    }

    let ext = full_path.extension().map(|e| e.to_string_lossy().to_string().to_lowercase()).unwrap_or_default();

    let is_image = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" | "svg");

    if is_image {
        let bytes = fs::read(&full_path).map_err(|e| format!("Cannot read image file: {}", e))?;
        let mime = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            "bmp" => "image/bmp",
            _ => "image/png",
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let data_url = format!("data:{};base64,{}", mime, b64);
        return Ok(serde_json::json!({
            "content": data_url,
            "path": file_path,
            "configType": "image",
            "modifiedTime": None::<u64>,
            "fileSize": bytes.len(),
            "modVersion": mod_info.version.clone(),
        }));
    }

    let (modified_time, file_size) = match fs::metadata(&full_path) {
        Ok(meta) => {
            let mtime = meta.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64);
            (mtime, meta.len())
        }
        Err(_) => (None, 0),
    };

    let content = fs::read_to_string(&full_path).map_err(|e| format!("Cannot read file: {}", e))?;
    crate::logger::log(&format!("read_mod_file: Read '{}' for mod '{}' ({} bytes)", file_path, mod_id, content.len()));

    Ok(serde_json::json!({
        "content": content,
        "path": file_path,
        "configType": ext,
        "modifiedTime": modified_time,
        "fileSize": file_size,
        "modVersion": mod_info.version.clone(),
    }))
}

#[tauri::command]
pub fn save_mod_file(mod_id: String, file_path: String, content: String, state: State<AppState>) -> Result<Value, String> {
    crate::logger::log(&format!("save_mod_file: Saving '{}' for mod '{}' ({} bytes)", file_path, mod_id, content.len()));
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();

    let mod_index = data.mods.iter().position(|m| m.id == mod_id).ok_or("Mod not found")?;
    let mod_info = &data.mods[mod_index];

    let full_path = get_full_mod_file_path(mod_info, &file_path)?;

    create_rotated_backup(&full_path);

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create directories: {}", e))?;
    }

    fs::write(&full_path, &content).map_err(|e| format!("Cannot write file: {}", e))?;
    crate::logger::log(&format!("save_mod_file: File '{}' written successfully", full_path.display()));

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn delete_mod_file(mod_id: String, file_path: String, state: State<AppState>) -> Result<Value, String> {
    crate::logger::log(&format!("delete_mod_file: Deleting '{}' for mod '{}'", file_path, mod_id));
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;
    let full_path = get_full_mod_file_path(mod_info, &file_path)?;

    if !full_path.exists() {
        return Err("File does not exist".to_string());
    }

    if full_path.is_file() {
        fs::remove_file(&full_path).map_err(|e| format!("Cannot delete file: {}", e))?;
    } else {
        return Err("Target is not a file".to_string());
    }

    crate::logger::log(&format!("delete_mod_file: File '{}' deleted successfully", full_path.display()));
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn restore_mod_backup(mod_id: String, backup_file_path: String, state: State<AppState>) -> Result<Value, String> {
    crate::logger::log(&format!("restore_mod_backup: Restoring '{}' for mod '{}'", backup_file_path, mod_id));
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let mod_info = data.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;

    let backup_full_path = get_full_mod_file_path(mod_info, &backup_file_path)?;
    if !backup_full_path.exists() || !backup_full_path.is_file() {
        return Err("Backup file not found".to_string());
    }

    let target_rel_path = if let Some(stripped) = backup_file_path.strip_suffix(".bak") {
        stripped
    } else if let Some(stripped) = backup_file_path.strip_suffix(".bak1") {
        stripped
    } else if let Some(stripped) = backup_file_path.strip_suffix(".bak2") {
        stripped
    } else {
        return Err("Not a recognized backup file extension (.bak, .bak1, .bak2)".to_string());
    };

    let target_full_path = get_full_mod_file_path(mod_info, target_rel_path)?;

    if target_full_path.exists() {
        create_rotated_backup(&target_full_path);
    }

    let backup_content = fs::read_to_string(&backup_full_path)
        .map_err(|e| format!("Cannot read backup file: {}", e))?;

    fs::write(&target_full_path, &backup_content)
        .map_err(|e| format!("Cannot restore file: {}", e))?;

    crate::logger::log(&format!("restore_mod_backup: Restored '{}' -> '{}' successfully", backup_full_path.display(), target_full_path.display()));

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);

    Ok(serde_json::json!({
        "success": true,
        "targetPath": target_rel_path,
        "content": backup_content
    }))
}

#[tauri::command]
pub fn merge_mod_backup(mod_id: String, backup_file_path: String, state: State<AppState>) -> Result<Value, String> {
    crate::logger::log(&format!("merge_mod_backup: Merging settings from '{}' for mod '{}'", backup_file_path, mod_id));
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let mod_info = data.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;

    let backup_full_path = get_full_mod_file_path(mod_info, &backup_file_path)?;
    if !backup_full_path.exists() || !backup_full_path.is_file() {
        return Err("Backup file not found".to_string());
    }

    let target_rel_path = if let Some(stripped) = backup_file_path.strip_suffix(".bak") {
        stripped
    } else if let Some(stripped) = backup_file_path.strip_suffix(".bak1") {
        stripped
    } else if let Some(stripped) = backup_file_path.strip_suffix(".bak2") {
        stripped
    } else {
        return Err("Not a recognized backup file extension (.bak, .bak1, .bak2)".to_string());
    };

    let target_full_path = get_full_mod_file_path(mod_info, target_rel_path)?;
    if !target_full_path.exists() || !target_full_path.is_file() {
        return Err("Target active file not found to merge into".to_string());
    }

    let backup_content = fs::read_to_string(&backup_full_path)
        .map_err(|e| format!("Cannot read backup file: {}", e))?;
    let active_content = fs::read_to_string(&target_full_path)
        .map_err(|e| format!("Cannot read active file: {}", e))?;

    let ext = Path::new(target_rel_path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let empty_ignored = Vec::new();
    let ignored_slice = mod_info.ignored_keys.as_deref().unwrap_or(&empty_ignored);

    let merged_content = crate::config_merge::merge_file_contents(&backup_content, &active_content, &ext, ignored_slice)
        .ok_or_else(|| "Failed to merge configuration: unsupported file format or invalid syntax".to_string())?;

    create_rotated_backup(&target_full_path);

    fs::write(&target_full_path, &merged_content)
        .map_err(|e| format!("Cannot write merged file: {}", e))?;

    crate::logger::log(&format!("merge_mod_backup: Merged '{}' into '{}' successfully", backup_full_path.display(), target_full_path.display()));

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);

    Ok(serde_json::json!({
        "success": true,
        "targetPath": target_rel_path,
        "content": merged_content
    }))
}

fn find_json_config(dir: &Path) -> Option<PathBuf> {
    let names = ["config.json", "settings.json", "options.json", "config.jsonc", "settings.jsonc", "options.jsonc"];
    for name in &names {
        let p = dir.join(name);
        if p.exists() { return Some(p); }
    }
    let subdirs = ["config", "settings"];
    for subdir in &subdirs {
        for name in &names {
            let p = dir.join(subdir).join(name);
            if p.exists() { return Some(p); }
        }
    }
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json" || ext == "jsonc") {
                return Some(path);
            }
        }
    }
    None
}

fn find_lua_config(dir: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "lua" {
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy();
                        if name.to_lowercase().contains("config")
                            || name.to_lowercase().contains("settings")
                            || name.to_lowercase().contains("options")
                        {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Rotates backups up to 3 versions: .bak -> .bak1 -> .bak2
/// to ensure a user's original reference backup is never destroyed on intermediate saves.
fn create_rotated_backup(path: &Path) {
    if !path.exists() || !path.is_file() {
        return;
    }

    let p_str = path.to_string_lossy();
    let bak1 = PathBuf::from(format!("{}.bak", p_str));
    let bak2 = PathBuf::from(format!("{}.bak1", p_str));
    let bak3 = PathBuf::from(format!("{}.bak2", p_str));

    if bak3.exists() {
        let _ = fs::remove_file(&bak3);
    }
    if bak2.exists() {
        let _ = fs::rename(&bak2, &bak3);
    }
    if bak1.exists() {
        let _ = fs::rename(&bak1, &bak2);
    }

    let _ = fs::copy(path, &bak1);
}
