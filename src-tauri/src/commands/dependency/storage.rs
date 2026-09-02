use std::fs;
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsageInfo {
    pub temp_downloads_size: u64,
    pub temp_downloads_count: usize,
    pub temp_downloads_path: String,
    pub library_size: u64,
    pub library_mods_count: usize,
    pub library_zips_count: usize,
    pub library_path: String,
}

#[tauri::command]
pub fn get_storage_usage_command(state: State<'_, AppState>) -> Result<StorageUsageInfo, String> {
    let program_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.program_path.clone()
    };

    // 1. Temp downloads
    let temp_dir = std::env::temp_dir().join("PalModManager_Downloads");
    let mut temp_size = 0u64;
    let mut temp_count = 0usize;
    if temp_dir.exists() {
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        temp_size += meta.len();
                        temp_count += 1;
                    }
                }
            }
        }
    }

    // 2. Local Library
    let lib_dir = crate::library::library_dir(&program_path);
    let mut lib_size = 0u64;
    let mut lib_mods_count = 0usize;
    let mut lib_zips_count = 0usize;
    if lib_dir.exists() {
        if let Ok(mod_entries) = fs::read_dir(&lib_dir) {
            for mod_entry in mod_entries.flatten() {
                let p = mod_entry.path();
                if p.is_dir() {
                    lib_mods_count += 1;
                    if let Ok(zip_entries) = fs::read_dir(&p) {
                        for zip_entry in zip_entries.flatten() {
                            let zp = zip_entry.path();
                            if let Ok(meta) = zp.metadata() {
                                if meta.is_file() {
                                    lib_size += meta.len();
                                    if zp.extension().and_then(|e| e.to_str()).map(|ext| ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("7z") || ext.eq_ignore_ascii_case("rar")).unwrap_or(false) {
                                        lib_zips_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(StorageUsageInfo {
        temp_downloads_size: temp_size,
        temp_downloads_count: temp_count,
        temp_downloads_path: temp_dir.to_string_lossy().to_string(),
        library_size: lib_size,
        library_mods_count: lib_mods_count,
        library_zips_count: lib_zips_count,
        library_path: lib_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn clear_temp_downloads_command() -> Result<u64, String> {
    let temp_dir = std::env::temp_dir().join("PalModManager_Downloads");
    let mut freed_bytes = 0u64;
    if temp_dir.exists() {
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(meta) = p.metadata() {
                    if meta.is_file() {
                        freed_bytes += meta.len();
                        let _ = fs::remove_file(&p);
                    } else if meta.is_dir() {
                        let _ = fs::remove_dir_all(&p);
                    }
                }
            }
        }
    }
    crate::logger::log(&format!("clear_temp_downloads: Freed {} bytes", freed_bytes));
    Ok(freed_bytes)
}

#[tauri::command]
pub fn open_temp_folder_command() -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join("PalModManager_Downloads");
    let _ = fs::create_dir_all(&temp_dir);
    crate::commands::mod_commands::open_path(temp_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_library_folder_command(state: State<'_, AppState>) -> Result<(), String> {
    let program_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.program_path.clone()
    };
    let lib_dir = crate::library::library_dir(&program_path);
    let _ = fs::create_dir_all(&lib_dir);
    crate::commands::mod_commands::open_path(lib_dir.to_string_lossy().to_string())
}
