use std::fs;
use std::path::PathBuf;
use tauri::State;
use crate::state::AppState;
use super::install::{apply_palschema_zip_bytes, apply_ue4ss_zip_bytes};

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

pub fn migrate_legacy_dependency_zips(program_path: &str) {
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
