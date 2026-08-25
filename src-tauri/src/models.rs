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
    #[serde(rename = "hybrid")]
    Hybrid,
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
    #[serde(default)]
    pub ignored_version: Option<String>,
    #[serde(default)]
    pub nexus_file_id: Option<u32>,
    #[serde(default)]
    pub ignored_keys: Option<Vec<String>>,
    #[serde(default)]
    pub has_pending_update: Option<bool>,
    #[serde(default)]
    pub origin_load_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NexusAccountInfo {
    pub user_id: Option<u64>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub is_premium: bool,
    pub is_supporter: bool,
    #[serde(default)]
    pub roles: Vec<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<i64>,
    pub kudos: Option<u32>,
    pub profile_views: Option<u32>,
    pub endorsements_given: Option<u32>,
    pub joined_date: Option<String>,
    pub last_active_date: Option<String>,
    pub about_me: Option<String>,
    pub mod_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexusUserEndorsement {
    #[serde(alias = "mod_id")]
    pub mod_id: u32,
    #[serde(alias = "domain_name")]
    pub domain_name: String,
    pub date: Option<String>,
    pub version: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub mod_title: Option<String>,
    #[serde(default)]
    pub picture_url: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexusUserTrackedMod {
    #[serde(alias = "mod_id")]
    pub mod_id: u32,
    #[serde(alias = "domain_name")]
    pub domain_name: String,
    #[serde(default)]
    pub mod_title: Option<String>,
    #[serde(default)]
    pub picture_url: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexusUserAuthoredMod {
    pub mod_id: u32,
    pub name: String,
    pub summary: Option<String>,
    pub version: Option<String>,
    pub downloads: Option<u32>,
    pub endorsements: Option<u32>,
    pub picture_url: Option<String>,
    pub game_name: Option<String>,
    pub domain_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub game_path: String,
    pub program_path: String,
    #[serde(default)]
    pub hide_native_mods: Option<bool>,
    #[serde(default)]
    pub debug_console: Option<bool>,
    #[serde(default)]
    pub force_load_order: Option<bool>,
    #[serde(default)]
    pub force_load_order_ue4ss: Option<bool>,
    #[serde(default)]
    pub force_load_order_palschema: Option<bool>,
    #[serde(default)]
    pub custom_data_path: Option<String>,
    #[serde(default)]
    pub window_width: Option<f64>,
    #[serde(default)]
    pub window_height: Option<f64>,
    #[serde(default)]
    pub window_maximized: Option<bool>,
    #[serde(default)]
    pub toolbar_scale: Option<f64>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub nexus_account: Option<NexusAccountInfo>,
    #[serde(default)]
    pub nexus_endorsements_cache: Option<Vec<NexusUserEndorsement>>,
    #[serde(default)]
    pub nexus_tracked_cache: Option<Vec<NexusUserTrackedMod>>,
    #[serde(default)]
    pub nexus_authored_cache: Option<Vec<NexusUserAuthoredMod>>,
    #[serde(default)]
    pub nexus_cache_timestamp: Option<i64>,
    #[serde(default)]
    pub dns_resolver: Option<String>,
    #[serde(default)]
    pub cache_remote_images: Option<bool>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFolder {
    pub id: String,
    pub name: String,
    pub mod_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyMode {
    Standard,
    Workshop,
    None,
}

fn default_dependency_mode() -> DependencyMode {
    DependencyMode::None
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
    #[serde(default = "default_dependency_mode")]
    pub dependency_mode: DependencyMode,
    #[serde(default)]
    pub mod_folders: Vec<ModFolder>,
    #[serde(default)]
    pub load_order_metadata: Option<Vec<(String, bool)>>,
    #[serde(default)]
    pub force_load_order_ue4ss: Option<bool>,
    #[serde(default)]
    pub force_load_order_palschema: Option<bool>,
    #[serde(default)]
    pub hide_native_mods: Option<bool>,
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
                hide_native_mods: Some(false),
                debug_console: Some(false),
                force_load_order: Some(false),
                force_load_order_ue4ss: Some(false),
                force_load_order_palschema: Some(false),
                custom_data_path: None,
                window_width: None,
                window_height: None,
                window_maximized: None,
                toolbar_scale: Some(1.0),
                language: None,
                nexus_account: None,
                nexus_endorsements_cache: None,
                nexus_tracked_cache: None,
                nexus_authored_cache: None,
                nexus_cache_timestamp: None,
                dns_resolver: Some("auto".to_string()),
                cache_remote_images: Some(true),
            },
            profiles: Vec::new(),
            current_profile_id: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RouteType {
    Ue4ss,
    PalSchema,
    Pak,
    LogicMods,
    Companion,
    Passthrough,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRoute {
    pub zip_path: String,
    pub dest_path: String,
    pub route_type: RouteType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallManifest {
    pub folder_name: String,
    pub display_name: String,
    pub mod_type: ModType,
    pub routes: Vec<FileRoute>,
    pub nexus_mod_id: Option<u32>,
    pub nexus_file_id: Option<u32>,
    pub has_pak: bool,
    pub has_ue4ss: bool,
    pub has_palschema: bool,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum WorkshopInstallType {
    UE4SSFramework,
    UE4SSMod,
    LuaMod,
    PalSchemaMod,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopMod {
    pub workshop_id: u64,
    pub package_name: String,
    pub mod_name: String,
    pub version: String,
    pub author: String,
    pub thumbnail_path: Option<String>,
    pub dependencies: Vec<String>,
    pub install_type: WorkshopInstallType,
    pub install_target: String,
    pub is_active: bool,
    pub is_installed: bool,
    pub is_framework: bool,
    pub last_install_time: Option<String>,
    pub last_update_time: Option<String>,
    pub has_pending_update: bool,
    #[serde(default)]
    pub installed_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopState {
    pub workshop_root: String,
    pub global_enabled: bool,
    pub active_mod_list: Vec<String>,
    pub mods: Vec<WorkshopMod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PalModSettings {
    pub global_enabled: bool,
    pub workshop_root: String,
    pub config_version: String,
    pub active_mod_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopOnlineModItem {
    pub workshop_id: u64,
    pub mod_name: String,
    pub package_name: String,
    pub local_time_updated: u64,
    pub remote_time_updated: u64,
    pub has_remote_update: bool,
    pub is_downloaded_to_disk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopOnlineCheckResult {
    pub total_checked: usize,
    pub pending_steam_downloads: Vec<WorkshopOnlineModItem>,
    pub ready_to_install_updates: Vec<WorkshopOnlineModItem>,
}



