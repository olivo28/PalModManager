use std::path::Path;
use tauri::State;
use serde_json::Value;
use crate::models::{ModInfo, ModType};
use crate::state::AppState;
use crate::db;

#[tauri::command]
pub fn rename_mod(mod_id: String, new_name: String, state: State<AppState>) -> Result<ModInfo, String> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() || new_name.len() > 200 {
        return Err("Invalid name".to_string());
    }
    if new_name.contains('/') || new_name.contains('\\') || new_name.contains('\0') {
        return Err("Name contains invalid characters".to_string());
    }

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let mod_info = data.mods.iter_mut().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;

    if mod_info.original_name.is_none() {
        mod_info.original_name = Some(mod_info.name.clone());
    }
    mod_info.custom_name = Some(new_name.clone());
    mod_info.name = new_name;
    let _ = crate::profiles::save_pmm_meta(mod_info);
    let result = mod_info.clone();
    let data_clone = data.clone();
    drop(data);
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn set_mod_version(mod_id: String, version: String, state: State<AppState>) -> Result<ModInfo, String> {
    let version = version.trim().to_string();
    if version.is_empty() || version.len() > 50 {
        return Err("Invalid version".to_string());
    }
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter_mut().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;
    mod_info.version = version.clone();

    // If version is now equal to or newer than cached nexus or github version, clear has_pending_update
    if let Some(ref nexus_ver) = mod_info.nexus_version_cached {
        let is_still_newer = crate::commands::nexus_commands::is_version_newer(&version, nexus_ver);
        if !is_still_newer {
            mod_info.has_pending_update = Some(false);
        }
    }

    if let Some(ref gh_ver) = mod_info.github_version {
        let is_still_newer = crate::commands::nexus_commands::is_version_newer(&version, gh_ver);
        if !is_still_newer {
            mod_info.has_pending_update = Some(false);
        }
    }

    let _ = crate::profiles::save_pmm_meta(mod_info);
    let result = mod_info.clone();
    let program_path = data.settings.program_path.clone();
    let data_clone = data.clone();
    drop(data);
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn set_mod_ignored_keys(
    mod_id: String,
    ignored_keys: Vec<String>,
    state: State<AppState>,
) -> Result<ModInfo, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter_mut().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;
    mod_info.ignored_keys = Some(ignored_keys);
    let result = mod_info.clone();
    let program_path = data.settings.program_path.clone();
    
    let _ = crate::profiles::save_pmm_meta(&result);

    let data_clone = data.clone();
    drop(data);
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub async fn check_github_version(repo: String) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let resp = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }
    let json: Value = resp.json().await
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;
    let tag = json["tag_name"].as_str()
        .ok_or_else(|| "No tag_name in response".to_string())?;
    Ok(tag.to_string())
}

#[tauri::command]
pub fn set_github_version(mod_id: String, repo: String, version: String, state: State<AppState>) -> Result<ModInfo, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter_mut().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;
    mod_info.github_repo = Some(repo);
    mod_info.github_version = Some(version);
    mod_info.github_cached_at = Some(chrono::Utc::now().to_rfc3339());
    let result = mod_info.clone();
    let program_path = data.settings.program_path.clone();
    let data_clone = data.clone();
    drop(data);
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn change_pak_destination(
    mod_id: String,
    destination: String,
    state: State<AppState>,
) -> Result<ModInfo, String> {
    if destination != "~mods" && destination != "LogicMods" {
        return Err("Invalid destination. Must be '~mods' or 'LogicMods'.".to_string());
    }

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path_base = data.settings.game_path.clone();

    let mod_index = data.mods.iter().position(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;
    
    let mut mod_info = data.mods[mod_index].clone();
    if mod_info.mod_type != ModType::Pak && mod_info.mod_type != ModType::LogicMods {
        return Err("Mod is not a Pak mod".to_string());
    }

    let old_dest = mod_info.pak_destination.clone().unwrap_or_else(|| {
        if mod_info.mod_type == ModType::LogicMods {
            "LogicMods".to_string()
        } else {
            "~mods".to_string()
        }
    });

    if old_dest == destination {
        return Ok(mod_info);
    }

    let new_type = if destination == "LogicMods" {
        ModType::LogicMods
    } else {
        ModType::Pak
    };

    if mod_info.enabled {
        let paks_dir = std::path::PathBuf::from(&game_path_base)
            .join("Pal")
            .join("Content")
            .join("Paks");
        
        let old_dir = paks_dir.join(&old_dest);
        let new_dir = paks_dir.join(&destination);
        let _ = std::fs::create_dir_all(&new_dir);

        let file_path = Path::new(&mod_info.game_path);
        if file_path.exists() {
            let file_stem = file_path.file_stem().unwrap().to_string_lossy().to_string();
            let mut new_game_path = String::new();

            for ext in &["pak", "ucas", "utoc"] {
                let old_file = old_dir.join(format!("{}.{}", file_stem, ext));
                if old_file.exists() {
                    let new_file = new_dir.join(format!("{}.{}", file_stem, ext));
                    std::fs::rename(&old_file, &new_file)
                        .map_err(|e| format!("Failed to move file {:?}: {}", old_file, e))?;
                    if ext == &"pak" {
                        new_game_path = new_file.to_string_lossy().to_string();
                    }
                }
            }
            let old_sidecar = old_dir.join(format!("{}.pak.pmm.json", file_stem));
            if old_sidecar.exists() {
                let new_sidecar = new_dir.join(format!("{}.pak.pmm.json", file_stem));
                let _ = std::fs::rename(&old_sidecar, &new_sidecar);
            }
            if !new_game_path.is_empty() {
                mod_info.game_path = new_game_path;
            }
        }
    } else {
        let profile_dir = std::path::PathBuf::from(&program_path)
            .join("profiles")
            .join(&data.current_profile_id);
        let disabled_base = profile_dir.join("disabled_mods");

        let old_type_dir = if old_dest == "LogicMods" { "logicmods" } else { "pak" };
        let new_type_dir = if destination == "LogicMods" { "logicmods" } else { "pak" };

        let old_dir = disabled_base.join(old_type_dir);
        let new_dir = disabled_base.join(new_type_dir);
        let _ = std::fs::create_dir_all(&new_dir);

        let file_path = Path::new(&mod_info.disabled_path);
        if file_path.exists() {
            let file_stem = file_path.file_stem().unwrap().to_string_lossy().to_string();
            let mut new_disabled_path = String::new();
            let mut new_extra_files = Vec::new();

            for ext in &["pak", "ucas", "utoc"] {
                let old_file = old_dir.join(format!("{}.{}", file_stem, ext));
                if old_file.exists() {
                    let new_file = new_dir.join(format!("{}.{}", file_stem, ext));
                    std::fs::rename(&old_file, &new_file)
                        .map_err(|e| format!("Failed to move disabled file {:?}: {}", old_file, e))?;
                    if ext == &"pak" {
                        new_disabled_path = new_file.to_string_lossy().to_string();
                    } else {
                        new_extra_files.push(new_file.to_string_lossy().to_string());
                    }
                }
            }
            let old_sidecar = old_dir.join(format!("{}.pak.pmm.json", file_stem));
            if old_sidecar.exists() {
                let new_sidecar = new_dir.join(format!("{}.pak.pmm.json", file_stem));
                let _ = std::fs::rename(&old_sidecar, &new_sidecar);
            }
            if !new_disabled_path.is_empty() {
                mod_info.disabled_path = new_disabled_path;
                mod_info.extra_files = new_extra_files;
            }
        }
    }

    mod_info.mod_type = new_type;
    mod_info.pak_destination = Some(destination);

    data.mods[mod_index] = mod_info.clone();
    let data_clone = data.clone();
    drop(data);
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(mod_info)
}

#[tauri::command]
pub fn save_mod_notes(mod_id: String, notes: String, state: State<AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();

    let mod_index = data.mods.iter().position(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;

    let trimmed = notes.trim();
    let notes_val = if trimmed.is_empty() {
        None
    } else {
        Some(notes.clone())
    };

    data.mods[mod_index].custom_notes = notes_val;
    let mod_info = data.mods[mod_index].clone();

    // Persist to .pmm.json sidecar file
    let _ = crate::profiles::save_pmm_meta(&mod_info);

    let data_clone = data.clone();
    drop(data);

    let _ = db::save_db(&program_path, &data_clone);
    crate::logger::log(&format!("Saved custom notes for mod '{}' (length: {} chars)", mod_id, notes.len()));

    Ok(())
}
