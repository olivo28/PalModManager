use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use super::types::UAssetLiveProperty;

/// Resolves authentic vanilla defaults for properties in a given asset by querying Pal-Windows.pak
/// or falling back to curated Palworld baseline schemas.
pub fn resolve_vanilla_property_defaults(
    pak_path: &Path,
    uasset_internal_path: &str,
) -> HashMap<String, String> {
    let mut defaults = get_baseline_curated_defaults(uasset_internal_path);

    // Try finding Pal-Windows.pak from pak_path or game installation
    if let Some(vanilla_pak) = locate_vanilla_pak(pak_path) {
        if let Ok(extracted) = extract_vanilla_asset_properties(&vanilla_pak, uasset_internal_path) {
            for (k, v) in extracted {
                defaults.insert(k, v);
            }
        }
    }

    defaults
}

/// Enriches live properties with authentic vanilla default values and recalculates `is_delta`
pub fn enrich_with_vanilla_defaults(
    pak_path: &Path,
    uasset_internal_path: &str,
    properties: &mut [UAssetLiveProperty],
) {
    if properties.is_empty() {
        return;
    }

    let vanilla_map = resolve_vanilla_property_defaults(pak_path, uasset_internal_path);

    for prop in properties.iter_mut() {
        let unqualified = prop.name.split(' ').next().unwrap_or(&prop.name);
        let vanilla_val = vanilla_map.get(&prop.name).or_else(|| vanilla_map.get(unqualified));
        if let Some(vanilla_val) = vanilla_val {
            prop.vanilla_default_display = Some(vanilla_val.clone());
            // Mark as delta ONLY if current value differs from the authentic vanilla baseline
            prop.is_delta = prop.raw_value_display != *vanilla_val;
        } else if let Some(ref d) = prop.vanilla_default_display {
            prop.is_delta = prop.raw_value_display != *d;
        } else {
            // If vanilla value could not be resolved, do NOT falsely mark it as a modified delta
            prop.is_delta = false;
        }
    }
}

/// Locates official Pal-Windows.pak by traversing up the pak path or checking Steam default paths
pub fn locate_vanilla_pak(pak_path: &Path) -> Option<PathBuf> {
    // 1. Check parent folders (~mods -> Paks, LogicMods -> Paks)
    let mut curr = pak_path.parent();
    for _ in 0..4 {
        if let Some(p) = curr {
            let candidate = p.join("Pal-Windows.pak");
            if candidate.is_file() {
                return Some(candidate);
            }
            let candidate_paks = p.join("Pal").join("Content").join("Paks").join("Pal-Windows.pak");
            if candidate_paks.is_file() {
                return Some(candidate_paks);
            }
            let in_paks = p.join("Paks").join("Pal-Windows.pak");
            if in_paks.is_file() {
                return Some(in_paks);
            }
            curr = p.parent();
        } else {
            break;
        }
    }

    // 2. Check standard Windows default Steam installation path
    let steam_default = PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Palworld\Pal\Content\Paks\Pal-Windows.pak");
    if steam_default.is_file() {
        return Some(steam_default);
    }

    None
}

/// Extracts raw bytes of the target asset from Pal-Windows.pak and decodes its properties
pub fn extract_vanilla_asset_properties(
    vanilla_pak_path: &Path,
    uasset_internal_path: &str,
) -> Result<HashMap<String, String>, String> {
    let mut file = File::open(vanilla_pak_path).map_err(|e| e.to_string())?;
    let pak = repak::PakBuilder::new().reader(&mut file).map_err(|e| e.to_string())?;

    let clean_uasset = uasset_internal_path.replace('\\', "/").trim_start_matches('/').to_string();
    let clean_uexp = if clean_uasset.ends_with(".uasset") {
        format!("{}.uexp", &clean_uasset[..clean_uasset.len() - 7])
    } else {
        format!("{}.uexp", clean_uasset)
    };

    // Find actual keys in pak (case-insensitive search)
    let all_files = pak.files();
    let uasset_key = all_files.iter().find(|f| f.replace('\\', "/").trim_start_matches('/').eq_ignore_ascii_case(&clean_uasset));
    let uexp_key = all_files.iter().find(|f| f.replace('\\', "/").trim_start_matches('/').eq_ignore_ascii_case(&clean_uexp));

    let uasset_key = match uasset_key {
        Some(k) => k,
        None => return Err(format!("Vanilla asset {} not found in Pal-Windows.pak", clean_uasset)),
    };

    let mut uasset_bytes = Vec::new();
    pak.read_file(uasset_key, &mut file, &mut uasset_bytes).map_err(|e| e.to_string())?;

    let mut uexp_bytes = Vec::new();
    if let Some(uk) = uexp_key {
        let _ = pak.read_file(uk, &mut file, &mut uexp_bytes);
    }

    // Parse vanilla asset using unreal_asset in protected catch_unwind
    let uexp_opt = if uexp_bytes.is_empty() { None } else { Some(uexp_bytes) };
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut parse_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let uasset_cursor = std::io::Cursor::new(uasset_bytes.clone());
        let uexp_cursor = uexp_opt.clone().map(std::io::Cursor::new);
        unreal_asset::Asset::new(uasset_cursor, uexp_cursor, unreal_asset::engine_version::EngineVersion::VER_UE5_1)
    }));

    // If parsing failed or panicked (e.g. unversioned u8 overflow on large classes), retry by stripping 0x2000
    if parse_res.is_err() || parse_res.as_ref().unwrap().is_err() {
        if let Some(stripped_uasset) = super::uasset::strip_pkg_unversioned_flag(&uasset_bytes) {
            parse_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let uasset_cursor = std::io::Cursor::new(stripped_uasset);
                let uexp_cursor = uexp_opt.map(std::io::Cursor::new);
                unreal_asset::Asset::new(uasset_cursor, uexp_cursor, unreal_asset::engine_version::EngineVersion::VER_UE5_1)
            }));
        }
    }

    std::panic::set_hook(prev_hook);

    let mut map = HashMap::new();
    if let Ok(Ok(asset)) = parse_res {
        // 1. Check normal exports
        let mut normal_props = super::property_extractor::extract_instantiated_properties(&asset.asset_data.exports);
        
        // 2. If empty, check unversioned CDO properties using USMAP
        if normal_props.is_empty() {
            let imports: Vec<super::types::UAssetImportItem> = asset.imports.iter().map(|i| {
                super::types::UAssetImportItem {
                    class_package: i.class_package.get_content(),
                    class_name: i.class_name.get_content(),
                    object_name: i.object_name.get_content(),
                }
            }).collect();
            let raw_names: Vec<String> = asset.get_name_map().get_ref().get_name_map_index_list().to_vec();
            let asset_name = Path::new(uasset_internal_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            normal_props = super::property_extractor::extract_unversioned_cdo_properties(
                &asset.asset_data.exports,
                &imports,
                &raw_names,
                &asset_name,
            );
        }

        for prop in normal_props {
            map.insert(prop.name.clone(), prop.raw_value_display.clone());
            let unqualified = prop.name.split(' ').next().unwrap_or(&prop.name);
            map.insert(unqualified.to_string(), prop.raw_value_display);
        }
    }

    Ok(map)
}

/// Fallback curated baseline values for known core Palworld Blueprints
fn get_baseline_curated_defaults(asset_internal_path: &str) -> HashMap<String, String> {
    let lower = asset_internal_path.to_lowercase();
    let mut map = HashMap::new();

    if lower.contains("itemchest") || lower.contains("chest") {
        map.insert("SlotNum".to_string(), "32".to_string());
    }
    if lower.contains("basecampmanager") || lower.contains("palbasecamp") {
        map.insert("WorkerCapacityNumDefault".to_string(), "15".to_string());
        map.insert("BaseCampWorkerMaxNum".to_string(), "20".to_string());
        map.insert("BaseCampLevelMax".to_string(), "20".to_string());
    }
    if lower.contains("gamesetting") {
        map.insert("PalEggDefaultHatchingSpeed".to_string(), "1.00".to_string());
    }

    map
}
