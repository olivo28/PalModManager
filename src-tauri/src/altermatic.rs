use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::models::ModInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltermaticDepStatus {
    pub altermatic_present: bool,
    pub unipalui_present: bool,
}

/// Helper to get the SwapJSON directory in Palworld:
/// `Pal/Content/Paks/~mods/SwapJSON/`
pub fn get_swapjson_dir(game_path: &Path) -> PathBuf {
    game_path
        .join("Pal")
        .join("Content")
        .join("Paks")
        .join("~mods")
        .join("SwapJSON")
}

/// Helper to get the `_LoadList.json` path in Palworld:
/// `Pal/Content/Paks/~mods/SwapJSON/_LoadList.json`
pub fn get_load_list_path(game_path: &Path) -> PathBuf {
    get_swapjson_dir(game_path).join("_LoadList.json")
}

/// Helper to get the LogicMods directory in Palworld:
/// `Pal/Content/Paks/LogicMods/`
pub fn get_logicmods_dir(game_path: &Path) -> PathBuf {
    game_path
        .join("Pal")
        .join("Content")
        .join("Paks")
        .join("LogicMods")
}

/// Checks presence of Altermatic (Nexus #1626) and UniPalUI (Nexus #1894) across ~mods, LogicMods, and UE4SS mods
pub fn check_altermatic_deps(game_path: &Path) -> AltermaticDepStatus {
    let paks_dir = game_path.join("Pal").join("Content").join("Paks");
    let logic_dir = paks_dir.join("LogicMods");
    let tildemods_dir = paks_dir.join("~mods");
    let ue4ss_dir = crate::dependency_checker::get_ue4ss_mods_dir(game_path);

    let altermatic_present = paks_dir.join("Altermatic.pak").exists()
        || paks_dir.join("Altermatic_P.pak").exists()
        || tildemods_dir.join("Altermatic.pak").exists()
        || tildemods_dir.join("Altermatic_P.pak").exists()
        || logic_dir.join("Altermatic.pak").exists()
        || logic_dir.join("Altermatic_P.pak").exists()
        || logic_dir.join("Altermatic").exists()
        || ue4ss_dir.join("Altermatic").exists();

    let unipalui_present = paks_dir.join("UniPalUI.pak").exists()
        || paks_dir.join("UniPalUI_P.pak").exists()
        || tildemods_dir.join("UniPalUI.pak").exists()
        || tildemods_dir.join("UniPalUI_P.pak").exists()
        || logic_dir.join("UniPalUI.pak").exists()
        || logic_dir.join("UniPalUI_P.pak").exists()
        || logic_dir.join("UniPalUI").exists()
        || ue4ss_dir.join("UniPalUI").exists();

    AltermaticDepStatus {
        altermatic_present,
        unipalui_present,
    }
}

/// Scans `~mods/SwapJSON/` for `.json` files (excluding `_LoadList.json`, `_LoadList_Example.json`, etc.) and returns basenames without extension.
pub fn detect_swapjson_configs(game_path: &Path) -> Vec<String> {
    let swap_dir = get_swapjson_dir(game_path);
    if !swap_dir.exists() {
        return Vec::new();
    }

    let mut configs = Vec::new();
    if let Ok(entries) = fs::read_dir(&swap_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                
                // Skip helper/template files and any file starting with '_'
                if file_name.starts_with('_') || file_name.starts_with('.') {
                    continue;
                }

                if file_name.to_lowercase().ends_with(".json") {
                    if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) {
                        let clean_stem = if stem.to_lowercase().ends_with(".swap") {
                            stem[..stem.len() - 5].to_string()
                        } else {
                            stem
                        };
                        if !clean_stem.is_empty() && !configs.contains(&clean_stem) {
                            configs.push(clean_stem);
                        }
                    }
                }
            }
        }
    }

    configs.sort();
    configs
}

/// Reads current `_LoadList.json` (returns `[]` if missing or invalid).
pub fn read_load_list(game_path: &Path) -> Vec<String> {
    let load_list_path = get_load_list_path(game_path);
    if !load_list_path.exists() {
        return Vec::new();
    }

    if let Ok(content) = fs::read_to_string(&load_list_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(arr) = val.get("LoadList").and_then(|v| v.as_array()) {
                return arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && !s.starts_with('_'))
                    .collect();
            }
        }
        if let Ok(list) = serde_json::from_str::<Vec<String>>(&content) {
            return list.into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && !s.starts_with('_'))
                .collect();
        }
    }

    Vec::new()
}

/// Writes `_LoadList.json` formatted in the exact JSON format expected by Altermatic:
/// {
///   "LoadList": [
///     "Config1",
///     "Config2"
///   ]
/// }
pub fn write_load_list(game_path: &Path, entries: &[String]) -> Result<(), String> {
    let swap_dir = get_swapjson_dir(game_path);
    if !swap_dir.exists() {
        fs::create_dir_all(&swap_dir)
            .map_err(|e| format!("Failed to create SwapJSON directory: {}", e))?;
    }

    let load_list_path = get_load_list_path(game_path);
    let clean_entries: Vec<String> = entries.iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with('_'))
        .collect();

    let payload = serde_json::json!({
        "LoadList": clean_entries
    });

    let json_content = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Failed to serialize _LoadList.json: {}", e))?;

    fs::write(&load_list_path, json_content)
        .map_err(|e| format!("Failed to write _LoadList.json: {}", e))?;

    Ok(())
}

/// Master sync function: computes the set of SwapJSON configs belonging to **enabled** mods,
/// compares them against the installed SwapJSON files, and regenerates `_LoadList.json`.
pub fn sync_load_list(
    game_path: &Path,
    enabled_mod_ids: &[String],
    all_mods: &[ModInfo],
) -> Result<Vec<String>, String> {
    let available_configs = detect_swapjson_configs(game_path);
    if available_configs.is_empty() {
        let load_list_path = get_load_list_path(game_path);
        if load_list_path.exists() {
            let _ = write_load_list(game_path, &[]);
        }
        return Ok(Vec::new());
    }

    let mut disabled_configs = std::collections::HashSet::new();
    let mut explicit_enabled_configs = std::collections::HashSet::new();

    for mod_info in all_mods {
        let is_mod_enabled = mod_info.enabled && (
            enabled_mod_ids.is_empty() 
            || enabled_mod_ids.iter().any(|id| id == &mod_info.id || id.eq_ignore_ascii_case(&mod_info.name))
        );
        let mut mod_swap_stems = Vec::new();

        if let Some(cfg) = &mod_info.config_path {
            let cfg_norm = cfg.replace('\\', "/");
            if cfg_norm.to_lowercase().contains("swapjson") {
                if let Some(stem) = Path::new(&cfg_norm).file_stem().map(|s| s.to_string_lossy().to_string()) {
                    if !stem.starts_with('_') {
                        mod_swap_stems.push(stem);
                    }
                }
            }
        }

        for extra in &mod_info.extra_files {
            let extra_norm = extra.replace('\\', "/");
            if extra_norm.to_lowercase().contains("swapjson") && extra_norm.to_lowercase().ends_with(".json") {
                if let Some(stem) = Path::new(&extra_norm).file_stem().map(|s| s.to_string_lossy().to_string()) {
                    if !stem.starts_with('_') {
                        mod_swap_stems.push(stem);
                    }
                }
            }
        }

        let mod_name_norm = mod_info.name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
        for cfg in &available_configs {
            let cfg_norm = cfg.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
            if !mod_name_norm.is_empty() && (mod_name_norm.contains(&cfg_norm) || cfg_norm.contains(&mod_name_norm)) {
                mod_swap_stems.push(cfg.clone());
            }
        }

        for stem in mod_swap_stems {
            if is_mod_enabled {
                explicit_enabled_configs.insert(stem.to_lowercase());
            } else {
                disabled_configs.insert(stem.to_lowercase());
            }
        }
    }

    let mut active_entries = Vec::new();
    for config_stem in &available_configs {
        let lower = config_stem.to_lowercase();
        if disabled_configs.contains(&lower) && !explicit_enabled_configs.contains(&lower) {
            continue;
        }
        active_entries.push(config_stem.clone());
    }

    write_load_list(game_path, &active_entries)?;
    crate::logger::log(&format!("sync_load_list: Successfully updated _LoadList.json with {} active swap configs: {:?}", active_entries.len(), active_entries));
    Ok(active_entries)
}
