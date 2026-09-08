use std::fs;
use std::path::PathBuf;
use palmodmanager_lib::commands::config_archive::{
    sanitize_archive_key, get_archive_dir, preview_archived_config_diff_internal,
    apply_archived_config_internal, ArchivedConfigInfo,
};
use crate::helpers::{TestEnv, ZipBuilder};

#[test]
fn test_sanitize_archive_key() {
    assert_eq!(sanitize_archive_key("PalVariety 4x (Shiny)!"), "palvariety_4x_shiny");
    assert_eq!(sanitize_archive_key("---"), "mod");
    assert_eq!(sanitize_archive_key("SimpleMod"), "simplemod");
}

#[test]
fn test_get_archive_dir() {
    let dir = get_archive_dir("C:/pmm", "default", "nexus_1234");
    assert!(dir.to_string_lossy().contains("archived_configs"));
    assert!(dir.to_string_lossy().ends_with("nexus_1234"));
}

#[test]
fn test_preview_and_apply_archived_config_with_ignored_files_and_keys() {
    let env = TestEnv::new_steam_win64();

    // 1. Set up simulated archive directory with archived configs
    let archive_dir = env.program_data.join("profiles/default/archived_configs/nexus_9999");
    fs::create_dir_all(&archive_dir).unwrap();

    let meta = ArchivedConfigInfo {
        archive_id: "nexus_9999".to_string(),
        mod_name: "TestMod".to_string(),
        mod_id: "testmod".to_string(),
        nexus_mod_id: Some(9999),
        archived_at: "2026-09-06T12:00:00Z".to_string(),
        files: vec!["config.json".to_string(), "settings.ini".to_string()],
    };
    fs::write(archive_dir.join("archive_meta.json"), serde_json::to_string(&meta).unwrap()).unwrap();

    // User customized config.json and settings.ini in the archive:
    fs::write(
        archive_dir.join("config.json"),
        r#"{"preserved_key": "user_val", "skipped_key": "user_skip_val"}"#,
    ).unwrap();
    fs::write(
        archive_dir.join("settings.ini"),
        "[General]\nUserSetting=100\n",
    ).unwrap();

    // 2. Build incoming update ZIP
    let update_zip = ZipBuilder::new("TestMod 2.0.0 9999 2.0.0 2026-09-06.zip")
        .add_text_file("TestMod/config.json", r#"{"preserved_key": "author_default", "skipped_key": "author_default", "new_key": 42}"#)
        .add_text_file("TestMod/settings.ini", "[General]\nUserSetting=1\nNewAuthorSetting=2\n")
        .add_text_file("TestMod/enabled.txt", "");

    let downloads = env.game_root.join("downloads");
    fs::create_dir_all(&downloads).unwrap();
    let zip_file = update_zip.build_in(&downloads);

    // 3. Test preview_archived_config_diff_internal
    let diffs = preview_archived_config_diff_internal(
        &zip_file.to_string_lossy(),
        &archive_dir,
    ).expect("Preview archived diff should succeed");

    assert_eq!(diffs.len(), 2, "Should have generated diffs for both files");
    let json_diff = diffs.iter().find(|d| d.file_name.contains("config.json")).expect("config.json diff found");
    assert!(json_diff.keys_user_changed.iter().any(|c| c.key == "preserved_key"));
    assert!(json_diff.keys_user_changed.iter().any(|c| c.key == "skipped_key"));
    assert!(json_diff.keys_added_by_author.iter().any(|k| k == "new_key"));

    // 4. Install the new mod on disk
    let installed = env.install_builder(&update_zip).expect("Install update zip");
    let target_dir = PathBuf::from(&installed.game_path);

    // 5. Apply restore with settings.ini ignored, and "skipped_key" ignored for config.json
    let ignored_files = vec!["settings.ini".to_string()];
    let ignored_keys = vec!["skipped_key".to_string()];

    let restored = apply_archived_config_internal(
        &archive_dir,
        &target_dir,
        &ignored_files,
        &ignored_keys,
    ).expect("Apply archived config should succeed");

    assert!(restored, "Restore should return true");

    // Verify config.json:
    // - "preserved_key" -> restored to "user_val"
    // - "skipped_key" -> kept author's "author_default" (because it was ignored)
    // - "new_key" -> merged as 42
    let restored_json_str = fs::read_to_string(target_dir.join("config.json")).unwrap();
    let restored_json: serde_json::Value = serde_json::from_str(&restored_json_str).unwrap();
    assert_eq!(restored_json["preserved_key"], "user_val");
    assert_eq!(restored_json["skipped_key"], "author_default");
    assert_eq!(restored_json["new_key"], 42);

    // Verify settings.ini:
    // - Entire file was skipped, so it contains author's fresh defaults (UserSetting=1, NewAuthorSetting=2)
    let restored_ini = fs::read_to_string(target_dir.join("settings.ini")).unwrap();
    assert!(restored_ini.contains("UserSetting=1"));
    assert!(restored_ini.contains("NewAuthorSetting=2"));
    assert!(!restored_ini.contains("UserSetting=100"));
}

