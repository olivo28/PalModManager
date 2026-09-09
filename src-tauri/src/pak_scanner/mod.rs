// Pak scanner engine module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod types;
pub mod reader;
pub mod uasset;
pub mod conflicts;
pub mod compatibility;
pub mod patch_risk;
pub mod property_extractor;
pub mod hierarchy_resolver;
pub mod vanilla_extractor;

// Re-export all types
pub use types::{
    PakModSource, PakConflict, PakInternalItem, PakInspectionResult,
    UAssetExportItem, UAssetImportItem, UAssetSummaryInfo, UAssetSchemaProperty,
    UAssetSchemaResolvedInfo, UAssetInspectionDetails, UAssetLiveProperty,
    UAssetDataTableGrid, UAssetDataTableRow,
    GamePassPakNotice, DeprecatedSchemaNotice, PatchRiskNotice,
};

// Re-export reader & classifiers
pub use reader::{
    list_pak_entries, extract_pak_entry,
    list_pak_entries_detailed, classify_asset_type,
};

// Re-export UAsset inspectors & schema resolver
pub use uasset::{
    resolve_usmap_schema, inspect_uasset_deep,
    inspect_uasset_from_pak,
};

// Re-export conflict detection
pub use conflicts::scan_pak_conflicts;

// Re-export compatibility checkers
pub use compatibility::{
    check_gamepass_pak_compatibility,
    check_mod_schema_compatibility,
};

pub use patch_risk::check_patch_risk_compatibility;
