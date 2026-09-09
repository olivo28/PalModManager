use std::fs;
use std::path::Path;
use super::types::{
    UAssetExportItem, UAssetImportItem, UAssetInspectionDetails,
    UAssetSchemaProperty, UAssetSchemaResolvedInfo, UAssetSummaryInfo,
};
use super::reader::{classify_asset_type, extract_pak_entry};

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

/// Pre-validate standard Unreal Engine package binary header before calling `unreal_asset::Asset::new`.
/// Prevents astronomical memory allocation aborts (e.g. 47GB+) on corrupted, non-standard,
/// or cooked Zen / IoStore packages.
fn is_safe_ue5_uasset_header(bytes: &[u8]) -> bool {
    // 1. Must satisfy minimum package header size (Unreal Engine package summary is >= 128 bytes)
    if bytes.len() < 128 {
        return false;
    }

    // 2. Package File Tag: Must match Unreal PACKAGE_FILE_TAG (0x9E2A83C1)
    if &bytes[0..4] != &[0xC1, 0x83, 0x2A, 0x9E] {
        return false;
    }

    // 3. Legacy File Version (bytes 4..8): Must be negative in modern UE4/UE5 (-7 or -8)
    let legacy_ver = i32::from_le_bytes(match bytes[4..8].try_into() {
        Ok(b) => b,
        Err(_) => return false,
    });
    if legacy_ver > -1 || legacy_ver < -10 {
        return false;
    }

    // 4. FileVersionUE4 (bytes 12..16) & FileVersionUE5 (bytes 16..20)
    let ue4_ver = i32::from_le_bytes(match bytes[12..16].try_into() {
        Ok(b) => b,
        Err(_) => return false,
    });
    let ue5_ver = i32::from_le_bytes(match bytes[16..20].try_into() {
        Ok(b) => b,
        Err(_) => return false,
    });

    // In cooked/unversioned UE5 mods, FileVersionUE4 and FileVersionUE5 are typically 0
    let is_unversioned = ue4_ver == 0 && ue5_ver == 0;
    if !is_unversioned {
        if ue4_ver < 500 || ue4_ver > 1000 {
            return false;
        }
        if ue5_ver < 1000 || ue5_ver > 5000 {
            return false;
        }
    }

    // 6. Custom Versions Count (bytes 24..28)
    let custom_ver_count = i32::from_le_bytes(match bytes[24..28].try_into() {
        Ok(b) => b,
        Err(_) => return false,
    });
    if custom_ver_count < 0 || custom_ver_count > 256 {
        return false;
    }

    // Each custom version entry is 20 bytes (16 bytes GUID + 4 bytes version)
    let custom_ver_bytes = (custom_ver_count as usize) * 20;
    let offset_after_custom_ver = 28 + custom_ver_bytes;
    if offset_after_custom_ver + 4 > bytes.len() {
        return false;
    }

    // 7. Total Header Size
    let total_header_size = i32::from_le_bytes(match bytes[offset_after_custom_ver..offset_after_custom_ver + 4].try_into() {
        Ok(b) => b,
        Err(_) => return false,
    });
    if total_header_size <= 0 || (total_header_size as usize) > bytes.len() + 4096 {
        return false;
    }

    true
}

/// Strips PKG_UNVERSIONED_PROPERTIES (0x2000) from package_flags in the .uasset binary header.
/// In third-party unreal_asset 0.1.16, UnversionedHeaderFragment uses u8 for property indexing,
/// which panics with arithmetic overflow on structs with > 255 properties (like PalGameSetting with 680 properties).
/// Stripping this flag forces unreal_asset to safely load all exports as RawExport without panicking,
/// allowing our high-capacity 32-bit unversioned decoder to extract all properties seamlessly.
pub(crate) fn strip_pkg_unversioned_flag(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 128 {
        return None;
    }
    let custom_ver_count = i32::from_le_bytes(bytes.get(24..28)?.try_into().ok()?);
    if custom_ver_count < 0 || custom_ver_count > 256 {
        return None;
    }
    let header_size_offset = 28 + (custom_ver_count as usize) * 20;
    let mut cur = header_size_offset + 4;
    let folder_name_len = i32::from_le_bytes(bytes.get(cur..cur + 4)?.try_into().ok()?);
    cur += 4;
    if folder_name_len > 0 {
        cur += folder_name_len as usize;
    }
    if cur + 4 > bytes.len() {
        return None;
    }
    let flags = u32::from_le_bytes(bytes[cur..cur + 4].try_into().ok()?);
    if (flags & 0x2000) != 0 {
        let mut modified = bytes.to_vec();
        let stripped = flags & !0x2000;
        modified[cur..cur + 4].copy_from_slice(&stripped.to_le_bytes());
        Some(modified)
    } else {
        None
    }
}

/// Deep inspection of an internal .uasset (and companion .uexp) from a .pak archive or loose file
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

    // Check if companion .uexp exists
    let uexp_path = if normalized_uasset_path.ends_with(".uasset") {
        format!("{}.uexp", &normalized_uasset_path[..normalized_uasset_path.len() - 7])
    } else {
        format!("{}.uexp", normalized_uasset_path)
    };

    let (uasset_bytes, uexp_bytes) = if pak_path.is_file() && pak_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("uasset")) {
        let uasset_b = fs::read(pak_path).map_err(|e| format!("Cannot read .uasset file: {}", e))?;
        let uexp_p = pak_path.with_extension("uexp");
        let uexp_b = if uexp_p.exists() { fs::read(&uexp_p).ok() } else { None };
        (uasset_b, uexp_b)
    } else {
        let uasset_b = extract_pak_entry(pak_path, &normalized_uasset_path)?;
        let uexp_b = extract_pak_entry(pak_path, &uexp_path).ok();
        (uasset_b, uexp_b)
    };

    let uasset_size_bytes = uasset_bytes.len();
    let uexp_size_bytes = uexp_bytes.as_ref().map(|b| b.len());
    
    let asset_name = Path::new(&normalized_uasset_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| normalized_uasset_path.clone());
    let asset_type = classify_asset_type(&normalized_uasset_path);

    // Only attempt unreal_asset parsing if the header passes strict UE5 structural validation.
    // Otherwise, skip directly to the high-reliability binary fallback parser to prevent 47GB allocation panics.
    if is_safe_ue5_uasset_header(&uasset_bytes) {
        let engine_versions = [
            (EngineVersion::VER_UE5_1, "Unreal Engine 5.1 (Palworld)"),
            (EngineVersion::VER_UE5_2, "Unreal Engine 5.2"),
        ];

        for (ver, ver_label) in engine_versions {
            let uasset_data = uasset_bytes.clone();
            let uexp_data = uexp_bytes.clone();

            // Suppress default panic hook during probe to prevent unreal_asset unversioned
            // overflow bugs (e.g. u8 overflow on structs with > 255 properties) from dumping stderr noise.
            let prev_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));

            let mut parse_result = catch_unwind(AssertUnwindSafe(|| {
                let uasset_cursor = Cursor::new(uasset_data.clone());
                let uexp_cursor = uexp_data.clone().map(Cursor::new);
                Asset::new(uasset_cursor, uexp_cursor, ver)
            }));

            // If normal parsing failed or panicked (e.g. unreal_asset 0.1.16 unversioned u8 overflow panic
            // on structs with > 255 properties like BP_PalGameSetting), retry by stripping PKG_UNVERSIONED_PROPERTIES (0x2000).
            // This forces unreal_asset to safely load all exports as RawExport without panicking, allowing our
            // 32-bit high-capacity unversioned decoder to extract all properties seamlessly.
            if parse_result.is_err() || parse_result.as_ref().unwrap().is_err() {
                if let Some(stripped_uasset) = strip_pkg_unversioned_flag(&uasset_data) {
                    parse_result = catch_unwind(AssertUnwindSafe(|| {
                        let uasset_cursor = Cursor::new(stripped_uasset);
                        let uexp_cursor = uexp_data.map(Cursor::new);
                        Asset::new(uasset_cursor, uexp_cursor, ver)
                    }));
                }
            }

            std::panic::set_hook(prev_hook);

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
            let names_sample: Vec<String> = raw_names.iter()
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

            let mut instantiated_properties = super::property_extractor::extract_instantiated_properties(&asset.asset_data.exports);
            if instantiated_properties.is_empty() {
                instantiated_properties = super::property_extractor::extract_unversioned_cdo_properties(
                    &asset.asset_data.exports,
                    &imports,
                    &raw_names,
                    &asset_name,
                );
            }
            super::vanilla_extractor::enrich_with_vanilla_defaults(
                pak_path,
                uasset_internal_path,
                &mut instantiated_properties,
            );
            let datatable_grid = super::property_extractor::extract_datatable_grid(&asset.asset_data.exports, &raw_names, &asset_name);
            let main_class = exports.first().map(|e| e.class_name.as_str()).unwrap_or(&asset_name);
            let class_hierarchy = super::hierarchy_resolver::resolve_class_hierarchy(main_class);
            let vanilla_verification = super::hierarchy_resolver::verify_vanilla_references(&imports);
            let has_original_backup = super::hierarchy_resolver::check_pak_backup_exists(pak_path);

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
                texture_preview: None,
                instantiated_properties,
                datatable_grid,
                class_hierarchy,
                vanilla_verification,
                has_original_backup,
            });
        }
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
    let vanilla_verification = super::hierarchy_resolver::verify_vanilla_references(&imports);
    let has_original_backup = super::hierarchy_resolver::check_pak_backup_exists(pak_path);

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
        texture_preview: None,
        instantiated_properties: Vec::new(),
        datatable_grid: None,
        class_hierarchy: Vec::new(),
        vanilla_verification,
        has_original_backup,
    })
}

/// Parses an internal .uasset (and companion .uexp if present) from a .pak and returns exported rows/objects
pub fn inspect_uasset_from_pak(pak_path: &Path, uasset_internal_path: &str) -> Result<Vec<String>, String> {
    inspect_uasset_deep(pak_path, uasset_internal_path)
        .map(|res| {
            res.exports.into_iter().map(|e| format!("Export: {} ({})", e.object_name, e.class_name)).collect()
        })
}
