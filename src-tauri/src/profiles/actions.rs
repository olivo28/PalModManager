use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{AppData, ModInfo, ModType};
use super::utils::{
    get_profile_dir, remove_junction_or_symlink, create_junction_or_symlink,
    move_path, save_pmm_meta,
};
use crate::profiles::effective_force_ue4ss;

pub fn get_mod_folder_name(mod_info: &ModInfo) -> String {
    if !mod_info.game_path.is_empty() {
        if let Some(name) = Path::new(&mod_info.game_path).file_name() {
            let mut name_str = name.to_string_lossy().to_string();
            if let Some(stripped) = name_str.strip_suffix(".disabled") {
                name_str = stripped.to_string();
            }
            if name_str.len() > 4 && name_str[..3].chars().all(|c| c.is_ascii_digit()) && name_str.as_bytes()[3] == b'_' {
                name_str = name_str[4..].to_string();
            }
            return name_str;
        }
    }
    if !mod_info.disabled_path.is_empty() {
        if let Some(name) = Path::new(&mod_info.disabled_path).file_name() {
            let mut name_str = name.to_string_lossy().to_string();
            if let Some(stripped) = name_str.strip_suffix(".disabled") {
                name_str = stripped.to_string();
            }
            if name_str.len() > 4 && name_str[..3].chars().all(|c| c.is_ascii_digit()) && name_str.as_bytes()[3] == b'_' {
                name_str = name_str[4..].to_string();
            }
            return name_str;
        }
    }
    let mut clean_name = mod_info.name.clone();
    if clean_name.len() > 4 && clean_name[..3].chars().all(|c| c.is_ascii_digit()) && clean_name.as_bytes()[3] == b'_' {
        clean_name = clean_name[4..].to_string();
    }
    clean_name
}

pub fn update_mods_txt_load_order(mods_txt: &Path, mod_name: &str, enabled: bool) -> Result<(), String> {
    let content = fs::read_to_string(mods_txt).map_err(|e| e.to_string())?;
    let target_val = if enabled { "1" } else { "0" };
    let mod_name_lower = mod_name.to_lowercase();

    let mut found = false;
    let mut lines_to_process: Vec<String> = Vec::new();

    for line in content.lines() {
        let line_clean = line.trim();
        if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
            let matches = if let Some(pos) = line_clean.find(':') {
                let name = line_clean[..pos].trim();
                name.to_lowercase() == mod_name_lower
            } else {
                line_clean.to_lowercase() == mod_name_lower
            };

            if matches {
                found = true;
                lines_to_process.push(format!("{} : {}", mod_name, target_val));
                continue;
            }
        }
        lines_to_process.push(line.to_string());
    }

    if !found {
        let mut insert_index = None;
        for (idx, line) in lines_to_process.iter().enumerate() {
            let line_clean = line.trim();
            if line_clean.contains("BPModLoaderMod") {
                insert_index = Some(idx + 1);
            }
        }

        if insert_index.is_none() {
            for (idx, line) in lines_to_process.iter().enumerate() {
                let line_clean = line.trim();
                if line_clean.contains("; Built-in keybinds") {
                    insert_index = Some(idx);
                }
            }
        }

        let final_idx = insert_index.unwrap_or(lines_to_process.len());
        let new_entry = format!("{} : {}", mod_name, target_val);
        lines_to_process.insert(final_idx, new_entry);
    }

    fs::write(mods_txt, lines_to_process.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_from_mods_txt(mods_txt: &Path, mod_name: &str) -> Result<(), String> {
    let content = fs::read_to_string(mods_txt).map_err(|e| e.to_string())?;
    let mut new_lines = Vec::new();
    let mut changed = false;

    for line in content.lines() {
        let line_clean = line.trim();
        if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
            if let Some(pos) = line_clean.find(':') {
                let name = line_clean[..pos].trim();
                if name.to_lowercase() == mod_name.to_lowercase() {
                    changed = true;
                    continue;
                }
            } else if line_clean.to_lowercase() == mod_name.to_lowercase() {
                changed = true;
                continue;
            }
        }
        new_lines.push(line.to_string());
    }

    if changed {
        fs::write(mods_txt, new_lines.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}

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
                current_p.and_then(|p| p.ue4ss_control_mode.as_deref()).unwrap_or("") == "mods_txt"
                    || data.settings.ue4ss_control_mode.as_deref().unwrap_or("") == "mods_txt"
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
        current_p.and_then(|p| p.ue4ss_control_mode.as_deref()).unwrap_or("") == "mods_txt"
            || data.settings.ue4ss_control_mode.as_deref().unwrap_or("") == "mods_txt"
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
                current_p.and_then(|p| p.ue4ss_control_mode.as_deref()).unwrap_or("") == "mods_txt"
                    || data.settings.ue4ss_control_mode.as_deref().unwrap_or("") == "mods_txt"
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
        current_p.and_then(|p| p.ue4ss_control_mode.as_deref()).unwrap_or("") == "mods_txt"
            || data.settings.ue4ss_control_mode.as_deref().unwrap_or("") == "mods_txt"
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

pub fn clean_mods_txt_native_only(mods_txt: &Path) -> Result<(), String> {
    if !mods_txt.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(mods_txt).unwrap_or_default();
    let mut native_lines = Vec::new();
    let mut bottom_lines = Vec::new();
    let mut in_bottom_keybinds = false;

    for line in content.lines() {
        let line_clean = line.trim();
        if line_clean.eq_ignore_ascii_case("; Built-in keybinds") 
            || line_clean.to_lowercase().starts_with("; built-in keybinds")
            || (line_clean.to_lowercase().starts_with("keybinds") && line_clean.contains(':'))
        {
            in_bottom_keybinds = true;
        }

        if in_bottom_keybinds {
            if !line_clean.is_empty() {
                bottom_lines.push(line_clean.to_string());
            }
            continue;
        }

        let name = if let Some(pos) = line_clean.find(':') {
            line_clean[..pos].trim()
        } else {
            line_clean
        };

        let is_native_tool = [
            "CheatManagerEnablerMod",
            "ConsoleCommandsMod",
            "ConsoleEnablerMod",
            "SplitScreenMod",
            "LineTraceMod",
            "BPML_GenericFunctions",
            "BPModLoaderMod",
        ].iter().any(|&n| n.eq_ignore_ascii_case(name));

        if is_native_tool {
            native_lines.push(line.to_string());
        }
    }

    if native_lines.is_empty() {
        native_lines = vec![
            "CheatManagerEnablerMod : 0".to_string(),
            "ConsoleCommandsMod : 0".to_string(),
            "ConsoleEnablerMod : 0".to_string(),
            "SplitScreenMod : 0".to_string(),
            "LineTraceMod : 0".to_string(),
            "BPML_GenericFunctions : 1".to_string(),
            "BPModLoaderMod : 1".to_string(),
        ];
    }

    if bottom_lines.is_empty() {
        bottom_lines = vec![
            "; Built-in keybinds, do not move up!".to_string(),
            "Keybinds : 1".to_string(),
        ];
    }

    let mut output_lines = Vec::new();
    for l in native_lines {
        output_lines.push(l);
    }
    output_lines.push("".to_string());
    for bl in bottom_lines {
        output_lines.push(bl);
    }

    fs::write(mods_txt, output_lines.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn sync_mods_txt_sections(
    mods_txt: &Path,
    profile: &crate::models::Profile,
    mods: &[ModInfo],
) -> Result<(), String> {
    let content = fs::read_to_string(mods_txt).unwrap_or_default();
    
    // 1. Identify native UE4SS mods and bottom keybinds from existing content
    let mut native_lines = Vec::new();
    let mut bottom_lines = Vec::new();
    let mut in_bottom_keybinds = false;

    for line in content.lines() {
        let line_clean = line.trim();
        if line_clean.eq_ignore_ascii_case("; Built-in keybinds") 
            || line_clean.to_lowercase().starts_with("; built-in keybinds")
            || (line_clean.to_lowercase().starts_with("keybinds") && line_clean.contains(':'))
        {
            in_bottom_keybinds = true;
        }

        if in_bottom_keybinds {
            bottom_lines.push(line.to_string());
            continue;
        }

        // Check if this is a native UE4SS tool line at the top
        let name = if let Some(pos) = line_clean.find(':') {
            line_clean[..pos].trim()
        } else {
            line_clean
        };

        let is_native_tool = [
            "CheatManagerEnablerMod",
            "ConsoleCommandsMod",
            "ConsoleEnablerMod",
            "SplitScreenMod",
            "LineTraceMod",
            "BPML_GenericFunctions",
            "BPModLoaderMod",
        ].iter().any(|&n| n.eq_ignore_ascii_case(name));

        if is_native_tool {
            native_lines.push(line.to_string());
        }
    }

    if native_lines.is_empty() {
        native_lines = vec![
            "CheatManagerEnablerMod : 0".to_string(),
            "ConsoleCommandsMod : 0".to_string(),
            "ConsoleEnablerMod : 0".to_string(),
            "SplitScreenMod : 0".to_string(),
            "LineTraceMod : 0".to_string(),
            "BPML_GenericFunctions : 1".to_string(),
            "BPModLoaderMod : 1".to_string(),
        ];
    }

    // 2. Filter UE4SS/Hybrid mods installed in this profile
    let mut installed_ue4ss_mods: Vec<&ModInfo> = Vec::new();
    for m in mods {
        if (m.mod_type == ModType::Ue4ss || m.mod_type == ModType::Hybrid)
            && m.nexus_author.as_deref() != Some("UE4SS Native Mod")
        {
            if profile.installed_mod_ids.iter().any(|entry| crate::profiles::mod_matches_profile_entry(m, entry)) {
                installed_ue4ss_mods.push(m);
            }
        }
    }

    // 3. Build output lines starting with top native tools
    let mut output_lines = Vec::new();
    for l in native_lines {
        output_lines.push(l);
    }

    let mut assigned_mod_ids = std::collections::HashSet::new();

    // 4. For each virtual folder in profile.mod_folders:
    for folder in &profile.mod_folders {
        let mut active_mod_lines = Vec::new();
        let mut disabled_mod_lines = Vec::new();

        for fid in &folder.mod_ids {
            if let Some(m) = installed_ue4ss_mods.iter().find(|m| crate::profiles::mod_matches_profile_entry(m, fid)) {
                let folder_name = get_mod_folder_name(m);
                let is_enabled = m.enabled && (profile.enabled_mod_ids.is_empty() || profile.enabled_mod_ids.iter().any(|id| crate::profiles::mod_matches_profile_entry(m, id)));
                if is_enabled {
                    active_mod_lines.push(format!("{} : 1", folder_name));
                } else {
                    disabled_mod_lines.push(format!("{} : 0", folder_name));
                }
                assigned_mod_ids.insert(m.id.clone());
            }
        }

        if !active_mod_lines.is_empty() || !disabled_mod_lines.is_empty() {
            output_lines.push("".to_string());
            output_lines.push(format!("; -----{}-----", folder.name));
            for aml in active_mod_lines {
                output_lines.push(aml);
            }
            output_lines.push("; -----Disabled Mods-----".to_string());
            for dml in disabled_mod_lines {
                output_lines.push(dml);
            }
        }
    }

    // 5. Any ungrouped UE4SS mods in this profile
    let mut ungrouped_active = Vec::new();
    let mut ungrouped_disabled = Vec::new();
    for m in &installed_ue4ss_mods {
        if !assigned_mod_ids.contains(&m.id) {
            let folder_name = get_mod_folder_name(m);
            let is_enabled = m.enabled && (profile.enabled_mod_ids.is_empty() || profile.enabled_mod_ids.iter().any(|id| crate::profiles::mod_matches_profile_entry(m, id)));
            if is_enabled {
                ungrouped_active.push(format!("{} : 1", folder_name));
            } else {
                ungrouped_disabled.push(format!("{} : 0", folder_name));
            }
        }
    }

    if !ungrouped_active.is_empty() || !ungrouped_disabled.is_empty() {
        output_lines.push("".to_string());
        for aml in ungrouped_active {
            output_lines.push(aml);
        }
        if !ungrouped_disabled.is_empty() {
            output_lines.push("; -----Disabled Mods-----".to_string());
            for dml in ungrouped_disabled {
                output_lines.push(dml);
            }
        }
    }

    // 6. Append bottom keybinds (always ensure a blank line before it!)
    let mut clean_bottom = Vec::new();
    for bl in bottom_lines {
        let trimmed = bl.trim();
        if !trimmed.is_empty() {
            clean_bottom.push(trimmed.to_string());
        }
    }

    if clean_bottom.is_empty() {
        clean_bottom = vec![
            "; Built-in keybinds, do not move up!".to_string(),
            "Keybinds : 1".to_string(),
        ];
    }

    output_lines.push("".to_string());
    for bl in clean_bottom {
        output_lines.push(bl);
    }

    fs::write(mods_txt, output_lines.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    Ok(())
}
