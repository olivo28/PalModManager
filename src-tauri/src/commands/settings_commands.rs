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

#[tauri::command]
pub fn set_nexus_api_key(api_key: Option<String>, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings.nexus_api_key = if let Some(key) = &api_key {
        if key.trim().is_empty() { None } else { Some(key.clone()) }
    } else {
        None
    };
    let result = serde_json::to_value(&data.settings).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
    Ok(result)
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
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
