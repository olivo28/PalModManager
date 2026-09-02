use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::models::ModInfo;
use super::types::{PakConflict, PakModSource};
use super::reader::{classify_asset_type, list_pak_entries};

/// Scans all active .pak files in Palworld (~mods, LogicMods, and Paks) and detects asset path collisions
pub fn scan_pak_conflicts(
    game_path: &Path,
    active_mods: &[ModInfo],
) -> (Vec<PakConflict>, u32) {
    let paks_dir = game_path.join("Pal").join("Content").join("Paks");
    let mods_dir = paks_dir.join("~mods");
    let logic_dir = paks_dir.join("LogicMods");

    // Collect all active .pak files on disk, separating active compatibility patches
    let mut pak_files: Vec<PathBuf> = Vec::new();
    let mut patch_files: Vec<PathBuf> = Vec::new();

    let scan_dir = |dir: &Path, list: &mut Vec<PathBuf>, patches: &mut Vec<PathBuf>| {
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        let is_target = p.extension().map_or(false, |ext| {
                            ext.eq_ignore_ascii_case("pak")
                                || ext.eq_ignore_ascii_case("utoc")
                                || ext.eq_ignore_ascii_case("ucas")
                        });
                        if is_target {
                            let filename = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                            // Ignore official base game paks if present
                            if filename.starts_with("Pal-Windows") {
                                continue;
                            }
                            // Detect compatibility patches (starting with zzz_)
                            if filename.starts_with("zzz_") && filename.to_lowercase().ends_with(".pak") {
                                patches.push(p);
                            } else {
                                list.push(p);
                            }
                        }
                    }
                }
            }
        }
    };

    scan_dir(&mods_dir, &mut pak_files, &mut patch_files);
    scan_dir(&logic_dir, &mut pak_files, &mut patch_files);
    scan_dir(&paks_dir, &mut pak_files, &mut patch_files);

    let total_paks_scanned = (pak_files.len() + patch_files.len()) as u32;
    if pak_files.len() < 2 {
        return (Vec::new(), total_paks_scanned);
    }

    // Index all assets covered by existing compatibility patches
    let mut patch_covered_assets: HashMap<String, String> = HashMap::new();
    for patch_path in &patch_files {
        let patch_filename = patch_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if let Ok(entries) = list_pak_entries(patch_path) {
            for entry in entries {
                let entry_lower = entry.to_lowercase();
                if !entry_lower.ends_with(".uasset") && !entry_lower.ends_with(".uexp") && !entry_lower.ends_with(".ubulk") {
                    continue;
                }
                let norm = if entry_lower.ends_with(".uexp") {
                    format!("{}.uasset", &entry[..entry.len() - 5])
                } else if entry_lower.ends_with(".ubulk") {
                    format!("{}.uasset", &entry[..entry.len() - 6])
                } else {
                    entry.clone()
                };
                patch_covered_assets.insert(norm, patch_filename.clone());
            }
        }
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
            let resolved_by_patch = patch_covered_assets.get(&internal_path).cloned();

            conflicts.push(PakConflict {
                internal_path,
                asset_name,
                asset_type,
                mods: sources,
                resolved_by_patch,
            });
        }
    }

    // Sort conflicts: Unresolved first, then DataTables (most critical), then alphabetically
    conflicts.sort_by(|a, b| {
        let a_resolved = a.resolved_by_patch.is_some();
        let b_resolved = b.resolved_by_patch.is_some();
        if !a_resolved && b_resolved {
            std::cmp::Ordering::Less
        } else if a_resolved && !b_resolved {
            std::cmp::Ordering::Greater
        } else if a.asset_type == "DataTable" && b.asset_type != "DataTable" {
            std::cmp::Ordering::Less
        } else if a.asset_type != "DataTable" && b.asset_type == "DataTable" {
            std::cmp::Ordering::Greater
        } else {
            a.internal_path.cmp(&b.internal_path)
        }
    });

    (conflicts, total_paks_scanned)
}
