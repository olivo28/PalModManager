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
        profiles::auto_add_scanned_mods_to_profile(&mut data);
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
