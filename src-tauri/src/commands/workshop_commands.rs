use crate::models::{WorkshopMod, WorkshopState};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_workshop_mods(state: State<'_, AppState>) -> Result<Vec<WorkshopMod>, String> {
    let game_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };
    if game_path.is_empty() {
        return Ok(Vec::new());
    }
    Ok(crate::workshop::scan_workshop_mods(&game_path))
}

#[tauri::command]
pub fn get_workshop_state(state: State<'_, AppState>) -> Result<WorkshopState, String> {
    let game_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };
    if game_path.is_empty() {
        return Ok(WorkshopState::default());
    }

    let settings = crate::workshop::read_pal_mod_settings(&game_path);
    let scanned = crate::workshop::scan_workshop_mods(&game_path);

    Ok(WorkshopState {
        workshop_root: settings.workshop_root,
        global_enabled: settings.global_enabled,
        active_mod_list: settings.active_mod_list,
        mods: scanned,
    })
}

#[tauri::command]
pub fn activate_workshop_mod_cmd(package_name: String, state: State<'_, AppState>) -> Result<(), String> {
    let (game_path, force_load_order_ue4ss) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.force_load_order_ue4ss.unwrap_or(false))
    };
    if game_path.is_empty() {
        return Err("Game path not set".to_string());
    }

    let wmods = crate::workshop::scan_workshop_mods(&game_path);
    let target = wmods.iter().find(|m| m.package_name == package_name)
        .ok_or_else(|| format!("Workshop mod {} not found", package_name))?;

    crate::workshop::activate_workshop_mod(&game_path, target, force_load_order_ue4ss)
}

#[tauri::command]
pub fn deactivate_workshop_mod_cmd(package_name: String, state: State<'_, AppState>) -> Result<(), String> {
    let (game_path, force_load_order_ue4ss) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.force_load_order_ue4ss.unwrap_or(false))
    };
    if game_path.is_empty() {
        return Err("Game path not set".to_string());
    }

    let wmods = crate::workshop::scan_workshop_mods(&game_path);
    let target = wmods.iter().find(|m| m.package_name == package_name)
        .ok_or_else(|| format!("Workshop mod {} not found", package_name))?;

    crate::workshop::deactivate_workshop_mod(&game_path, target, force_load_order_ue4ss)
}

#[tauri::command]
pub fn set_workshop_global_enabled(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let game_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };
    if game_path.is_empty() {
        return Err("Game path not set".to_string());
    }

    let mut settings = crate::workshop::read_pal_mod_settings(&game_path);
    settings.global_enabled = enabled;
    crate::workshop::write_pal_mod_settings(&game_path, &settings)
}
