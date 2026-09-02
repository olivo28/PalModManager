// Shared types used across all scanner sub-modules
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConflictingMod {
    pub mod_id: String,
    pub mod_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub detail: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TableRowConflict {
    pub table_name: String,
    pub row_name: String,
    pub mods: Vec<ConflictingMod>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HookConflict {
    pub hook_target: String,
    pub hook_fn: String,
    pub mods: Vec<ConflictingMod>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModSummary {
    pub mod_id: String,
    pub mod_name: String,
    pub mod_type: String,
    pub palschema_rows: Vec<String>,
    pub ue4ss_hooks: Vec<String>,
    pub pak_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsmapHookDiagnostic {
    pub mod_id: String,
    pub mod_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub hook_target: String,
    pub target_class: String,
    pub target_function: String,
    pub status: String,
    pub reason: String,
    pub suggestion: Option<String>,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct PalschemaTableEntry {
    pub mod_id: String,
    pub mod_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub table_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsmapDiagnosticSummary {
    pub has_usmap: bool,
    pub usmap_version: String,
    pub total_hooks_checked: u32,
    pub valid_hooks: u32,
    pub broken_hooks: u32,
    pub diagnostics: Vec<UsmapHookDiagnostic>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub total_scanned: u32,
    pub palschema_scanned: u32,
    pub ue4ss_scanned: u32,
    pub pak_scanned: u32,
    pub table_conflicts: Vec<TableRowConflict>,
    pub hook_conflicts: Vec<HookConflict>,
    pub pak_conflicts: Vec<crate::pak_scanner::PakConflict>,
    pub internal_table_conflicts: Vec<TableRowConflict>,
    pub internal_hook_conflicts: Vec<HookConflict>,
    pub warnings: Vec<String>,
    pub mod_summaries: Vec<ModSummary>,
    pub gamepass_notices: Vec<crate::pak_scanner::GamePassPakNotice>,
    pub schema_notices: Vec<crate::pak_scanner::DeprecatedSchemaNotice>,
    pub usmap_diagnostics: Option<UsmapDiagnosticSummary>,
    pub is_gamepass: bool,
}

pub type TableMap = HashMap<String, Vec<ConflictingMod>>;
pub type HookMap = HashMap<String, (String, Vec<ConflictingMod>)>;
