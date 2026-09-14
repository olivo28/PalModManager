use std::path::{Path, PathBuf};
use chrono::Utc;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;
use crate::db;
use crate::installer;
use crate::library;
use crate::nexus;
use crate::state::AppState;
use crate::zip_handler;
use super::utils::{check_mod_dependencies, sync_altermatic_helper};
use super::InstallProgressPayload;

#[tauri::command]
pub async fn build_install_manifest(
    zip_path: String,
    game_path: String,
    pak_destination: Option<String>,
    custom_name: Option<String>,
    custom_folder: Option<String>,
) -> Result<crate::models::InstallManifest, String> {
    crate::zip_handler::build_install_manifest_with_folder(
        &zip_path,
        Path::new(&game_path),
        pak_destination.as_deref(),
        custom_name,
        custom_folder,
    )
}

#[tauri::command]
pub async fn install_mod_with_manifest(
    app: AppHandle,
    manifest: crate::models::InstallManifest,
    zip_path: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    crate::logger::log(&format!("install_mod_with_manifest: Installing '{}' (type: {:?}) from '{}'", manifest.display_name, manifest.mod_type, zip_path));
    let (game_path, program_path, existing_mod_info) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let target_nexus = manifest.nexus_mod_id;
        let target_name = &manifest.display_name;
        let target_folder = &manifest.folder_name;

        let existing = data.mods.iter().find(|m| {
            (target_nexus.is_some() && m.nexus_mod_id == target_nexus)
                || m.name.eq_ignore_ascii_case(target_name)
                || (!target_folder.is_empty() && crate::profiles::get_mod_folder_name(m).eq_ignore_ascii_case(target_folder))
        }).cloned();

        (data.settings.game_path.clone(), data.settings.program_path.clone(), existing)
    };

    if game_path.is_empty() {
        crate::logger::log("install_mod_with_manifest: Error - Game path not configured");
        return Err("No game path configured. Set it first.".to_string());
    }

    // Clean-slate update: snapshot text configs and wipe old mod directory before installing new files
    let old_snapshot = if let Some(ref existing) = existing_mod_info {
        let snap = super::clean_slate::snapshot_mod_text_files(Path::new(&game_path), existing);
        super::clean_slate::purge_existing_mod_files(Path::new(&game_path), existing);
        snap
    } else {
        std::collections::HashMap::new()
    };

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
    let downloads_val = nexus_info.as_ref().map(|i| i.downloads);
    let endorsements_val = nexus_info.as_ref().map(|i| i.endorsements);

    let now_str = Utc::now().to_rfc3339();

    let app_handle = std::sync::Arc::new(app);
    let app_emit = app_handle.clone();
    let zip_path_clone = zip_path.clone();
    let manifest_clone = manifest.clone();
    let game_path_clone = game_path.clone();
    let author_val_clone = author_val.clone();
    let summary_val_clone = summary_val.clone();
    let picture_val_clone = picture_val.clone();
    let now_str_clone = now_str.clone();

    let mut final_mod = tauri::async_runtime::spawn_blocking(move || -> Result<crate::models::ModInfo, String> {
        let _ = app_emit.emit("install-progress", InstallProgressPayload {
            stage: "extracting".to_string(),
            percent: 10,
        });

        let temp_dir = std::env::temp_dir().join(format!("palmodmanager_{}", Uuid::new_v4()));
        let extracted = zip_handler::extract_zip_to_temp(&zip_path_clone, &temp_dir)?;

        let _ = app_emit.emit("install-progress", InstallProgressPayload {
            stage: "installing".to_string(),
            percent: 50,
        });

        let mod_res = installer::execute_manifest(
            &manifest_clone,
            &extracted,
            Path::new(&game_path_clone),
            author_val_clone,
            summary_val_clone,
            picture_val_clone,
            downloads_val,
            endorsements_val,
            &now_str_clone,
            force_load_order_ue4ss,
            force_load_order_palschema,
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
        mod_res
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app_handle.emit("install-progress", InstallProgressPayload {
        stage: "saving".to_string(),
        percent: 90,
    });

    // Always preserve the exact original filename — rollback depends on it.
    // Temp/nexus filenames (nexus_*, disc_*, temp_*) are already handled inside copy_to_library.
    final_mod.source_zip = Path::new(&zip_path).file_name().unwrap_or_default().to_string_lossy().to_string();
    final_mod.fomod_choices = manifest.fomod_choices.clone();

    if let Some(ref existing) = existing_mod_info {
        if final_mod.custom_name.is_none() && existing.custom_name.is_some() {
            final_mod.custom_name = existing.custom_name.clone();
        }
        if final_mod.custom_notes.is_none() && existing.custom_notes.is_some() {
            final_mod.custom_notes = existing.custom_notes.clone();
        }
    }

    let config_diffs = if !old_snapshot.is_empty() {
        super::clean_slate::diff_exact_match_configs(&old_snapshot, Path::new(&game_path), &final_mod)
    } else {
        Vec::new()
    };

    let parsed_nexus = crate::nexus::parse_mod_filename(&final_mod.source_zip);
    if let Some(ref pv) = parsed_nexus.version {
        if crate::commands::nexus_commands::is_version_newer(&final_mod.version, pv) {
            final_mod.version = pv.clone();
        }
    }

    if let Some(ref info) = nexus_info {
        if final_mod.version == "unknown"
            || final_mod.version.is_empty()
            || final_mod.version.contains('-')
            || crate::commands::nexus_commands::is_version_newer(&final_mod.version, &info.version)
        {
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
        
        // Check if an active conflicting mod or variant already exists
        let should_auto_disable = {
            let target_nexus_id = final_mod.nexus_mod_id;
            let target_folder = crate::profiles::get_mod_folder_name(&final_mod);
            let target_type = final_mod.mod_type.clone();
            let target_phys = crate::installer::helpers::get_physical_identity(&final_mod.game_path, &final_mod.disabled_path);

            data.mods.iter().any(|other| {
                if other.id != final_mod.id && other.enabled && other.nexus_author.as_deref() != Some("UE4SS Native Mod") {
                    let same_nexus = target_nexus_id.is_some() && other.nexus_mod_id == target_nexus_id && other.mod_type == target_type;
                    let other_folder = crate::profiles::get_mod_folder_name(other);
                    let same_folder = !target_folder.is_empty() && target_folder.eq_ignore_ascii_case(&other_folder) && other.mod_type == target_type;
                    let other_phys = crate::installer::helpers::get_physical_identity(&other.game_path, &other.disabled_path);
                    let same_phys = !target_phys.is_empty() && target_phys.eq_ignore_ascii_case(&other_phys);
                    same_nexus || same_folder || same_phys
                } else {
                    false
                }
            })
        };

        // Remove existing mod with the same ID or existing match if it is an update
        if let Some(ref ex) = existing_mod_info {
            if let Some(pos) = data.mods.iter().position(|m| m.id == ex.id) {
                data.mods.remove(pos);
            }
        }
        if let Some(pos) = data.mods.iter().position(|m| m.id == final_mod.id) {
            data.mods.remove(pos);
        }
        data.mods.push(final_mod.clone());

        if should_auto_disable {
            crate::logger::log(&format!(
                "install_mod_with_manifest: Auto-disabling variant '{}' due to active conflicting mod",
                final_mod.name
            ));
            let _ = crate::profiles::disable_mod_internal(&mut data, &program_path, &final_mod.id);
            if let Some(m) = data.mods.iter().find(|m| m.id == final_mod.id) {
                final_mod = m.clone();
            }
        }

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

    let _ = app_handle.emit("install-progress", InstallProgressPayload {
        stage: "complete".to_string(),
        percent: 100,
    });

    let mut result_value = serde_json::to_value(&final_mod).map_err(|e| e.to_string())?;
    if !config_diffs.is_empty() {
        if let Some(obj) = result_value.as_object_mut() {
            obj.insert("configDiffs".to_string(), serde_json::to_value(&config_diffs).unwrap_or_default());
        }
    }

    Ok(result_value)
}
