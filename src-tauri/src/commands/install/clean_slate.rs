use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::commands::install::diff::ConfigDiff;
use crate::models::ModInfo;

/// Snapshots text-based configuration and setting files for a mod before update.
/// Returns a map of normalized relative path -> (exact relative path, file content).
pub fn snapshot_mod_text_files(
    game_path: &Path,
    mod_info: &ModInfo,
) -> HashMap<String, (String, String)> {
    let mut snapshot = HashMap::new();
    let roots = collect_mod_disk_roots(game_path, mod_info);

    for (root_dir, is_standalone_file) in roots {
        if is_standalone_file {
            if root_dir.exists() && root_dir.is_file() {
                if is_candidate_text_file(&root_dir) {
                    let file_name = root_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if let Ok(content) = fs::read_to_string(&root_dir) {
                        snapshot.insert(file_name.to_lowercase(), (file_name, content));
                    }
                }
            }
        } else if root_dir.exists() && root_dir.is_dir() {
            for entry in walkdir::WalkDir::new(&root_dir).into_iter().filter_map(|e| e.ok()) {
                let full_path = entry.path();
                if full_path.is_file() && is_candidate_text_file(full_path) {
                    let rel_path = full_path
                        .strip_prefix(&root_dir)
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| full_path.file_name().unwrap_or_default().to_string_lossy().to_string());
                    
                    if let Ok(content) = fs::read_to_string(full_path) {
                        snapshot.insert(rel_path.to_lowercase(), (rel_path, content));
                    }
                }
            }
        }
    }

    crate::logger::log(&format!(
        "[clean_slate] Snapshotted {} text/config files for mod '{}'",
        snapshot.len(),
        mod_info.name
    ));

    snapshot
}

/// Physically purges existing mod files, directories, and junctions from disk to provide a clean slate.
pub fn purge_existing_mod_files(game_path: &Path, mod_info: &ModInfo) {
    crate::logger::log(&format!(
        "[clean_slate] Performing clean-slate purge for mod '{}'",
        mod_info.name
    ));

    let delete_path_and_sidecar = |path_str: &str| {
        if path_str.is_empty() {
            return;
        }
        let p = crate::config_merge::resolve_path_in_game(game_path, path_str);
        if crate::profiles::is_junction_or_symlink(&p) {
            let _ = crate::profiles::remove_junction_or_symlink(&p);
        } else if p.exists() {
            // Guard: Never delete Steam Workshop download folders
            let is_steam_workshop = p.to_string_lossy().replace('\\', "/").to_lowercase().contains("steamapps/workshop/content");
            if !is_steam_workshop {
                if p.is_dir() {
                    let _ = fs::remove_dir_all(&p);
                } else {
                    let _ = fs::remove_file(&p);
                    for comp_ext in &["ucas", "utoc", "sig"] {
                        let comp_file = p.with_extension(comp_ext);
                        if comp_file.exists() {
                            let _ = fs::remove_file(comp_file);
                        }
                    }
                }
            }
        }
        let sidecar = PathBuf::from(format!("{}.pmm.json", p.to_string_lossy()));
        if sidecar.exists() {
            let _ = fs::remove_file(sidecar);
        }
    };

    delete_path_and_sidecar(&mod_info.game_path);
    delete_path_and_sidecar(&mod_info.disabled_path);

    for extra in &mod_info.extra_files {
        delete_path_and_sidecar(extra);
    }

    // Also clean up PalSchema Storage and junction directories for this mod
    let binaries_dir = crate::dependency_checker::get_binaries_dir(game_path);
    let folder_name = crate::profiles::get_mod_folder_name(mod_info);
    let ue4ss_roots = vec![
        binaries_dir.join("ue4ss").join("Mods"),
        binaries_dir.join("Mods"),
        game_path.join("Mods").join("NativeMods").join("UE4SS").join("Mods"),
    ];

    for u_dir in &ue4ss_roots {
        if !u_dir.exists() {
            continue;
        }

        let palschema_mods_dir = u_dir.join("PalSchema").join("mods");
        if palschema_mods_dir.exists() {
            // Direct mod dir
            let direct_ps = palschema_mods_dir.join(&folder_name);
            if direct_ps.exists() {
                if crate::profiles::is_junction_or_symlink(&direct_ps) {
                    let _ = crate::profiles::remove_junction_or_symlink(&direct_ps);
                } else {
                    let _ = fs::remove_dir_all(&direct_ps);
                }
            }
            let direct_ps_name = palschema_mods_dir.join(&mod_info.name);
            if direct_ps_name.exists() {
                if crate::profiles::is_junction_or_symlink(&direct_ps_name) {
                    let _ = crate::profiles::remove_junction_or_symlink(&direct_ps_name);
                } else {
                    let _ = fs::remove_dir_all(&direct_ps_name);
                }
            }

            // Storage dir
            let storage_dir = palschema_mods_dir.join("Storage");
            if storage_dir.exists() {
                let s_mod = storage_dir.join(&folder_name);
                if s_mod.exists() {
                    let _ = fs::remove_dir_all(&s_mod);
                }
                let s_name = storage_dir.join(&mod_info.name);
                if s_name.exists() {
                    let _ = fs::remove_dir_all(&s_name);
                }
            }

            // Numbered FLO junctions (e.g. 001_ModName)
            if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if name.len() > 4 && name[..3].chars().all(|c| c.is_ascii_digit()) && name.as_bytes()[3] == b'_' {
                        let sub_name = &name[4..];
                        if sub_name.eq_ignore_ascii_case(&folder_name) || sub_name.eq_ignore_ascii_case(&mod_info.name) {
                            if crate::profiles::is_junction_or_symlink(&p) {
                                let _ = crate::profiles::remove_junction_or_symlink(&p);
                            } else if p.is_dir() {
                                let _ = fs::remove_dir_all(&p);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Diffs newly installed config files against the snapshotted pre-purge files.
/// ONLY compares files with the EXACT SAME relative path and filename.
pub fn diff_exact_match_configs(
    old_snapshot: &HashMap<String, (String, String)>,
    game_path: &Path,
    final_mod: &ModInfo,
) -> Vec<ConfigDiff> {
    let mut diffs = Vec::new();
    if old_snapshot.is_empty() {
        return diffs;
    }

    let roots = collect_mod_disk_roots(game_path, final_mod);

    for (root_dir, is_standalone_file) in roots {
        if is_standalone_file {
            if root_dir.exists() && root_dir.is_file() {
                let file_name = root_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
                let key = file_name.to_lowercase();
                if let Some((exact_old_path, old_content)) = old_snapshot.get(&key) {
                    if let Ok(new_content) = fs::read_to_string(&root_dir) {
                        if &new_content != old_content {
                            let ext = root_dir.extension().and_then(|e| e.to_str()).unwrap_or("");
                            if let Some((keys_user_changed, keys_author_added, keys_removed)) =
                                crate::config_merge::generate_config_diff(old_content, &new_content, ext)
                            {
                                if !keys_user_changed.is_empty() {
                                    diffs.push(ConfigDiff {
                                        file_name: exact_old_path.clone(),
                                        keys_user_changed,
                                        keys_added_by_author: keys_author_added,
                                        keys_removed_by_author: keys_removed,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        } else if root_dir.exists() && root_dir.is_dir() {
            for entry in walkdir::WalkDir::new(&root_dir).into_iter().filter_map(|e| e.ok()) {
                let full_path = entry.path();
                if full_path.is_file() && is_candidate_text_file(full_path) {
                    let rel_path = full_path
                        .strip_prefix(&root_dir)
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| full_path.file_name().unwrap_or_default().to_string_lossy().to_string());

                    let key = rel_path.to_lowercase();
                    if let Some((exact_old_path, old_content)) = old_snapshot.get(&key) {
                        if let Ok(new_content) = fs::read_to_string(full_path) {
                            if &new_content != old_content {
                                let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                if let Some((keys_user_changed, keys_author_added, keys_removed)) =
                                    crate::config_merge::generate_config_diff(old_content, &new_content, ext)
                                {
                                    if !keys_user_changed.is_empty() {
                                        diffs.push(ConfigDiff {
                                            file_name: exact_old_path.clone(),
                                            keys_user_changed,
                                            keys_added_by_author: keys_author_added,
                                            keys_removed_by_author: keys_removed,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    crate::logger::log(&format!(
        "[clean_slate] Exact-match diff found {} modified config files with user changes for mod '{}'",
        diffs.len(),
        final_mod.name
    ));

    diffs
}

fn collect_mod_disk_roots(game_path: &Path, mod_info: &ModInfo) -> Vec<(PathBuf, bool)> {
    let mut roots = Vec::new();

    let mut add_path = |path_str: &str| {
        if path_str.is_empty() {
            return;
        }
        let p = crate::config_merge::resolve_path_in_game(game_path, path_str);
        if p.exists() {
            let is_file = p.is_file();
            if !roots.iter().any(|(existing, _)| existing == &p) {
                roots.push((p, is_file));
            }
        }
    };

    add_path(&mod_info.game_path);
    add_path(&mod_info.disabled_path);

    for extra in &mod_info.extra_files {
        add_path(extra);
    }

    roots
}

fn is_candidate_text_file(p: &Path) -> bool {
    let file_name = p.file_name().unwrap_or_default().to_string_lossy();
    if file_name.starts_with('.') || file_name.eq_ignore_ascii_case("modinfo.pmm.json") || file_name.eq_ignore_ascii_case("enabled.txt") {
        return false;
    }
    match p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
        "json" | "jsonc" | "lua" | "ini" | "cfg" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_exact_match_configs_only_exact_names() {
        let temp_dir = std::env::temp_dir().join(format!("test_exact_diff_{}", uuid::Uuid::new_v4()));
        let mod_dir = temp_dir.join("PalSchema").join("mods").join("BetterBaseBuilding");
        fs::create_dir_all(&mod_dir.join("blueprints")).unwrap();

        // Old file 1: Config with modified user key
        let old_config_file = "config/settings.json".to_string();
        let old_content = "{\"Speed\": 10, \"AutoSave\": true}".to_string();

        // Old file 2: 0.4x ranch option
        let old_ranch_file = "blueprints/BuildingResize Ranch X 0.4.jsonc".to_string();
        let old_ranch_content = "{\"Scale\": 0.4}".to_string();

        let mut old_snapshot = HashMap::new();
        old_snapshot.insert(old_config_file.to_lowercase(), (old_config_file.clone(), old_content));
        old_snapshot.insert(old_ranch_file.to_lowercase(), (old_ranch_file.clone(), old_ranch_content));

        // Now newly installed mod has:
        // 1. config/settings.json with default Speed 5
        let new_config_path = mod_dir.join("config").join("settings.json");
        fs::create_dir_all(new_config_path.parent().unwrap()).unwrap();
        fs::write(&new_config_path, "{\"Speed\": 5, \"AutoSave\": true}").unwrap();

        // 2. blueprints/BuildingResize Ranch X 0.5.jsonc (different filename!)
        let new_ranch_path = mod_dir.join("blueprints").join("BuildingResize Ranch X 0.5.jsonc");
        fs::write(&new_ranch_path, "{\"Scale\": 0.5}").unwrap();

        let mod_info = ModInfo {
            id: "bbb".to_string(),
            name: "BetterBaseBuilding".to_string(),
            mod_type: crate::models::ModType::PalSchema,
            nexus_mod_id: None,
            nexus_url: None,
            nexus_author: None,
            nexus_summary: None,
            nexus_picture_url: None,
            nexus_endorsements: None,
            nexus_downloads: None,
            version: "2.2".to_string(),
            install_date: "".to_string(),
            source_zip: "".to_string(),
            config_path: None,
            config_paths: None,
            config_type: None,
            enabled: true,
            game_path: mod_dir.to_string_lossy().to_string(),
            disabled_path: "".to_string(),
            pak_destination: None,
            has_enabled_txt: false,
            mods_txt_order: None,
            extra_files: vec![],
            nexus_description: None,
            nexus_version_cached: None,
            nexus_cached_at: None,
            nexus_category: None,
            nexus_tags: vec![],
            github_repo: None,
            github_version: None,
            github_cached_at: None,
            update_date: None,
            library_zip: None,
            ignored_version: None,
            nexus_file_id: None,
            ignored_keys: None,
            has_pending_update: None,
            origin_load_method: None,
            custom_notes: None,
            original_name: None,
            custom_name: None,
            fomod_choices: None,
        };

        let diffs = diff_exact_match_configs(&old_snapshot, &temp_dir, &mod_info);

        // Result should only include settings.json because it matches exact path!
        // Ranch 0.4 vs Ranch 0.5 must NEVER match or diff!
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].file_name, "config/settings.json");
        assert_eq!(diffs[0].keys_user_changed.len(), 1);
        assert_eq!(diffs[0].keys_user_changed[0].key, "Speed");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
