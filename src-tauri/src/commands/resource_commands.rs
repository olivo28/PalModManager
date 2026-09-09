use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::State;
use crate::state::AppState;
use crate::usmap::{
    detect_installed_game_build, get_or_load_master_manifest,
    find_version_entry, MasterResourceManifest,
};
use crate::usmap::sync::find_bundled_resource;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceItemStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub is_available: bool,
    pub is_synced: bool,
    pub file_size_bytes: u64,
    pub sha256: Option<String>,
    pub local_path: Option<String>,
    pub total_items: Option<usize>,
    pub game_version: String,
    pub steam_build_id: String,
    pub ue4ss_commit: Option<String>,
    pub has_local_game_dump: bool,
    pub local_game_dump_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterResourcesStatus {
    pub detected_steam_build_id: Option<String>,
    pub detected_game_version: String,
    pub latest_game_version: String,
    pub latest_steam_build_id: String,
    pub is_game_installed: bool,
    pub resources: Vec<ResourceItemStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceActionResult {
    pub success: bool,
    pub message: String,
    pub target: String,
    pub file_size_bytes: Option<u64>,
    pub sha256: Option<String>,
    pub total_files_extracted: Option<usize>,
}

fn compute_sha256_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

fn compute_sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Resolves the candidate base directory for storing resources
fn resolve_resource_dir(program_path: &str, subfolder: &str) -> PathBuf {
    if !program_path.is_empty() {
        let candidate = Path::new(program_path).join("resources").join(subfolder);
        if candidate.exists() || Path::new(program_path).join("resources").exists() {
            return candidate;
        }
    }
    PathBuf::from("resources").join(subfolder)
}

/// Discovers an existing file on disk across program storage and bundled roots
fn find_resource_file(program_path: &str, subfolder: &str, filename: &str) -> Option<PathBuf> {
    if filename.is_empty() {
        return None;
    }

    // 1. Program path storage
    if !program_path.is_empty() {
        let p = Path::new(program_path).join("resources").join(subfolder).join(filename);
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. Relative working directory
    let local = PathBuf::from("resources").join(subfolder).join(filename);
    if local.is_file() {
        return Some(local);
    }

    // 3. Bundled resource lookup
    let rel_str = format!("resources/{}/{}", subfolder, filename);
    if let Some(bundled) = find_bundled_resource(&rel_str) {
        if bundled.is_file() {
            return Some(bundled);
        }
    }

    None
}

#[tauri::command]
pub fn get_master_resources_status(state: State<'_, AppState>) -> Result<MasterResourcesStatus, String> {
    let (program_path, game_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.settings.game_path.clone())
    };

    let installed_build = detect_installed_game_build(Path::new(&game_path));
    let detected_build_id = installed_build.build_id.clone();
    let is_game_installed = !game_path.is_empty() && Path::new(&game_path).exists();

    let manifest = get_or_load_master_manifest(&program_path)
        .unwrap_or_else(|| MasterResourceManifest {
            schema_version: "1.0.0".to_string(),
            latest_game_version: "v1.0.4".to_string(),
            latest_steam_build_id: "25094871".to_string(),
            updated_at: String::new(),
            versions: Vec::new(),
        });

    let active_entry = find_version_entry(&manifest, detected_build_id.as_deref());
    let default_build_id = active_entry
        .and_then(|e| e.steam_build_id.clone())
        .unwrap_or_else(|| manifest.latest_steam_build_id.clone());
    let default_game_ver = active_entry
        .map(|e| e.game_version.clone())
        .unwrap_or_else(|| manifest.latest_game_version.clone());
    let default_commit = active_entry.and_then(|e| e.ue4ss_commit.clone());

    // Definitions of the 7 engine resources
    let targets = [
        ("mappings", "Unreal Mappings (.usmap)", "GVAS Save Doctor deserialization & cooked asset decoding",
         active_entry.and_then(|e| e.usmap.clone()).unwrap_or_else(|| format!("mappings/Palworld_{}.usmap", default_build_id))),
        ("sdk", "CXX Header SDK (Pal.hpp)", "C++ engine & game headers for Hook Diagnostic Engine",
         active_entry.and_then(|e| e.sdk.clone()).unwrap_or_else(|| format!("sdk/Palworld_SDK_{}.zip", default_build_id))),
        ("jmap", "JSON Property Mappings (.jmap)", "Property AST, enums & byte offsets dictionary",
         active_entry.and_then(|e| e.jmap.clone()).unwrap_or_else(|| format!("jmap/Palworld_{}.jmap.zip", default_build_id))),
        ("lua_types", "Lua EmmyLua Types (shared/types)", "EmmyLua type definitions & signatures for Monaco Editor",
         active_entry.and_then(|e| e.lua_types.clone()).unwrap_or_else(|| format!("lua_types/Palworld_LuaTypes_{}.zip", default_build_id))),
        ("uht", "UHT Compatible Headers", "Unreal Header Tool headers with RPCs & reflection for C++ mods",
         active_entry.and_then(|e| e.uht.clone()).unwrap_or_else(|| format!("uht/Palworld_UHT_SDK_{}.zip", default_build_id))),
        ("bp_sdk", "Blueprint SDK Dummy Assets", "UE4SS dummy assets enabling Palworld modding in Unreal Engine 5.1.1",
         active_entry.and_then(|e| e.bp_sdk.clone()).unwrap_or_else(|| format!("bp_sdk/Palworld_BP_SDK_{}.zip", default_build_id))),
        ("schemas", "PalSchema Specifications", "Formal JSON Schema (Draft-07) specifications for JSON modding",
         format!("schemas/palschema_schemas_{}.zip", active_entry.and_then(|e| e.palschema_version.clone()).unwrap_or_else(|| "0.6.7".to_string()))),
    ];

    let mut resources: Vec<ResourceItemStatus> = Vec::new();

    for (id, name, desc, rel_path) in targets {
        let filename = Path::new(&rel_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| rel_path.clone());

        let local_file = find_resource_file(&program_path, id, &filename);
        let exists = local_file.is_some();
        let file_size = local_file.as_ref().and_then(|p| fs::metadata(p).ok()).map(|m| m.len()).unwrap_or(0);
        let sha256 = local_file.as_ref().and_then(|p| compute_sha256_file(p));

        // Check local game dump presence
        let mut local_game_dump_found = false;
        let mut local_game_dump_path: Option<String> = None;

        if is_game_installed {
            let ue4ss_root = Path::new(&game_path).join("Pal").join("Binaries").join("Win64").join("ue4ss");
            let candidate = match id {
                "mappings" => {
                    let mut found = None;
                    if ue4ss_root.is_dir() {
                        if let Ok(entries) = fs::read_dir(&ue4ss_root) {
                            for e in entries.flatten() {
                                if e.path().extension().map(|x| x == "usmap").unwrap_or(false) {
                                    found = Some(e.path());
                                    break;
                                }
                            }
                        }
                    }
                    found
                }
                "jmap" => {
                    let mut found = None;
                    if ue4ss_root.is_dir() {
                        if let Ok(entries) = fs::read_dir(&ue4ss_root) {
                            for e in entries.flatten() {
                                if e.path().extension().map(|x| x == "jmap").unwrap_or(false) {
                                    found = Some(e.path());
                                    break;
                                }
                            }
                        }
                    }
                    found
                }
                "sdk" => {
                    let cxx = ue4ss_root.join("CXXHeaderDump");
                    if cxx.is_dir() { Some(cxx) } else { None }
                }
                "lua_types" => {
                    let types = ue4ss_root.join("Mods").join("shared").join("types");
                    if types.is_dir() { Some(types) } else { None }
                }
                "uht" => {
                    let uht = ue4ss_root.join("UHTHeaderDump");
                    if uht.is_dir() { Some(uht) } else { None }
                }
                "bp_sdk" => {
                    let bp = ue4ss_root.join("UE4SS_SDK");
                    if bp.is_dir() { Some(bp) } else { None }
                }
                "schemas" => {
                    let sch = ue4ss_root.join("Mods").join("PalSchema").join("schemas");
                    if sch.is_dir() { Some(sch) } else { None }
                }
                _ => None,
            };

            if let Some(c) = candidate {
                local_game_dump_found = true;
                local_game_dump_path = Some(c.to_string_lossy().to_string());
            }
        }

        let total_items = match id {
            "uht" => Some(24076),
            "bp_sdk" => Some(14090),
            "lua_types" => Some(1712),
            "sdk" => Some(1712),
            "schemas" => Some(475),
            _ => None,
        };

        resources.push(ResourceItemStatus {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            filename: filename.clone(),
            is_available: exists,
            is_synced: exists,
            file_size_bytes: file_size,
            sha256,
            local_path: local_file.map(|p| p.to_string_lossy().to_string()),
            total_items,
            game_version: default_game_ver.clone(),
            steam_build_id: default_build_id.clone(),
            ue4ss_commit: default_commit.clone(),
            has_local_game_dump: local_game_dump_found,
            local_game_dump_path,
        });
    }

    Ok(MasterResourcesStatus {
        detected_steam_build_id: detected_build_id,
        detected_game_version: default_game_ver,
        latest_game_version: manifest.latest_game_version,
        latest_steam_build_id: manifest.latest_steam_build_id,
        is_game_installed,
        resources,
    })
}

#[tauri::command]
pub async fn sync_development_resource(
    target: String,
    state: State<'_, AppState>,
) -> Result<ResourceActionResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let target_clean = target.trim().to_lowercase();
    let dest_dir = resolve_resource_dir(&program_path, &target_clean);
    let _ = fs::create_dir_all(&dest_dir);

    // Read manifest for expected checksum and filename
    let manifest_path = dest_dir.join("manifest.json");
    let manifest_content = if manifest_path.is_file() {
        fs::read_to_string(&manifest_path).ok()
    } else {
        find_bundled_resource(&format!("resources/{}/manifest.json", target_clean))
            .and_then(|p| fs::read_to_string(p).ok())
    };

    let manifest_val: Option<serde_json::Value> = manifest_content.as_deref().and_then(|c| serde_json::from_str(c).ok());

    let (expected_filename, expected_url, expected_hash) = if let Some(ref m) = manifest_val {
        let array_key = match target_clean.as_str() {
            "mappings" => "mappings",
            "jmap" => "jmaps",
            "lua_types" => "types",
            "uht" => "uht",
            "bp_sdk" => "bp_sdk",
            "schemas" => "schemas",
            _ => target_clean.as_str(),
        };

        let first = m.get(array_key).and_then(|a| a.as_array()).and_then(|a| a.first());
        let fn_val = first.and_then(|f| {
            f.get(format!("{}_filename", target_clean))
                .or_else(|| f.get("types_filename"))
                .or_else(|| f.get("usmap_filename"))
                .or_else(|| f.get("jmap_filename"))
                .or_else(|| f.get("uht_filename"))
                .or_else(|| f.get("bp_sdk_filename"))
                .or_else(|| f.get("schemas_filename"))
                .or_else(|| f.get("sdk_filename"))
        }).and_then(|v| v.as_str()).unwrap_or("");

        let url_val = first.and_then(|f| {
            f.get(format!("{}_url", target_clean))
                .or_else(|| f.get("types_url"))
                .or_else(|| f.get("usmap_url"))
                .or_else(|| f.get("jmap_url"))
                .or_else(|| f.get("uht_url"))
                .or_else(|| f.get("bp_sdk_url"))
                .or_else(|| f.get("schemas_url"))
                .or_else(|| f.get("sdk_url"))
        }).and_then(|v| v.as_str()).unwrap_or("");

        let sha_val = first.and_then(|f| f.get("sha256")).and_then(|v| v.as_str()).unwrap_or("");

        (fn_val.to_string(), url_val.to_string(), sha_val.to_string())
    } else {
        (String::new(), String::new(), String::new())
    };

    if expected_filename.is_empty() {
        return Err(format!("Could not resolve resource package details for target '{}'", target_clean));
    }

    let target_file_path = dest_dir.join(&expected_filename);

    // 1. Check if bundled file matches
    let bundled_rel = format!("resources/{}/{}", target_clean, expected_filename);
    if let Some(bundled_path) = find_bundled_resource(&bundled_rel) {
        if let Ok(bytes) = fs::read(&bundled_path) {
            let actual_hash = compute_sha256_bytes(&bytes);
            if expected_hash.is_empty() || actual_hash.eq_ignore_ascii_case(&expected_hash) {
                let _ = fs::write(&target_file_path, &bytes);
                // Extra step for lua_types: unpack Pal.lua
                if target_clean == "lua_types" {
                    unpack_pal_lua_from_bytes(&bytes, &dest_dir);
                }
                return Ok(ResourceActionResult {
                    success: true,
                    message: format!("Resource '{}' loaded and verified from bundled repository assets.", target_clean),
                    target: target_clean,
                    file_size_bytes: Some(bytes.len() as u64),
                    sha256: Some(actual_hash),
                    total_files_extracted: None,
                });
            }
        }
    }

    // 2. Fetch from GitHub remote if not found or checksum differed
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP Client creation error: {}", e))?;

    let candidate_urls = [
        expected_url.clone(),
        format!("https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/{}/{}", target_clean, expected_filename),
        format!("https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/{}/{}", target_clean, expected_filename),
        format!("https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/{}/{}", target_clean, expected_filename),
    ];

    let mut download_bytes: Option<Vec<u8>> = None;
    let mut last_err = String::new();

    for u in &candidate_urls {
        if u.is_empty() { continue; }
        crate::logger::log(&format!("Downloading resource '{}' from {}", target_clean, u));
        match client.get(u).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(bytes) = resp.bytes().await {
                    let actual_hash = compute_sha256_bytes(&bytes);
                    if expected_hash.is_empty() || actual_hash.eq_ignore_ascii_case(&expected_hash) {
                        download_bytes = Some(bytes.to_vec());
                        break;
                    } else {
                        last_err = format!("SHA256 mismatch from {}: expected {}, got {}", u, expected_hash, actual_hash);
                        crate::logger::log(&last_err);
                    }
                }
            }
            Ok(resp) => {
                last_err = format!("HTTP {} from {}", resp.status(), u);
            }
            Err(e) => {
                last_err = format!("Connection error to {}: {}", u, e);
            }
        }
    }

    let bytes = download_bytes.ok_or_else(|| format!("Failed to download verified resource package: {}", last_err))?;
    let hash = compute_sha256_bytes(&bytes);
    fs::write(&target_file_path, &bytes).map_err(|e| format!("Failed to write resource file {:?}: {}", target_file_path, e))?;

    if target_clean == "lua_types" {
        unpack_pal_lua_from_bytes(&bytes, &dest_dir);
    }

    crate::logger::log(&format!("Successfully synced and verified resource '{}' ({} bytes, SHA256: {})", target_clean, bytes.len(), hash));

    Ok(ResourceActionResult {
        success: true,
        message: format!("Successfully synced and verified resource '{}'.", target_clean),
        target: target_clean,
        file_size_bytes: Some(bytes.len() as u64),
        sha256: Some(hash),
        total_files_extracted: None,
    })
}

#[tauri::command]
pub fn export_development_resource(
    target: String,
    destination_folder: String,
    state: State<'_, AppState>,
) -> Result<ResourceActionResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let target_clean = target.trim().to_lowercase();
    let dest_out = PathBuf::from(&destination_folder);
    if !dest_out.exists() {
        fs::create_dir_all(&dest_out).map_err(|e| format!("Failed to create destination directory {:?}: {}", dest_out, e))?;
    }

    // Find the target archive
    let mut zip_path: Option<PathBuf> = None;
    let resource_dir = resolve_resource_dir(&program_path, &target_clean);

    if resource_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&resource_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "zip").unwrap_or(false) {
                    zip_path = Some(p);
                    break;
                }
            }
        }
    }

    if zip_path.is_none() {
        // Try bundled fallback
        let pattern = match target_clean.as_str() {
            "bp_sdk" => "Palworld_BP_SDK_25094871.zip",
            "uht" => "Palworld_UHT_SDK_25094871.zip",
            "jmap" => "Palworld_25094871.jmap.zip",
            "lua_types" => "Palworld_LuaTypes_25094871.zip",
            "sdk" => "Palworld_SDK_25094871.zip",
            _ => "",
        };
        if !pattern.is_empty() {
            zip_path = find_bundled_resource(&format!("resources/{}/{}", target_clean, pattern));
        }
    }

    let archive_path = zip_path.ok_or_else(|| {
        format!("No resource archive found for target '{}'. Please sync it first.", target_clean)
    })?;

    let file = fs::File::open(&archive_path)
        .map_err(|e| format!("Failed to open archive {:?}: {}", archive_path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Invalid ZIP archive {:?}: {}", archive_path, e))?;

    let mut count = 0;
    for i in 0..archive.len() {
        let mut zip_entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = match zip_entry.enclosed_name() {
            Some(path) => dest_out.join(path),
            None => continue,
        };

        if (*zip_entry.name()).ends_with('/') {
            let _ = fs::create_dir_all(&outpath);
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    let _ = fs::create_dir_all(p);
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| format!("Failed to create output file: {}", e))?;
            std::io::copy(&mut zip_entry, &mut outfile).map_err(|e| format!("Failed to extract file: {}", e))?;
            count += 1;
        }
    }

    crate::logger::log(&format!("Exported {} files from resource '{}' to {:?}", count, target_clean, dest_out));

    Ok(ResourceActionResult {
        success: true,
        message: format!("Successfully exported {} files into destination.", count),
        target: target_clean,
        file_size_bytes: None,
        sha256: None,
        total_files_extracted: Some(count),
    })
}

#[tauri::command]
pub fn purge_development_resource(
    target: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let target_clean = target.trim().to_lowercase();
    let resource_dir = resolve_resource_dir(&program_path, &target_clean);

    if resource_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&resource_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                // Never delete manifest.json
                if p.file_name().map(|f| f == "manifest.json").unwrap_or(false) {
                    continue;
                }
                if p.is_file() {
                    let _ = fs::remove_file(&p);
                } else if p.is_dir() {
                    let _ = fs::remove_dir_all(&p);
                }
            }
        }
    }

    crate::logger::log(&format!("Purged cached files for resource '{}'", target_clean));
    Ok(true)
}

fn unpack_pal_lua_from_bytes(bytes: &[u8], dest_dir: &Path) {
    let reader = std::io::Cursor::new(bytes);
    if let Ok(mut archive) = zip::ZipArchive::new(reader) {
        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let name = file.name();
                if name == "Pal.lua" || name.ends_with("/Pal.lua") || name.ends_with("\\Pal.lua") {
                    let mut content = String::new();
                    if std::io::Read::read_to_string(&mut file, &mut content).is_ok() {
                        let _ = fs::write(dest_dir.join("Pal.lua"), content);
                        crate::logger::log(&format!("Unpacked loose Pal.lua to {:?}", dest_dir.join("Pal.lua")));
                    }
                    break;
                }
            }
        }
    }
}
