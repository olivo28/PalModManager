use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{AppData, ModType};
use crate::profiles::utils::move_path;
use super::folder_name::get_mod_folder_name;
use super::sync::{clean_mods_txt_native_only, sync_mods_txt_sections};

pub fn reconcile_ue4ss_control_mode(
    data: &mut AppData,
    program_path: &str,
    target_mode: &str,
) -> Result<(), String> {
    if data.settings.game_path.is_empty() {
        return Ok(());
    }

    let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
    let ue4ss_mods_dir = gp.ue4ss_mods_dir.clone();
    let mods_txt = ue4ss_mods_dir.join("mods.txt");
    let current_profile_id = data.current_profile_id.clone();
    let disabled_base = PathBuf::from(program_path)
        .join("profiles")
        .join(&current_profile_id)
        .join("disabled_mods")
        .join("ue4ss");

    let current_profile = data.profiles.iter().find(|p| p.id == current_profile_id).cloned();
    let installed_ids = current_profile.as_ref().map(|p| p.installed_mod_ids.clone()).unwrap_or_default();
    let enabled_ids = current_profile.as_ref().map(|p| p.enabled_mod_ids.clone()).unwrap_or_default();

    if target_mode == "mods_txt" {
        // 1. Restore disabled UE4SS mods belonging to THIS profile from disabled_mods/ue4ss back into ue4ss/Mods
        for mod_info in data.mods.iter_mut() {
            if (mod_info.mod_type == ModType::Ue4ss || mod_info.mod_type == ModType::Hybrid)
                && mod_info.nexus_author.as_deref() != Some("UE4SS Native Mod")
            {
                let is_in_profile = installed_ids.iter().any(|entry| crate::profiles::mod_matches_profile_entry(mod_info, entry));
                if !is_in_profile {
                    continue;
                }

                if !mod_info.disabled_path.is_empty() {
                    let d_path = PathBuf::from(&mod_info.disabled_path);
                    if d_path.exists() {
                        let folder_name = d_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let active_dest = ue4ss_mods_dir.join(&folder_name);
                        let _ = fs::create_dir_all(&ue4ss_mods_dir);
                        let _ = move_path(&d_path, &active_dest);
                        mod_info.game_path = active_dest.to_string_lossy().to_string();
                        mod_info.disabled_path = String::new();
                    }
                } else if mod_info.game_path.is_empty() {
                    let folder_name = get_mod_folder_name(mod_info);
                    let active_dest = ue4ss_mods_dir.join(&folder_name);
                    if active_dest.exists() {
                        mod_info.game_path = active_dest.to_string_lossy().to_string();
                    }
                }
            }
        }

        // 2. Remove enabled.txt from ALL UE4SS mod folders in ue4ss/Mods
        if ue4ss_mods_dir.exists() {
            if let Ok(entries) = fs::read_dir(&ue4ss_mods_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let enabled_file = path.join("enabled.txt");
                        if enabled_file.exists() {
                            let _ = fs::remove_file(&enabled_file);
                        }
                    }
                }
            }
        }

        // 3. Synchronize mods.txt strictly with UE4SS mods in this profile, organized by virtual folders!
        if mods_txt.exists() {
            if let Some(ref prof) = current_profile {
                let _ = sync_mods_txt_sections(&mods_txt, prof, &data.mods);
            }
        }
    } else {
        // target_mode == "enabled_txt" (PMM Native mode)
        let _ = fs::create_dir_all(&disabled_base);

        for mod_info in data.mods.iter_mut() {
            if (mod_info.mod_type == ModType::Ue4ss || mod_info.mod_type == ModType::Hybrid)
                && mod_info.nexus_author.as_deref() != Some("UE4SS Native Mod")
            {
                let is_in_profile = installed_ids.iter().any(|entry| crate::profiles::mod_matches_profile_entry(mod_info, entry));
                if !is_in_profile {
                    continue;
                }

                let is_enabled = mod_info.enabled && (enabled_ids.is_empty() || enabled_ids.iter().any(|id| id.to_lowercase() == mod_info.id.to_lowercase() || id.to_lowercase() == mod_info.name.to_lowercase()));

                if is_enabled {
                    // Make sure folder is in game_path
                    if mod_info.game_path.is_empty() && !mod_info.disabled_path.is_empty() {
                        let d_path = PathBuf::from(&mod_info.disabled_path);
                        if d_path.exists() {
                            let folder_name = d_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let active_dest = ue4ss_mods_dir.join(&folder_name);
                            let _ = move_path(&d_path, &active_dest);
                            mod_info.game_path = active_dest.to_string_lossy().to_string();
                            mod_info.disabled_path = String::new();
                        }
                    }
                    // Create enabled.txt
                    if !mod_info.game_path.is_empty() {
                        let active_dir = PathBuf::from(&mod_info.game_path);
                        if active_dir.exists() {
                            let enabled_file = active_dir.join("enabled.txt");
                            if !enabled_file.exists() {
                                let _ = fs::write(&enabled_file, "");
                            }
                        }
                    }
                } else {
                    // Disabled mod: remove enabled.txt and move folder to disabled_mods
                    let src_path = if !mod_info.game_path.is_empty() {
                        PathBuf::from(&mod_info.game_path)
                    } else {
                        ue4ss_mods_dir.join(get_mod_folder_name(mod_info))
                    };

                    if src_path.exists() {
                        let enabled_file = src_path.join("enabled.txt");
                        if enabled_file.exists() {
                            let _ = fs::remove_file(&enabled_file);
                        }
                        let file_name = src_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let dest = disabled_base.join(&file_name);
                        let _ = move_path(&src_path, &dest);
                        mod_info.disabled_path = dest.to_string_lossy().to_string();
                        mod_info.game_path = String::new();
                    }
                }
            }
        }

        // In enabled_txt mode, if Force Load Order UE4SS is NOT active, completely clean mods.txt to native tools only!
        let is_flo = data.settings.force_load_order_ue4ss.unwrap_or(false);
        if !is_flo && mods_txt.exists() {
            let _ = clean_mods_txt_native_only(&mods_txt);
        }
    }

    Ok(())
}
