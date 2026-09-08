use std::fs;
use serde_json::json;
use crate::helpers::{TestEnv, ZipBuilder};
use palmodmanager_lib::models::ModType;
use palmodmanager_lib::zip_handler::build_install_manifest;
use palmodmanager_lib::zip_handler::workshop_rule::try_resolve_workshop_routing;
use palmodmanager_lib::dependency_manifest::CANONICAL_UE4SS_SYSTEM_MODS;

#[test]
fn test_workshop_info_json_lua_routing() {
    let env = TestEnv::new_steam_win64();
    let info_val = json!({
        "ModName": "Automatically Skip Mod Caution",
        "PackageName": "AutomaticallySkipModCaution",
        "Version": "1.0.0",
        "Author": "Otoha",
        "InstallRule": [
            {
                "Type": "Lua",
                "Targets": ["./Scripts"]
            }
        ]
    });

    let archive_files = vec![
        "Info.json".to_string(),
        "Scripts/main.lua".to_string(),
        "preview.png".to_string(),
    ];

    let resolved = try_resolve_workshop_routing(&info_val, &archive_files, &env.game_root)
        .expect("Should resolve Lua workshop routing");

    assert_eq!(resolved.folder_name, "AutomaticallySkipModCaution");
    assert_eq!(resolved.display_name, "Automatically Skip Mod Caution");
    assert_eq!(resolved.mod_type, ModType::Ue4ss);

    // Verify main.lua is routed into ue4ss/Mods/AutomaticallySkipModCaution/Scripts/main.lua
    let lua_route = resolved.routes.iter().find(|r| r.zip_path == "Scripts/main.lua")
        .expect("main.lua route must exist");
    assert!(lua_route.dest_path.contains("ue4ss"));
    assert!(lua_route.dest_path.contains("AutomaticallySkipModCaution"));
    assert!(lua_route.dest_path.ends_with("main.lua"));
}

#[test]
fn test_workshop_info_json_palschema_routing() {
    let env = TestEnv::new_steam_win64();
    let info_val = json!({
        "ModName": "Recover Pal Spheres",
        "PackageName": "RecoverPalSpheres",
        "Version": "1.0.2",
        "Author": "Creator",
        "InstallRule": [
            {
                "Type": "PalSchema",
                "Targets": ["./PalSchema/"]
            }
        ]
    });

    let archive_files = vec![
        "Info.json".to_string(),
        "PalSchema/config.json".to_string(),
        "PalSchema/script.lua".to_string(),
    ];

    let resolved = try_resolve_workshop_routing(&info_val, &archive_files, &env.game_root)
        .expect("Should resolve PalSchema workshop routing");

    assert_eq!(resolved.folder_name, "RecoverPalSpheres");
    assert_eq!(resolved.mod_type, ModType::PalSchema);

    // Verify files routed to PalSchema/mods/RecoverPalSpheres
    let cfg_route = resolved.routes.iter().find(|r| r.zip_path == "PalSchema/config.json")
        .expect("config.json route must exist");
    assert!(cfg_route.dest_path.contains("PalSchema"));
    assert!(cfg_route.dest_path.contains("RecoverPalSpheres"));
}

#[test]
fn test_workshop_info_json_paks_routing() {
    let env = TestEnv::new_steam_win64();
    let info_val = json!({
        "ModName": "More Pouch Slots",
        "PackageName": "MorePouchSlots",
        "Version": "1.0.0",
        "Author": "Modder",
        "InstallRule": [
            {
                "Type": "Paks",
                "Targets": ["./Paks/"]
            }
        ]
    });

    let archive_files = vec![
        "Info.json".to_string(),
        "Paks/MorePouchSlots_P.pak".to_string(),
    ];

    let resolved = try_resolve_workshop_routing(&info_val, &archive_files, &env.game_root)
        .expect("Should resolve Paks workshop routing");

    assert_eq!(resolved.folder_name, "MorePouchSlots");
    assert_eq!(resolved.mod_type, ModType::Pak);

    let pak_route = resolved.routes.iter().find(|r| r.zip_path == "Paks/MorePouchSlots_P.pak")
        .expect("Pak route must exist");
    assert!(pak_route.dest_path.contains("~mods"));
    assert!(pak_route.dest_path.ends_with("MorePouchSlots_P.pak"));
}

#[test]
fn test_workshop_info_json_hybrid_routing() {
    let env = TestEnv::new_steam_win64();
    let info_val = json!({
        "ModName": "Base Trimmer",
        "PackageName": "BaseTrimmer",
        "Version": "2.1.0",
        "Author": "TrimmerTeam",
        "InstallRule": [
            {
                "Type": "Lua",
                "Targets": ["./Scripts"]
            },
            {
                "Type": "Paks",
                "Targets": ["./Paks"]
            }
        ]
    });

    let archive_files = vec![
        "Info.json".to_string(),
        "Scripts/main.lua".to_string(),
        "Paks/BaseTrimmer_P.pak".to_string(),
    ];

    let resolved = try_resolve_workshop_routing(&info_val, &archive_files, &env.game_root)
        .expect("Should resolve hybrid workshop routing");

    // Hybrid should have both Lua and Pak routes
    let has_lua = resolved.routes.iter().any(|r| r.dest_path.contains("ue4ss") && r.dest_path.ends_with("main.lua"));
    let has_pak = resolved.routes.iter().any(|r| r.dest_path.contains("~mods") && r.dest_path.ends_with("BaseTrimmer_P.pak"));

    assert!(has_lua, "Hybrid mod must route Lua scripts to ue4ss/Mods");
    assert!(has_pak, "Hybrid mod must route Pak files to ~mods");
}

#[test]
fn test_workshop_zip_end_to_end_manifest_building() {
    let env = TestEnv::new_steam_win64();
    let info_content = r#"{
        "ModName": "Senator Armstrong Pal",
        "PackageName": "SenatorArmstrong",
        "Version": "1.0",
        "Author": "EPK",
        "InstallRule": [
            { "Type": "Paks", "Targets": ["./Paks"] }
        ]
    }"#;

    let zip_path = ZipBuilder::new("SenatorArmstrong 1.0 1623730.zip")
        .add_text_file("Info.json", info_content)
        .add_file("Paks/SenatorArmstrong_P.pak", b"MOCK_PAK_CONTENT")
        .build_in(&env.temp_dir);

    let manifest = build_install_manifest(zip_path.to_str().unwrap(), &env.game_root, None, None)
        .expect("Should build manifest from workshop archive using InstallRule");

    assert_eq!(manifest.display_name, "Senator Armstrong Pal");
    assert_eq!(manifest.mod_type, ModType::Pak);
    let pak_route = manifest.routes.iter().find(|r| r.zip_path.ends_with(".pak"))
        .expect("Must have pak route");
    assert!(pak_route.dest_path.contains("~mods"));
}

#[test]
fn test_workshop_root_prefix_and_hybrid_rules() {
    let env = TestEnv::new_steam_win64();
    let info_val = json!({
        "ModName": "QualityOfLife",
        "PackageName": "QualityOfLife",
        "Version": "1.0.1",
        "Author": "yinqvanr3",
        "InstallRule": [
            { "Type": "Lua", "Targets": ["./Scripts"] },
            { "Type": "Paks", "Targets": ["./Paks/"] }
        ]
    });

    let archive_files = vec![
        "3761921027/.workshop.json".to_string(),
        "3761921027/Info.json".to_string(),
        "3761921027/Paks/QualityOfLife_base-drop-stacker_P.pak".to_string(),
        "3761921027/Paks/QualityOfLife_capture-statue-cost_P.pak".to_string(),
        "3761921027/Scripts/main.lua".to_string(),
        "3761921027/Scripts/QualityOfLifeBaseRange/base_range.lua".to_string(),
        "3761921027/thumbnail.png".to_string(),
    ];

    let resolved = try_resolve_workshop_routing(&info_val, &archive_files, &env.game_root)
        .expect("Should resolve root-prefixed workshop routing");

    assert_eq!(resolved.folder_name, "QualityOfLife");
    assert_eq!(resolved.mod_type, ModType::Hybrid);

    // Pak files must route to ~mods
    let pak1 = resolved.routes.iter().find(|r| r.zip_path.ends_with("QualityOfLife_base-drop-stacker_P.pak"))
        .expect("Pak 1 must exist");
    assert!(pak1.dest_path.contains("~mods"));
    assert_eq!(pak1.route_type, palmodmanager_lib::models::RouteType::Pak);

    // Script files must route to ue4ss/Mods/QualityOfLife/Scripts/
    let lua1 = resolved.routes.iter().find(|r| r.zip_path.ends_with("main.lua"))
        .expect("main.lua must exist");
    assert!(lua1.dest_path.contains("ue4ss"));
    assert!(lua1.dest_path.contains("QualityOfLife"));
    assert!(lua1.dest_path.contains("Scripts"));
    assert_eq!(lua1.route_type, palmodmanager_lib::models::RouteType::Ue4ss);
}

#[test]
fn test_workshop_rule_specificity_over_blanket() {
    let env = TestEnv::new_steam_win64();
    let info_val = json!({
        "ModName": "HybridBlanketMod",
        "PackageName": "HybridBlanketMod",
        "Version": "1.0",
        "InstallRule": [
            { "Type": "PalSchema", "Targets": ["."] },
            { "Type": "Paks", "Targets": ["./Paks/"] }
        ]
    });

    let archive_files = vec![
        "Info.json".to_string(),
        "Paks/Companion_P.pak".to_string(),
        "PalSchema/config.json".to_string(),
    ];

    let resolved = try_resolve_workshop_routing(&info_val, &archive_files, &env.game_root)
        .expect("Should resolve routes with specificity ordering");

    // Pak should be claimed by Paks rule into ~mods, not PalSchema
    let pak = resolved.routes.iter().find(|r| r.zip_path == "Paks/Companion_P.pak").unwrap();
    assert_eq!(pak.route_type, palmodmanager_lib::models::RouteType::Pak);
    assert!(pak.dest_path.contains("~mods"));

    // Config should be claimed by PalSchema rule
    let cfg = resolved.routes.iter().find(|r| r.zip_path == "PalSchema/config.json").unwrap();
    assert_eq!(cfg.route_type, palmodmanager_lib::models::RouteType::PalSchema);
    assert!(cfg.dest_path.contains("PalSchema"));
}

#[test]
fn test_quality_of_life_zip_end_to_end_manifest_building() {
    let env = TestEnv::new_steam_win64();
    let info_content = r#"{
        "ModName": "QualityOfLife",
        "PackageName": "QualityOfLife",
        "Version": "1.0.1",
        "Author": "yinqvanr3",
        "InstallRule": [
            { "Type": "Lua", "Targets": ["./Scripts"] },
            { "Type": "Paks", "Targets": ["./Paks/"] }
        ]
    }"#;

    let zip_path = ZipBuilder::new("Quality Of Life 4599 1 2026-07-30T23-44Z.zip")
        .add_text_file("3761921027/Info.json", info_content)
        .add_file("3761921027/Paks/QualityOfLife_base-drop-stacker_P.pak", b"MOCK_PAK")
        .add_file("3761921027/Scripts/main.lua", b"print('QoL')")
        .build_in(&env.temp_dir);

    let manifest = build_install_manifest(zip_path.to_str().unwrap(), &env.game_root, None, None)
        .expect("Should build manifest from root-prefixed workshop archive");

    assert_eq!(manifest.display_name, "QualityOfLife");
    assert_eq!(manifest.mod_type, ModType::Hybrid);

    let pak_route = manifest.routes.iter().find(|r| r.zip_path.ends_with(".pak"))
        .expect("Must have pak route");
    assert_eq!(pak_route.route_type, palmodmanager_lib::models::RouteType::Pak);
    assert!(pak_route.dest_path.contains("~mods"));

    let lua_route = manifest.routes.iter().find(|r| r.zip_path.ends_with(".lua"))
        .expect("Must have lua route");
    assert_eq!(lua_route.route_type, palmodmanager_lib::models::RouteType::Ue4ss);
    assert!(lua_route.dest_path.contains("ue4ss"));
    assert!(lua_route.dest_path.contains("QualityOfLife"));
}

#[test]
fn test_dependency_uninstall_preserves_user_mods() {
    let env = TestEnv::new_steam_win64();
    let win64 = env.game_root.join("Pal").join("Binaries").join("Win64");
    let ue4ss_dir = win64.join("ue4ss");
    let mods_dir = ue4ss_dir.join("Mods");

    // Setup simulated UE4SS runtime files
    fs::create_dir_all(&mods_dir).unwrap();
    fs::write(ue4ss_dir.join("UE4SS.dll"), b"MOCK_DLL").unwrap();
    fs::write(ue4ss_dir.join("UE4SS-settings.ini"), b"bEnableModLoader = 1").unwrap();
    fs::write(ue4ss_dir.join("ue4ss.version"), b"3.0.1").unwrap();
    fs::write(ue4ss_dir.join("ue4ss.pmm.json"), b"{}").unwrap();

    // Setup canonical system mod
    let sys_mod_dir = mods_dir.join("BPModLoaderMod");
    fs::create_dir_all(&sys_mod_dir).unwrap();
    fs::write(sys_mod_dir.join("enabled.txt"), b"").unwrap();

    // Setup user mod that MUST BE PRESERVED
    let user_mod_dir = mods_dir.join("MyAwesomeUserMod").join("Scripts");
    fs::create_dir_all(&user_mod_dir).unwrap();
    fs::write(user_mod_dir.join("main.lua"), b"print('Hello User Mod')").unwrap();

    // Perform selective purge (matching uninstall_ue4ss logic)
    let runtime_files = [
        "UE4SS.dll", "UE4SS-settings.ini", "MemberVariableLayout.ini",
        "LICENSE", "ue4ss.version", "ue4ss.pmm.json", "ue4ss.manifest.json"
    ];
    for f in runtime_files {
        let p = ue4ss_dir.join(f);
        if p.exists() { let _ = fs::remove_file(p); }
    }

    // Clean only canonical system mods
    for sys_mod in CANONICAL_UE4SS_SYSTEM_MODS {
        let sys_p = mods_dir.join(sys_mod);
        if sys_p.exists() { let _ = fs::remove_dir_all(sys_p); }
    }

    // Verify runtime files are removed
    assert!(!ue4ss_dir.join("UE4SS.dll").exists(), "UE4SS.dll must be removed");
    assert!(!ue4ss_dir.join("ue4ss.version").exists(), "ue4ss.version must be removed");
    assert!(!sys_mod_dir.exists(), "BPModLoaderMod must be removed");

    // CRITICAL: User mod must be preserved 100% intact!
    assert!(user_mod_dir.join("main.lua").exists(), "User mod script MUST remain intact on disk!");
    let content = fs::read_to_string(user_mod_dir.join("main.lua")).unwrap();
    assert_eq!(content, "print('Hello User Mod')");
}
