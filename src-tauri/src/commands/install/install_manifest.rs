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
    crate::logger::log(&format!("install_mod_with_manifest: Installing '{}' (type: {:?}) from '{}'", manifest.display_name, manifest.mod_type, zip_path));
    let (game_path, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.program_path.clone())
    };

    if game_path.is_empty() {
        crate::logger::log("install_mod_with_manifest: Error - Game path not configured");
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

    let author_val = nexus_info.as_ref().map(|i| i.author.clone()).or_else(|| manifest.author.clone());
    let summary_val = nexus_info.as_ref().map(|i| i.summary.clone()).or_else(|| manifest.summary.clone());
    let picture_val = nexus_info.as_ref().map(|i| i.picture_url.clone()).or_else(|| manifest.picture_url.clone());

    let now_str = Utc::now().to_rfc3339();
    let mut final_mod = installer::execute_manifest(
        &manifest,
        &extracted,
        Path::new(&game_path),
        author_val.clone(),
        summary_val.clone(),
        picture_val.clone(),
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
    } else if author_val.is_some() || picture_val.is_some() || summary_val.is_some() {
        let cache_dir = if final_mod.enabled {
            PathBuf::from(&final_mod.game_path)
        } else {
            PathBuf::from(&final_mod.disabled_path)
        };
        let cache_json = serde_json::json!({
            "modId": manifest.nexus_mod_id,
            "name": final_mod.name,
            "author": author_val.as_deref().unwrap_or_default(),
            "summary": summary_val.as_deref().unwrap_or_default(),
            "description": summary_val.as_deref().unwrap_or_default(),
            "version": final_mod.version,
            "downloads": 0,
            "endorsements": 0,
            "pictureUrl": picture_val.as_deref().unwrap_or_default(),
            "createdAt": now_str,
            "updatedAt": now_str,
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

        let lib_dir = library::get_library_path(&program_path, &lib_folder_name);
        if lib_dir.exists() {
            if let Some(ref info) = nexus_info {
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
            } else if author_val.is_some() || picture_val.is_some() || summary_val.is_some() {
                let cache_json = serde_json::json!({
                    "modId": manifest.nexus_mod_id,
                    "name": final_mod.name,
                    "author": author_val.as_deref().unwrap_or_default(),
                    "summary": summary_val.as_deref().unwrap_or_default(),
                    "description": summary_val.as_deref().unwrap_or_default(),
                    "version": final_mod.version,
                    "downloads": 0,
                    "endorsements": 0,
                    "pictureUrl": picture_val.as_deref().unwrap_or_default(),
                    "createdAt": now_str,
                    "updatedAt": now_str,
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
        let mod_name = final_mod.name.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            let in_installed = profile.installed_mod_ids.iter().any(|id| id.to_lowercase() == mod_name.to_lowercase() || id == &final_mod.id);
            if !in_installed {
                profile.installed_mod_ids.push(mod_name.clone());
            }
            if final_mod.enabled {
                let in_enabled = profile.enabled_mod_ids.iter().any(|id| id.to_lowercase() == mod_name.to_lowercase() || id == &final_mod.id);
                if !in_enabled {
                    profile.enabled_mod_ids.push(mod_name.clone());
                }
            }
        }

        crate::profiles::cleanup_profile_mod_lists(&mut data);
        crate::profiles::sync_current_profile_states(&mut data);

        let data_clone = data.clone();
        let _ = db::save_db(&program_path, &data_clone);
        sync_altermatic_helper(&data_clone);
    }

    let _ = crate::profiles::save_pmm_meta(&final_mod);

    Ok(serde_json::to_value(&final_mod).map_err(|e| e.to_string())?)
}
