use crate::db;
use crate::models;
use crate::state::AppState;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::State;
use walkdir::WalkDir;

#[tauri::command]
pub async fn get_game_version(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let start = Instant::now();
    crate::logger::log("Starting get_game_version...");
    let game_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };
    if game_path.is_empty() {
        crate::logger::log("get_game_version: game_path is empty");
        return Ok(None);
    }
    let exe_path = crate::dependency_checker::get_shipping_exe_path(Path::new(&game_path));
    let fallback = PathBuf::from(&game_path).join("Palworld.exe");
    
    let result = if exe_path.exists() || fallback.exists() {
        Some("Palworld".to_string())
    } else {
        None
    };
    crate::logger::log(&format!("get_game_version completed in {:?}. Result: {:?}", start.elapsed(), result));
    Ok(result)
}

#[tauri::command]
pub fn get_mods(state: State<AppState>) -> Result<Value, String> {
    crate::logger::log("get_mods: Requesting cached mods...");
    let start = Instant::now();
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    crate::profiles::sync_current_profile_states(&mut data);
    let profile_mods = filter_mods_for_current_profile(&data);
    crate::logger::log(&format!("get_mods completed in {:?}", start.elapsed()));
    serde_json::to_value(&profile_mods).map_err(|e| e.to_string())
}

/// Filter the global mod list to only include mods installed in the current profile.
/// Native UE4SS mods always pass through when UE4SS is enabled for the profile.
pub fn filter_mods_for_current_profile_pub(data: &crate::models::AppData) -> Vec<models::ModInfo> {
    filter_mods_for_current_profile(data)
}

fn filter_mods_for_current_profile(data: &crate::models::AppData) -> Vec<models::ModInfo> {
    let current_id = &data.current_profile_id;
    let profile = data.profiles.iter().find(|p| p.id == *current_id);
    let ue4ss_enabled = profile.map(|p| p.ue4ss_enabled).unwrap_or(false);
    let palschema_enabled = profile.map(|p| p.palschema_enabled).unwrap_or(false);

    data.mods.iter().filter(|m| {
        let is_native = m.nexus_author.as_deref() == Some("UE4SS Native Mod");
        if is_native {
            return ue4ss_enabled; // Native mods visible only when UE4SS active
        }
        let is_palschema = m.mod_type == models::ModType::PalSchema;
        if is_palschema && !palschema_enabled && !ue4ss_enabled {
            return false;
        }
        // Regular mods: visible if they match any entry in this profile's installed list
        if let Some(prof) = profile {
            prof.installed_mod_ids.iter().any(|entry| {
                crate::profiles::mod_matches_profile_entry(m, entry)
            })
        } else {
            false
        }
    }).cloned().collect()
}


/// Public helper: scan disk mods and merge with an existing db_mods slice.
/// Used both by scan_mods command and internally by switch_profile.
pub fn scan_mods_internal(
    game_path: &str,
    program_path: &str,
    current_profile_id: &str,
    installed_ids: &[String],
    db_mods: &[models::ModInfo],
) -> Vec<models::ModInfo> {
    if game_path.is_empty() {
        return Vec::new();
    }
    let game = PathBuf::from(game_path);
    let mut fs_mods: Vec<models::ModInfo> = vec![];

    let ue4ss_mods_dir = crate::dependency_checker::get_binaries_dir(&game).join("ue4ss").join("Mods");
    if ue4ss_mods_dir.exists() {
        scan_ue4ss_mods(&ue4ss_mods_dir, &mut fs_mods);
    }

    let palschema_dir = crate::dependency_checker::get_binaries_dir(&game).join("ue4ss").join("Mods").join("PalSchema").join("mods");
    if palschema_dir.exists() {
        scan_palschema_mods(&palschema_dir, &mut fs_mods);
    }

    let pak_mods_dir = game.join("Pal").join("Content").join("Paks").join("~mods");
    if pak_mods_dir.exists() {
        scan_pak_mods(&pak_mods_dir, "pak", &mut fs_mods);
    }

    let logic_mods_dir = game.join("Pal").join("Content").join("Paks").join("LogicMods");
    if logic_mods_dir.exists() {
        scan_pak_mods(&logic_mods_dir, "logicmods", &mut fs_mods);
    }

    let disabled_base = PathBuf::from(program_path)
        .join("profiles")
        .join(current_profile_id)
        .join("disabled_mods");
    if disabled_base.exists() {
        scan_disabled_mods(&disabled_base, &mut fs_mods);
    }

    merge_scan_with_db(current_profile_id, installed_ids, db_mods, &fs_mods)
}

#[tauri::command]
pub fn scan_mods(state: State<AppState>) -> Result<Value, String> {
    crate::logger::log("scan_mods: Starting full disk scan...");
    let start_scan = Instant::now();
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path = data.settings.game_path.clone();
    let program_path = data.settings.program_path.clone();
    let current_profile_id = data.current_profile_id.clone();
    let current_profile = data.profiles.iter().find(|p| p.id == current_profile_id);
    let installed_ids = current_profile.map(|p| p.installed_mod_ids.clone()).unwrap_or_default();

    if game_path.is_empty() {
        crate::logger::log("scan_mods: game_path empty, aborting scan");
        return Ok(serde_json::json!([]));
    }

    let merged = scan_mods_internal(&game_path, &program_path, &current_profile_id, &installed_ids, &data.mods.clone());
    data.mods = merged;
    crate::profiles::auto_add_scanned_mods_to_profile(&mut data);
    crate::profiles::cleanup_profile_mod_lists(&mut data);
    crate::profiles::sync_current_profile_states(&mut data);
    let profile_mods = filter_mods_for_current_profile(&data);
    let data_clone = data.clone();
    drop(data);

    let _ = db::save_db(&program_path, &data_clone);
    crate::logger::log(&format!("scan_mods: Full disk scan finished in total {:?}", start_scan.elapsed()));
    serde_json::to_value(&profile_mods).map_err(|e| e.to_string())
}


#[tauri::command]
pub fn remove_mod(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path_str = data.settings.game_path.clone();
    let current_profile_id = data.current_profile_id.clone();

    let mod_index = match data.mods.iter().position(|m| m.id == mod_id) {
        Some(idx) => idx,
        None => return Ok(serde_json::json!({ "success": true })),
    };
    
    let mod_info = data.mods[mod_index].clone();

    crate::logger::log(&format!("remove_mod: Removing mod '{}' (id: {})", mod_info.name, mod_info.id));

    // Helper to delete path and its companion sidecar if it's a file
    let delete_path_and_sidecar = |path_str: &str| {
        if path_str.is_empty() {
            return;
        }
        let p = Path::new(path_str);
        if p.exists() {
            if p.is_dir() {
                let _ = fs::remove_dir_all(p);
            } else {
                let _ = fs::remove_file(p);
                let sidecar = std::path::PathBuf::from(format!("{}.pmm.json", path_str));
                if sidecar.exists() {
                    let _ = fs::remove_file(sidecar);
                }
            }
        } else {
            let sidecar = std::path::PathBuf::from(format!("{}.pmm.json", path_str));
            if sidecar.exists() {
                let _ = fs::remove_file(sidecar);
            }
        }
    };

    // 1. Delete game_path if exists
    delete_path_and_sidecar(&mod_info.game_path);

    // 2. Delete disabled_path if exists
    delete_path_and_sidecar(&mod_info.disabled_path);

    // 3. Delete extra_files
    for extra in &mod_info.extra_files {
        delete_path_and_sidecar(extra);
    }

    // Clean up reference in mods.txt
    if !game_path_str.is_empty() {
        let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path_str));
        let mods_txt = binaries_dir.join("ue4ss").join("Mods").join("mods.txt");
        if mods_txt.exists() {
            let folder_name = crate::profiles::get_mod_folder_name(&mod_info);
            let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &folder_name);
            let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &mod_info.name);
            
            for extra_path_str in &mod_info.extra_files {
                let extra_path_lower = extra_path_str.to_lowercase();
                if extra_path_lower.contains("ue4ss/mods/") {
                    let extra_path = Path::new(extra_path_str);
                    if let Some(extra_folder_name) = extra_path.file_name().map(|n| n.to_string_lossy().to_string()) {
                        let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &extra_folder_name);
                    }
                }
            }
        }
    }

    // 4. For Pak/LogicMods, cleanup remnants by name in target folders
    if mod_info.mod_type == models::ModType::Pak || mod_info.mod_type == models::ModType::LogicMods {
        if !game_path_str.is_empty() {
            let game_base = PathBuf::from(&game_path_str);
            let check_dirs = vec![
                game_base.join("Pal").join("Content").join("Paks").join("~mods"),
                game_base.join("Pal").join("Content").join("Paks").join("LogicMods"),
                PathBuf::from(&program_path).join("profiles").join(&current_profile_id).join("disabled_mods").join("pak"),
                PathBuf::from(&program_path).join("profiles").join(&current_profile_id).join("disabled_mods").join("logicmods"),
            ];
            for dir in check_dirs {
                if dir.exists() {
                    if let Ok(entries) = fs::read_dir(&dir) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.to_lowercase().starts_with(&mod_info.name.to_lowercase()) {
                                let _ = fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Remove mod_id and mod_name from ALL profiles (both installed and enabled lists)
    for profile in &mut data.profiles {
        profile.installed_mod_ids.retain(|id| id != &mod_info.id && id.to_lowercase() != mod_info.name.to_lowercase());
        profile.enabled_mod_ids.retain(|id| id != &mod_info.id && id.to_lowercase() != mod_info.name.to_lowercase());
        
        let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }

    // 6. Purge mod from data.mods
    data.mods.retain(|m| m.id != mod_info.id);


    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);

    crate::logger::log(&format!("remove_mod: Mod '{}' successfully purged physically and from database.", mod_info.name));

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn disable_mod(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    crate::profiles::disable_mod_internal(&mut data, &program_path, &mod_id)?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn enable_mod(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    crate::profiles::enable_mod_internal(&mut data, &program_path, &mod_id)?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    Ok(serde_json::json!({ "success": true }))
}


#[tauri::command]
pub fn disable_all_mods(state: State<AppState>) -> Result<Value, String> {
    let mod_ids: Vec<String> = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.iter()
            .filter(|m| m.enabled && m.nexus_author.as_deref() != Some("UE4SS Native Mod"))
            .map(|m| m.id.clone())
            .collect()
    };
    let mut disabled_count = 0u32;
    for mod_id in mod_ids {
        if disable_mod(mod_id, state.clone()).is_ok() {
            disabled_count += 1;
        }
    }
    Ok(serde_json::json!({ "success": true, "disabled": disabled_count }))
}

#[tauri::command]
pub fn enable_all_mods(state: State<AppState>) -> Result<Value, String> {
    let mod_ids: Vec<String> = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.iter()
            .filter(|m| !m.enabled && m.nexus_author.as_deref() != Some("UE4SS Native Mod"))
            .map(|m| m.id.clone())
            .collect()
    };
    let mut enabled_count = 0u32;
    for mod_id in mod_ids {
        if enable_mod(mod_id, state.clone()).is_ok() {
            enabled_count += 1;
        }
    }
    Ok(serde_json::json!({ "success": true, "enabled": enabled_count }))
}

// Internal helpers

fn get_physical_identity(game_path: &str, disabled_path: &str) -> String {
    let path_str = if !game_path.is_empty() { game_path } else { disabled_path };
    if path_str.is_empty() {
        return String::new();
    }
    let path = Path::new(path_str);
    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

fn load_from_modinfo_pmm_json(path: &Path) -> Option<models::ModInfo> {
    if !path.is_dir() {
        return None;
    }
    let pmm_path = path.join("modinfo.pmm.json");
    if !pmm_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&pmm_path).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    let name = meta.get("name").and_then(|n| n.as_str()).unwrap_or("Scanned Mod").to_string();
    let version = meta.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0").to_string();
    let description = meta.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string();
    let author = meta.get("author").and_then(|a| a.as_str()).map(|s| s.to_string());
    let nexus_id = meta.get("nexusModId").and_then(|id| id.as_u64()).map(|n| n as u32);
    let mod_type_str = meta.get("modType").and_then(|t| t.as_str()).unwrap_or("ue4ss");
    
    let mod_type = match mod_type_str.to_lowercase().as_str() {
        "ue4ss" => models::ModType::Ue4ss,
        "palschema" => models::ModType::PalSchema,
        "pak" => models::ModType::Pak,
        "logicmods" => models::ModType::LogicMods,
        _ => models::ModType::Hybrid,
    };

    let path_str = path.to_string_lossy().to_string();
    let is_disabled = path_str.contains("disabled_mods");
    
    let game_path = if is_disabled { String::new() } else { path_str.clone() };
    let disabled_path = if is_disabled { path_str.clone() } else { String::new() };

    Some(models::ModInfo {
        id: name.clone(),
        name: name.clone(),
        mod_type,
        nexus_mod_id: nexus_id,
        nexus_url: nexus_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id)),
        nexus_author: author,
        nexus_summary: Some(description),
        nexus_picture_url: None,
        nexus_endorsements: None,
        nexus_downloads: None,
        version,
        install_date: file_install_date(path),
        source_zip: String::new(),
        config_path: detect_config(path),
        config_type: Some("auto".to_string()),
        enabled: !is_disabled,
        game_path,
        disabled_path,
        pak_destination: None,
        has_enabled_txt: path.join("enabled.txt").exists(),
        mods_txt_order: None,
        extra_files: Vec::new(),
        nexus_description: None,
        nexus_version_cached: None,
        nexus_cached_at: None,
        nexus_category: None,
        nexus_tags: Vec::new(),
        github_repo: None,
        github_version: None,
        github_cached_at: None,
        update_date: None,
        library_zip: None,
        ignored_version: None,
        nexus_file_id: None,
    })
}

fn load_pmm_meta(path: &Path) -> Option<models::ModInfo> {
    let pmm_path = if path.is_file() {
        PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()))
    } else {
        path.join(".pmm.json")
    };
    if pmm_path.exists() {
        if let Ok(content) = fs::read_to_string(&pmm_path) {
            if let Ok(mut mod_info) = serde_json::from_str::<models::ModInfo>(&content) {
                let path_str = path.to_string_lossy().to_string();
                let is_disabled = path_str.contains("disabled_mods");
                if is_disabled {
                    mod_info.disabled_path = path_str;
                    mod_info.game_path = String::new();
                    mod_info.enabled = false;
                } else {
                    mod_info.game_path = path_str;
                    mod_info.disabled_path = String::new();
                    if path.is_file() {
                        mod_info.enabled = true;
                    }
                }
                return Some(mod_info);
            }
        }
    }
    if path.is_dir() {
        if let Some(m) = load_from_modinfo_pmm_json(path) {
            return Some(m);
        }
    }
    None
}

fn merge_scan_with_db(
    current_profile_id: &str,
    installed_ids: &[String],
    db_mods: &[models::ModInfo],
    fs_mods: &[models::ModInfo],
) -> Vec<models::ModInfo> {
    let mut consolidated_db: Vec<models::ModInfo> = Vec::new();
    for db_mod in db_mods {
        let db_path_norm = if !db_mod.game_path.is_empty() {
            db_mod.game_path.replace("\\", "/").to_lowercase()
        } else {
            db_mod.disabled_path.replace("\\", "/").to_lowercase()
        };

        if !db_path_norm.is_empty() {
            let is_extra_of_other = db_mods.iter().any(|other| {
                other.id != db_mod.id && other.extra_files.iter().any(|extra| {
                    extra.replace("\\", "/").to_lowercase() == db_path_norm
                })
            });
            if is_extra_of_other {
                crate::logger::log(&format!("merge_scan_with_db: Purging duplicate sub-component mod '{}' because its files are owned by another mod.", db_mod.name));
                continue;
            }
        }

        let db_id = get_physical_identity(&db_mod.game_path, &db_mod.disabled_path);
        if let Some(existing_idx) = consolidated_db.iter().position(|m| {
            if m.mod_type != db_mod.mod_type {
                return false;
            }
            let existing_id = get_physical_identity(&m.game_path, &m.disabled_path);
            existing_id == db_id
        }) {
            let existing = &mut consolidated_db[existing_idx];
            if existing.nexus_mod_id.is_none() && db_mod.nexus_mod_id.is_some() {
                existing.nexus_mod_id = db_mod.nexus_mod_id;
                existing.nexus_url = db_mod.nexus_url.clone();
                existing.nexus_author = db_mod.nexus_author.clone();
                existing.nexus_summary = db_mod.nexus_summary.clone();
                existing.nexus_picture_url = db_mod.nexus_picture_url.clone();
                existing.nexus_endorsements = db_mod.nexus_endorsements;
                existing.nexus_downloads = db_mod.nexus_downloads;
                existing.name = db_mod.name.clone();
            }
            if existing.game_path.is_empty() && !db_mod.game_path.is_empty() {
                existing.game_path = db_mod.game_path.clone();
                existing.enabled = db_mod.enabled;
            }
            if existing.disabled_path.is_empty() && !db_mod.disabled_path.is_empty() {
                existing.disabled_path = db_mod.disabled_path.clone();
            }
        } else {
            consolidated_db.push(db_mod.clone());
        }
    }

    let mut result: Vec<models::ModInfo> = Vec::new();
    let mut matched_db: Vec<bool> = vec![false; consolidated_db.len()];

    for fs_mod in fs_mods {
        let fs_path_norm = if !fs_mod.game_path.is_empty() {
            fs_mod.game_path.replace("\\", "/").to_lowercase()
        } else {
            fs_mod.disabled_path.replace("\\", "/").to_lowercase()
        };

        if !fs_path_norm.is_empty() {
            let is_registered_as_extra = consolidated_db.iter().any(|dm| {
                dm.extra_files.iter().any(|extra| {
                    extra.replace("\\", "/").to_lowercase() == fs_path_norm
                })
            });
            if is_registered_as_extra {
                continue;
            }
        }

        let fs_id = get_physical_identity(&fs_mod.game_path, &fs_mod.disabled_path);
        if let Some(db_idx) = consolidated_db.iter().position(|dm| {
            let dm_idx = consolidated_db.iter().position(|x| x as *const _ == dm as *const _).unwrap_or(0);
            if matched_db[dm_idx] {
                return false;
            }
            let db_id = get_physical_identity(&dm.game_path, &dm.disabled_path);
            (db_id == fs_id && !db_id.is_empty() && dm.mod_type == fs_mod.mod_type) || 
            (dm.id.to_lowercase() == fs_mod.id.to_lowercase()) ||
            (dm.name.to_lowercase() == fs_mod.name.to_lowercase() && dm.mod_type == fs_mod.mod_type)
        }) {
            matched_db[db_idx] = true;
            let db_mod = &consolidated_db[db_idx];
            let mut merged = db_mod.clone();
            merged.game_path = fs_mod.game_path.clone();
            merged.disabled_path = fs_mod.disabled_path.clone();
            merged.has_enabled_txt = fs_mod.has_enabled_txt;
            if merged.config_path.is_none() {
                merged.config_path = fs_mod.config_path.clone();
            }
            if fs_mod.extra_files.len() > merged.extra_files.len() {
                merged.extra_files = fs_mod.extra_files.clone();
            }
            merged.enabled = fs_mod.enabled;
            result.push(merged);
        } else {
            result.push(fs_mod.clone());
        }
    }

    for (i, dm) in consolidated_db.iter().enumerate() {
        if !matched_db[i] {
            // Check if this mod belongs to the current profile.
            // If it belongs to a different profile, we must NOT purge it or clear its paths.
            let is_installed_in_current = installed_ids.iter().any(|entry| {
                crate::profiles::mod_matches_profile_entry(dm, entry)
            });

            let normalized_disabled = dm.disabled_path.replace("\\", "/");
            let is_disabled_in_other_profile = normalized_disabled.contains("/profiles/") 
                && !normalized_disabled.contains(&format!("/profiles/{}/", current_profile_id));

            if !is_installed_in_current || is_disabled_in_other_profile {
                // Keep the mod exactly as is, without purging or altering paths
                result.push(dm.clone());
                continue;
            }

            let has_game = !dm.game_path.is_empty() && Path::new(&dm.game_path).exists();
            let has_disabled = !dm.disabled_path.is_empty() && Path::new(&dm.disabled_path).exists();
            let has_metadata = dm.nexus_mod_id.is_some() || dm.library_zip.is_some();
 
            if has_game || has_disabled {
                // Still physically on disk somewhere
                result.push(dm.clone());
            } else if has_metadata {
                // Not on disk right now (profile switch moved it), but has Nexus/library metadata
                // Keep it as disabled to preserve all metadata (images, author, downloads, etc.)
                let mut kept = dm.clone();
                kept.enabled = false;
                result.push(kept);
            } else {
                crate::logger::log(&format!("merge_scan_with_db: Mod '{}' no longer exists on disk and has no metadata. Purging from database.", dm.name));
            }
        }
    }



    let mut final_deduped: Vec<models::ModInfo> = Vec::new();
    for m in result {
        if let Some(existing_idx) = final_deduped.iter().position(|em| em.id == m.id) {
            let existing = final_deduped[existing_idx].clone();
            let mut merged = existing.clone();
            if merged.game_path.is_empty() && !m.game_path.is_empty() {
                merged.game_path = m.game_path.clone();
            }
            if merged.disabled_path.is_empty() && !m.disabled_path.is_empty() {
                merged.disabled_path = m.disabled_path.clone();
            }
            for extra in &m.extra_files {
                if !merged.extra_files.contains(extra) {
                    merged.extra_files.push(extra.clone());
                }
            }
            merged.enabled = existing.enabled || m.enabled;
            if merged.config_path.is_none() && m.config_path.is_some() {
                merged.config_path = m.config_path.clone();
            }
            final_deduped[existing_idx] = merged;
        } else {
            final_deduped.push(m);
        }
    }

    final_deduped
}


fn file_install_date(path: &Path) -> String {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            // Format as RFC 3339-like date
            let naive = chrono::DateTime::from_timestamp(secs as i64, 0)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            naive
        })
        .unwrap_or_else(|_| "unknown".to_string())
}

fn scan_ue4ss_mods(dir: &Path, results: &mut Vec<models::ModInfo>) {
    if !dir.exists() { return; }

    let mods_txt_path = dir.join("mods.txt");
    let mut mods_txt_states: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    if mods_txt_path.exists() {
        if let Ok(content) = fs::read_to_string(&mods_txt_path) {
            for line in content.lines() {
                let line_clean = line.trim();
                if line_clean.starts_with(';') || line_clean.starts_with("//") {
                    continue;
                }
                if let Some(pos) = line_clean.find(':') {
                    let name = line_clean[..pos].trim().to_lowercase();
                    let val = line_clean[pos+1..].trim();
                    mods_txt_states.insert(name, val == "1");
                } else if !line_clean.is_empty() {
                    mods_txt_states.insert(line_clean.to_lowercase(), true);
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) { continue; }
            let mod_name = entry.file_name().to_string_lossy().to_string();
            let mod_path = entry.path();
            if ["ConsoleUnlocker", "LuaPlugin", "PalSchema"].contains(&mod_name.as_str()) { continue; }

            let is_native_mod = ["BPModLoaderMod", "CheatManagerEnablerMod", "ConsoleCommandsMod", "ConsoleEnablerMod", "Keybinds", "LineTraceMod", "SplitScreenMod", "BPML_GenericFunctions", "shared", "adapters"].contains(&mod_name.as_str());

            let is_enabled = if let Some(&state) = mods_txt_states.get(&mod_name.to_lowercase()) {
                state
            } else {
                mod_path.join("enabled.txt").exists() || is_native_mod
            };

            if let Some(mut m) = load_pmm_meta(&mod_path) {
                m.enabled = is_enabled;
                results.push(m);
                continue;
            }

            let author = if is_native_mod { Some("UE4SS Native Mod".to_string()) } else { None };
            let summary = if is_native_mod { Some("Core dependency mod installed by UE4SS. Controlled by mods.txt.".to_string()) } else { None };

            results.push(models::ModInfo {
                id: mod_name.clone(),
                name: mod_name.clone(),
                mod_type: models::ModType::Ue4ss,
                nexus_mod_id: None, nexus_url: None, nexus_author: author, nexus_summary: summary,
                nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                version: "1.0.0".to_string(),
                install_date: file_install_date(&mod_path),
                source_zip: String::new(),
                config_path: detect_config(&mod_path),
                config_type: Some("auto".to_string()),
                enabled: is_enabled,
                game_path: mod_path.to_string_lossy().to_string(),
                disabled_path: String::new(),
                pak_destination: None,
                has_enabled_txt: mod_path.join("enabled.txt").exists(),
                mods_txt_order: None,
                extra_files: Vec::new(),
                nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                nexus_category: None, nexus_tags: Vec::new(),
                github_repo: None, github_version: None, github_cached_at: None,
                update_date: None, library_zip: None,
                ignored_version: None,
                nexus_file_id: None,
            });
        }
    }
}


fn scan_palschema_mods(dir: &Path, results: &mut Vec<models::ModInfo>) {
    if !dir.exists() { return; }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) { continue; }
            let mod_name = entry.file_name().to_string_lossy().to_string();
            let mod_path = entry.path();

            if let Some(m) = load_pmm_meta(&mod_path) {
                results.push(m);
                continue;
            }

            let has_json = WalkDir::new(&mod_path).max_depth(2).into_iter().filter_map(|e| e.ok()).any(|e| {
                e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "json" || ext == "jsonc")
            });

            if has_json {
                let install_date = file_install_date(&mod_path);
                results.push(models::ModInfo {
                    id: mod_name.clone(),
                    name: mod_name.clone(),
                    mod_type: models::ModType::PalSchema,
                    nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
                    nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                    version: "unknown".to_string(), install_date,
                    source_zip: String::new(), config_path: detect_config(&mod_path),
                    config_type: Some("auto".to_string()), enabled: true,
                    game_path: mod_path.to_string_lossy().to_string(),
                    disabled_path: String::new(),
                    pak_destination: None, has_enabled_txt: false, mods_txt_order: None,
                    extra_files: Vec::new(),
                    nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                    nexus_category: None, nexus_tags: Vec::new(),
                    github_repo: None, github_version: None, github_cached_at: None,
                    update_date: None, library_zip: None,
                    ignored_version: None,
                    nexus_file_id: None,
                });
            }
        }
    }
}

fn scan_pak_mods(dir: &Path, pak_type: &str, results: &mut Vec<models::ModInfo>) {
    if !dir.exists() { return; }
    for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        let ext = entry.path().extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        if ext != "pak" { continue; }

        let mod_path = entry.path();
        if let Some(m) = load_pmm_meta(&mod_path) {
            results.push(m);
            continue;
        }

        let file_stem = entry.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "unknown".to_string());
        let mod_name = file_stem.trim_end_matches("_P").to_string();
        let mod_path = entry.path();
        let install_date = file_install_date(mod_path);
        let mod_path_str = mod_path.to_string_lossy().to_string();
        let mut extra_files: Vec<String> = Vec::new();
        for companion_ext in &["ucas", "utoc"] {
            let companion_path = dir.join(format!("{}.{}", file_stem, companion_ext));
            if companion_path.exists() {
                extra_files.push(companion_path.to_string_lossy().to_string());
            }
        }
        let mt = if pak_type == "logicmods" { models::ModType::LogicMods } else { models::ModType::Pak };
        results.push(models::ModInfo {
            id: mod_name.clone(), name: mod_name.clone(), mod_type: mt,
            nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
            nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
            version: "unknown".to_string(), install_date,
            source_zip: String::new(), config_path: None, config_type: None,
            enabled: true, game_path: mod_path_str, disabled_path: String::new(),
            pak_destination: Some(pak_type.to_string()), has_enabled_txt: false,
            mods_txt_order: None, extra_files,
            nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
            nexus_category: None, nexus_tags: Vec::new(),
            github_repo: None,
            github_version: None,
            github_cached_at: None,
            update_date: None,
            library_zip: None,
            ignored_version: None,
            nexus_file_id: None,
        });
    }
}

fn scan_disabled_mods(disabled_base: &Path, results: &mut Vec<models::ModInfo>) {
    let type_dirs = [("ue4ss", models::ModType::Ue4ss), ("palschema", models::ModType::PalSchema)];
    for (type_str, mod_type) in &type_dirs {
        let dir = disabled_base.join(type_str);
        if !dir.exists() { continue; }
        if let Ok(rd) = fs::read_dir(&dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if !entry.file_type().map_or(false, |ft| ft.is_dir()) { continue; }
                let mod_name = entry.file_name().to_string_lossy().to_string();
                let mod_path = entry.path();

                if let Some(m) = load_pmm_meta(&mod_path) {
                    results.push(m);
                    continue;
                }

                let install_date = file_install_date(&mod_path);
                results.push(models::ModInfo {
                    id: mod_name.clone(), name: mod_name.clone(), mod_type: mod_type.clone(),
                    nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
                    nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                    version: "unknown".to_string(), install_date,
                    source_zip: String::new(), config_path: detect_config(&mod_path),
                    config_type: Some("auto".to_string()), enabled: false,
                    game_path: String::new(), disabled_path: mod_path.to_string_lossy().to_string(),
                    pak_destination: None, has_enabled_txt: mod_path.join("enabled.txt").exists(),
                    mods_txt_order: None, extra_files: Vec::new(),
                    nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                    nexus_category: None, nexus_tags: Vec::new(),
                    github_repo: None,
                    github_version: None,
                    github_cached_at: None,
                    update_date: None,
                    library_zip: None,
                    ignored_version: None,
                    nexus_file_id: None,
                });
            }
        }
    }

    for (type_str, pak_type) in &[("pak", "pak"), ("logicmods", "logicmods")] {
        let dir = disabled_base.join(type_str);
        if !dir.exists() { continue; }
        if let Ok(rd) = fs::read_dir(&dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if !entry.file_type().map_or(false, |ft| ft.is_file()) { continue; }
                let ext = entry.path().extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                if ext != "pak" { continue; }

                let mod_path = entry.path();
                if let Some(m) = load_pmm_meta(&mod_path) {
                    results.push(m);
                    continue;
                }

                let file_stem = entry.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let mod_name = file_stem.trim_end_matches("_P").to_string();
                let install_date = file_install_date(&entry.path());
                let mt = if *pak_type == "logicmods" { models::ModType::LogicMods } else { models::ModType::Pak };
                results.push(models::ModInfo {
                    id: mod_name.clone(), name: mod_name.clone(), mod_type: mt,
                    nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
                    nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                    version: "unknown".to_string(), install_date,
                    source_zip: String::new(), config_path: None, config_type: None,
                    enabled: false, game_path: String::new(),
                    disabled_path: entry.path().to_string_lossy().to_string(),
                    pak_destination: Some(pak_type.to_string()), has_enabled_txt: false,
                    mods_txt_order: None, extra_files: Vec::new(),
                    nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                    nexus_category: None, nexus_tags: Vec::new(),
                    github_repo: None,
                    github_version: None,
                    github_cached_at: None,
                    update_date: None,
                    library_zip: None,
                    ignored_version: None,
                    nexus_file_id: None,
                });
            }
        }
    }
}

fn detect_config(mod_path: &Path) -> Option<String> {
    let config_names = ["config.json", "settings.json", "options.json"];
    for name in &config_names {
        if mod_path.join(name).exists() {
            return Some(name.to_string());
        }
    }
    let config_subdirs = ["config", "settings"];
    for subdir in &config_subdirs {
        for name in &config_names {
            let full = mod_path.join(subdir).join(name);
            if full.exists() {
                return Some(format!("{}/{}", subdir, name));
            }
        }
    }
    if let Ok(rd) = fs::read_dir(mod_path) {
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                return Some(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
    }
    None
}

#[allow(dead_code)]
pub fn reorder_mods_txt(mods_dir: &Path, mod_name: &str, adding: bool) {
    let mods_txt = mods_dir.join("mods.txt");
    if !mods_txt.exists() {
        if adding {
            let _ = fs::write(&mods_txt, format!("{}\n", mod_name));
        }
        return;
    }

    let content = fs::read_to_string(&mods_txt).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    if adding {
        if !lines.iter().any(|l| l == mod_name) {
            lines.push(mod_name.to_string());
            let _ = fs::write(&mods_txt, lines.join("\n") + "\n");
        }
    } else {
        let before_len = lines.len();
        lines.retain(|l| l != mod_name);
        if lines.len() < before_len {
            let _ = fs::write(&mods_txt, lines.join("\n") + "\n");
        }
    }
}

#[tauri::command]
pub fn open_folder(mod_id: String, state: State<AppState>) -> Result<(), String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;
    let path = if mod_info.enabled { &mod_info.game_path } else { &mod_info.disabled_path };

    let mut dir = std::path::Path::new(path);
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
pub fn rename_mod(mod_id: String, new_name: String, state: State<AppState>) -> Result<models::ModInfo, String> {
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
    let current_dir = std::path::Path::new(current_path);

    let parent = current_dir.parent().ok_or_else(|| "Cannot rename root directory".to_string())?;
    let new_dir = parent.join(&new_name);

    if current_dir != new_dir {
        std::fs::rename(current_dir, &new_dir)
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
pub fn set_mod_version(mod_id: String, version: String, state: State<AppState>) -> Result<models::ModInfo, String> {
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
    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;
    let tag = json["tag_name"].as_str()
        .ok_or_else(|| "No tag_name in response".to_string())?;
    Ok(tag.to_string())
}

#[tauri::command]
pub fn set_github_version(mod_id: String, repo: String, version: String, state: State<AppState>) -> Result<models::ModInfo, String> {
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
    let path = std::path::PathBuf::from(&path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;
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
    let mut dir = std::path::Path::new(first_extra);
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
    use std::fs::File;
    use std::io::{Write, Read};
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
            crate::models::ModType::Ue4ss | crate::models::ModType::Hybrid => "UE4SS",
            crate::models::ModType::PalSchema => "PalSchema",
            crate::models::ModType::Pak => "Paks",
            crate::models::ModType::LogicMods => "LogicMods",
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
    use std::fs::File;
    use std::io::Read;
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
    use std::fs::File;
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
            crate::dependency_checker::get_binaries_dir(&game).join("ue4ss").join("Mods")
        }
        "palschema" => {
            if game_path_str.is_empty() { return Err("Game path not configured".to_string()); }
            let game = std::path::Path::new(game_path_str);
            crate::dependency_checker::get_binaries_dir(&game).join("ue4ss").join("Mods").join("PalSchema").join("mods")
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
) -> Result<models::ModInfo, String> {
    if destination != "~mods" && destination != "LogicMods" {
        return Err("Invalid destination. Must be '~mods' or 'LogicMods'.".to_string());
    }

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path_base = data.settings.game_path.clone();

    let mod_index = data.mods.iter().position(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;
    
    let mut mod_info = data.mods[mod_index].clone();
    if mod_info.mod_type != models::ModType::Pak && mod_info.mod_type != models::ModType::LogicMods {
        return Err("Mod is not a Pak mod".to_string());
    }

    let old_dest = mod_info.pak_destination.clone().unwrap_or_else(|| {
        if mod_info.mod_type == models::ModType::LogicMods {
            "LogicMods".to_string()
        } else {
            "~mods".to_string()
        }
    });

    if old_dest == destination {
        return Ok(mod_info);
    }

    let new_type = if destination == "LogicMods" {
        models::ModType::LogicMods
    } else {
        models::ModType::Pak
    };

    if mod_info.enabled {
        // Move active files in game directory
        let paks_dir = std::path::PathBuf::from(&game_path_base)
            .join("Pal")
            .join("Content")
            .join("Paks");
        
        let old_dir = paks_dir.join(&old_dest);
        let new_dir = paks_dir.join(&destination);
        let _ = std::fs::create_dir_all(&new_dir);

        let file_path = std::path::Path::new(&mod_info.game_path);
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
        // Move disabled files in profile directory
        let profile_dir = std::path::PathBuf::from(&program_path)
            .join("profiles")
            .join(&data.current_profile_id);
        let disabled_base = profile_dir.join("disabled_mods");

        let old_type_dir = if old_dest == "LogicMods" { "logicmods" } else { "pak" };
        let new_type_dir = if destination == "LogicMods" { "logicmods" } else { "pak" };

        let old_dir = disabled_base.join(old_type_dir);
        let new_dir = disabled_base.join(new_type_dir);
        let _ = std::fs::create_dir_all(&new_dir);

        let file_path = std::path::Path::new(&mod_info.disabled_path);
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

    // Save back to db
    data.mods[mod_index] = mod_info.clone();
    let data_clone = data.clone();
    drop(data);
    db::save_db(&program_path, &data_clone).map_err(|e| e.to_string())?;

    Ok(mod_info)
}



