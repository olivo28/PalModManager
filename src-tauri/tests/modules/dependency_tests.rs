use palmodmanager_lib::commands::dependency::status::{is_remote_newer, compare_versions, parse_dmy};
use palmodmanager_lib::commands::dependency::vault::{extract_version_from_vault_filename, sanitize_version_tag};
use palmodmanager_lib::dependency_checker::check_ue4ss_latest;

#[test]
fn test_directional_semver_comparisons() {
    // 1. Local is newer: no update needed
    assert!(!is_remote_newer("0.6.6", "0.6.5"));
    assert!(!is_remote_newer("v0.6.6", "0.6.5"));
    assert!(!is_remote_newer("v0.6.6", "v0.6.5"));
    assert!(!is_remote_newer("2.0.0", "1.99.99"));

    // 2. Equal versions: no update needed
    assert!(!is_remote_newer("0.6.5", "0.6.5"));
    assert!(!is_remote_newer("v0.6.5", "0.6.5"));
    assert!(!is_remote_newer("0.6.5", "v0.6.5"));
    assert!(!is_remote_newer("v1.0.0", "v1.0.0"));

    // 3. Remote is strictly newer: update needed
    assert!(is_remote_newer("0.6.4", "0.6.5"));
    assert!(is_remote_newer("v0.6.4", "v0.6.5"));
    assert!(is_remote_newer("0.5.9", "0.6.0"));
    assert!(is_remote_newer("1.9.9", "2.0.0"));

    // 4. Asymmetric segment counts (e.g. 1.2 vs 1.2.1)
    assert!(is_remote_newer("1.2", "1.2.1"), "Missing segment in local should default to 0");
    assert!(!is_remote_newer("1.2.1", "1.2"), "Extra segment in local should be considered newer");

    // 5. compare_versions helper inverted verification
    assert!(compare_versions("0.6.6", "0.6.5"));
    assert!(compare_versions("0.6.5", "0.6.5"));
    assert!(!compare_versions("0.6.4", "0.6.5"));

    println!("  [DEPENDENCY] Directional SemVer Comparisons:");
    println!("               * '0.6.6' vs '0.6.5' (remote newer: {})", is_remote_newer("0.6.6", "0.6.5"));
    println!("               * '0.6.4' vs '0.6.5' (remote newer: {})", is_remote_newer("0.6.4", "0.6.5"));
    println!("               * '1.2' vs '1.2.1'   (remote newer: {})", is_remote_newer("1.2", "1.2.1"));
}

#[test]
fn test_dependency_date_parsing() {
    let date1 = parse_dmy("10.08.2026").expect("Should parse valid d.m.y date");
    let date2 = parse_dmy("15.08.2026").expect("Should parse valid d.m.y date");
    assert!(date2 > date1, "15.08.2026 must be after 10.08.2026");

    assert!(parse_dmy("invalid-date").is_none());
    assert!(parse_dmy("").is_none());

    println!("  [DEPENDENCY] Date parsing '%d.%m.%Y': {:?} < {:?}", date1, date2);
}

#[test]
fn test_vault_filename_sanitization_and_flavors() {
    // PalSchema version extraction
    assert_eq!(extract_version_from_vault_filename("PalSchema - 0.6.5.zip", "palschema"), "0.6.5");
    assert_eq!(extract_version_from_vault_filename("PalSchema-v0.6.4.zip", "palschema"), "v0.6.4");

    // UE4SS version extraction
    assert_eq!(extract_version_from_vault_filename("UE4SS - v3.0.1.zip", "ue4ss"), "v3.0.1");

    // Flavor preservation
    let zdev_tag = sanitize_version_tag("UE4SS-Palworld-v3.0.1-zDev", "ue4ss");
    assert!(zdev_tag.contains("zDev"), "Should preserve zDev build flavor");

    println!("  [DEPENDENCY] Vault sanitization: 'PalSchema - 0.6.5.zip' -> '0.6.5', 'UE4SS - v3.0.1.zip' -> 'v3.0.1'");
    println!("  [DEPENDENCY] Preserved flavor tag: '{}'", zdev_tag);
}

#[test]
fn test_check_ue4ss_latest_offline_resilient() {
    tauri::async_runtime::block_on(async {
        if let Ok((tag, date_str)) = check_ue4ss_latest().await {
            assert!(!tag.is_empty(), "Expected non-empty tag for UE4SS, got: {}", tag);
            assert!(date_str.contains("202"), "Expected 202x date for UE4SS, got: {}", date_str);
            println!("  [DEPENDENCY] Live/cached UE4SS latest: tag='{}', date='{}'", tag, date_str);
        }
    });
}

#[test]
fn test_palschema_install_mode_decoupling_and_empty_directory_handling() {
    use crate::helpers::test_env::TestEnv;
    let env = TestEnv::new_steam_win64();

    // Case 1: Empty native mods residue folder without UE4SS or PalSchema binaries
    let win64_dir = env.game_root.join("Pal").join("Binaries").join("Win64");
    let _ = std::fs::remove_file(win64_dir.join("dwmapi.dll"));
    let _ = std::fs::remove_dir_all(win64_dir.join("ue4ss"));

    let native_ps = env.game_root.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("PalSchema");
    std::fs::create_dir_all(&native_ps).unwrap();
    // Only put manifest and empty mods subfolder
    std::fs::create_dir_all(native_ps.join("mods")).unwrap();
    std::fs::write(native_ps.join("palschema.manifest.json"), r#"{"version":"Workshop","files":[]}"#).unwrap();

    let status = palmodmanager_lib::dependency_checker::check_dependencies(&env.game_root.to_string_lossy());
    assert!(!status.ue4ss_installed, "UE4SS must NOT be detected as installed without binaries");
    assert_eq!(status.ue4ss_install_mode, "NotFound", "UE4SS mode must be NotFound when empty folder exists");
    assert!(!status.palschema_installed, "PalSchema must NOT be detected as installed with only manifest and empty directory");
    assert_eq!(status.palschema_install_mode, "NotFound", "PalSchema mode must be NotFound");

    // Case 2: Authentic standard PalSchema install
    let std_ps = env.game_root.join("Pal").join("Binaries").join("Win64").join("ue4ss").join("Mods").join("PalSchema");
    std::fs::create_dir_all(std_ps.join("dlls")).unwrap();
    std::fs::write(std_ps.join("dlls").join("main.dll"), b"MZ").unwrap();
    std::fs::write(std_ps.join("palschema.version"), "0.6.7").unwrap();

    let status_std = palmodmanager_lib::dependency_checker::check_dependencies(&env.game_root.to_string_lossy());
    assert!(status_std.palschema_installed, "PalSchema must be detected when real main.dll exists");
    assert_eq!(status_std.palschema_install_mode, "Standard", "PalSchema mode must be Standard");
    assert_eq!(status_std.palschema_version.as_deref(), Some("0.6.7"));
    assert!(!status_std.ue4ss_installed, "UE4SS remains uninstalled");
    assert_eq!(status_std.ue4ss_install_mode, "NotFound");
}

#[test]
fn test_dependency_manifest_pmm_recording_and_legacy_migration() {
    use crate::helpers::test_env::TestEnv;
    use palmodmanager_lib::dependency_manifest::*;

    let env = TestEnv::new_steam_win64();
    let game_path_str = env.game_root.to_string_lossy();
    let win64 = env.game_root.join("Pal").join("Binaries").join("Win64");
    let ue4ss_dir = win64.join("ue4ss");
    let palschema_dir = ue4ss_dir.join("Mods").join("PalSchema");

    // 1. Test immediate UE4SS recording
    let temp_framework = env.program_data.join("fake_framework");
    std::fs::create_dir_all(&temp_framework).unwrap();
    std::fs::write(temp_framework.join("UE4SS.dll"), b"MZ").unwrap();
    std::fs::write(temp_framework.join("UE4SS-settings.ini"), b"[Settings]\n").unwrap();

    let ue4ss_manifest = record_ue4ss_extracted_install(&game_path_str, "28.08.2026", &temp_framework, true);
    assert!(ue4ss_manifest.is_some());
    let pmm_file = ue4ss_dir.join(UE4SS_MANIFEST_PMM);
    assert!(pmm_file.exists(), "ue4ss.pmm.json must be created");

    let manifest_content = std::fs::read_to_string(&pmm_file).unwrap();
    assert!(manifest_content.contains("28.08.2026"));
    assert!(manifest_content.contains("dwmapi.dll"));
    assert!(manifest_content.contains("ue4ss/UE4SS.dll"));

    // 2. Test immediate PalSchema recording
    let temp_ps_root = env.program_data.join("fake_ps_root");
    std::fs::create_dir_all(temp_ps_root.join("dlls")).unwrap();
    std::fs::write(temp_ps_root.join("dlls").join("main.dll"), b"MZ").unwrap();
    std::fs::write(temp_ps_root.join("palschema.version"), "0.6.7").unwrap();

    let ps_manifest = record_palschema_extracted_install(&game_path_str, "0.6.7", &temp_ps_root);
    assert!(ps_manifest.is_some());
    let ps_pmm_file = palschema_dir.join(PALSCHEMA_MANIFEST_PMM);
    assert!(ps_pmm_file.exists(), "palschema.pmm.json must be created");

    let ps_content = std::fs::read_to_string(&ps_pmm_file).unwrap();
    assert!(ps_content.contains("0.6.7"));
    assert!(ps_content.contains("PalSchema/dlls/main.dll"));

    // 3. Test legacy migration for UE4SS
    let _ = std::fs::remove_file(&pmm_file);
    let legacy_ue4ss = ue4ss_dir.join(UE4SS_MANIFEST_LEGACY);
    std::fs::write(&legacy_ue4ss, r#"{"depType":"ue4ss","version":"legacy-1.0","installDate":"2026-01-01T00:00:00Z","files":["dwmapi.dll"]}"#).unwrap();

    let migrated_ue4ss = ensure_ue4ss_manifest(&game_path_str, None);
    assert!(migrated_ue4ss.is_some());
    assert_eq!(migrated_ue4ss.unwrap().version, "legacy-1.0");
    assert!(pmm_file.exists(), "ue4ss.pmm.json must exist after migration");
    assert!(!legacy_ue4ss.exists(), "legacy ue4ss.manifest.json must be removed");

    // 4. Test legacy migration for PalSchema
    let _ = std::fs::remove_file(&ps_pmm_file);
    let legacy_ps = palschema_dir.join(PALSCHEMA_MANIFEST_LEGACY);
    std::fs::write(&legacy_ps, r#"{"depType":"palschema","version":"0.6.4","installDate":"2026-01-01T00:00:00Z","files":["PalSchema/dlls/main.dll"]}"#).unwrap();

    let migrated_ps = ensure_palschema_manifest(&game_path_str, None);
    assert!(migrated_ps.is_some());
    assert_eq!(migrated_ps.unwrap().version, "0.6.4");
    assert!(ps_pmm_file.exists(), "palschema.pmm.json must exist after migration");
    assert!(!legacy_ps.exists(), "legacy palschema.manifest.json must be removed");
}
