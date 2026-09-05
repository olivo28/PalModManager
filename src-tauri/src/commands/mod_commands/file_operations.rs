use std::fs::{self, File};
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use tauri::State;
use serde_json::Value;
use crate::models::ModType;
use crate::state::AppState;

#[tauri::command]
pub fn open_folder(mod_id: String, state: State<AppState>) -> Result<(), String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;
    let path = if mod_info.enabled { &mod_info.game_path } else { &mod_info.disabled_path };

    let mut dir = Path::new(path);
    if dir.is_file() {
        if let Some(parent) = dir.parent() {
            dir = parent;
        }
    }
    if !dir.exists() {
        return Err("Directory does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let mut dir = Path::new(&path);
    if dir.is_file() {
        if let Some(parent) = dir.parent() {
            dir = parent;
        }
    }
    if !dir.exists() {
        return Err("Directory does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open path: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open path: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open path: {}", e))?;
    }

    Ok(())
}

#[allow(unused_imports)]
pub use super::metadata::{
    rename_mod, set_mod_version, set_mod_ignored_keys, check_github_version, set_github_version,
    change_pak_destination, save_mod_notes,
};


#[tauri::command]
pub fn export_mods_json(path: String, state: State<AppState>) -> Result<String, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&data.mods).map_err(|e| e.to_string())?;
    let path = PathBuf::from(&path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_profile_pack_cmd(
    profile_id: String,
    target_path: String,
    state: State<AppState>,
) -> Result<String, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    crate::profiles::export_profile_pack_internal(&data, &profile_id, &target_path)
}

#[tauri::command]
pub fn import_profile_pack_cmd(
    source_path: String,
    custom_name: Option<String>,
    state: State<AppState>,
) -> Result<crate::profiles::ImportProfileResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let result = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        crate::profiles::import_profile_pack_internal(&mut data, &source_path, custom_name)?
    };

    // Save state changes into database
    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    let _ = crate::db::save_db(&program_path, &data_clone);

    Ok(result)
}

#[tauri::command]
pub fn open_extra_folder(mod_id: String, state: State<AppState>) -> Result<(), String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;
    if mod_info.extra_files.is_empty() {
        return Err("No extra files".to_string());
    }
    let first_extra = &mod_info.extra_files[0];
    let mut dir = Path::new(first_extra);
    if dir.is_file() {
        if let Some(parent) = dir.parent() {
            dir = parent;
        }
    }
    if !dir.exists() {
        return Err("Directory does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    Ok(())
}

fn determine_category_for_path(path: &Path) -> &'static str {
    let path_lower = path.to_string_lossy().to_lowercase();
    if path_lower.contains("logicmods") {
        "LogicMods"
    } else if path_lower.contains("~mods") {
        "Paks"
    } else if path_lower.contains("palschema") {
        "PalSchema"
    } else if path_lower.contains("ue4ss") {
        "UE4SS"
    } else {
        "Paks"
    }
}

#[tauri::command]
pub async fn create_backup(target_dir: String, state: State<'_, AppState>) -> Result<String, String> {
    use zip::write::{FileOptions, ZipWriter};

    let mods = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.clone()
    };

    let mods_to_backup: Vec<_> = mods.into_iter().filter(|m| {
        if m.nexus_author.as_deref() == Some("UE4SS Native Mod") {
            return false;
        }
        let is_in_game = !m.game_path.is_empty() && Path::new(&m.game_path).exists();
        let is_disabled = !m.disabled_path.is_empty() && Path::new(&m.disabled_path).exists();
        is_in_game || is_disabled
    }).collect();

    let num_mods = mods_to_backup.len();
    let date_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let zip_name = format!("PMM_{}Mods_{}_Backup.zip", num_mods, date_str);
    let zip_path = Path::new(&target_dir).join(&zip_name);

    let zip_file = File::create(&zip_path).map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);

    for m in mods_to_backup {
        let src_path = if !m.game_path.is_empty() && Path::new(&m.game_path).exists() {
            PathBuf::from(&m.game_path)
        } else {
            PathBuf::from(&m.disabled_path)
        };

        let category_folder = match m.mod_type {
            ModType::Ue4ss | ModType::Hybrid => "UE4SS",
            ModType::PalSchema => "PalSchema",
            ModType::Pak => "Paks",
            ModType::LogicMods => "LogicMods",
            ModType::Altermatic => "Altermatic",
        };

        let mut paths_to_backup = vec![(src_path, category_folder.to_string())];
        for extra_str in &m.extra_files {
            let extra_path = PathBuf::from(extra_str);
            if extra_path.exists() {
                let cat = determine_category_for_path(&extra_path).to_string();
                paths_to_backup.push((extra_path, cat));
            }
        }

        for (path, cat) in paths_to_backup {
            if path.is_dir() {
                for entry in walkdir::WalkDir::new(&path) {
                    let entry = entry.map_err(|e| format!("Walkdir error: {}", e))?;
                    let p = entry.path();
                    if p.is_file() {
                        let relative_path = p.strip_prefix(&path.parent().unwrap())
                            .map_err(|e| format!("Failed to strip prefix: {}", e))?;
                        let zip_entry_name = format!("{}/{}", cat, relative_path.to_string_lossy().replace('\\', "/"));
                        
                        let _ = zip.start_file(&zip_entry_name, options);
                        if let Ok(mut file) = File::open(p) {
                            let mut buffer = Vec::new();
                            if file.read_to_end(&mut buffer).is_ok() {
                                let _ = zip.write_all(&buffer);
                            }
                        }
                    }
                }
            } else if path.is_file() {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                let zip_entry_name = format!("{}/{}", cat, filename);
                let _ = zip.start_file(&zip_entry_name, options);
                
                if let Ok(mut file) = File::open(&path) {
                    let mut buffer = Vec::new();
                    if file.read_to_end(&mut buffer).is_ok() {
                        let _ = zip.write_all(&buffer);
                    }
                }

                if path.extension().map(|ext| ext == "pak").unwrap_or(false) {
                    let companions = crate::zip_handler::find_pak_companions(&path);
                    for companion in companions {
                        if companion != path {
                            let comp_filename = companion.file_name().unwrap().to_string_lossy().to_string();
                            let zip_entry_name = format!("{}/{}", cat, comp_filename);
                            let _ = zip.start_file(&zip_entry_name, options);
                            
                            if let Ok(mut file) = File::open(&companion) {
                                  let mut buffer = Vec::new();
                                  if file.read_to_end(&mut buffer).is_ok() {
                                      let _ = zip.write_all(&buffer);
                                  }
                            }
                        }
                    }
                }
            }
        }
    }

    zip.finish().map_err(|e| format!("Failed to finalize zip: {}", e))?;
    Ok(zip_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn restore_backup(zip_path: String, state: State<'_, AppState>) -> Result<(), String> {
    use zip::read::ZipArchive;

    let game_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };
    if game_path.is_empty() {
        return Err("Game path not set".to_string());
    }
    let game = Path::new(&game_path);
    let binaries = crate::dependency_checker::get_binaries_dir(game);

    let file = File::open(&zip_path).map_err(|e| format!("Failed to open backup zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid backup zip: {}", e))?;

    let mut has_ue4ss = false;
    let mut has_palschema = false;

    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name();
            if name.starts_with("UE4SS/") {
                has_ue4ss = true;
            }
            if name.starts_with("PalSchema/") {
                has_palschema = true;
            }
        }
    }

    if has_ue4ss {
        let dwmapi = binaries.join("dwmapi.dll");
        if !dwmapi.exists() {
            return Err("UE4SS is not installed. The backup contains UE4SS mods which require UE4SS to operate. Please install UE4SS first.".to_string());
        }
    }

    if has_palschema {
        let ps_dll = binaries.join("ue4ss").join("Mods").join("PalSchema").join("dlls").join("main.dll");
        if !ps_dll.exists() {
            return Err("PalSchema is not installed. The backup contains PalSchema mods which require PalSchema to operate. Please install PalSchema first.".to_string());
        }
    }

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let name = entry.name().to_string();
        if entry.is_dir() || name.contains("..") {
            continue;
        }

        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() < 2 {
            continue;
        }
        let category = parts[0];
        let subpath = parts[1..].join("/");

        let dest_path = match category {
            "UE4SS" => {
                binaries.join("ue4ss").join("Mods").join(&subpath)
            }
            "PalSchema" => {
                binaries.join("ue4ss").join("Mods").join("PalSchema").join("mods").join(&subpath)
            }
            "Paks" => {
                game.join("Pal").join("Content").join("Paks").join("~mods").join(&subpath)
            }
            "LogicMods" => {
                game.join("Pal").join("Content").join("Paks").join("LogicMods").join(&subpath)
            }
            _ => continue,
        };

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create folder: {}", e))?;
        }

        let mut outfile = File::create(&dest_path).map_err(|e| format!("Failed to create file {}: {}", dest_path.display(), e))?;
        let mut buffer = Vec::new();
        entry.read_to_end(&mut buffer).map_err(|e| format!("Failed to read file from zip: {}", e))?;
        std::io::Write::write_all(&mut outfile, &buffer).map_err(|e| format!("Failed to write file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn analyze_backup(zip_path: String) -> Result<Value, String> {
    use zip::read::ZipArchive;

    let file = File::open(&zip_path).map_err(|e| format!("Failed to open backup zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid backup zip: {}", e))?;

    let mut has_ue4ss = false;
    let mut has_palschema = false;
    let mut has_paks = false;

    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name();
            if name.starts_with("UE4SS/") {
                has_ue4ss = true;
            }
            if name.starts_with("PalSchema/") {
                has_palschema = true;
            }
            if name.starts_with("Paks/") || name.starts_with("LogicMods/") {
                has_paks = true;
            }
        }
    }

    Ok(serde_json::json!({
        "hasUe4ss": has_ue4ss,
        "hasPalSchema": has_palschema,
        "hasPaks": has_paks,
    }))
}

#[tauri::command]
pub fn open_folder_by_type(folder_type: String, state: State<'_, AppState>) -> Result<(), String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path_str = &data.settings.game_path;
    let program_path = &data.settings.program_path;
    let current_profile_id = &data.current_profile_id;

    let path = match folder_type.as_str() {
        "ue4ss" => {
            if game_path_str.is_empty() { return Err("Game path not configured".to_string()); }
            let game = std::path::Path::new(game_path_str);
            crate::dependency_checker::get_ue4ss_mods_dir(&game)
        }
        "palschema" => {
            if game_path_str.is_empty() { return Err("Game path not configured".to_string()); }
            let game = std::path::Path::new(game_path_str);
            crate::dependency_checker::get_ue4ss_mods_dir(&game).join("PalSchema").join("mods")
        }
        "paks" => {
            if game_path_str.is_empty() { return Err("Game path not configured".to_string()); }
            let game = std::path::Path::new(game_path_str);
            game.join("Pal").join("Content").join("Paks")
        }
        "app_data" => {
            std::path::PathBuf::from(program_path)
        }
        "profile" => {
            std::path::PathBuf::from(program_path).join("profiles").join(current_profile_id)
        }
        _ => return Err("Unknown folder type".to_string()),
    };

    if !path.exists() {
        if folder_type == "app_data" || folder_type == "profile" {
            let _ = std::fs::create_dir_all(&path);
        } else {
            return Err(format!("Folder does not exist: {}", path.display()));
        }
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    Ok(())
}



