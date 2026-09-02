use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::usmap::{get_blueprints_dir, get_datatables_dir};

const BLUEPRINT_MANIFEST_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/blueprints/manifest.json",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/blueprints/manifest.json",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/blueprints/manifest.json",
];

const DATATABLES_INDEX_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/datatables/dt_index.json",
    "https://cdn.jsdelivr.net/gh/olivo28/PalModManager@main/resources/datatables/dt_index.json",
    "https://fastly.jsdelivr.net/gh/olivo28/PalModManager@main/resources/datatables/dt_index.json",
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

    let filename = "dt_index.json".to_string();
    let target_path = dt_dir.join(&filename);

    let mut downloaded_bytes: Option<Vec<u8>> = None;

    for url in DATATABLES_INDEX_URLS {
        if let Ok(res) = client.get(*url).send().await {
            if res.status().is_success() {
                if let Ok(bytes) = res.bytes().await {
                    downloaded_bytes = Some(bytes.to_vec());
                    break;
                }
            }
        }
    }

    let bytes = downloaded_bytes.ok_or_else(|| "Failed to download DataTables index from GitHub.".to_string())?;

    // Parse to count tables
    let total_tables = if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        parsed.get("total_tables").and_then(|t| t.as_u64()).unwrap_or(423) as usize
    } else {
        423
    };

    fs::write(&target_path, &bytes).map_err(|e| format!("Failed to write dt_index.json to disk: {}", e))?;

    // Invalidate in-memory cache
    crate::usmap::datatable_index::invalidate_datatable_cache();

    crate::logger::log(&format!("PalSchema DataTables catalog synced successfully ({} bytes, {} tables)", bytes.len(), total_tables));

    Ok(SyncCatalogResult {
        success: true,
        message: format!("Synced {} DataTables successfully.", total_tables),
        total_items: total_tables,
        filename,
        updated: true,
    })
}
