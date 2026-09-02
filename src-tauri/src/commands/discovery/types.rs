use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryModItem {
    pub mod_id: u32,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub version: String,
    pub downloads: u32,
    pub endorsements: u32,
    pub picture_url: String,
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub contains_adult_content: bool,
    pub is_endorsed: bool,
    pub is_tracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCategory {
    pub category_id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryFileItem {
    pub file_id: u64,
    pub name: String,
    pub version: String,
    pub category_id: u32,
    pub category_name: String,
    pub is_primary: bool,
    pub size_in_bytes: u64,
    pub size_formatted: String,
    pub uploaded_at: String,
    pub uploaded_timestamp: Option<i64>,
    pub description: String,
    pub unique_downloads: Option<u32>,
    pub total_downloads: Option<u32>,
    pub scan_status: Option<String>,
    pub changelog_entries: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryModDetails {
    pub mod_id: u32,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub downloads: u32,
    pub endorsements: u32,
    pub picture_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub contains_adult_content: bool,
    pub files: Vec<DiscoveryFileItem>,
    pub images: Vec<String>,
    pub is_endorsed: bool,
    pub is_tracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResponse {
    pub mods: Vec<DiscoveryModItem>,
    pub total_count: u32,
    pub page: u32,
    pub page_size: u32,
}
