use std::fs;
use std::path::Path;
use super::models::{MasterResourceManifest, MasterVersionEntry};
use super::sync::find_bundled_resource;

const DEFAULT_MASTER_MANIFEST: &str = include_str!("../../../resources/manifest.json");

/// Loads the Master Resource Manifest, preferring live files on disk over embedded compile-time data.
pub fn get_or_load_master_manifest(program_path: &str) -> Option<MasterResourceManifest> {
    // 1. Check live directory on disk (e.g. program_path/resources/manifest.json)
    if !program_path.is_empty() {
        let candidate = Path::new(program_path).join("resources").join("manifest.json");
        if candidate.is_file() {
            if let Ok(content) = fs::read_to_string(&candidate) {
                if let Ok(manifest) = serde_json::from_str::<MasterResourceManifest>(&content) {
                    return Some(manifest);
                }
            }
        }
    }

    // 2. Check bundled resource relative to current working directory or executable
    if let Some(bundled) = find_bundled_resource("resources/manifest.json") {
        if let Ok(content) = fs::read_to_string(&bundled) {
            if let Ok(manifest) = serde_json::from_str::<MasterResourceManifest>(&content) {
                return Some(manifest);
            }
        }
    }

    // 3. Fallback to compile-time embedded manifest
    serde_json::from_str::<MasterResourceManifest>(DEFAULT_MASTER_MANIFEST).ok()
}

/// Finds the most appropriate version entry in the master manifest matching the given Steam build ID,
/// or falls back to the entry marked as latest.
pub fn find_version_entry<'a>(
    manifest: &'a MasterResourceManifest,
    build_id: Option<&str>,
) -> Option<&'a MasterVersionEntry> {
    find_version_entry_refined(manifest, build_id, None)
}

/// Refined entry lookup matching both Steam build ID and optional UE4SS commit hash.
pub fn find_version_entry_refined<'a>(
    manifest: &'a MasterResourceManifest,
    build_id: Option<&str>,
    ue4ss_commit: Option<&str>,
) -> Option<&'a MasterVersionEntry> {
    if manifest.versions.is_empty() {
        return None;
    }

    // 1. If both build_id and ue4ss_commit are provided, look for exact compound match
    if let (Some(bid), Some(commit)) = (build_id, ue4ss_commit) {
        let clean_commit = commit.trim().to_lowercase();
        if let Some(found) = manifest.versions.iter().find(|v| {
            v.steam_build_id.as_deref() == Some(bid)
                && v.ue4ss_commit.as_deref().map(|c| c.to_lowercase() == clean_commit).unwrap_or(false)
        }) {
            return Some(found);
        }
    }

    // 2. Match exact Steam build ID (preferring is_latest == true for that build)
    if let Some(bid) = build_id {
        let mut matches = manifest.versions.iter().filter(|v| v.steam_build_id.as_deref() == Some(bid));
        if let Some(latest_for_build) = matches.clone().find(|v| v.is_latest) {
            return Some(latest_for_build);
        }
        if let Some(first_for_build) = matches.next() {
            return Some(first_for_build);
        }
    }

    // 3. Match entry marked as global is_latest
    if let Some(latest) = manifest.versions.iter().find(|v| v.is_latest) {
        return Some(latest);
    }

    // 4. Match entry matching manifest.latest_game_version
    if let Some(matching_ver) = manifest.versions.iter().find(|v| v.game_version == manifest.latest_game_version) {
        return Some(matching_ver);
    }

    // 5. Default to first available entry
    manifest.versions.first()
}

/// Resolves the human-readable game version (e.g. "v1.0.4") dynamically from the master manifest.
pub fn resolve_game_version(manifest: &MasterResourceManifest, build_id: Option<&str>) -> String {
    resolve_game_version_refined(manifest, build_id, None)
}

pub fn resolve_game_version_refined(
    manifest: &MasterResourceManifest,
    build_id: Option<&str>,
    ue4ss_commit: Option<&str>,
) -> String {
    if let Some(entry) = find_version_entry_refined(manifest, build_id, ue4ss_commit) {
        entry.game_version.clone()
    } else {
        manifest.latest_game_version.clone()
    }
}

/// Resolves the relative path of the .usmap file (e.g. "mappings/Palworld_25094871.usmap").
pub fn resolve_usmap_relative_path(
    manifest: &MasterResourceManifest,
    build_id: Option<&str>,
) -> Option<String> {
    resolve_usmap_relative_path_refined(manifest, build_id, None)
}

pub fn resolve_usmap_relative_path_refined(
    manifest: &MasterResourceManifest,
    build_id: Option<&str>,
    ue4ss_commit: Option<&str>,
) -> Option<String> {
    find_version_entry_refined(manifest, build_id, ue4ss_commit).and_then(|e| e.usmap.clone())
}

/// Resolves the associated PalSchema version from the master manifest for a given build.
pub fn resolve_palschema_version(
    manifest: &MasterResourceManifest,
    build_id: Option<&str>,
) -> Option<String> {
    find_version_entry(manifest, build_id).and_then(|e| e.palschema_version.clone())
}
