use std::path::PathBuf;
use palmodmanager_lib::config_merge::{
    generate_config_diff, merge_lua, snapshot_configs, apply_config_merge, ConfigSnapshot,
};

#[test]
fn test_merge_lua_preserves_user_settings() {
    let old_lua = r#"
local Config = {
    ["ShowTimer"] = false,
    ["Scale"] = 1.5,
    ["PosX"] = 250,
    ["PosY"] = 400,
}
return Config
"#;

    let new_lua = r#"
local Config = {
    ["ShowTimer"] = true,
    ["Scale"] = 1.0,
    ["PosX"] = 100,
    ["PosY"] = 200,
    ["NewAuthorFeature"] = true,
}
return Config
"#;

    let diff = generate_config_diff(old_lua, new_lua, "lua").expect("diff should succeed");
    let (user_changed, added, removed) = diff;
    assert_eq!(user_changed.len(), 4);
    assert_eq!(added.len(), 1);
    assert_eq!(added[0], "NewAuthorFeature");
    assert_eq!(removed.len(), 0);

    let merged = merge_lua(old_lua, new_lua, &[]).expect("merge should succeed");
    assert!(merged.contains("[\"ShowTimer\"] = false"));
    assert!(merged.contains("[\"Scale\"] = 1.5"));
    assert!(merged.contains("[\"PosX\"] = 250"));
    assert!(merged.contains("[\"PosY\"] = 400"));
    assert!(merged.contains("[\"NewAuthorFeature\"] = true"));
}

#[test]
fn test_merge_lua_respects_ignored_keys() {
    let old_lua = r#"
Config = {}
Config.Enabled = false
Config.Version = "1.0.0"
"#;
    let new_lua = r#"
Config = {}
Config.Enabled = true
Config.Version = "2.0.0"
"#;
    let ignored = vec!["Version".to_string(), "Config.Version".to_string()];
    let merged = merge_lua(old_lua, new_lua, &ignored).expect("merge should succeed");
    assert!(merged.contains("Config.Enabled = false"));
    assert!(merged.contains("Config.Version = \"2.0.0\""));
}

#[test]
fn test_snapshot_configs_lua_only_when_configured() {
    let temp_dir = std::env::temp_dir().join(format!("pmm_test_{}", uuid::Uuid::new_v4()));
    let scripts_dir = temp_dir.join("Scripts");
    std::fs::create_dir_all(&scripts_dir).unwrap();

    // Create main.lua, config.lua, and .nexus.json
    std::fs::write(scripts_dir.join("main.lua"), "-- main script").unwrap();
    std::fs::write(scripts_dir.join("config.lua"), "return { Setting = 1 }").unwrap();
    std::fs::write(temp_dir.join(".nexus.json"), "{}").unwrap();

    // Scenario 1: No custom_config configured -> main.lua and config.lua must NOT be snapshotted
    let snap1 = snapshot_configs(&temp_dir, None);
    assert!(!snap1.entries.iter().any(|(p, _)| p.to_string_lossy().contains("main.lua")));
    assert!(!snap1.entries.iter().any(|(p, _)| p.to_string_lossy().contains("config.lua")));
    assert!(!snap1.entries.iter().any(|(p, _)| p.to_string_lossy().contains(".nexus.json")));

    // Scenario 2: custom_config set to "config.lua" -> ONLY config.lua snapshotted, NEVER main.lua
    let snap2 = snapshot_configs(&temp_dir, Some("Scripts/config.lua"));
    assert!(!snap2.entries.iter().any(|(p, _)| p.to_string_lossy().contains("main.lua")));
    assert!(!snap2.entries.iter().any(|(p, _)| p.to_string_lossy().contains(".nexus.json")));
    assert!(snap2.entries.iter().any(|(p, _)| p.to_string_lossy().contains("config.lua")));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_apply_config_merge_skips_ignored_file() {
    let temp_dir = std::env::temp_dir().join(format!("pmm_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let cfg_file = temp_dir.join("config.json");
    std::fs::write(&cfg_file, r#"{"setting": "new_from_author"}"#).unwrap();

    let snap = ConfigSnapshot {
        entries: vec![(PathBuf::from("config.json"), r#"{"setting": "old_user_value"}"#.to_string())],
    };

    // If "config.json" is in ignored_keys, it must NOT be merged (keeps new_from_author)
    apply_config_merge(&temp_dir, &snap, &["config.json".to_string()]);
    let content = std::fs::read_to_string(&cfg_file).unwrap();
    assert!(content.contains("new_from_author"));
    assert!(!content.contains("old_user_value"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_merge_lua_preserves_comparison_operators() {
    let old_lua = r#"
Config = {}
Config.Speed = 100
"#;
    let new_lua = r#"
Config = {}
Config.Speed = 50
if Config.Speed == 50 then
    print("default speed")
end
if Config.Mode ~= "debug" then
    print("normal")
end
"#;
    let merged = merge_lua(old_lua, new_lua, &[]).expect("merge should succeed");
    // Should update Config.Speed = 100
    assert!(merged.contains("Config.Speed = 100"));
    // Must NEVER mangle conditional comparisons into assignments
    assert!(merged.contains("if Config.Speed == 50 then"));
    assert!(merged.contains("if Config.Mode ~= \"debug\" then"));
    assert!(!merged.contains("if Config.Speed = 100 then"));
}

#[test]
fn test_snapshot_configs_strictly_blacklists_main_lua() {
    let temp_dir = std::env::temp_dir().join(format!("pmm_test_{}", uuid::Uuid::new_v4()));
    let scripts_dir = temp_dir.join("Scripts");
    std::fs::create_dir_all(&scripts_dir).unwrap();

    std::fs::write(scripts_dir.join("main.lua"), "-- main entry point").unwrap();
    std::fs::write(temp_dir.join("main.lua"), "-- root main entry point").unwrap();

    // Even if explicitly requested as custom_config, main.lua must NEVER be snapshotted
    let snap = snapshot_configs(&temp_dir, Some("main.lua"));
    assert_eq!(snap.entries.len(), 0);

    let snap2 = snapshot_configs(&temp_dir, Some("Scripts/main.lua"));
    assert_eq!(snap2.entries.len(), 0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

