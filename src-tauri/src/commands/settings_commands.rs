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

