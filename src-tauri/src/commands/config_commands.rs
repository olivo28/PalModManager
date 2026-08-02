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
        base_dir.join(custom)
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
        base_dir.join(custom)
    } else {
        let found = find_json_config(&base_dir).or_else(|| find_lua_config(&base_dir));
        match found {
            Some(p) => p,
            None => return Err("No config file found for this mod".to_string()),
        }
    };

    if config_path.exists() {
        let ext = config_path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        let bak_path = config_path.with_extension(format!("{}.bak", ext));
        let _ = fs::copy(&config_path, &bak_path);
    }

    fs::write(&config_path, &content).map_err(|e| format!("Cannot write config: {}", e))?;

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn set_mod_config(mod_id: String, config_path: Option<String>, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_index = data.mods.iter().position(|m| m.id == mod_id).ok_or("Mod not found")?;
    data.mods[mod_index].config_path = config_path;
    data.mods[mod_index].config_type = Some("manual".to_string());
    let result = serde_json::to_value(&data.mods[mod_index]).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    Ok(result)
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

fn get_full_mod_file_path(mod_info: &crate::models::ModInfo, file_path: &str) -> Result<PathBuf, String> {
    if mod_info.mod_type == crate::models::ModType::Hybrid {
        let path_obj = Path::new(file_path);
        let components: Vec<&str> = path_obj.iter().map(|c| c.to_str().unwrap_or_default()).collect();
        if !components.is_empty() {
            let prefix = components[0];
            let relative_part = path_obj.strip_prefix(prefix)
                .map_err(|e| format!("Failed to strip prefix: {}", e))?;

            // 1. Check UE4SS base
            let base_path1 = get_mod_base_dir(mod_info);
            let folder_name1 = base_path1.file_name().unwrap_or_default().to_string_lossy().to_string();
            if prefix == folder_name1 {
                return Ok(base_path1.join(relative_part));
            }

            // 2. Check PalSchema base
            if let Some(extra_str) = mod_info.extra_files.first() {
                let base_path2 = PathBuf::from(extra_str);
                let folder_name2 = base_path2.file_name().unwrap_or_default().to_string_lossy().to_string();
                if prefix == folder_name2 {
                    return Ok(base_path2.join(relative_part));
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

    if mod_info.mod_type == crate::models::ModType::Hybrid {
        // 1. Get UE4SS base path
        let base_path1 = get_mod_base_dir(mod_info);
        if base_path1.exists() && base_path1.is_dir() {
            let folder_name1 = base_path1.file_name().unwrap_or_default().to_string_lossy().to_string();
            let mut files1 = Vec::new();
            walk_dir(&base_path1, &mut files1, &base_path1).map_err(|e| e.to_string())?;
            for f in files1 {
                files.push(format!("{}/{}", folder_name1, f));
            }
        }

        // 2. Get PalSchema base path
        if let Some(extra_str) = mod_info.extra_files.first() {
            let base_path2 = PathBuf::from(extra_str);
            if base_path2.exists() && base_path2.is_dir() {
                let folder_name2 = base_path2.file_name().unwrap_or_default().to_string_lossy().to_string();
                let mut files2 = Vec::new();
                walk_dir(&base_path2, &mut files2, &base_path2).map_err(|e| e.to_string())?;
                for f in files2 {
                    files.push(format!("{}/{}", folder_name2, f));
                }
            }
        }
    } else {
        let base_path = get_mod_base_dir(mod_info);
        walk_dir(&base_path, &mut files, &base_path).map_err(|e| e.to_string())?;
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
                files.push(relative.to_string_lossy().to_string());
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
        }));
    }

    let content = fs::read_to_string(&full_path).map_err(|e| format!("Cannot read file: {}", e))?;

    Ok(serde_json::json!({
        "content": content,
        "path": file_path,
        "configType": ext,
    }))
}

#[tauri::command]
pub fn save_mod_file(mod_id: String, file_path: String, content: String, state: State<AppState>) -> Result<Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();

    let mod_index = data.mods.iter().position(|m| m.id == mod_id).ok_or("Mod not found")?;
    let mod_info = &data.mods[mod_index];

    let full_path = get_full_mod_file_path(mod_info, &file_path)?;

    if full_path.exists() {
        let bak_path = full_path.with_extension("bak");
        let _ = fs::copy(&full_path, &bak_path);
    }

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create directories: {}", e))?;
    }

    fs::write(&full_path, &content).map_err(|e| format!("Cannot write file: {}", e))?;

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);

    Ok(serde_json::json!({ "success": true }))
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
