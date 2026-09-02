// Scanner module — barrel re-exports all public commands and types
#![allow(unused_imports)]
pub mod types;
pub mod utils;
pub mod conflicts;
pub mod hook_validator;
pub mod palschema;
pub mod hotkeys;
pub mod pak_inspector;
pub mod gamepass;
pub mod saves;
pub mod patch_builder;

// Re-export types used externally
pub use types::{
    ConflictingMod, TableRowConflict, HookConflict, ModSummary,
    UsmapHookDiagnostic, PalschemaTableEntry, UsmapDiagnosticSummary, ScanResult,
};

// Re-export public validator functions called from conflicts.rs
pub use hook_validator::validate_hook_with_usmap;
pub use palschema::validate_palschema_table_with_usmap;

// Re-export utility (used by editor_commands and others)
pub use utils::strip_jsonc_comments;

// Re-export Tauri commands
pub use conflicts::scan_conflicts;
pub use hotkeys::{ModHotkey, scan_mod_hotkeys, update_mod_hotkey};
pub use pak_inspector::{inspect_pak_file_tree, inspect_mod_pak_contents, inspect_pak_asset, inspect_uasset_deep_cmd, decode_uasset_texture_cmd};
pub use gamepass::{convert_mod_to_gamepass, convert_all_gamepass_mods};
pub use saves::{
    list_save_worlds_cmd, deep_scan_save_cmd, repair_save_cmd, restore_save_backup_cmd,
    create_world_backup_cmd, list_pmm_world_backups_cmd, restore_pmm_world_backup_cmd,
    delete_pmm_world_backup_cmd, open_pmm_world_backups_folder_cmd, open_world_folder_cmd,
    export_world_zip_cmd, prune_world_backups_cmd, save_world_custom_meta_cmd,
    get_world_custom_meta_cmd, inspect_snapshot_details_cmd,
};
pub use patch_builder::{build_compatibility_pak_cmd, list_generated_patches_cmd, delete_generated_patch_cmd};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_inspect_pak_from_rar() {
        let rar_file = "C:/Users/Antikux/Downloads/Modern Wooden Building - Steam Version(Emprxss) 5366 2 2026-08-27T18-44Z mu9Q1NbQr.rar";
        if !std::path::Path::new(rar_file).exists() {
            return;
        }
        let res = super::inspect_pak_file_tree("Emprxss_Wooden_Modern_Building_P.pak".to_string(), Some(rar_file.to_string()));
        println!("RAR Inspection result: {:?}", res);
        assert!(res.is_ok(), "Should inspect pak inside RAR");
        let inspection = res.unwrap();
        assert!(!inspection.files.is_empty(), "Files should not be empty");

        // Inspect uasset inside RAR pak
        let temp_dir = std::env::temp_dir().join("pmm_uasset_inspect_test");
        let _ = fs::create_dir_all(&temp_dir);
        let ext_res = crate::zip_handler::extract_zip_to_temp(rar_file, &temp_dir).expect("Should extract rar");
        let pak_path = ext_res.join("Pal/Content/Paks/~mods/Emprxss_Wooden_Modern_Building_P.pak");
        let pak_real = if pak_path.exists() {
            pak_path
        } else {
            walkdir::WalkDir::new(&temp_dir)
                .into_iter()
                .flatten()
                .find(|e| e.path().extension().map_or(false, |ext| ext == "pak"))
                .map(|e| e.path().to_path_buf())
                .unwrap()
        };

        let uasset_details = crate::pak_scanner::inspect_uasset_deep(&pak_real, "Model/Prop/Architecture/Architecture_Wood/Material/MI_PalProp_DoorBase_Wood.uasset");
        println!("Uasset details: {:?}", uasset_details);
        assert!(uasset_details.is_ok(), "Should parse uasset inside pak");
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
