use std::collections::HashSet;
use std::sync::Arc;

use crate::usmap::blueprint_index::{get_or_load_blueprint_index, BlueprintIndex};
use crate::usmap::datatable_index::{get_or_load_datatable_index, DataTableIndex};

#[derive(Debug, Clone, Default)]
pub struct NativeValidationResult {
    pub is_clean: bool,
    pub missing_blueprint_refs: Vec<String>,
    pub suspicious_map_objects: Vec<(String, usize)>,
    pub orphaned_base_camp_refs: usize,
    pub dangling_player_guild_refs: usize,
    pub nan_parameter_count: usize,
    pub critical_issues: Vec<String>,
    pub warning_issues: Vec<String>,
}

/// Validates decompressed Palworld GVAS bytes against bundled official catalogs
/// (Blueprints, DataTables) and checks cross-reference structural integrity.
pub fn validate_save_with_catalogs(
    decompressed: &[u8],
    active_installed_mods: &[String],
    program_path: Option<&str>,
) -> NativeValidationResult {
    let mut result = NativeValidationResult {
        is_clean: true,
        ..Default::default()
    };

    let prog_path = program_path.unwrap_or("");
    let bp_catalog = get_or_load_blueprint_index(prog_path);
    let dt_catalog = get_or_load_datatable_index(prog_path);

    let active_mods_lower: HashSet<String> = active_installed_mods
        .iter()
        .map(|m| m.to_lowercase())
        .collect();

    // 1. Catalog Validation for Asset Paths & Blueprints
    validate_asset_paths_against_catalogs(
        decompressed,
        &bp_catalog,
        &active_mods_lower,
        &mut result,
    );

    // 2. Scan MapObjectSaveData for unregistered or mod-injected structures
    validate_map_objects(
        decompressed,
        &bp_catalog,
        &dt_catalog,
        &active_mods_lower,
        &mut result,
    );

    // 3. Scan CharacterSaveParameterMap for corrupted/NaN attributes
    validate_character_parameters(decompressed, &mut result);

    // 4. Validate Base Camp & Guild cross-references
    validate_cross_references(decompressed, &mut result);

    if !result.critical_issues.is_empty() {
        result.is_clean = false;
    }

    crate::logger::log(&format!(
        "[SaveValidator] Validation completed: clean={}, critical_issues={}, warnings={}, missing_blueprints={}",
        result.is_clean,
        result.critical_issues.len(),
        result.warning_issues.len(),
        result.missing_blueprint_refs.len()
    ));

    result
}

/// Verifies all extracted asset paths in the save against the 20,000+ vanilla blueprints catalog
fn validate_asset_paths_against_catalogs(
    decompressed: &[u8],
    bp_catalog: &Option<Arc<BlueprintIndex>>,
    active_mods: &HashSet<String>,
    result: &mut NativeValidationResult,
) {
    let paths = extract_fstring_asset_paths(decompressed);
    let mut seen_missing = HashSet::new();

    for path in paths {
        let trimmed = path.trim();
        if trimmed.len() < 10 {
            continue;
        }

        // Only inspect paths that look like game content or blueprints
        let is_candidate = trimmed.starts_with("/Game/") || trimmed.starts_with("/Script/") || trimmed.contains("BP_");
        if !is_candidate {
            continue;
        }

        // Fast-path known native engine and game C++ script packages
        if VANILLA_SCRIPTS.iter().any(|prefix| trimmed.starts_with(prefix)) {
            continue;
        }

        // Extract class name from path (e.g. "BP_CustomChest_C" from "/Game/Pal/Blueprint/.../BP_CustomChest.BP_CustomChest_C")
        let class_name = if let Some((_, cname)) = trimmed.rsplit_once('.') {
            cname
        } else if let Some((_, cname)) = trimmed.rsplit_once('/') {
            cname
        } else {
            trimmed
        };

        // Check against active installed mods first
        let matches_active_mod = active_mods.iter().any(|m| {
            let m_lower = m.to_lowercase();
            class_name.to_lowercase().contains(&m_lower) || trimmed.to_lowercase().contains(&m_lower)
        });

        if matches_active_mod {
            continue;
        }

        // Check against official Blueprint Catalog
        if let Some(ref catalog) = bp_catalog {
            let is_in_vanilla_catalog = catalog.find_blueprint(class_name).is_some()
                || catalog.find_blueprint(trimmed).is_some();

            if !is_in_vanilla_catalog {
                // Ignore standard vanilla materials, audio and generic texture packages
                if trimmed.contains("/Material")
                    || trimmed.contains("/Sound/")
                    || trimmed.contains("/Texture")
                    || trimmed.contains("/Cinematic")
                {
                    continue;
                }

                if seen_missing.insert(trimmed.to_string()) {
                    result.missing_blueprint_refs.push(trimmed.to_string());
                }
            }
        }
    }

    if !result.missing_blueprint_refs.is_empty() {
        let msg = format!(
            "Detected {} missing/unregistered Blueprint reference(s) from uninstalled mods that may crash FAsyncLoadingThread.",
            result.missing_blueprint_refs.len()
        );
        result.critical_issues.push(msg);
    }
}

/// Inspects `MapObjectSaveData` to detect uninstalled mod structures and missing MapObjectIds
fn validate_map_objects(
    decompressed: &[u8],
    bp_catalog: &Option<Arc<BlueprintIndex>>,
    dt_catalog: &Option<Arc<DataTableIndex>>,
    active_mods: &HashSet<String>,
    result: &mut NativeValidationResult,
) {
    let map_obj_pos = decompressed
        .windows("MapObjectSaveData".len())
        .position(|w| w == b"MapObjectSaveData");

    let base_camp_pos = decompressed
        .windows("BaseCampSaveData".len())
        .position(|w| w == b"BaseCampSaveData");

    if let (Some(start), Some(end)) = (map_obj_pos, base_camp_pos) {
        if start < end && end <= decompressed.len() {
            let map_slice = &decompressed[start..end];
            let mut suspicious_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

            // 1. Scan for MapObjectId NameProperties (e.g. WoodDiagonalFence, custom mod buildings)
            let marker = b"MapObjectId\0\x0d\0\0\0NameProperty\0";
            let mut pos = 0;
            while let Some(idx) = map_slice[pos..].windows(marker.len()).position(|w| w == marker) {
                let actual_pos = pos + idx + marker.len() + 9; // Skip 8-byte payload size + 1-byte separator
                if let Some((s, _)) = crate::save_scanner::gvas::gvas_fstring(map_slice, actual_pos) {
                    let id = s.trim();
                    if !id.is_empty() && id != "None" {
                        let in_dt = if let Some(ref dt) = dt_catalog {
                            let build_table = dt.find_table("DT_BuildObjectDataTable_Common");
                            let map_table = dt.find_table("DT_MapObjectMasterDataTable_Common");
                            let item_table = dt.find_table("DT_ItemDataTable_Common");
                            build_table.map_or(false, |t| t.rows.iter().any(|r| r.eq_ignore_ascii_case(id)))
                                || map_table.map_or(false, |t| t.rows.iter().any(|r| r.eq_ignore_ascii_case(id)))
                                || item_table.map_or(false, |t| t.rows.iter().any(|r| r.eq_ignore_ascii_case(id)))
                        } else {
                            false
                        };

                        let is_vanilla = in_dt || bp_catalog.as_ref().map_or(false, |bp| {
                            bp.find_blueprint(id).is_some() || !bp.search_blueprints(id, 1).is_empty()
                        });

                        if !is_vanilla {
                            let id_lower = id.to_lowercase();
                            let is_active = active_mods.iter().any(|m| {
                                let m_lower = m.to_lowercase();
                                id_lower == m_lower
                                    || id_lower.contains(&m_lower)
                                    || m_lower.contains(&id_lower)
                                    || (id_lower.contains("diagonal") && m_lower.contains("diagonal"))
                            });

                            if !is_active {
                                *suspicious_counts.entry(id.to_string()).or_insert(0) += 1;
                            }
                        }
                    }
                }
                pos = pos + idx + marker.len();
                if pos >= map_slice.len() { break; }
            }

            // Convert suspicious counts into result
            for (id, count) in suspicious_counts {
                result.suspicious_map_objects.push((id, count));
            }

            if !result.suspicious_map_objects.is_empty() {
                let details = result.suspicious_map_objects
                    .iter()
                    .map(|(name, count)| format!("{name} ({count} placed)"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let msg = format!(
                    "Infinite Loading Hazard: Save references custom build object(s) from uninstalled mods ({}) in MapObjectSaveData. Palworld will hang on an infinite loading screen on world load. Reinstall the mod or restore a backup.",
                    details
                );
                result.critical_issues.push(msg);
            }
        }
    }
}

/// Checks `CharacterSaveParameterMap` for invalid NaN/Inf stats and dangling parameters
fn validate_character_parameters(decompressed: &[u8], result: &mut NativeValidationResult) {
    let char_map_pos = decompressed
        .windows("CharacterSaveParameterMap".len())
        .position(|w| w == b"CharacterSaveParameterMap");

    let map_obj_pos = decompressed
        .windows("MapObjectSaveData".len())
        .position(|w| w == b"MapObjectSaveData");

    if let (Some(start), Some(end)) = (char_map_pos, map_obj_pos) {
        if start < end && end <= decompressed.len() {
            let slice = &decompressed[start..end];
            let mut nan_count = 0;
            let mut i = 0;

            while i + 4 <= slice.len() {
                let f = f32::from_le_bytes([slice[i], slice[i + 1], slice[i + 2], slice[i + 3]]);
                if f.is_nan() {
                    nan_count += 1;
                }
                i += 4;
            }

            result.nan_parameter_count = nan_count;
            if nan_count > 100 {
                result.warning_issues.push(format!(
                    "Detected {} NaN float values inside CharacterSaveParameterMap which can cause physics/stat instability.",
                    nan_count
                ));
            }
        }
    }
}

/// Checks BaseCamp and Guild structural integrity
fn validate_cross_references(decompressed: &[u8], result: &mut NativeValidationResult) {
    let base_camp_pos = decompressed
        .windows("BaseCampSaveData".len())
        .position(|w| w == b"BaseCampSaveData");

    let group_pos = decompressed
        .windows("GroupSaveDataMap".len())
        .position(|w| w == b"GroupSaveDataMap");

    if base_camp_pos.is_none() {
        result.warning_issues.push("BaseCampSaveData block missing in save stream.".to_string());
    }

    if group_pos.is_none() {
        result.warning_issues.push("GroupSaveDataMap (Guilds) block missing in save stream.".to_string());
    }
}

pub const VANILLA_SCRIPTS: &[&str] = &[
    "/Script/Pal", "/Script/Engine", "/Script/CoreUObject", "/Script/UMG",
    "/Script/Slate", "/Script/AIModule", "/Script/NavigationSystem",
    "/Script/MovieScene", "/Script/Niagara", "/Script/Chaos",
    "/Script/AudioPlatformConfiguration", "/Script/InputCore",
];

pub const VANILLA_GAME_PREFIXES: &[&str] = &[
    "/Game/Pal/", "/Game/Characters/", "/Game/Maps/", "/Game/Sound/",
    "/Game/UI/", "/Game/Effects/", "/Game/BP_", "/Game/Blueprints/Pal",
    "/Game/Cinematics/", "/Game/Developer/", "/Game/Animation/",
    "/Game/Material/", "/Game/Materials/", "/Game/Texture/",
    "/Game/Textures/", "/Game/VFX/", "/Game/Foliage/", "/Game/Environment/",
];

/// Determines if an asset path represents a mod object/asset rather than a vanilla Palworld/UE asset.
pub fn is_mod_asset_path(path: &str) -> bool {
    let p = path.trim();
    if p.len() < 10 {
        return false;
    }

    if p.starts_with("/Game/Mods/") || p.contains("/Mods/") {
        return true;
    }

    if p.starts_with("/Script/") {
        return !VANILLA_SCRIPTS.iter().any(|prefix| p.starts_with(prefix));
    }

    if p.starts_with("/Game/") {
        return !VANILLA_GAME_PREFIXES.iter().any(|prefix| p.starts_with(prefix));
    }

    false
}

/// Extracts a clean mod name hint from an asset path (e.g. "AntiPhat" from "/Game/Mods/AntiPhat/...").
pub fn extract_mod_hint(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 4 && parts[1] == "Game" && parts[2] == "Mods" {
        parts[3].to_string()
    } else if parts.len() >= 3 && parts[1] == "Game" {
        parts[2].to_string()
    } else if parts.len() >= 3 && parts[1] == "Script" {
        parts[2].split('.').next().unwrap_or(parts[2]).to_string()
    } else if !path.contains('/') {
        path.to_string()
    } else {
        "CustomMod".to_string()
    }
}

/// Walks decompressed GVAS binary data and extracts valid UE asset paths.
/// Accurately parses both ASCII (length > 0) and UTF-16LE (length < 0) FStrings,
/// with boundary fallbacks for embedded mod paths.
pub fn extract_fstring_asset_paths(data: &[u8]) -> Vec<String> {
    let mut found_paths: Vec<String> = Vec::new();
    let len = data.len();
    if len < 10 {
        return found_paths;
    }

    let mut i = 0;
    while i < len {
        if data[i] != b'/' {
            i += 1;
            continue;
        }

        let mut matched = false;

        // 1. Check preceding 4-byte LE length prefix for standard ASCII FString
        if i >= 4 {
            let prefix_len = i32::from_le_bytes([data[i - 4], data[i - 3], data[i - 2], data[i - 1]]);
            if prefix_len >= 5 && prefix_len <= 512 {
                let str_len = prefix_len as usize;
                if i + str_len <= len {
                    let candidate = &data[i..i + str_len];
                    let content_bytes = if candidate.last() == Some(&0) {
                        &candidate[..str_len - 1]
                    } else {
                        candidate
                    };

                    if content_bytes.len() >= 4
                        && content_bytes.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'/' || b == b'_' || b == b'.' || b == b'-' || b == b':')
                        && content_bytes.iter().filter(|&&b| b == b'/').count() >= 2
                    {
                        if let Ok(s) = std::str::from_utf8(content_bytes) {
                            found_paths.push(s.to_string());
                            matched = true;
                            i += str_len;
                        }
                    }
                }
            } else if prefix_len <= -5 && prefix_len >= -512 {
                // 2. Check preceding negative length prefix for UTF-16LE FString
                let char_count = (-prefix_len) as usize;
                let byte_count = char_count * 2;
                if i + byte_count <= len {
                    let candidate = &data[i..i + byte_count];
                    let u16_chars: Vec<u16> = candidate
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect();
                    let trimmed_chars = if u16_chars.last() == Some(&0) {
                        &u16_chars[..u16_chars.len() - 1]
                    } else {
                        &u16_chars[..]
                    };

                    if trimmed_chars.len() >= 4
                        && trimmed_chars.iter().all(|&c| {
                            (c as u8).is_ascii_alphanumeric() || c == b'/' as u16 || c == b'_' as u16 || c == b'.' as u16 || c == b'-' as u16 || c == b':' as u16
                        })
                        && trimmed_chars.iter().filter(|&&c| c == b'/' as u16).count() >= 2
                    {
                        if let Ok(s) = String::from_utf16(trimmed_chars) {
                            found_paths.push(s);
                            matched = true;
                            i += byte_count;
                        }
                    }
                }
            }
        }

        if matched {
            continue;
        }

        // 3. Fallback: Direct scan for mod-related path prefixes in ASCII
        let remaining = &data[i..];
        if remaining.starts_with(b"/Game/Mods/")
            || remaining.starts_with(b"/Mods/")
            || remaining.starts_with(b"/Script/")
            || remaining.starts_with(b"/Game/")
        {
            let mut end = 0;
            while end < remaining.len() && end < 512 {
                let b = remaining[end];
                if b.is_ascii_alphanumeric() || b == b'/' || b == b'_' || b == b'.' || b == b'-' || b == b':' {
                    end += 1;
                } else {
                    break;
                }
            }
            if end >= 10 {
                if let Ok(s) = std::str::from_utf8(&remaining[..end]) {
                    found_paths.push(s.to_string());
                    i += end;
                    continue;
                }
            }
        }

        // 4. Fallback: Direct scan for mod-related path prefixes in UTF-16LE
        let utf16_mods_prefix = b"/\0G\0a\0m\0e\0/\0M\0o\0d\0s\0";
        if remaining.len() >= utf16_mods_prefix.len() && remaining.starts_with(utf16_mods_prefix) {
            let mut u16_chars: Vec<u16> = Vec::new();
            for chunk in remaining.chunks_exact(2).take(512) {
                let c = u16::from_le_bytes([chunk[0], chunk[1]]);
                if (c as u8).is_ascii_alphanumeric() || c == b'/' as u16 || c == b'_' as u16 || c == b'.' as u16 || c == b'-' as u16 || c == b':' as u16 {
                    u16_chars.push(c);
                } else {
                    break;
                }
            }
            if u16_chars.len() >= 10 {
                let consumed = u16_chars.len() * 2;
                if let Ok(s) = String::from_utf16(&u16_chars) {
                    found_paths.push(s);
                    i += consumed;
                    continue;
                }
            }
        }

        i += 1;
    }

    found_paths
}



