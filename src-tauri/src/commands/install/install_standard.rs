use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde_json::Value;
use tauri::State;
use uuid::Uuid;
use crate::db;
use crate::installer;
use crate::library;
use crate::nexus;
use crate::state::AppState;
use crate::zip_handler;
use super::utils::{check_mod_dependencies, sync_altermatic_helper};

#[tauri::command]
pub async fn install_mod_command(
    zip_path: String,
    custom_type: Option<String>,
    pak_destination: Option<String>,
    custom_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    crate::logger::log(&format!("install_mod_from_zip: Installing mod from '{}' (custom_type: {:?}, pak_destination: {:?})", zip_path, custom_type, pak_destination));
    let (game_path, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.program_path.clone())
    };

    if game_path.is_empty() {
        crate::logger::log("install_mod_from_zip: Error - Game path not configured");
        return Err("No game path configured. Set it first.".to_string());
    }

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let resolved_type = custom_type.as_deref().unwrap_or(match analysis.detected_type {
        zip_handler::DetectedModType::Ue4ss => "ue4ss",
        zip_handler::DetectedModType::PalSchema => "palschema",
        zip_handler::DetectedModType::Pak => "pak",
        zip_handler::DetectedModType::LogicMods => "logicmods",
        zip_handler::DetectedModType::Hybrid => "hybrid",
        zip_handler::DetectedModType::Altermatic => "altermatic",
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
        sync_altermatic_helper(&data_clone);
    }

    let _ = crate::profiles::save_pmm_meta(&final_mod);

    Ok(serde_json::to_value(&final_mod).map_err(|e| e.to_string())?)
}
