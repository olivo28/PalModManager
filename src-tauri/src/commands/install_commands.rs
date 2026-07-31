use crate::db;
use crate::installer;
use crate::library;
use crate::nexus;
use crate::state::AppState;
use crate::zip_handler;
use chrono::Utc;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use uuid::Uuid;

fn check_mod_dependencies(game_path: &str, mod_type: &str, analysis: &zip_handler::ZipAnalysis) -> Result<(), String> {
    let binaries = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    
    // Check UE4SS dependency
    let ue4ss_required = match mod_type {
        "ue4ss" | "palschema" | "hybrid" => true,
        _ => false,
    };
    if ue4ss_required {
        let dwmapi = binaries.join("dwmapi.dll");
        if !dwmapi.exists() {
            return Err("UE4SS is not installed. This mod requires UE4SS to operate. Please install UE4SS first.".to_string());
        }
    }

    // Check PalSchema dependency
    let has_palschema_folder = analysis.files.iter().any(|f| f.to_lowercase().contains("palschema"));
    let palschema_required = match mod_type {
        "palschema" => true,
        "hybrid" if (analysis.has_json || has_palschema_folder) => true,
        _ => false,
    };
    if palschema_required {
        let ps_dll = binaries.join("ue4ss").join("Mods").join("PalSchema").join("dlls").join("main.dll");
        if !ps_dll.exists() {
            return Err("PalSchema is not installed. This mod requires PalSchema to operate. Please install PalSchema first.".to_string());
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn analyze_zip(zip_path: String, state: State<'_, AppState>) -> Result<Value, String> {
    println!("[INFO] Analyzing zip file: {}", zip_path);
    let _ = state; // state unused here now

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let nexus_id = zip_handler::extract_nexus_id_from_path(&zip_path)
        .or_else(|| {
            let filename = Path::new(&zip_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            nexus::extract_nexus_id(&filename)
        });

    let detected_type = match analysis.detected_type {
        zip_handler::DetectedModType::Ue4ss => "ue4ss",
        zip_handler::DetectedModType::PalSchema => "palschema",
        zip_handler::DetectedModType::Pak => "pak",
        zip_handler::DetectedModType::LogicMods => "logicmods",
        zip_handler::DetectedModType::Hybrid => "hybrid",
        zip_handler::DetectedModType::Unknown => "unknown",
    };

    let detected_version = {
        let filename = Path::new(&zip_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        nexus::parse_mod_filename(&filename).version
    };

    Ok(serde_json::json!({
        "zipPath": zip_path,
        "detectedType": detected_type,
        "hasLua": analysis.has_lua,
        "hasJson": analysis.has_json,
        "hasPak": analysis.has_pak,
        "hasInfoJson": analysis.has_info_json,
        "pakDestinationHint": analysis.pak_destination_hint,
        "rootFolder": analysis.root_folder,
        "fileCount": analysis.files.len(),
        "nexusModId": nexus_id,
        "detectedVersion": detected_version,
        "nexusInfo": null,
    }))
}

#[tauri::command]
pub async fn install_mod_command(
    zip_path: String,
    custom_type: Option<String>,
    pak_destination: Option<String>,
    custom_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    println!("[INFO] Installing mod from zip: {}", zip_path);
    let (game_path, program_path, api_key) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.program_path.clone(), data.settings.nexus_api_key.clone())
    };

    if game_path.is_empty() {
        eprintln!("[ERROR] Game path not configured.");
        return Err("No game path configured. Set it first.".to_string());
    }

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let resolved_type = custom_type.as_deref().unwrap_or(match analysis.detected_type {
        zip_handler::DetectedModType::Ue4ss => "ue4ss",
        zip_handler::DetectedModType::PalSchema => "palschema",
        zip_handler::DetectedModType::Pak => "pak",
        zip_handler::DetectedModType::LogicMods => "logicmods",
        zip_handler::DetectedModType::Hybrid => "hybrid",
        zip_handler::DetectedModType::Unknown => "unknown",
    });

    check_mod_dependencies(&game_path, resolved_type, &analysis)?;

    let nexus_id = {
        let filename = Path::new(&zip_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        nexus::extract_nexus_id(&filename)
    };

    let nexus_info = if let Some(id) = nexus_id {
        nexus::fetch_mod_info(id, api_key.as_deref()).await.ok()
    } else {
        None
    };

    let temp_dir = std::env::temp_dir().join(format!("palmodmanager_{}", Uuid::new_v4()));
    let extracted = zip_handler::extract_zip_to_temp(&zip_path, &temp_dir)?;

    let pak_dest_ref = pak_destination.as_deref();

    let zip_filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mod_info = installer::install_mod(
        &game_path,
        &extracted,
        &analysis,
        &zip_filename,
        nexus_id,
        nexus_info.as_ref().map(|i| i.name.clone()),
        nexus_info.as_ref().map(|i| i.author.clone()),
        nexus_info.as_ref().map(|i| i.summary.clone()),
        nexus_info.as_ref().map(|i| i.picture_url.clone()),
        nexus_info.as_ref().map(|i| i.downloads),
        nexus_info.as_ref().map(|i| i.endorsements),
        pak_dest_ref,
        custom_name,
        custom_type,
        nexus_info.as_ref().and_then(|i| if i.category.is_empty() { None } else { Some(i.category.clone()) }),
        nexus_info.as_ref().map(|i| i.tags.clone()).unwrap_or_default(),
    )?;

    let _ = fs::remove_dir_all(&temp_dir);

    let mut final_mod = mod_info;
    if let Some(ref info) = nexus_info {
        if final_mod.version == "unknown" || final_mod.version.is_empty() {
            final_mod.version = info.version.clone();
        }
        final_mod.nexus_description = if info.description.is_empty() { None } else { Some(info.description.clone()) };
        final_mod.nexus_version_cached = if info.version.is_empty() { None } else { Some(info.version.clone()) };
        final_mod.nexus_cached_at = Some(Utc::now().to_rfc3339());

        let cache_dir = if final_mod.enabled {
            PathBuf::from(&final_mod.game_path)
        } else {
            PathBuf::from(&final_mod.disabled_path)
        };
        let cache_json = serde_json::json!({
            "modId": nexus_id,
            "name": info.name,
            "author": info.author,
            "summary": info.summary,
            "description": info.description,
            "version": info.version,
            "downloads": info.downloads,
            "endorsements": info.endorsements,
            "pictureUrl": info.picture_url,
            "createdAt": info.created_at,
            "updatedAt": info.updated_at,
        });
        if cache_dir.exists() {
            let _ = fs::write(cache_dir.join(".nexus.json"), serde_json::to_string_pretty(&cache_json).unwrap_or_default());
        }
    }

    // Copy to library using mod name instead of UUID (if not already in library)
    let lib_folder_name = final_mod.name.clone();
    let is_already_in_lib = Path::new(&zip_path).starts_with(library::library_dir(&program_path));
    if !is_already_in_lib {
        let _ = library::copy_to_library(&zip_path, &program_path, &lib_folder_name);

        if let Some(ref info) = nexus_info {
            let lib_dir = library::get_library_path(&program_path, &lib_folder_name);
            if lib_dir.exists() {
                let cache_json = serde_json::json!({
                    "modId": nexus_id,
                    "name": info.name,
                    "author": info.author,
                    "summary": info.summary,
                    "description": info.description,
                    "version": info.version,
                    "downloads": info.downloads,
                    "endorsements": info.endorsements,
                    "pictureUrl": info.picture_url,
                    "createdAt": info.created_at,
                    "updatedAt": info.updated_at,
                });
                let _ = fs::write(lib_dir.join(".nexus.json"), serde_json::to_string_pretty(&cache_json).unwrap_or_default());
            }
        }
    }


    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        final_mod.library_zip = Some(
            library::get_library_path(&program_path, &lib_folder_name)
                .join(&zip_filename)
                .to_string_lossy()
                .to_string(),
        );
        data.mods.push(final_mod.clone());

        // Register the mod in the current profile's installed and enabled lists
        // Use mod name (stable across scans) instead of UUID (changes if mod is re-scanned)
        if final_mod.nexus_author.as_deref() != Some("UE4SS Native Mod") {
            let current_profile_id = data.current_profile_id.clone();
            let mod_name = final_mod.name.clone();
            if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
                // Always add to installed_mod_ids
                let in_installed = profile.installed_mod_ids.iter().any(|id| id.to_lowercase() == mod_name.to_lowercase());
                if !in_installed {
                    profile.installed_mod_ids.push(mod_name.clone());
                }
                // Add to enabled_mod_ids only if mod is enabled
                if final_mod.enabled {
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
        let _ = db::save_db(&program_path, &data_clone);
    }

    let _ = crate::profiles::save_pmm_meta(&final_mod);

    Ok(serde_json::to_value(&final_mod).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn check_mod_exists_command(
    zip_path: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let analysis = zip_handler::analyze_zip(&zip_path)?;
    let (mod_folder_name, _has_game_path, _wrapper, _subdir) = zip_handler::detect_mod_folder(&analysis.files);

    let folder_name = mod_folder_name.or_else(|| {
        Path::new(&zip_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
    });

    let data = state.data.lock().map_err(|e| e.to_string())?;

    let filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let nexus_id = nexus::extract_nexus_id(&filename);

    let existing = folder_name
        .as_ref()
        .and_then(|name| installer::check_mod_exists(name, nexus_id, &data.mods));

    Ok(serde_json::json!({
        "exists": existing.is_some(),
        "modInfo": existing,
        "modFolderName": folder_name,
    }))
}

#[tauri::command]
pub async fn update_mod_command(
    zip_path: String,
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let (game_path, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.program_path.clone())
    };

    if game_path.is_empty() {
        return Err("No game path configured. Set it first.".to_string());
    }

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let mod_type_str = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let existing = data
            .mods
            .iter()
            .find(|m| m.id == mod_id)
            .ok_or_else(|| "Mod not found".to_string())?;
        match existing.mod_type {
            crate::models::ModType::Ue4ss => "ue4ss",
            crate::models::ModType::PalSchema => "palschema",
            crate::models::ModType::Pak => "pak",
            crate::models::ModType::LogicMods => "logicmods",
            crate::models::ModType::Hybrid => "hybrid",
        }
        .to_string()
    };

    check_mod_dependencies(&game_path, &mod_type_str, &analysis)?;
    let temp_dir = std::env::temp_dir().join(format!("palmodmanager_{}", Uuid::new_v4()));
    let extracted = zip_handler::extract_zip_to_temp(&zip_path, &temp_dir)?;

    let zip_filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let now = Utc::now().to_rfc3339();

    let updated_mod = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let existing = data
            .mods
            .iter_mut()
            .find(|m| m.id == mod_id)
            .ok_or_else(|| "Mod not found".to_string())?;

        installer::update_mod(existing, &game_path, &extracted, &analysis, &zip_filename, &now)?;
        existing.clone()
    };

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        // Update library zip using mod name instead of UUID (if not already in library)
        let is_already_in_lib = Path::new(&zip_path).starts_with(library::library_dir(&program_path));
        if !is_already_in_lib {
            let lib_folder_name = updated_mod.name.clone();
            let _ = library::copy_to_library(&zip_path, &program_path, &lib_folder_name);
        }
        if let Some(existing) = data.mods.iter_mut().find(|m| m.id == mod_id) {
            existing.update_date = Some(now);
            existing.source_zip = zip_filename.clone();
        }
        let data_clone = data.clone();
        drop(data);
        let _ = db::save_db(&program_path, &data_clone);
    }

    let _ = fs::remove_dir_all(&temp_dir);

    Ok(serde_json::to_value(&updated_mod).map_err(|e| e.to_string())?)
}
