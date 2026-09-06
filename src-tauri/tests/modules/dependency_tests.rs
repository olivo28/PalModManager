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
