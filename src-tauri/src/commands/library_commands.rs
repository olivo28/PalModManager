use crate::library::{self, LibraryEntry};
use crate::state::AppState;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn get_library(state: State<AppState>) -> Result<Vec<LibraryEntry>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);
    library::list_library(&program_path)
}

#[tauri::command]
pub fn install_mod_from_library(
    mod_id: String,
    state: State<AppState>,
) -> Result<Value, String> {
    let (program_path, game_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.settings.game_path.clone())
    };

    if game_path.is_empty() {
        return Err("No game path configured. Set it first.".to_string());
    }

    let lib_path = library::get_library_path(&program_path, &mod_id);
    let zip_file = {
        let mut found = None;
        if lib_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&lib_path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file()
                        && path
                            .extension()
                            .map(|e| e == "zip" || e == "rar")
                            .unwrap_or(false)
                    {
                        found = Some(path.to_string_lossy().to_string());
                        break;
                    }
                }
            }
        }
        found.ok_or_else(|| "No ZIP file found in library for this mod".to_string())?
    };

    // Re-run the install using existing install command logic
    let analysis = crate::zip_handler::analyze_zip(&zip_file)?;

    let temp_dir =
        std::env::temp_dir().join(format!("palmodmanager_lib_{}", uuid::Uuid::new_v4()));
    let extracted = crate::zip_handler::extract_zip_to_temp(&zip_file, &temp_dir)?;

    let zip_filename = std::path::Path::new(&zip_file)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mod_info = crate::installer::install_mod(
        &game_path, &extracted, &analysis, &zip_filename,
        None, None, None, None, None, None, None, None,
        None, None, None, Vec::new(),
    )?;

    let _ = std::fs::remove_dir_all(&temp_dir);

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.push(mod_info.clone());

        // Register the mod in the current profile's installed and enabled lists
        // Use mod name (stable across scans) instead of UUID (changes if mod is re-scanned)
        if mod_info.nexus_author.as_deref() != Some("UE4SS Native Mod") {
            let current_profile_id = data.current_profile_id.clone();
            let mod_name = mod_info.name.clone();
            if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
                // Always add to installed_mod_ids
                let in_installed = profile.installed_mod_ids.iter().any(|id| id.to_lowercase() == mod_name.to_lowercase());
                if !in_installed {
                    profile.installed_mod_ids.push(mod_name.clone());
                }
                // Add to enabled_mod_ids only if mod is enabled
                if mod_info.enabled {
                    let in_enabled = profile.enabled_mod_ids.iter().any(|id| id.to_lowercase() == mod_name.to_lowercase());
                    if !in_enabled {
                        profile.enabled_mod_ids.push(mod_name.clone());
                    }
                }
            }
            // Persist the updated profile.json
            let p_dir = crate::profiles::get_profile_dir(&program_path, &data.current_profile_id);
            if let Some(profile) = data.profiles.iter().find(|p| p.id == data.current_profile_id) {
                if let Ok(json) = serde_json::to_string_pretty(profile) {
                    let _ = std::fs::write(p_dir.join("profile.json"), json);
                }
            }
        }



        let data_clone = data.clone();
        drop(data);
        crate::db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;
    }

    serde_json::to_value(&mod_info).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_from_library(
    mod_id: String,
    zip_name: Option<String>,
    state: tauri::State<AppState>,
) -> Result<Value, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };
    library::remove_from_library(&program_path, &mod_id, zip_name.as_deref())?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn get_library_zip_path(mod_id: String, state: State<AppState>) -> Result<String, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    let lib_path = library::get_library_path(&program_path, &mod_id);
    if lib_path.exists() {
        if let Ok(entries) = std::fs::read_dir(&lib_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name != ".nexus.json" {
                        return Ok(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    Err("ZIP file not found in library".to_string())
}

#[tauri::command]
pub async fn copy_to_library_command(
    zip_path: String,
    mod_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let filename = std::path::Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let folder_name = mod_name.unwrap_or_else(|| {
        let stem = std::path::Path::new(&filename)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| filename.clone());
        stem
    });

    let mut entry = library::copy_to_library(&zip_path, &program_path, &folder_name)?;

    let nexus_id = crate::zip_handler::extract_nexus_id_from_path(&zip_path)
        .or_else(|| crate::nexus::extract_nexus_id(&filename));

    if let Some(mod_id) = nexus_id {
        if let Ok(info) = crate::nexus::fetch_mod_info(mod_id).await {
            let lib_path = library::get_library_path(&program_path, &folder_name);
            let json_path = lib_path.join(".nexus.json");
            let json_data = serde_json::json!({
                "modId": info.mod_id,
                "name": info.name,
                "author": info.author,
                "summary": info.summary,
                "version": info.version,
                "pictureUrl": info.picture_url,
            });
            let _ = std::fs::write(&json_path, json_data.to_string());

            entry.nexus_picture_url = Some(info.picture_url);
            entry.nexus_name = Some(info.name);
            entry.nexus_author = Some(info.author);
            entry.nexus_summary = Some(info.summary);
            entry.nexus_mod_id = Some(info.mod_id);
            entry.nexus_version = Some(info.version);
        }
    }

    Ok(serde_json::to_value(&entry).map_err(|e| e.to_string())?)
}

