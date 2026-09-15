use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceItemStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub is_available: bool,
    pub is_synced: bool,
    #[serde(default)]
    pub is_build_matched: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevResourceProgressPayload {
    pub target: String,
    pub percent: u8,
    pub current_file: String,
    pub processed_files: usize,
    pub total_files: usize,
}
