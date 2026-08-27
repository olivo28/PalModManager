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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakInternalItem {
    pub path: String,
    pub name: String,
    pub asset_type: String, // "DataTable", "Blueprint", "Texture", "Mesh", "Audio", "Asset"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakInspectionResult {
    pub pak_name: String,
    pub total_files: usize,
    pub files: Vec<PakInternalItem>,
    pub summary_by_type: HashMap<String, usize>,
}

/// Reads the file index and classifies all internal assets using `repak`
pub fn list_pak_entries_detailed(pak_path: &Path) -> Result<PakInspectionResult, String> {
    let mut file = File::open(pak_path).map_err(|e| format!("Failed to open pak file {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;
    
    let pak = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| format!("Failed to read pak header {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;

    let pak_name = pak_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let raw_files: Vec<String> = pak.files().into_iter().map(|s| s.replace('\\', "/")).collect();
    let total_files = raw_files.len();

    let mut summary_by_type: HashMap<String, usize> = HashMap::new();
    let mut files = Vec::with_capacity(total_files);

    for p in raw_files {
        let asset_type = classify_asset_type(&p);
        *summary_by_type.entry(asset_type.clone()).or_insert(0) += 1;
        let name = Path::new(&p)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| p.clone());

        files.push(PakInternalItem {
            path: p,
            name,
            asset_type,
        });
    }

    Ok(PakInspectionResult {
        pak_name,
        total_files,
        files,
        summary_by_type,
    })
}

/// Reads the file index of an Unreal Engine .pak container using pure-Rust `repak`
pub fn list_pak_entries(pak_path: &Path) -> Result<Vec<String>, String> {
    let mut file = File::open(pak_path).map_err(|e| format!("Failed to open pak file {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;
    
    let pak = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| format!("Failed to read pak header {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;

    let files: Vec<String> = pak.files().into_iter().map(|s| s.replace('\\', "/")).collect();
    Ok(files)
}

/// Extracts raw bytes of a file inside a .pak archive
pub fn extract_pak_entry(pak_path: &Path, entry_path: &str) -> Result<Vec<u8>, String> {
    let mut file = File::open(pak_path).map_err(|e| format!("Failed to open pak file: {e}"))?;
    let pak = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| format!("Failed to read pak header: {e}"))?;

    let out = pak.get(entry_path, &mut file)
        .map_err(|e| format!("Failed to extract {entry_path}: {e}"))?;
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetExportItem {
    pub object_name: String,
    pub class_name: String,
    pub outer_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetImportItem {
    pub object_name: String,
    pub class_name: String,
    pub class_package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetSummaryInfo {
    pub uasset_size_bytes: usize,
    pub uexp_size_bytes: Option<usize>,
    pub export_count: usize,
    pub import_count: usize,
    pub name_count: usize,
    pub package_flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetInspectionDetails {
    pub asset_name: String,
    pub asset_path: String,
    pub asset_type: String,
    pub engine_version: String,
    pub summary: UAssetSummaryInfo,
    pub exports: Vec<UAssetExportItem>,
    pub imports: Vec<UAssetImportItem>,
    pub names_sample: Vec<String>,
}

/// Deep inspection of an internal .uasset (and companion .uexp) from a .pak archive
pub fn inspect_uasset_deep(pak_path: &Path, uasset_internal_path: &str) -> Result<UAssetInspectionDetails, String> {
    use std::io::Cursor;
    use unreal_asset::{Asset, engine_version::EngineVersion, exports::ExportBaseTrait};

    // Normalize path: if caller passed a .uexp, .ubulk, or .uptnl, resolve the base .uasset
    let normalized_uasset_path = if uasset_internal_path.to_lowercase().ends_with(".uexp") {
        format!("{}.uasset", &uasset_internal_path[..uasset_internal_path.len() - 5])
    } else if uasset_internal_path.to_lowercase().ends_with(".ubulk") {
        format!("{}.uasset", &uasset_internal_path[..uasset_internal_path.len() - 6])
    } else if uasset_internal_path.to_lowercase().ends_with(".uptnl") {
        format!("{}.uasset", &uasset_internal_path[..uasset_internal_path.len() - 6])
    } else {
        uasset_internal_path.to_string()
    };

    let uasset_bytes = extract_pak_entry(pak_path, &normalized_uasset_path)?;
    let uasset_size_bytes = uasset_bytes.len();
    
    // Check if companion .uexp exists
    let uexp_path = if normalized_uasset_path.ends_with(".uasset") {
        format!("{}.uexp", &normalized_uasset_path[..normalized_uasset_path.len() - 7])
    } else {
        format!("{}.uexp", normalized_uasset_path)
    };
    
    let uexp_bytes = extract_pak_entry(pak_path, &uexp_path).ok();
    let uexp_size_bytes = uexp_bytes.as_ref().map(|b| b.len());
    
    let mut uasset_cursor = Cursor::new(uasset_bytes);
    let mut uexp_cursor = uexp_bytes.map(Cursor::new);

    let asset = Asset::new(
        &mut uasset_cursor,
        uexp_cursor.as_mut(),
        EngineVersion::VER_UE5_1,
    ).map_err(|e| format!("Failed to parse Unreal Engine asset: {e}"))?;

    let asset_name = Path::new(&normalized_uasset_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| normalized_uasset_path.clone());
    let asset_type = classify_asset_type(&normalized_uasset_path);

    let mut exports = Vec::new();
    for export in &asset.asset_data.exports {
        let base = export.get_base_export();
        let object_name = base.object_name.get_content();
        let class_name = if base.class_index.index < 0 {
            let import_idx = (-base.class_index.index - 1) as usize;
            asset.imports.get(import_idx)
                .map(|i| i.object_name.get_content())
                .unwrap_or_else(|| format!("Import #{}", import_idx))
        } else if base.class_index.index > 0 {
            format!("Export #{}", base.class_index.index)
        } else {
            "Class".to_string()
        };

        let outer_name = if base.outer_index.index < 0 {
            let import_idx = (-base.outer_index.index - 1) as usize;
            asset.imports.get(import_idx).map(|i| i.object_name.get_content())
        } else if base.outer_index.index > 0 {
            let export_idx = (base.outer_index.index - 1) as usize;
            asset.asset_data.exports.get(export_idx).map(|e| e.get_base_export().object_name.get_content())
        } else {
            None
        };

        exports.push(UAssetExportItem {
            object_name,
            class_name,
            outer_name,
        });
    }

    let mut imports = Vec::new();
    for import in &asset.imports {
        imports.push(UAssetImportItem {
            object_name: import.object_name.get_content(),
            class_name: import.class_name.get_content(),
            class_package: import.class_package.get_content(),
        });
    }

    let name_map = asset.get_name_map();
    let names_sample: Vec<String> = name_map.get_ref().get_name_map_index_list().to_vec();
    let name_count = names_sample.len();

    let summary = UAssetSummaryInfo {
        uasset_size_bytes,
        uexp_size_bytes,
        export_count: exports.len(),
        import_count: imports.len(),
        name_count,
        package_flags: asset.asset_data.package_flags.bits(),
    };

    Ok(UAssetInspectionDetails {
        asset_name,
        asset_path: uasset_internal_path.to_string(),
        asset_type,
        engine_version: "Unreal Engine 5.1 (GVAS/Zen Package)".to_string(),
        summary,
        exports,
        imports,
        names_sample,
    })
}

/// Parses an internal .uasset (and companion .uexp if present) from a .pak and returns exported rows/objects
pub fn inspect_uasset_from_pak(pak_path: &Path, uasset_internal_path: &str) -> Result<Vec<String>, String> {
    inspect_uasset_deep(pak_path, uasset_internal_path)
        .map(|res| {
            res.exports.into_iter().map(|e| format!("Export: {} ({})", e.object_name, e.class_name)).collect()
        })
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
    } else if filename.starts_with("WBP_") || filename.starts_with("wbp_") || lower.contains("/umg/") || lower.contains("/widget/") {
        "Widget".to_string()
    } else if filename.starts_with("M_") || filename.starts_with("m_") || filename.starts_with("MI_") || filename.starts_with("mi_") || lower.contains("/material/") {
        "Material".to_string()
    } else if lower.ends_with(".ubulk") || lower.contains("/texture/") || lower.contains("/ui/") || filename.starts_with("T_") || filename.starts_with("t_") {
        "Texture".to_string()
    } else if filename.starts_with("SK_") || filename.starts_with("sk_") || filename.starts_with("SM_") || filename.starts_with("sm_") || lower.contains("/model/") || lower.contains("/mesh/") || lower.contains("/skeletalmesh/") {
        "Mesh".to_string()
    } else if lower.ends_with(".wem") || lower.ends_with(".bnk") || lower.contains("/sound/") || lower.contains("/audio/") || filename.starts_with("AkAudio") || filename.starts_with("SB_") {
        "Audio".to_string()
    } else if filename.starts_with("AM_") || filename.starts_with("am_") || filename.starts_with("AS_") || filename.starts_with("as_") || lower.contains("/animation/") || lower.contains("/anim/") {
        "Animation".to_string()
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
                    if p.is_file() {
                        let is_target = p.extension().map_or(false, |ext| {
                            ext.eq_ignore_ascii_case("pak")
                                || ext.eq_ignore_ascii_case("utoc")
                                || ext.eq_ignore_ascii_case("ucas")
                        });
                        if is_target {
                            let filename = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                            // Ignore official base game paks if present
                            if !filename.starts_with("Pal-Windows") {
                                list.push(p);
                            }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GamePassPakNotice {
    pub mod_id: String,
    pub mod_name: String,
    pub pak_filename: String,
    pub pak_path: String,
    pub missing_containers: Vec<String>,
}

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
                        if let Some(ext) = p.extension() {
                            if ext.eq_ignore_ascii_case("pak") {
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
        }
    };

    check_dir(&mods_dir, &mut notices);
    check_dir(&logic_dir, &mut notices);

    (true, notices)
}
