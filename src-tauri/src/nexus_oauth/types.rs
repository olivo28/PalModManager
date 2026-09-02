use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusUserInfoResponse {
    pub sub: Option<serde_json::Value>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub membership_roles: Option<Vec<String>>,
    pub premium_expiry: Option<serde_json::Value>,
    pub user_id: Option<u64>,
    pub user: Option<NexusUserNested>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusUserNested {
    pub id: Option<u64>,
    pub username: Option<String>,
    pub membership_roles: Option<Vec<String>>,
    pub premium_expiry: Option<serde_json::Value>,
}

pub struct FetchedProfile {
    pub user_id: Option<u64>,
    pub username: String,
    pub avatar_url: Option<String>,
    pub roles: Vec<String>,
    pub kudos: Option<u32>,
    pub profile_views: Option<u32>,
    pub endorsements_given: Option<u32>,
    pub joined_date: Option<String>,
    pub last_active_date: Option<String>,
    pub about_me: Option<String>,
    pub mod_count: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ModBasicInfo {
    pub name: String,
    pub summary: Option<String>,
    pub picture_url: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxmLinkInfo {
    pub raw_url: String,
    pub game_domain: String,
    pub mod_id: u32,
    pub file_id: u64,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxmModMetadata {
    pub mod_id: u32,
    pub name: String,
    pub summary: Option<String>,
    pub picture_url: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxmDownloadProgressEvent {
    pub download_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f32,
}
