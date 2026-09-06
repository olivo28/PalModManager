use std::fs;
use crate::helpers::test_env::TestEnv;
use palmodmanager_lib::commands::scanner::gather_candidate_paks;
use palmodmanager_lib::models::ModInfo;

#[test]
fn test_gather_candidate_paks_resolves_multiple_game_paks_and_deduplicates() {
    let env = TestEnv::new_steam_win64();

    // 1. Create real dummy PAK files in game directory
    let mods_dir = env.game_root.join("Pal/Content/Paks/~mods");
    let logicmods_dir = env.game_root.join("Pal/Content/Paks/LogicMods");
    fs::create_dir_all(&mods_dir).unwrap();
    fs::create_dir_all(&logicmods_dir).unwrap();

    let primary_pak = mods_dir.join("PalVariety_Core_P.pak");
    let addon_pak1 = mods_dir.join("PalVariety_ShinyAddon_P.pak");
    let logic_pak = logicmods_dir.join("PalVariety_Logic_P.pak");

    fs::write(&primary_pak, b"DUMMY_PAK_1").unwrap();
    fs::write(&addon_pak1, b"DUMMY_PAK_2").unwrap();
    fs::write(&logic_pak, b"DUMMY_PAK_3").unwrap();

    // 2. Setup ModInfo with primary pak, extra files, duplicate entry, and non-pak entry
    let mod_info: ModInfo = serde_json::from_value(serde_json::json!({
        "id": "mod-variety-multi",
        "name": "PalVariety Multi-PAK Suite",
        "type": "hybrid",
        "version": "1.0.0",
        "installDate": "2026-09-05",
        "sourceZip": "PalVarietySuite.zip",
        "enabled": true,
        "gamePath": primary_pak.to_string_lossy().to_string(),
        "disabledPath": "",
        "hasEnabledTxt": false,
        "extraFiles": [
            // Relative path to addon
            "Pal/Content/Paks/~mods/PalVariety_ShinyAddon_P.pak",
            // Direct path to logic pak
            logic_pak.to_string_lossy().to_string(),
            // Duplicate reference to primary pak (should be deduplicated)
            primary_pak.to_string_lossy().to_string(),
            // Non-pak file (should be ignored by gather_candidate_paks)
            "Pal/Binaries/Win64/ue4ss/Mods/PalVariety/config.lua",
            // Non-existent pak (should be filtered out because file doesn't exist on disk)
            "Pal/Content/Paks/~mods/NonExistent_P.pak"
        ]
    })).unwrap();

    let candidates = gather_candidate_paks(&mod_info, env.game_root.to_str().unwrap());

    // 3. Verify exactly 3 existing unique paks are gathered
    assert_eq!(candidates.len(), 3, "Must gather exactly 3 unique candidate PAKs on disk");
    assert!(candidates.contains(&primary_pak), "Primary PAK must be gathered");
    assert!(candidates.contains(&addon_pak1), "Extra Addon PAK must be gathered");
    assert!(candidates.contains(&logic_pak), "LogicMods PAK must be gathered");

    println!("  [MULTI-PAK] Discovered and deduplicated {} unique PAKs on disk:", candidates.len());
    for p in &candidates {
        println!("              * Candidate: {:?}", p.file_name().unwrap_or_default());
    }
}

#[test]
fn test_gather_candidate_paks_empty_when_no_paks_exist() {
    let env = TestEnv::new_steam_win64();

    let mod_info: ModInfo = serde_json::from_value(serde_json::json!({
        "id": "mod-pure-lua",
        "name": "Pure Lua Mod",
        "type": "ue4ss",
        "version": "1.0.0",
        "installDate": "2026-09-05",
        "sourceZip": "PureLua.zip",
        "enabled": true,
        "gamePath": env.game_root.join("Pal/Binaries/Win64/ue4ss/Mods/PureLua").to_string_lossy().to_string(),
        "disabledPath": "",
        "hasEnabledTxt": false,
        "extraFiles": [
            "Pal/Binaries/Win64/ue4ss/Mods/PureLua/Scripts/main.lua",
            "Pal/Binaries/Win64/ue4ss/Mods/PureLua/enabled.txt"
        ]
    })).unwrap();

    let candidates = gather_candidate_paks(&mod_info, env.game_root.to_str().unwrap());
    assert!(candidates.is_empty(), "Pure Lua mod should yield 0 candidate PAKs");

    println!("  [MULTI-PAK] Pure Lua mod correctly yielded 0 candidate PAKs");
}
