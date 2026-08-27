use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBackupSnapshot {
    pub slot_name: String,
    pub timestamp: String,
    pub level_size_bytes: u64,
    pub uncompressed_size_bytes: Option<u64>,
    pub local_data_exists: bool,
    pub in_game_day: Option<u32>,
    pub player_level: Option<u32>,
    pub host_player_name: Option<String>,
    pub mod_refs_count: Option<usize>,
    pub is_clean_vanilla: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalEditDiagnostic {
    pub is_modified: bool,
    pub tool_name: Option<String>,
    pub details: String,
    pub editor_backup_count: usize,
    pub current_size_bytes: u64,
    pub latest_backup_size_bytes: u64,
    pub size_reduction_pct: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldOptionSettings {
    pub exists: bool,
    pub difficulty: Option<String>,
    pub day_time_speed_rate: Option<f32>,
    pub night_time_speed_rate: Option<f32>,
    pub exp_rate: Option<f32>,
    pub pal_capture_rate: Option<f32>,
    pub pal_spawn_num_rate: Option<f32>,
    pub pal_damage_rate: Option<f32>,
    pub player_damage_rate: Option<f32>,
    pub player_stomach_decrease_rate: Option<f32>,
    pub player_stamina_decrease_rate: Option<f32>,
    pub player_auto_hp_regene_rate: Option<f32>,
    pub player_auto_hp_regene_rate_in_sleeping: Option<f32>,
    pub pal_stomach_decrease_rate: Option<f32>,
    pub pal_stamina_decrease_rate: Option<f32>,
    pub pal_auto_hp_regene_rate: Option<f32>,
    pub pal_auto_hp_regene_rate_in_sleeping: Option<f32>,
    pub build_object_damage_rate: Option<f32>,
    pub build_object_deterioration_damage_rate: Option<f32>,
    pub collection_drop_rate: Option<f32>,
    pub collection_object_hp_rate: Option<f32>,
    pub collection_object_respawn_speed_rate: Option<f32>,
    pub enemy_drop_item_rate: Option<f32>,
    pub death_penalty: Option<String>,
    pub enable_player_to_player_damage: Option<bool>,
    pub enable_friendly_fire: Option<bool>,
    pub enable_invader_enemy: Option<bool>,
    pub active_unko: Option<bool>,
    pub drop_item_max_num: Option<i32>,
    pub base_camp_max_num: Option<i32>,
    pub base_camp_worker_max_num: Option<i32>,
    pub drop_item_alive_max_hours: Option<f32>,
    pub guild_player_max_num: Option<i32>,
    pub pal_egg_hatching_hours: Option<f32>,
    pub work_speed_rate: Option<f32>,
    pub is_multiplay: Option<bool>,
    pub is_pvp: Option<bool>,
    pub can_pickup_other_guild_death_penalty_drop: Option<bool>,
    pub enable_non_login_penalty: Option<bool>,
    pub enable_fast_travel: Option<bool>,
    pub is_start_location_select_by_map: Option<bool>,
    pub exist_player_after_logout: Option<bool>,
    pub supply_drop_span: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSaveInfo {
    pub player_uid: String,
    pub player_name: Option<String>,
    pub player_level: Option<u32>,
    pub is_host: bool,
    pub file_size_bytes: u64,
    pub last_played_date: Option<String>,
    pub is_corrupt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStorageBreakdown {
    pub level_sav_bytes: u64,
    pub players_dir_bytes: u64,
    pub backups_dir_bytes: u64,
    pub total_world_bytes: u64,
    pub uncompressed_level_bytes: u64,
    pub compression_ratio_pct: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldCustomMeta {
    pub nickname: Option<String>,
    pub notes: Option<String>,
    pub bound_profile_id: Option<String>,
    pub bound_profile_name: Option<String>,
    pub tags: Vec<String>,
    pub pre_launch_backup_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveWorldSummary {
    pub world_id: String,
    pub world_name: String,
    pub world_dir: String,
    pub host_player_name: Option<String>,
    pub host_player_uid: Option<String>,
    pub player_level: Option<u32>,
    pub in_game_day: Option<u32>,
    pub save_date: Option<String>,
    pub level_size_bytes: u64,
    pub player_count: usize,
    pub backup_count: usize,
    pub latest_backup_date: Option<String>,
    pub has_external_edits: bool,
    pub health_status: String, // "healthy", "warning", "corrupt"
    pub detected_issues_count: usize,
    pub custom_meta: Option<WorldCustomMeta>,
    pub world_options: Option<WorldOptionSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanedModRef {
    pub mod_hint_name: String,
    pub asset_path: String,
    pub occurrences: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveHealthReport {
    pub world_id: String,
    pub world_name: String,
    pub level_sav_path: String,
    pub host_player_name: Option<String>,
    pub host_player_uid: Option<String>,
    pub player_level: Option<u32>,
    pub in_game_day: Option<u32>,
    pub is_valid_gvas: bool,
    pub compression_type: String, // "Oodle PlM", "Zlib PlZ", "Raw GVAS", "Unknown"
    pub uncompressed_size: u64,
    pub health_status: String, // "healthy", "warning", "corrupt"
    pub summary_message: String,
    pub orphaned_mod_refs: Vec<OrphanedModRef>,
    pub raw_mod_paths_found: Vec<String>,
    pub total_mod_references: usize,
    pub backup_count: usize,
    pub latest_backup_date: Option<String>,
    pub available_backups: Vec<SaveBackupSnapshot>,
    pub has_external_edits: bool,
    pub external_edit_details: Option<ExternalEditDiagnostic>,
    pub can_repair: bool,
    pub can_restore_backup: bool,
    pub world_options: Option<WorldOptionSettings>,
    pub player_roster: Vec<PlayerSaveInfo>,
    pub storage_breakdown: Option<SaveStorageBreakdown>,
    pub custom_meta: Option<WorldCustomMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRepairResult {
    pub success: bool,
    pub backup_zip_path: String,
    pub sanitized_refs_count: usize,
    pub message: String,
}
