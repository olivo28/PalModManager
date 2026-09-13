use super::*;
use std::path::Path;

#[test]
fn test_parse_ue4ss_log_mod_lifecycle_and_errors() {
    let sample_log = r#"[2024-02-15 14:22:01.123] [UE4SS] Initializing UE4SS v3.0.1
[2024-02-15 14:22:02.456] Starting C++ mod 'PalSchema'
[2024-02-15 14:22:02.490] [PalSchema] PalSchema v0.6.5 by Okaetsu loaded.
[2024-02-15 14:22:02.500] [PalSchema] Loading mod: ZZZ_MelwenMods - Upgradable Pal Spheres
[2024-02-15 14:22:02.600] Starting Lua mod 'BPModLoaderMod'
[2024-02-15 14:22:02.700] Mod 'LineTraceMod' disabled in mods.txt.
[2024-02-15 14:22:04.200] [Warning] Mod 'OldMod' uses deprecated function
[2024-02-15 14:22:05.300] [Error] Mods/CustomPal/scripts/main.lua:42: attempt to call a nil value
[2024-02-15 14:22:06.400] [Crash] Unhandled exception at 0x00007FF7
[2024-02-15 14:22:07.000] Failed to load C++ mod ModIntegratedStorageCpp, dlls folder must contain either main.dll or ModIntegratedStorageCpp.dll
[2024-02-15 14:22:07.100] Failed to load dll <E:\PalSchema\dlls\main.dll> for mod PalSchema, error: [0x7f]
[2024-02-15 14:22:07.200] Was unable to install mod 'ModIntegratedStorageCpp' for unknown reasons. Mod is not installable.
"#;

    let dummy_path = Path::new("dummy_ue4ss.log");
    let diag = parse_ue4ss_log_content(sample_log, dummy_path);

    assert_eq!(diag.total_lines, 12);
    assert_eq!(diag.warning_count, 1);
    assert_eq!(diag.error_count, 5); // 1 error + 1 crash + 3 load/install failures

    // Check loaded mods
    let mod_names: Vec<String> = diag.loaded_mods.iter().map(|m| m.name.clone()).collect();
    assert!(mod_names.contains(&"PalSchema".to_string()));
    assert!(mod_names.contains(&"ZZZ_MelwenMods - Upgradable Pal Spheres".to_string()));
    assert!(mod_names.contains(&"BPModLoaderMod".to_string()));
    assert!(mod_names.contains(&"LineTraceMod".to_string()));
    assert!(mod_names.contains(&"ModIntegratedStorageCpp".to_string()));

    let melwen = diag.loaded_mods.iter().find(|m| m.name == "ZZZ_MelwenMods - Upgradable Pal Spheres").unwrap();
    assert_eq!(melwen.status, "loaded");
    assert_eq!(melwen.mod_type.as_deref(), Some("palschema"));

    let palschema = diag.loaded_mods.iter().find(|m| m.name == "PalSchema").unwrap();
    assert_eq!(palschema.status, "failed");

    let storage_mod = diag.loaded_mods.iter().find(|m| m.name == "ModIntegratedStorageCpp").unwrap();
    assert_eq!(storage_mod.status, "failed");

    let bpmod = diag.loaded_mods.iter().find(|m| m.name == "BPModLoaderMod").unwrap();
    assert_eq!(bpmod.mod_type.as_deref(), Some("lua"));

    let disabled = diag.loaded_mods.iter().find(|m| m.name == "LineTraceMod").unwrap();
    assert_eq!(disabled.status, "disabled");
    assert_eq!(disabled.mod_type.as_deref(), Some("lua"));

    // Check Lua script jump target extraction
    let lua_error = diag.entries.iter().find(|e| e.line_number == 8).expect("Line 8 entry");
    assert_eq!(lua_error.level, "error");
    assert_eq!(lua_error.mod_name.as_deref(), Some("CustomPal"));
    assert_eq!(lua_error.script_file.as_deref(), Some("scripts/main.lua"));
    assert_eq!(lua_error.script_line, Some(42));
}

#[test]
fn test_truncate_preserves_early_errors_and_warnings() {
    let entries = (1..=2155).map(|i| Ue4ssLogEntry {
        line_number: i,
        timestamp: None,
        level: if i == 8 { "error" } else if i == 966 { "warning" } else { "info" }.to_string(),
        tag: None,
        message: format!("Log line {i}"),
        mod_name: None,
        script_file: None,
        script_line: None,
    }).collect();

    let truncated = truncate_entries_preserving_severity(entries, 2000);
    assert_eq!(truncated.len(), 2000);
    assert!(truncated.iter().any(|e| e.line_number == 8 && e.level == "error"));
    assert!(truncated.iter().any(|e| e.line_number == 966 && e.level == "warning"));
    assert!(truncated.windows(2).all(|w| w[0].line_number < w[1].line_number));
}
