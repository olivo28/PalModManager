use crate::dependency_checker;
use crate::state::AppState;
use crate::zip_handler;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

fn empty_status() -> dependency_checker::DependencyStatus {
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

fn parse_dmy(s: &str) -> Option<chrono::NaiveDate> {
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
pub fn get_safety_backup_info_command(state: State<AppState>) -> Result<crate::safety_backup::SafetyBackupInfo, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    crate::safety_backup::get_safety_backup_info(&program_path, &game_path)
}

#[tauri::command]
pub fn trigger_safety_backup_command(state: State<AppState>) -> Result<bool, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    crate::safety_backup::create_initial_safety_backup(&game_path, &program_path, true)
}

#[tauri::command]
pub fn restore_safety_backup_command(state: State<AppState>) -> Result<(), String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    crate::safety_backup::restore_initial_safety_backup(&game_path, &program_path)
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

fn compare_versions(local: &str, remote: &str) -> bool {
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

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Cannot create dest dir: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Cannot read source dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();
        let dest_path = dst.join(file_name);
        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| {
                format!("Cannot copy file {}: {}", file_name.to_string_lossy(), e)
            })?;
        }
    }
    Ok(())
}

fn find_extracted_root(src: &Path) -> PathBuf {
    if src.is_dir() {
        let entries: Vec<_> = fs::read_dir(src)
            .ok()
            .into_iter()
            .flat_map(|rd| rd.filter_map(|e| e.ok()))
            .filter(|e| {
                let n = e.file_name();
                n != ".." && n != "." && n != "__MACOSX"
            })
            .collect();
        if entries.len() == 1 && entries[0].file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            return entries[0].path();
        }
    }
    src.to_path_buf()
}

pub fn get_vault_dir(program_path: &str, dep_type: &str) -> PathBuf {
    let folder = match dep_type.to_lowercase().as_str() {
        "palschema" => "PalSchema",
        _ => "UE4SS",
    };
    PathBuf::from(program_path).join("mods-library").join("dependencies").join(folder)
}

pub fn sanitize_version_tag(raw: &str, _dep_type: &str) -> String {
    let mut s = raw.trim();
    if let Some(stripped) = s.strip_suffix(".zip").or_else(|| s.strip_suffix(".ZIP")) {
        s = stripped.trim();
    }

    let prefixes = [
        "palschema - ", "palschema_", "palschema-", "palschema ", "palschema.",
        "ue4ss - ", "ue4ss_", "ue4ss-", "ue4ss ", "ue4ss.",
        "re-ue4ss - ", "re-ue4ss_", "re-ue4ss-", "re-ue4ss ",
        "ue4ss-palworld-", "ue4ss-palworld_", "ue4ss-palworld "
    ];

    let mut changed = true;
    while changed {
        changed = false;
        let lower = s.to_lowercase();
        for p in &prefixes {
            if lower.starts_with(p) {
                s = s[p.len()..].trim();
                changed = true;
                break;
            }
        }
    }

    if s.starts_with('(') && s.ends_with(')') && s.len() > 2 {
        s = s[1..s.len()-1].trim();
    }

    if s.is_empty() {
        return "custom".to_string();
    }

    s.to_string()
}

pub fn extract_version_from_vault_filename(filename: &str, dep_type: &str) -> String {
    sanitize_version_tag(filename, dep_type)
}

pub fn save_to_vault(program_path: &str, dep_type: &str, version: &str, zip_bytes: &[u8]) -> Result<PathBuf, String> {
    let vault_dir = get_vault_dir(program_path, dep_type);
    let _ = fs::create_dir_all(&vault_dir);
    let clean_ver = sanitize_version_tag(version, dep_type);
    let safe_ver = clean_ver.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    let filename = match dep_type.to_lowercase().as_str() {
        "palschema" => format!("PalSchema - {}.zip", safe_ver),
        _ => format!("UE4SS - {}.zip", safe_ver),
    };
    let target_path = vault_dir.join(&filename);
    fs::write(&target_path, zip_bytes).map_err(|e| format!("Failed to save archive to vault: {}", e))?;
    Ok(target_path)
}

fn migrate_legacy_dependency_zips(program_path: &str) {
    let base = PathBuf::from(program_path).join("mods-library").join("dependencies");
    let legacy_ue4ss = base.join("ue4ss.zip");
    let legacy_ue4ss_ver = base.join("ue4ss.version");
    if legacy_ue4ss.exists() {
        let ver = fs::read_to_string(&legacy_ue4ss_ver).unwrap_or_else(|_| "10.08.2026".to_string()).trim().to_string();
        let vault_dir = base.join("UE4SS");
        let _ = fs::create_dir_all(&vault_dir);
        let dest = vault_dir.join(format!("UE4SS - {}.zip", ver));
        if !dest.exists() {
            let _ = fs::copy(&legacy_ue4ss, &dest);
        }
    }

    let legacy_ps = base.join("palschema.zip");
    let legacy_ps_ver = base.join("palschema.version");
    if legacy_ps.exists() {
        let ver = fs::read_to_string(&legacy_ps_ver).unwrap_or_else(|_| "0.6.4".to_string()).trim().to_string();
        let vault_dir = base.join("PalSchema");
        let _ = fs::create_dir_all(&vault_dir);
        let dest = vault_dir.join(format!("PalSchema - {}.zip", ver));
        if !dest.exists() {
            let _ = fs::copy(&legacy_ps, &dest);
        }
    }
}

pub async fn apply_ue4ss_zip_bytes(
    zip_bytes: &[u8],
    publish_date: &str,
    program_path: &str,
    game_path: &str,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let ue4ss_dir = win64.join("ue4ss");

    let temp_dir = std::env::temp_dir().join("pmm_ue4ss");
    let zip_path = temp_dir.join("ue4ss.zip");
    crate::logger::log(&format!("install_ue4ss: Saving temporary ZIP to {}", zip_path.display()));
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    fs::write(&zip_path, zip_bytes).map_err(|e| e.to_string())?;

    crate::logger::log("install_ue4ss: Extracting ZIP archive...");
    let extracted = zip_handler::extract_zip_to_temp(&zip_path.to_string_lossy(), &temp_dir.join("extracted"))?;
    let root = find_extracted_root(&extracted);

    let (framework_src, dwmapi_src) = {
        let ue4ss_sub = root.join("ue4ss");
        if ue4ss_sub.is_dir() {
            (ue4ss_sub, root.join("dwmapi.dll"))
        } else {
            (root.clone(), root.join("dwmapi.dll"))
        }
    };

    if dwmapi_src.exists() {
        let dwmapi_dst = win64.join("dwmapi.dll");
        crate::logger::log(&format!("install_ue4ss: Copying dwmapi.dll to {}", dwmapi_dst.display()));
        fs::copy(&dwmapi_src, &dwmapi_dst).map_err(|e| e.to_string())?;
    }

    if !ue4ss_dir.exists() {
        crate::logger::log(&format!("install_ue4ss: Creating target directory ue4ss at {}", ue4ss_dir.display()));
        fs::create_dir_all(&ue4ss_dir).map_err(|e| format!("Cannot create ue4ss directory: {}", e))?;
    }

    crate::logger::log(&format!("install_ue4ss: Copying framework content to {}", ue4ss_dir.display()));
    if let Ok(rd) = fs::read_dir(&framework_src) {
        for entry in rd.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "dwmapi.dll" || name == "Mods" || name == "mods" { continue; }
            let dst = ue4ss_dir.join(&name);
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                copy_dir_all(&entry.path(), &dst)?;
            } else {
                fs::copy(&entry.path(), &dst).map_err(|e| e.to_string())?;
            }
        }
    }

    // Copiar Mods que vienen por defecto en UE4SS sin sobreescribir la carpeta completa
    let framework_mods = framework_src.join("Mods");
    if framework_mods.exists() {
        let dest_mods = ue4ss_dir.join("Mods");
        let _ = fs::create_dir_all(&dest_mods);
        if let Ok(rd) = fs::read_dir(&framework_mods) {
            for entry in rd.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                let dst = dest_mods.join(&name);
                if !dst.exists() {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        let _ = copy_dir_all(&entry.path(), &dst);
                    } else {
                        let _ = fs::copy(&entry.path(), &dst);
                    }
                }
            }
        }
    }

    let version_file = ue4ss_dir.join("ue4ss.version");
    crate::logger::log(&format!("install_ue4ss: Writing version '{}' to {}", publish_date, version_file.display()));
    let _ = fs::write(&version_file, publish_date);

    // Escribir enabled.txt vacíos y registrar mods nativos de UE4SS en DB
    let dest_mods = ue4ss_dir.join("Mods");
    let mods_txt_path = ue4ss_dir.join("mods.txt");
    let mut native_mods_to_add: Vec<crate::models::ModInfo> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dest_mods) {
        for entry in rd.filter_map(|e| e.ok()) {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let mod_name = entry.file_name().to_string_lossy().to_string();
                if mod_name.to_lowercase() == "palschema" { continue; }

                let mod_path = entry.path();
                let is_enabled = if mods_txt_path.exists() {
                    let mut found_val = true;
                    if let Ok(content) = fs::read_to_string(&mods_txt_path) {
                        for line in content.lines() {
                            let line_clean = line.trim();
                            if line_clean.starts_with(';') || line_clean.starts_with("//") {
                                continue;
                            }
                            if let Some(pos) = line_clean.find(':') {
                                let name = line_clean[..pos].trim();
                                let val = line_clean[pos+1..].trim();
                                if name.to_lowercase() == mod_name.to_lowercase() {
                                    found_val = val == "1";
                                    break;
                                }
                            } else if line_clean.to_lowercase() == mod_name.to_lowercase() {
                                found_val = true;
                                break;
                            }
                        }
                    }
                    found_val
                } else {
                    true
                };

                native_mods_to_add.push(crate::models::ModInfo {
                    id: mod_name.clone(),
                    name: mod_name.clone(),
                    mod_type: crate::models::ModType::Ue4ss,
                    nexus_mod_id: None,
                    nexus_url: None,
                    nexus_author: Some("UE4SS Native Mod".to_string()),
                    nexus_summary: Some("Core dependency mod installed by UE4SS. Recommended not to disable for safety.".to_string()),
                    nexus_picture_url: None,
                    nexus_endorsements: None,
                    nexus_downloads: None,
                    version: "1.0.0".to_string(),
                    install_date: chrono::Utc::now().to_rfc3339(),
                    source_zip: "ue4ss_framework.zip".to_string(),
                    config_path: None,
                    config_type: Some("auto".to_string()),
                    enabled: is_enabled,
                    game_path: mod_path.to_string_lossy().to_string(),
                    disabled_path: String::new(),
                    pak_destination: None,
                    has_enabled_txt: false,
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
                    ignored_keys: None,
                    has_pending_update: None,
                    origin_load_method: Some("mods_txt".to_string()),
                    custom_notes: None,
                });
            }
        }
    }

    {
        let clean_ver = sanitize_version_tag(publish_date, "ue4ss");
        let version_file = ue4ss_dir.join("ue4ss.version");
        let _ = fs::write(&version_file, &clean_ver);

        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        for nm in native_mods_to_add {
            if !data.mods.iter().any(|m| m.name.to_lowercase() == nm.name.to_lowercase()) {
                data.mods.push(nm);
            }
        }
        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.ue4ss_enabled = true;
            profile.dependency_mode = crate::models::DependencyMode::Standard;
            let p_dir = crate::profiles::get_profile_dir(program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
            let ue4ss_backup = p_dir.join("ue4ss");
            let dwmapi_backup = p_dir.join("dwmapi.dll");
            if ue4ss_dir.exists() {
                let _ = copy_dir_all(&ue4ss_dir, &ue4ss_backup);
            }
            if win64.join("dwmapi.dll").exists() {
                let _ = fs::copy(win64.join("dwmapi.dll"), &dwmapi_backup);
            }
            crate::logger::log(&format!("install_ue4ss: Updated profile '{}' with ue4ss_enabled=true, dependency_mode=Standard, and synced backup.", profile.id));
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(program_path, &data_clone);
    }

    crate::logger::log("install_ue4ss: Cleaning temporary directory...");
    let _ = fs::remove_dir_all(&temp_dir);
    crate::logger::log("install_ue4ss: Installation completed successfully.");
    Ok(format!("UE4SS ({}) installed successfully.", publish_date))
}

pub async fn apply_palschema_zip_bytes(
    zip_bytes: &[u8],
    tag: &str,
    program_path: &str,
    game_path: &str,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let palschema_dir = if win64.join("dwmapi.dll").exists() {
        win64.join("ue4ss").join("Mods").join("PalSchema")
    } else {
        Path::new(game_path).join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("PalSchema")
    };

    let temp_dir = std::env::temp_dir().join("pmm_palschema");
    let zip_path = temp_dir.join("palschema.zip");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    fs::write(&zip_path, zip_bytes).map_err(|e| e.to_string())?;

    let extracted = zip_handler::extract_zip_to_temp(&zip_path.to_string_lossy(), &temp_dir.join("extracted"))?;
    let root = find_extracted_root(&extracted);

    if !palschema_dir.exists() {
        fs::create_dir_all(&palschema_dir).map_err(|e| e.to_string())?;
    }
    if let Ok(rd) = fs::read_dir(&root) {
        for entry in rd.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "mods" || name == "Mods" { continue; }
            let dst = palschema_dir.join(&name);
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                copy_dir_all(&entry.path(), &dst)?;
            } else {
                fs::copy(&entry.path(), &dst).map_err(|e| e.to_string())?;
            }
        }
    }

    let clean_tag = sanitize_version_tag(tag, "palschema");
    let version_file = palschema_dir.join("palschema.version");
    crate::logger::log(&format!("install_palschema: Escribiendo versión '{}' en {}", clean_tag, version_file.display()));
    let _ = fs::write(&version_file, &clean_tag);

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.palschema_enabled = true;
            if profile.dependency_mode == crate::models::DependencyMode::None {
                profile.dependency_mode = crate::models::DependencyMode::Standard;
            }
            let p_dir = crate::profiles::get_profile_dir(program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
            let palschema_backup = p_dir.join("palschema");
            if palschema_dir.exists() {
                let _ = copy_dir_all(&palschema_dir, &palschema_backup);
            }
            crate::logger::log(&format!("install_palschema: Updated profile '{}' with palschema_enabled=true and synced backup.", profile.id));
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(program_path, &data_clone);
    }

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(format!("PalSchema ({}) installed successfully.", tag))
}

#[tauri::command]
pub async fn install_ue4ss(force_download: bool, state: State<'_, AppState>) -> Result<String, String> {
    crate::logger::log("install_ue4ss: Starting UE4SS installation process...");
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() {
        crate::logger::log("install_ue4ss: Error - Game path not configured.");
        return Err("Game path not set".to_string());
    }

    let dep_status = crate::dependency_checker::check_dependencies(&game_path);
    if dep_status.ue4ss_installed && !force_download {
        crate::logger::log("install_ue4ss: UE4SS is already installed and force_download is false. Skipping installation.");
        return Ok("UE4SS is already installed.".to_string());
    }

    migrate_legacy_dependency_zips(&program_path);

    let client = reqwest::Client::builder()
        .user_agent("PalModManager/1.7.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut asset_url = String::new();
    let mut publish_date = String::new();
    let mut api_success = false;

    let release_url = "https://api.github.com/repos/Okaetsu/RE-UE4SS/releases/tags/experimental-palworld";
    crate::logger::log(&format!("install_ue4ss: Fetching GitHub API release from {}", release_url));
    if let Ok(resp) = client.get(release_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(assets) = json["assets"].as_array() {
                    if let Some(asset) = assets.iter().find(|a| {
                        a["name"].as_str().map_or(false, |n| n.ends_with(".zip") && !n.contains("symbols"))
                    }) {
                        if let Some(url) = asset["browser_download_url"].as_str() {
                            asset_url = url.to_string();
                            let mut latest_asset_dt: Option<chrono::DateTime<chrono::FixedOffset>> = None;
                            for a in assets {
                                if let Some(updated) = a["updated_at"].as_str() {
                                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(updated) {
                                        match latest_asset_dt {
                                            Some(cur) if dt > cur => latest_asset_dt = Some(dt),
                                            None => latest_asset_dt = Some(dt),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            if let Some(dt) = latest_asset_dt {
                                publish_date = dt.format("%d.%m.%Y").to_string();
                            }
                            api_success = true;
                        }
                    }
                }
            }
        }
    }

    if !api_success {
        crate::logger::log("install_ue4ss: GitHub API rate limited or failed. Using HTML fallback...");
        if let Ok(r) = client.get("https://github.com/Okaetsu/RE-UE4SS/releases/tag/experimental-palworld").send().await {
            if let Ok(html) = r.text().await {
                let mut search_pos = 0;
                while let Some(pos) = html[search_pos..].find("/Okaetsu/RE-UE4SS/releases/download/experimental-palworld/") {
                    let start = search_pos + pos;
                    if let Some(end_quote) = html[start..].find('"') {
                        let url_path = &html[start..start + end_quote];
                        search_pos = start + end_quote;
                        let lower = url_path.to_lowercase();
                        if lower.ends_with(".zip") && !lower.contains("symbols") {
                            asset_url = format!("https://github.com{}", url_path);
                            let mut latest_html_dt: Option<chrono::DateTime<chrono::Utc>> = None;
                            let mut cursor = 0;
                            while let Some(pos) = html[cursor..].find("datetime=\"") {
                                let time_start = cursor + pos + 10;
                                if let Some(len) = html[time_start..].find('"') {
                                    let dt_raw = &html[time_start..time_start + len];
                                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_raw) {
                                        let dt_utc: chrono::DateTime<chrono::Utc> = dt.into();
                                        match latest_html_dt {
                                            Some(cur) if dt_utc > cur => { latest_html_dt = Some(dt_utc); }
                                            None => { latest_html_dt = Some(dt_utc); }
                                            _ => {}
                                        }
                                    }
                                    cursor = time_start + len;
                                } else {
                                    break;
                                }
                            }
                            if let Some(dt) = latest_html_dt {
                                publish_date = dt.format("%d.%m.%Y").to_string();
                            }
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    if asset_url.is_empty() {
        return Err("Could not resolve UE4SS download URL".to_string());
    }

    if publish_date.is_empty() {
        publish_date = chrono::Utc::now().format("%d.%m.%Y").to_string();
    }

    crate::logger::log(&format!("install_ue4ss: Downloading ZIP from {}", asset_url));
    let bytes = client.get(&asset_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    
    let zip_bytes = bytes.to_vec();

    // Save into versioned vault
    let _ = save_to_vault(&program_path, "ue4ss", &publish_date, &zip_bytes);

    apply_ue4ss_zip_bytes(&zip_bytes, &publish_date, &program_path, &game_path, &state).await
}

#[tauri::command]
pub async fn install_palschema(force_download: bool, state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() {
        return Err("Game path not set".to_string());
    }

    let dep_status = crate::dependency_checker::check_dependencies(&game_path);
    if dep_status.palschema_installed && !force_download {
        crate::logger::log("install_palschema: PalSchema is already installed and force_download is false. Skipping installation.");
        return Ok("PalSchema is already installed.".to_string());
    }

    if !dep_status.ue4ss_installed {
        return Err("UE4SS is not installed. PalSchema requires UE4SS to operate.".to_string());
    }

    migrate_legacy_dependency_zips(&program_path);

    let tag = match dependency_checker::check_palschema_latest().await {
        Ok(t) => t,
        Err(e) => return Err(format!("Could not determine latest PalSchema tag: {}", e)),
    };

    let client = reqwest::Client::builder()
        .user_agent("PalModManager/1.7.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut asset_url = String::new();
    let mut api_success = false;

    let api_url = format!("https://api.github.com/repos/Okaetsu/PalSchema/releases/tags/{}", tag);
    crate::logger::log(&format!("install_palschema: Fetching GitHub API release from {}", api_url));
    if let Ok(resp) = client.get(&api_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(assets) = json["assets"].as_array() {
                    if let Some(asset) = assets.iter().find(|a| {
                        a["name"].as_str().map_or(false, |n| n.ends_with(".zip"))
                    }) {
                        if let Some(url) = asset["browser_download_url"].as_str() {
                            asset_url = url.to_string();
                            api_success = true;
                        }
                    }
                }
            }
        }
    }

    if !api_success {
        crate::logger::log("install_palschema: GitHub API rate limited or failed. Using HTML fallback...");
        let release_page_url = format!("https://github.com/Okaetsu/PalSchema/releases/tag/{}", tag);
        if let Ok(resp) = client.get(&release_page_url).send().await {
            if let Ok(html) = resp.text().await {
                let download_prefix = format!("/Okaetsu/PalSchema/releases/download/{}/", tag);
                let mut search_pos = 0;
                while let Some(pos) = html[search_pos..].find(&download_prefix) {
                    let start = search_pos + pos;
                    if let Some(end_quote) = html[start..].find('"') {
                        let url_path = &html[start..start + end_quote];
                        search_pos = start + end_quote;
                        if url_path.to_lowercase().ends_with(".zip") {
                            asset_url = format!("https://github.com{}", url_path);
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    if asset_url.is_empty() {
        asset_url = format!("https://github.com/Okaetsu/PalSchema/releases/download/{}/PalSchema_{}.zip", tag, tag.trim_start_matches('v'));
    }

    crate::logger::log(&format!("install_palschema: Descargando desde {}", asset_url));
    let resp = client.get(&asset_url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed. GitHub returned HTTP {}", resp.status()));
    }

    let bytes = resp.bytes()
        .await
        .map_err(|e| format!("Failed to read download bytes: {}", e))?;
    
    let zip_bytes = bytes.to_vec();

    // Save into versioned vault
    let _ = save_to_vault(&program_path, "palschema", &tag, &zip_bytes);

    apply_palschema_zip_bytes(&zip_bytes, &tag, &program_path, &game_path, &state).await
}

#[tauri::command]
pub fn get_dependency_vault(dep_type: String, state: State<'_, AppState>) -> Result<Vec<crate::models::DependencyVaultEntry>, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if program_path.is_empty() {
        return Ok(Vec::new());
    }

    migrate_legacy_dependency_zips(&program_path);

    let vault_dir = get_vault_dir(&program_path, &dep_type);
    let _ = fs::create_dir_all(&vault_dir);

    // Get current installed version
    let dep_status = crate::dependency_checker::check_dependencies(&game_path);
    let installed_ver = if dep_type.to_lowercase() == "palschema" {
        dep_status.palschema_version.unwrap_or_default()
    } else {
        dep_status.ue4ss_version.unwrap_or_default()
    };

    let mut entries = Vec::new();
    if let Ok(rd) = fs::read_dir(&vault_dir) {
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("zip")) {
                let filename = entry.file_name().to_string_lossy().to_string();
                let metadata = entry.metadata().ok();
                let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified_time = metadata.and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                    .unwrap_or_default();

                let version = extract_version_from_vault_filename(&filename, &dep_type);
                let canonical_name = match dep_type.to_lowercase().as_str() {
                    "palschema" => format!("PalSchema - {}.zip", version),
                    _ => format!("UE4SS - {}.zip", version),
                };

                let (final_path, final_filename) = if filename != canonical_name {
                    let dest = vault_dir.join(&canonical_name);
                    if !dest.exists() {
                        let _ = fs::rename(&path, &dest);
                        (dest, canonical_name)
                    } else {
                        (path, filename)
                    }
                } else {
                    (path, filename)
                };

                let clean_installed = sanitize_version_tag(&installed_ver, &dep_type);
                let is_installed = !clean_installed.is_empty() && (
                    version.trim().eq_ignore_ascii_case(&clean_installed) ||
                    version.trim_start_matches('v').eq_ignore_ascii_case(clean_installed.trim_start_matches('v'))
                );
                let is_custom = !final_filename.starts_with("UE4SS - ") && !final_filename.starts_with("PalSchema - ");

                entries.push(crate::models::DependencyVaultEntry {
                    dep_type: dep_type.clone(),
                    version,
                    filename: final_filename,
                    file_path: final_path.to_string_lossy().to_string(),
                    file_size,
                    modified_time,
                    is_installed,
                    is_custom,
                });
            }
        }
    }

    entries.sort_by(|a, b| b.modified_time.cmp(&a.modified_time));
    Ok(entries)
}

#[tauri::command]
pub async fn install_dependency_from_vault(dep_type: String, filename: String, state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() || program_path.is_empty() {
        return Err("Game path or program path not configured".to_string());
    }
    let vault_dir = get_vault_dir(&program_path, &dep_type);
    let zip_file = vault_dir.join(&filename);
    if !zip_file.exists() {
        return Err(format!("Vault archive not found: {}", filename));
    }
    let zip_bytes = fs::read(&zip_file).map_err(|e| format!("Failed to read archive: {}", e))?;
    let version = extract_version_from_vault_filename(&filename, &dep_type);

    if dep_type.to_lowercase() == "palschema" {
        apply_palschema_zip_bytes(&zip_bytes, &version, &program_path, &game_path, &state).await
    } else {
        apply_ue4ss_zip_bytes(&zip_bytes, &version, &program_path, &game_path, &state).await
    }
}

#[tauri::command]
pub async fn install_dependency_from_custom_zip(dep_type: String, zip_path: String, custom_version: Option<String>, state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() || program_path.is_empty() {
        return Err("Game path or program path not configured".to_string());
    }
    let src_path = PathBuf::from(&zip_path);
    if !src_path.exists() {
        return Err("Selected ZIP file does not exist".to_string());
    }
    let zip_bytes = fs::read(&src_path).map_err(|e| format!("Failed to read ZIP: {}", e))?;
    let raw_ver = custom_version.unwrap_or_else(|| {
        src_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "custom".to_string())
    });
    let clean_ver = sanitize_version_tag(&raw_ver, &dep_type);

    let _ = save_to_vault(&program_path, &dep_type, &clean_ver, &zip_bytes);

    if dep_type.to_lowercase() == "palschema" {
        apply_palschema_zip_bytes(&zip_bytes, &clean_ver, &program_path, &game_path, &state).await
    } else {
        apply_ue4ss_zip_bytes(&zip_bytes, &clean_ver, &program_path, &game_path, &state).await
    }
}

#[tauri::command]
pub fn delete_dependency_vault_entry(dep_type: String, filename: String, state: State<'_, AppState>) -> Result<(), String> {
    let program_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.program_path.clone()
    };
    let vault_dir = get_vault_dir(&program_path, &dep_type);
    let target = vault_dir.join(&filename);
    if target.exists() {
        fs::remove_file(&target).map_err(|e| format!("Failed to delete archive: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_dependency_vault_folder(dep_type: String, state: State<'_, AppState>) -> Result<(), String> {
    let program_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.program_path.clone()
    };
    let vault_dir = get_vault_dir(&program_path, &dep_type);
    let _ = fs::create_dir_all(&vault_dir);
    open::that(&vault_dir).map_err(|e| format!("Failed to open directory: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn uninstall_ue4ss(state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() { return Err("Game path not set".to_string()); }

    // Guard: Workshop installations cannot be uninstalled from PMM
    let game_profile = crate::dependency_checker::build_game_profile(Path::new(&game_path));
    if game_profile.ue4ss_install_mode == crate::dependency_checker::UE4SSInstallMode::Workshop {
        return Err("UE4SS is managed by Steam Workshop. To uninstall, unsubscribe from the mod in Steam.".to_string());
    }

    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let dwmapi = win64.join("dwmapi.dll");
    let ue4ss_dir = win64.join("ue4ss");
    
    if dwmapi.exists() { let _ = fs::remove_file(dwmapi); }
    if ue4ss_dir.exists() { let _ = fs::remove_dir_all(ue4ss_dir); }
    
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.retain(|m| m.mod_type != crate::models::ModType::Ue4ss && m.mod_type != crate::models::ModType::PalSchema);

        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.ue4ss_enabled = false;
            profile.palschema_enabled = false;
            // Also clean non-native UE4SS/PalSchema mods from profile lists
            profile.installed_mod_ids.clear();
            profile.enabled_mod_ids.clear();

            let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    
    crate::logger::log("uninstall_ue4ss: UE4SS desinstalado con éxito.");
    Ok("UE4SS uninstalled successfully".to_string())
}

#[tauri::command]
pub fn uninstall_palschema(state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() { return Err("Game path not set".to_string()); }

    // Guard: Workshop installations cannot be uninstalled from PMM
    let game_profile = crate::dependency_checker::build_game_profile(Path::new(&game_path));
    if game_profile.ue4ss_install_mode == crate::dependency_checker::UE4SSInstallMode::Workshop {
        return Err("PalSchema is managed by Steam Workshop. To uninstall, unsubscribe from the mod in Steam.".to_string());
    }

    // Use the profile's resolved palschema path (handles both Standard and edge cases)
    let palschema_dir = game_profile.ue4ss_mods_dir.join("PalSchema");
        
    if palschema_dir.exists() { let _ = fs::remove_dir_all(palschema_dir); }
    
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.retain(|m| m.mod_type != crate::models::ModType::PalSchema);

        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.palschema_enabled = false;
            
            let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }


    
    crate::logger::log("uninstall_palschema: PalSchema desinstalado con éxito.");
    Ok("PalSchema uninstalled successfully".to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsageInfo {
    pub temp_downloads_size: u64,
    pub temp_downloads_count: usize,
    pub temp_downloads_path: String,
    pub library_size: u64,
    pub library_mods_count: usize,
    pub library_zips_count: usize,
    pub library_path: String,
}

#[tauri::command]
pub fn get_storage_usage_command(state: State<'_, AppState>) -> Result<StorageUsageInfo, String> {
    let program_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.program_path.clone()
    };

    // 1. Temp downloads
    let temp_dir = std::env::temp_dir().join("PalModManager_Downloads");
    let mut temp_size = 0u64;
    let mut temp_count = 0usize;
    if temp_dir.exists() {
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        temp_size += meta.len();
                        temp_count += 1;
                    }
                }
            }
        }
    }

    // 2. Local Library
    let lib_dir = crate::library::library_dir(&program_path);
    let mut lib_size = 0u64;
    let mut lib_mods_count = 0usize;
    let mut lib_zips_count = 0usize;
    if lib_dir.exists() {
        if let Ok(mod_entries) = fs::read_dir(&lib_dir) {
            for mod_entry in mod_entries.flatten() {
                let p = mod_entry.path();
                if p.is_dir() {
                    lib_mods_count += 1;
                    if let Ok(zip_entries) = fs::read_dir(&p) {
                        for zip_entry in zip_entries.flatten() {
                            let zp = zip_entry.path();
                            if let Ok(meta) = zp.metadata() {
                                if meta.is_file() {
                                    lib_size += meta.len();
                                    if zp.extension().and_then(|e| e.to_str()).map(|ext| ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("7z") || ext.eq_ignore_ascii_case("rar")).unwrap_or(false) {
                                        lib_zips_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(StorageUsageInfo {
        temp_downloads_size: temp_size,
        temp_downloads_count: temp_count,
        temp_downloads_path: temp_dir.to_string_lossy().to_string(),
        library_size: lib_size,
        library_mods_count: lib_mods_count,
        library_zips_count: lib_zips_count,
        library_path: lib_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn clear_temp_downloads_command() -> Result<u64, String> {
    let temp_dir = std::env::temp_dir().join("PalModManager_Downloads");
    let mut freed_bytes = 0u64;
    if temp_dir.exists() {
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(meta) = p.metadata() {
                    if meta.is_file() {
                        freed_bytes += meta.len();
                        let _ = fs::remove_file(&p);
                    } else if meta.is_dir() {
                        let _ = fs::remove_dir_all(&p);
                    }
                }
            }
        }
    }
    crate::logger::log(&format!("clear_temp_downloads: Freed {} bytes", freed_bytes));
    Ok(freed_bytes)
}

#[tauri::command]
pub fn open_temp_folder_command() -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join("PalModManager_Downloads");
    let _ = fs::create_dir_all(&temp_dir);
    crate::commands::mod_commands::open_path(temp_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_library_folder_command(state: State<'_, AppState>) -> Result<(), String> {
    let program_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.program_path.clone()
    };
    let lib_dir = crate::library::library_dir(&program_path);
    let _ = fs::create_dir_all(&lib_dir);
    crate::commands::mod_commands::open_path(lib_dir.to_string_lossy().to_string())
}

