use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakModSource {
    pub mod_id: String,
    pub mod_name: String,
    pub pak_filename: String,
    pub pak_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakConflict {
    pub internal_path: String,
    pub asset_name: String,
    pub asset_type: String, // "DataTable", "Blueprint", "Texture", "Mesh", "Asset"
    pub mods: Vec<PakModSource>,
    #[serde(default)]
    pub resolved_by_patch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakInternalItem {
    pub path: String,
    pub name: String,
    pub asset_type: String, // "DataTable", "Blueprint", "Texture", "Mesh", "Audio", "Asset"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakInspectionResult {
    pub pak_name: String,
    pub total_files: usize,
    pub files: Vec<PakInternalItem>,
    pub summary_by_type: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetExportItem {
    pub object_name: String,
    pub class_name: String,
    pub outer_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetImportItem {
    pub object_name: String,
    pub class_name: String,
    pub class_package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetSummaryInfo {
    pub uasset_size_bytes: usize,
    pub uexp_size_bytes: Option<usize>,
    pub export_count: usize,
    pub import_count: usize,
    pub name_count: usize,
    pub package_flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetSchemaProperty {
    pub name: String,
    pub type_name: String,
    pub struct_type: Option<String>,
    pub enum_type: Option<String>,
    pub inner_type: Option<String>,
    pub array_dim: u8,
    pub index: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetSchemaResolvedInfo {
    pub matched_struct_name: String,
    pub super_type: Option<String>,
    pub properties: Vec<UAssetSchemaProperty>,
    pub total_properties: usize,
    pub game_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetLiveProperty {
    pub export_index: usize,
    pub name: String,
    pub property_type: String,
    pub value: serde_json::Value,
    pub raw_value_display: String,
    pub is_editable: bool,
    #[serde(default)]
    pub struct_type: Option<String>,
    #[serde(default)]
    pub enum_value: Option<String>,
    #[serde(default)]
    pub vanilla_default_display: Option<String>,
    #[serde(default)]
    pub is_delta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetDataTableRow {
    pub row_name: String,
    pub values: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetDataTableGrid {
    pub row_struct_name: String,
    pub columns: Vec<String>,
    pub rows: Vec<UAssetDataTableRow>,
    pub total_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UAssetInspectionDetails {
    pub asset_name: String,
    pub asset_path: String,
    pub asset_type: String,
    pub engine_version: String,
    pub summary: UAssetSummaryInfo,
    pub exports: Vec<UAssetExportItem>,
    pub imports: Vec<UAssetImportItem>,
    pub names_sample: Vec<String>,
    pub resolved_schema: Option<UAssetSchemaResolvedInfo>,
    pub texture_preview: Option<crate::texture_decoder::TexturePreviewInfo>,
    #[serde(default)]
    pub instantiated_properties: Vec<UAssetLiveProperty>,
    #[serde(default)]
    pub datatable_grid: Option<UAssetDataTableGrid>,
    #[serde(default)]
    pub class_hierarchy: Vec<String>,
    #[serde(default)]
    pub vanilla_verification: HashMap<String, bool>,
    #[serde(default)]
    pub has_original_backup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GamePassPakNotice {
    pub mod_id: String,
    pub mod_name: String,
    pub pak_filename: String,
    pub pak_path: String,
    pub missing_containers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeprecatedSchemaNotice {
    pub mod_id: String,
    pub mod_name: String,
    pub asset_path: String,
    pub struct_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchRiskNotice {
    pub mod_id: String,
    pub mod_name: String,
    pub pak_filename: String,
    pub asset_path: String,
    pub asset_category: String,
    pub risk_level: String,
    pub reason: String,
}
