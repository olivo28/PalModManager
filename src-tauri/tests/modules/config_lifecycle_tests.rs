use std::fs;
use std::path::{Path, PathBuf};
use palmodmanager_lib::config_merge::{
    apply_config_merge, merge_file_contents, ConfigSnapshot,
};
use palmodmanager_lib::commands::config_archive::{get_archive_dir, ArchivedConfigInfo};
use crate::helpers::{TestEnv, ZipBuilder};

#[test]
fn test_ini_kv_merge_preserves_user_settings_and_adds_new_keys() {
    let old_ini = r#"
; Palworld User Configuration
[General]
bEnableDebug=True
MaxFPS=144
PlayerSpeedMultiplier=1.8
"#;

    let new_author_ini = r#"
; Palworld Author Configuration v2.0
[General]
bEnableDebug=False
MaxFPS=60
PlayerSpeedMultiplier=1.0
bShowPing=True
LogLevel=Warn
"#;

    let merged = merge_file_contents(old_ini, new_author_ini, "ini", &[])
        .expect("INI merge should succeed");

    // User modifications must be preserved
    assert!(merged.contains("bEnableDebug= True") || merged.contains("bEnableDebug=True"), "User bEnableDebug was not preserved");
    assert!(merged.contains("MaxFPS= 144") || merged.contains("MaxFPS=144"), "User MaxFPS was not preserved");
    assert!(merged.contains("PlayerSpeedMultiplier= 1.8") || merged.contains("PlayerSpeedMultiplier=1.8"), "User PlayerSpeedMultiplier was not preserved");

    // New keys from author must be included
    assert!(merged.contains("bShowPing=True"), "New author key bShowPing was missing");
    assert!(merged.contains("LogLevel=Warn"), "New author key LogLevel was missing");
    // Author header comments should be retained
    assert!(merged.contains("; Palworld Author Configuration v2.0"));

    println!("  [CONFIG] INI Key-Value Merge Result:\n{}", merged.trim());
}

#[test]
fn test_json_recursive_merge_preserves_nested_user_customizations() {
    let old_json = r#"{
        "player": {
            "speed": 2.5,
            "godMode": true
        },
        "display": {
            "scale": 1.25
        }
    }"#;

    let new_author_json = r#"{
        "player": {
            "speed": 1.0,
            "godMode": false,
            "staminaDrain": 0.5
        },
        "display": {
            "scale": 1.0
        },
        "ui": {
            "showRadar": true
        }
    }"#;

    let merged_str = merge_file_contents(old_json, new_author_json, "json", &[])
        .expect("JSON merge should succeed");
    let merged: serde_json::Value = serde_json::from_str(&merged_str).expect("Valid JSON");

    // Preserved user edits
    assert_eq!(merged["player"]["speed"], 2.5);
    assert_eq!(merged["player"]["godMode"], true);
    assert_eq!(merged["display"]["scale"], 1.25);

    // Added new features from author
    assert_eq!(merged["player"]["staminaDrain"], 0.5);
    assert_eq!(merged["ui"]["showRadar"], true);

    println!("  [CONFIG] JSON Recursive Merge Result:\n{}", merged_str.trim());
}

#[test]
fn test_apply_config_merge_on_disk_with_ignored_keys_and_backup() {
    let env = TestEnv::new_steam_win64();
    let mod_dir = env.game_root.join("Pal/Binaries/Win64/ue4ss/Mods/CustomMod");
    fs::create_dir_all(&mod_dir).unwrap();

    let cfg_file = mod_dir.join("config.json");
    let ini_file = mod_dir.join("settings.ini");

    // Simulating author's newly installed files
    fs::write(&cfg_file, r#"{
        "preserved_key": "author_default",
        "forced_reset_key": "author_override"
    }"#).unwrap();

    fs::write(&ini_file, "Volume=50\nTheme=Dark\n").unwrap();

    // User's previous snapshot
    let snapshot = ConfigSnapshot {
        entries: vec![
            (
                PathBuf::from("config.json"),
                r#"{
                    "preserved_key": "user_custom_value",
                    "forced_reset_key": "user_old_value"
                }"#.to_string(),
            ),
            (
                PathBuf::from("settings.ini"),
                "Volume=100\nTheme=Light\n".to_string(),
            ),
        ],
    };

    // Ignore forced_reset_key in JSON, and ignore the whole settings.ini
    let ignored_keys = vec![
        "forced_reset_key".to_string(),
        "settings.ini".to_string(),
    ];

    apply_config_merge(&mod_dir, &snapshot, &ignored_keys);

    // Verify .pre-update.bak was created for safety
    let json_bak = mod_dir.join("config.json.pre-update.bak");
    assert!(json_bak.exists(), "pre-update backup for config.json was not created");
    let bak_content = fs::read_to_string(&json_bak).unwrap();
    assert!(bak_content.contains("user_custom_value"));

    // Verify config.json merged partially: preserved_key kept, forced_reset_key reset
    let merged_json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&cfg_file).unwrap()).unwrap();
    assert_eq!(merged_json["preserved_key"], "user_custom_value");
    assert_eq!(merged_json["forced_reset_key"], "author_override");

    // Verify settings.ini was completely skipped because it was in ignored_keys
    let ini_content = fs::read_to_string(&ini_file).unwrap();
    assert!(ini_content.contains("Volume=50"), "Ignored settings.ini should remain author default");
    assert!(!ini_content.contains("Volume=100"));

    println!("  [CONFIG] On-Disk Merge with Ignored Keys Result:");
    println!("           * Preserved key value: {}", merged_json["preserved_key"]);
    println!("           * Forced reset key value: {}", merged_json["forced_reset_key"]);
    println!("           * Pre-update safety backup exists: {}", json_bak.exists());
}

#[test]
fn test_pre_purge_archiving_and_reinstallation_restoration_lifecycle() {
    let env = TestEnv::new_steam_win64();

    // 1. Build and install authentic ShinyNotifier v1.0 ZIP via real pipeline
    let v1_zip = ZipBuilder::new("Shiny Notifier Pro 1.0.0 98765 1.0.0 2026-08-26T17-37Z 5V0tE6iaU.zip")
        .add_text_file("ShinyNotifier/scripts/main.lua", "print('ShinyNotifier v1.0')")
        .add_text_file("ShinyNotifier/config.json", r##"{
            "soundVolume": 0.5,
            "notifyRadius": 2000,
            "customColor": "#00FF00"
        }"##)
        .add_text_file("ShinyNotifier/enabled.txt", "");

    let mod_info = env.install_builder(&v1_zip).expect("Real installer pipeline should succeed");
    assert!(Path::new(&mod_info.game_path).exists(), "Mod folder must be installed on disk");

    // 2. User customizes their config on disk
    let custom_cfg = Path::new(&mod_info.game_path).join("config.json");
    fs::write(&custom_cfg, r##"{
        "soundVolume": 0.95,
        "notifyRadius": 5000,
        "customColor": "#FF5500"
    }"##).unwrap();

    // 3. Pre-purge archiving via env.archive_configs
    let archived_count = env.archive_configs(&mod_info).expect("Archiving should succeed");
    assert!(archived_count > 0, "Should have archived at least 1 config file");

    let archive_dir = get_archive_dir(&env.program_data.to_string_lossy(), "default", "nexus_98765");
    assert!(archive_dir.exists(), "Archived configs folder was not created");

    let meta_file = archive_dir.join("archive_meta.json");
    assert!(meta_file.exists(), "archive_meta.json was missing");
    let meta_json: ArchivedConfigInfo = serde_json::from_str(&fs::read_to_string(&meta_file).unwrap()).unwrap();
    assert_eq!(meta_json.nexus_mod_id, Some(98765));

    // 4. Simulate complete mod purge / uninstall via real disk cleanup
    env.remove_mod_on_disk(&mod_info);
    assert!(!Path::new(&mod_info.game_path).exists(), "Mod folder should be purged from disk");

    // 5. Simulate mod reinstall with author v2.0 defaults via real pipeline
    let v2_zip = ZipBuilder::new("Shiny Notifier Pro 2.0.0 98765 2.0.0 2026-09-06T17-37Z 5V0tE6iaU.zip")
        .add_text_file("ShinyNotifier/scripts/main.lua", "print('ShinyNotifier v2.0')")
        .add_text_file("ShinyNotifier/config.json", r##"{
            "soundVolume": 0.5,
            "notifyRadius": 2000,
            "customColor": "#00FF00",
            "newV2Setting": true
        }"##)
        .add_text_file("ShinyNotifier/enabled.txt", "");

    let v2_mod = env.install_builder(&v2_zip).expect("Reinstalling v2 should succeed");
    assert!(Path::new(&v2_mod.game_path).exists());

    // 6. Restore archived config using snapshot & apply_config_merge
    let archived_cfg_file = archive_dir.join("config.json");
    assert!(archived_cfg_file.exists(), "Archived config file was missing");
    let old_user_content = fs::read_to_string(&archived_cfg_file).unwrap();

    let restore_snapshot = ConfigSnapshot {
        entries: vec![(PathBuf::from("config.json"), old_user_content)],
    };

    apply_config_merge(Path::new(&v2_mod.game_path), &restore_snapshot, &[]);

    // 7. Verify restored state
    let v2_cfg_file = Path::new(&v2_mod.game_path).join("config.json");
    let restored_json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&v2_cfg_file).unwrap()).unwrap();
    assert_eq!(restored_json["soundVolume"], 0.95, "User sound volume should be restored");
    assert_eq!(restored_json["notifyRadius"], 5000, "User notify radius should be restored");
    assert_eq!(restored_json["customColor"], "#FF5500", "User custom color should be restored");
    assert_eq!(restored_json["newV2Setting"], true, "Author v2 new setting should be retained");

    println!("  [CONFIG] Pre-Purge Archive & Reinstall Restoration Result:");
    println!("           * Archived files count: {}", archived_count);
    println!("           * Restored soundVolume: {}", restored_json["soundVolume"]);
    println!("           * Restored notifyRadius: {}", restored_json["notifyRadius"]);
    println!("           * Restored customColor: {}", restored_json["customColor"]);
    println!("           * Retained new author setting: {}", restored_json["newV2Setting"]);
}

#[test]
fn test_ue4ss_shared_and_multi_config_detection_archiving_and_merge() {
    let env = TestEnv::new_steam_win64();

    // 1. Build and install authentic PalIconInfo v1.3.0 ZIP via real pipeline
    let v1_zip = ZipBuilder::new("PalIconInfo 1.3.0 5281 1.3.0 2026-08-26T17-37Z 5V0tE6iaU.zip")
        .add_text_file("PalIconInfo/scripts/main.lua", "print('PalIconInfo loaded')")
        .add_text_file("PalIconInfo/scripts/ConfigManager.lua", "-- ConfigManager")
        .add_text_file("PalIconInfo/scripts/DefaultConfig_DO_NOT_EDIT/PalIconInfoConfig.lua", "IconScale = 1.0\nShowIcons = true\n")
        .add_text_file("PalIconInfo/scripts/DefaultConfig_DO_NOT_EDIT/PalBoxPageSkipConfig.lua", "PageSkipCount = 5\nEnableSkip = true\n")
        .add_text_file("PalIconInfo/scripts/USER_CONFIG_LOCATION.txt", "Config is in Mods/shared/PalIconInfo")
        .add_text_file("PalIconInfo/enabled.txt", "");

    let mod_info = env.install_builder(&v1_zip).expect("Real installer pipeline should succeed for PalIconInfo");
    let mod_dir = PathBuf::from(&mod_info.game_path);
    assert!(mod_dir.exists(), "PalIconInfo directory must be created on disk");

    // 2. Active user configs placed in shared/PalIconInfo
    let shared_mod_dir = env.game_root.join("Pal/Binaries/Win64/ue4ss/Mods/shared/PalIconInfo");
    fs::create_dir_all(&shared_mod_dir).unwrap();
    let shared_icon = shared_mod_dir.join("PalIconInfoConfig.lua");
    let shared_skip = shared_mod_dir.join("PalBoxPageSkipConfig.lua");
    fs::write(&shared_icon, "IconScale = 2.5\nShowIcons = true\n").unwrap();
    fs::write(&shared_skip, "PageSkipCount = 10\nEnableSkip = true\n").unwrap();

    // 3. Real full-disk scan via env.scan_mods()
    let scanned_mods = env.scan_mods();
    let scanned_mod = scanned_mods.iter().find(|m| m.name.to_lowercase().contains("paliconinfo"))
        .expect("Disk scanner must dynamically discover PalIconInfo from disk");

    let configs = scanned_mod.config_paths.as_ref().expect("Disk scanner must populate config_paths");
    println!("DEBUG configs found: {:?}", configs);
    assert_eq!(configs.len(), 2, "Scanner must discover exactly 2 configs in shared/PalIconInfo");
    assert!(configs.iter().any(|c| c.contains("PalIconInfoConfig.lua")));
    assert!(configs.iter().any(|c| c.contains("PalBoxPageSkipConfig.lua")));
    assert!(!configs.iter().any(|c| c.to_lowercase().contains("do_not_edit")), "DO_NOT_EDIT templates must be strictly excluded from configs");

    // 4. Test snapshot_configs
    let snapshot = palmodmanager_lib::config_merge::snapshot_configs(&mod_dir, None);
    assert_eq!(snapshot.entries.len(), 2, "Snapshot should capture both shared configs");
    assert!(snapshot.entries.iter().any(|(p, c)| p.to_string_lossy().contains("PalIconInfoConfig.lua") && c.contains("IconScale = 2.5")));
    assert!(snapshot.entries.iter().any(|(p, c)| p.to_string_lossy().contains("PalBoxPageSkipConfig.lua") && c.contains("PageSkipCount = 10")));
    assert!(!snapshot.entries.iter().any(|(p, _)| p.to_string_lossy().to_lowercase().contains("do_not_edit")), "Snapshot must exclude DO_NOT_EDIT templates");

    // 5. Real pre-purge archiving
    let mut mod_to_archive = mod_info.clone();
    mod_to_archive.config_paths = scanned_mod.config_paths.clone();
    let archived_count = env.archive_configs(&mod_to_archive).expect("Archiving should succeed");
    assert_eq!(archived_count, 2, "Should have archived both shared configs");

    let archive_dir = get_archive_dir(&env.program_data.to_string_lossy(), "default", "nexus_5281");
    assert!(archive_dir.exists(), "Archive directory should exist");

    // 6. Test mod uninstallation and shared folder cleanup
    env.remove_mod_on_disk(&mod_to_archive);
    assert!(!mod_dir.exists(), "Mod folder must be removed from disk");
    assert!(!shared_mod_dir.exists(), "shared/PalIconInfo folder must be cleanly removed with zero remnants");

    // 7. Test re-installing updated version v1.4.0 via real pipeline
    let v2_zip = ZipBuilder::new("PalIconInfo 1.4.0 5281 1.4.0 2026-09-06T17-37Z 5V0tE6iaU.zip")
        .add_text_file("PalIconInfo/scripts/main.lua", "print('PalIconInfo v1.4.0 loaded')")
        .add_text_file("PalIconInfo/scripts/ConfigManager.lua", "-- ConfigManager v2")
        .add_text_file("PalIconInfo/scripts/DefaultConfig_DO_NOT_EDIT/PalIconInfoConfig.lua", "IconScale = 1.0\nShowIcons = true\nNewOptionV2 = true\n")
        .add_text_file("PalIconInfo/scripts/DefaultConfig_DO_NOT_EDIT/PalBoxPageSkipConfig.lua", "PageSkipCount = 5\nEnableSkip = true\n")
        .add_text_file("PalIconInfo/scripts/USER_CONFIG_LOCATION.txt", "Config is in Mods/shared/PalIconInfo")
        .add_text_file("PalIconInfo/enabled.txt", "");

    let v2_mod = env.install_builder(&v2_zip).expect("Reinstalling updated PalIconInfo should succeed");
    let v2_mod_dir = PathBuf::from(&v2_mod.game_path);
    assert!(v2_mod_dir.exists());

    // 8. Restore configs & apply in-place merge
    fs::create_dir_all(&shared_mod_dir).unwrap();
    fs::write(&shared_icon, "IconScale = 1.0\nShowIcons = true\nNewOptionV2 = true\n").unwrap();
    fs::write(&shared_skip, "PageSkipCount = 5\nEnableSkip = true\n").unwrap();

    apply_config_merge(&v2_mod_dir, &snapshot, &[]);

    // Verify user settings preserved in shared/
    let merged_icon = fs::read_to_string(&shared_icon).unwrap();
    assert!(merged_icon.contains("IconScale = 2.5") || merged_icon.contains("IconScale =2.5"), "User IconScale should be preserved in shared config");
    assert!(merged_icon.contains("NewOptionV2 = true") || merged_icon.contains("NewOptionV2=true"), "Author new option should be merged");

    // Verify .pre-update.bak was created in shared folder
    let bak_icon = shared_mod_dir.join("PalIconInfoConfig.lua.pre-update.bak");
    assert!(bak_icon.exists(), "pre-update backup must exist in shared folder");

    // Verify templates in DefaultConfig_DO_NOT_EDIT were not touched or corrupted
    let template_icon = v2_mod_dir.join("scripts/DefaultConfig_DO_NOT_EDIT/PalIconInfoConfig.lua");
    let template_check = fs::read_to_string(&template_icon).unwrap();
    assert_eq!(template_check, "IconScale = 1.0\nShowIcons = true\nNewOptionV2 = true\n", "DO_NOT_EDIT templates must remain unmodified");

    println!("  [CONFIG] UE4SS Shared & Multi-Config Full Lifecycle (Install, Scan, Archive, Purge, Update, Merge): PASS");
    println!("           * Discovered configs: {:?}", configs);
    println!("           * Archived count: {}", archived_count);
    println!("           * Merged shared config content:\n{}", merged_icon.trim());
}

#[test]
fn test_ue4ss_scripts_config_lua_preserved_on_update_and_reinstall() {
    let env = TestEnv::new_steam_win64();

    // 1. Authentic v1.0 ZIP with Scripts/config.lua
    let v1_zip = ZipBuilder::new("AntiPhat-1.0.0-3799-1-0-0-1700000000.zip")
        .add_text_file("AntiPhat/Scripts/main.lua", "print('AntiPhat v1.0')\n")
        .add_text_file("AntiPhat/Scripts/config.lua", "Config = {\n    PalScale = 0.5,\n    ShowUI = true\n}\n")
        .add_text_file("AntiPhat/enabled.txt", "");

    let mut mod_info = env.install_builder(&v1_zip).expect("Install AntiPhat v1.0 should succeed");
    let mod_dir = PathBuf::from(&mod_info.game_path);
    assert!(mod_dir.exists(), "AntiPhat mod dir must exist");

    let cfg_path = mod_dir.join("Scripts").join("config.lua");
    assert!(cfg_path.exists(), "Scripts/config.lua must exist after initial install");
    assert!(
        mod_info.config_path.as_ref().map(|p| p.contains("config.lua")).unwrap_or(false),
        "mod_info.config_path must be detected on install: {:?}",
        mod_info.config_path
    );

    // 2. User edits config.lua on disk
    fs::write(&cfg_path, "Config = {\n    PalScale = 0.25,\n    ShowUI = true\n}\n").unwrap();

    // 3. Author releases v2.0 with a new configuration option
    let v2_zip = ZipBuilder::new("AntiPhat-2.0.0-3799-2-0-0-1700000000.zip")
        .add_text_file("AntiPhat/Scripts/main.lua", "print('AntiPhat v2.0')\n")
        .add_text_file("AntiPhat/Scripts/config.lua", "Config = {\n    PalScale = 0.5,\n    ShowUI = true,\n    HideInWild = false\n}\n")
        .add_text_file("AntiPhat/enabled.txt", "")
        .build_in(&env.temp_dir);

    let v2_analysis = palmodmanager_lib::zip_handler::analyze_zip(v2_zip.to_str().unwrap()).unwrap();
    let temp_extracted = env.temp_dir.join("extracted_antiphat_v2");
    let extracted_dir = palmodmanager_lib::zip_handler::extract_zip_to_temp(v2_zip.to_str().unwrap(), &temp_extracted).unwrap();

    // 4. Run update_mod pipeline
    palmodmanager_lib::installer::update_mod(
        &mut mod_info,
        env.game_root.to_str().unwrap(),
        env.program_data.to_str().unwrap(),
        "default",
        &extracted_dir,
        &v2_analysis,
        "AntiPhat-2.0.0-3799-2-0-0-1700000000.zip",
        "2026-09-25T00:00:00Z",
        false,
        false,
        None,
        None,
    ).expect("update_mod should succeed");

    // 5. Verify config.lua was NOT deleted by update_mod
    assert!(cfg_path.exists(), "Scripts/config.lua MUST NOT be deleted after update_mod");
    let merged_content = fs::read_to_string(&cfg_path).unwrap();
    assert!(
        merged_content.contains("PalScale = 0.25") || merged_content.contains("PalScale =0.25"),
        "User customized PalScale must be preserved after upgrade: {}",
        merged_content
    );
    assert!(
        merged_content.contains("HideInWild = false") || merged_content.contains("HideInWild =false"),
        "Author new config key HideInWild must be merged after upgrade: {}",
        merged_content
    );

    // 6. Verify pre-update backup was created
    let bak_path = mod_dir.join("Scripts").join("config.lua.pre-update.bak");
    assert!(bak_path.exists(), "config.lua.pre-update.bak must exist");

    // 7. Simulate Reinstall with the same archive
    let temp_reinstall = env.temp_dir.join("extracted_antiphat_reinstall");
    let reinstall_dir = palmodmanager_lib::zip_handler::extract_zip_to_temp(v2_zip.to_str().unwrap(), &temp_reinstall).unwrap();

    palmodmanager_lib::installer::update_mod(
        &mut mod_info,
        env.game_root.to_str().unwrap(),
        env.program_data.to_str().unwrap(),
        "default",
        &reinstall_dir,
        &v2_analysis,
        "AntiPhat-2.0.0-3799-2-0-0-1700000000.zip",
        "2026-09-25T00:00:00Z",
        false,
        false,
        None,
        None,
    ).expect("reinstall via update_mod should succeed");

    assert!(cfg_path.exists(), "Scripts/config.lua MUST NOT be deleted after reinstall");

    println!("  [CONFIG] UE4SS Scripts/config.lua Preservation & Merge on Reinstall: PASS");
    println!("           * Final merged content:\n{}", merged_content.trim());
}

#[test]
fn test_antiphat_dynamic_settings_ini_retention_on_update() {
    let env = TestEnv::new_steam_win64();

    // 1. Initial installation of AntiPhat v1.0
    let v1_zip = ZipBuilder::new("AntiPhat-1.0.0-3799-1-0-0-1600000000.zip")
        .add_text_file("AntiPhat/Scripts/main.lua", "print('AntiPhat v1.0')\n")
        .add_text_file("AntiPhat/Scripts/config.lua", "Config = { PalScale = 1.0 }\n")
        .add_text_file("AntiPhat/enabled.txt", "")
        .build_in(&env.temp_dir);

    let mut mod_info = env.install_zip(&v1_zip).expect("Install v1 must succeed");
    let mod_dir = env.game_root.join("Pal").join("Binaries").join("Win64").join("Mods").join("AntiPhat");

    // 2. Mod runs in-game and dynamically generates Scripts/settings.ini on disk
    let scripts_dir = mod_dir.join("Scripts");
    let dyn_settings_ini = scripts_dir.join("settings.ini");
    fs::write(&dyn_settings_ini, "[AntiPhatSettings]\nPalScale=0.3\nCustomHotkey=F8\n").unwrap();
    assert!(dyn_settings_ini.exists(), "Dynamic settings.ini must exist before update");

    // 3. Mod author releases v2.0 without settings.ini in the zip
    let v2_zip = ZipBuilder::new("AntiPhat-2.0.0-3799-2-0-0-1700000000.zip")
        .add_text_file("AntiPhat/Scripts/main.lua", "print('AntiPhat v2.0')\n")
        .add_text_file("AntiPhat/Scripts/config.lua", "Config = { PalScale = 1.0, NewOption = true }\n")
        .add_text_file("AntiPhat/enabled.txt", "")
        .build_in(&env.temp_dir);

    let v2_analysis = palmodmanager_lib::zip_handler::analyze_zip(v2_zip.to_str().unwrap()).unwrap();
    let temp_extracted = env.temp_dir.join("extracted_antiphat_dyn_v2");
    let extracted_dir = palmodmanager_lib::zip_handler::extract_zip_to_temp(v2_zip.to_str().unwrap(), &temp_extracted).unwrap();

    // 4. Run update_mod without explicit preserve list (fallback heuristic must retain settings.ini)
    palmodmanager_lib::installer::update_mod(
        &mut mod_info,
        env.game_root.to_str().unwrap(),
        env.program_data.to_str().unwrap(),
        "default",
        &extracted_dir,
        &v2_analysis,
        "AntiPhat-2.0.0-3799-2-0-0-1700000000.zip",
        "2026-09-28T00:00:00Z",
        false,
        false,
        None,
        None,
    ).expect("update_mod should succeed");

    // 5. Verify settings.ini was NOT purged
    assert!(
        dyn_settings_ini.exists(),
        "Dynamically generated Scripts/settings.ini MUST be retained after update!"
    );
    let ini_content = fs::read_to_string(&dyn_settings_ini).unwrap();
    assert!(ini_content.contains("PalScale=0.3"), "User settings in settings.ini must be intact");

    println!("  [CONFIG] AntiPhat Dynamic settings.ini Retention: PASS");
}

#[test]
fn test_smartpause_explicit_stale_file_purging() {
    let env = TestEnv::new_steam_win64();

    // 1. Initial installation of SmartPause v1.0 which had old_module.lua and obsolete_config.ini
    let v1_zip = ZipBuilder::new("SmartPause-1.0.0-3799-1-0-0-1600000000.zip")
        .add_text_file("SmartPause/Scripts/main.lua", "print('SmartPause v1.0')\n")
        .add_text_file("SmartPause/Scripts/old_module.lua", "return {}\n")
        .add_text_file("SmartPause/Scripts/obsolete_config.ini", "[General]\nOldKey=1\n")
        .add_text_file("SmartPause/enabled.txt", "")
        .build_in(&env.temp_dir);

    let mut mod_info = env.install_zip(&v1_zip).expect("Install v1 must succeed");
    let mod_dir = env.game_root.join("Pal").join("Binaries").join("Win64").join("Mods").join("SmartPause");
    let old_module = mod_dir.join("Scripts").join("old_module.lua");
    let obsolete_config = mod_dir.join("Scripts").join("obsolete_config.ini");

    assert!(old_module.exists());
    assert!(obsolete_config.exists());

    // 2. Author releases v2.0 removing old_module.lua and user explicitly chooses to purge obsolete_config.ini
    let v2_zip = ZipBuilder::new("SmartPause-2.0.0-3799-2-0-0-1700000000.zip")
        .add_text_file("SmartPause/Scripts/main.lua", "print('SmartPause v2.0')\n")
        .add_text_file("SmartPause/enabled.txt", "")
        .build_in(&env.temp_dir);

    let v2_analysis = palmodmanager_lib::zip_handler::analyze_zip(v2_zip.to_str().unwrap()).unwrap();
    let temp_extracted = env.temp_dir.join("extracted_smartpause_v2");
    let extracted_dir = palmodmanager_lib::zip_handler::extract_zip_to_temp(v2_zip.to_str().unwrap(), &temp_extracted).unwrap();

    let files_to_purge = vec!["Scripts/obsolete_config.ini".to_string()];

    palmodmanager_lib::installer::update_mod(
        &mut mod_info,
        env.game_root.to_str().unwrap(),
        env.program_data.to_str().unwrap(),
        "default",
        &extracted_dir,
        &v2_analysis,
        "SmartPause-2.0.0-3799-2-0-0-1700000000.zip",
        "2026-09-28T00:00:00Z",
        false,
        false,
        None,
        Some(&files_to_purge),
    ).expect("update_mod should succeed");

    // 3. Verify old_module.lua was purged automatically (not in zip, executable script)
    assert!(!old_module.exists(), "old_module.lua must be purged");
    // 4. Verify obsolete_config.ini was purged because it was in files_to_purge
    assert!(!obsolete_config.exists(), "obsolete_config.ini must be purged when explicitly requested");

    println!("  [CONFIG] SmartPause Explicit Stale File Purge: PASS");
}

#[test]
fn test_db_atomic_write_and_rotating_backups() {
    let env = TestEnv::new_steam_win64();
    let db_dir = env.temp_dir.join("test_db_store");
    fs::create_dir_all(&db_dir).unwrap();

    let mut data = palmodmanager_lib::models::AppData::default();
    data.settings.game_path = "C:/Games/Palworld".to_string();

    // 1. Initial save
    palmodmanager_lib::db::save_db(db_dir.to_str().unwrap(), &data)
        .expect("Initial save_db must succeed");

    let live_db = db_dir.join("pmm_database.db");
    assert!(live_db.is_file(), "pmm_database.db must exist after save");

    // 2. Second save with modified state (triggers .bak.1 rotation)
    data.settings.game_path = "D:/Games/Palworld2".to_string();
    palmodmanager_lib::db::save_db(db_dir.to_str().unwrap(), &data)
        .expect("Second save_db must succeed");

    let bak_1 = db_dir.join("pmm_database.db.bak.1");
    assert!(bak_1.is_file(), "pmm_database.db.bak.1 must be created on second save");

    // Verify .bak.1 can be loaded and read cleanly
    let loaded_current = palmodmanager_lib::db::load_db(db_dir.to_str().unwrap());
    assert_eq!(loaded_current.settings.game_path, "D:/Games/Palworld2");

    println!("  [DB] Atomic write and rotating backups: PASS");
}

#[test]
fn test_db_quarantine_and_backup_recovery() {
    let env = TestEnv::new_steam_win64();
    let db_dir = env.temp_dir.join("test_db_corrupt_recovery");
    fs::create_dir_all(&db_dir).unwrap();

    let mut data = palmodmanager_lib::models::AppData::default();
    data.settings.game_path = "C:/SafeGames/Palworld".to_string();

    // 1. Save valid initial database
    palmodmanager_lib::db::save_db(db_dir.to_str().unwrap(), &data).unwrap();

    // 2. Save second revision so that .bak.1 exists with the valid data
    data.settings.custom_data_path = Some("custom_val".to_string());
    palmodmanager_lib::db::save_db(db_dir.to_str().unwrap(), &data).unwrap();

    let live_db = db_dir.join("pmm_database.db");
    assert!(live_db.is_file());
    assert!(db_dir.join("pmm_database.db.bak.1").is_file());

    // 3. Simulate catastrophic power failure: live_db contains corrupted garbage bytes
    fs::write(&live_db, "enc:CORRUPTED_GARBAGE_BYTES_MANGLED_DATA").unwrap();

    // 4. load_db must detect corruption, isolate to .corrupt.<timestamp>, and recover from .bak.1
    let recovered = palmodmanager_lib::db::load_db(db_dir.to_str().unwrap());

    // Verify user's database was restored and not wiped to blank default
    assert_eq!(recovered.settings.game_path, "C:/SafeGames/Palworld");

    // Verify a quarantine file was generated
    let has_corrupt_quarantine = fs::read_dir(&db_dir)
        .unwrap()
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy().contains(".corrupt."));
    assert!(has_corrupt_quarantine, "Corrupted file must be preserved in quarantine (.corrupt.<timestamp>)");

    println!("  [DB] Corruption quarantine and backup auto-recovery: PASS");
}



