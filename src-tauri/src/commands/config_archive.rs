use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::models::ModInfo;
use crate::state::AppState;
use crate::profiles::utils::get_profile_dir;
use crate::commands::install::diff::ConfigDiff;

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

pub fn sanitize_archive_key(name: &str) -> String {
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
        let ue4ss_roots = [
            binaries_dir.join("ue4ss").join("Mods"),
            binaries_dir.join("Mods"),
        ];
        for u_root in &ue4ss_roots {
            let u_mod = u_root.join(&folder_name);
            if u_mod.is_dir() && !candidate_dirs.contains(&u_mod) {
                candidate_dirs.push(u_mod);
            }
            let shared = u_root.join("shared");
            let s_folder = shared.join(&folder_name);
            if s_folder.is_dir() && !candidate_dirs.contains(&s_folder) {
                candidate_dirs.push(s_folder);
            }
            let s_name = shared.join(&mod_info.name);
            if s_name.is_dir() && !candidate_dirs.contains(&s_name) {
                candidate_dirs.push(s_name);
            }
        }
    }

    let mut all_entries: Vec<(PathBuf, String)> = Vec::new();

    for dir in &candidate_dirs {
        let snap = crate::config_merge::snapshot_configs(dir, mod_info.config_path.as_deref());
        for entry in snap.entries {
            if let Some(pos) = all_entries.iter().position(|(p, _)| p == &entry.0 || p.file_name() == entry.0.file_name()) {
                if !all_entries[pos].0.to_string_lossy().contains("shared") && entry.0.to_string_lossy().contains("shared") {
                    all_entries[pos] = entry;
                }
            } else {
                all_entries.push(entry);
            }
        }
    }

    // Also check custom config paths if pointing to loose files
    if let Some(ref c_paths) = mod_info.config_paths {
        for custom_cfg in c_paths {
            let p = Path::new(custom_cfg);
            if p.is_file() && p.exists() {
                if let Ok(content) = fs::read_to_string(p) {
                    let fname = PathBuf::from(p.file_name().unwrap_or_default());
                    if !all_entries.iter().any(|(p_entry, _)| p_entry == &fname || p_entry.file_name() == fname.file_name()) {
                        all_entries.push((fname, content));
                    }
                }
            }
        }
    } else if let Some(ref custom_cfg) = mod_info.config_path {
        let p = Path::new(custom_cfg);
        if p.is_file() && p.exists() {
            if let Ok(content) = fs::read_to_string(p) {
                let fname = PathBuf::from(p.file_name().unwrap_or_default());
                if !all_entries.iter().any(|(p_entry, _)| p_entry == &fname || p_entry.file_name() == fname.file_name()) {
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
pub async fn preview_archived_config_diff(
    zip_path: String,
    archive_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ConfigDiff>, String> {
    let (program_path, current_profile_id) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.current_profile_id.clone())
    };

    let archive_dir = get_archive_dir(&program_path, &current_profile_id, &archive_id);
    preview_archived_config_diff_internal(&zip_path, &archive_dir)
}

pub fn preview_archived_config_diff_internal(
    zip_path: &str,
    archive_dir: &Path,
) -> Result<Vec<ConfigDiff>, String> {
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

    if file_list.is_empty() {
        return Ok(Vec::new());
    }

    let temp_dir = std::env::temp_dir().join(format!("pmm_arch_diff_{}", uuid::Uuid::new_v4()));
    let extracted = crate::zip_handler::extract_zip_to_temp(zip_path, &temp_dir)?;

    let mut diffs = Vec::new();

    fn find_matching_file_rec(dir: &Path, target_name: &std::ffi::OsStr) -> Option<PathBuf> {
        if let Ok(rd) = fs::read_dir(dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if let Some(found) = find_matching_file_rec(&p, target_name) {
                        return Some(found);
                    }
                } else if p.is_file() && p.file_name() == Some(target_name) {
                    return Some(p);
                }
            }
        }
        None
    }

    for file_rel in file_list {
        let archived_file = archive_dir.join(&file_rel);
        if !archived_file.exists() || !archived_file.is_file() {
            continue;
        }

        let archived_content = match fs::read_to_string(&archived_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let fname = Path::new(&file_rel).file_name().unwrap_or_default();
        let direct_incoming = extracted.join(&file_rel);
        let incoming_file = if direct_incoming.exists() && direct_incoming.is_file() {
            Some(direct_incoming)
        } else {
            find_matching_file_rec(&extracted, fname)
        };

        let ext = Path::new(&file_rel).extension().and_then(|s| s.to_str()).unwrap_or("");

        if let Some(ref inc_path) = incoming_file {
            if let Ok(incoming_content) = fs::read_to_string(inc_path) {
                if let Some((user_changed, added, removed)) = crate::config_merge::generate_config_diff(&archived_content, &incoming_content, ext) {
                    diffs.push(ConfigDiff {
                        file_name: file_rel.clone(),
                        keys_user_changed: user_changed,
                        keys_added_by_author: added,
                        keys_removed_by_author: removed,
                    });
                    continue;
                }
            }
        }

        // If file is only in archive or format is non-diffable, present it with empty key diffs so user can toggle whole file
        diffs.push(ConfigDiff {
            file_name: file_rel.clone(),
            keys_user_changed: Vec::new(),
            keys_added_by_author: Vec::new(),
            keys_removed_by_author: Vec::new(),
        });
    }

    let _ = fs::remove_dir_all(&temp_dir);

    Ok(diffs)
}

#[tauri::command]
pub fn apply_archived_config(
    mod_id: String,
    archive_id: String,
    ignored_files: Option<Vec<String>>,
    ignored_keys: Option<Vec<String>>,
    state: State<AppState>,
) -> Result<bool, String> {
    let (program_path, current_profile_id, mod_info) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let m = data.mods.iter().find(|m| m.id == mod_id || m.name.eq_ignore_ascii_case(&mod_id)).cloned().ok_or_else(|| format!("Mod '{}' not found", mod_id))?;
        (data.settings.program_path.clone(), data.current_profile_id.clone(), m)
    };

    let archive_dir = get_archive_dir(&program_path, &current_profile_id, &archive_id);

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

    apply_archived_config_internal(
        &archive_dir,
        &target_dir,
        &ignored_files.unwrap_or_default(),
        &ignored_keys.unwrap_or_default(),
    )
}

pub fn apply_archived_config_internal(
    archive_dir: &Path,
    target_dir: &Path,
    ignored_files: &[String],
    ignored_keys: &[String],
) -> Result<bool, String> {
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

    // Read all archived files into a ConfigSnapshot (skipping user-ignored files)
    let mut snapshot_entries: Vec<(PathBuf, String)> = Vec::new();
    for file_rel in file_list {
        let fname_str = Path::new(&file_rel).file_name().unwrap_or_default().to_string_lossy().to_string();
        if ignored_files.iter().any(|ig| ig.eq_ignore_ascii_case(&file_rel) || ig.eq_ignore_ascii_case(&fname_str)) {
            crate::logger::log(&format!("apply_archived_config: User ignored file '{}', skipping restore.", file_rel));
            continue;
        }

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

    if target_dir.exists() && target_dir.is_dir() {
        crate::config_merge::apply_config_merge(target_dir, &snapshot, ignored_keys);
        crate::logger::log(&format!(
            "apply_archived_config: Successfully restored {} archived configs into {:?}",
            snapshot.entries.len(),
            target_dir
        ));
        Ok(true)
    } else {
        crate::logger::log(&format!(
            "apply_archived_config: Target directory not found: {:?}",
            target_dir
        ));
        Ok(false)
    }
}

