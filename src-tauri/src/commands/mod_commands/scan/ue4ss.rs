use std::fs;
use std::path::Path;
use crate::models::{ModInfo, ModType};
use super::super::utils::{detect_config, file_install_date};
use super::meta::load_pmm_meta;

pub fn scan_ue4ss_mods(dir: &Path, results: &mut Vec<ModInfo>, ignored_names: &std::collections::HashSet<String>) {
    if !dir.exists() { return; }

    let mods_txt_path = dir.join("mods.txt");
    let mut mods_txt_states: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut mods_txt_positions: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    if mods_txt_path.exists() {
        if let Ok(content) = fs::read_to_string(&mods_txt_path) {
            for (line_idx, line) in content.lines().enumerate() {
                let line_clean = line.trim();
                if line_clean.starts_with(';') || line_clean.starts_with("//") {
                    continue;
                }
                if let Some(pos) = line_clean.find(':') {
                    let name = line_clean[..pos].trim().to_lowercase();
                    let val = line_clean[pos+1..].trim();
                    mods_txt_states.insert(name.clone(), val == "1");
                    mods_txt_positions.insert(name, line_idx as u32);
                } else if !line_clean.is_empty() {
                    let name = line_clean.to_lowercase();
                    mods_txt_states.insert(name.clone(), true);
                    mods_txt_positions.insert(name, line_idx as u32);
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) { continue; }
            let mod_name = entry.file_name().to_string_lossy().to_string();
            if ignored_names.contains(&mod_name.to_lowercase()) { continue; }
            let mod_path = entry.path();
            if ["ConsoleUnlocker", "LuaPlugin", "PalSchema"].contains(&mod_name.as_str()) { continue; }

            let is_native_mod = ["BPModLoaderMod", "CheatManagerEnablerMod", "ConsoleCommandsMod", "ConsoleEnablerMod", "Keybinds", "LineTraceMod", "SplitScreenMod", "BPML_GenericFunctions", "shared", "adapters"].contains(&mod_name.as_str());

            let name_lower = mod_name.to_lowercase();
            let is_in_mods_txt = mods_txt_states.contains_key(&name_lower);
            let is_enabled = if let Some(&state) = mods_txt_states.get(&name_lower) {
                state
            } else {
                mod_path.join("enabled.txt").exists() || is_native_mod
            };

            let order_pos = mods_txt_positions.get(&name_lower).copied();
            let origin_load = if is_in_mods_txt {
                Some("mods_txt".to_string())
            } else if mod_path.join("enabled.txt").exists() {
                Some("enabled_txt".to_string())
            } else {
                None
            };

            if let Some(mut m) = load_pmm_meta(&mod_path) {
                m.enabled = is_enabled;
                if m.mods_txt_order.is_none() && order_pos.is_some() {
                    m.mods_txt_order = order_pos;
                }
                if m.origin_load_method.is_none() && origin_load.is_some() {
                    m.origin_load_method = origin_load;
                }
                results.push(m);
                continue;
            }

            let author = if is_native_mod { Some("UE4SS Native Mod".to_string()) } else { None };
            let summary = if is_native_mod { Some("Core dependency mod installed by UE4SS. Controlled by mods.txt.".to_string()) } else { None };

            results.push(ModInfo {
                id: mod_name.clone(),
                name: mod_name.clone(),
                mod_type: ModType::Ue4ss,
                nexus_mod_id: None, nexus_url: None, nexus_author: author, nexus_summary: summary,
                nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                version: "1.0.0".to_string(),
                install_date: file_install_date(&mod_path),
                source_zip: String::new(),
                config_path: detect_config(&mod_path),
                config_type: Some("auto".to_string()),
                enabled: is_enabled,
                game_path: mod_path.to_string_lossy().to_string(),
                disabled_path: String::new(),
                pak_destination: None,
                has_enabled_txt: mod_path.join("enabled.txt").exists(),
                mods_txt_order: order_pos,
                extra_files: Vec::new(),
                nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                nexus_category: None, nexus_tags: Vec::new(),
                github_repo: None, github_version: None, github_cached_at: None,
                update_date: None, library_zip: None,
                ignored_version: None,
                nexus_file_id: None,
                ignored_keys: None,
                has_pending_update: None,
                origin_load_method: origin_load,
                custom_notes: None,
            });
        }
    }
}
