use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use uuid::Uuid;
use crate::state::AppState;
use crate::zip_handler;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigDiff {
    pub file_name: String,
    pub keys_user_changed: Vec<crate::config_merge::ChangedKeyDetail>,
    pub keys_added_by_author: Vec<String>,
    pub keys_removed_by_author: Vec<String>,
}

#[tauri::command]
pub async fn preview_config_diff(
    zip_path: String,
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ConfigDiff>, String> {
    let (game_path, _program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.game_path.clone(), data.settings.program_path.clone())
    };

    if game_path.is_empty() {
        return Err("Game path is not configured".to_string());
    }

    let (mod_name, mod_config_path, installed_roots) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let m = data.mods.iter().find(|m| m.id == mod_id)
            .ok_or_else(|| "Mod not found".to_string())?;
        
        let game = Path::new(&game_path);
        let mut roots = Vec::new();
        if !m.game_path.is_empty() {
            let p = crate::config_merge::resolve_path_in_game(game, &m.game_path);
            if p.exists() {
                let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
                roots.push(r);
            }
        }
        if !m.disabled_path.is_empty() {
            let p = crate::config_merge::resolve_path_in_game(game, &m.disabled_path);
            if p.exists() {
                let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
                if !roots.contains(&r) {
                    roots.push(r);
                }
            }
        }
        for extra in &m.extra_files {
            let p = crate::config_merge::resolve_path_in_game(game, extra);
            if p.exists() {
                let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
                if !roots.contains(&r) {
                    roots.push(r);
                }
            }
        }
        (m.name.clone(), m.config_path.clone(), roots)
    };

    let temp_dir = std::env::temp_dir().join(format!("palmodmanager_diff_{}", Uuid::new_v4()));
    let extracted = zip_handler::extract_zip_to_temp(&zip_path, &temp_dir)?;

    let zip_filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            let full_path = extracted.join(target_file);
            if full_path.exists() {
                if let Ok(content) = std::fs::read_to_string(full_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        modinfo_data = Some(val);
                    }
                }
            }
        }
    }

    let game = Path::new(&game_path);
    let manifest = zip_handler::build_manifest_from_files(
        &analysis.files,
        &zip_filename,
        game,
        None,
        Some(mod_name),
        modinfo_data,
    )?;

    let incoming_mod_dir = if !manifest.folder_name.is_empty() {
        fn find_folder(current: &Path, folder_name: &str) -> Option<PathBuf> {
            if current.is_dir() {
                if let Some(name) = current.file_name().and_then(|n| n.to_str()) {
                    if name.to_lowercase() == folder_name.to_lowercase() {
                        return Some(current.to_path_buf());
                    }
                }
                if let Ok(entries) = fs::read_dir(current) {
                    for entry in entries.flatten() {
                        if let Some(found) = find_folder(&entry.path(), folder_name) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
        find_folder(&extracted, &manifest.folder_name).unwrap_or(extracted.clone())
    } else {
        extracted.clone()
    };

    let mut incoming_snapshot = crate::config_merge::snapshot_configs(&incoming_mod_dir, mod_config_path.as_deref());
    // If incoming_mod_dir didn't catch everything (e.g. extracted has full subpaths), also scan extracted root
    if incoming_mod_dir != extracted {
        let root_snapshot = crate::config_merge::snapshot_configs(&extracted, mod_config_path.as_deref());
        for entry in root_snapshot.entries {
            if !incoming_snapshot.entries.iter().any(|(r, _)| r.file_name() == entry.0.file_name()) {
                incoming_snapshot.entries.push(entry);
            }
        }
    }

    let mut diffs = Vec::new();

    for (rel_path, new_content) in incoming_snapshot.entries {
        let mut old_content_opt = None;

        for root in &installed_roots {
            let direct = root.join(&rel_path);
            if direct.exists() && direct.is_file() {
                if let Ok(c) = fs::read_to_string(&direct) {
                    old_content_opt = Some(c);
                    break;
                }
            }
            let candidate_scripts = root.join("Scripts").join(&rel_path);
            if candidate_scripts.exists() && candidate_scripts.is_file() {
                if let Ok(c) = fs::read_to_string(&candidate_scripts) {
                    old_content_opt = Some(c);
                    break;
                }
            }
            if let Some(fname) = rel_path.file_name() {
                let candidate = root.join("Scripts").join(fname);
                if candidate.exists() && candidate.is_file() {
                    if let Ok(c) = fs::read_to_string(&candidate) {
                        old_content_opt = Some(c);
                        break;
                    }
                }
                let candidate2 = root.join(fname);
                if candidate2.exists() && candidate2.is_file() {
                    if let Ok(c) = fs::read_to_string(&candidate2) {
                        old_content_opt = Some(c);
                        break;
                    }
                }
            }
        }

        if old_content_opt.is_none() {
            if let Some(ref custom_str) = mod_config_path {
                let cp = crate::config_merge::resolve_path_in_game(game, custom_str);
                if cp.exists() && cp.is_file() {
                    if let Ok(c) = fs::read_to_string(&cp) {
                        old_content_opt = Some(c);
                    }
                }
            }
        }

        if let Some(old_content) = old_content_opt {
            let ext = rel_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if let Some((user_changed, added, removed)) = crate::config_merge::generate_config_diff(&old_content, &new_content, ext) {
                if !user_changed.is_empty() || !added.is_empty() || !removed.is_empty() {
                    diffs.push(ConfigDiff {
                        file_name: rel_path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| rel_path.to_string_lossy().to_string()),
                        keys_user_changed: user_changed,
                        keys_added_by_author: added,
                        keys_removed_by_author: removed,
                    });
                }
            }
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(diffs)
}
