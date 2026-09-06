use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{AppData, ModType};
use crate::profiles::utils::{
    get_profile_dir, remove_junction_or_symlink, create_junction_or_symlink,
    move_path, save_pmm_meta,
};
use crate::profiles::effective_force_ue4ss;
use super::folder_name::get_mod_folder_name;
use super::mods_txt::{remove_from_mods_txt, update_mods_txt_load_order};
use super::sync::sync_mods_txt_sections;

pub fn enable_mod_internal(
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
    let force_ue4ss_effective = effective_force_ue4ss(data);
    let force_palschema_effective = crate::profiles::effective_force_palschema(data);
    let game_paks = PathBuf::from(&data.settings.game_path).join("Pal").join("Content").join("Paks");
    let disabled_base = PathBuf::from(program_path)
        .join("profiles")
        .join(&data.current_profile_id)
        .join("disabled_mods");

    let is_workshop = data.mods[mod_index].nexus_summary.as_deref()
        .map_or(false, |s| s.starts_with("Steam Workshop Mod"));
    if is_workshop {
        let game_path = data.settings.game_path.clone();
        let force_load_order_ue4ss = effective_force_ue4ss(data);
        let wmods = crate::workshop::scan_workshop_mods(&game_path);
        let target_id = data.mods[mod_index].id.clone();
        let target_name = data.mods[mod_index].name.clone();
        if let Some(target) = wmods.iter().find(|m| m.package_name.eq_ignore_ascii_case(&target_id) || target_name.to_lowercase().starts_with(&m.package_name.to_lowercase())) {
            let _ = crate::workshop::activate_workshop_mod(&game_path, target, force_load_order_ue4ss);
        }
        data.mods[mod_index].enabled = true;
        let current_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_id) {
            let mod_name = data.mods[mod_index].name.clone();
            let mod_id = data.mods[mod_index].id.clone();
            if !profile.installed_mod_ids.iter().any(|id| id.eq_ignore_ascii_case(&mod_id) || id.eq_ignore_ascii_case(&mod_name)) {
                profile.installed_mod_ids.push(mod_id.clone());
            }
            if !profile.enabled_mod_ids.iter().any(|id| id.eq_ignore_ascii_case(&mod_id) || id.eq_ignore_ascii_case(&mod_name)) {
                profile.enabled_mod_ids.push(mod_id);
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
            let _ = update_mods_txt_load_order(&path, &folder_name, true);
        }
        mod_info.enabled = true;
    } else if mod_type == ModType::Ue4ss {
        let mod_info = &mut data.mods[mod_index];
        let primary_disabled = PathBuf::from(&mod_info.disabled_path);
        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let dest_dir = gp.ue4ss_mods_dir.clone();
        let mods_txt = dest_dir.join("mods.txt");

        let is_mods_txt_mode = {
            let current_p = data.profiles.iter().find(|p| p.id == data.current_profile_id);
            current_p.and_then(|p| p.ue4ss_control_mode.as_deref()).unwrap_or("enabled_txt") == "mods_txt"
        };

        if is_mods_txt_mode {
            let folder_name = get_mod_folder_name(mod_info);
            if primary_disabled.exists() {
                let filename = primary_disabled.file_name().unwrap().to_string_lossy().to_string();
                let dest = dest_dir.join(&filename);
                let _ = fs::create_dir_all(&dest_dir);
                let _ = move_path(&primary_disabled, &dest);
                mod_info.game_path = dest.to_string_lossy().to_string();
                mod_info.disabled_path = String::new();
            } else if mod_info.game_path.is_empty() {
                let dest = dest_dir.join(&folder_name);
                if dest.exists() {
                    mod_info.game_path = dest.to_string_lossy().to_string();
                }
            }
            if !mod_info.game_path.is_empty() {
                let enabled_file = Path::new(&mod_info.game_path).join("enabled.txt");
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
            }
            if mods_txt.exists() {
                let _ = update_mods_txt_load_order(&mods_txt, &folder_name, true);
                if mod_info.name.to_lowercase() != folder_name.to_lowercase() {
                    let _ = remove_from_mods_txt(&mods_txt, &mod_info.name);
                }
            }
            mod_info.enabled = true;
        } else if primary_disabled.exists() {
            let filename = primary_disabled.file_name().unwrap().to_string_lossy().to_string();
            let dest = dest_dir.join(&filename);
            let _ = fs::create_dir_all(&dest_dir);
            move_path(&primary_disabled, &dest)?;

            let force_order = data.settings.force_load_order.unwrap_or(false) && force_ue4ss_effective;
            let origin_mods_txt = mod_info.origin_load_method.as_deref() == Some("mods_txt");
            let mods_txt = dest_dir.join("mods.txt");
            if mods_txt.exists() {
                let folder_name = get_mod_folder_name(mod_info);
                if force_order || origin_mods_txt {
                    let _ = update_mods_txt_load_order(&mods_txt, &folder_name, true);
                } else {
                    let _ = remove_from_mods_txt(&mods_txt, &folder_name);
                }
            }
            let enabled_file = dest.join("enabled.txt");
            if force_order {
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
            } else if origin_mods_txt {
                // If it originated from mods.txt, don't force enabled.txt unless it had one
                if mod_info.has_enabled_txt {
                    let _ = fs::write(&enabled_file, "");
                } else if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
            } else {
                let _ = fs::write(&enabled_file, "");
            }

            mod_info.game_path = dest.to_string_lossy().to_string();
            mod_info.disabled_path = String::new();
            mod_info.enabled = true;
        } else {
            mod_info.enabled = true;
        }
    } else if mod_type == ModType::PalSchema {
        let mod_info = &mut data.mods[mod_index];
        let primary_disabled = PathBuf::from(&mod_info.disabled_path);
        
        let folder_name = get_mod_folder_name(mod_info);
        let force_order = data.settings.force_load_order.unwrap_or(false) && force_palschema_effective;

        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let palschema_mods_dir = gp.palschema_mods_dir.clone();
        let palschema_storage_dir = gp.palschema_storage_dir.clone();

        if primary_disabled.exists() {
            let _ = fs::create_dir_all(&palschema_mods_dir);
            if force_order {
                let storage_dest = palschema_storage_dir.join(&folder_name);
                let _ = fs::create_dir_all(&palschema_storage_dir);
                move_path(&primary_disabled, &storage_dest)?;

                // Calculate next available numeric prefix
                let next_order = palschema_mods_dir
                    .read_dir()
                    .map(|rd| {
                        rd.flatten()
                            .filter(|e| {
                                let n = e.file_name().to_string_lossy().to_string();
                                n.len() > 4 && n[..3].chars().all(|c| c.is_ascii_digit()) && n.as_bytes()[3] == b'_'
                            })
                            .count() as u32
                            + 1
                    })
                    .unwrap_or(1);

                let link_name = format!("{:03}_{}", next_order, folder_name);
                let link_path = palschema_mods_dir.join(&link_name);
                let _ = remove_junction_or_symlink(&link_path);
                create_junction_or_symlink(&storage_dest, &link_path)?;

                mod_info.game_path = link_path.to_string_lossy().to_string();
            } else {
                let dest = palschema_mods_dir.join(&folder_name);
                move_path(&primary_disabled, &dest)?;

                mod_info.game_path = dest.to_string_lossy().to_string();
            }
            mod_info.disabled_path = String::new();
        }
        mod_info.enabled = true;
    } else if mod_type == ModType::Pak || mod_type == ModType::LogicMods || mod_type == ModType::Altermatic {
        let mod_info = &mut data.mods[mod_index];
        let mut moved_back = Vec::new();
        let mut moved_extras = Vec::new();
        let primary_disabled = PathBuf::from(&mod_info.disabled_path);
        let dest_subdir = if mod_type == ModType::LogicMods { "LogicMods" } else { "~mods" };
        let dest_dir = game_paks.join(dest_subdir);

        if primary_disabled.exists() {
            let filename = primary_disabled.file_name().unwrap().to_string_lossy().to_string();
            let dest = dest_dir.join(&filename);
            let _ = fs::create_dir_all(&dest_dir);
            move_path(&primary_disabled, &dest)?;
            moved_back.push(dest.to_string_lossy().to_string());
        }

        if mod_type == ModType::Altermatic {
            let swap_dir = game_paks.join("~mods").join("SwapJSON");
            let _ = fs::create_dir_all(&swap_dir);

            if let Some(cfg) = &mod_info.config_path {
                let cfg_path = PathBuf::from(cfg);
                if cfg_path.exists() {
                    let filename = cfg_path.file_name().unwrap().to_string_lossy().to_string();
                    let dest = swap_dir.join(&filename);
                    if let Ok(_) = move_path(&cfg_path, &dest) {
                        mod_info.config_path = Some(dest.to_string_lossy().to_string());
                    }
                }
            }

            for extra_disabled_str in &mod_info.extra_files {
                let extra_disabled = PathBuf::from(extra_disabled_str);
                if extra_disabled.exists() {
                    let filename = extra_disabled.file_name().unwrap().to_string_lossy().to_string();
                    let dest = if extra_disabled_str.to_lowercase().contains("swapjson") {
                        swap_dir.join(&filename)
                    } else {
                        dest_dir.join(&filename)
                    };
                    let _ = fs::create_dir_all(dest.parent().unwrap());
                    if let Ok(_) = move_path(&extra_disabled, &dest) {
                        moved_extras.push(dest.to_string_lossy().to_string());
                    }
                }
            }
            mod_info.extra_files = moved_extras;
        } else {
            for extra_disabled_str in &mod_info.extra_files {
                let extra_disabled = PathBuf::from(extra_disabled_str);
                if extra_disabled.exists() {
                    let filename = extra_disabled.file_name().unwrap().to_string_lossy().to_string();
                    let dest = dest_dir.join(&filename);
                    move_path(&extra_disabled, &dest)?;
                    moved_back.push(dest.to_string_lossy().to_string());
                }
            }
            let extras: Vec<String> = moved_back.iter().skip(1).cloned().collect();
            mod_info.extra_files = extras;
        }

        mod_info.game_path = moved_back.first().cloned().unwrap_or_default();
        mod_info.disabled_path = String::new();
        mod_info.enabled = true;
    } else if mod_type == ModType::Hybrid {
        let mod_info = &mut data.mods[mod_index];
        let primary_disabled = PathBuf::from(&mod_info.disabled_path);
        let mut dest_path = primary_disabled.clone();
        let mut primary_has_scripts = false;

        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let ue4ss_mods_dir = gp.ue4ss_mods_dir.clone();
        let palschema_mods_dir = gp.palschema_mods_dir.clone();

        if primary_disabled.exists() {
            let filename = primary_disabled.file_name().unwrap().to_string_lossy().to_string();
            primary_has_scripts = primary_disabled.join("Scripts").exists()
                || primary_disabled.join("scripts").exists()
                || primary_disabled.join("enabled.txt").exists()
                || primary_disabled.join("main.lua").exists()
                || primary_disabled.join(format!("{}.dll", filename)).exists()
                || primary_disabled.extension().map_or(false, |e| e == "lua");
            let dest = if primary_has_scripts {
                ue4ss_mods_dir.join(&filename)
            } else {
                palschema_mods_dir.join(&filename)
            };
            let _ = fs::create_dir_all(dest.parent().unwrap());
            move_path(&primary_disabled, &dest)?;
            dest_path = dest;
        }
        
        let mut moved_back = Vec::new();
        for extra_disabled_str in &mod_info.extra_files {
            let extra_disabled = PathBuf::from(extra_disabled_str);
            if extra_disabled.exists() {
                let filename = extra_disabled.file_name().unwrap().to_string_lossy().to_string();
                let extra_lower = extra_disabled_str.to_lowercase();
                
                let is_logic = extra_lower.contains("logicmods");
                let is_palschema = extra_lower.contains("palschema");
                
                let dest = if filename.ends_with(".pak") {
                    let dest_subdir = if is_logic { "LogicMods" } else { "~mods" };
                    let dest_dir = game_paks.join(dest_subdir);
                    let _ = fs::create_dir_all(&dest_dir);
                    dest_dir.join(&filename)
                } else if is_palschema {
                    let dest_dir = palschema_mods_dir.clone();
                    let _ = fs::create_dir_all(&dest_dir);
                    dest_dir.join(&filename)
                } else {
                    let dest_dir = ue4ss_mods_dir.clone();
                    let _ = fs::create_dir_all(&dest_dir);
                    dest_dir.join(&filename)
                };
                
                move_path(&extra_disabled, &dest)?;
                moved_back.push(dest.to_string_lossy().to_string());
                
                let parent = extra_disabled.parent().unwrap();
                let stem = extra_disabled.file_stem().unwrap().to_string_lossy().to_string();
                if filename.ends_with(".pak") {
                    let dest_subdir = if is_logic { "LogicMods" } else { "~mods" };
                    let dest_dir = game_paks.join(dest_subdir);
                    for c_ext in &["ucas", "utoc"] {
                        let companion = parent.join(format!("{}.{}", stem, c_ext));
                        if companion.exists() {
                            let c_dest = dest_dir.join(format!("{}.{}", stem, c_ext));
                            let _ = move_path(&companion, &c_dest);
                        }
                    }
                    if !filename.ends_with(".pmm.json") {
                        let sidecar = parent.join(format!("{}.pmm.json", filename));
                        if sidecar.exists() {
                            let c_dest = dest_dir.join(format!("{}.pmm.json", filename));
                            let _ = move_path(&sidecar, &c_dest);
                        }
                    }
                }
            }
        }

        // Fallback recovery: restore any orphaned hybrid components matching this mod in disabled_base/hybrid/
        let disabled_hybrid = disabled_base.join("hybrid");
        if disabled_hybrid.exists() {
            let mod_folder_name = get_mod_folder_name(mod_info);
            let mod_id_clean = mod_info.id.to_lowercase().replace(' ', "").replace('_', "");
            let mod_name_clean = mod_info.name.to_lowercase().replace(' ', "").replace('_', "");
            let folder_clean = mod_folder_name.to_lowercase().replace(' ', "").replace('_', "");

            let matches_mod = |candidate_name: &str| -> bool {
                let cand_clean = candidate_name.to_lowercase().replace(' ', "").replace('_', "");
                let cand_stem = candidate_name.strip_suffix(".pak").unwrap_or(candidate_name)
                    .strip_suffix("_P").unwrap_or(candidate_name)
                    .to_lowercase().replace(' ', "").replace('_', "");
                
                cand_clean == mod_id_clean || cand_clean == mod_name_clean || cand_clean == folder_clean
                    || cand_stem == mod_id_clean || cand_stem == mod_name_clean || cand_stem == folder_clean
                    || (!mod_id_clean.is_empty() && cand_stem.contains(&mod_id_clean))
                    || (!mod_name_clean.is_empty() && cand_stem.contains(&mod_name_clean))
                    || (!folder_clean.is_empty() && cand_stem.contains(&folder_clean))
            };

            // 1. Check logicmods/
            let logic_dir = disabled_hybrid.join("logicmods");
            if logic_dir.exists() {
                if let Ok(entries) = fs::read_dir(&logic_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if filename.ends_with(".pak") && matches_mod(&filename) {
                            let dest_dir = game_paks.join("LogicMods");
                            let _ = fs::create_dir_all(&dest_dir);
                            let dest = dest_dir.join(&filename);
                            let dest_str = dest.to_string_lossy().to_string();
                            if !moved_back.contains(&dest_str) {
                                if let Ok(_) = move_path(&path, &dest) {
                                    crate::logger::log(&format!("enable_mod: Fallback recovered orphaned LogicMods pak '{}' to '{:?}'", filename, dest));
                                    moved_back.push(dest_str);
                                    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                                    for c_ext in &["ucas", "utoc"] {
                                        let companion = logic_dir.join(format!("{}.{}", stem, c_ext));
                                        if companion.exists() {
                                            let _ = move_path(&companion, &dest_dir.join(format!("{}.{}", stem, c_ext)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Check pak/ (~mods)
            let pak_dir = disabled_hybrid.join("pak");
            if pak_dir.exists() {
                if let Ok(entries) = fs::read_dir(&pak_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if filename.ends_with(".pak") && matches_mod(&filename) {
                            let dest_dir = game_paks.join("~mods");
                            let _ = fs::create_dir_all(&dest_dir);
                            let dest = dest_dir.join(&filename);
                            let dest_str = dest.to_string_lossy().to_string();
                            if !moved_back.contains(&dest_str) {
                                if let Ok(_) = move_path(&path, &dest) {
                                    crate::logger::log(&format!("enable_mod: Fallback recovered orphaned pak '{}' to '{:?}'", filename, dest));
                                    moved_back.push(dest_str);
                                    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                                    for c_ext in &["ucas", "utoc"] {
                                        let companion = pak_dir.join(format!("{}.{}", stem, c_ext));
                                        if companion.exists() {
                                            let _ = move_path(&companion, &dest_dir.join(format!("{}.{}", stem, c_ext)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 3. Check palschema/
            let schema_dir = disabled_hybrid.join("palschema");
            if schema_dir.exists() {
                if let Ok(entries) = fs::read_dir(&schema_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if matches_mod(&filename) {
                            let dest_dir = palschema_mods_dir.clone();
                            let _ = fs::create_dir_all(&dest_dir);
                            let dest = dest_dir.join(&filename);
                            let dest_str = dest.to_string_lossy().to_string();
                            if !moved_back.contains(&dest_str) {
                                if let Ok(_) = move_path(&path, &dest) {
                                    crate::logger::log(&format!("enable_mod: Fallback recovered orphaned PalSchema folder '{}' to '{:?}'", filename, dest));
                                    moved_back.push(dest_str);
                                }
                            }
                        }
                    }
                }
            }

            // 4. Check ue4ss/
            let ue_dir = disabled_hybrid.join("ue4ss");
            if ue_dir.exists() {
                if let Ok(entries) = fs::read_dir(&ue_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if matches_mod(&filename) {
                            let dest_dir = ue4ss_mods_dir.clone();
                            let _ = fs::create_dir_all(&dest_dir);
                            let dest = dest_dir.join(&filename);
                            let dest_str = dest.to_string_lossy().to_string();
                            if !moved_back.contains(&dest_str) {
                                if let Ok(_) = move_path(&path, &dest) {
                                    crate::logger::log(&format!("enable_mod: Fallback recovered orphaned UE4SS folder '{}' to '{:?}'", filename, dest));
                                    moved_back.push(dest_str);
                                }
                            }
                        }
                    }
                }
            }
        }

        let force_order = data.settings.force_load_order.unwrap_or(false) && force_ue4ss_effective;

        if let Some(ue4ss_mods_dir) = dest_path.parent() {
            let mods_txt = ue4ss_mods_dir.join("mods.txt");
            if mods_txt.exists() {
                for file_str in &moved_back {
                    let path = PathBuf::from(file_str);
                    if path.exists() && path.is_dir() && path.parent() == Some(ue4ss_mods_dir) {
                        let extra_folder_name = path.file_name().unwrap().to_string_lossy().to_string();
                        if force_order {
                            let _ = update_mods_txt_load_order(&mods_txt, &extra_folder_name, true);
                        } else {
                            let _ = remove_from_mods_txt(&mods_txt, &extra_folder_name);
                        }
                        
                        let enabled_file = path.join("enabled.txt");
                        if force_order {
                            if enabled_file.exists() {
                                let _ = fs::remove_file(&enabled_file);
                            }
                        } else {
                            let _ = fs::write(&enabled_file, "");
                        }
                    }
                }
            }
        }

        if primary_has_scripts {
            if let Some(ue4ss_mods_dir) = dest_path.parent() {
                let mods_txt = ue4ss_mods_dir.join("mods.txt");
                if mods_txt.exists() {
                    let folder_name = get_mod_folder_name(mod_info);
                    if force_order {
                        let _ = update_mods_txt_load_order(&mods_txt, &folder_name, true);
                    } else {
                        let _ = remove_from_mods_txt(&mods_txt, &folder_name);
                    }
                }
            }
            let enabled_file = dest_path.join("enabled.txt");
            if force_order {
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
            } else {
                let _ = fs::write(&enabled_file, "");
            }
        }
        
        mod_info.game_path = dest_path.to_string_lossy().to_string();
        mod_info.extra_files = moved_back;
        mod_info.disabled_path = String::new();
        mod_info.enabled = true;
    } else {
        return Ok(());
    }

    // Add to active profile's installed + enabled lists and persist profile.json
    let current_id = data.current_profile_id.clone();
    let mod_name_for_profile = data.mods.iter().find(|m| m.id == mod_id).map(|m| m.name.clone());
    if let Some(ref name) = mod_name_for_profile {
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_id) {
            let in_installed = profile.installed_mod_ids.iter().any(|id| id.to_lowercase() == name.to_lowercase());
            if !in_installed {
                profile.installed_mod_ids.push(name.clone());
            }
            let in_enabled = profile.enabled_mod_ids.iter().any(|id| id.to_lowercase() == name.to_lowercase());
            if !in_enabled {
                profile.enabled_mod_ids.push(name.clone());
            }
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
