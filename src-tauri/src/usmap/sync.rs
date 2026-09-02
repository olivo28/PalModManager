use std::path::{Path, PathBuf};
use std::fs;
use sha2::{Sha256, Digest};
use super::models::{MappingsManifest, MappingEntry, UsmapStatus};
use super::build_detector::detect_installed_game_build;

const REMOTE_MANIFEST_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/mappings/manifest.json",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/mappings/manifest.json",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/mappings/manifest.json",
];
const DEFAULT_EMBEDDED_MANIFEST: &str = include_str!("../../../resources/mappings/manifest.json");

pub fn find_bundled_resource(subpath: &str) -> Option<PathBuf> {
    // 1. Direct path from current working directory
    let p1 = PathBuf::from(subpath);
    if p1.exists() { return Some(p1); }

    // 2. Parent directory traversal (cwd is src-tauri or subfolder)
    let p2 = PathBuf::from("..").join(subpath);
    if p2.exists() { return Some(p2); }

    let p2_b = PathBuf::from("..").join("..").join(subpath);
    if p2_b.exists() { return Some(p2_b); }

    // 3. Multi-parent traversal relative to current executable directory
    if let Ok(exe_path) = std::env::current_exe() {
        let mut cur = exe_path.parent();
        for _ in 0..6 {
            if let Some(parent) = cur {
                let candidate = parent.join(subpath);
                if candidate.exists() { return Some(candidate); }
                cur = parent.parent();
            } else {
                break;
            }
        }
    }

    None
}

pub fn get_mappings_dir(program_path: &str) -> PathBuf {
    let base = if !program_path.is_empty() {
        PathBuf::from(program_path)
    } else {
        #[cfg(target_os = "windows")]
        let p = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("PalModManager");
        #[cfg(not(target_os = "windows"))]
        let p = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".local").join("share"))
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("PalModManager");
        p
    };
    base.join("resources").join("mappings")
}

pub fn get_active_usmap_path(program_path: &str) -> PathBuf {
    let dir = get_mappings_dir(program_path);
    let local_manifest = load_local_manifest(program_path);

    // 1. Check if manifest has an active/best filename (e.g. Palworld_24575825.usmap)
    if let Some(ref manifest) = local_manifest {
        if let Some(best) = find_best_mapping(manifest, None, None) {
            let target = dir.join(&best.usmap_filename);
            if target.exists() {
                return target;
            }
        }
    }

    let default_target = dir.join("Palworld_24575825.usmap");
    if default_target.exists() {
        return default_target;
    }

    let legacy_target = dir.join("Palworld.usmap");
    if legacy_target.exists() {
        return legacy_target;
    }

    // Check if any custom-named .usmap file exists in the directory
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()) == Some("usmap".to_string()) {
                return p;
            }
        }
    }

    // Auto-seed from bundled repository resource if missing in LocalAppData
    let candidates = [
        "resources/mappings/Palworld_24575825.usmap",
        "resources/mappings/Palworld.usmap",
    ];
    for cand in &candidates {
        if let Some(bundled) = find_bundled_resource(cand) {
            let _ = fs::create_dir_all(&dir);
            let file_name = bundled.file_name().unwrap_or_default();
            let dest = dir.join(file_name);
            let _ = fs::copy(&bundled, &dest);
            if dest.exists() {
                return dest;
            }
            return bundled;
        }
    }

    default_target
}

pub fn load_local_manifest(program_path: &str) -> Option<MappingsManifest> {
    let dir = get_mappings_dir(program_path);
    let local_file = dir.join("manifest.json");
    if local_file.exists() {
        if let Ok(content) = fs::read_to_string(&local_file) {
            if let Ok(manifest) = serde_json::from_str::<MappingsManifest>(&content) {
                return Some(manifest);
            }
        }
    }

    // 1. Auto-seed from bundled repository resource if missing in LocalAppData
    if let Some(bundled) = find_bundled_resource("resources/mappings/manifest.json") {
        if let Ok(content) = fs::read_to_string(&bundled) {
            if let Ok(manifest) = serde_json::from_str::<MappingsManifest>(&content) {
                let _ = fs::create_dir_all(&dir);
                let _ = fs::write(&local_file, &content);
                return Some(manifest);
            }
        }
    }

    // 2. Embedded compile-time manifest fallback
    if let Ok(manifest) = serde_json::from_str::<MappingsManifest>(DEFAULT_EMBEDDED_MANIFEST) {
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(&local_file, DEFAULT_EMBEDDED_MANIFEST);
        return Some(manifest);
    }

    None
}

pub fn compute_sha256(path: &Path) -> Option<String> {
    if !path.exists() { return None; }
    let bytes = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    Some(format!("{:x}", result))
}

pub fn find_best_mapping<'a>(manifest: &'a MappingsManifest, build_id: Option<&str>, _game_version: Option<&str>) -> Option<&'a MappingEntry> {
    if manifest.mappings.is_empty() { return None; }

    // 1. Try matching exact Steam Build ID
    if let Some(bid) = build_id {
        if let Some(found) = manifest.mappings.iter().find(|m| m.steam_build_id.as_deref() == Some(bid)) {
            return Some(found);
        }
    }

    // 2. Try matching entry marked as latest
    if let Some(latest) = manifest.mappings.iter().find(|m| m.is_latest || m.game_version == manifest.latest_game_version) {
        return Some(latest);
    }

    // 3. Fallback to first available mapping
    manifest.mappings.first()
}

pub fn get_mappings_status(program_path: &str, game_path: &str) -> UsmapStatus {
    let build_info = detect_installed_game_build(Path::new(game_path));
    let local_manifest = load_local_manifest(program_path);
    let mappings_dir = get_mappings_dir(program_path);
    let local_usmap_path = get_active_usmap_path(program_path);
    let local_exists = local_usmap_path.exists();
    let local_size = if local_exists { fs::metadata(&local_usmap_path).map(|m| m.len()).unwrap_or(0) } else { 0 };
    let local_hash = if local_exists { compute_sha256(&local_usmap_path) } else { None };

    let active_mapping = local_manifest.as_ref().and_then(|m| {
        find_best_mapping(m, build_info.build_id.as_deref(), build_info.game_version.as_deref()).cloned()
    });

    let is_synced = match (&active_mapping, &local_hash) {
        (Some(entry), Some(hash)) => hash.to_lowercase() == entry.sha256.to_lowercase(),
        _ => local_exists && local_size > 1000,
    };

    UsmapStatus {
        installed_build: build_info,
        active_mapping,
        is_synced,
        local_usmap_exists: local_exists,
        local_file_size: local_size,
        local_sha256: local_hash,
        latest_remote_version: local_manifest.as_ref().map(|m| m.latest_game_version.clone()),
        error_message: None,
        mappings_path: mappings_dir.to_string_lossy().to_string(),
    }
}

pub async fn sync_mappings_async(program_path: String, game_path: String) -> Result<UsmapStatus, String> {
    let mappings_dir = get_mappings_dir(&program_path);
    let _ = fs::create_dir_all(&mappings_dir);

    // 1. Fetch remote manifest
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("PalModManager/1.7.0 (GitHub: olivo28/PalModManager)")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut remote_manifest: Option<MappingsManifest> = None;

    for url in REMOTE_MANIFEST_URLS {
        if let Ok(res) = client.get(*url).send().await {
            if res.status().is_success() {
                if let Ok(m) = res.json::<MappingsManifest>().await {
                    if let Ok(json_str) = serde_json::to_string_pretty(&m) {
                        let _ = fs::write(mappings_dir.join("manifest.json"), json_str);
                    }
                    remote_manifest = Some(m);
                    break;
                }
            }
        }
    }

    if remote_manifest.is_none() {
        remote_manifest = load_local_manifest(&program_path);
    }

    let manifest = remote_manifest.ok_or_else(|| "No USMAP mappings manifest available (remote unreachable and no local manifest found).".to_string())?;

    let build_info = detect_installed_game_build(Path::new(&game_path));
    let best_entry = find_best_mapping(&manifest, build_info.build_id.as_deref(), build_info.game_version.as_deref())
        .ok_or_else(|| "No compatible mapping entry found in manifest.".to_string())?
        .clone();

    let target_usmap_path = mappings_dir.join("Palworld.usmap");
    let current_hash = compute_sha256(&target_usmap_path);

    // Check if we need to download
    let needs_download = match current_hash {
        Some(ref h) => h.to_lowercase() != best_entry.sha256.to_lowercase(),
        None => true,
    };

    if needs_download {
        let mut synced_from_bundle = false;

        // Check if bundled resource already matches the target checksum
        if let Some(bundled) = find_bundled_resource("resources/mappings/Palworld.usmap") {
            if let Some(b_hash) = compute_sha256(&bundled) {
                if b_hash.to_lowercase() == best_entry.sha256.to_lowercase() {
                    let _ = fs::copy(&bundled, &target_usmap_path);
                    synced_from_bundle = true;
                    crate::logger::log(&format!("Seeded USMAP from local bundle to {:?}", target_usmap_path));
                }
            }
        }

        if !synced_from_bundle {
            crate::logger::log(&format!("Downloading updated USMAP from {} for game version {}", best_entry.usmap_url, best_entry.game_version));
            let res_result = client.get(&best_entry.usmap_url).send().await;
            
            match res_result {
                Ok(res) if res.status().is_success() => {
                    if let Ok(bytes) = res.bytes().await {
                        // Verify SHA-256 of downloaded payload
                        let mut hasher = Sha256::new();
                        hasher.update(&bytes);
                        let downloaded_hash = format!("{:x}", hasher.finalize());

                        if downloaded_hash.to_lowercase() == best_entry.sha256.to_lowercase() {
                            let temp_path = mappings_dir.join("Palworld.usmap.tmp");
                            if fs::write(&temp_path, &bytes).is_ok() {
                                let _ = fs::rename(&temp_path, &target_usmap_path);
                                crate::logger::log(&format!("USMAP successfully synchronized! Path: {:?} ({} bytes)", target_usmap_path, bytes.len()));
                            }
                        } else {
                            crate::logger::log(&format!("SHA-256 checksum mismatch! Expected: {}, Downloaded: {}", best_entry.sha256, downloaded_hash));
                        }
                    }
                }
                Ok(res) => {
                    crate::logger::log(&format!("Remote USMAP binary download returned HTTP status {}", res.status()));
                    if !target_usmap_path.exists() {
                        return Err(format!("Download failed with HTTP status {}", res.status()));
                    }
                }
                Err(e) => {
                    crate::logger::log(&format!("Could not reach remote USMAP binary: {}", e));
                    if !target_usmap_path.exists() {
                        return Err(format!("Failed to download USMAP: {}", e));
                    }
                }
            }
        }
    } else {
        crate::logger::log("USMAP is already up to date and verified against checksum.");
    }

    Ok(get_mappings_status(&program_path, &game_path))
}
