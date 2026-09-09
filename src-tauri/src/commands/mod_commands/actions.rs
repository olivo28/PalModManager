use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use serde_json::Value;
use crate::db;
use crate::models::ModType;
use crate::state::AppState;
use super::utils::filter_mods_for_current_profile;
use super::scan::scan_mods_internal;

#[tauri::command]
pub async fn get_game_version(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let start = std::time::Instant::now();
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
    let start = std::time::Instant::now();
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    crate::profiles::sync_current_profile_states(&mut data);
    let profile_mods = filter_mods_for_current_profile(&data);
    crate::logger::log(&format!("get_mods completed in {:?}", start.elapsed()));
    serde_json::to_value(&profile_mods).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_mods(state: State<'_, AppState>) -> Result<Value, String> {
    crate::logger::log("scan_mods: Starting full disk scan...");
    let start_scan = std::time::Instant::now();
    
    let (game_path, program_path, current_profile_id, installed_ids, mut mods_clone, initial_data) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let current_profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
        let installed_ids = current_profile.map(|p| p.installed_mod_ids.clone()).unwrap_or_default();
        (
            data.settings.game_path.clone(),
            data.settings.program_path.clone(),
            data.current_profile_id.clone(),
            installed_ids,
            data.mods.clone(),
            data.clone(),
        )
    };

    if game_path.is_empty() {
        crate::logger::log("scan_mods: game_path empty, aborting scan");
        return Ok(serde_json::json!([]));
    }

    crate::workshop::cleanup_unsubscribed_workshop_mods(&game_path, &mut mods_clone);

    let mut merged = scan_mods_internal(&game_path, &program_path, &current_profile_id, &installed_ids, &mods_clone);
    
    // Asynchronously fetch missing descriptions/pictures for Workshop mods
    for m in &mut merged {
        if m.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod")) {
            if m.nexus_description.is_none() || m.nexus_description.as_deref() == Some("") {
                let workshop_id = m.nexus_summary.as_ref()
                    .and_then(|s| s.lines().find(|l| l.contains("Workshop ID: ")))
                    .and_then(|l| l.find("Workshop ID: ").and_then(|pos| l[pos + "Workshop ID: ".len()..].trim().parse::<u64>().ok()))
                    .unwrap_or(0);
                if workshop_id > 0 {
                    if let Ok((desc, preview_url)) = crate::workshop::fetch_workshop_metadata(workshop_id).await {
                        m.nexus_description = Some(desc);
                        if !preview_url.is_empty() {
                            m.nexus_picture_url = Some(preview_url);
                        }
                    }
                }
            }
        }
    }

    let (profile_mods, data_to_save) = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods = merged;
        crate::profiles::auto_add_scanned_mods_to_profile(&mut data);
        crate::profiles::cleanup_profile_mod_lists(&mut data);
        crate::profiles::sync_current_profile_states(&mut data);
        let profile_mods = filter_mods_for_current_profile(&data);
        let changed = *data != initial_data;
        let data_clone = if changed { Some(data.clone()) } else { None };
        (profile_mods, data_clone)
    };

    if let Some(data_clone) = data_to_save {
        let _ = db::save_db(&program_path, &data_clone);
        crate::logger::log(&format!("scan_mods: Full disk scan finished in {:?} (changes saved to DB)", start_scan.elapsed()));
    } else {
        crate::logger::log(&format!("scan_mods: Full disk scan finished in {:?} (no changes, skipped DB save)", start_scan.elapsed()));
    }

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

    // Archive config files before purge so they can be restored on future reinstall
    let _ = crate::commands::config_archive::archive_mod_configs(
        &mod_info,
        &program_path,
        &current_profile_id,
        &game_path_str,
    );

    let delete_path_and_sidecar = |path_str: &str| {
        if path_str.is_empty() {
            return;
        }
        let p = Path::new(path_str);
        
        // Handle junction/symlink cleanup safely first
        if junction::exists(p).unwrap_or(false) {
            let _ = crate::profiles::remove_junction_or_symlink(p);
        } else if p.exists() {
            // Guard: Never physically delete the Steam Workshop subscribed download directory!
            let is_steam_workshop_path = path_str.replace('\\', "/").to_lowercase().contains("steamapps/workshop/content");
            if is_steam_workshop_path {
                crate::logger::log(&format!("remove_mod: Preserving Steam Workshop source directory on disk: {}", path_str));
                return;
            }

            if p.is_dir() {
                let _ = fs::remove_dir_all(p);
            } else {
                let _ = fs::remove_file(p);
                for comp_ext in &["ucas", "utoc", "sig"] {
                    let comp_file = p.with_extension(comp_ext);
                    if comp_file.exists() {
                        let _ = fs::remove_file(comp_file);
                    }
                }
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

    delete_path_and_sidecar(&mod_info.game_path);
    delete_path_and_sidecar(&mod_info.disabled_path);

    let mut match_stems: Vec<String> = Vec::new();
    match_stems.push(mod_info.name.to_lowercase());
    let clean_name = mod_info.name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
    if !clean_name.is_empty() {
        match_stems.push(clean_name);
    }
    match_stems.push(mod_info.id.to_lowercase());

    if let Some(ref cfg) = mod_info.config_path {
        delete_path_and_sidecar(cfg);
        if let Some(stem) = Path::new(cfg).file_stem().map(|s| s.to_string_lossy().to_string()) {
            let stem_lower = stem.to_lowercase();
            let stem_clean = stem_lower.replace(|c: char| !c.is_alphanumeric(), "");
            match_stems.push(stem_lower);
            if !stem_clean.is_empty() {
                match_stems.push(stem_clean);
            }
        }
    }

    for extra in &mod_info.extra_files {
        delete_path_and_sidecar(extra);
        if let Some(stem) = Path::new(extra).file_stem().map(|s| s.to_string_lossy().to_string()) {
            let stem_lower = stem.to_lowercase();
            let stem_clean = stem_lower.replace(|c: char| !c.is_alphanumeric(), "");
            match_stems.push(stem_lower);
            if !stem_clean.is_empty() {
                match_stems.push(stem_clean);
            }
        }
    }

    let is_altermatic_framework = mod_info.nexus_mod_id == Some(1626) || mod_info.name.to_lowercase().contains("altermatic");
    let is_unipalui_framework = mod_info.nexus_mod_id == Some(1894) || mod_info.name.to_lowercase().contains("unipalui");

    // Clean up UE4SS and PalSchema folders, storage, and junction artifacts
    if !game_path_str.is_empty() {
        let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path_str));
        let folder_name = crate::profiles::get_mod_folder_name(&mod_info);
        let ue4ss_roots = vec![
            binaries_dir.join("ue4ss").join("Mods"),
            binaries_dir.join("Mods"),
            PathBuf::from(&game_path_str).join("Mods").join("NativeMods").join("UE4SS").join("Mods"),
        ];

        for u_dir in &ue4ss_roots {
            if !u_dir.exists() { continue; }
            let target_mod_folder = u_dir.join(&folder_name);
            if target_mod_folder.exists() {
                let _ = fs::remove_dir_all(&target_mod_folder);
            }
            let target_mod_name = u_dir.join(&mod_info.name);
            if target_mod_name.exists() {
                let _ = fs::remove_dir_all(&target_mod_name);
            }

            let shared_dir = u_dir.join("shared");
            if shared_dir.exists() {
                let s_mod = shared_dir.join(&folder_name);
                if s_mod.exists() {
                    let _ = fs::remove_dir_all(&s_mod);
                }
                let s_name = shared_dir.join(&mod_info.name);
                if s_name.exists() {
                    let _ = fs::remove_dir_all(&s_name);
                }
            }

            let mods_txt = u_dir.join("mods.txt");
            if mods_txt.exists() {
                let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &folder_name);
                let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &mod_info.name);
                
                for extra_path_str in &mod_info.extra_files {
                    let extra_path_lower = extra_path_str.to_lowercase();
                    if extra_path_lower.contains("mods/") {
                        let extra_path = Path::new(extra_path_str);
                        if let Some(extra_folder_name) = extra_path.file_name().map(|n| n.to_string_lossy().to_string()) {
                            let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &extra_folder_name);
                        }
                    }
                }
            }

            // Clean PalSchema Storage & junctions within this ue4ss root
            let palschema_mods_dir = u_dir.join("PalSchema").join("mods");
            let storage_dir = palschema_mods_dir.join("Storage");
            if storage_dir.exists() {
                let mod_storage = storage_dir.join(&folder_name);
                if mod_storage.exists() {
                    let _ = fs::remove_dir_all(&mod_storage);
                }
                let mod_storage_name = storage_dir.join(&mod_info.name);
                if mod_storage_name.exists() {
                    let _ = fs::remove_dir_all(&mod_storage_name);
                }
            }

            if palschema_mods_dir.exists() {
                if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let clean_name = if name.len() > 4 && name.chars().take(3).all(|c| c.is_ascii_digit()) && name.chars().nth(3) == Some('_') {
                            &name[4..]
                        } else {
                            &name
                        };
                        if clean_name.eq_ignore_ascii_case(&folder_name) || clean_name.eq_ignore_ascii_case(&mod_info.name) {
                            let _ = crate::profiles::remove_junction_or_symlink(&path);
                            if path.exists() {
                                let _ = fs::remove_dir_all(&path);
                            }
                        }
                    }
                }
            }
        }
    }

    if mod_info.mod_type == ModType::Pak || mod_info.mod_type == ModType::LogicMods || mod_info.mod_type == ModType::Altermatic {
        if !game_path_str.is_empty() {
            let game_base = PathBuf::from(&game_path_str);
            let check_dirs = vec![
                game_base.join("Pal").join("Content").join("Paks").join("~mods"),
                game_base.join("Pal").join("Content").join("Paks").join("LogicMods"),
                game_base.join("Pal").join("Content").join("Paks").join("~mods").join("SwapJSON"),
                game_base.join("Pal").join("Content").join("Paks").join("~mods").join("AlterConfig"),
                game_base.join("Pal").join("Content").join("Paks").join("~mods").join("JSON_Templates"),
                PathBuf::from(&program_path).join("profiles").join(&current_profile_id).join("disabled_mods").join("pak"),
                PathBuf::from(&program_path).join("profiles").join(&current_profile_id).join("disabled_mods").join("logicmods"),
                PathBuf::from(&program_path).join("profiles").join(&current_profile_id).join("disabled_mods").join("altermatic"),
                PathBuf::from(&program_path).join("profiles").join(&current_profile_id).join("disabled_mods").join("altermatic").join("SwapJSON"),
            ];
            for dir in check_dirs {
                if dir.exists() {
                    if let Ok(entries) = fs::read_dir(&dir) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let file_name = entry.file_name().to_string_lossy().to_string();
                            let file_name_lower = file_name.to_lowercase();
                            let file_stem_lower = entry.path().file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
                            let file_stem_clean = file_stem_lower.replace(|c: char| !c.is_alphanumeric(), "");

                            let is_match = (is_altermatic_framework && (file_name_lower.starts_with("altermatic") || file_name_lower == "_loadlist.json"))
                                || (is_unipalui_framework && file_name_lower.starts_with("unipalui"))
                                || match_stems.iter().any(|stem| {
                                    !stem.is_empty() && (
                                        file_stem_lower == *stem
                                        || file_stem_clean == *stem
                                        || file_name_lower.starts_with(stem)
                                        || (!file_stem_clean.is_empty() && stem.contains(&file_stem_clean))
                                    )
                                });

                            if is_match {
                                let _ = fs::remove_file(entry.path());
                                for comp_ext in &["ucas", "utoc", "sig"] {
                                    let comp_file = entry.path().with_extension(comp_ext);
                                    if comp_file.exists() {
                                        let _ = fs::remove_file(comp_file);
                                    }
                                }
                                let sidecar = PathBuf::from(format!("{}.pmm.json", entry.path().to_string_lossy()));
                                if sidecar.exists() {
                                    let _ = fs::remove_file(sidecar);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let is_workshop = mod_info.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod"));
    if is_workshop {
        let pkg_from_summary = mod_info.nexus_summary.as_ref()
            .and_then(|s| s.lines().find(|l| l.contains("Package Name: ")))
            .and_then(|l| l.find("Package Name: ").map(|pos| l[pos + "Package Name: ".len()..].trim().to_string()));

        let clean_mod_name = mod_info.name.replace(" (Workshop)", "");

        let matches_target = |id: &str| -> bool {
            id.eq_ignore_ascii_case(&mod_info.id)
                || id.eq_ignore_ascii_case(&mod_info.name)
                || id.eq_ignore_ascii_case(&clean_mod_name)
                || pkg_from_summary.as_ref().map_or(false, |pkg| id.eq_ignore_ascii_case(pkg))
                || mod_info.name.to_lowercase().starts_with(&format!("{} (", id.to_lowercase()))
        };

        if let Some(current_prof) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            current_prof.installed_mod_ids.retain(|id| !matches_target(id));
            current_prof.enabled_mod_ids.retain(|id| !matches_target(id));
        }
        let is_used_by_other_profile = data.profiles.iter().any(|p| {
            p.id != current_profile_id && p.installed_mod_ids.iter().any(|id| matches_target(id))
        });
        if !is_used_by_other_profile {
            data.mods.retain(|m| !matches_target(&m.id) && !matches_target(&m.name));
        }

        let p_dir = crate::profiles::get_profile_dir(&program_path, &current_profile_id);
        if let Some(current_prof) = data.profiles.iter().find(|p| p.id == current_profile_id) {
            if let Ok(json) = serde_json::to_string_pretty(current_prof) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }

        if let Some(current_prof) = data.profiles.iter().find(|p| p.id == current_profile_id) {
            if current_prof.dependency_mode == crate::models::DependencyMode::Workshop {
                let package_name = mod_info.nexus_summary.as_ref()
                    .and_then(|s| s.lines().find(|l| l.contains("Package Name: ")))
                    .and_then(|l| l.find("Package Name: ").map(|pos| l[pos + "Package Name: ".len()..].trim().to_string()))
                    .unwrap_or_else(|| mod_info.id.clone());
                let wmods = crate::workshop::scan_workshop_mods(&game_path_str);
                if let Some(target) = wmods.iter().find(|m| m.package_name == package_name) {
                    let _ = crate::workshop::deactivate_workshop_mod(&game_path_str, target, false);
                }
            }
        }
    } else {
        for profile in &mut data.profiles {
            profile.installed_mod_ids.retain(|id| id != &mod_info.id && id.to_lowercase() != mod_info.name.to_lowercase());
            profile.enabled_mod_ids.retain(|id| id != &mod_info.id && id.to_lowercase() != mod_info.name.to_lowercase());
        }
        data.mods.retain(|m| m.id != mod_info.id);
    }

    crate::profiles::cleanup_profile_mod_lists(&mut data);
    crate::profiles::sync_current_profile_states(&mut data);

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    sync_altermatic_helper(&data_clone);

    crate::logger::log(&format!("remove_mod: Mod '{}' successfully purged physically and from database.", mod_info.name));
    Ok(serde_json::json!({ "success": true }))
}

fn sync_altermatic_helper(data: &crate::models::AppData) {
    if !data.settings.game_path.is_empty() {
        let game_path = PathBuf::from(&data.settings.game_path);
        let current_profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
        let enabled_mod_ids: Vec<String> = if let Some(p) = current_profile {
            p.enabled_mod_ids.clone()
        } else {
            data.mods.iter().filter(|m| m.enabled).map(|m| m.id.clone()).collect()
        };
        let _ = crate::altermatic::sync_load_list(&game_path, &enabled_mod_ids, &data.mods);
    }
}

#[tauri::command]
pub fn disable_mod(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    crate::profiles::disable_mod_internal(&mut data, &program_path, &mod_id)?;
    crate::profiles::sync_current_profile_states(&mut data);
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    sync_altermatic_helper(&data_clone);
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn enable_mod(mod_id: String, state: State<AppState>) -> Result<Value, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    crate::profiles::enable_mod_internal(&mut data, &program_path, &mod_id)?;
    crate::profiles::sync_current_profile_states(&mut data);
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    sync_altermatic_helper(&data_clone);
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

#[tauri::command]
pub fn merge_mods_as_hybrid(
    primary_mod_id: String,
    secondary_mod_id: String,
    state: State<AppState>,
) -> Result<crate::models::ModInfo, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();

    let primary_idx = data.mods.iter().position(|m| m.id == primary_mod_id)
        .ok_or_else(|| format!("Primary mod '{}' not found", primary_mod_id))?;
    let secondary_idx = data.mods.iter().position(|m| m.id == secondary_mod_id)
        .ok_or_else(|| format!("Secondary mod '{}' not found", secondary_mod_id))?;

    if primary_idx == secondary_idx {
        return Err("Cannot merge a mod with itself".to_string());
    }

    let secondary = data.mods[secondary_idx].clone();
    let primary = &mut data.mods[primary_idx];

    crate::logger::log(&format!(
        "merge_mods_as_hybrid: Merging '{}' (id: {}) with secondary '{}' (id: {})",
        primary.name, primary.id, secondary.name, secondary.id
    ));

    // Combine paths into extra_files
    let mut all_paths: Vec<String> = Vec::new();
    if !primary.game_path.is_empty() {
        all_paths.push(primary.game_path.clone());
    }
    if !secondary.game_path.is_empty() && !all_paths.contains(&secondary.game_path) {
        all_paths.push(secondary.game_path.clone());
    }
    for extra in &primary.extra_files {
        if !all_paths.contains(extra) {
            all_paths.push(extra.clone());
        }
    }
    for extra in &secondary.extra_files {
        if !all_paths.contains(extra) {
            all_paths.push(extra.clone());
        }
    }

    // Sort by canonical priority: UE4SS > PalSchema > LogicMods/~mods (.pak)
    if all_paths.len() > 1 {
        all_paths.sort_by_key(|p| crate::commands::mod_commands::scan::merge::get_path_priority(p));
        primary.game_path = all_paths[0].clone();
        primary.extra_files = all_paths[1..].to_vec();
    }

    primary.mod_type = ModType::Hybrid;

    // Inherit metadata if primary was missing it
    if primary.nexus_mod_id.is_none() && secondary.nexus_mod_id.is_some() {
        primary.nexus_mod_id = secondary.nexus_mod_id;
        primary.nexus_url = secondary.nexus_url.clone();
        primary.nexus_author = secondary.nexus_author.clone();
        primary.nexus_summary = secondary.nexus_summary.clone();
        primary.nexus_picture_url = secondary.nexus_picture_url.clone();
    }
    if primary.config_path.is_none() && secondary.config_path.is_some() {
        primary.config_path = secondary.config_path.clone();
        primary.config_type = secondary.config_type.clone();
    }

    // Save updated PMM metadata for primary
    let _ = crate::profiles::save_pmm_meta(primary);

    let final_primary = primary.clone();

    // Remove secondary from data.mods
    data.mods.retain(|m| m.id != secondary_mod_id);

    // Remove secondary from profile mod lists and migrate folder assignments
    for profile in &mut data.profiles {
        profile.installed_mod_ids.retain(|id| id != &secondary_mod_id);
        profile.enabled_mod_ids.retain(|id| id != &secondary_mod_id);

        let primary_id = final_primary.id.clone();
        for folder in &mut profile.mod_folders {
            let mut had_secondary = false;
            folder.mod_ids.retain(|id| {
                if id == &secondary_mod_id {
                    had_secondary = true;
                    false
                } else {
                    true
                }
            });
            if had_secondary && !folder.mod_ids.contains(&primary_id) {
                folder.mod_ids.push(primary_id.clone());
            }
        }
    }

    crate::profiles::cleanup_profile_mod_lists(&mut data);
    crate::profiles::sync_current_profile_states(&mut data);

    let data_clone = data.clone();
    drop(data);

    let _ = db::save_db(&program_path, &data_clone);

    Ok(final_primary)
}
