use std::path::PathBuf;
use tauri::State;
use crate::altermatic::{self, AltermaticDepStatus};
use crate::state::AppState;

#[tauri::command]
pub fn sync_altermatic_load_list(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path_str = &data.settings.game_path;
    if game_path_str.is_empty() {
        return Ok(Vec::new());
    }

    let game_path = PathBuf::from(game_path_str);
    
    // Find current profile to get enabled mod IDs
    let current_profile = data.profiles
        .iter()
        .find(|p| p.id == data.current_profile_id);

    let enabled_mod_ids: Vec<String> = if let Some(p) = current_profile {
        p.enabled_mod_ids.clone()
    } else {
        data.mods.iter().filter(|m| m.enabled).map(|m| m.id.clone()).collect()
    };

    altermatic::sync_load_list(&game_path, &enabled_mod_ids, &data.mods)
}

#[tauri::command]
pub fn get_altermatic_dep_status(state: State<'_, AppState>) -> Result<AltermaticDepStatus, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path_str = &data.settings.game_path;
    if game_path_str.is_empty() {
        return Ok(AltermaticDepStatus {
            altermatic_present: false,
            unipalui_present: false,
        });
    }

    let game_path = PathBuf::from(game_path_str);
    Ok(altermatic::check_altermatic_deps(&game_path))
}
