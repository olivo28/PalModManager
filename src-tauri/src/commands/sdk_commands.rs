use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use tauri::State;
use crate::AppState;
use crate::usmap::{get_sdk_dir, invalidate_sdk_cache, get_or_load_sdk_index};
use crate::usmap::sync::find_bundled_resource;

const REMOTE_SDK_MANIFEST_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/sdk/manifest.json",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/sdk/manifest.json",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/sdk/manifest.json",
];

const REMOTE_SDK_ZIP_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/sdk/Palworld_SDK_24575825.zip",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/sdk/Palworld_SDK_24575825.zip",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/sdk/Palworld_SDK_24575825.zip",
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/sdk/Palworld_SDK.zip",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/sdk/Palworld_SDK.zip",
];

const DEFAULT_EMBEDDED_SDK_MANIFEST: &str = include_str!("../../../resources/sdk/manifest.json");

fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[tauri::command]
pub fn get_sdk_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();
    drop(data);

    let sdk_dir = get_sdk_dir(&program_path);
    let mut local_game_cxx_path: Option<String> = None;

    if !game_path.is_empty() {
        let game_cxx = PathBuf::from(&game_path)
            .join("Pal")
            .join("Binaries")
            .join("Win64")
            .join("ue4ss")
            .join("CXXHeaderDump");
        if game_cxx.exists() {
            local_game_cxx_path = Some(game_cxx.to_string_lossy().to_string());
        }
    }

    if let Some(sdk) = get_or_load_sdk_index(&program_path, &game_path) {
        Ok(serde_json::json!({
            "installed": true,
            "totalClasses": sdk.total_classes,
            "totalFunctions": sdk.total_functions,
            "source": sdk.source,
            "gameVersion": sdk.game_version,
            "buildId": sdk.build_id,
            "sha256": sdk.sha256,
            "totalHeaders": sdk.total_headers,
            "generatedAt": sdk.generated_at,
            "path": sdk_dir.to_string_lossy().to_string(),
            "localGameCxxFound": local_game_cxx_path.is_some(),
            "localGameCxxPath": local_game_cxx_path
        }))
    } else {
        Ok(serde_json::json!({
            "installed": false,
            "totalClasses": 0,
            "totalFunctions": 0,
            "source": "Not Installed",
            "gameVersion": "Unknown",
            "buildId": null,
            "sha256": null,
            "totalHeaders": null,
            "generatedAt": null,
            "path": sdk_dir.to_string_lossy().to_string(),
            "localGameCxxFound": local_game_cxx_path.is_some(),
            "localGameCxxPath": local_game_cxx_path
        }))
    }
}

#[tauri::command]
pub fn import_local_sdk(
    folder_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();
    drop(data);

    let src_dir = if let Some(p) = folder_path {
        if !p.trim().is_empty() {
            PathBuf::from(p)
        } else {
            find_local_game_cxx(&game_path).ok_or_else(|| "No local CXXHeaderDump folder found in game path.".to_string())?
        }
    } else {
        find_local_game_cxx(&game_path).ok_or_else(|| "No local CXXHeaderDump folder found in game path.".to_string())?
    };

    if !src_dir.exists() || !src_dir.is_dir() {
        return Err(format!("Source folder does not exist: {:?}", src_dir));
    }

    let target_dir = get_sdk_dir(&program_path);
    let _ = fs::create_dir_all(&target_dir);

    // Copy all .hpp / .h files from src_dir to target_dir
    let mut copied_count = 0;
    if let Ok(entries) = fs::read_dir(&src_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("hpp") || ext.eq_ignore_ascii_case("h") {
                        if let Some(file_name) = p.file_name() {
                            let dest = target_dir.join(file_name);
                            if fs::copy(&p, &dest).is_ok() {
                                copied_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // Write a local source manifest
    let local_manifest = serde_json::json!({
        "schema_version": "1.0.0",
        "latest_game_version": "local",
        "updated_at": chrono::Utc::now().to_rfc3339(),
        "sdk": [{
            "game_version": "local",
            "source": "Local CXXHeaderDump Import",
            "total_headers": copied_count,
            "imported_from": src_dir.to_string_lossy().to_string()
        }]
    });
    let _ = fs::write(target_dir.join("manifest.json"), serde_json::to_string_pretty(&local_manifest).unwrap_or_default());

    crate::logger::log(&format!("Imported {} local SDK headers from {:?}", copied_count, src_dir));

    invalidate_sdk_cache();
    get_sdk_status(state)
}

fn find_local_game_cxx(game_path: &str) -> Option<PathBuf> {
    if game_path.is_empty() {
        return None;
    }
    let game_cxx = PathBuf::from(game_path)
        .join("Pal")
        .join("Binaries")
        .join("Win64")
        .join("ue4ss")
        .join("CXXHeaderDump");
    if game_cxx.exists() {
        return Some(game_cxx);
    }
    None
}

#[tauri::command]
pub async fn sync_sdk_from_repo(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let (program_path, _game_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.settings.game_path.clone())
    };

    let target_dir = get_sdk_dir(&program_path);
    let _ = fs::create_dir_all(&target_dir);

    // 1. Fetch Remote Manifest or Fallback to Embedded Manifest
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("HTTP Client error: {}", e))?;

    let mut manifest_content = DEFAULT_EMBEDDED_SDK_MANIFEST.to_string();
    for url in REMOTE_SDK_MANIFEST_URLS {
        if let Ok(resp) = client.get(*url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    crate::logger::log(&format!("Fetched fresh SDK manifest from {}", url));
                    manifest_content = text;
                    break;
                }
            }
        }
    }

    let manifest_val: serde_json::Value = serde_json::from_str(&manifest_content)
        .unwrap_or_else(|_| serde_json::from_str(DEFAULT_EMBEDDED_SDK_MANIFEST).unwrap());

    let expected_hash = manifest_val.get("sdk")
        .and_then(|s| s.as_array())
        .and_then(|arr| arr.first())
        .and_then(|f| f.get("sha256"))
        .and_then(|h| h.as_str())
        .unwrap_or("523615b2776f4d935e382a0008cc603853a7990b4dc176ecdb986705c1cf848a");

    let mut download_bytes: Option<Vec<u8>> = None;
    let mut last_err = String::new();

    // 2. Check if bundled local resource exists
    let bundled_candidates = [
        "resources/sdk/Palworld_SDK_24575825.zip",
        "resources/sdk/Palworld_SDK.zip",
    ];
    for b_path in &bundled_candidates {
        if let Some(bundled_zip) = find_bundled_resource(b_path) {
            if let Ok(bytes) = fs::read(&bundled_zip) {
                let actual_hash = compute_sha256(&bytes);
                if actual_hash.eq_ignore_ascii_case(expected_hash) {
                    crate::logger::log(&format!("Loaded verified bundled SDK archive from {:?}", bundled_zip));
                    download_bytes = Some(bytes);
                    break;
                }
            }
        }
    }

    // 3. If not found locally, download from remote mirrors
    if download_bytes.is_none() {
        for url in REMOTE_SDK_ZIP_URLS {
            crate::logger::log(&format!("Attempting to download SDK archive from: {}", url));
            match client.get(*url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.bytes().await {
                            Ok(bytes) => {
                                let actual_hash = compute_sha256(&bytes);
                                if actual_hash.eq_ignore_ascii_case(expected_hash) {
                                    crate::logger::log(&format!("Successfully downloaded and verified SDK archive ({} bytes, SHA256: {})", bytes.len(), actual_hash));
                                    download_bytes = Some(bytes.to_vec());
                                    break;
                                } else {
                                    last_err = format!("SHA256 mismatch from {}: expected {}, got {}", url, expected_hash, actual_hash);
                                    crate::logger::log(&last_err);
                                }
                            }
                            Err(e) => {
                                last_err = format!("Failed to read SDK bytes from {}: {}", url, e);
                            }
                        }
                    } else {
                        last_err = format!("HTTP {} from {}", resp.status(), url);
                    }
                }
                Err(e) => {
                    last_err = format!("Connection error to {}: {}", url, e);
                }
            }
        }
    }

    let bytes = download_bytes.ok_or_else(|| {
        format!("Failed to obtain verified SDK package: {}", last_err)
    })?;

    // 4. Atomic Extraction via Temporary Staging Directory
    let temp_staging_dir = target_dir.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("sdk_staging_{}", uuid::Uuid::new_v4()));
    let _ = fs::create_dir_all(&temp_staging_dir);

    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("Invalid ZIP archive for SDK package: {}", e))?;

    let mut extracted_count = 0;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Zip entry error: {}", e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => temp_staging_dir.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            let _ = fs::create_dir_all(&outpath);
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    let _ = fs::create_dir_all(p);
                }
            }
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create output file {:?}: {}", outpath, e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to write SDK file {:?}: {}", outpath, e))?;
            extracted_count += 1;
        }
    }

    // Write verified manifest.json into staging directory
    let _ = fs::write(temp_staging_dir.join("manifest.json"), manifest_content);

    // Atomic promotion: move files from temp_staging_dir to target_dir
    if let Ok(entries) = fs::read_dir(&temp_staging_dir) {
        for entry in entries.flatten() {
            let src = entry.path();
            if let Some(file_name) = src.file_name() {
                let dest = target_dir.join(file_name);
                let _ = fs::rename(&src, &dest).or_else(|_| fs::copy(&src, &dest).map(|_| ()));
            }
        }
    }
    let _ = fs::remove_dir_all(&temp_staging_dir);

    crate::logger::log(&format!("Successfully synced and extracted {} verified SDK headers into {:?}", extracted_count, target_dir));

    invalidate_sdk_cache();
    get_sdk_status(state)
}

#[tauri::command]
pub fn purge_sdk_cache(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    let target_dir = get_sdk_dir(&program_path);
    if target_dir.exists() {
        let _ = fs::remove_dir_all(&target_dir);
    }

    invalidate_sdk_cache();
    get_sdk_status(state)
}
