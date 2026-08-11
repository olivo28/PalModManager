use crate::db;
use crate::models::Profile;
use crate::profiles;
use crate::state::AppState;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn get_profiles(state: State<AppState>) -> Result<Vec<Profile>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    Ok(data.profiles.clone())
}

#[tauri::command]
pub fn get_current_profile(state: State<AppState>) -> Result<Profile, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let profile_id = &data.current_profile_id;
    data.profiles
        .iter()
        .find(|p| p.id == *profile_id)
        .cloned()
        .ok_or_else(|| "Current profile not found".to_string())
}

#[tauri::command]
pub fn switch_profile_command(
    profile_id: String,
    state: State<AppState>,
) -> Result<Value, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let target_profile = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or_else(|| "Profile not found".to_string())?
    };

    // 1. Switch profile (backup current, restore target game files)
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        profiles::switch_profile(&mut data, &program_path, &target_profile)?;
    }

    // 2. Re-scan disk to get the actual mods present for the new profile
    let (game_path, db_mods, current_profile_id, installed_ids) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let current_profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
        let ids = current_profile.map(|p| p.installed_mod_ids.clone()).unwrap_or_default();
        (data.settings.game_path.clone(), data.mods.clone(), data.current_profile_id.clone(), ids)
    };

    let fresh_mods = crate::commands::mod_commands::scan_mods_internal(&game_path, &program_path, &current_profile_id, &installed_ids, &db_mods);

    // 3. Apply the fresh scan into state and sync profile
    let profile_mods = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods = fresh_mods;
        profiles::cleanup_profile_mod_lists(&mut data);
        profiles::sync_current_profile_states(&mut data);
        // Return ONLY mods that belong to the target profile
        crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data)
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    serde_json::to_value(&profile_mods).map_err(|e| e.to_string())
}


#[tauri::command]
pub fn create_profile_command(name: String, state: State<AppState>) -> Result<Profile, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let profile = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        profiles::create_profile(&mut data, name)?
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub fn clone_profile_command(
    profile_id: String,
    new_name: String,
    state: State<AppState>,
) -> Result<Profile, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let profile = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        profiles::clone_profile(&mut data, &profile_id, new_name)?
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub fn delete_profile_command(profile_id: String, state: State<AppState>) -> Result<Value, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        profiles::delete_profile(&mut data, &profile_id)?;
    }

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn rename_profile_command(
    profile_id: String,
    name: String,
    state: State<AppState>,
) -> Result<Profile, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        profiles::rename_profile(&mut data, &profile_id, name)?;
    }

    let (profile, data_clone) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let p = data
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or_else(|| "Profile not found".to_string())?;
        (p, data.clone())
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub fn set_mod_profile_state(
    mod_id: String,
    enabled: bool,
    state: State<AppState>,
) -> Result<Value, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let profile_id = data.current_profile_id.clone();
        profiles::set_profile_mod_state(&mut data, &profile_id, &mod_id, enabled)?;
    }

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn create_mod_folder_command(
    profile_id: String,
    name: String,
    state: State<AppState>,
) -> Result<Profile, String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err("Invalid folder name".to_string());
    }

    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let profile = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let p_idx = data.profiles.iter().position(|p| p.id == profile_id)
            .ok_or_else(|| "Profile not found".to_string())?;
        
        let folder_id = crate::profiles::sanitize_profile_id(&name);
        if data.profiles[p_idx].mod_folders.iter().any(|f| f.id == folder_id) {
            return Err("A folder with a similar name already exists".to_string());
        }

        let new_folder = crate::models::ModFolder {
            id: folder_id,
            name,
            mod_ids: Vec::new(),
        };

        data.profiles[p_idx].mod_folders.push(new_folder);
        
        let p_dir = crate::profiles::get_profile_dir(&program_path, &profile_id);
        if let Ok(json) = serde_json::to_string_pretty(&data.profiles[p_idx]) {
            let _ = std::fs::write(p_dir.join("profile.json"), json);
        }
        data.profiles[p_idx].clone()
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub fn delete_mod_folder_command(
    profile_id: String,
    folder_id: String,
    state: State<AppState>,
) -> Result<Profile, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let profile = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let p_idx = data.profiles.iter().position(|p| p.id == profile_id)
            .ok_or_else(|| "Profile not found".to_string())?;

        data.profiles[p_idx].mod_folders.retain(|f| f.id != folder_id);

        let p_dir = crate::profiles::get_profile_dir(&program_path, &profile_id);
        if let Ok(json) = serde_json::to_string_pretty(&data.profiles[p_idx]) {
            let _ = std::fs::write(p_dir.join("profile.json"), json);
        }
        data.profiles[p_idx].clone()
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub fn rename_mod_folder_command(
    profile_id: String,
    folder_id: String,
    new_name: String,
    state: State<AppState>,
) -> Result<Profile, String> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() || new_name.len() > 100 {
        return Err("Invalid folder name".to_string());
    }

    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let profile = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let p_idx = data.profiles.iter().position(|p| p.id == profile_id)
            .ok_or_else(|| "Profile not found".to_string())?;

        let f_idx = data.profiles[p_idx].mod_folders.iter().position(|f| f.id == folder_id)
            .ok_or_else(|| "Folder not found".to_string())?;

        data.profiles[p_idx].mod_folders[f_idx].name = new_name;

        let p_dir = crate::profiles::get_profile_dir(&program_path, &profile_id);
        if let Ok(json) = serde_json::to_string_pretty(&data.profiles[p_idx]) {
            let _ = std::fs::write(p_dir.join("profile.json"), json);
        }
        data.profiles[p_idx].clone()
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub fn add_mod_to_folder_command(
    profile_id: String,
    folder_id: Option<String>,
    mod_id: String,
    state: State<AppState>,
) -> Result<Profile, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let profile = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let p_idx = data.profiles.iter().position(|p| p.id == profile_id)
            .ok_or_else(|| "Profile not found".to_string())?;

        for folder in &mut data.profiles[p_idx].mod_folders {
            folder.mod_ids.retain(|id| id != &mod_id);
        }

        if let Some(fid) = folder_id {
            if let Some(folder) = data.profiles[p_idx].mod_folders.iter_mut().find(|f| f.id == fid) {
                folder.mod_ids.push(mod_id);
            }
        }

        let p_dir = crate::profiles::get_profile_dir(&program_path, &profile_id);
        if let Ok(json) = serde_json::to_string_pretty(&data.profiles[p_idx]) {
            let _ = std::fs::write(p_dir.join("profile.json"), json);
        }
        data.profiles[p_idx].clone()
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub fn toggle_folder_mods_command(
    profile_id: String,
    folder_id: String,
    enabled: bool,
    state: State<AppState>,
) -> Result<Profile, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let profile = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        
        let p_idx = data.profiles.iter().position(|p| p.id == profile_id)
            .ok_or_else(|| "Profile not found".to_string())?;
        
        let folder = data.profiles[p_idx].mod_folders.iter().find(|f| f.id == folder_id)
            .ok_or_else(|| "Folder not found".to_string())?.clone();

        for mod_id in &folder.mod_ids {
            let _ = crate::profiles::set_profile_mod_state(&mut data, &profile_id, mod_id, enabled);
        }

        data.profiles[p_idx].clone()
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub fn clear_profile_command(
    profile_id: String,
    state: State<AppState>,
) -> Result<Vec<Profile>, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    
    // 1. Wipe database mod listing and backing physical files
    profiles::clear_profile(&mut data, &profile_id)?;

    // 2. Re-scan and clean up states
    let game_path = data.settings.game_path.clone();
    let db_mods = data.mods.clone();
    let current_profile_id = data.current_profile_id.clone();
    let current_profile = data.profiles.iter().find(|p| p.id == current_profile_id);
    let installed_ids = current_profile.map(|p| p.installed_mod_ids.clone()).unwrap_or_default();

    let fresh_mods = crate::commands::mod_commands::scan_mods_internal(&game_path, &program_path, &current_profile_id, &installed_ids, &db_mods);
    data.mods = fresh_mods;
    
    profiles::cleanup_profile_mod_lists(&mut data);
    profiles::sync_current_profile_states(&mut data);

    // 3. Save database
    db::save_db(&program_path, &data).map_err(|e| e.to_string())?;

    Ok(data.profiles.clone())
}

