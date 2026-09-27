use palmodmanager_lib::commands::editor::{find_hook_start, extract_string_literal};

#[test]
fn test_find_hook_start_and_extract_string_literal() {
    let line1 = r#"local ok, err = pcall(RegisterHook, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter", function(self) end)"#;
    let start1 = find_hook_start(line1, 0).expect("Should find hook start");
    let (target1, _, _) = extract_string_literal(line1, start1).expect("Should extract string literal");
    assert_eq!(target1, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter");

    let line2 = r#"RegisterHook('/Script/Engine.Actor:K2_DestroyActor', callback)"#;
    let start2 = find_hook_start(line2, 0).expect("Should find hook start");
    let (target2, _, _) = extract_string_literal(line2, start2).expect("Should extract string literal");
    assert_eq!(target2, "/Script/Engine.Actor:K2_DestroyActor");

    let line3 = r#"xpcall(RegisterHook, debug.traceback, "/Script/Pal.PalCharacter:Die", on_die)"#;
    let start3 = find_hook_start(line3, 0).expect("Should find hook start");
    let (target3, _, _) = extract_string_literal(line3, start3).expect("Should extract string literal");
    assert_eq!(target3, "/Script/Pal.PalCharacter:Die");
}

#[test]
fn test_map_object_validation_vanilla_recognition() {
    use palmodmanager_lib::usmap::get_or_load_datatable_index;
    if let Some(dt) = get_or_load_datatable_index("") {
        let build_table = dt.find_table("DT_BuildObjectDataTable")
            .or_else(|| dt.find_table("DT_BuildObjectDataTable_Common"));
        let map_table = dt.find_table("DT_MapObjectMasterDataTable")
            .or_else(|| dt.find_table("DT_MapObjectMasterDataTable_Common"));

        assert!(build_table.is_some(), "DT_BuildObjectDataTable should be found");
        assert!(map_table.is_some(), "DT_MapObjectMasterDataTable should be found");

        let b = build_table.unwrap();
        let m = map_table.unwrap();

        assert!(b.rows.iter().any(|r| r.eq_ignore_ascii_case("Wooden_wall")), "Wooden_wall must be in DT_BuildObjectDataTable");
        assert!(m.rows.iter().any(|r| r.eq_ignore_ascii_case("TreasureBox_FishingJunk_RequiredLongHold")), "TreasureBox_FishingJunk_RequiredLongHold must be in DT_MapObjectMasterDataTable");
        assert!(!b.rows.iter().any(|r| r.eq_ignore_ascii_case("EPK_GachaPC")), "EPK_GachaPC must not be in build table");
        assert!(!m.rows.iter().any(|r| r.eq_ignore_ascii_case("EPK_GachaPC")), "EPK_GachaPC must not be in map table");
    }
}

