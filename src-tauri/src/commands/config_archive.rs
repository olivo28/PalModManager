use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::models::ModInfo;
use crate::state::AppState;
use crate::profiles::utils::get_profile_dir;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedConfigInfo {
    pub archive_id: String,
    pub mod_name: String,
    pub mod_id: String,
    pub nexus_mod_id: Option<u32>,
    pub archived_at: String,
    pub files: Vec<String>,
}

fn sanitize_archive_key(name: &str) -> String {
    let mut clean = String::new();
    let mut last_was_underscore = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            clean.push(c.to_ascii_lowercase());
            last_was_underscore = false;
        } else if !last_was_underscore {
            clean.push('_');
            last_was_underscore = true;
        }
    }
    let trimmed = clean.trim_matches('_');
    if trimmed.is_empty() {
        "mod".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn get_archive_dir(program_path: &str, profile_id: &str, archive_key: &str) -> PathBuf {
    get_profile_dir(program_path, profile_id)
        .join("archived_configs")
        .join(archive_key)
}

/// Archive configuration files for a mod before it is removed or purged.
pub fn archive_mod_configs(
    mod_info: &ModInfo,
    program_path: &str,
    profile_id: &str,
    game_path: &str,
) -> Result<usize, String> {
    if program_path.is_empty() || profile_id.is_empty() {
        return Ok(0);
    }

    let archive_key = if let Some(nexus_id) = mod_info.nexus_mod_id {
        format!("nexus_{}", nexus_id)
    } else {
        format!("mod_{}", sanitize_archive_key(&mod_info.name))
    };

    let target_dir = get_archive_dir(program_path, profile_id, &archive_key);

    // Determine candidate directories to snapshot
    let mut candidate_dirs: Vec<PathBuf> = Vec::new();

    if !mod_info.game_path.is_empty() {
        let gp = PathBuf::from(&mod_info.game_path);
        if gp.is_dir() && gp.exists() {
            candidate_dirs.push(gp);
        }
    }
    if !mod_info.disabled_path.is_empty() {
        let dp = PathBuf::from(&mod_info.disabled_path);
        if dp.is_dir() && dp.exists() {
            candidate_dirs.push(dp);
        }
    }

    if !game_path.is_empty() {
        let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
        let folder_name = crate::profiles::get_mod_folder_name(mod_info);
        let ue4ss_mod_folder = binaries_dir.join("Mods").join(&folder_name);
        if ue4ss_mod_folder.is_dir() && ue4ss_mod_folder.exists() && !candidate_dirs.contains(&ue4ss_mod_folder) {
            candidate_dirs.push(ue4ss_mod_folder);
        }
    }

    let mut all_entries: Vec<(PathBuf, String)> = Vec::new();

    for dir in &candidate_dirs {
        let snap = crate::config_merge::snapshot_configs(dir, mod_info.config_path.as_deref());
        for entry in snap.entries {
            if !all_entries.iter().any(|(p, _)| p == &entry.0) {
                all_entries.push(entry);
            }
        }
    }

    // Also check single custom config file if pointing to a loose file
    if let Some(ref custom_cfg) = mod_info.config_path {
        let p = Path::new(custom_cfg);
        if p.is_file() && p.exists() {
            if let Ok(content) = fs::read_to_string(p) {
                let fname = PathBuf::from(p.file_name().unwrap_or_default());
                if !all_entries.iter().any(|(p, _)| p == &fname) {
                    all_entries.push((fname, content));
                }
            }
        }
    }

    if all_entries.is_empty() {
        return Ok(0);
    }

    let _ = fs::create_dir_all(&target_dir);

    let mut saved_file_names: Vec<String> = Vec::new();

    for (rel_path, content) in &all_entries {
        let dest_file = target_dir.join(rel_path);
        if let Some(parent) = dest_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(&dest_file, content).is_ok() {
            saved_file_names.push(rel_path.to_string_lossy().to_string());
        }
    }

    let meta = ArchivedConfigInfo {
        archive_id: archive_key,
        mod_name: mod_info.name.clone(),
        mod_id: mod_info.id.clone(),
        nexus_mod_id: mod_info.nexus_mod_id,
        archived_at: Utc::now().to_rfc3339(),
        files: saved_file_names.clone(),
    };

    let meta_file = target_dir.join("archive_meta.json");
    let _ = fs::write(&meta_file, serde_json::to_string_pretty(&meta).unwrap_or_default());

    crate::logger::log(&format!(
        "archive_mod_configs: Archived {} configuration files for mod '{}' in {:?}",
        saved_file_names.len(),
        mod_info.name,
        target_dir
    ));

    Ok(saved_file_names.len())
}

#[tauri::command]
pub fn check_archived_config(
    nexus_mod_id: Option<u32>,
    mod_name: String,
    state: State<AppState>,
) -> Result<Option<ArchivedConfigInfo>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let current_profile_id = data.current_profile_id.clone();
    drop(data);

    let archives_root = get_profile_dir(&program_path, &current_profile_id).join("archived_configs");
    if !archives_root.exists() {
        return Ok(None);
    }

    // Try finding by nexus_mod_id first
    if let Some(nid) = nexus_mod_id {
        let nexus_dir = archives_root.join(format!("nexus_{}", nid));
        if nexus_dir.exists() {
            let meta_path = nexus_dir.join("archive_meta.json");
            if meta_path.exists() {
                if let Ok(content) = fs::read_to_string(&meta_path) {
                    if let Ok(meta) = serde_json::from_str::<ArchivedConfigInfo>(&content) {
                        return Ok(Some(meta));
                    }
                }
            }
        }
    }

    // Fallback: match by sanitized mod name
    let clean_name = sanitize_archive_key(&mod_name);
    let name_dir = archives_root.join(format!("mod_{}", clean_name));
    if name_dir.exists() {
        let meta_path = name_dir.join("archive_meta.json");
        if meta_path.exists() {
            if let Ok(content) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<ArchivedConfigInfo>(&content) {
                    return Ok(Some(meta));
                }
            }
        }
    }

    // Scan all archive meta files in case name matches case-insensitively
    if let Ok(entries) = fs::read_dir(&archives_root) {
        for entry in entries.flatten() {
            let meta_path = entry.path().join("archive_meta.json");
            if meta_path.exists() {
                if let Ok(content) = fs::read_to_string(&meta_path) {
                    if let Ok(meta) = serde_json::from_str::<ArchivedConfigInfo>(&content) {
                        if meta.mod_name.eq_ignore_ascii_case(&mod_name)
                            || (nexus_mod_id.is_some() && meta.nexus_mod_id == nexus_mod_id)
                        {
                            return Ok(Some(meta));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

#[tauri::command]
pub fn apply_archived_config(
    mod_id: String,
    archive_id: String,
    state: State<AppState>,
) -> Result<bool, String> {
    let (program_path, current_profile_id, mod_info) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let m = data.mods.iter().find(|m| m.id == mod_id || m.name.eq_ignore_ascii_case(&mod_id)).cloned().ok_or_else(|| format!("Mod '{}' not found", mod_id))?;
        (data.settings.program_path.clone(), data.current_profile_id.clone(), m)
    };

    let archive_dir = get_archive_dir(&program_path, &current_profile_id, &archive_id);
    if !archive_dir.exists() {
        return Err(format!("Archived config directory not found: {:?}", archive_dir));
    }

    let meta_path = archive_dir.join("archive_meta.json");
    let file_list = if meta_path.exists() {
        if let Ok(content) = fs::read_to_string(&meta_path) {
            serde_json::from_str::<ArchivedConfigInfo>(&content).map(|m| m.files).unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Read all archived files into a ConfigSnapshot
    let mut snapshot_entries: Vec<(PathBuf, String)> = Vec::new();
    for file_rel in file_list {
        let src_file = archive_dir.join(&file_rel);
        if src_file.exists() && src_file.is_file() {
            if let Ok(content) = fs::read_to_string(&src_file) {
                snapshot_entries.push((PathBuf::from(file_rel), content));
            }
        }
    }

    if snapshot_entries.is_empty() {
        return Ok(false);
    }

    let snapshot = crate::config_merge::ConfigSnapshot { entries: snapshot_entries };

    // Determine target mod directory to apply merge into
    let target_dir = if !mod_info.game_path.is_empty() && Path::new(&mod_info.game_path).is_dir() {
        PathBuf::from(&mod_info.game_path)
    } else if !mod_info.disabled_path.is_empty() && Path::new(&mod_info.disabled_path).is_dir() {
        PathBuf::from(&mod_info.disabled_path)
    } else if let Some(ref cfg) = mod_info.config_path {
        Path::new(cfg).parent().unwrap_or_else(|| Path::new("")).to_path_buf()
    } else {
        PathBuf::new()
    };

    if target_dir.exists() && target_dir.is_dir() {
        crate::config_merge::apply_config_merge(&target_dir, &snapshot, &[]);
        crate::logger::log(&format!(
            "apply_archived_config: Successfully restored {} archived configs into {:?}",
            snapshot.entries.len(),
            target_dir
        ));
        Ok(true)
    } else {
        crate::logger::log(&format!(
            "apply_archived_config: Target directory not found for mod '{}'",
            mod_info.name
        ));
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_archive_key() {
        assert_eq!(sanitize_archive_key("PalVariety 4x (Shiny)!"), "palvariety_4x_shiny");
        assert_eq!(sanitize_archive_key("---"), "mod");
        assert_eq!(sanitize_archive_key("SimpleMod"), "simplemod");
    }

    #[test]
    fn test_get_archive_dir() {
        let dir = get_archive_dir("C:/pmm", "default", "nexus_1234");
        assert!(dir.to_string_lossy().contains("archived_configs"));
        assert!(dir.to_string_lossy().ends_with("nexus_1234"));
    }
}
