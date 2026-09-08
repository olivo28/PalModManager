use crate::helpers::{TestEnv, ZipBuilder};
use palmodmanager_lib::models::{AppData, ModFolder, ModType};
use palmodmanager_lib::profiles::ensure_default_profile;
use palmodmanager_lib::commands::mod_commands::filter_mods_for_current_profile;

#[test]
fn test_hybrid_mod_update_preserves_profile_visibility_and_virtual_folders() {
    let env = TestEnv::new_steam_win64();

    // 1. Build authentic initial v1.0.0 Hybrid mod (PalInsight: UE4SS + LogicMods PAK)
    let v1_zip_filename = "PalInsight 1.0.0 7777 1.0.0 2026-09-01T10-00Z abcdef123.zip";
    let v1_mod_info = env.install_builder(
        &ZipBuilder::new(v1_zip_filename)
            .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/PalInsight/Scripts/main.lua", "print('PalInsight v1.0 active')")
            .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/PalInsight/enabled.txt", "")
            .add_file("Pal/Content/Paks/LogicMods/PalInsight_P.pak", b"MOCK_LOGICMODS_PAK_V1"),
    ).expect("Initial PalInsight v1.0 installation should succeed");

    assert_eq!(v1_mod_info.mod_type, ModType::Hybrid);
    assert!(!v1_mod_info.game_path.is_empty());
    assert_eq!(v1_mod_info.extra_files.len(), 1);

    // 2. Set up AppData and assign the mod to a virtual folder ("UI Overhauls")
    let mut app_data = AppData::default();
    app_data.settings.game_path = env.game_root.to_string_lossy().to_string();
    app_data.settings.program_path = env.program_data.to_string_lossy().to_string();
    app_data.current_profile_id = "default".to_string();
    app_data.mods = vec![v1_mod_info.clone()];
    ensure_default_profile(&mut app_data);

    let default_profile = app_data.profiles.iter_mut().find(|p| p.id == "default").unwrap();
    default_profile.installed_mod_ids.push(v1_mod_info.name.clone());
    default_profile.enabled_mod_ids.push(v1_mod_info.name.clone());

    // Create virtual folder and place mod in it
    let folder_id = "folder_ui_mods".to_string();
    default_profile.mod_folders.push(ModFolder {
        id: folder_id.clone(),
        name: "UI Overhauls".to_string(),
        mod_ids: vec![v1_mod_info.id.clone()],
    });

    // Verify initial visibility via filter_mods_for_current_profile
    let visible_before = filter_mods_for_current_profile(&app_data);
    assert_eq!(visible_before.len(), 1, "PalInsight must be visible before update");
    assert_eq!(visible_before[0].id, v1_mod_info.id);

    // 3. Build authentic updated v2.0.0 Hybrid mod (reproducing Valdacil's drag-and-drop zip)
    let v2_zip_filename = "PalInsight 2.0.0 7777 2.0.0 2026-09-07T22-00Z ghijkl456.zip";
    let v2_zip_path = ZipBuilder::new(v2_zip_filename)
        .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/PalInsight/Scripts/main.lua", "print('PalInsight v2.0 active with improvements')")
        .add_text_file("Pal/Binaries/Win64/ue4ss/Mods/PalInsight/enabled.txt", "")
        .add_file("Pal/Content/Paks/LogicMods/PalInsight_P.pak", b"MOCK_LOGICMODS_PAK_V2")
        .build_in(&env.temp_dir);

    // 4. Execute the real update_mod pipeline
    let extract_target = std::env::temp_dir().join(format!("pmm_extract_update_{}", uuid::Uuid::new_v4()));
    let extracted_dir = palmodmanager_lib::zip_handler::extract_zip_to_temp(v2_zip_path.to_str().unwrap(), &extract_target)
        .expect("Extraction should succeed");
    let analysis = palmodmanager_lib::zip_handler::analyze_zip(v2_zip_path.to_str().unwrap())
        .expect("Analysis should succeed");

    let mut existing_mod = app_data.mods[0].clone();
    palmodmanager_lib::installer::update_mod(
        &mut existing_mod,
        &env.game_root.to_string_lossy(),
        &env.program_data.to_string_lossy(),
        "default",
        &extracted_dir,
        &analysis,
        v2_zip_filename,
        "2026-09-08T00:00:00Z",
        false,
        false,
    ).expect("installer::update_mod should succeed");

    let _ = std::fs::remove_dir_all(&extract_target);

    // Update app_data.mods with the updated mod
    app_data.mods[0] = existing_mod.clone();

    // Replicate update_mod_command's duplicate purging and profile synchronization
    let final_m = existing_mod.clone();
    let updated_extra_files = final_m.extra_files.clone();
    let updated_nexus_id = final_m.nexus_mod_id;
    let mut purged_ids: Vec<String> = Vec::new();

    app_data.mods.retain(|other| {
        if other.id == final_m.id {
            return true;
        }
        let matches_extra = (!other.game_path.is_empty() && updated_extra_files.contains(&other.game_path))
            || (!other.disabled_path.is_empty() && updated_extra_files.contains(&other.disabled_path));
        let matches_nexus = updated_nexus_id.is_some() && other.nexus_mod_id == updated_nexus_id && other.mod_type != final_m.mod_type;
        if matches_extra || matches_nexus {
            purged_ids.push(other.id.clone());
            false
        } else {
            true
        }
    });

    if !purged_ids.is_empty() {
        for profile in &mut app_data.profiles {
            profile.installed_mod_ids.retain(|id| !purged_ids.contains(id));
            profile.enabled_mod_ids.retain(|id| !purged_ids.contains(id));

            for folder in &mut profile.mod_folders {
                let mut had_purged = false;
                folder.mod_ids.retain(|id| {
                    if purged_ids.contains(id) {
                        had_purged = true;
                        false
                    } else {
                        true
                    }
                });
                if had_purged && !folder.mod_ids.contains(&final_m.id) {
                    folder.mod_ids.push(final_m.id.clone());
                }
            }
        }
    }

    // Guarantee final_m is registered in profile
    if final_m.nexus_author.as_deref() != Some("UE4SS Native Mod") {
        let mod_name = final_m.name.clone();
        let mod_id = final_m.id.clone();
        if let Some(profile) = app_data.profiles.iter_mut().find(|p| p.id == "default") {
            let in_installed = profile.installed_mod_ids.iter().any(|id| {
                id.eq_ignore_ascii_case(&mod_name) || id == &mod_id
            });
            if !in_installed {
                profile.installed_mod_ids.push(mod_name.clone());
            }
            if final_m.enabled {
                let in_enabled = profile.enabled_mod_ids.iter().any(|id| {
                    id.eq_ignore_ascii_case(&mod_name) || id == &mod_id
                });
                if !in_enabled {
                    profile.enabled_mod_ids.push(mod_name.clone());
                }
            }
        }
    }

    palmodmanager_lib::profiles::cleanup_profile_mod_lists(&mut app_data);
    palmodmanager_lib::profiles::sync_current_profile_states(&mut app_data);

    // 5. Verification: Check physical files, profile retention, and folder assignment
    let expected_lua = env.game_root.join("Pal").join("Binaries").join("Win64").join("ue4ss").join("Mods").join("PalInsight").join("Scripts").join("main.lua");
    let expected_pak = env.game_root.join("Pal").join("Content").join("Paks").join("LogicMods").join("PalInsight_P.pak");

    assert!(expected_lua.exists(), "UE4SS script must exist on disk after update");
    assert!(expected_pak.exists(), "LogicMods pak must exist on disk after update (NOT deleted)");

    let updated_profile = app_data.profiles.iter().find(|p| p.id == "default").unwrap();
    assert!(
        updated_profile.installed_mod_ids.iter().any(|entry| palmodmanager_lib::profiles::mod_matches_profile_entry(&final_m, entry)),
        "Updated PalInsight must remain in profile installed_mod_ids"
    );

    let test_folder = updated_profile.mod_folders.iter().find(|f| f.id == folder_id).unwrap();
    assert!(
        test_folder.mod_ids.contains(&final_m.id),
        "Updated PalInsight must remain assigned to its virtual folder"
    );

    // CRITICAL: Filter for current profile must return the mod (NOT disappear from PMM interface!)
    let visible_after = filter_mods_for_current_profile(&app_data);
    assert_eq!(visible_after.len(), 1, "PalInsight MUST be visible in PMM interface after update!");
    assert_eq!(visible_after[0].id, final_m.id);
}
