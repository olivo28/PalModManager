use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use crate::dependency_checker;
use crate::state::AppState;

pub fn empty_status() -> dependency_checker::DependencyStatus {
    dependency_checker::DependencyStatus {
        ue4ss_installed: false,
        ue4ss_version: None,
        ue4ss_latest_tag: None,
        ue4ss_latest_date: None,
        ue4ss_needs_update: false,
        ue4ss_install_mode: "NotFound".to_string(),
        palschema_installed: false,
        palschema_version: None,
        palschema_latest_version: None,
        palschema_needs_update: false,
        game_platform: "Unknown".to_string(),
        has_dll_conflict: false,
        conflicting_dlls: Vec::new(),
        ue4ss_updated_from: None,
        palschema_updated_from: None,
        altermatic_installed: false,
        unipalui_installed: false,
    }
}

pub fn parse_dmy(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y").ok()
}

#[tauri::command]
pub fn check_dependencies(state: State<AppState>) -> Result<dependency_checker::DependencyStatus, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() {
        return Ok(empty_status());
    }
    let mut status = dependency_checker::check_dependencies(&game_path);

    if !program_path.is_empty() {
        let cache_dir = PathBuf::from(&program_path);
        let ue4ss_cache_file = cache_dir.join(".ue4ss_last_ver");
        let palschema_cache_file = cache_dir.join(".palschema_last_ver");

        if let Some(ref cur_ue4ss) = status.ue4ss_version {
            if ue4ss_cache_file.exists() {
                if let Ok(prev) = fs::read_to_string(&ue4ss_cache_file) {
                    let prev_clean = prev.trim();
                    let is_prev_date = prev_clean.contains('.');
                    let is_cur_date = cur_ue4ss.contains('.');
                    // Only trigger if both are dates or neither is a date, avoiding false update toast on mode switch
                    if !prev_clean.is_empty() && prev_clean != cur_ue4ss && prev_clean != "unknown" && prev_clean != "Workshop" && cur_ue4ss != "Workshop" && (is_prev_date == is_cur_date) {
                        status.ue4ss_updated_from = Some(prev_clean.to_string());
                    }
                }
            }
            let _ = fs::write(&ue4ss_cache_file, cur_ue4ss);
        }

        if let Some(ref cur_schema) = status.palschema_version {
            if palschema_cache_file.exists() {
                if let Ok(prev) = fs::read_to_string(&palschema_cache_file) {
                    let prev_clean = prev.trim();
                    let is_prev_date = prev_clean.contains('.');
                    let is_cur_date = cur_schema.contains('.');
                    if !prev_clean.is_empty() && prev_clean != cur_schema && prev_clean != "unknown" && prev_clean != "Workshop" && cur_schema != "Workshop" && (is_prev_date == is_cur_date) {
                        status.palschema_updated_from = Some(prev_clean.to_string());
                    }
                }
            }
            let _ = fs::write(&palschema_cache_file, cur_schema);
        }
    }

    Ok(status)
}

#[tauri::command]
pub fn clean_conflict_dlls(state: State<AppState>) -> Result<Vec<String>, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() {
        return Err("Game path is not set".to_string());
    }

    let profile = dependency_checker::build_game_profile(Path::new(&game_path));
    let quarantine_dir = PathBuf::from(&program_path).join("dll_quarantine");
    let _ = fs::create_dir_all(&quarantine_dir);

    let mut removed = Vec::new();
    for dll in &["dwmapi.dll", "xinput1_3.dll"] {
        let src = profile.binaries_dir.join(dll);
        if src.exists() {
            let dst = quarantine_dir.join(dll);
            let _ = fs::copy(&src, &dst);
            let _ = fs::remove_file(&src);
            removed.push(dll.to_string());
            crate::logger::log(&format!("clean_conflict_dlls: Moved {:?} to quarantine {:?}", src, dst));
        }
    }
    Ok(removed)
}

#[tauri::command]
pub fn reset_workshop_cache(state: State<AppState>) -> Result<(), String> {
    let game_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.game_path.clone()
    };
    if game_path.is_empty() {
        return Err("Game path is not set".to_string());
    }

    let managed_ue4ss = PathBuf::from(&game_path)
        .join("Mods")
        .join("ManagedMods")
        .join("UE4SSExperimentalPW");

    if managed_ue4ss.exists() {
        fs::remove_dir_all(&managed_ue4ss).map_err(|e| format!("Failed to delete workshop cache folder: {}", e))?;
        crate::logger::log(&format!("reset_workshop_cache: Deleted {:?}", managed_ue4ss));
    }
    Ok(())
}

#[tauri::command]
pub async fn check_ue4ss_latest() -> Result<String, String> {
    // Return the tag name for display
    let (tag, _date) = dependency_checker::check_ue4ss_latest().await?;
    Ok(tag)
}

#[tauri::command]
pub async fn check_palschema_latest() -> Result<String, String> {
    dependency_checker::check_palschema_latest().await
}

pub fn compare_versions(local: &str, remote: &str) -> bool {
    let local_clean = local.trim_start_matches('v').trim();
    let remote_clean = remote.trim_start_matches('v').trim();

    let local_parts: Vec<&str> = local_clean.split('.').collect();
    let remote_parts: Vec<&str> = remote_clean.split('.').collect();

    let max_len = std::cmp::max(local_parts.len(), remote_parts.len());
    for i in 0..max_len {
        let l = local_parts.get(i).unwrap_or(&"0");
        let r = remote_parts.get(i).unwrap_or(&"0");

        let l_num: u32 = l.parse().unwrap_or(0);
        let r_num: u32 = r.parse().unwrap_or(0);

        if l_num != r_num {
            return false;
        }
    }
    true
}

#[tauri::command]
pub async fn check_dependencies_full(state: State<'_, AppState>) -> Result<dependency_checker::DependencyStatus, String> {
    let game_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.game_path.clone()
    };
    if game_path.is_empty() {
        return Ok(empty_status());
    }

    let mut status = dependency_checker::check_dependencies(&game_path);

    let is_workshop = status.ue4ss_install_mode == "Workshop";

    if let Ok((ue4ss_tag, ue4ss_date)) = dependency_checker::check_ue4ss_latest().await {
        status.ue4ss_latest_tag = Some(ue4ss_tag.clone());
        status.ue4ss_latest_date = Some(ue4ss_date.clone());
        if is_workshop {
            // Check if Workshop staging has a pending update for UE4SS
            let w_state = crate::workshop::scan_workshop_mods(&game_path);
            if let Some(w_mod) = w_state.iter().find(|m| m.is_framework && (m.package_name.to_lowercase().contains("ue4ss") || m.package_name == "UE4SSExperimentalPW")) {
                status.ue4ss_needs_update = w_mod.has_pending_update || (w_mod.is_installed && w_mod.installed_version.is_some() && w_mod.installed_version.as_ref() != Some(&w_mod.version));
                if status.ue4ss_needs_update {
                    status.ue4ss_latest_tag = Some(format!("v{}", w_mod.version));
                }
            } else {
                status.ue4ss_needs_update = false;
            }
            crate::logger::log(&format!("UE4SS check [Workshop]: local='{:?}', needs_update={}", status.ue4ss_version, status.ue4ss_needs_update));
        } else {
            status.ue4ss_needs_update = match &status.ue4ss_version {
                Some(local) if local == "Workshop" => {
                    crate::logger::log("UE4SS check: installed via Steam Workshop");
                    false
                }
                Some(local) => {
                    let needs_up = match (parse_dmy(local.trim()), parse_dmy(ue4ss_date.trim())) {
                        (Some(l), Some(r)) => l < r,
                        _ => true,
                    };
                    crate::logger::log(&format!("UE4SS check: local='{}', remote='{}' (tag: {}), match={}", local, ue4ss_date, ue4ss_tag, !needs_up));
                    needs_up
                }
                None => {
                    crate::logger::log("UE4SS check: local is None (not installed or version not read)");
                    true
                }
            };
        }
    }

    if let Ok(ps_version) = dependency_checker::check_palschema_latest().await {
        status.palschema_latest_version = Some(ps_version.clone());
        if is_workshop {
            // Check if Workshop staging has a pending update for PalSchema
            let w_state = crate::workshop::scan_workshop_mods(&game_path);
            if let Some(w_mod) = w_state.iter().find(|m| m.package_name.eq_ignore_ascii_case("PalSchema")) {
                status.palschema_needs_update = w_mod.has_pending_update || (w_mod.is_installed && w_mod.installed_version.is_some() && w_mod.installed_version.as_ref() != Some(&w_mod.version));
                if status.palschema_needs_update {
                    status.palschema_latest_version = Some(w_mod.version.clone());
                }
            } else {
                status.palschema_needs_update = false;
            }
            crate::logger::log(&format!("PalSchema check [Workshop]: local='{:?}', needs_update={}", status.palschema_version, status.palschema_needs_update));
        } else {
            status.palschema_needs_update = match &status.palschema_version {
                Some(local) if local == "Workshop" => {
                    crate::logger::log("PalSchema check: installed via Steam Workshop");
                    false
                }
                Some(local) => {
                    let eq = compare_versions(local, &ps_version);
                    crate::logger::log(&format!("PalSchema check: local='{}', remote='{}', match={}", local, ps_version, eq));
                    !eq
                }
                None => {
                    crate::logger::log("PalSchema check: local is None (not installed or version not read)");
                    true
                }
            };
        }
    }

    Ok(status)
}
