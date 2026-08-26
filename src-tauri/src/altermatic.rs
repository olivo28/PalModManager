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

/// Checks presence of `LogicMods/Altermatic.pak` and `LogicMods/UniPalUI.pak`
pub fn check_altermatic_deps(game_path: &Path) -> AltermaticDepStatus {
    let logic_dir = get_logicmods_dir(game_path);
    let altermatic_present = logic_dir.join("Altermatic.pak").exists();
    let unipalui_present = logic_dir.join("UniPalUI.pak").exists();

    AltermaticDepStatus {
        altermatic_present,
        unipalui_present,
    }
}

/// Scans `~mods/SwapJSON/` for `.json` files (excluding `_LoadList.json`) and returns basenames without extension.
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
                
                if file_name.eq_ignore_ascii_case("_LoadList.json") {
                    continue;
                }

                if file_name.to_lowercase().ends_with(".json") {
                    if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) {
                        configs.push(stem);
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
        if let Ok(list) = serde_json::from_str::<Vec<String>>(&content) {
            return list;
        }
    }

    Vec::new()
}

/// Writes `_LoadList.json` formatted as a JSON array.
pub fn write_load_list(game_path: &Path, entries: &[String]) -> Result<(), String> {
    let swap_dir = get_swapjson_dir(game_path);
    if !swap_dir.exists() {
        fs::create_dir_all(&swap_dir)
            .map_err(|e| format!("Failed to create SwapJSON directory: {}", e))?;
    }

    let load_list_path = get_load_list_path(game_path);
    let json_content = serde_json::to_string_pretty(entries)
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

    // Collect all JSON config stems that belong to currently enabled mods
    let mut active_entries = Vec::new();

    for config_stem in &available_configs {
        let lower_stem = config_stem.to_lowercase();
        let mut is_enabled = false;

        // Check if any enabled mod claims or matches this SwapJSON config
        for mod_info in all_mods {
            if !enabled_mod_ids.contains(&mod_info.id) || !mod_info.enabled {
                continue;
            }

            // 1. Check extra_files and config_path for explicit SwapJSON references
            let mut matches_mod = false;

            if let Some(cfg) = &mod_info.config_path {
                let cfg_lower = cfg.to_lowercase().replace('\\', "/");
                if cfg_lower.contains("swapjson") && cfg_lower.contains(&lower_stem) {
                    matches_mod = true;
                }
            }

            if !matches_mod {
                for extra in &mod_info.extra_files {
                    let extra_lower = extra.to_lowercase().replace('\\', "/");
                    if extra_lower.contains("swapjson") && extra_lower.contains(&lower_stem) {
                        matches_mod = true;
                        break;
                    }
                }
            }

            // 2. Fallback: match by mod name / ID if it's a dedicated swap mod
            if !matches_mod {
                let mod_name_norm = mod_info.name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
                let stem_norm = lower_stem.replace(|c: char| !c.is_alphanumeric(), "");
                if !mod_name_norm.is_empty() && (mod_name_norm.contains(&stem_norm) || stem_norm.contains(&mod_name_norm)) {
                    matches_mod = true;
                }
            }

            if matches_mod {
                is_enabled = true;
                break;
            }
        }

        // If the mod is not tracked as a distinct managed mod (e.g. manually dropped in SwapJSON),
        // we keep it enabled if there are no conflicting disabled states
        if is_enabled {
            active_entries.push(config_stem.clone());
        }
    }

    // If no specific mod mappings matched but configs exist, include all currently present configs in SwapJSON
    // to prevent breaking existing manual installs
    let final_entries = if active_entries.is_empty() && !enabled_mod_ids.is_empty() {
        // Only fallback if no mods explicitly registered SwapJSON files
        let any_mod_has_swapjson = all_mods.iter().any(|m| {
            m.extra_files.iter().any(|f| f.to_lowercase().contains("swapjson"))
                || m.config_path.as_deref().unwrap_or("").to_lowercase().contains("swapjson")
        });

        if any_mod_has_swapjson {
            active_entries
        } else {
            available_configs
        }
    } else {
        active_entries
    };

    write_load_list(game_path, &final_entries)?;
    Ok(final_entries)
}
