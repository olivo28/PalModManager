use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;
use crate::db;
use crate::installer;
use crate::library;
use crate::state::AppState;
use crate::zip_handler;
use super::utils::{check_mod_dependencies, sync_altermatic_helper};
use super::InstallProgressPayload;

#[tauri::command]
pub async fn update_mod_command(
    app: AppHandle,
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
            crate::models::ModType::Altermatic => "altermatic",
        }
        .to_string()
    };

    check_mod_dependencies(&game_path, &mod_type_str, &analysis)?;

    let zip_filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let now = Utc::now().to_rfc3339();

    let (mut existing_clone, force_load_order_ue4ss, force_load_order_palschema) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let force_ue = data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_ue4ss(&data);
        let force_pal = data.settings.force_load_order.unwrap_or(false) && crate::profiles::effective_force_palschema(&data);
        let existing = data
            .mods
            .iter()
            .find(|m| m.id == mod_id)
            .ok_or_else(|| "Mod not found".to_string())?;
        (existing.clone(), force_ue, force_pal)
    };

    let app_handle = std::sync::Arc::new(app);
    let app_emit = app_handle.clone();
    let zip_path_clone = zip_path.clone();
    let game_path_clone = game_path.clone();
    let program_path_clone = program_path.clone();
    let current_profile_id_clone = current_profile_id.clone();
    let analysis_clone = analysis.clone();
    let now_clone = now.clone();

    let mut updated_mod = tauri::async_runtime::spawn_blocking(move || -> Result<crate::models::ModInfo, String> {
        let _ = app_emit.emit("install-progress", InstallProgressPayload {
            stage: "extracting".to_string(),
            percent: 10,
        });

        let temp_dir = std::env::temp_dir().join(format!("palmodmanager_{}", Uuid::new_v4()));
        let extracted = zip_handler::extract_zip_to_temp(&zip_path_clone, &temp_dir)?;

        let zip_filename = Path::new(&zip_path_clone)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let _ = app_emit.emit("install-progress", InstallProgressPayload {
            stage: "installing".to_string(),
            percent: 50,
        });

        let update_res = installer::update_mod(
            &mut existing_clone,
            &game_path_clone,
            &program_path_clone,
            &current_profile_id_clone,
            &extracted,
            &analysis_clone,
            &zip_filename,
            &now_clone,
            force_load_order_ue4ss,
            force_load_order_palschema,
        );

        let _ = fs::remove_dir_all(&temp_dir);
        update_res.map(|_| existing_clone)
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app_handle.emit("install-progress", InstallProgressPayload {
        stage: "saving".to_string(),
        percent: 90,
    });

    let final_mod = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let sidecar_version = {
            let p1 = PathBuf::from(format!("{}.pmm.json", zip_path));
            let p2 = Path::new(&zip_path).with_extension("pmm.json");
            let target = if p1.exists() { Some(p1) } else if p2.exists() { Some(p2) } else { None };
            target.and_then(|p| fs::read_to_string(p).ok())
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .and_then(|v| {
                    v.get("version").or_else(|| v.get("nexusVersion")).and_then(|x| x.as_str()).map(|s| s.to_string())
                })
                .filter(|v| !v.is_empty() && v != "unknown")
        };

        let parsed_version = crate::nexus::parse_mod_filename(&zip_filename).version
            .filter(|v| !v.is_empty() && v != "unknown");

        let detected_archive_ver = sidecar_version
            .or(parsed_version)
            .or_else(|| {
                if !updated_mod.version.is_empty() && updated_mod.version != "unknown" && updated_mod.version != "1.0" {
                    Some(updated_mod.version.clone())
                } else {
                    None
                }
            });

        let final_version = if let Some(ver) = detected_archive_ver {
            ver
        } else if let Some(ref info) = nexus_info {
            if !info.version.is_empty() && info.version != "unknown" {
                info.version.clone()
            } else if !updated_mod.version.is_empty() && updated_mod.version != "unknown" {
                updated_mod.version.clone()
            } else {
                "1.0".to_string()
            }
        } else if !updated_mod.version.is_empty() && updated_mod.version != "unknown" {
            updated_mod.version.clone()
        } else {
            "1.0".to_string()
        };

        updated_mod.version = final_version.clone();

        let is_already_in_lib = Path::new(&zip_path).starts_with(library::library_dir(&program_path));
        if !is_already_in_lib {
            let lib_folder_name = updated_mod.name.clone();
            let lib_entry = library::copy_to_library(&zip_path, &program_path, &lib_folder_name, Some(&zip_filename), Some(&updated_mod.version)).ok();
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
            existing.version = final_version.clone();

            if let Some(ref info) = nexus_info {
                existing.nexus_version_cached = Some(info.version.clone());
                existing.nexus_cached_at = Some(now.clone());
                existing.nexus_picture_url = Some(info.picture_url.clone());
                existing.nexus_author = Some(info.author.clone());
                existing.nexus_summary = Some(info.summary.clone());
                existing.nexus_description = Some(info.description.clone());
                existing.nexus_endorsements = Some(info.endorsements);
                existing.nexus_downloads = Some(info.downloads);

                let is_newer = crate::commands::nexus_commands::is_version_newer(&existing.version, &info.version);
                existing.has_pending_update = Some(is_newer);

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
                existing.has_pending_update = Some(false);
            }
            let _ = crate::profiles::save_pmm_meta(existing);
            existing.clone()
        } else {
            updated_mod.clone()
        };

        // Auto-purge redundant duplicate mod entries that match the updated mod's extra_files or nexus ID
        let updated_extra_files = final_m.extra_files.clone();
        let updated_nexus_id = final_m.nexus_mod_id;
        let mut purged_ids: Vec<String> = Vec::new();

        data.mods.retain(|other| {
            if other.id == mod_id {
                return true;
            }
            let matches_extra = (!other.game_path.is_empty() && updated_extra_files.contains(&other.game_path))
                || (!other.disabled_path.is_empty() && updated_extra_files.contains(&other.disabled_path));
            let matches_nexus = updated_nexus_id.is_some() && other.nexus_mod_id == updated_nexus_id && other.mod_type != final_m.mod_type;
            if matches_extra || matches_nexus {
                crate::logger::log(&format!(
                    "update_mod: Purging redundant duplicate mod entry '{}' (id: {}) merged into updated mod '{}'",
                    other.name, other.id, final_m.name
                ));
                purged_ids.push(other.id.clone());
                false
            } else {
                true
            }
        });

        if !purged_ids.is_empty() {
            for profile in &mut data.profiles {
                profile.installed_mod_ids.retain(|id| !purged_ids.contains(id));
                profile.enabled_mod_ids.retain(|id| !purged_ids.contains(id));

                // Migrate virtual folder assignments from purged duplicate entries to final_m.id
                for folder in &mut profile.mod_folders {
                    let mut had_purged = false;
                    folder.mod_ids.retain(|id| {
                        if purged_ids.contains(id) {
                            had_purged = true;
                            false
                        } else {
                            true
                        }
                    });
                    if had_purged && !folder.mod_ids.contains(&final_m.id) {
                        folder.mod_ids.push(final_m.id.clone());
                    }
                }
            }
        }

        // Guarantee final_m is registered in the current profile's installed and enabled lists
        if final_m.nexus_author.as_deref() != Some("UE4SS Native Mod") {
            let mod_name = final_m.name.clone();
            let mod_id = final_m.id.clone();
            if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
                let in_installed = profile.installed_mod_ids.iter().any(|id| {
                    id.eq_ignore_ascii_case(&mod_name) || id == &mod_id
                });
                if !in_installed {
                    profile.installed_mod_ids.push(mod_name.clone());
                }
                if final_m.enabled {
                    let in_enabled = profile.enabled_mod_ids.iter().any(|id| {
                        id.eq_ignore_ascii_case(&mod_name) || id == &mod_id
                    });
                    if !in_enabled {
                        profile.enabled_mod_ids.push(mod_name.clone());
                    }
                }
            }
        }

        crate::profiles::cleanup_profile_mod_lists(&mut data);
        crate::profiles::sync_current_profile_states(&mut data);

        let data_clone = data.clone();
        drop(data);
        let _ = db::save_db(&program_path, &data_clone);
        sync_altermatic_helper(&data_clone);
        final_m
    };

    let _ = app_handle.emit("install-progress", InstallProgressPayload {
        stage: "complete".to_string(),
        percent: 100,
    });

    Ok(serde_json::to_value(&final_mod).map_err(|e| e.to_string())?)
}
