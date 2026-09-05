use std::fs;
use std::path::Path;
use crate::models::{ModInfo, ModType};
use super::super::utils::{detect_config, file_install_date};
use super::meta::load_pmm_meta;

pub fn scan_disabled_mods(disabled_base: &Path, results: &mut Vec<ModInfo>) {
    let type_dirs = [
        ("ue4ss", ModType::Ue4ss),
        ("palschema", ModType::PalSchema),
        ("hybrid", ModType::Hybrid),
    ];
    for (type_str, mod_type) in &type_dirs {
        let dir = disabled_base.join(type_str);
        if !dir.exists() { continue; }
        if let Ok(rd) = fs::read_dir(&dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if !entry.file_type().map_or(false, |ft| ft.is_dir()) { continue; }
                let mod_name = entry.file_name().to_string_lossy().to_string();
                if type_str == &"hybrid" && ["logicmods", "palschema", "pak", "ue4ss", "extras"].contains(&mod_name.to_lowercase().as_str()) {
                    continue;
                }
                let mod_path = entry.path();

                if let Some(m) = load_pmm_meta(&mod_path) {
                    results.push(m);
                    continue;
                }

                let install_date = file_install_date(&mod_path);
                results.push(ModInfo {
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
                let mod_name = file_stem.strip_suffix("_P").unwrap_or(&file_stem).to_string();
                let install_date = file_install_date(&entry.path());
                let mt = if *pak_type == "logicmods" { ModType::LogicMods } else { ModType::Pak };
                results.push(ModInfo {
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
