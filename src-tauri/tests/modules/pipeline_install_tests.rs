use std::fs;
use crate::helpers::{TestEnv, ZipBuilder};
use palmodmanager_lib::models::{ModInfo, ModType, PmmMetadata};
use palmodmanager_lib::zip_handler::{analyze_zip, build_install_manifest};
use palmodmanager_lib::profiles::utils::save_pmm_meta;
use palmodmanager_lib::library::copy_to_library;

#[test]
fn test_steam_win64_installation_pipeline() {
    let env = TestEnv::new_steam_win64();

    // 1. Build a real multi-component hybrid ZIP
    let zip_filename = "PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip";
    let zip_path = ZipBuilder::new(zip_filename)
        .add_file("Pal/Content/Paks/~mods/PalVariety_RarityShiny_P.pak", b"MOCK_PAK_CONTENT")
        .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/PalVarietyShiny4x/Scripts/main.lua", "print('PalVariety loaded')")
        .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/PalVarietyShiny4x/config.lua", "Config = { ShinyRate = 4096 }")
        .build_in(&env.temp_dir);

    assert!(zip_path.exists(), "ZIP archive must be generated");

    // 2. Real ZIP analysis
    let analysis = analyze_zip(zip_path.to_str().unwrap()).expect("analyze_zip should succeed");
    assert!(analysis.has_pak, "Must detect PAK component");
    assert!(analysis.has_lua, "Must detect Lua component");
    assert_eq!(analysis.files.len(), 3, "Must detect exactly 3 files");

    // 3. Build install manifest against Steam Win64
    let manifest = build_install_manifest(zip_path.to_str().unwrap(), &env.game_root, None, None)
        .expect("build_install_manifest should succeed");

    assert_eq!(manifest.mod_type, ModType::Hybrid, "Multi-component mod must be classified as Hybrid");
    assert_eq!(manifest.routes.len(), 3, "All 3 files must have valid installation routes");

    // Verify PAK destination
    let pak_route = manifest.routes.iter().find(|r| r.zip_path.ends_with(".pak")).unwrap();
    assert!(
        pak_route.dest_path.contains("~mods") && pak_route.dest_path.ends_with("PalVariety_RarityShiny_P.pak"),
        "PAK must route to ~mods, got: {}",
        pak_route.dest_path
    );

    // Verify Lua script destination in Steam Win64 ue4ss
    let lua_route = manifest.routes.iter().find(|r| r.zip_path.ends_with("main.lua")).unwrap();
    assert!(
        lua_route.dest_path.contains("Win64") && lua_route.dest_path.contains("ue4ss") && lua_route.dest_path.ends_with("main.lua"),
        "Lua script must route to Win64/ue4ss/Mods, got: {}",
        lua_route.dest_path
    );

    println!("  [PIPELINE] Steam Win64 Hybrid Installation Result:");
    println!("             * Mod folder: {}", manifest.folder_name);
    println!("             * Classification: {:?}", manifest.mod_type);
    println!("             * PAK routed to: {}", pak_route.dest_path);
    println!("             * Lua routed to: {}", lua_route.dest_path);
}

#[test]
fn test_xbox_wingdk_installation_pipeline() {
    let env = TestEnv::new_xbox_wingdk();

    let zip_filename = "InfiniteStamina_V1.2.0.zip";
    let zip_path = ZipBuilder::new(zip_filename)
        .add_text_file("Scripts/main.lua", "print('Infinite Stamina active')")
        .add_text_file("enabled.txt", "")
        .build_in(&env.temp_dir);

    let manifest = build_install_manifest(zip_path.to_str().unwrap(), &env.game_root, None, None)
        .expect("build_install_manifest should succeed on Xbox WinGDK");

    assert_eq!(manifest.mod_type, ModType::Ue4ss);
    for r in &manifest.routes {
        assert!(
            r.dest_path.contains("WinGDK"),
            "Xbox WinGDK routes must point to WinGDK binaries directory, got: {}",
            r.dest_path
        );
    }

    println!("  [PIPELINE] Xbox WinGDK Installation Result:");
    println!("             * Mod folder: {}", manifest.folder_name);
    for r in &manifest.routes {
        println!("             * Route: {} -> {}", r.zip_path, r.dest_path);
    }
}

#[test]
fn test_steam_workshop_installation_pipeline() {
    let env = TestEnv::new_workshop();

    let zip_filename = "CustomHUD_1.0.0.zip";
    let zip_path = ZipBuilder::new(zip_filename)
        .add_text_file("Scripts/main.lua", "print('Custom HUD loaded')")
        .build_in(&env.temp_dir);

    let manifest = build_install_manifest(zip_path.to_str().unwrap(), &env.game_root, None, None)
        .expect("build_install_manifest should succeed in Steam Workshop mode");

    let lua_route = manifest.routes.iter().find(|r| r.zip_path.ends_with("main.lua")).unwrap();
    assert!(
        lua_route.dest_path.contains("NativeMods") && lua_route.dest_path.contains("UE4SS"),
        "Workshop active mode must route scripts into Mods/NativeMods/UE4SS/Mods, got: {}",
        lua_route.dest_path
    );

    println!("  [PIPELINE] Steam Workshop Installation Result:");
    println!("             * Mod folder: {}", manifest.folder_name);
    println!("             * NativeMods route: {}", lua_route.dest_path);
}

#[test]
fn test_pmm_metadata_sidecar_persistence() {
    let env = TestEnv::new_steam_win64();

    // Create primary PAK file and companion directory on disk
    let paks_dir = env.game_root.join("Pal").join("Content").join("Paks").join("~mods");
    let primary_pak = paks_dir.join("PalVariety_RarityShiny_P.pak");
    fs::write(&primary_pak, b"DUMMY_PAK").unwrap();

    let ue4ss_dir = env.game_root.join("Pal").join("Binaries").join("Win64").join("ue4ss").join("Mods").join("PalVarietyShiny4x");
    fs::create_dir_all(&ue4ss_dir).unwrap();
    fs::write(ue4ss_dir.join("main.lua"), b"-- test").unwrap();

    let mod_info = ModInfo {
        id: "palvariety_shiny".to_string(),
        name: "PalVariety Rarity Shiny4096".to_string(),
        mod_type: ModType::Hybrid,
        nexus_mod_id: Some(4140),
        nexus_url: None,
        nexus_author: None,
        nexus_summary: None,
        nexus_picture_url: None,
        nexus_endorsements: None,
        nexus_downloads: None,
        version: "1.1.3".to_string(),
        install_date: "2026-09-05".to_string(),
        source_zip: "PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip".to_string(),
        config_path: Some(ue4ss_dir.join("config.lua").to_string_lossy().to_string()),
        config_paths: None,
        config_type: Some("lua".to_string()),
        enabled: true,
        game_path: primary_pak.to_string_lossy().to_string(),
        disabled_path: String::new(),
        pak_destination: None,
        has_enabled_txt: false,
        mods_txt_order: None,
        extra_files: vec![ue4ss_dir.to_string_lossy().to_string()],
        nexus_description: None,
        nexus_version_cached: None,
        nexus_cached_at: None,
        nexus_category: None,
        nexus_tags: Vec::new(),
        github_repo: None,
        github_version: None,
        github_cached_at: None,
        update_date: None,
        library_zip: None,
        ignored_version: None,
        nexus_file_id: Some("QXTyhgia8".to_string()),
        ignored_keys: None,
        has_pending_update: Some(false),
        origin_load_method: None,
        custom_notes: None,
        original_name: None,
        custom_name: None,
    };

    // Save metadata
    let res = save_pmm_meta(&mod_info);
    assert!(res.is_ok(), "save_pmm_meta must succeed: {:?}", res.err());

    // Verify sidecar exists next to the primary PAK
    let sidecar_path = paks_dir.join("PalVariety_RarityShiny_P.pak.pmm.json");
    assert!(sidecar_path.exists(), "Sidecar .pmm.json must exist at {:?}", sidecar_path);

    // Read and verify metadata fields
    let sidecar_content = fs::read_to_string(&sidecar_path).unwrap();
    let meta: PmmMetadata = serde_json::from_str(&sidecar_content).expect("PmmMetadata must deserialize cleanly");

    assert_eq!(meta.source_zip, Some("PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip".to_string()));
    assert_eq!(meta.nexus_file_id, Some("QXTyhgia8".to_string()));
    assert_eq!(meta.nexus_mod_id, Some(4140));

    let folders = meta.installed_folders.expect("installed_folders must be populated and not null");
    assert!(folders.contains(&"PalVarietyShiny4x".to_string()), "Must contain companion UE4SS folder component");

    println!("  [PIPELINE] .pmm.json Sidecar Persisted:");
    println!("             * File: {:?}", sidecar_path);
    println!("             * sourceZip: {:?}", meta.source_zip);
    println!("             * nexusFileId: {:?}", meta.nexus_file_id);
    println!("             * installedFolders: {:?}", folders);
}

#[test]
fn test_library_filename_retention() {
    let env = TestEnv::new_steam_win64();

    // 1. Download with real Nexus name
    let real_name = "PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip";
    let real_zip = ZipBuilder::new(real_name)
        .add_text_file("readme.txt", "info")
        .build_in(&env.temp_dir);

    let program_dir = env.temp_dir.join("pmm_app");
    fs::create_dir_all(&program_dir).unwrap();

    let entry = copy_to_library(
        real_zip.to_str().unwrap(),
        program_dir.to_str().unwrap(),
        "palvariety_shiny",
        None,
        Some("1.1.3"),
    ).expect("copy_to_library should succeed");

    assert_eq!(entry.zip_name, real_name, "Real archive must retain 100% of original downloaded filename");

    // 2. Download with anonymous/temp name
    let temp_name = "nexus_temp_9999.zip";
    let temp_zip = ZipBuilder::new(temp_name)
        .add_text_file("readme.txt", "info")
        .build_in(&env.temp_dir);

    let entry_anon = copy_to_library(
        temp_zip.to_str().unwrap(),
        program_dir.to_str().unwrap(),
        "MyCoolMod",
        None,
        Some("2.0.0"),
    ).expect("copy_to_library should succeed for anonymous zip");

    assert_eq!(entry_anon.zip_name, "MyCoolMod - 2.0.0.zip", "Anonymous archive must fall back cleanly to name - version.zip");

    println!("  [PIPELINE] Library Filename Retention:");
    println!("             * Original preserved: '{}'", entry.zip_name);
    println!("             * Anonymous fallback: '{}'", entry_anon.zip_name);
}
