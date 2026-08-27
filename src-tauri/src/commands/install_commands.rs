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
    let dep_status = crate::dependency_checker::check_dependencies(game_path);
    
    // Check UE4SS dependency
    let ue4ss_required = match mod_type {
        "ue4ss" | "palschema" | "hybrid" => true,
        _ => false,
    };
    if ue4ss_required && !dep_status.ue4ss_installed {
        return Err("UE4SS is not installed. This mod requires UE4SS to operate. Please install UE4SS first.".to_string());
    }

    // Check PalSchema dependency
    let has_palschema_folder = analysis.files.iter().any(|f| f.to_lowercase().contains("palschema"));
    let palschema_required = match mod_type {
        "palschema" => true,
        "hybrid" if (analysis.has_palschema_json || has_palschema_folder) => true,
        _ => false,
    };
    if palschema_required && !dep_status.palschema_installed {
        return Err("PalSchema is not installed. This mod requires PalSchema to operate. Please install PalSchema first.".to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn analyze_zip(zip_path: String, state: State<'_, AppState>) -> Result<Value, String> {
    println!("[INFO] Analyzing zip file: {}", zip_path);
    let _ = state; // state unused here now

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let mut nexus_id = zip_handler::extract_nexus_id_from_path(&zip_path)
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

    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            if let Some(content) = zip_handler::read_archive_file(&zip_path, target_file) {
                if let Ok(val) = serde_json::from_str::<Value>(&content) {
                    modinfo_data = Some(val);
                }
            }
        }
    }

    let mut sidecar_data: Option<Value> = None;
    let pmm_sidecar = PathBuf::from(format!("{}.pmm.json", zip_path));
    let pmm_sidecar_alt = Path::new(&zip_path).with_extension("pmm.json");
    if pmm_sidecar.exists() {
        if let Ok(c) = fs::read_to_string(&pmm_sidecar) {
            sidecar_data = serde_json::from_str(&c).ok();
        }
    } else if pmm_sidecar_alt.exists() {
        if let Ok(c) = fs::read_to_string(&pmm_sidecar_alt) {
            sidecar_data = serde_json::from_str(&c).ok();
        }
    }

    if nexus_id.is_none() {
        if let Some(ref info) = modinfo_data {
            if let Some(id) = info.get("nexusModId").and_then(|id| id.as_u64()).or_else(|| info.get("nexus_mod_id").and_then(|id| id.as_u64())) {
                nexus_id = Some(id as u32);
            } else if let Some(id_str) = info.get("nexusModId").and_then(|id| id.as_str()).or_else(|| info.get("nexus_mod_id").and_then(|id| id.as_str())) {
                if let Ok(id) = id_str.parse::<u32>() {
                    nexus_id = Some(id);
                }
            }
        }
        if nexus_id.is_none() {
            if let Some(ref sc) = sidecar_data {
                if let Some(id) = sc.get("nexusModId").or_else(|| sc.get("modId")).and_then(|id| id.as_u64()) {
                    nexus_id = Some(id as u32);
                } else if let Some(id_str) = sc.get("nexusModId").or_else(|| sc.get("modId")).and_then(|id| id.as_str()) {
                    if let Ok(id) = id_str.parse::<u32>() {
                        nexus_id = Some(id);
                    }
                }
            }
        }
    }

    let detected_version = {
        let from_info = modinfo_data.as_ref().and_then(|info| {
            info.get("version").and_then(|v| v.as_str()).map(|s| s.to_string())
        });
        if from_info.is_some() {
            from_info
        } else if let Some(from_sc) = sidecar_data.as_ref().and_then(|sc| {
            sc.get("version").or_else(|| sc.get("nexusVersion")).and_then(|v| v.as_str()).map(|s| s.to_string())
        }) {
            Some(from_sc)
        } else {
            let filename = Path::new(&zip_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            nexus::parse_mod_filename(&filename).version
        }
    };

    let nexus_info = if let Some(ref sc) = sidecar_data {
        let pic = sc.get("nexusPictureUrl").or_else(|| sc.get("pictureUrl")).and_then(|v| v.as_str());
        let name = sc.get("nexusName").or_else(|| sc.get("name")).and_then(|v| v.as_str());
        let author = sc.get("nexusAuthor").or_else(|| sc.get("author")).and_then(|v| v.as_str());
        let summary = sc.get("nexusSummary").or_else(|| sc.get("summary")).or_else(|| sc.get("description")).and_then(|v| v.as_str());
        let ver = sc.get("version").or_else(|| sc.get("nexusVersion")).and_then(|v| v.as_str());
        if pic.is_some() || name.is_some() || author.is_some() {
            Some(serde_json::json!({
                "modId": nexus_id.unwrap_or(0),
                "name": name.unwrap_or(""),
                "author": author.unwrap_or(""),
                "summary": summary.unwrap_or(""),
                "pictureUrl": pic.unwrap_or(""),
                "version": ver.unwrap_or(""),
                "downloads": sc.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                "endorsements": sc.get("endorsements").and_then(|v| v.as_u64()).unwrap_or(0),
            }))
        } else {
            None
        }
    } else {
        None
    };

    Ok(serde_json::json!({
        "zipPath": zip_path,
        "detectedType": detected_type,
        "hasLua": analysis.has_lua,
        "hasJson": analysis.has_json,
        "hasPalSchemaJson": analysis.has_palschema_json,
        "hasPak": analysis.has_pak,
        "hasInfoJson": analysis.has_info_json,
        "pakDestinationHint": analysis.pak_destination_hint,
        "rootFolder": analysis.root_folder,
        "fileCount": analysis.files.len(),
        "nexusModId": nexus_id,
        "detectedVersion": detected_version,
        "nexusInfo": nexus_info,
        "modinfo": modinfo_data,
        "files": analysis.files,
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
    println!("[INFO] Installing mod from zip: {}, custom_type: {:?}, pak_destination: {:?}", zip_path, custom_type, pak_destination);
    let (game_path, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.program_path.clone())
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
        nexus::fetch_mod_info(id).await.ok()
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

    let (force_load_order_ue4ss, force_load_order_palschema) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_ue4ss(&data),
            data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_palschema(&data)
        )
    };

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
        force_load_order_ue4ss,
        force_load_order_palschema,
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
        let lib_entry = library::copy_to_library(&zip_path, &program_path, &lib_folder_name, None, Some(&final_mod.version)).ok();
        let target_zip_name = lib_entry.map(|e| e.zip_name).unwrap_or_else(|| zip_filename.clone());

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

        let lib_zip_dest = library::get_library_path(&program_path, &lib_folder_name).join(&target_zip_name);
        let pmm_dest = std::path::PathBuf::from(format!("{}.pmm.json", lib_zip_dest.to_string_lossy()));
        let _ = fs::write(&pmm_dest, serde_json::to_string_pretty(&final_mod).unwrap_or_default());

        final_mod.source_zip = target_zip_name;
        final_mod.library_zip = Some(lib_zip_dest.to_string_lossy().to_string());
    } else {
        final_mod.library_zip = Some(zip_path.clone());
    }

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
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

        if !data_clone.settings.game_path.is_empty() {
            let current_profile = data_clone.profiles.iter().find(|p| p.id == data_clone.current_profile_id);
            let enabled_ids: Vec<String> = if let Some(p) = current_profile {
                p.enabled_mod_ids.clone()
            } else {
                data_clone.mods.iter().filter(|m| m.enabled).map(|m| m.id.clone()).collect()
            };
            let _ = crate::altermatic::sync_load_list(
                &std::path::PathBuf::from(&data_clone.settings.game_path),
                &enabled_ids,
                &data_clone.mods,
            );
        }
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
    let filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let nexus_id = nexus::extract_nexus_id(&filename);

    let mut metadata_folder_name = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            if let Some(content) = zip_handler::read_archive_file(&zip_path, target_file) {
                if let Ok(val) = serde_json::from_str::<Value>(&content) {
                    if let Some(name) = val.get("folderName").and_then(|v| v.as_str()) {
                        metadata_folder_name = Some(name.to_string());
                    } else if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                        metadata_folder_name = Some(name.to_string());
                    }
                }
            }
        }
    }

    let folder_name = metadata_folder_name.unwrap_or_else(|| {
        let detected = zip_handler::detect_folder_name_from_files(&analysis.files, &filename);
        if detected.is_empty() || detected == "UnresolvedMod" {
            installer::clean_zip_name(&filename)
        } else {
            detected
        }
    });

    let data = state.data.lock().map_err(|e| e.to_string())?;

    let mod_type = match analysis.detected_type {
        zip_handler::DetectedModType::Ue4ss => crate::models::ModType::Ue4ss,
        zip_handler::DetectedModType::PalSchema => crate::models::ModType::PalSchema,
        zip_handler::DetectedModType::Pak => crate::models::ModType::Pak,
        zip_handler::DetectedModType::LogicMods => crate::models::ModType::LogicMods,
        zip_handler::DetectedModType::Hybrid => crate::models::ModType::Hybrid,
        _ => crate::models::ModType::Pak,
    };

    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    let existing = installer::check_mod_exists(&folder_name, &mod_type, nexus_id, &profile_mods);

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
    let (game_path, program_path, current_profile_id) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.game_path.clone(),
            data.settings.program_path.clone(),
            data.current_profile_id.clone(),
        )
    };

    if game_path.is_empty() {
        return Err("No game path configured. Set it first.".to_string());
    }

    let nexus_id = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.iter().find(|m| m.id == mod_id).and_then(|m| m.nexus_mod_id)
    };

    let nexus_info = if let Some(id) = nexus_id {
        crate::nexus::fetch_mod_info(id).await.ok()
    } else {
        None
    };

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

    let mut updated_mod = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let force_load_order_ue4ss = data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_ue4ss(&data);
        let force_load_order_palschema = data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_palschema(&data);
        let existing = data
            .mods
            .iter_mut()
            .find(|m| m.id == mod_id)
            .ok_or_else(|| "Mod not found".to_string())?;

        installer::update_mod(existing, &game_path, &program_path, &current_profile_id, &extracted, &analysis, &zip_filename, &now, force_load_order_ue4ss, force_load_order_palschema)?;
        existing.clone()
    };

    let final_mod = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        // Update library zip using mod name instead of UUID (if not already in library)
        let is_already_in_lib = Path::new(&zip_path).starts_with(library::library_dir(&program_path));
        if !is_already_in_lib {
            let lib_folder_name = updated_mod.name.clone();
            let lib_entry = library::copy_to_library(&zip_path, &program_path, &lib_folder_name, None, Some(&updated_mod.version)).ok();
            if let Some(entry) = lib_entry {
                let lib_zip_dest = library::get_library_path(&program_path, &lib_folder_name).join(&entry.zip_name);
                let pmm_dest = std::path::PathBuf::from(format!("{}.pmm.json", lib_zip_dest.to_string_lossy()));
                let _ = fs::write(&pmm_dest, serde_json::to_string_pretty(&updated_mod).unwrap_or_default());
                updated_mod.source_zip = entry.zip_name;
                updated_mod.library_zip = Some(lib_zip_dest.to_string_lossy().to_string());
            }
        }
        let final_m = if let Some(existing) = data.mods.iter_mut().find(|m| m.id == mod_id) {
            existing.update_date = Some(now.clone());
            existing.source_zip = zip_filename.clone();
            if let Some(ref info) = nexus_info {
                if existing.version == "unknown" || existing.version.is_empty() {
                    existing.version = info.version.clone();
                }
                existing.nexus_version_cached = Some(info.version.clone());
                existing.nexus_cached_at = Some(now.clone());
                existing.nexus_picture_url = Some(info.picture_url.clone());
                existing.nexus_author = Some(info.author.clone());
                existing.nexus_summary = Some(info.summary.clone());
                existing.nexus_description = Some(info.description.clone());
                existing.nexus_endorsements = Some(info.endorsements);
                existing.nexus_downloads = Some(info.downloads);

                let cache_dir = if existing.enabled {
                    PathBuf::from(&existing.game_path)
                } else {
                    PathBuf::from(&existing.disabled_path)
                };
                if cache_dir.exists() {
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
                    let _ = fs::write(cache_dir.join(".nexus.json"), serde_json::to_string_pretty(&cache_json).unwrap_or_default());
                }
            } else {
                existing.nexus_version_cached = Some(existing.version.clone());
                existing.nexus_cached_at = Some(now.clone());
            }
            let _ = crate::profiles::save_pmm_meta(existing);
            existing.clone()
        } else {
            updated_mod.clone()
        };
        let data_clone = data.clone();
        drop(data);
        let _ = db::save_db(&program_path, &data_clone);
        final_m
    };

    let _ = fs::remove_dir_all(&temp_dir);

    Ok(serde_json::to_value(&final_mod).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn build_install_manifest(
    zip_path: String,
    game_path: String,
    pak_destination: Option<String>,
    custom_name: Option<String>,
) -> Result<crate::models::InstallManifest, String> {
    crate::zip_handler::build_install_manifest(
        &zip_path,
        Path::new(&game_path),
        pak_destination.as_deref(),
        custom_name,
    )
}

#[tauri::command]
pub async fn install_mod_with_manifest(
    manifest: crate::models::InstallManifest,
    zip_path: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    println!("[INFO] Installing mod with manifest: {}", manifest.display_name);
    let (game_path, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.program_path.clone())
    };

    if game_path.is_empty() {
        return Err("No game path configured. Set it first.".to_string());
    }

    let mod_type_str = match manifest.mod_type {
        crate::models::ModType::Ue4ss => "ue4ss",
        crate::models::ModType::PalSchema => "palschema",
        crate::models::ModType::Hybrid => "hybrid",
        _ => "pak",
    };
    let analysis = zip_handler::analyze_zip(&zip_path)?;
    check_mod_dependencies(&game_path, mod_type_str, &analysis)?;

    let nexus_info = if let Some(id) = manifest.nexus_mod_id {
        nexus::fetch_mod_info(id).await.ok()
    } else {
        None
    };

    let temp_dir = std::env::temp_dir().join(format!("palmodmanager_{}", Uuid::new_v4()));
    let extracted = zip_handler::extract_zip_to_temp(&zip_path, &temp_dir)?;

    let (force_load_order_ue4ss, force_load_order_palschema) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_ue4ss(&data),
            data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_palschema(&data)
        )
    };

    let now_str = Utc::now().to_rfc3339();
    let mut final_mod = installer::execute_manifest(
        &manifest,
        &extracted,
        Path::new(&game_path),
        nexus_info.as_ref().map(|i| i.author.clone()),
        nexus_info.as_ref().map(|i| i.summary.clone()),
        nexus_info.as_ref().map(|i| i.picture_url.clone()),
        nexus_info.as_ref().map(|i| i.downloads),
        nexus_info.as_ref().map(|i| i.endorsements),
        &now_str,
        force_load_order_ue4ss,
        force_load_order_palschema,
    )?;

    let raw_source_name = Path::new(&zip_path).file_name().unwrap_or_default().to_string_lossy().to_string();
    if raw_source_name.to_lowercase().starts_with("nexus_") || (raw_source_name.contains('-') && raw_source_name.len() > 30) {
        final_mod.source_zip = format!("{}.zip", manifest.display_name);
    } else {
        final_mod.source_zip = raw_source_name;
    }

    if let Some(ref info) = nexus_info {
        if final_mod.version == "unknown" || final_mod.version.is_empty() || final_mod.version.contains('-') {
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
            "modId": manifest.nexus_mod_id,
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
            let _ = std::fs::write(cache_dir.join(".nexus.json"), serde_json::to_string_pretty(&cache_json).unwrap_or_default());
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);

    // Copy to library using mod name instead of UUID (if not already in library)
    let lib_folder_name = final_mod.name.clone();
    let is_already_in_lib = Path::new(&zip_path).starts_with(library::library_dir(&program_path));
    if !is_already_in_lib {
        let lib_entry = library::copy_to_library(&zip_path, &program_path, &lib_folder_name, Some(&final_mod.source_zip), Some(&final_mod.version)).ok();
        let target_zip_name = lib_entry.map(|e| e.zip_name).unwrap_or_else(|| final_mod.source_zip.clone());

        if let Some(ref info) = nexus_info {
            let lib_dir = library::get_library_path(&program_path, &lib_folder_name);
            if lib_dir.exists() {
                let cache_json = serde_json::json!({
                    "modId": manifest.nexus_mod_id,
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
                let _ = std::fs::write(lib_dir.join(".nexus.json"), serde_json::to_string_pretty(&cache_json).unwrap_or_default());
            }
        }

        let lib_zip_dest = library::get_library_path(&program_path, &lib_folder_name).join(&target_zip_name);
        let pmm_dest = std::path::PathBuf::from(format!("{}.pmm.json", lib_zip_dest.to_string_lossy()));
        let _ = std::fs::write(&pmm_dest, serde_json::to_string_pretty(&final_mod).unwrap_or_default());

        final_mod.source_zip = target_zip_name.clone();
        final_mod.library_zip = Some(lib_zip_dest.to_string_lossy().to_string());
    } else {
        final_mod.library_zip = Some(zip_path.clone());
    }

    // Save to DB and profiles
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;

        final_mod.library_zip = Some(
            library::get_library_path(&program_path, &lib_folder_name)
                .join(&final_mod.source_zip)
                .to_string_lossy()
                .to_string(),
        );
        
        // Remove existing mod with the same ID if it is an update
        if let Some(pos) = data.mods.iter().position(|m| m.id == final_mod.id) {
            data.mods.remove(pos);
        }
        data.mods.push(final_mod.clone());

        // Update profile
        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            if !profile.installed_mod_ids.contains(&final_mod.id) {
                profile.installed_mod_ids.push(final_mod.id.clone());
            }
            if !profile.enabled_mod_ids.contains(&final_mod.id) {
                profile.enabled_mod_ids.push(final_mod.id.clone());
            }
        }

        let data_clone = data.clone();
        let _ = db::save_db(&program_path, &data_clone);
    }

    let _ = crate::profiles::save_pmm_meta(&final_mod);

    Ok(serde_json::to_value(&final_mod).map_err(|e| e.to_string())?)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigDiff {
    pub file_name: String,
    pub keys_user_changed: Vec<crate::config_merge::ChangedKeyDetail>,
    pub keys_added_by_author: Vec<String>,
    pub keys_removed_by_author: Vec<String>,
}

#[tauri::command]
pub async fn preview_config_diff(
    zip_path: String,
    mod_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ConfigDiff>, String> {
    use crate::zip_handler;
    use std::fs;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    let (game_path, _program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.program_path.clone())
    };

    if game_path.is_empty() {
        return Err("Game path is not configured".to_string());
    }

    let (mod_name, mod_config_path, installed_roots) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let m = data.mods.iter().find(|m| m.id == mod_id)
            .ok_or_else(|| "Mod not found".to_string())?;
        
        let game = Path::new(&game_path);
        let mut roots = Vec::new();
        if !m.game_path.is_empty() {
            let p = crate::config_merge::resolve_path_in_game(game, &m.game_path);
            if p.exists() {
                let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
                roots.push(r);
            }
        }
        if !m.disabled_path.is_empty() {
            let p = crate::config_merge::resolve_path_in_game(game, &m.disabled_path);
            if p.exists() {
                let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
                if !roots.contains(&r) {
                    roots.push(r);
                }
            }
        }
        for extra in &m.extra_files {
            let p = crate::config_merge::resolve_path_in_game(game, extra);
            if p.exists() {
                let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
                if !roots.contains(&r) {
                    roots.push(r);
                }
            }
        }
        (m.name.clone(), m.config_path.clone(), roots)
    };

    let temp_dir = std::env::temp_dir().join(format!("palmodmanager_diff_{}", Uuid::new_v4()));
    let extracted = zip_handler::extract_zip_to_temp(&zip_path, &temp_dir)?;

    let zip_filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            let full_path = extracted.join(target_file);
            if full_path.exists() {
                if let Ok(content) = std::fs::read_to_string(full_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        modinfo_data = Some(val);
                    }
                }
            }
        }
    }

    let game = Path::new(&game_path);
    let manifest = zip_handler::build_manifest_from_files(
        &analysis.files,
        &zip_filename,
        game,
        None,
        Some(mod_name),
        modinfo_data,
    )?;

    let incoming_mod_dir = if !manifest.folder_name.is_empty() {
        fn find_folder(current: &Path, folder_name: &str) -> Option<PathBuf> {
            if current.is_dir() {
                if let Some(name) = current.file_name().and_then(|n| n.to_str()) {
                    if name.to_lowercase() == folder_name.to_lowercase() {
                        return Some(current.to_path_buf());
                    }
                }
                if let Ok(entries) = fs::read_dir(current) {
                    for entry in entries.flatten() {
                        if let Some(found) = find_folder(&entry.path(), folder_name) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
        find_folder(&extracted, &manifest.folder_name).unwrap_or(extracted.clone())
    } else {
        extracted.clone()
    };

    let mut incoming_snapshot = crate::config_merge::snapshot_configs(&incoming_mod_dir, mod_config_path.as_deref());
    // If incoming_mod_dir didn't catch everything (e.g. extracted has full subpaths), also scan extracted root
    if incoming_mod_dir != extracted {
        let root_snapshot = crate::config_merge::snapshot_configs(&extracted, mod_config_path.as_deref());
        for entry in root_snapshot.entries {
            if !incoming_snapshot.entries.iter().any(|(r, _)| r.file_name() == entry.0.file_name()) {
                incoming_snapshot.entries.push(entry);
            }
        }
    }

    let mut diffs = Vec::new();

    for (rel_path, new_content) in incoming_snapshot.entries {
        let mut old_content_opt = None;

        for root in &installed_roots {
            let direct = root.join(&rel_path);
            if direct.exists() && direct.is_file() {
                if let Ok(c) = fs::read_to_string(&direct) {
                    old_content_opt = Some(c);
                    break;
                }
            }
            let candidate_scripts = root.join("Scripts").join(&rel_path);
            if candidate_scripts.exists() && candidate_scripts.is_file() {
                if let Ok(c) = fs::read_to_string(&candidate_scripts) {
                    old_content_opt = Some(c);
                    break;
                }
            }
            if let Some(fname) = rel_path.file_name() {
                let candidate = root.join("Scripts").join(fname);
                if candidate.exists() && candidate.is_file() {
                    if let Ok(c) = fs::read_to_string(&candidate) {
                        old_content_opt = Some(c);
                        break;
                    }
                }
                let candidate2 = root.join(fname);
                if candidate2.exists() && candidate2.is_file() {
                    if let Ok(c) = fs::read_to_string(&candidate2) {
                        old_content_opt = Some(c);
                        break;
                    }
                }
            }
        }

        if old_content_opt.is_none() {
            if let Some(ref custom_str) = mod_config_path {
                let cp = crate::config_merge::resolve_path_in_game(game, custom_str);
                if cp.exists() && cp.is_file() {
                    if let Ok(c) = fs::read_to_string(&cp) {
                        old_content_opt = Some(c);
                    }
                }
            }
        }

        if let Some(old_content) = old_content_opt {
            let ext = rel_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if let Some((user_changed, added, removed)) = crate::config_merge::generate_config_diff(&old_content, &new_content, ext) {
                if !user_changed.is_empty() || !added.is_empty() || !removed.is_empty() {
                    diffs.push(ConfigDiff {
                        file_name: rel_path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| rel_path.to_string_lossy().to_string()),
                        keys_user_changed: user_changed,
                        keys_added_by_author: added,
                        keys_removed_by_author: removed,
                    });
                }
            }
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(diffs)
}

