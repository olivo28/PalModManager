use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{AppData, ModType};
use crate::profiles::utils::{
    get_profile_dir, remove_junction_or_symlink,
    move_path, save_pmm_meta,
};
use crate::profiles::core::mod_matches_profile_entry;
use super::folder_name::get_mod_folder_name;
use super::mods_txt::{remove_from_mods_txt, update_mods_txt_load_order};
use super::sync::sync_mods_txt_sections;

pub fn disable_mod_internal(
    data: &mut AppData,
    program_path: &str,
    mod_id: &str,
) -> Result<(), String> {
    let mod_index = data
        .mods
        .iter()
        .position(|m| m.id == mod_id)
        .ok_or("Mod not found")?;
    
    let is_native = data.mods[mod_index].nexus_author.as_deref() == Some("UE4SS Native Mod");
    let mod_type = data.mods[mod_index].mod_type.clone();
    let mod_name = data.mods[mod_index].name.clone();
    let current_profile_id = data.current_profile_id.clone();
    let disabled_base = PathBuf::from(program_path)
        .join("profiles")
        .join(&current_profile_id)
        .join("disabled_mods");
    let _ = fs::create_dir_all(&disabled_base);

    let is_workshop = data.mods[mod_index].nexus_summary.as_deref()
        .map_or(false, |s| s.starts_with("Steam Workshop Mod"));
    if is_workshop {
        let game_path = data.settings.game_path.clone();
        let force_load_order_ue4ss = crate::profiles::effective_force_ue4ss(data);
        let wmods = crate::workshop::scan_workshop_mods(&game_path);
        let target_id = data.mods[mod_index].id.clone();
        let target_name = data.mods[mod_index].name.clone();
        if let Some(target) = wmods.iter().find(|m| m.package_name.eq_ignore_ascii_case(&target_id) || target_name.to_lowercase().starts_with(&m.package_name.to_lowercase())) {
            let _ = crate::workshop::deactivate_workshop_mod(&game_path, target, force_load_order_ue4ss);
        }
        data.mods[mod_index].enabled = false;
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.enabled_mod_ids.retain(|id| !mod_matches_profile_entry(&data.mods[mod_index], id));
        }
        if !program_path.is_empty() {
            let p_dir = get_profile_dir(program_path, &current_profile_id);
            if let Some(profile) = data.profiles.iter().find(|p| p.id == current_profile_id) {
                if let Ok(json) = serde_json::to_string_pretty(profile) {
                    let _ = fs::write(p_dir.join("profile.json"), json);
                }
            }
        }
        return Ok(());
    }

    if is_native {
        let mod_info = &mut data.mods[mod_index];
        let game_dir = PathBuf::from(&mod_info.game_path);
        let mut mods_txt = None;
        if let Some(mods_dir) = game_dir.parent() {
            let path1 = mods_dir.join("mods.txt");
            if path1.exists() {
                mods_txt = Some(path1);
            } else if let Some(parent_dir) = mods_dir.parent() {
                let path2 = parent_dir.join("mods.txt");
                if path2.exists() {
                    mods_txt = Some(path2);
                }
            }
        }
        if let Some(path) = mods_txt {
            let folder_name = get_mod_folder_name(mod_info);
            let _ = update_mods_txt_load_order(&path, &folder_name, false);
            if mod_info.name.to_lowercase() != folder_name.to_lowercase() {
                let _ = remove_from_mods_txt(&path, &mod_info.name);
            }
        }
        mod_info.enabled = false;
    } else if mod_type == ModType::Ue4ss {
        let mod_info = &mut data.mods[mod_index];
        let src_path = PathBuf::from(&mod_info.game_path);
        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let ue4ss_mods_dir = gp.ue4ss_mods_dir.clone();
        let mods_txt = ue4ss_mods_dir.join("mods.txt");

        let is_mods_txt_mode = {
            let current_p = data.profiles.iter().find(|p| p.id == current_profile_id);
            current_p.and_then(|p| p.ue4ss_control_mode.as_deref()).unwrap_or("enabled_txt") == "mods_txt"
        };

        if is_mods_txt_mode {
            if mods_txt.exists() {
                let folder_name = get_mod_folder_name(mod_info);
                let _ = update_mods_txt_load_order(&mods_txt, &folder_name, false);
                if mod_info.name.to_lowercase() != folder_name.to_lowercase() {
                    let _ = remove_from_mods_txt(&mods_txt, &mod_info.name);
                }
            }
            if src_path.exists() {
                let enabled_file = src_path.join("enabled.txt");
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
            }
            mod_info.enabled = false;
        } else if src_path.exists() {
            if let Some(mods_dir) = src_path.parent() {
                let mods_txt = mods_dir.join("mods.txt");
                if mods_txt.exists() {
                    let folder_name = get_mod_folder_name(mod_info);
                    let _ = remove_from_mods_txt(&mods_txt, &folder_name);
                    let _ = remove_from_mods_txt(&mods_txt, &mod_info.name);
                }
            }
            let enabled_file = src_path.join("enabled.txt");
            if enabled_file.exists() {
                let _ = fs::remove_file(&enabled_file);
            }
            
            let file_name = src_path.file_name().unwrap().to_string_lossy().to_string();
            let dest_dir = disabled_base.join("ue4ss");
            let _ = fs::create_dir_all(&dest_dir);
            let dest = dest_dir.join(&file_name);
            move_path(&src_path, &dest)?;
            mod_info.disabled_path = dest.to_string_lossy().to_string();
            mod_info.game_path = String::new();
            mod_info.enabled = false;
        } else {
            mod_info.enabled = false;
        }
    } else if mod_type == ModType::PalSchema {
        let mod_info = &mut data.mods[mod_index];
        let folder_name = get_mod_folder_name(mod_info);
        let src_path = PathBuf::from(&mod_info.game_path);

        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let palschema_mods_dir = gp.palschema_mods_dir.clone();
        let palschema_storage_dir = gp.palschema_storage_dir.clone();

        if palschema_mods_dir.exists() {
            if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    let clean_name = if name.len() > 4 && name[..3].chars().all(|c| c.is_ascii_digit()) && name.as_bytes()[3] == b'_' {
                        &name[4..]
                    } else {
                        &name
                    };
                    if clean_name.to_lowercase() == folder_name.to_lowercase() {
                        let _ = remove_junction_or_symlink(&path);
                    }
                }
            }
        }

        let storage_path = palschema_storage_dir.join(&folder_name);
        let final_src = if storage_path.exists() {
            storage_path
        } else if src_path.exists() {
            src_path
        } else {
            palschema_mods_dir.join(&folder_name)
        };

        if final_src.exists() {
            let dest_dir = disabled_base.join("palschema");
            let _ = fs::create_dir_all(&dest_dir);
            let dest = dest_dir.join(&folder_name);
            move_path(&final_src, &dest)?;
            mod_info.disabled_path = dest.to_string_lossy().to_string();
            mod_info.game_path = String::new();
        }
        mod_info.enabled = false;
    } else if mod_type == ModType::Pak || mod_type == ModType::LogicMods || mod_type == ModType::Altermatic {
        let mod_info = &mut data.mods[mod_index];
        let src_path = PathBuf::from(&mod_info.game_path);
        let mut moved_files = Vec::new();
        let mut moved_extras = Vec::new();

        if src_path.exists() {
            if let Some(parent) = src_path.parent() {
                let file_stem = src_path.file_stem().unwrap().to_string_lossy().to_string();
                let type_dir = match mod_type {
                    ModType::LogicMods => "logicmods",
                    ModType::Altermatic => "altermatic",
                    _ => "pak",
                };
                let dest_dir = disabled_base.join(type_dir);
                let _ = fs::create_dir_all(&dest_dir);

                for ext in &["pak", "ucas", "utoc"] {
                    let companion = parent.join(format!("{}.{}", file_stem, ext));
                    if companion.exists() {
                        let dest = dest_dir.join(format!("{}.{}", file_stem, ext));
                        move_path(&companion, &dest)?;
                        moved_files.push(dest.to_string_lossy().to_string());
                    }
                }
                let sidecar = parent.join(format!("{}.pak.pmm.json", file_stem));
                if sidecar.exists() {
                    let dest = dest_dir.join(format!("{}.pak.pmm.json", file_stem));
                    let _ = move_path(&sidecar, &dest);
                }
            }
            mod_info.disabled_path = moved_files.first().cloned().unwrap_or_default();
            mod_info.game_path = String::new();
        }

        if mod_type == ModType::Altermatic {
            let swap_dest_dir = disabled_base.join("altermatic").join("SwapJSON");
            let _ = fs::create_dir_all(&swap_dest_dir);

            if let Some(cfg) = &mod_info.config_path {
                let cfg_path = PathBuf::from(cfg);
                if cfg_path.exists() {
                    let filename = cfg_path.file_name().unwrap().to_string_lossy().to_string();
                    let dest = swap_dest_dir.join(&filename);
                    if let Ok(_) = move_path(&cfg_path, &dest) {
                        mod_info.config_path = Some(dest.to_string_lossy().to_string());
                    }
                }
            }

            for extra in &mod_info.extra_files {
                let extra_path = PathBuf::from(extra);
                if extra_path.exists() {
                    let filename = extra_path.file_name().unwrap().to_string_lossy().to_string();
                    let dest = if extra.to_lowercase().contains("swapjson") {
                        swap_dest_dir.join(&filename)
                    } else {
                        disabled_base.join("altermatic").join(&filename)
                    };
                    let _ = fs::create_dir_all(dest.parent().unwrap());
                    if let Ok(_) = move_path(&extra_path, &dest) {
                        moved_extras.push(dest.to_string_lossy().to_string());
                    }
                }
            }
            mod_info.extra_files = moved_extras;
        } else {
            mod_info.extra_files = moved_files.into_iter().skip(1).collect();
        }

        mod_info.enabled = false;
    } else if mod_type == ModType::Hybrid {
        let mod_info = &mut data.mods[mod_index];
        let src_path = PathBuf::from(&mod_info.game_path);
        
        if let Some(ue4ss_mods_dir) = src_path.parent() {
            let mods_txt = ue4ss_mods_dir.join("mods.txt");
            if mods_txt.exists() {
                for extra in &mod_info.extra_files {
                    let extra_path = PathBuf::from(extra);
                    if extra_path.exists() && extra_path.is_dir() {
                        let extra_folder_name = extra_path.file_name().unwrap().to_string_lossy().to_string();
                        let _ = remove_from_mods_txt(&mods_txt, &extra_folder_name);
                        let enabled_file = extra_path.join("enabled.txt");
                        if enabled_file.exists() {
                            let _ = fs::remove_file(&enabled_file);
                        }
                    }
                }
            }
        }

        let mut moved_extras = Vec::new();
        for extra in &mod_info.extra_files {
            let extra_path = PathBuf::from(extra);
            if extra_path.exists() {
                let file_name = extra_path.file_name().unwrap().to_string_lossy().to_string();
                
                let is_logic = extra.to_lowercase().contains("logicmods");
                let is_palschema = extra.to_lowercase().contains("palschema");
                let type_dir = if is_logic {
                    "logicmods"
                } else if is_palschema {
                    "palschema"
                } else if extra_path.extension().map(|e| e == "pak").unwrap_or(false) {
                    "pak"
                } else {
                    "ue4ss"
                };

                let dest_dir = disabled_base.join("hybrid").join(type_dir);
                let _ = fs::create_dir_all(&dest_dir);
                
                if extra_path.is_dir() {
                    let dest = dest_dir.join(&file_name);
                    move_path(&extra_path, &dest)?;
                    moved_extras.push(dest.to_string_lossy().to_string());
                } else {
                    let parent = extra_path.parent().unwrap();
                    let stem = extra_path.file_stem().unwrap().to_string_lossy().to_string();
                    let dest = dest_dir.join(&file_name);
                    move_path(&extra_path, &dest)?;
                    moved_extras.push(dest.to_string_lossy().to_string());
                    
                    for c_ext in &["ucas", "utoc"] {
                        let companion = parent.join(format!("{}.{}", stem, c_ext));
                        if companion.exists() {
                            let c_dest = dest_dir.join(format!("{}.{}", stem, c_ext));
                            let _ = move_path(&companion, &c_dest);
                        }
                    }
                    if !file_name.ends_with(".pmm.json") {
                        let sidecar = parent.join(format!("{}.pmm.json", file_name));
                        if sidecar.exists() {
                            let c_dest = dest_dir.join(format!("{}.pmm.json", file_name));
                            let _ = move_path(&sidecar, &c_dest);
                        }
                    }
                }
            }
        }

        if src_path.exists() {
            if let Some(ue4ss_mods_dir) = src_path.parent() {
                let mods_txt = ue4ss_mods_dir.join("mods.txt");
                if mods_txt.exists() {
                    let folder_name = get_mod_folder_name(mod_info);
                    let _ = remove_from_mods_txt(&mods_txt, &folder_name);
                    let _ = remove_from_mods_txt(&mods_txt, &mod_info.name);
                }
            }
            let enabled_file = src_path.join("enabled.txt");
            if enabled_file.exists() {
                let _ = fs::remove_file(&enabled_file);
            }
            
            let file_name = src_path.file_name().unwrap().to_string_lossy().to_string();
            let dest_dir = disabled_base.join("hybrid");
            let _ = fs::create_dir_all(&dest_dir);
            let dest = dest_dir.join(&file_name);
            move_path(&src_path, &dest)?;
            
            mod_info.disabled_path = dest.to_string_lossy().to_string();
            mod_info.game_path = String::new();
        }
        mod_info.extra_files = moved_extras;
        mod_info.enabled = false;
    } else {
        return Ok(());
    }

    // Remove from active profile's enabled_mod_ids and persist profile.json
    let current_id = data.current_profile_id.clone();
    if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_id) {
        if let Some(mod_info) = data.mods.iter().find(|m| m.id == mod_id) {
            profile.enabled_mod_ids.retain(|entry| {
                !crate::profiles::mod_matches_profile_entry(mod_info, entry)
            });
        } else {
            profile.enabled_mod_ids.retain(|id| {
                id.to_lowercase() != mod_id.to_lowercase() && 
                id.to_lowercase() != mod_name.to_lowercase()
            });
        }
    }
    if !program_path.is_empty() {
        let p_dir = get_profile_dir(program_path, &current_id);
        if let Some(profile) = data.profiles.iter().find(|p| p.id == current_id) {
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
    }

    // Save updated mod metadata in .pmm.json
    if let Some(mod_info) = data.mods.iter().find(|m| m.id == mod_id) {
        let _ = save_pmm_meta(mod_info);
    }

    let is_mods_txt_mode = {
        let current_p = data.profiles.iter().find(|p| p.id == current_id);
        current_p.and_then(|p| p.ue4ss_control_mode.as_deref()).unwrap_or("enabled_txt") == "mods_txt"
    };
    if is_mods_txt_mode && !data.settings.game_path.is_empty() {
        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let mods_txt = gp.ue4ss_mods_dir.join("mods.txt");
        if mods_txt.exists() {
            if let Some(profile) = data.profiles.iter().find(|p| p.id == current_id) {
                let _ = sync_mods_txt_sections(&mods_txt, profile, &data.mods);
            }
        }
    }

    Ok(())
}
