use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use crate::models::{ModInfo, ModType};
use super::super::utils::file_install_date;
use super::meta::load_pmm_meta;

pub fn scan_pak_mods(
    dir: &Path,
    pak_type: &str,
    results: &mut Vec<ModInfo>,
    registered_patches: &[crate::pak_patcher::RegisteredPatch],
) {
    if !dir.exists() { return; }
    for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        let fname = entry.path().file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if fname.ends_with(".pmm.json.pmm.json") || fname.ends_with(".json.pmm.json") {
            let _ = fs::remove_file(entry.path());
            continue;
        }
        let ext = entry.path().extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        if ext != "pak" { continue; }

        let mod_path = entry.path();
        if let Some(m) = load_pmm_meta(&mod_path) {
            results.push(m);
            continue;
        }

        let file_stem = entry.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "unknown".to_string());
        let filename = entry.path().file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let path_str = entry.path().to_string_lossy().to_string();

        // Ignore internal compatibility patches (e.g. zzz_PMM_Patch_*, zzz_MergedMods_*, or in registry) from main mods list
        if file_stem.starts_with("zzz_") || registered_patches.iter().any(|p| p.pak_filename == filename || p.pak_path == path_str) {
            continue;
        }

        let mod_name = file_stem.trim_end_matches("_P").to_string();
        let mod_path = entry.path();
        let install_date = file_install_date(mod_path);
        let mod_path_str = mod_path.to_string_lossy().to_string();
        let mut extra_files: Vec<String> = Vec::new();
        for companion_ext in &["ucas", "utoc"] {
            let companion_path = dir.join(format!("{}.{}", file_stem, companion_ext));
            if companion_path.exists() {
                extra_files.push(companion_path.to_string_lossy().to_string());
            }
        }
        let is_altermatic = mod_name.to_lowercase().contains("altermatic")
            || file_stem.to_lowercase().contains("altermatic")
            || dir.join("SwapJSON").join(format!("{}.json", file_stem)).exists()
            || dir.join("SwapJSON").join(format!("{}.json", mod_name)).exists()
            || dir.parent().map_or(false, |p| p.join("~mods").join("SwapJSON").join(format!("{}.json", file_stem)).exists() || p.join("~mods").join("SwapJSON").join(format!("{}.json", mod_name)).exists());

        let is_unipalui = mod_name.to_lowercase().contains("unipalui") || file_stem.to_lowercase().contains("unipalui");

        let mt = if is_altermatic {
            ModType::Altermatic
        } else if is_unipalui {
            ModType::Hybrid
        } else if pak_type == "logicmods" {
            ModType::LogicMods
        } else {
            ModType::Pak
        };
        results.push(ModInfo {
            id: mod_name.clone(), name: mod_name.clone(), mod_type: mt,
            nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
            nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
            version: "unknown".to_string(), install_date,
            source_zip: String::new(), config_path: None, config_type: None,
            enabled: true, game_path: mod_path_str, disabled_path: String::new(),
            pak_destination: Some(pak_type.to_string()), has_enabled_txt: false,
            mods_txt_order: None, extra_files,
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
        });
    }
}
