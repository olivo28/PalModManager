use palmodmanager_lib::commands::scanner::conflicts::extract_literal_hooks;
use palmodmanager_lib::commands::scanner::inspect_pak_file_tree;

#[test]
fn test_extract_literal_hooks_pcall_and_xpcall() {
    let lua_code = r#"
-- Direct call
RegisterHook("/Script/Pal.PalBullet:OnHitToActor", function(self) end)

-- Pcall reference
local ok, err = pcall(RegisterHook, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter", function(self) end)

-- Single quotes
local ok2, err2 = pcall(RegisterHook, '/Script/Engine.Actor:K2_DestroyActor', callback)

-- NotifyOnNewObject via pcall
pcall(NotifyOnNewObject, "/Script/Pal.PalPlayerCharacter", function(self) end)

-- Xpcall
xpcall(RegisterHook, debug.traceback, "/Script/Pal.PalCharacter:Die", on_die)

-- Multiline pcall
pcall(
    RegisterHook,
    "/Script/Pal.PalDamageSubsystem:ApplyDamage",
    function(self) end
)
"#;
    let hooks = extract_literal_hooks(lua_code);
    let targets: Vec<String> = hooks.into_iter().map(|(_, t, _, _)| t).collect();

    assert_eq!(targets.len(), 6);
    assert!(targets.contains(&"/Script/Pal.PalBullet:OnHitToActor".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalPlayerController:RequestUseItemToCharacter".to_string()));
    assert!(targets.contains(&"/Script/Engine.Actor:K2_DestroyActor".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalPlayerCharacter".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalCharacter:Die".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalDamageSubsystem:ApplyDamage".to_string()));
}

#[test]
fn test_inspect_pak_from_archive_not_found() {
    tauri::async_runtime::block_on(async {
        let env = crate::helpers::TestEnv::new_steam_win64();
        let zip_path = crate::helpers::ZipBuilder::new("EmptyMod.zip")
            .add_text_file("dummy.txt", "hello")
            .build_in(&env.temp_dir);

        let res = inspect_pak_file_tree("NonExistent_P.pak".to_string(), Some(zip_path.to_str().unwrap().to_string())).await;
        assert!(res.is_err(), "Should return error when pak is not in archive");
        assert!(res.err().unwrap().contains("not found inside archive"));
    });
}

#[test]
fn test_lua_hotkey_table_and_loop_resolution() {
    use std::path::PathBuf;
    use palmodmanager_lib::commands::scanner::{find_variable_in_lua_files, find_table_array_keys};

    let config_lua = r#"
local Config = {
    Pal1 = {
        Key = Key.K,
    },
    Pal2 = {
        Key = Key.L,
    },
    Pals = {
        { Name = "PalA", Key = Key.O },
        { Name = "PalB", Key = Key.P },
        { Name = "PalC", Key = Key.I },
    }
}
return Config
"#;
    let fake_path = PathBuf::from("scripts/config.lua");
    let lua_files = vec![(fake_path.clone(), config_lua.to_string())];

    // 1. Scoped table variables: Pal1.Key vs Pal2.Key must resolve to different keys and lines
    let pal1_res = find_variable_in_lua_files("Pal1.Key", &lua_files, 0);
    assert!(pal1_res.is_some(), "Pal1.Key should be found");
    let (pal1_key, _, pal1_line) = pal1_res.unwrap();
    assert_eq!(pal1_key, "Key.K");

    let pal2_res = find_variable_in_lua_files("Pal2.Key", &lua_files, 0);
    assert!(pal2_res.is_some(), "Pal2.Key should be found");
    let (pal2_key, _, pal2_line) = pal2_res.unwrap();
    assert_eq!(pal2_key, "Key.L");

    assert_ne!(pal1_line, pal2_line, "Pal1 and Pal2 must not have identical line numbers");
    assert_ne!(pal1_key, pal2_key, "Pal1 and Pal2 must resolve to distinct keys");

    // 2. Array table resolution: find_table_array_keys for Pals must return all 3 entries
    let array_keys = find_table_array_keys("Config.Pals", "Key", &lua_files);
    assert_eq!(array_keys.len(), 3, "Should find 3 distinct keys inside Pals array");
    assert_eq!(array_keys[0].0, "Key.O");
    assert_eq!(array_keys[1].0, "Key.P");
    assert_eq!(array_keys[2].0, "Key.I");
    assert_eq!(array_keys[0].3, "Pals[1].Key");
    assert_eq!(array_keys[1].3, "Pals[2].Key");
    assert_eq!(array_keys[2].3, "Pals[3].Key");
}

#[test]
fn test_fstring_asset_path_extraction_and_mod_filtering() {
    use palmodmanager_lib::save_scanner::{
        extract_fstring_asset_paths, is_mod_asset_path, extract_mod_hint,
    };

    // 1. Validate mod asset path qualification
    assert!(is_mod_asset_path("/Game/Mods/AntiPhat/BP_AntiPhat.BP_AntiPhat_C"));
    assert!(is_mod_asset_path("/Game/Mods/Valdacil_Mod/Actors/Obj"));
    assert!(is_mod_asset_path("/Game/CustomMod/MyActor"));
    assert!(is_mod_asset_path("/Script/CustomModScript.MyActor"));

    // Vanilla paths must be rejected
    assert!(!is_mod_asset_path("/Game/Pal/Character/Pals/Anubis"));
    assert!(!is_mod_asset_path("/Game/Pal/Blueprint/Pals/BP_Pal"));
    assert!(!is_mod_asset_path("/Game/Characters/Player/Meshes/SK_Player"));
    assert!(!is_mod_asset_path("/Game/Maps/WorldMap/Levels/L_Main"));
    assert!(!is_mod_asset_path("/Game/Sound/BGM/Pal_Battle"));
    assert!(!is_mod_asset_path("/Script/Pal.PalPlayerCharacter"));
    assert!(!is_mod_asset_path("/Script/Engine.Actor"));
    assert!(!is_mod_asset_path("/Script/CoreUObject.Object"));

    // 2. Validate mod hint name extraction
    assert_eq!(extract_mod_hint("/Game/Mods/AntiPhat/BP_AntiPhat.BP_AntiPhat_C"), "AntiPhat");
    assert_eq!(extract_mod_hint("/Game/Mods/Valdacil_Mod/Actors/Obj"), "Valdacil_Mod");
    assert_eq!(extract_mod_hint("/Game/DuckPal/Meshes/SM_Duck"), "DuckPal");
    assert_eq!(extract_mod_hint("/Script/CustomMod.ActorClass"), "CustomMod");

    // 3. Validate binary FString extraction (simulated GVAS binary payload)
    let mut payload: Vec<u8> = Vec::new();
    // Prefix dummy binary bytes
    payload.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);

    // Construct ASCII FString: [len: i32 LE][bytes with \0]
    let path_ascii = "/Game/Mods/AntiPhat/BP_AntiPhat.BP_AntiPhat_C";
    let ascii_bytes = path_ascii.as_bytes();
    let ascii_len = (ascii_bytes.len() + 1) as i32; // includes null terminator
    payload.extend_from_slice(&ascii_len.to_le_bytes());
    payload.extend_from_slice(ascii_bytes);
    payload.push(0); // null terminator

    // More dummy binary bytes
    payload.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

    // Construct UTF-16LE FString: [len: negative i32 LE][utf16 bytes with \0]
    let path_utf16 = "/Game/Mods/Valdacil_Mod/Actors/Obj";
    let u16_chars: Vec<u16> = path_utf16.encode_utf16().collect();
    let u16_len = -((u16_chars.len() + 1) as i32); // negative indicates UTF-16
    payload.extend_from_slice(&u16_len.to_le_bytes());
    for c in &u16_chars {
        payload.extend_from_slice(&c.to_le_bytes());
    }
    payload.extend_from_slice(&0u16.to_le_bytes()); // null terminator

    // Add vanilla path ASCII FString
    let path_vanilla = "/Game/Pal/Character/Pals/Anubis";
    let vanilla_bytes = path_vanilla.as_bytes();
    let vanilla_len = (vanilla_bytes.len() + 1) as i32;
    payload.extend_from_slice(&vanilla_len.to_le_bytes());
    payload.extend_from_slice(vanilla_bytes);
    payload.push(0);

    // Extract all paths using the binary walker
    let extracted = extract_fstring_asset_paths(&payload);
    assert!(extracted.contains(&path_ascii.to_string()), "Must extract ASCII FString");
    assert!(extracted.contains(&path_utf16.to_string()), "Must extract UTF-16LE FString");
    assert!(extracted.contains(&path_vanilla.to_string()), "Must extract vanilla FString");

    // Filter by mod asset paths
    let mod_paths: Vec<String> = extracted.into_iter().filter(|p| is_mod_asset_path(p)).collect();
    assert_eq!(mod_paths.len(), 2, "Must filter out the vanilla path and keep both mod paths");
    assert!(mod_paths.contains(&path_ascii.to_string()));
    assert!(mod_paths.contains(&path_utf16.to_string()));
}

#[test]
fn test_world_option_guild_bases_and_meta() {
    use palmodmanager_lib::save_scanner::gvas::{gvas_read_int, gvas_read_str};

    // Construct mock GVAS bytes containing BaseCampMaxNum = 128 and BaseCampMaxNumInGuild = 15
    let mut data = Vec::new();
    data.extend_from_slice(b"GVAS");

    fn add_int_prop(data: &mut Vec<u8>, name: &str, val: i32) {
        let name_len = (name.len() + 1) as i32;
        data.extend_from_slice(&name_len.to_le_bytes());
        data.extend_from_slice(name.as_bytes());
        data.push(0);

        let type_name = "IntProperty";
        let type_len = (type_name.len() + 1) as i32;
        data.extend_from_slice(&type_len.to_le_bytes());
        data.extend_from_slice(type_name.as_bytes());
        data.push(0);

        data.extend_from_slice(&4u64.to_le_bytes()); // size 4
        data.push(0); // terminating zero for property
        data.extend_from_slice(&val.to_le_bytes());
    }

    fn add_str_prop(data: &mut Vec<u8>, name: &str, val: &str) {
        let name_len = (name.len() + 1) as i32;
        data.extend_from_slice(&name_len.to_le_bytes());
        data.extend_from_slice(name.as_bytes());
        data.push(0);

        let type_name = "StrProperty";
        let type_len = (type_name.len() + 1) as i32;
        data.extend_from_slice(&type_len.to_le_bytes());
        data.extend_from_slice(type_name.as_bytes());
        data.push(0);

        let val_len = (val.len() + 1) as i32;
        data.extend_from_slice(&((val.len() + 5) as u64).to_le_bytes());
        data.push(0);
        data.extend_from_slice(&val_len.to_le_bytes());
        data.extend_from_slice(val.as_bytes());
        data.push(0);
    }

    add_int_prop(&mut data, "BaseCampMaxNum", 128);
    add_int_prop(&mut data, "BaseCampMaxNumInGuild", 15);
    add_str_prop(&mut data, "HostPlayerName", "Valdacil");
    add_int_prop(&mut data, "HostPlayerLevel", 69);
    data.extend_from_slice(&[0u8; 128]);

    let server_cap = gvas_read_int(&data, "BaseCampMaxNum", 0);
    let guild_cap = gvas_read_int(&data, "BaseCampMaxNumInGuild", 0);
    let host_name = gvas_read_str(&data, "HostPlayerName", 0);
    let host_lvl = gvas_read_int(&data, "HostPlayerLevel", 0);

    assert_eq!(server_cap, Some(128));
    assert_eq!(guild_cap, Some(15));
    assert_eq!(host_name, Some("Valdacil".to_string()));
    assert_eq!(host_lvl, Some(69));

    // Ensure guild cap is prioritized
    let effective_base_cap = guild_cap.or(server_cap);
    assert_eq!(effective_base_cap, Some(15), "Must prioritize BaseCampMaxNumInGuild over server cap");
}

#[test]
fn test_deep_scan_crashed_saved_game_detects_wood_diagonal_fence() {
    use std::path::Path;
    use palmodmanager_lib::save_scanner::deep_scan::deep_scan_save;

    let world_dir = r"C:\Users\Antikux\AppData\Local\Pal\Saved\SaveGames\CrashedSavedGame";
    if !Path::new(world_dir).join("Level.sav").exists() {
        println!("CrashedSavedGame Level.sav does not exist, skipping");
        return;
    }

    let program_path = r"C:\Users\Antikux\Documents\PalModManager";
    // Scan with NO active mods
    let report = deep_scan_save(world_dir, &[], Some(program_path), None, None).expect("deep_scan_save should succeed");

    println!("Report status: {}", report.health_status);
    println!("Report summary: {}", report.summary_message);
    println!("Total mod references: {}", report.total_mod_references);
    println!("Orphaned mod refs: {:?}", report.orphaned_mod_refs);

    assert!(
        report.orphaned_mod_refs.iter().any(|o| o.asset_path == "WoodDiagonalFence" && o.occurrences == 4),
        "Must identify WoodDiagonalFence with 4 occurrences"
    );
    assert_eq!(report.total_mod_references, 4);
    assert_eq!(report.health_status, "corrupt");
    assert!(report.summary_message.contains("WoodDiagonalFence"));
}
