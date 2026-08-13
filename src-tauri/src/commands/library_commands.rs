use crate::library::{self, LibraryEntry};
use crate::state::AppState;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn get_library(state: State<AppState>) -> Result<Vec<LibraryEntry>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let installed_mods = data.mods.clone();
    drop(data);
    library::list_library(&program_path, &installed_mods)
}

#[tauri::command]
pub fn install_mod_from_library(
    mod_id: String,
    state: State<AppState>,
) -> Result<Value, String> {
    let (program_path, game_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.settings.game_path.clone())
    };

    if game_path.is_empty() {
        return Err("No game path configured. Set it first.".to_string());
    }

    let lib_path = library::get_library_path(&program_path, &mod_id);
    let zip_file = {
        let mut found = None;
        if lib_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&lib_path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file()
                        && path
                            .extension()
                            .map(|e| e == "zip" || e == "rar")
                            .unwrap_or(false)
                    {
                        found = Some(path.to_string_lossy().to_string());
                        break;
                    }
                }
            }
        }
        found.ok_or_else(|| "No ZIP file found in library for this mod".to_string())?
    };

    // Re-run the install using existing install command logic
    let analysis = crate::zip_handler::analyze_zip(&zip_file)?;

    let temp_dir =
        std::env::temp_dir().join(format!("palmodmanager_lib_{}", uuid::Uuid::new_v4()));
    let extracted = crate::zip_handler::extract_zip_to_temp(&zip_file, &temp_dir)?;

    let zip_filename = std::path::Path::new(&zip_file)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let (force_load_order_ue4ss, force_load_order_palschema) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_ue4ss(&data),
            data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_palschema(&data)
        )
    };

    let mod_info = crate::installer::install_mod(
        &game_path, &extracted, &analysis, &zip_filename,
        None, None, None, None, None, None, None, None,
        None, None, None, Vec::new(),
        force_load_order_ue4ss,
        force_load_order_palschema,
    )?;

    let _ = std::fs::remove_dir_all(&temp_dir);

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.push(mod_info.clone());

        // Register the mod in the current profile's installed and enabled lists
        // Use mod name (stable across scans) instead of UUID (changes if mod is re-scanned)
        if mod_info.nexus_author.as_deref() != Some("UE4SS Native Mod") {
            let current_profile_id = data.current_profile_id.clone();
            let mod_name = mod_info.name.clone();
            if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
                // Always add to installed_mod_ids
                let in_installed = profile.installed_mod_ids.iter().any(|id| id.to_lowercase() == mod_name.to_lowercase());
                if !in_installed {
                    profile.installed_mod_ids.push(mod_name.clone());
                }
                // Add to enabled_mod_ids only if mod is enabled
                if mod_info.enabled {
                    let in_enabled = profile.enabled_mod_ids.iter().any(|id| id.to_lowercase() == mod_name.to_lowercase());
                    if !in_enabled {
                        profile.enabled_mod_ids.push(mod_name.clone());
                    }
                }
            }
            // Persist the updated profile.json
            let p_dir = crate::profiles::get_profile_dir(&program_path, &data.current_profile_id);
            if let Some(profile) = data.profiles.iter().find(|p| p.id == data.current_profile_id) {
                if let Ok(json) = serde_json::to_string_pretty(profile) {
                    let _ = std::fs::write(p_dir.join("profile.json"), json);
                }
            }
        }



        let data_clone = data.clone();
        drop(data);
        crate::db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;
    }

    serde_json::to_value(&mod_info).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_from_library(
    mod_id: String,
    zip_name: Option<String>,
    state: tauri::State<AppState>,
) -> Result<Value, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };
    library::remove_from_library(&program_path, &mod_id, zip_name.as_deref())?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn get_library_zip_path(mod_id: String, zip_name: Option<String>, state: State<AppState>) -> Result<String, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    let lib_path = library::get_library_path(&program_path, &mod_id);
    if lib_path.exists() {
        if let Some(ref specific_zip) = zip_name {
            let target = lib_path.join(specific_zip);
            if target.exists() {
                return Ok(target.to_string_lossy().to_string());
            }
        }
        if let Ok(entries) = std::fs::read_dir(&lib_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name != ".nexus.json" && !name.ends_with(".pmm.json") {
                        return Ok(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    Err("ZIP file not found in library".to_string())
}

#[tauri::command]
pub async fn copy_to_library_command(
    zip_path: String,
    mod_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let (program_path, installed_mods) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.mods.clone())
    };

    let filename = std::path::Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // 1. Analyze the zip archive for internal metadata or structure
    let analysis = crate::zip_handler::analyze_zip(&zip_path).ok();

    // 2. Extract version & clean stem from filename
    let parsed_info = crate::nexus::parse_mod_filename(&filename);
    let mut detected_version = parsed_info.version;
    let mut detected_name = parsed_info.name;
    let mut detected_nexus_id = parsed_info.nexus_id
        .or_else(|| crate::zip_handler::extract_nexus_id_from_path(&zip_path))
        .or_else(|| crate::nexus::extract_nexus_id(&filename));

    let mut internal_author = None;
    let mut internal_desc = None;
    let mut internal_type = None;

    if let Some(ref ana) = analysis {
        if ana.has_info_json {
            let info_file_path = ana.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
                .or_else(|| ana.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
                .or_else(|| ana.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
            if let Some(target_file) = info_file_path {
                if let Some(content) = crate::zip_handler::read_archive_file(&zip_path, target_file) {
                    if let Ok(val) = serde_json::from_str::<Value>(&content) {
                        if let Some(n) = val.get("name").or_else(|| val.get("folderName")).and_then(|v| v.as_str()) {
                            detected_name = Some(n.to_string());
                        }
                        if let Some(v) = val.get("version").and_then(|v| v.as_str()) {
                            detected_version = Some(v.to_string());
                        }
                        if let Some(a) = val.get("nexusAuthor").or_else(|| val.get("author")).and_then(|v| v.as_str()) {
                            internal_author = Some(a.to_string());
                        }
                        if let Some(d) = val.get("nexusSummary").or_else(|| val.get("description")).and_then(|v| v.as_str()) {
                            internal_desc = Some(d.to_string());
                        }
                        if let Some(id) = val.get("nexusModId").and_then(|v| v.as_u64()) {
                            detected_nexus_id = Some(id as u32);
                        }
                    }
                }
            }
        }
        internal_type = Some(format!("{:?}", ana.detected_type).to_lowercase());
    }

    // 3. Clean stem
    let clean_stem = detected_name.unwrap_or_else(|| {
        let cleaned = crate::installer::clean_zip_name(&filename);
        if cleaned.is_empty() || cleaned == "unknown" {
            std::path::Path::new(&filename).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| filename.clone())
        } else {
            cleaned
        }
    });

    let normalize = |s: &str| -> String {
        s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
    };
    let norm_clean = normalize(&clean_stem);

    // 4. Find matching existing mod in installed mods or existing folders in mods-library
    let mut matched_folder = None;
    let mut matched_nexus_info = None;

    if let Some(m) = installed_mods.iter().find(|m| {
        normalize(&m.name) == norm_clean || normalize(&m.id) == norm_clean || (detected_nexus_id.is_some() && m.nexus_mod_id == detected_nexus_id)
    }) {
        matched_folder = Some(m.name.clone());
        if detected_nexus_id.is_none() {
            detected_nexus_id = m.nexus_mod_id;
        }
        if internal_author.is_none() {
            internal_author = m.nexus_author.clone();
        }
        if internal_desc.is_none() {
            internal_desc = m.nexus_summary.clone();
        }
    }

    let lib_dir = library::library_dir(&program_path);
    if matched_folder.is_none() && lib_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&lib_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.file_type().map_or(false, |t| t.is_dir()) {
                    let d_name = entry.file_name().to_string_lossy().to_string();
                    if normalize(&d_name) == norm_clean {
                        matched_folder = Some(d_name.clone());
                        let nexus_path = entry.path().join(".nexus.json");
                        if nexus_path.exists() {
                            if let Ok(c) = std::fs::read_to_string(&nexus_path) {
                                if let Ok(val) = serde_json::from_str::<Value>(&c) {
                                    if detected_nexus_id.is_none() {
                                        detected_nexus_id = val.get("modId").and_then(|v| v.as_u64()).map(|v| v as u32);
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    // 5. If Nexus ID is found, fetch fresh Nexus metadata
    if let Some(mod_id) = detected_nexus_id {
        if let Ok(info) = crate::nexus::fetch_mod_info(mod_id).await {
            matched_nexus_info = Some(info);
        }
    }

    let folder_name = mod_name
        .or_else(|| matched_nexus_info.as_ref().map(|i| i.name.clone()))
        .or(matched_folder)
        .unwrap_or(clean_stem);

    let mut entry = library::copy_to_library(&zip_path, &program_path, &folder_name)?;

    if let Some(ref info) = matched_nexus_info {
        let lib_path = library::get_library_path(&program_path, &folder_name);
        let json_path = lib_path.join(".nexus.json");
        let json_data = serde_json::json!({
            "modId": info.mod_id,
            "name": info.name,
            "author": info.author,
            "summary": info.summary,
            "version": info.version,
            "pictureUrl": info.picture_url,
        });
        let _ = std::fs::write(&json_path, json_data.to_string());

        entry.nexus_picture_url = Some(info.picture_url.clone());
        entry.nexus_name = Some(info.name.clone());
        entry.nexus_author = Some(info.author.clone());
        entry.nexus_summary = Some(info.summary.clone());
        entry.nexus_mod_id = Some(info.mod_id);
        entry.nexus_version = Some(info.version.clone());
        entry.author = Some(info.author.clone());
        entry.description = Some(info.summary.clone());
    } else {
        entry.author = internal_author;
        entry.description = internal_desc;
    }

    entry.version = detected_version.or(entry.nexus_version.clone());
    entry.mod_type = internal_type;

    // Save matching .pmm.json sidecar for this specific zip
    let lib_zip_path = library::get_library_path(&program_path, &folder_name).join(&filename);
    let pmm_sidecar = std::path::PathBuf::from(format!("{}.pmm.json", lib_zip_path.to_string_lossy()));
    let pmm_json = serde_json::json!({
        "name": entry.nexus_name.as_deref().unwrap_or(&folder_name),
        "author": entry.author.as_deref().or(entry.nexus_author.as_deref()).unwrap_or(""),
        "description": entry.description.as_deref().or(entry.nexus_summary.as_deref()).unwrap_or(""),
        "version": entry.version.as_deref().or(entry.nexus_version.as_deref()).unwrap_or(""),
        "modType": entry.mod_type.as_deref().unwrap_or(""),
        "nexusModId": entry.nexus_mod_id,
        "nexusPictureUrl": entry.nexus_picture_url.as_deref().unwrap_or(""),
        "zipName": filename,
    });
    let _ = std::fs::write(&pmm_sidecar, serde_json::to_string_pretty(&pmm_json).unwrap_or_default());

    Ok(serde_json::to_value(&entry).map_err(|e| e.to_string())?)
}

