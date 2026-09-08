use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingEntry {
    #[serde(alias = "game_version")]
    pub game_version: String,
    #[serde(alias = "steam_build_id")]
    pub steam_build_id: Option<String>,
    #[serde(alias = "app_id")]
    pub app_id: Option<u32>,
    #[serde(alias = "usmap_filename")]
    pub usmap_filename: String,
    #[serde(alias = "usmap_url")]
    pub usmap_url: String,
    pub sha256: String,
    #[serde(alias = "file_size_bytes")]
    pub file_size_bytes: u64,
    #[serde(alias = "engine_version")]
    pub engine_version: String,
    #[serde(alias = "build_id")]
    pub build_id: String,
    #[serde(default, alias = "is_latest")]
    pub is_latest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingsManifest {
    #[serde(alias = "schema_version")]
    pub schema_version: String,
    #[serde(alias = "latest_game_version")]
    pub latest_game_version: String,
    #[serde(default, alias = "latest_steam_build_id")]
    pub latest_steam_build_id: Option<String>,
    #[serde(alias = "updated_at")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterVersionEntry {
    #[serde(alias = "game_version")]
    pub game_version: String,
    #[serde(default, alias = "steam_build_id")]
    pub steam_build_id: Option<String>,
    #[serde(default, alias = "ue4ss_commit")]
    pub ue4ss_commit: Option<String>,
    #[serde(default, alias = "engine_version")]
    pub engine_version: Option<String>,
    #[serde(default, alias = "is_latest")]
    pub is_latest: bool,
    #[serde(default)]
    pub usmap: Option<String>,
    #[serde(default)]
    pub jmap: Option<String>,
    #[serde(default)]
    pub sdk: Option<String>,
    #[serde(default, alias = "lua_types")]
    pub lua_types: Option<String>,
    #[serde(default)]
    pub uht: Option<String>,
    #[serde(default, alias = "bp_sdk")]
    pub bp_sdk: Option<String>,
    #[serde(default, alias = "palschema_version")]
    pub palschema_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterResourceManifest {
    #[serde(alias = "schema_version")]
    pub schema_version: String,
    #[serde(alias = "latest_game_version")]
    pub latest_game_version: String,
    #[serde(alias = "latest_steam_build_id")]
    pub latest_steam_build_id: String,
    #[serde(alias = "updated_at")]
    pub updated_at: String,
    #[serde(default)]
    pub versions: Vec<MasterVersionEntry>,
}

