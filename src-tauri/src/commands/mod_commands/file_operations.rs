use std::fs::{self, File};
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use tauri::State;
use serde_json::Value;
use crate::models::{ModInfo, ModType};
use crate::state::AppState;
use crate::db;

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

    let current_path = if mod_info.enabled { &mod_info.game_path } else { &mod_info.disabled_path };
    let current_dir = Path::new(current_path);

    let parent = current_dir.parent().ok_or_else(|| "Cannot rename root directory".to_string())?;
    let new_dir = parent.join(&new_name);

    if current_dir != new_dir {
        fs::rename(current_dir, &new_dir)
            .map_err(|e| format!("Failed to rename directory: {}", e))?;

        if mod_info.enabled {
            mod_info.game_path = new_dir.to_string_lossy().to_string();
        } else {
            mod_info.disabled_path = new_dir.to_string_lossy().to_string();
        }
    }

    mod_info.name = new_name;
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
    mod_info.version = version;
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
        .user_agent("PalModManager/1.0")
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

            for ext in &["pak", "ucas", "utoc", "pak.pmm.json"] {
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

            for ext in &["pak", "ucas", "utoc", "pak.pmm.json"] {
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
