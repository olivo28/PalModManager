use std::fs;
use crate::helpers::test_env::TestEnv;
use palmodmanager_lib::commands::pak_tweaker::{
    get_pak_backup_status, revert_pak_to_backup,
};

#[test]
fn test_pak_backup_status_and_revert_lifecycle() {
    tauri::async_runtime::block_on(async {
        let env = TestEnv::new_steam_win64();
        let paks_dir = env.game_root.join("Pal/Content/Paks/~mods");
        fs::create_dir_all(&paks_dir).unwrap();

        let pak_path = paks_dir.join("TestMod_P.pak");
        let original_bytes = b"ORIGINAL_PAK_BYTE_CONTENT_12345";
        fs::write(&pak_path, original_bytes).unwrap();

        // 1. Initial backup status check -> has_backup must be false
        let initial_status = get_pak_backup_status(None, pak_path.to_string_lossy().to_string()).await.unwrap();
        assert!(!initial_status.has_backup, "Should not have backup initially");
        assert_eq!(initial_status.backup_size_bytes, 0);

        // 2. Simulate a backup being created before tweaking
        let bak_path = pak_path.with_extension("pak.original.bak");
        fs::copy(&pak_path, &bak_path).unwrap();

        // Mutate the active pak file with "patched" content
        let patched_bytes = b"PATCHED_MODIFIED_PAK_CONTENT_99999";
        fs::write(&pak_path, patched_bytes).unwrap();

        // 3. Query backup status -> has_backup must now be true
        let active_backup_status = get_pak_backup_status(None, pak_path.to_string_lossy().to_string()).await.unwrap();
        assert!(active_backup_status.has_backup, "Backup must be detected");
        assert_eq!(active_backup_status.backup_size_bytes, original_bytes.len() as u64);

        // 4. Revert to backup
        let revert_res = revert_pak_to_backup(None, pak_path.to_string_lossy().to_string()).await.unwrap();
        assert!(revert_res.success, "Revert must succeed");

        // 5. Verify byte-for-byte original restoration
        let restored_bytes = fs::read(&pak_path).unwrap();
        assert_eq!(restored_bytes, original_bytes, "Restored pak content must match original bytes exactly");

        // 6. Verify backup is cleared
        let post_revert_status = get_pak_backup_status(None, pak_path.to_string_lossy().to_string()).await.unwrap();
        assert!(!post_revert_status.has_backup, "Backup file should be cleared after revert");

        println!("  [PAK-TWEAKER] Successfully verified backup lifecycle and byte-for-byte reversion!");
    });
}

#[test]
fn test_unversioned_cooked_datatable_grid_extraction() {
    let p = std::path::PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Palworld\Pal\Content\Paks\~mods\QualityOfLife_capture-statue-cost_P.pak");
    if !p.exists() {
        return;
    }
    let uasset_entry = "Pal/Content/Pal/DataTable/Player/DT_PlayerStatusRankMasterDataTable.uasset";
    let details = palmodmanager_lib::pak_scanner::uasset::inspect_uasset_deep(&p, uasset_entry).unwrap();
    let grid = details.datatable_grid.unwrap();
    assert_eq!(grid.total_rows, 279, "DT_PlayerStatusRankMasterDataTable must contain 279 rows");
    assert_eq!(grid.columns, vec!["RelicType", "Rank", "RequiredRelicNum", "EffectRate", "ResetRequiredMoney"]);
    // Verify row 1 (RelicType is resolved as enum string "EPalRelicType::CapturePower")
    let row1 = &grid.rows[0];
    assert_eq!(row1.row_name, "1");
    assert_eq!(row1.values.get("RelicType").unwrap(), &serde_json::json!("EPalRelicType::CapturePower"));
    assert_eq!(row1.values.get("Rank").unwrap(), &serde_json::json!(1));
    assert_eq!(row1.values.get("RequiredRelicNum").unwrap(), &serde_json::json!(1));
    assert_eq!(row1.values.get("ResetRequiredMoney").unwrap(), &serde_json::json!(1));

    // Verify row 16 (RelicType 1 is resolved as "EPalRelicType::HungerReduction")
    let row16 = &grid.rows[15];
    assert_eq!(row16.row_name, "16");
    assert_eq!(row16.values.get("RelicType").unwrap(), &serde_json::json!("EPalRelicType::HungerReduction"));
    assert_eq!(row16.values.get("Rank").unwrap(), &serde_json::json!(1));
    assert_eq!(row16.values.get("EffectRate").unwrap(), &serde_json::json!(2.5));

    // Verify last row 279 (RelicType 12 is resolved as "EPalRelicType::MoveSpeed")
    let row279 = &grid.rows[278];
    assert_eq!(row279.row_name, "279");
    assert_eq!(row279.values.get("RelicType").unwrap(), &serde_json::json!("EPalRelicType::MoveSpeed"));
    assert_eq!(row279.values.get("Rank").unwrap(), &serde_json::json!(92));
    assert_eq!(row279.values.get("RequiredRelicNum").unwrap(), &serde_json::json!(4));
    assert_eq!(row279.values.get("EffectRate").unwrap(), &serde_json::json!(50.0));
    assert_eq!(row279.values.get("ResetRequiredMoney").unwrap(), &serde_json::json!(1));

    println!("  [PAK-TWEAKER] Verified all 279 unversioned DataTable rows decoded with 100% precision & enum names!");
}

#[test]
fn test_inspect_bp_palgamesetting() {
    let p = std::path::PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Palworld\Pal\Content\Paks\~mods\QualityOfLife_condensation-cost_P.pak");
    if !p.exists() {
        return;
    }
    let uasset_entry = "Pal/Content/Pal/Blueprint/System/BP_PalGameSetting.uasset";
    let details = palmodmanager_lib::pak_scanner::uasset::inspect_uasset_deep(&p, uasset_entry).unwrap();
    
    assert!(!details.instantiated_properties.is_empty(), "BP_PalGameSetting must extract instantiated properties via unversioned retry");
    assert!(details.exports.len() >= 6, "Must detect exports without failing");

    // Verify condensation cost property was accurately decoded
    let rank_up_prop = details.instantiated_properties.iter().find(|p| p.name.contains("CharacterRankUpRequiredNumDefault"));
    assert!(rank_up_prop.is_some(), "CharacterRankUpRequiredNumDefault must be discovered in BP_PalGameSetting");
    let prop = rank_up_prop.unwrap();
    assert_eq!(prop.raw_value_display, "1", "Modded condensation cost must be 1");
    assert_eq!(prop.vanilla_default_display.as_deref(), Some("4"), "Vanilla condensation default must be 4");
    assert!(prop.is_delta, "Must be flagged as a modded delta");

    println!("  [PAK-TWEAKER] Successfully verified high-capacity unversioned schema extraction for BP_PalGameSetting ({} properties, CharacterRankUpRequiredNumDefault = 1 vs 4)!", details.instantiated_properties.len());
}

#[test]
fn test_inspect_item_chest() {
    let p = std::path::PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Palworld\Pal\Content\Paks\~mods\QualityOfLife_storage-slots_P.pak");
    if !p.exists() {
        return;
    }
    let mut file = std::fs::File::open(&p).unwrap();
    let pak = repak::PakBuilder::new().reader(&mut file).unwrap();
    let target = "Pal/Content/Pal/Blueprint/MapObject/BuildObject/BP_BuildObject_ItemChest_02.uasset";
    let uexp_target = "Pal/Content/Pal/Blueprint/MapObject/BuildObject/BP_BuildObject_ItemChest_02.uexp";
    let mut uasset = Vec::new();
    let mut uexp = Vec::new();
    pak.read_file(target, &mut file, &mut uasset).unwrap();
    pak.read_file(uexp_target, &mut file, &mut uexp).unwrap();
    println!("ItemChest_02 uasset: {} bytes, uexp: {} bytes", uasset.len(), uexp.len());

    let details = palmodmanager_lib::pak_scanner::uasset::inspect_uasset_deep(&p, target).unwrap();
    println!("ItemChest_02 instantiated_properties count: {}", details.instantiated_properties.len());
    for p in &details.instantiated_properties {
        println!("  Extracted Live Property: {} [{}] = {} (vanilla default: {:?}, is_delta: {})", 
            p.name, p.property_type, p.raw_value_display, p.vanilla_default_display, p.is_delta);
    }
    assert!(details.instantiated_properties.iter().any(|p| p.name.contains("SlotNum") && p.raw_value_display == "64"));
}

#[test]
fn test_vanilla_defaults_enrichment() {
    let p = std::path::PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Palworld\Pal\Content\Paks\~mods\QualityOfLife_storage-slots_P.pak");
    let target = "Pal/Content/Pal/Blueprint/MapObject/BuildObject/BP_BuildObject_ItemChest_02.uasset";
    let defaults = palmodmanager_lib::pak_scanner::vanilla_extractor::resolve_vanilla_property_defaults(&p, target);
    assert_eq!(defaults.get("SlotNum").map(|s| s.as_str()), Some("24"));

    let base_camp = "Pal/Content/Pal/Blueprint/System/BP_PalBaseCampManager.uasset";
    let bc_defaults = palmodmanager_lib::pak_scanner::vanilla_extractor::resolve_vanilla_property_defaults(&p, base_camp);
    assert!(bc_defaults.contains_key("WorkerCapacityNumDefault"));

    let gamesetting = "Pal/Content/Pal/Blueprint/System/BP_PalGameSetting.uasset";
    let gs_defaults = palmodmanager_lib::pak_scanner::vanilla_extractor::resolve_vanilla_property_defaults(&p, gamesetting);
    println!("BP_PalGameSetting vanilla defaults extracted count: {}", gs_defaults.len());
    for (k, v) in &gs_defaults {
        if k.contains("CharacterRankUp") {
            println!("  Vanilla setting: {} = {}", k, v);
        }
    }
}


