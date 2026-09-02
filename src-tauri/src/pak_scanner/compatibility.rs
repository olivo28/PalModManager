use std::fs;
use std::path::{Path, PathBuf};
use crate::models::ModInfo;
use super::types::{DeprecatedSchemaNotice, GamePassPakNotice};
use super::reader::{extract_pak_entry, list_pak_entries};

/// Checks if game installation is PC Game Pass (WinGDK) and identifies active .pak files missing IoStore companion files (.utoc/.ucas)
pub fn check_gamepass_pak_compatibility(
    game_path: &Path,
    active_mods: &[ModInfo],
) -> (bool, Vec<GamePassPakNotice>) {
    let binaries_dir = crate::dependency_checker::get_binaries_dir(game_path);
    let is_xbox = binaries_dir.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("wingdk".to_string());

    if !is_xbox {
        return (false, Vec::new());
    }

    let paks_dir = game_path.join("Pal").join("Content").join("Paks");
    let mods_dir = paks_dir.join("~mods");
    let logic_dir = paks_dir.join("LogicMods");

    let mut notices = Vec::new();

    let check_dir = |dir: &Path, list: &mut Vec<GamePassPakNotice>| {
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        let is_pak = p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak"));
                        if is_pak {
                            let filename = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                            if filename.starts_with("Pal-Windows") {
                                continue;
                            }

                            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                            let utoc_exists = dir.join(format!("{}.utoc", stem)).exists();
                            let ucas_exists = dir.join(format!("{}.ucas", stem)).exists();

                            if !utoc_exists || !ucas_exists {
                                let mut missing = Vec::new();
                                if !utoc_exists { missing.push(".utoc".to_string()); }
                                if !ucas_exists { missing.push(".ucas".to_string()); }

                                let (mod_id, mod_name) = active_mods
                                    .iter()
                                    .find(|m| {
                                        if !m.enabled { return false; }
                                        let norm_pak = filename.to_lowercase();
                                        if m.game_path.to_lowercase().contains(&norm_pak) { return true; }
                                        if m.extra_files.iter().any(|extra| extra.to_lowercase().contains(&norm_pak)) { return true; }
                                        false
                                    })
                                    .map(|m| (m.id.clone(), m.name.clone()))
                                    .unwrap_or_else(|| ("untracked".to_string(), filename.clone()));

                                list.push(GamePassPakNotice {
                                    mod_id,
                                    mod_name,
                                    pak_filename: filename,
                                    pak_path: p.to_string_lossy().to_string(),
                                    missing_containers: missing,
                                });
                            }
                        }
                    }
                }
            }
        }
    };

    check_dir(&mods_dir, &mut notices);
    check_dir(&logic_dir, &mut notices);

    (true, notices)
}

/// Validates active mods against the loaded USMAP engine schema for Palworld
pub fn check_mod_schema_compatibility(active_mods: &[ModInfo]) -> Vec<DeprecatedSchemaNotice> {
    let schema = match crate::usmap::get_or_load_schema("") {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut notices = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();

    for m in active_mods {
        if !m.enabled || m.game_path.is_empty() {
            continue;
        }

        let mut candidate_paks = Vec::new();
        let is_pak = |s: &str| s.to_lowercase().ends_with(".pak");
        if is_pak(&m.game_path) {
            candidate_paks.push(PathBuf::from(&m.game_path));
        }
        for extra in &m.extra_files {
            if is_pak(extra) {
                candidate_paks.push(PathBuf::from(extra));
            }
        }

        for pak_path in candidate_paks {
            if !pak_path.exists() { continue; }
            if let Ok(entries) = list_pak_entries(&pak_path) {
                for entry in entries {
                    let lower = entry.to_lowercase();
                    if lower.ends_with(".uasset") && (lower.contains("datatable") || lower.contains("dt_") || lower.contains("character") || lower.contains("save")) {
                        if let Ok(uasset_bytes) = extract_pak_entry(&pak_path, &entry) {
                            let text = String::from_utf8_lossy(&uasset_bytes);
                            for token in text.split(|c: char| c == '\0' || c < ' ' || c > '~') {
                                let trimmed = token.trim();
                                if trimmed.starts_with("Pal") && trimmed.len() > 6 && trimmed.len() < 64 {
                                    if !schema.names.iter().any(|n| n.eq_ignore_ascii_case(trimmed)) && !schema.structs.contains_key(trimmed) {
                                        if trimmed.ends_with("Parameter") || trimmed.ends_with("Data") || trimmed.ends_with("SaveData") {
                                            let dedup_key = format!("{}:{}:{}", m.id, entry, trimmed);
                                            if seen_keys.insert(dedup_key) {
                                                notices.push(DeprecatedSchemaNotice {
                                                    mod_id: m.id.clone(),
                                                    mod_name: m.name.clone(),
                                                    asset_path: entry.clone(),
                                                    struct_name: trimmed.to_string(),
                                                    message: format!("Asset references unmapped or legacy engine structure '{}'", trimmed),
                                                });
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    notices
}
