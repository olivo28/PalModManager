use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use crate::models::{ModInfo, ModType};
use super::super::utils::file_install_date;
use super::meta::load_pmm_meta;

pub fn scan_palschema_mods(dir: &Path, results: &mut Vec<ModInfo>, ignored_names: &std::collections::HashSet<String>) {
    if !dir.exists() { return; }
    let storage_dir = dir.parent().map(|p| p.join("Storage"));

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) { continue; }
            let raw_name = entry.file_name().to_string_lossy().to_string();
            let clean_name = if raw_name.len() > 4 && raw_name[..3].chars().all(|c| c.is_ascii_digit()) && raw_name.as_bytes()[3] == b'_' {
                raw_name[4..].to_string()
            } else {
                raw_name.clone()
            };

            if ignored_names.contains(&clean_name.to_lowercase()) || ignored_names.contains(&raw_name.to_lowercase()) {
                continue;
            }
            let mod_path = entry.path();

            if let Some(mut m) = load_pmm_meta(&mod_path) {
                if m.name.len() > 4 && m.name[..3].chars().all(|c| c.is_ascii_digit()) && m.name.as_bytes()[3] == b'_' {
                    m.name = m.name[4..].to_string();
                }
                results.push(m);
                continue;
            }

            // Check if storage folder has .pmm.json
            if let Some(ref s_dir) = storage_dir {
                let storage_mod_dir = s_dir.join(&clean_name);
                if storage_mod_dir.exists() {
                    if let Some(mut m) = load_pmm_meta(&storage_mod_dir) {
                        m.game_path = mod_path.to_string_lossy().to_string();
                        m.enabled = true;
                        if m.name.len() > 4 && m.name[..3].chars().all(|c| c.is_ascii_digit()) && m.name.as_bytes()[3] == b'_' {
                            m.name = m.name[4..].to_string();
                        }
                        results.push(m);
                        continue;
                    }
                }
            }

            let has_json = WalkDir::new(&mod_path).max_depth(2).into_iter().filter_map(|e| e.ok()).any(|e| {
                e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "json" || ext == "jsonc")
            });

            if has_json {
                let install_date = file_install_date(&mod_path);
                results.push(ModInfo {
                    id: clean_name.clone(),
                    name: clean_name.clone(),
                    mod_type: ModType::PalSchema,
                    nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
                    nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                    version: "unknown".to_string(), install_date,
                    source_zip: String::new(), config_path: None,
                    config_type: None, enabled: true,
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
                    ignored_keys: None,
                    has_pending_update: None,
                    origin_load_method: None,
                    custom_notes: None,
                    original_name: None,
                    custom_name: None,
                });
            }
        }
    }
}
