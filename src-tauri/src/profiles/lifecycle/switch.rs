use std::fs;
use std::path::Path;
use crate::models::{AppData, ModInfo, Profile};
use super::super::utils::ensure_profile_structure;
use super::super::core::{cleanup_profile_enabled_ids, sync_current_profile_states};
use super::backup_restore::{backup_game_files_to_profile, restore_profile_files_to_game};

pub fn switch_profile(
    data: &mut AppData,
    program_path: &str,
    target_profile: &Profile,
) -> Result<Vec<ModInfo>, String> {
    crate::logger::log(&format!("switch_profile: Switching to profile '{}' (folder: {}, ue4ss_enabled: {}, dependency_mode: {:?})", target_profile.name, target_profile.id, target_profile.ue4ss_enabled, target_profile.dependency_mode));

    let game_path = data.settings.game_path.clone();

    let current_id = data.current_profile_id.clone();
    if !current_id.is_empty() {
        let current_dir = ensure_profile_structure(program_path, &current_id);
        if let Some(current_profile) = data.profiles.iter().find(|p| p.id == current_id).cloned() {
            backup_game_files_to_profile(&game_path, &current_dir, &current_profile);
        }
        
        if !game_path.is_empty() {
            let ue4ss_mods_dir = crate::dependency_checker::get_ue4ss_mods_dir(Path::new(&game_path));
            let mods_txt = ue4ss_mods_dir.join("mods.txt");
            if mods_txt.exists() {
                let snapshot_path = current_dir.join("mods.txt.snapshot");
                let _ = fs::copy(&mods_txt, &snapshot_path);
            }
        }
    }

    let target_dir = ensure_profile_structure(program_path, &target_profile.id);
    let force_palschema = data.settings.force_load_order.unwrap_or(false) && target_profile.force_load_order_palschema
        .or(data.settings.force_load_order_palschema)
        .unwrap_or(false);
    restore_profile_files_to_game(&game_path, &target_dir, target_profile, program_path, force_palschema);

    if !game_path.is_empty() {
        let ue4ss_mods_dir = crate::dependency_checker::get_ue4ss_mods_dir(Path::new(&game_path));
        let mods_txt = ue4ss_mods_dir.join("mods.txt");
        let snapshot_path = target_dir.join("mods.txt.snapshot");
        if snapshot_path.exists() {
            let _ = fs::copy(&snapshot_path, &mods_txt);
        } else {
            let ws_backup = target_dir.join("ue4ss_workshop_mods").join("mods.txt");
            let std_backup = target_dir.join("ue4ss_mods").join("mods.txt");
            if ws_backup.exists() {
                let _ = fs::copy(&ws_backup, &mods_txt);
            } else if std_backup.exists() {
                let _ = fs::copy(&std_backup, &mods_txt);
            }
        }
    }

    data.current_profile_id = target_profile.id.clone();
    cleanup_profile_enabled_ids(data);
    sync_current_profile_states(data);

    let target_mode = target_profile.ue4ss_control_mode.as_deref()
        .unwrap_or("enabled_txt")
        .to_string();
    let _ = crate::profiles::reconcile_ue4ss_control_mode(data, program_path, &target_mode);

    if let Ok(json) = serde_json::to_string_pretty(target_profile) {
        let _ = fs::write(target_dir.join("profile.json"), json);
    }

    Ok(data.mods.clone())
}
