use std::path::{Path, PathBuf};
use std::fs;
use sha2::{Sha256, Digest};
use super::models::{MappingsManifest, MappingEntry, UsmapStatus};
use super::build_detector::detect_installed_game_build;

const REMOTE_MANIFEST_URL: &str = "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/mappings/manifest.json";

pub fn find_bundled_resource(subpath: &str) -> Option<PathBuf> {
    // 1. Direct path from current working directory
    let p1 = PathBuf::from(subpath);
    if p1.exists() { return Some(p1); }

    // 2. Parent directory (e.g. when cwd is src-tauri during cargo run)
    let p2 = PathBuf::from("..").join(subpath);
    if p2.exists() { return Some(p2); }

    // 3. Relative to current executable directory (production bundle)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let p3 = parent.join(subpath);
            if p3.exists() { return Some(p3); }
            let p4 = parent.join("..").join(subpath);
            if p4.exists() { return Some(p4); }
            let p5 = parent.join("..").join("..").join(subpath);
            if p5.exists() { return Some(p5); }
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
    let target = dir.join("Palworld.usmap");
    if target.exists() {
        return target;
    }

    // Auto-seed from bundled repository resource if missing in LocalAppData
    if let Some(bundled) = find_bundled_resource("resources/mappings/Palworld.usmap") {
        let _ = fs::create_dir_all(&dir);
        let _ = fs::copy(&bundled, &target);
        if target.exists() {
            return target;
        }
        return bundled;
    }

    target
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

    // Auto-seed from bundled repository resource if missing in LocalAppData
    if let Some(bundled) = find_bundled_resource("resources/mappings/manifest.json") {
        if let Ok(content) = fs::read_to_string(&bundled) {
            if let Ok(manifest) = serde_json::from_str::<MappingsManifest>(&content) {
                let _ = fs::create_dir_all(&dir);
                let _ = fs::write(&local_file, &content);
                return Some(manifest);
            }
        }
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

    let remote_manifest = match client.get(REMOTE_MANIFEST_URL).send().await {
        Ok(res) if res.status().is_success() => {
            match res.json::<MappingsManifest>().await {
                Ok(m) => {
                    // Save downloaded manifest locally
                    if let Ok(json_str) = serde_json::to_string_pretty(&m) {
                        let _ = fs::write(mappings_dir.join("manifest.json"), json_str);
                    }
                    Some(m)
                }
                Err(e) => {
                    crate::logger::log(&format!("Failed to parse remote USMAP manifest: {}", e));
                    load_local_manifest(&program_path)
                }
            }
        }
        Ok(res) => {
            crate::logger::log(&format!("Remote USMAP manifest HTTP status: {}", res.status()));
            load_local_manifest(&program_path)
        }
        Err(e) => {
            crate::logger::log(&format!("Could not reach remote USMAP manifest: {}", e));
            load_local_manifest(&program_path)
        }
    };

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
            let res = client.get(&best_entry.usmap_url).send().await
                .map_err(|e| format!("Failed to download USMAP: {}", e))?;

            if !res.status().is_success() {
                return Err(format!("Download failed with HTTP status {}", res.status()));
            }

            let bytes = res.bytes().await
                .map_err(|e| format!("Failed to read USMAP response bytes: {}", e))?;

            // Verify SHA-256 of downloaded payload
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let downloaded_hash = format!("{:x}", hasher.finalize());

            if downloaded_hash.to_lowercase() != best_entry.sha256.to_lowercase() {
                return Err(format!("SHA-256 checksum mismatch! Expected: {}, Downloaded: {}", best_entry.sha256, downloaded_hash));
            }

            let temp_path = mappings_dir.join("Palworld.usmap.tmp");
            fs::write(&temp_path, &bytes)
                .map_err(|e| format!("Failed to write temporary USMAP file: {}", e))?;

            fs::rename(&temp_path, &target_usmap_path)
                .map_err(|e| format!("Failed to finalize USMAP file: {}", e))?;

            crate::logger::log(&format!("USMAP successfully synchronized! Path: {:?} ({} bytes)", target_usmap_path, bytes.len()));
        }
    } else {
        crate::logger::log("USMAP is already up to date and verified against checksum.");
    }

    Ok(get_mappings_status(&program_path, &game_path))
}
