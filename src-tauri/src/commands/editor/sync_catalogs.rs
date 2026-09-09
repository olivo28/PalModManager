use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::State;
use crate::state::AppState;
use crate::usmap::{get_blueprints_dir, get_datatables_dir, PalSchemaDefinition};

const BLUEPRINT_MANIFEST_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/blueprints/manifest.json",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/blueprints/manifest.json",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/blueprints/manifest.json",
];

const DATATABLES_MANIFEST_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/datatables/manifest.json",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/datatables/manifest.json",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/datatables/manifest.json",
];

const PALSCHEMA_MANIFEST_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/schemas/manifest.json",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/schemas/manifest.json",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/schemas/manifest.json",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCatalogResult {
    pub success: bool,
    pub message: String,
    pub total_items: usize,
    pub filename: String,
    pub updated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataTableManifestEntry {
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub steam_build_id: String,
    #[serde(default)]
    pub datatables_filename: String,
    #[serde(default)]
    pub datatables_url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub file_size_bytes: u64,
    #[serde(default)]
    pub total_tables: usize,
    #[serde(default)]
    pub total_rows: usize,
    #[serde(default)]
    pub is_latest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataTableManifest {
    #[serde(default)]
    pub datatables: Vec<DataTableManifestEntry>,
    #[serde(default)]
    pub latest_game_version: String,
    #[serde(default)]
    pub latest_steam_build_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlueprintManifestEntry {
    #[serde(default)]
    pub blueprints_filename: String,
    #[serde(default)]
    pub blueprints_url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub file_size_bytes: u64,
    #[serde(default)]
    pub total_blueprints: usize,
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub steam_build_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlueprintManifest {
    #[serde(default)]
    pub blueprints: Vec<BlueprintManifestEntry>,
    #[serde(default)]
    pub latest_game_version: String,
    #[serde(default)]
    pub latest_steam_build_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PalSchemaManifestEntry {
    #[serde(default)]
    pub palschema_version: String,
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub steam_build_id: String,
    #[serde(default)]
    pub schemas_filename: String,
    #[serde(default)]
    pub schemas_url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub file_size_bytes: u64,
    #[serde(default)]
    pub total_raw_schemas: usize,
    #[serde(default)]
    pub total_domain_schemas: usize,
    #[serde(default)]
    pub is_latest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PalSchemaManifest {
    #[serde(default)]
    pub latest_palschema_version: String,
    #[serde(default)]
    pub latest_game_version: String,
    #[serde(default)]
    pub latest_steam_build_id: String,
    #[serde(default)]
    pub schemas: Vec<PalSchemaManifestEntry>,
}

fn compute_sha256(path: &PathBuf) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

#[tauri::command]
pub async fn sync_blueprints_catalog(program_path: Option<String>) -> Result<SyncCatalogResult, String> {
    let prog = program_path.unwrap_or_default();
    let bp_dir = get_blueprints_dir(&prog);
    let _ = fs::create_dir_all(&bp_dir);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(&format!("PalModManager/{} (GitHub: olivo28/PalModManager)", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // 1. Fetch manifest
    let mut manifest: Option<BlueprintManifest> = None;
    for url in BLUEPRINT_MANIFEST_URLS {
        if let Ok(res) = client.get(*url).send().await {
            if res.status().is_success() {
                if let Ok(text) = res.text().await {
                    if let Ok(m) = serde_json::from_str::<BlueprintManifest>(&text) {
                        let _ = fs::write(bp_dir.join("manifest.json"), &text);
                        manifest = Some(m);
                        break;
                    }
                }
            }
        }
    }

    let manifest = manifest.ok_or_else(|| "Failed to reach remote Blueprints manifest on GitHub.".to_string())?;
    let entry = manifest.blueprints.first().ok_or_else(|| "Remote Blueprints manifest has no blueprint entries.".to_string())?;

    let filename = if !entry.blueprints_filename.is_empty() {
        entry.blueprints_filename.clone()
    } else {
        format!("Palworld_Blueprints_{}.json", entry.steam_build_id)
    };

    let target_path = bp_dir.join(&filename);

    // 2. Check if already up-to-date
    if target_path.exists() {
        if let Some(hash) = compute_sha256(&target_path) {
            if !entry.sha256.is_empty() && hash.eq_ignore_ascii_case(&entry.sha256) {
                crate::logger::log(&format!("Blueprints catalog is already up to date: {} ({} BPs)", filename, entry.total_blueprints));
                return Ok(SyncCatalogResult {
                    success: true,
                    message: "Blueprints catalog is already up to date.".to_string(),
                    total_items: entry.total_blueprints,
                    filename,
                    updated: false,
                });
            }
        }
    }

    // 3. Download the catalog JSON
    let download_url = if !entry.blueprints_url.is_empty() {
        entry.blueprints_url.clone()
    } else {
        format!("https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/blueprints/{}", filename)
    };

    crate::logger::log(&format!("Downloading Live Game Blueprints from {}", download_url));

    let res = client.get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download Blueprints catalog: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Download failed with status: {}", res.status()));
    }

    let bytes = res.bytes().await.map_err(|e| format!("Failed to read response body: {}", e))?;
    fs::write(&target_path, &bytes).map_err(|e| format!("Failed to save Blueprints catalog to disk: {}", e))?;

    // Invalidate in-memory cache
    crate::usmap::blueprint_index::invalidate_blueprint_cache();

    crate::logger::log(&format!("Live Game Blueprints catalog synced successfully: {} ({} bytes, {} items)", filename, bytes.len(), entry.total_blueprints));

    Ok(SyncCatalogResult {
        success: true,
        message: format!("Synced {} blueprints successfully.", entry.total_blueprints),
        total_items: entry.total_blueprints,
        filename,
        updated: true,
    })
}

#[tauri::command]
pub async fn sync_datatables_catalog(program_path: Option<String>) -> Result<SyncCatalogResult, String> {
    let prog = program_path.unwrap_or_default();
    let dt_dir = get_datatables_dir(&prog);
    let _ = fs::create_dir_all(&dt_dir);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(&format!("PalModManager/{} (GitHub: olivo28/PalModManager)", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // 1. Fetch remote manifest or fall back to local bundled manifest
    let mut manifest_opt: Option<DataTableManifest> = None;
    let mut manifest_raw: Option<String> = None;

    for url in DATATABLES_MANIFEST_URLS {
        if let Ok(res) = client.get(*url).send().await {
            if res.status().is_success() {
                if let Ok(text) = res.text().await {
                    if let Ok(m) = serde_json::from_str::<DataTableManifest>(&text) {
                        manifest_raw = Some(text);
                        manifest_opt = Some(m);
                        break;
                    }
                }
            }
        }
    }

    if manifest_opt.is_none() {
        let bundled_manifest = crate::usmap::sync::find_bundled_resource("resources/datatables/manifest.json")
            .or_else(|| {
                let p = dt_dir.join("manifest.json");
                if p.exists() { Some(p) } else { None }
            });
        if let Some(mp) = bundled_manifest {
            if let Ok(text) = fs::read_to_string(&mp) {
                if let Ok(m) = serde_json::from_str::<DataTableManifest>(&text) {
                    manifest_opt = Some(m);
                }
            }
        }
    }

    let manifest = manifest_opt.ok_or_else(|| "Failed to load DataTables manifest from GitHub or local backup.".to_string())?;
    let entry = manifest.datatables.iter().find(|d| d.is_latest).or_else(|| manifest.datatables.first())
        .ok_or_else(|| "DataTables manifest contains no datatable entries.".to_string())?;

    let filename = if !entry.datatables_filename.is_empty() {
        entry.datatables_filename.clone()
    } else {
        format!("Palworld_DataTables_{}.json", entry.steam_build_id)
    };

    let target_path = dt_dir.join(&filename);

    // Save manifest if fetched
    if let Some(ref text) = manifest_raw {
        let _ = fs::write(dt_dir.join("manifest.json"), text);
    }

    // 2. Check if already up-to-date in dt_dir
    if target_path.exists() {
        if let Some(hash) = compute_sha256(&target_path) {
            if !entry.sha256.is_empty() && hash.eq_ignore_ascii_case(&entry.sha256) {
                crate::usmap::datatable_index::invalidate_datatable_cache();
                crate::logger::log(&format!("DataTables catalog is already up to date: {} ({} tables)", filename, entry.total_tables));
                return Ok(SyncCatalogResult {
                    success: true,
                    message: "DataTables catalog is already up to date.".to_string(),
                    total_items: entry.total_tables,
                    filename,
                    updated: false,
                });
            }
        }
    }

    // 3. Check bundled local resource
    let rel_bundled = format!("resources/datatables/{}", filename);
    if let Some(bundled_path) = crate::usmap::sync::find_bundled_resource(&rel_bundled) {
        if let Ok(bytes) = fs::read(&bundled_path) {
            let hash = {
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                format!("{:x}", hasher.finalize())
            };
            if entry.sha256.is_empty() || hash.eq_ignore_ascii_case(&entry.sha256) {
                let _ = fs::write(&target_path, &bytes);
                crate::usmap::datatable_index::invalidate_datatable_cache();
                crate::logger::log(&format!("DataTables catalog loaded from verified local bundle: {} ({} tables)", filename, entry.total_tables));
                return Ok(SyncCatalogResult {
                    success: true,
                    message: format!("Synced {} DataTables from verified local bundle.", entry.total_tables),
                    total_items: entry.total_tables,
                    filename,
                    updated: true,
                });
            }
        }
    }

    // 4. Download from remote CDN
    let mut download_urls = Vec::new();
    if !entry.datatables_url.is_empty() {
        download_urls.push(entry.datatables_url.clone());
    }
    download_urls.push(format!("https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/datatables/{}", filename));
    download_urls.push(format!("https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/datatables/{}", filename));
    download_urls.push(format!("https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/datatables/{}", filename));

    let mut downloaded_bytes: Option<Vec<u8>> = None;
    for url in &download_urls {
        if let Ok(res) = client.get(url).send().await {
            if res.status().is_success() {
                if let Ok(bytes) = res.bytes().await {
                    downloaded_bytes = Some(bytes.to_vec());
                    break;
                }
            }
        }
    }

    let bytes = downloaded_bytes.ok_or_else(|| "Failed to download DataTables index from GitHub or mirrors.".to_string())?;

    fs::write(&target_path, &bytes).map_err(|e| format!("Failed to write DataTables catalog to disk: {}", e))?;

    crate::usmap::datatable_index::invalidate_datatable_cache();

    crate::logger::log(&format!("PalSchema DataTables catalog synced successfully ({} bytes, {} tables)", bytes.len(), entry.total_tables));

    Ok(SyncCatalogResult {
        success: true,
        message: format!("Synced {} DataTables successfully ({}).", entry.total_tables, filename),
        total_items: entry.total_tables,
        filename,
        updated: true,
    })
}

#[tauri::command]
pub fn get_palschema_schemas_catalog(state: State<'_, AppState>) -> Result<crate::usmap::PalSchemaCatalogStatus, String> {
    let (program_path, game_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.program_path.clone(), locked.settings.game_path.clone())
    };
    Ok(crate::usmap::get_or_load_palschema_catalog(&program_path, &game_path))
}

#[tauri::command]
pub fn get_palschema_monaco_definitions(
    include_raw: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::usmap::PalSchemaDefinition>, String> {
    let (program_path, game_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.program_path.clone(), locked.settings.game_path.clone())
    };
    let arc_defs = crate::usmap::load_palschema_definitions_for_monaco(
        &program_path,
        &game_path,
        include_raw.unwrap_or(false),
    );
    Ok((*arc_defs).clone())
}

#[tauri::command]
pub async fn sync_palschema_schemas(program_path: Option<String>, state: State<'_, AppState>) -> Result<SyncCatalogResult, String> {
    let (prog, game_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (program_path.unwrap_or_else(|| locked.settings.program_path.clone()), locked.settings.game_path.clone())
    };

    let schemas_base = if !prog.is_empty() {
        PathBuf::from(&prog).join("resources").join("schemas")
    } else {
        PathBuf::from("resources").join("schemas")
    };
    let _ = fs::create_dir_all(&schemas_base);

    let schemas_dir = crate::usmap::get_palschema_schemas_dir(&prog, &game_path);
    let _ = fs::create_dir_all(&schemas_dir);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(&format!("PalModManager/{} (GitHub: olivo28/PalModManager)", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // 1. Fetch remote manifest or fall back to local manifest
    let mut manifest_bytes: Option<Vec<u8>> = None;
    for url in PALSCHEMA_MANIFEST_URLS {
        if let Ok(res) = client.get(*url).send().await {
            if res.status().is_success() {
                if let Ok(bytes) = res.bytes().await {
                    manifest_bytes = Some(bytes.to_vec());
                    break;
                }
            }
        }
    }

    let manifest: PalSchemaManifest = if let Some(ref mb) = manifest_bytes {
        serde_json::from_slice(mb).unwrap_or(PalSchemaManifest {
            latest_palschema_version: "0.6.6".to_string(),
            latest_game_version: "v1.0.3".to_string(),
            latest_steam_build_id: "24575825".to_string(),
            schemas: vec![],
        })
    } else {
        let local_manifest = schemas_base.join("manifest.json");
        if local_manifest.exists() {
            let data = fs::read(&local_manifest).map_err(|e| format!("Failed to read local manifest: {}", e))?;
            serde_json::from_slice(&data).unwrap_or(PalSchemaManifest {
                latest_palschema_version: "0.6.6".to_string(),
                latest_game_version: "v1.0.3".to_string(),
                latest_steam_build_id: "24575825".to_string(),
                schemas: vec![],
            })
        } else {
            PalSchemaManifest {
                latest_palschema_version: "0.6.6".to_string(),
                latest_game_version: "v1.0.3".to_string(),
                latest_steam_build_id: "24575825".to_string(),
                schemas: vec![],
            }
        }
    };

    // 2. Select target entry (latest or first)
    let target_entry = manifest.schemas.iter().find(|s| s.is_latest).or_else(|| manifest.schemas.first());
    let filename = target_entry
        .map(|e| e.schemas_filename.clone())
        .unwrap_or_else(|| "palschema_schemas_0.6.6.zip".to_string());
    let expected_sha256 = target_entry.map(|e| e.sha256.clone()).unwrap_or_default();
    let target_path = schemas_base.join(&filename);

    // 3. Check if local file is already present and matches sha256
    let mut already_valid = false;
    if target_path.exists() && !expected_sha256.is_empty() {
        if let Some(actual_hash) = compute_sha256(&target_path) {
            if actual_hash.eq_ignore_ascii_case(&expected_sha256) {
                already_valid = true;
            }
        }
    }

    if already_valid {
        crate::usmap::invalidate_palschema_catalog_cache();
        let status = crate::usmap::get_or_load_palschema_catalog(&prog, &game_path);
        let total = status.total_raw_schemas + status.total_domain_schemas;
        return Ok(SyncCatalogResult {
            success: true,
            message: format!("PalSchema schemas catalog is already up to date ({}, {} specifications).", filename, total),
            total_items: total,
            filename,
            updated: false,
        });
    }

    // 4. Download versioned zip from GitHub / CDN
    let mut download_urls = Vec::new();
    if let Some(entry) = target_entry {
        if !entry.schemas_url.is_empty() {
            download_urls.push(entry.schemas_url.as_str());
        }
    }
    let default_raw = format!("https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/schemas/{}", filename);
    let default_jsdelivr = format!("https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/schemas/{}", filename);
    let default_fastly = format!("https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/schemas/{}", filename);
    download_urls.push(&default_raw);
    download_urls.push(&default_jsdelivr);
    download_urls.push(&default_fastly);

    let mut downloaded_bytes: Option<Vec<u8>> = None;
    for url in download_urls {
        if let Ok(res) = client.get(url).send().await {
            if res.status().is_success() {
                if let Ok(bytes) = res.bytes().await {
                    downloaded_bytes = Some(bytes.to_vec());
                    break;
                }
            }
        }
    }

    // 5. Fallback to local files if network failed
    let zip_bytes = if let Some(bytes) = downloaded_bytes {
        let _ = fs::write(&target_path, &bytes);
        if let Some(mb) = manifest_bytes {
            let _ = fs::write(schemas_base.join("manifest.json"), mb);
        }
        bytes
    } else {
        if target_path.exists() {
            fs::read(&target_path).map_err(|e| format!("Failed to read local schemas zip: {}", e))?
        } else {
            let fallback_alt = schemas_base.join("palschema_schemas.zip");
            if fallback_alt.exists() {
                fs::read(&fallback_alt).map_err(|e| format!("Failed to read local schemas zip: {}", e))?
            } else {
                return Err(format!("Failed to download {} from GitHub and no local backup was found.", filename));
            }
        }
    };

    // 6. Extract into schemas_dir
    let temp_zip = std::env::temp_dir().join("palschema_schemas_sync.zip");
    fs::write(&temp_zip, &zip_bytes).map_err(|e| format!("Failed to write temporary zip: {}", e))?;
    let _ = crate::zip_handler::extract_zip_to_temp(&temp_zip.to_string_lossy(), &schemas_dir);
    let _ = fs::remove_file(&temp_zip);

    crate::usmap::invalidate_palschema_catalog_cache();
    let status = crate::usmap::get_or_load_palschema_catalog(&prog, &game_path);

    let total = status.total_raw_schemas + status.total_domain_schemas;
    crate::logger::log(&format!("PalSchema schemas synced successfully: {} raw, {} domain schemas ({})", status.total_raw_schemas, status.total_domain_schemas, filename));

    Ok(SyncCatalogResult {
        success: true,
        message: format!("Synced {} PalSchema specification schemas successfully ({}).", total, filename),
        total_items: total,
        filename,
        updated: true,
    })
}

#[tauri::command]
pub fn get_palschema_raw_schema(
    table_name: String,
    state: State<'_, AppState>,
) -> Result<Option<PalSchemaDefinition>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();

    Ok(crate::usmap::load_single_raw_palschema_definition(&program_path, &game_path, &table_name))
}


