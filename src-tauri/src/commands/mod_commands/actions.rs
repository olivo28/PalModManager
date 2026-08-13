use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use serde_json::Value;
use crate::db;
use crate::models::ModType;
use crate::state::AppState;
use super::utils::filter_mods_for_current_profile;
use super::scan::scan_mods_internal;

#[tauri::command]
pub async fn get_game_version(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let start = std::time::Instant::now();
    crate::logger::log("Starting get_game_version...");
    let game_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };
    if game_path.is_empty() {
        crate::logger::log("get_game_version: game_path is empty");
        return Ok(None);
    }
    let exe_path = crate::dependency_checker::get_shipping_exe_path(Path::new(&game_path));
    let fallback = PathBuf::from(&game_path).join("Palworld.exe");
    
    let result = if exe_path.exists() || fallback.exists() {
        Some("Palworld".to_string())
    } else {
        None
    };
    crate::logger::log(&format!("get_game_version completed in {:?}. Result: {:?}", start.elapsed(), result));
    Ok(result)
}

#[tauri::command]
pub fn get_mods(state: State<AppState>) -> Result<Value, String> {
    crate::logger::log("get_mods: Requesting cached mods...");
    let start = std::time::Instant::now();
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    crate::profiles::sync_current_profile_states(&mut data);
    let profile_mods = filter_mods_for_current_profile(&data);
    crate::logger::log(&format!("get_mods completed in {:?}", start.elapsed()));
    serde_json::to_value(&profile_mods).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_mods(state: State<AppState>) -> Result<Value, String> {
    crate::logger::log("scan_mods: Starting full disk scan...");
    let start_scan = std::time::Instant::now();
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path = data.settings.game_path.clone();
    let program_path = data.settings.program_path.clone();
    let current_profile_id = data.current_profile_id.clone();
    let current_profile = data.profiles.iter().find(|p| p.id == current_profile_id);
    let installed_ids = current_profile.map(|p| p.installed_mod_ids.clone()).unwrap_or_default();

    if game_path.is_empty() {
        crate::logger::log("scan_mods: game_path empty, aborting scan");
        return Ok(serde_json::json!([]));
    }

    crate::workshop::cleanup_unsubscribed_workshop_mods(&game_path, &mut data.mods);

    let merged = scan_mods_internal(&game_path, &program_path, &current_profile_id, &installed_ids, &data.mods.clone());
    data.mods = merged;
    crate::profiles::auto_add_scanned_mods_to_profile(&mut data);
    crate::profiles::cleanup_profile_mod_lists(&mut data);
    crate::profiles::sync_current_profile_states(&mut data);
    let profile_mods = filter_mods_for_current_profile(&data);
    let data_clone = data.clone();
    drop(data);

    let _ = db::save_db(&program_path, &data_clone);
    crate::logger::log(&format!("scan_mods: Full disk scan finished in total {:?}", start_scan.elapsed()));
    serde_json::to_value(&profile_mods).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_mod(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path_str = data.settings.game_path.clone();
    let current_profile_id = data.current_profile_id.clone();

    let mod_index = match data.mods.iter().position(|m| m.id == mod_id) {
        Some(idx) => idx,
        None => return Ok(serde_json::json!({ "success": true })),
    };
    
    let mod_info = data.mods[mod_index].clone();

    crate::logger::log(&format!("remove_mod: Removing mod '{}' (id: {})", mod_info.name, mod_info.id));

    let delete_path_and_sidecar = |path_str: &str| {
        if path_str.is_empty() {
            return;
        }
        let p = Path::new(path_str);
        if p.exists() {
            if p.is_dir() {
                let _ = fs::remove_dir_all(p);
            } else {
                let _ = fs::remove_file(p);
                let sidecar = std::path::PathBuf::from(format!("{}.pmm.json", path_str));
                if sidecar.exists() {
                    let _ = fs::remove_file(sidecar);
                }
            }
        } else {
            let sidecar = std::path::PathBuf::from(format!("{}.pmm.json", path_str));
            if sidecar.exists() {
                let _ = fs::remove_file(sidecar);
            }
        }
    };

    delete_path_and_sidecar(&mod_info.game_path);
    delete_path_and_sidecar(&mod_info.disabled_path);

    for extra in &mod_info.extra_files {
        delete_path_and_sidecar(extra);
    }

    if !game_path_str.is_empty() {
        let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path_str));
        let mods_txt = binaries_dir.join("ue4ss").join("Mods").join("mods.txt");
        if mods_txt.exists() {
            let folder_name = crate::profiles::get_mod_folder_name(&mod_info);
            let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &folder_name);
            let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &mod_info.name);
            
            for extra_path_str in &mod_info.extra_files {
                let extra_path_lower = extra_path_str.to_lowercase();
                if extra_path_lower.contains("ue4ss/mods/") {
                    let extra_path = Path::new(extra_path_str);
                    if let Some(extra_folder_name) = extra_path.file_name().map(|n| n.to_string_lossy().to_string()) {
                        let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &extra_folder_name);
                    }
                }
            }
        }
    }

    if mod_info.mod_type == ModType::Pak || mod_info.mod_type == ModType::LogicMods {
        if !game_path_str.is_empty() {
            let game_base = PathBuf::from(&game_path_str);
            let check_dirs = vec![
                game_base.join("Pal").join("Content").join("Paks").join("~mods"),
                game_base.join("Pal").join("Content").join("Paks").join("LogicMods"),
                PathBuf::from(&program_path).join("profiles").join(&current_profile_id).join("disabled_mods").join("pak"),
                PathBuf::from(&program_path).join("profiles").join(&current_profile_id).join("disabled_mods").join("logicmods"),
            ];
            for dir in check_dirs {
                if dir.exists() {
                    if let Ok(entries) = fs::read_dir(&dir) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.to_lowercase().starts_with(&mod_info.name.to_lowercase()) {
                                let _ = fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }
    }

    for profile in &mut data.profiles {
        profile.installed_mod_ids.retain(|id| id != &mod_info.id && id.to_lowercase() != mod_info.name.to_lowercase());
        profile.enabled_mod_ids.retain(|id| id != &mod_info.id && id.to_lowercase() != mod_info.name.to_lowercase());
        
        let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }

    data.mods.retain(|m| m.id != mod_info.id);

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);

    crate::logger::log(&format!("remove_mod: Mod '{}' successfully purged physically and from database.", mod_info.name));
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn disable_mod(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    crate::profiles::disable_mod_internal(&mut data, &program_path, &mod_id)?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn enable_mod(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    crate::profiles::enable_mod_internal(&mut data, &program_path, &mod_id)?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn disable_all_mods(state: State<AppState>) -> Result<Value, String> {
    let mod_ids: Vec<String> = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.iter()
            .filter(|m| m.enabled && m.nexus_author.as_deref() != Some("UE4SS Native Mod"))
            .map(|m| m.id.clone())
            .collect()
    };
    let mut disabled_count = 0u32;
    for mod_id in mod_ids {
        if disable_mod(mod_id, state.clone()).is_ok() {
            disabled_count += 1;
        }
    }
    Ok(serde_json::json!({ "success": true, "disabled": disabled_count }))
}

#[tauri::command]
pub fn enable_all_mods(state: State<AppState>) -> Result<Value, String> {
    let mod_ids: Vec<String> = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.iter()
            .filter(|m| !m.enabled && m.nexus_author.as_deref() != Some("UE4SS Native Mod"))
            .map(|m| m.id.clone())
            .collect()
    };
    let mut enabled_count = 0u32;
    for mod_id in mod_ids {
        if enable_mod(mod_id, state.clone()).is_ok() {
            enabled_count += 1;
        }
    }
    Ok(serde_json::json!({ "success": true, "enabled": enabled_count }))
}
