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
    #[serde(default)]
    pub resolved_by_patch: Option<String>,
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

    let clean_target = entry_path.replace('\\', "/").trim_start_matches('/').to_string();
    let all_files = pak.files();

    if let Some(exact_key) = all_files.iter().find(|f| {
        let clean_f = f.replace('\\', "/").trim_start_matches('/').to_string();
        clean_f.eq_ignore_ascii_case(&clean_target)
    }) {
        return pak.get(exact_key, &mut file).map_err(|e| format!("Failed to extract {entry_path}: {e}"));
    }

    if let Some(suffix_key) = all_files.iter().find(|f| {
        let clean_f = f.replace('\\', "/").trim_start_matches('/').to_string();
        clean_f.ends_with(&clean_target) || clean_target.ends_with(&clean_f)
    }) {
        return pak.get(suffix_key, &mut file).map_err(|e| format!("Failed to extract {entry_path}: {e}"));
    }

    pak.get(entry_path, &mut file).map_err(|e| format!("Failed to extract {entry_path}: {e}"))
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
pub struct UAssetSchemaProperty {
    pub name: String,
    pub type_name: String,
    pub struct_type: Option<String>,
    pub enum_type: Option<String>,
    pub inner_type: Option<String>,
    pub array_dim: u8,
    pub index: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetSchemaResolvedInfo {
    pub matched_struct_name: String,
    pub super_type: Option<String>,
    pub properties: Vec<UAssetSchemaProperty>,
    pub total_properties: usize,
    pub game_version: String,
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
    pub resolved_schema: Option<UAssetSchemaResolvedInfo>,
}

pub fn resolve_usmap_schema(
    asset_name: &str,
    exports: &[UAssetExportItem],
    imports: &[UAssetImportItem],
    names_sample: &[String],
) -> Option<UAssetSchemaResolvedInfo> {
    let schema = crate::usmap::get_or_load_schema("")?;

    let mut candidate_names: Vec<String> = Vec::new();

    // 1. Clean asset name & stripped prefixes (WBP_, BP_, DT_, DA_, etc.)
    let clean_asset = asset_name
        .trim_end_matches(".uasset")
        .trim_end_matches(".uexp")
        .trim_end_matches(".ubulk")
        .to_string();
    candidate_names.push(clean_asset.clone());

    for prefix in &["WBP_", "BP_", "DT_", "DA_", "BPI_", "UI_", "W_", "Pal"] {
        if clean_asset.starts_with(prefix) {
            let stripped = clean_asset[prefix.len()..].to_string();
            if !stripped.is_empty() {
                candidate_names.push(stripped.clone());
                candidate_names.push(format!("Pal{}", stripped));
                candidate_names.push(format!("FPal{}", stripped));
                candidate_names.push(format!("UPal{}", stripped));
            }
        }
    }

    // 2. Export classes and object names
    for exp in exports {
        candidate_names.push(exp.class_name.clone());
        candidate_names.push(exp.object_name.clone());
        if exp.object_name.ends_with("_C") {
            let stripped = exp.object_name.trim_end_matches("_C").to_string();
            candidate_names.push(stripped.clone());
            if stripped.starts_with("WBP_") || stripped.starts_with("BP_") {
                candidate_names.push(stripped[stripped.find('_').unwrap() + 1..].to_string());
            }
        }
    }

    // 3. Import object & class names
    for imp in imports {
        candidate_names.push(imp.object_name.clone());
        candidate_names.push(imp.class_name.clone());
    }

    // 4. Sample names with Pal/Engine prefixes (full sample)
    for n in names_sample.iter() {
        if n.starts_with("Pal") || n.starts_with("FPal") || n.starts_with("UPal") || n.starts_with("APal") || n.starts_with("UserWidget") {
            candidate_names.push(n.clone());
        }
    }

    for candidate in candidate_names {
        if candidate.is_empty() || candidate == "Package" || candidate == "Object" || candidate == "Class" || candidate == "None" {
            continue;
        }
        if let Some(st) = schema.find_struct(&candidate) {
            let properties: Vec<UAssetSchemaProperty> = st.properties.iter().map(|p| {
                UAssetSchemaProperty {
                    name: p.name.clone(),
                    type_name: p.type_name.clone(),
                    struct_type: p.struct_type.clone(),
                    enum_type: p.enum_type.clone(),
                    inner_type: p.inner_type.clone(),
                    array_dim: p.array_dim,
                    index: p.index,
                }
            }).collect();

            let total_properties = properties.len();
            return Some(UAssetSchemaResolvedInfo {
                matched_struct_name: st.name.clone(),
                super_type: st.super_type.clone(),
                properties,
                total_properties,
                game_version: schema.game_version.clone(),
            });
        }
    }

    None
}

/// Deep inspection of an internal .uasset (and companion .uexp) from a .pak archive
pub fn inspect_uasset_deep(pak_path: &Path, uasset_internal_path: &str) -> Result<UAssetInspectionDetails, String> {
    use std::io::Cursor;
    use std::panic::{catch_unwind, AssertUnwindSafe};
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
    
    let asset_name = Path::new(&normalized_uasset_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| normalized_uasset_path.clone());
    let asset_type = classify_asset_type(&normalized_uasset_path);

    // Try parsing with unreal_asset across candidate Unreal Engine versions safely inside catch_unwind
    let engine_versions = [
        (EngineVersion::VER_UE5_1, "Unreal Engine 5.1 (GVAS/Zen)"),
        (EngineVersion::VER_UE5_0, "Unreal Engine 5.0"),
        (EngineVersion::VER_UE5_2, "Unreal Engine 5.2"),
        (EngineVersion::VER_UE4_27, "Unreal Engine 4.27"),
        (EngineVersion::UNKNOWN, "Unreal Engine (Generic)"),
    ];

    for (ver, ver_label) in engine_versions {
        let uasset_data = uasset_bytes.clone();
        let uexp_data = uexp_bytes.clone();

        let parse_result = catch_unwind(AssertUnwindSafe(|| {
            let uasset_cursor = Cursor::new(uasset_data);
            let uexp_cursor = uexp_data.map(Cursor::new);
            Asset::new(uasset_cursor, uexp_cursor, ver)
        }));

        if let Ok(Ok(asset)) = parse_result {
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
            let raw_names: Vec<String> = name_map.get_ref().get_name_map_index_list().to_vec();
            let names_sample: Vec<String> = raw_names.into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s.chars().any(|c| c.is_alphanumeric()))
                .collect();
            let name_count = names_sample.len();

            let summary = UAssetSummaryInfo {
                uasset_size_bytes,
                uexp_size_bytes,
                export_count: exports.len(),
                import_count: imports.len(),
                name_count,
                package_flags: asset.asset_data.package_flags.bits(),
            };

            let resolved_schema = resolve_usmap_schema(&asset_name, &exports, &imports, &names_sample);

            return Ok(UAssetInspectionDetails {
                asset_name,
                asset_path: uasset_internal_path.to_string(),
                asset_type,
                engine_version: ver_label.to_string(),
                summary,
                exports,
                imports,
                names_sample,
                resolved_schema,
            });
        }
    }

    // --- High-Reliability Binary Fallback Parser ---
    // If unreal_asset panics or fails on custom/cooked engine structures, extract string tokens,
    // export objects, and dependency paths directly from the raw binary stream.
    let mut combined_bytes = uasset_bytes.clone();
    if let Some(ref uexp) = uexp_bytes {
        combined_bytes.extend_from_slice(uexp);
    }

    let mut names_sample = Vec::new();
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    let text = String::from_utf8_lossy(&combined_bytes);
    for token in text.split(|c: char| c == '\0' || c < ' ' || c > '~') {
        let trimmed = token.trim();
        if trimmed.len() >= 3 && trimmed.len() <= 128 && !trimmed.contains('\n') && !trimmed.contains('\r') && trimmed.chars().any(|c| c.is_alphanumeric()) {
            if seen_names.insert(trimmed.to_string()) {
                names_sample.push(trimmed.to_string());

                if trimmed.starts_with("/Script/") || trimmed.starts_with("/Game/") || trimmed.starts_with("/Engine/") {
                    let parts: Vec<&str> = trimmed.split('.').collect();
                    let pkg = parts[0].to_string();
                    let obj = parts.get(1).unwrap_or(&"").to_string();
                    imports.push(UAssetImportItem {
                        object_name: if obj.is_empty() { pkg.clone() } else { obj },
                        class_name: "Package".to_string(),
                        class_package: pkg,
                    });
                } else if trimmed.ends_with("_C") || trimmed.starts_with("BP_") || trimmed.starts_with("Default__") || trimmed.contains("Function") {
                    exports.push(UAssetExportItem {
                        object_name: trimmed.to_string(),
                        class_name: if trimmed.ends_with("_C") { "BlueprintGeneratedClass".to_string() } else { "Object".to_string() },
                        outer_name: Some(asset_name.clone()),
                    });
                }
            }
        }
    }

    let summary = UAssetSummaryInfo {
        uasset_size_bytes,
        uexp_size_bytes,
        export_count: exports.len(),
        import_count: imports.len(),
        name_count: names_sample.len(),
        package_flags: 0,
    };

    let resolved_schema = resolve_usmap_schema(&asset_name, &exports, &imports, &names_sample);

    Ok(UAssetInspectionDetails {
        asset_name,
        asset_path: uasset_internal_path.to_string(),
        asset_type,
        engine_version: "Unreal Engine (Binary Stream Extractor)".to_string(),
        summary,
        exports,
        imports,
        names_sample,
        resolved_schema,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeprecatedSchemaNotice {
    pub mod_id: String,
    pub mod_name: String,
    pub asset_path: String,
    pub struct_name: String,
    pub message: String,
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



