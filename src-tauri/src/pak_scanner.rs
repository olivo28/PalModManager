use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::models::ModInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakModSource {
    pub mod_id: String,
    pub mod_name: String,
    pub pak_filename: String,
    pub pak_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakConflict {
    pub internal_path: String,
    pub asset_name: String,
    pub asset_type: String, // "DataTable", "Blueprint", "Texture", "Mesh", "Asset"
    pub mods: Vec<PakModSource>,
}

/// Reads the file index of an Unreal Engine .pak container using pure-Rust `repak`
/// Reads the file index of an Unreal Engine .pak container using pure-Rust `repak`
pub fn list_pak_entries(pak_path: &Path) -> Result<Vec<String>, String> {
    let mut file = File::open(pak_path).map_err(|e| format!("Failed to open pak file {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;
    
    let pak = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| format!("Failed to read pak header {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;

    let files: Vec<String> = pak.files().into_iter().map(|s| s.replace('\\', "/")).collect();
    Ok(files)
}

/// Classifies asset type based on filename / extension
pub fn classify_asset_type(internal_path: &str) -> String {
    let lower = internal_path.to_lowercase();
    let filename = Path::new(internal_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    if filename.starts_with("DT_") || filename.starts_with("dt_") || lower.contains("/datatable/") {
        "DataTable".to_string()
    } else if filename.starts_with("BP_") || filename.starts_with("bp_") || lower.contains("/blueprint/") {
        "Blueprint".to_string()
    } else if lower.ends_with(".ubulk") || lower.contains("/texture/") || lower.contains("/ui/") {
        "Texture".to_string()
    } else if lower.contains("/model/") || lower.contains("/mesh/") || lower.contains("/skeletalmesh/") {
        "Mesh".to_string()
    } else {
        "Asset".to_string()
    }
}

/// Scans all active .pak files in Palworld (~mods and LogicMods) and detects asset path collisions
pub fn scan_pak_conflicts(
    game_path: &Path,
    active_mods: &[ModInfo],
) -> (Vec<PakConflict>, u32) {
    let paks_dir = game_path.join("Pal").join("Content").join("Paks");
    let mods_dir = paks_dir.join("~mods");
    let logic_dir = paks_dir.join("LogicMods");

    // Collect all active .pak files on disk
    let mut pak_files: Vec<PathBuf> = Vec::new();

    let scan_dir = |dir: &Path, list: &mut Vec<PathBuf>| {
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                        let filename = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        // Ignore official base game paks if present
                        if !filename.starts_with("Pal-Windows") {
                            list.push(p);
                        }
                    }
                }
            }
        }
    };

    scan_dir(&mods_dir, &mut pak_files);
    scan_dir(&logic_dir, &mut pak_files);

    let total_paks_scanned = pak_files.len() as u32;
    if pak_files.len() < 2 {
        return (Vec::new(), total_paks_scanned);
    }

    // Map: internal_path -> Vec<PakModSource>
    let mut collision_map: HashMap<String, Vec<PakModSource>> = HashMap::new();

    for pak_path in &pak_files {
        let pak_filename = pak_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let pak_path_str = pak_path.to_string_lossy().to_string();

        // Identify which mod owns this .pak file
        let (mod_id, mod_name) = active_mods
            .iter()
            .find(|m| {
                if !m.enabled { return false; }
                let norm_pak = pak_filename.to_lowercase();
                if m.game_path.to_lowercase().contains(&norm_pak) { return true; }
                if m.extra_files.iter().any(|extra| extra.to_lowercase().contains(&norm_pak)) { return true; }
                false
            })
            .map(|m| (m.id.clone(), m.name.clone()))
            .unwrap_or_else(|| ("untracked".to_string(), pak_filename.clone()));

        if let Ok(entries) = list_pak_entries(pak_path) {
            for entry in entries {
                let entry_lower = entry.to_lowercase();
                // Filter out non-asset entries (e.g. metadata / manifest)
                if !entry_lower.ends_with(".uasset") && !entry_lower.ends_with(".uexp") && !entry_lower.ends_with(".ubulk") {
                    continue;
                }

                // Group companion .uexp/.ubulk under the main .uasset stem to avoid duplicate clutter
                let normalized_internal_path = if entry_lower.ends_with(".uexp") {
                    format!("{}.uasset", &entry[..entry.len() - 5])
                } else if entry_lower.ends_with(".ubulk") {
                    format!("{}.uasset", &entry[..entry.len() - 6])
                } else {
                    entry.clone()
                };

                let source = PakModSource {
                    mod_id: mod_id.clone(),
                    mod_name: mod_name.clone(),
                    pak_filename: pak_filename.clone(),
                    pak_path: pak_path_str.clone(),
                };

                let list = collision_map.entry(normalized_internal_path).or_default();
                // Avoid duplicate source entries from the same pak (e.g. uasset + uexp from same pak)
                if !list.iter().any(|s| s.pak_path == source.pak_path) {
                    list.push(source);
                }
            }
        }
    }

    let mut conflicts = Vec::new();

    for (internal_path, sources) in collision_map {
        // A conflict exists when 2 or more distinct .pak files provide the exact same internal asset
        if sources.len() > 1 {
            let asset_name = Path::new(&internal_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let asset_type = classify_asset_type(&internal_path);

            conflicts.push(PakConflict {
                internal_path,
                asset_name,
                asset_type,
                mods: sources,
            });
        }
    }

    // Sort conflicts: DataTables first (most critical), then alphabetically
    conflicts.sort_by(|a, b| {
        if a.asset_type == "DataTable" && b.asset_type != "DataTable" {
            std::cmp::Ordering::Less
        } else if a.asset_type != "DataTable" && b.asset_type == "DataTable" {
            std::cmp::Ordering::Greater
        } else {
            a.internal_path.cmp(&b.internal_path)
        }
    });

    (conflicts, total_paks_scanned)
}
