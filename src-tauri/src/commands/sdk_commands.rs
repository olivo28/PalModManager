use std::fs;
use std::path::PathBuf;
use tauri::State;
use crate::AppState;
use crate::usmap::{get_sdk_dir, invalidate_sdk_cache, get_or_load_sdk_index};

const REMOTE_SDK_ZIP_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/sdk/Palworld_SDK.zip",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/sdk/Palworld_SDK.zip",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/sdk/Palworld_SDK.zip",
];

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

    if copied_count == 0 {
        return Err(format!("No .hpp header files found in {:?}", src_dir));
    }

    crate::logger::log(&format!("Imported {} SDK headers from {:?} to {:?}", copied_count, src_dir, target_dir));

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

    let mut download_bytes: Option<Vec<u8>> = None;
    let mut last_err = String::new();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("HTTP Client error: {}", e))?;

    for url in REMOTE_SDK_ZIP_URLS {
        crate::logger::log(&format!("Attempting to download SDK archive from: {}", url));
        match client.get(*url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.bytes().await {
                        Ok(bytes) => {
                            crate::logger::log(&format!("Successfully downloaded SDK archive ({} bytes)", bytes.len()));
                            download_bytes = Some(bytes.to_vec());
                            break;
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

    let bytes = download_bytes.ok_or_else(|| {
        format!("Failed to download SDK package from repository mirrors: {}", last_err)
    })?;

    // Decompress zip archive in memory and write files to target_dir
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("Invalid ZIP archive for SDK package: {}", e))?;

    let mut extracted_count = 0;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Zip entry error: {}", e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
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

    crate::logger::log(&format!("Extracted {} SDK files to {:?}", extracted_count, target_dir));

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
