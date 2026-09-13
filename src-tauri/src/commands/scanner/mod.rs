// Scanner module — barrel re-exports all public commands and types
#![allow(unused_imports)]
pub mod types;
pub mod utils;
pub mod conflicts;
pub mod conflict_parsers;
pub mod hook_validator;
pub mod palschema;
pub mod hotkeys;
pub mod pak_inspector;
pub mod gamepass;
pub mod saves;
pub mod patch_builder;
pub mod ue4ss_log;

// Re-export types used externally
pub use types::{
    ConflictingMod, TableRowConflict, HookConflict, ModSummary,
    UsmapHookDiagnostic, PalschemaTableEntry, UsmapDiagnosticSummary, ScanResult,
};
pub use ue4ss_log::{Ue4ssLogDiagnostics, Ue4ssLogEntry, Ue4ssModStatus};

// Re-export public validator functions called from conflicts.rs
pub use hook_validator::validate_hook_with_usmap;
pub use palschema::validate_palschema_table_with_usmap;

// Re-export utility (used by editor_commands and others)
pub use utils::strip_jsonc_comments;

// Re-export Tauri commands
pub use conflicts::scan_conflicts;
pub use hotkeys::{ModHotkey, scan_mod_hotkeys, update_mod_hotkey, find_variable_in_lua_files, find_table_array_keys};
pub use pak_inspector::{inspect_pak_file_tree, inspect_mod_pak_contents, inspect_pak_asset, inspect_uasset_deep_cmd, decode_uasset_texture_cmd, gather_candidate_paks};
pub use gamepass::{convert_mod_to_gamepass, convert_all_gamepass_mods};
pub use saves::{
    list_save_worlds_cmd, deep_scan_save_cmd, repair_save_cmd, restore_save_backup_cmd,
    create_world_backup_cmd, list_pmm_world_backups_cmd, restore_pmm_world_backup_cmd,
    delete_pmm_world_backup_cmd, open_pmm_world_backups_folder_cmd, open_world_folder_cmd,
    export_world_zip_cmd, prune_world_backups_cmd, save_world_custom_meta_cmd,
    get_world_custom_meta_cmd, inspect_snapshot_details_cmd,
};
pub use patch_builder::{build_compatibility_pak_cmd, list_generated_patches_cmd, delete_generated_patch_cmd};
pub use ue4ss_log::{get_ue4ss_log_diagnostics, read_raw_ue4ss_log, clear_ue4ss_log, copy_ue4ss_log_file_to_clipboard, reveal_ue4ss_log_in_explorer};
