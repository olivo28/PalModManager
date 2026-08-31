use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingEntry {
    pub game_version: String,
    pub steam_build_id: Option<String>,
    pub app_id: Option<u32>,
    pub usmap_filename: String,
    pub usmap_url: String,
    pub sha256: String,
    pub file_size_bytes: u64,
    pub engine_version: String,
    pub build_id: String,
    #[serde(default)]
    pub is_latest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingsManifest {
    pub schema_version: String,
    pub latest_game_version: String,
    pub updated_at: String,
    pub mappings: Vec<MappingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledBuildInfo {
    pub app_id: Option<u32>,
    pub build_id: Option<String>,
    pub game_version: Option<String>,
    pub source: String,
    pub is_steam: bool,
    pub is_gamepass: bool,
    pub last_updated_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsmapStatus {
    pub installed_build: InstalledBuildInfo,
    pub active_mapping: Option<MappingEntry>,
    pub is_synced: bool,
    pub local_usmap_exists: bool,
    pub local_file_size: u64,
    pub local_sha256: Option<String>,
    pub latest_remote_version: Option<String>,
    pub error_message: Option<String>,
    pub mappings_path: String,
}
