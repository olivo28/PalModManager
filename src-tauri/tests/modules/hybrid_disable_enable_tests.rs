use std::path::Path;
use crate::helpers::{TestEnv, ZipBuilder};
use palmodmanager_lib::models::{AppData, ModType};
use palmodmanager_lib::profiles::{
    disable_mod_internal, enable_mod_internal, ensure_default_profile,
};

#[test]
fn test_hybrid_ue4ss_and_logicmods_disable_enable_lifecycle() {
    let env = TestEnv::new_steam_win64();

    // 1. Build an authentic Hybrid mod ZIP: UE4SS script + LogicMods .pak
    // (Reproducing Expedition Timer On-Screen HUD structure)
    let zip_filename = "ExpeditionTimerHUD-1.0.0-4567-1-0-0-1700000000.zip";
    let mod_info = env.install_builder(
        &ZipBuilder::new(zip_filename)
            .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/ExpeditionTimerHUD/Scripts/main.lua", "print('ExpeditionTimer active')")
            .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/ExpeditionTimerHUD/enabled.txt", "")
            .add_file("Pal/Content/Paks/LogicMods/ExpeditionTimerHUD_P.pak", b"MOCK_LOGICMODS_PAK"),
    ).expect("Hybrid mod installation should succeed");

    assert_eq!(mod_info.mod_type, ModType::Hybrid, "Must be classified as Hybrid");
    assert!(!mod_info.game_path.is_empty(), "Primary game path must not be empty");
    assert_eq!(mod_info.extra_files.len(), 1, "Must have exactly 1 extra file (LogicMods PAK)");
    assert!(mod_info.extra_files[0].contains("LogicMods"), "Extra file must point to LogicMods");

    // Verify files exist on disk before disable
    assert!(Path::new(&mod_info.game_path).exists(), "UE4SS directory must exist in game");
    assert!(Path::new(&mod_info.extra_files[0]).exists(), "LogicMods PAK must exist in game");

    // 2. Set up AppData
    let mut app_data = AppData::default();
    app_data.settings.game_path = env.game_root.to_string_lossy().to_string();
    app_data.settings.program_path = env.program_data.to_string_lossy().to_string();
    app_data.mods = vec![mod_info.clone()];
    ensure_default_profile(&mut app_data);

    // 3. Disable the hybrid mod (simulating "All Off" or individual toggle Off)
    disable_mod_internal(&mut app_data, &env.program_data.to_string_lossy(), &mod_info.id)
        .expect("disable_mod_internal should succeed");

    let disabled_mod = &app_data.mods[0];
    assert!(!disabled_mod.enabled, "Mod must be marked disabled");
    assert!(disabled_mod.game_path.is_empty(), "game_path must be empty when disabled");
    assert!(!disabled_mod.disabled_path.is_empty(), "disabled_path must point to disabled folder");
    assert_eq!(disabled_mod.extra_files.len(), 1, "Must retain 1 disabled extra file");
    assert!(disabled_mod.extra_files[0].contains("disabled_mods"), "extra file must be moved to disabled_mods");
    assert!(disabled_mod.extra_files[0].contains("logicmods"), "extra file must be in hybrid/logicmods");
    assert!(Path::new(&disabled_mod.extra_files[0]).exists(), "Disabled PAK must exist on disk in disabled_mods");

    // 4. Run real full-disk scanner while disabled (reproducing loadMods execution after disable)
    let scanned_mods = env.scan_mods_with_db(&app_data.mods);
    let scanned_hybrid = scanned_mods.iter().find(|m| m.id == mod_info.id)
        .expect("Scanned mods must find the disabled hybrid mod");

    assert!(!scanned_hybrid.enabled, "Scanned disabled mod must remain disabled");
    assert!(scanned_hybrid.game_path.is_empty(), "game_path must stay empty in scanner for disabled mod");
    assert_eq!(scanned_hybrid.extra_files.len(), 1, "Scanner must not clobber or drop the disabled LogicMods extra file");
    assert!(scanned_hybrid.extra_files[0].contains("logicmods"), "extra_files must preserve disabled logicmods path");

    // Update app_data with scanned state
    app_data.mods = scanned_mods;

    // 5. Re-enable the hybrid mod (simulating "All On" or individual toggle On)
    enable_mod_internal(&mut app_data, &env.program_data.to_string_lossy(), &mod_info.id)
        .expect("enable_mod_internal should succeed");

    let re_enabled_mod = &app_data.mods[0];
    assert!(re_enabled_mod.enabled, "Mod must be marked enabled");
    assert!(!re_enabled_mod.game_path.is_empty(), "Primary game_path must be restored");
    assert!(re_enabled_mod.disabled_path.is_empty(), "disabled_path must be cleared");
    assert_eq!(re_enabled_mod.extra_files.len(), 1, "Must have 1 restored extra file");

    // 6. Assert physical disk state: 100% restored to target game directories
    let expected_ue4ss = env.game_root.join("Pal").join("Binaries").join("Win64").join("ue4ss").join("Mods").join("ExpeditionTimerHUD");
    let expected_pak = env.game_root.join("Pal").join("Content").join("Paks").join("LogicMods").join("ExpeditionTimerHUD_P.pak");

    assert!(expected_ue4ss.exists(), "UE4SS component must be restored to ue4ss/Mods/");
    assert!(expected_pak.exists(), "LogicMods PAK must be restored to Pal/Content/Paks/LogicMods/");

    // Assert zero leftover files in disabled_mods/
    let disabled_base = env.program_data.join("profiles").join("default").join("disabled_mods");
    let disabled_pak = disabled_base.join("hybrid").join("logicmods").join("ExpeditionTimerHUD_P.pak");
    assert!(!disabled_pak.exists(), "Disabled PAK must not remain stranded in disabled_mods/hybrid/logicmods/");
}

#[test]
fn test_hybrid_ue4ss_palschema_and_mods_pak_disable_enable_lifecycle() {
    let env = TestEnv::new_steam_win64();

    // 1. Build an authentic Hybrid mod ZIP: UE4SS + PalSchema + ~mods PAK
    // (Reproducing Passive Trait Extraction structure)
    let zip_filename = "PassiveTraitExtraction-2.0.0-5678-2-0-0-1700000000.zip";
    let mod_info = env.install_builder(
        &ZipBuilder::new(zip_filename)
            .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/PassiveTraitExtraction/Scripts/main.lua", "print('PassiveTraitExtraction active')")
            .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/PalSchema/mods/PassiveTraitExtraction/schema.json", "{}")
            .add_file("Pal/Content/Paks/~mods/PassiveTraitExtraction_P.pak", b"MOCK_MODS_PAK"),
    ).expect("Hybrid mod installation should succeed");

    assert_eq!(mod_info.mod_type, ModType::Hybrid);
    assert_eq!(mod_info.extra_files.len(), 2, "Must have 2 extra files (PalSchema folder and ~mods PAK)");

    // 2. Set up AppData
    let mut app_data = AppData::default();
    app_data.settings.game_path = env.game_root.to_string_lossy().to_string();
    app_data.settings.program_path = env.program_data.to_string_lossy().to_string();
    app_data.mods = vec![mod_info.clone()];
    ensure_default_profile(&mut app_data);

    // 3. Disable the hybrid mod
    disable_mod_internal(&mut app_data, &env.program_data.to_string_lossy(), &mod_info.id)
        .expect("disable_mod_internal should succeed");

    // 4. Run real full-disk scanner while disabled
    let scanned_mods = env.scan_mods_with_db(&app_data.mods);
    let scanned_hybrid = scanned_mods.iter().find(|m| m.id == mod_info.id)
        .expect("Scanned mods must find the disabled hybrid mod");

    assert!(!scanned_hybrid.enabled);
    assert!(scanned_hybrid.game_path.is_empty(), "game_path must not be populated with a disabled extra component");
    assert_eq!(scanned_hybrid.extra_files.len(), 2, "Both PalSchema and PAK must be preserved in extra_files");

    app_data.mods = scanned_mods;

    // 5. Re-enable the hybrid mod
    enable_mod_internal(&mut app_data, &env.program_data.to_string_lossy(), &mod_info.id)
        .expect("enable_mod_internal should succeed");

    // 6. Assert physical disk state: all 3 components restored
    let expected_ue4ss = env.game_root.join("Pal").join("Binaries").join("Win64").join("ue4ss").join("Mods").join("PassiveTraitExtraction");
    let expected_palschema = env.game_root.join("Pal").join("Binaries").join("Win64").join("ue4ss").join("Mods").join("PalSchema").join("mods").join("PassiveTraitExtraction");
    let expected_pak = env.game_root.join("Pal").join("Content").join("Paks").join("~mods").join("PassiveTraitExtraction_P.pak");

    assert!(expected_ue4ss.exists(), "UE4SS component must be restored to ue4ss/Mods/");
    assert!(expected_palschema.exists(), "PalSchema component must be restored to PalSchema/mods/");
    assert!(expected_pak.exists(), "PAK must be restored to Pal/Content/Paks/~mods/");

    // Assert zero leftovers in disabled_mods
    let disabled_base = env.program_data.join("profiles").join("default").join("disabled_mods");
    assert!(!disabled_base.join("hybrid").join("palschema").join("PassiveTraitExtraction").exists());
    assert!(!disabled_base.join("hybrid").join("pak").join("PassiveTraitExtraction_P.pak").exists());
}

#[test]
fn test_hybrid_fallback_recovery_for_orphaned_components() {
    let env = TestEnv::new_steam_win64();

    // 1. Build and install ExpeditionTimerHUD
    let zip_filename = "ExpeditionTimerHUD-1.0.0-4567-1-0-0-1700000000.zip";
    let mod_info = env.install_builder(
        &ZipBuilder::new(zip_filename)
            .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/ExpeditionTimerHUD/Scripts/main.lua", "print('ExpeditionTimer active')")
            .add_file("Pal/Content/Paks/LogicMods/ExpeditionTimerHUD_P.pak", b"MOCK_LOGICMODS_PAK"),
    ).expect("Installation should succeed");

    let mut app_data = AppData::default();
    app_data.settings.game_path = env.game_root.to_string_lossy().to_string();
    app_data.settings.program_path = env.program_data.to_string_lossy().to_string();
    app_data.mods = vec![mod_info.clone()];
    ensure_default_profile(&mut app_data);

    // 2. Disable mod
    disable_mod_internal(&mut app_data, &env.program_data.to_string_lossy(), &mod_info.id).unwrap();

    // 3. Intentionally wipe extra_files to simulate legacy corrupted metadata state
    app_data.mods[0].extra_files.clear();

    // 4. Enable mod — fallback recovery must scan disabled_mods/hybrid/ and recover the LogicMods pak
    enable_mod_internal(&mut app_data, &env.program_data.to_string_lossy(), &mod_info.id)
        .expect("enable_mod_internal should succeed with fallback recovery");

    let expected_pak = env.game_root.join("Pal").join("Content").join("Paks").join("LogicMods").join("ExpeditionTimerHUD_P.pak");
    assert!(expected_pak.exists(), "Fallback recovery must restore orphaned LogicMods pak to game directory");
    assert!(app_data.mods[0].extra_files.iter().any(|f| f.contains("LogicMods")), "extra_files must include recovered path");
}
