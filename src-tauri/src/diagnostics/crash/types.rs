use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrashMemoryStats {
    pub total_physical_gb: Option<f64>,
    pub available_physical_gb: Option<f64>,
    pub total_virtual_gb: Option<f64>,
    pub available_virtual_gb: Option<f64>,
    pub is_oom_risk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrashCallstackFrame {
    pub raw_symbol: String,
    pub module_name: Option<String>,
    pub function_name: Option<String>,
    pub offset: Option<u64>,
    pub is_mod_related: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FaultingModuleInfo {
    pub name: String,
    pub full_path: String,
    pub base_address: u64,
    pub size: u64,
    pub rip_offset: u64,
    pub category: String, // "mod_pak", "ue4ss", "palschema", "overlay", "directx", "vanilla", "unknown"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrashContextXml {
    pub error_message: String,
    pub crash_type: String,
    pub seconds_since_start: u64,
    pub crashed_thread_name: Option<String>,
    pub crashed_thread_callstack: Option<String>,
    pub callstack_raw: String,
    pub memory_stats: CrashMemoryStats,
    pub loaded_modules: Vec<String>,
    pub game_version: Option<String>,
    pub engine_version: Option<String>,
    pub raw_xml_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RawDmpModule {
    pub name: String,
    pub full_path: String,
    pub base_address_hex: String,
    pub size_bytes: u32,
    pub is_faulting: bool,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MinidumpInfo {
    pub exception_code: u32,
    pub exception_code_hex: String,
    pub exception_meaning: String,
    pub faulting_address: u64,
    pub faulting_address_hex: String,
    pub thread_id: u32,
    pub faulting_module: Option<FaultingModuleInfo>,
    pub total_modules_count: usize,
    pub raw_modules: Vec<RawDmpModule>,
    pub raw_dump_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveModSnapshot {
    pub id: String,
    pub name: String,
    pub version: String,
    pub mod_type: String,
    pub author: Option<String>,
    pub nexus_id: Option<u32>,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrashActionSuggestion {
    pub id: String,
    pub label_key: String,
    pub description_key: String,
    pub action_type: String, // "disable_mod", "open_folder", "purge_reshade", "update_ue4ss", "none"
    pub target_id: Option<String>,
    pub target_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedCrashReport {
    pub id: String,
    pub timestamp: String,
    pub crash_folder: String,
    pub has_xml: bool,
    pub has_dmp: bool,
    pub has_ue4ss_log: bool,
    
    // Diagnóstico
    pub severity: String, // "Critical", "Warning", "Fatal"
    pub primary_cause: String, // "mod_pak_corrupt", "ue4ss_script", "palschema_outdated", "overlay_conflict", "directx_error", "out_of_memory", "early_startup", "game_engine"
    pub culprit_name: Option<String>,
    pub culprit_type: Option<String>,
    pub human_summary: String,
    pub technical_details: String,
    pub origin_category: String, // "pak_mod", "ue4ss", "game_engine", "out_of_memory", "directx", "overlay"
    
    // Componentes del diagnóstico
    pub xml_context: Option<CrashContextXml>,
    pub minidump: Option<MinidumpInfo>,
    pub recent_log_errors: Vec<String>,
    pub active_mods: Vec<ActiveModSnapshot>,
    pub suggestions: Vec<CrashActionSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SupportBundleResult {
    pub success: bool,
    pub bundle_path: String,
    pub file_size_bytes: u64,
    pub bundled_files: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrashHistoryItem {
    pub id: String,
    pub folder_path: String,
    pub timestamp: String,
    pub has_xml: bool,
    pub has_dmp: bool,
    pub folder_name: String,
    pub origin: String, // "pak_mod", "ue4ss", "game_engine", "out_of_memory", "directx", "overlay", "unknown"
}
