use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModType {
    #[serde(rename = "ue4ss")]
    Ue4ss,
    #[serde(rename = "palschema")]
    PalSchema,
    #[serde(rename = "pak")]
    Pak,
    #[serde(rename = "logicmods")]
    LogicMods,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub mod_type: ModType,
    pub nexus_mod_id: Option<u32>,
    pub nexus_url: Option<String>,
    pub nexus_author: Option<String>,
    pub nexus_summary: Option<String>,
    pub nexus_picture_url: Option<String>,
    pub nexus_endorsements: Option<u32>,
    pub nexus_downloads: Option<u32>,
    pub version: String,
    pub install_date: String,
    pub source_zip: String,
    pub config_path: Option<String>,
    pub config_type: Option<String>,
    pub enabled: bool,
    pub game_path: String,
    pub disabled_path: String,
    pub pak_destination: Option<String>,
    pub has_enabled_txt: bool,
    pub mods_txt_order: Option<u32>,
    #[serde(default)]
    pub extra_files: Vec<String>,
    #[serde(default)]
    pub nexus_description: Option<String>,
    #[serde(default)]
    pub nexus_version_cached: Option<String>,
    #[serde(default)]
    pub nexus_cached_at: Option<String>,
    #[serde(default)]
    pub nexus_category: Option<String>,
    #[serde(default)]
    pub nexus_tags: Vec<String>,
    #[serde(default)]
    pub github_repo: Option<String>,
    #[serde(default)]
    pub github_version: Option<String>,
    #[serde(default)]
    pub github_cached_at: Option<String>,
    #[serde(default)]
    pub update_date: Option<String>,
    #[serde(default)]
    pub library_zip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub game_path: String,
    pub program_path: String,
    pub nexus_api_key: Option<String>,
    #[serde(default)]
    pub hide_native_mods: Option<bool>,
    #[serde(default)]
    pub debug_console: Option<bool>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub created_at: String,
    /// All mods installed in this profile — always visible in the mod list
    #[serde(default)]
    pub installed_mod_ids: Vec<String>,
    /// Subset of installed_mod_ids that are currently active/enabled
    #[serde(default)]
    pub enabled_mod_ids: Vec<String>,
    #[serde(default)]
    pub ue4ss_enabled: bool,
    #[serde(default)]
    pub palschema_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppData {
    pub mods: Vec<ModInfo>,
    pub settings: AppSettings,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default = "default_profile_id")]
    pub current_profile_id: String,
}

fn default_profile_id() -> String {
    "default".to_string()
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            mods: Vec::new(),
            settings: AppSettings {
                game_path: String::new(),
                program_path: String::new(),
                nexus_api_key: None,
                hide_native_mods: Some(false),
                debug_console: Some(false),
            },
            profiles: Vec::new(),
            current_profile_id: "default".to_string(),
        }
    }
}

