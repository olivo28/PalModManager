use crate::db;
use crate::state::AppState;
use serde_json::Value;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(&data.settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_game_path(path: String, state: State<AppState>) -> Result<Value, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }
    let has_paks = path_buf.join("Pal/Content/Paks").exists();
    let has_binaries = path_buf.join("Pal/Binaries").exists();
    let has_logic_mods = path_buf.join("Pal/Content/Paks/LogicMods").exists();
    if !has_paks && !has_binaries && !has_logic_mods {
        return Err("Path does not look like a Palworld installation (missing Pal/Content/Paks and Pal/Binaries)".to_string());
    }

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings.game_path = path;
    let result = serde_json::to_value(&data.settings).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    Ok(result)
}

use std::path::Path;
use std::fs;

#[tauri::command]
pub fn set_force_load_order(enabled: bool, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings.force_load_order = Some(enabled);
    
    let result = serde_json::to_value(&data.settings).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    return Ok(result);
}

#[tauri::command]
pub fn set_force_load_order_ue4ss(enabled: bool, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings.force_load_order_ue4ss = Some(enabled);
    
    let game_path = data.settings.game_path.clone();
    if !game_path.is_empty() {
        let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
        let mods_txt = binaries_dir.join("ue4ss").join("Mods").join("mods.txt");
        
        if mods_txt.exists() {
            let current_profile_id = data.current_profile_id.clone();
            if let Some(current_profile) = data.profiles.iter().find(|p| p.id == current_profile_id).cloned() {
                let target_mods: Vec<crate::models::ModInfo> = data.mods.iter()
                    .filter(|m| {
                        current_profile.installed_mod_ids.contains(&m.id)
                        && !m.game_path.is_empty()
                        && (m.mod_type == crate::models::ModType::Ue4ss || m.mod_type == crate::models::ModType::Hybrid)
                        && m.nexus_author.as_deref() != Some("UE4SS Native Mod")
                    })
                    .cloned()
                    .collect();
                    
                if enabled {
                    let mut reconciled_items: Vec<(String, bool)> = Vec::new();
                    if let Some(ref metadata) = current_profile.load_order_metadata {
                        for (folder_name, is_enabled) in metadata {
                            if target_mods.iter().any(|m| crate::profiles::get_mod_folder_name(m).to_lowercase() == folder_name.to_lowercase()) {
                                reconciled_items.push((folder_name.clone(), *is_enabled));
                            }
                        }
                    }
                    for m in &target_mods {
                        let folder_name = crate::profiles::get_mod_folder_name(m);
                        if !reconciled_items.iter().any(|(f, _)| f.to_lowercase() == folder_name.to_lowercase()) {
                            reconciled_items.push((folder_name, true));
                        }
                    }
                    if let Ok(content) = fs::read_to_string(&mods_txt) {
                        let ordered_folder_names: Vec<String> = reconciled_items.iter().map(|(f, _)| f.clone()).collect();
                        let mut custom_lines = Vec::new();
                        for (folder_name, is_enabled) in &reconciled_items {
                            let val = if *is_enabled { "1" } else { "0" };
                            custom_lines.push(format!("{} : {}", folder_name, val));
                        }
                        let mut lines_to_keep = Vec::new();
                        for line in content.lines() {
                            let line_clean = line.trim();
                            if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
                                let name = if let Some(pos) = line_clean.find(':') {
                                    line_clean[..pos].trim()
                                } else {
                                    line_clean
                                };
                                if ordered_folder_names.iter().any(|f| f.to_lowercase() == name.to_lowercase()) 
                                   || target_mods.iter().any(|m| m.name.to_lowercase() == name.to_lowercase()) {
                                    continue;
                                }
                            }
                            lines_to_keep.push(line.to_string());
                        }
                        let mut insert_index = None;
                        for (idx, line) in lines_to_keep.iter().enumerate() {
                            let line_clean = line.trim();
                            if line_clean.contains("BPModLoaderMod") {
                                insert_index = Some(idx + 1);
                            }
                        }
                        if insert_index.is_none() {
                            for (idx, line) in lines_to_keep.iter().enumerate() {
                                let line_clean = line.trim();
                                if line_clean.contains("; Built-in keybinds") {
                                    insert_index = Some(idx);
                                }
                            }
                        }
                        let final_idx = insert_index.unwrap_or(lines_to_keep.len());
                        for (offset, custom_line) in custom_lines.into_iter().enumerate() {
                            lines_to_keep.insert(final_idx + offset, custom_line);
                        }
                        let _ = fs::write(&mods_txt, lines_to_keep.join("\r\n") + "\r\n");
                    }
                } else {
                    if let Ok(content) = fs::read_to_string(&mods_txt) {
                        let mut enabled_in_mods_txt = std::collections::HashSet::new();
                        for line in content.lines() {
                            let line_clean = line.trim();
                            if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
                                if let Some(pos) = line_clean.find(':') {
                                    let name = line_clean[..pos].trim().to_string();
                                    let e_str = line_clean[pos+1..].trim();
                                    if e_str != "0" {
                                        enabled_in_mods_txt.insert(name.to_lowercase());
                                    }
                                } else {
                                    enabled_in_mods_txt.insert(line_clean.to_lowercase());
                                }
                            }
                        }
                        let mut lines_to_keep = Vec::new();
                        let target_folder_names: Vec<String> = target_mods.iter().map(|m| crate::profiles::get_mod_folder_name(m)).collect();
                        for line in content.lines() {
                            let line_clean = line.trim();
                            if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
                                let name = if let Some(pos) = line_clean.find(':') {
                                    line_clean[..pos].trim()
                                } else {
                                    line_clean
                                };
                                if target_folder_names.iter().any(|f| f.to_lowercase() == name.to_lowercase())
                                   || target_mods.iter().any(|m| m.name.to_lowercase() == name.to_lowercase()) {
                                    continue;
                                }
                            }
                            lines_to_keep.push(line.to_string());
                        }
                        let _ = fs::write(&mods_txt, lines_to_keep.join("\r\n") + "\r\n");
                        for m in &target_mods {
                            let folder_name = crate::profiles::get_mod_folder_name(m);
                            let was_enabled = enabled_in_mods_txt.contains(&folder_name.to_lowercase()) || !content.contains(&folder_name);
                            let enabled_file = Path::new(&m.game_path).join("enabled.txt");
                            if was_enabled {
                                if !enabled_file.exists() {
                                    let _ = fs::write(&enabled_file, "");
                                }
                            } else {
                                if enabled_file.exists() {
                                    let _ = fs::remove_file(&enabled_file);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let result = serde_json::to_value(&data.settings).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    Ok(result)
}

#[tauri::command]
pub fn set_force_load_order_palschema(enabled: bool, state: State<AppState>) -> Result<Value, String> {
    #[cfg(not(target_os = "windows"))]
    {
        return Err("NTFS Junction Load Order for PalSchema is only supported on Windows".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.force_load_order_palschema = Some(enabled);
        
        let game_path = data.settings.game_path.clone();
        if !game_path.is_empty() {
            let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
            let palschema_mods_dir = binaries_dir.join("ue4ss").join("Mods").join("PalSchema").join("mods");
            let palschema_storage_dir = binaries_dir.join("ue4ss").join("Mods").join("PalSchema").join("Storage");
            let current_profile_id = data.current_profile_id.clone();
            let (_palschema_mod_ids, enabled_mod_ids) = if let Some(current_profile) = data.profiles.iter().find(|p| p.id == current_profile_id) {
                (current_profile.installed_mod_ids.clone(), current_profile.enabled_mod_ids.clone())
            } else {
                (Vec::new(), Vec::new())
            };
            let mut current_mods_links = Vec::new();
            if palschema_mods_dir.exists() {
                if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let name = path.file_name().unwrap().to_string_lossy().to_string();
                        current_mods_links.push((path, name));
                    }
                }
            }

            if !enabled {
                for (path, _) in &current_mods_links {
                    let _ = crate::profiles::remove_junction_or_symlink(path);
                }
                if palschema_storage_dir.exists() {
                    if let Ok(entries) = fs::read_dir(&palschema_storage_dir) {
                        for entry in entries.flatten() {
                            let src_path = entry.path();
                            if src_path.is_dir() {
                                let folder_name = src_path.file_name().unwrap().to_string_lossy().to_string();
                                let dest_path = palschema_mods_dir.join(&folder_name);
                                let _ = fs::create_dir_all(&palschema_mods_dir);
                                let _ = crate::profiles::move_path(&src_path, &dest_path);
                            }
                        }
                    }
                    let _ = fs::remove_dir(&palschema_storage_dir);
                }
            for mod_info in &mut data.mods {
                if mod_info.mod_type == crate::models::ModType::PalSchema || mod_info.mod_type == crate::models::ModType::Hybrid {
                    let folder_name = crate::profiles::get_mod_folder_name(mod_info);
                    let direct_path = palschema_mods_dir.join(&folder_name);
                    if direct_path.exists() {
                        mod_info.game_path = direct_path.to_string_lossy().to_string();
                    }
                }
            }
            } else {
                let mut idx = 0;
                for mod_info in &mut data.mods {
                    if mod_info.mod_type == crate::models::ModType::PalSchema || mod_info.mod_type == crate::models::ModType::Hybrid {
                        let folder_name = crate::profiles::get_mod_folder_name(mod_info);
                        let is_enabled = enabled_mod_ids.iter().any(|id| id.to_lowercase() == mod_info.id.to_lowercase() || id.to_lowercase() == mod_info.name.to_lowercase() || id.to_lowercase() == folder_name.to_lowercase());
                        let storage_dest = palschema_storage_dir.join(&folder_name);
                        let has_palschema_folder = storage_dest.exists() || current_mods_links.iter().any(|(_, name)| name == &folder_name || (name.len() > 4 && &name[4..] == &folder_name));
                        if !has_palschema_folder && mod_info.mod_type == crate::models::ModType::Hybrid {
                            continue;
                        }
                        if is_enabled {
                            let mut physical_src = None;
                            for (path, name) in &current_mods_links {
                                if (name == &folder_name || (name.len() > 4 && &name[4..] == &folder_name)) && !junction::exists(path).unwrap_or(false) && path.is_dir() {
                                    physical_src = Some(path.clone());
                                }
                            }
                            if let Some(src) = physical_src {
                                let _ = fs::create_dir_all(&palschema_storage_dir);
                                let _ = crate::profiles::move_path(&src, &storage_dest);
                            }
                            for (path, name) in &current_mods_links {
                                if name == &folder_name || (name.len() > 4 && &name[4..] == &folder_name) {
                                    let _ = crate::profiles::remove_junction_or_symlink(path);
                                }
                            }
                            if storage_dest.exists() {
                                let link_name = format!("{:03}_{}", idx, folder_name);
                                let link_path = palschema_mods_dir.join(&link_name);
                                let _ = fs::create_dir_all(&palschema_mods_dir);
                                let _ = crate::profiles::create_junction_or_symlink(&storage_dest, &link_path);
                                if mod_info.mod_type == crate::models::ModType::PalSchema || mod_info.mod_type == crate::models::ModType::Hybrid {
                                    mod_info.game_path = link_path.to_string_lossy().to_string();
                                }
                                mod_info.mods_txt_order = Some(idx);
                                idx += 1;
                            }
                        }
                    }
                }
            }
        }
        let result = serde_json::to_value(&data.settings).map_err(|e| e.to_string())?;
        let data_clone = data.clone();
        drop(data);
        let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
        Ok(result)
    }
}

#[tauri::command]
pub fn set_hide_native_mods(hide: bool, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings.hide_native_mods = Some(hide);
    let result = serde_json::to_value(&data.settings).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    Ok(result)
}

#[tauri::command]
pub fn set_debug_console(enabled: bool, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings.debug_console = Some(enabled);
    let result = serde_json::to_value(&data.settings).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    crate::logger::set_console_visibility(enabled);
    Ok(result)
}

#[tauri::command]
pub fn log_from_js(msg: String) {
    crate::logger::log(&msg);
}


#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", &url])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn get_default_program_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let program_path = std::env::var("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("PalModManager");

    #[cfg(not(target_os = "windows"))]
    let program_path = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local").join("share"))
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("PalModManager");
        
    program_path
}

#[tauri::command]
pub fn set_custom_data_path(path: Option<String>, state: State<AppState>) -> Result<Value, String> {
    let current_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let default_path = get_default_program_path();
    let target_path = match &path {
        Some(p) if p == "__portable__" => {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
                .ok_or_else(|| "Failed to get executable directory".to_string())?
        }
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => default_path.clone(),
    };

    if target_path.to_string_lossy().to_string() == current_path {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        return serde_json::to_value(&data.settings).map_err(|e| e.to_string());
    }

    let _ = std::fs::create_dir_all(&target_path);

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings.custom_data_path = path.clone();
    
    let items_to_migrate = &["profiles", "mods-library"];
    for item in items_to_migrate {
        let src = PathBuf::from(&current_path).join(item);
        let dst = target_path.join(item);
        if src.exists() && !dst.exists() {
            let _ = crate::profiles::copy_dir_all(&src, &dst);
        }
    }

    let mut target_db_data = data.clone();
    target_db_data.settings.program_path = target_path.to_string_lossy().to_string();
    let _ = db::save_db(&target_db_data.settings.program_path, &target_db_data);

    let default_db_path = default_path.to_string_lossy().to_string();
    let mut default_db_data = db::load_db(&default_db_path);
    default_db_data.settings.custom_data_path = path;
    let _ = db::save_db(&default_db_path, &default_db_data);

    *data = target_db_data;

    serde_json::to_value(&data.settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_toolbar_scale(scale: f64, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings.toolbar_scale = Some(scale);
    let result = serde_json::to_value(&data.settings).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    Ok(result)
}

