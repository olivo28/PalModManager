use std::fs;
use std::io::Read;
use crate::helpers::{TestEnv, ZipBuilder};
use palmodmanager_lib::models::RouteType;
use palmodmanager_lib::zip_handler::{
    build_install_manifest, detect_folder_name_from_files, find_resilient_zip_boundary,
    is_forbidden, open_resilient_zip,
};

#[test]
fn test_synthetic_zip_70kb_trailing_bytes_and_open_resilient_zip() {
    let env = TestEnv::new_steam_win64();

    // 1. Build a valid ZIP
    let zip_filename = "RotateIt_Defective.zip";
    let valid_zip_path = ZipBuilder::new(zip_filename)
        .add_text_file("RotateIt/Scripts/main.lua", "print('RotateIt running')")
        .add_text_file("RotateIt/enabled.txt", "")
        .build_in(&env.temp_dir);

    let original_bytes = fs::read(&valid_zip_path).unwrap();
    let original_len = original_bytes.len();

    // 2. Corrupt by appending 70,000 trailing bytes (exceeds zip spec standard 65KB comment scanner)
    let mut corrupted_bytes = original_bytes.clone();
    corrupted_bytes.extend_from_slice(&vec![0xEE; 70_000]);

    let corrupted_path = env.temp_dir.join("RotateIt_Corrupted_70KB_Trailing.zip");
    fs::write(&corrupted_path, &corrupted_bytes).unwrap();

    // 3. Test find_resilient_zip_boundary identifies true EOCD end
    let boundary = find_resilient_zip_boundary(&corrupted_bytes);
    assert_eq!(boundary, Some(original_len), "Should locate exact EOCD boundary before trailing noise");

    // 4. Test open_resilient_zip successfully recovers and reads the files
    let mut archive = open_resilient_zip(corrupted_path.to_str().unwrap())
        .expect("open_resilient_zip must salvage corrupted zip with trailing bytes");

    let file_count = archive.len();
    assert_eq!(file_count, 2, "Salvaged archive must contain 2 entries");

    let script_content = {
        let mut entry = archive.by_name("RotateIt/Scripts/main.lua").expect("Entry should exist");
        let mut content = String::new();
        entry.read_to_string(&mut content).unwrap();
        content
    };
    assert_eq!(script_content, "print('RotateIt running')");

    println!("  [RESILIENCE] 70KB Trailing Bytes Recovery Result:");
    println!("               * Original size: {} bytes, Corrupted size: {} bytes", original_len, corrupted_bytes.len());
    println!("               * Detected true EOCD boundary: {:?}", boundary);
    println!("               * Salvaged files count: {}", file_count);
    println!("               * Recovered script: {}", script_content.trim());
}

#[test]
fn test_forbidden_mod_name_normalization() {
    // 1. Forbidden game structural folders must be rejected
    assert!(is_forbidden("Pal"), "'Pal' must be forbidden");
    assert!(is_forbidden("binaries"), "'binaries' must be forbidden");
    assert!(is_forbidden("Win64"), "'Win64' must be forbidden");
    assert!(is_forbidden("WinGDK"), "'WinGDK' must be forbidden");
    assert!(is_forbidden("Mods"), "'Mods' must be forbidden");
    assert!(is_forbidden("~mods"), "'~mods' must be forbidden");
    assert!(is_forbidden("ue4ss"), "'ue4ss' must be forbidden");
    assert!(is_forbidden("(Steam) Mod Folder"), "Steam folder aliases must be forbidden");
    assert!(is_forbidden("[GamePass]"), "GamePass aliases must be forbidden");

    // 2. detect_folder_name_from_files must discard forbidden parents and pick true mod name
    let deeply_nested_ue4ss = vec![
        "Pal/Binaries/Win64/ue4ss/Mods/AutoCatchPal/Scripts/main.lua".to_string(),
        "Pal/Binaries/Win64/ue4ss/Mods/AutoCatchPal/enabled.txt".to_string(),
    ];
    let detected = detect_folder_name_from_files(&deeply_nested_ue4ss, "fallback.zip");
    assert_eq!(detected, "AutoCatchPal", "Should detect 'AutoCatchPal' despite deep game hierarchy");

    // 3. Steam Workshop NativeMods structure
    let workshop_files = vec![
        "Mods/NativeMods/UE4SS/Mods/BetterBuilding/Scripts/main.lua".to_string(),
        "Mods/NativeMods/UE4SS/Mods/BetterBuilding/config.lua".to_string(),
    ];
    let detected_workshop = detect_folder_name_from_files(&workshop_files, "BetterBuilding_v1.zip");
    assert_eq!(detected_workshop, "BetterBuilding", "Should extract mod name from NativeMods path");

    // 4. PalSchema markers
    let palschema_files = vec![
        "CustomPalPack/pals/DarkGryff.json".to_string(),
        "CustomPalPack/skills/DarkPulse.json".to_string(),
    ];
    let detected_schema = detect_folder_name_from_files(&palschema_files, "CustomPal.zip");
    assert_eq!(detected_schema, "CustomPalPack", "Should identify PalSchema mod root");

    println!("  [RESILIENCE] Forbidden Name Normalization Result:");
    println!("               * Deeply nested path detected name: '{}'", detected);
    println!("               * Workshop NativeMods detected name: '{}'", detected_workshop);
    println!("               * PalSchema detected name: '{}'", detected_schema);
}

#[test]
fn test_deep_subfolder_asset_preservation() {
    let env = TestEnv::new_steam_win64();

    let zip_filename = "PalMercyToggle.zip";
    let zip_path = ZipBuilder::new(zip_filename)
        .add_text_file("PalMercyToggle/Scripts/main.lua", "print('Mercy Toggle')")
        .add_text_file("PalMercyToggle/enabled.txt", "")
        .add_file("PalMercyToggle/Assets/mercy_on.png", b"PNG_IMAGE_DATA_ON")
        .add_file("PalMercyToggle/Assets/subfolder/mercy_dark.png", b"PNG_IMAGE_DATA_DARK")
        .add_text_file("PalMercyToggle/CustomUI/styles/theme.css", ".icon { opacity: 1.0; }")
        .build_in(&env.temp_dir);

    let manifest = build_install_manifest(zip_path.to_str().unwrap(), &env.game_root, None, None)
        .expect("build_install_manifest should succeed");

    assert_eq!(manifest.folder_name, "PalMercyToggle");
    assert_eq!(manifest.routes.len(), 5, "All 5 files including deep subfolder assets must be routed");

    // Verify deep assets are not flattened or lost
    let icon_route = manifest.routes.iter().find(|r| r.zip_path.ends_with("mercy_on.png")).unwrap();
    assert_eq!(icon_route.route_type, RouteType::Ue4ss);
    assert!(
        icon_route.dest_path.replace('\\', "/").ends_with("PalMercyToggle/Assets/mercy_on.png"),
        "Asset must preserve Assets/ folder structure: {}",
        icon_route.dest_path
    );

    let nested_icon = manifest.routes.iter().find(|r| r.zip_path.ends_with("mercy_dark.png")).unwrap();
    assert!(
        nested_icon.dest_path.replace('\\', "/").ends_with("PalMercyToggle/Assets/subfolder/mercy_dark.png"),
        "Nested asset must preserve Assets/subfolder structure: {}",
        nested_icon.dest_path
    );

    let css_route = manifest.routes.iter().find(|r| r.zip_path.ends_with("theme.css")).unwrap();
    assert!(
        css_route.dest_path.replace('\\', "/").ends_with("PalMercyToggle/CustomUI/styles/theme.css"),
        "Custom UI asset must preserve CustomUI/styles structure: {}",
        css_route.dest_path
    );

    println!("  [RESILIENCE] Deep Subfolder Asset Preservation Result:");
    println!("               * Mod folder: {}", manifest.folder_name);
    println!("               * Total routed files: {}", manifest.routes.len());
    println!("               * Assets/ icon route: {}", icon_route.dest_path);
    println!("               * Assets/subfolder/ icon route: {}", nested_icon.dest_path);
    println!("               * CustomUI/styles route: {}", css_route.dest_path);
}
