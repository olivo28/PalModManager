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
            let name_str = name.to_string_lossy().to_string();
            if let Some(stripped) = name_str.strip_suffix(".disabled") {
                return stripped.to_string();
            }
            return name_str;
        }
    }
    if !mod_info.disabled_path.is_empty() {
        if let Some(name) = Path::new(&mod_info.disabled_path).file_name() {
            let name_str = name.to_string_lossy().to_string();
            if let Some(stripped) = name_str.strip_suffix(".disabled") {
                return stripped.to_string();
            }
            return name_str;
        }
    }
    mod_info.name.clone()
}

pub fn update_mods_txt_load_order(mods_txt: &Path, mod_name: &str, enabled: bool) -> Result<(), String> {
    let content = fs::read_to_string(mods_txt).map_err(|e| e.to_string())?;
    let target_val = if enabled { "1" } else { "0" };

    let mut lines_to_process = Vec::new();
    for line in content.lines() {
        let line_clean = line.trim();
        if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
            if let Some(pos) = line_clean.find(':') {
                let name = line_clean[..pos].trim();
                if name.to_lowercase() == mod_name.to_lowercase() {
                    continue;
                }
            } else if line_clean.to_lowercase() == mod_name.to_lowercase() {
                continue;
            }
        }
        lines_to_process.push(line.to_string());
    }

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
            let _ = update_mods_txt_load_order(&path, &mod_info.name, false);
        }
        mod_info.enabled = false;
        return Ok(());
    }

    let current_profile_id = data.current_profile_id.clone();
    let disabled_base = PathBuf::from(program_path)
        .join("profiles")
        .join(&current_profile_id)
        .join("disabled_mods");
    let _ = fs::create_dir_all(&disabled_base);

    if mod_type == ModType::Ue4ss {
        let mod_info = &mut data.mods[mod_index];
        let src_path = PathBuf::from(&mod_info.game_path);
        if src_path.exists() {
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
        }
        mod_info.enabled = false;
    } else if mod_type == ModType::PalSchema {
        let mod_info = &mut data.mods[mod_index];
        let src_path = PathBuf::from(&mod_info.game_path);
        let folder_name = get_mod_folder_name(mod_info);

        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let palschema_mods_dir = gp.palschema_mods_dir.clone();
        let palschema_storage_dir = gp.palschema_mods_dir.parent().unwrap().join("Storage");

        if palschema_mods_dir.exists() {
            if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if name == folder_name || (name.len() > 4 && &name[4..] == folder_name) {
                        let _ = remove_junction_or_symlink(&path);
                    }
                }
            }
        }

        let storage_path = palschema_storage_dir.join(&folder_name);
        let final_src = if storage_path.exists() { storage_path } else { src_path };

        if final_src.exists() {
            let file_name = final_src.file_name().unwrap().to_string_lossy().to_string();
            let dest_dir = disabled_base.join("palschema");
            let _ = fs::create_dir_all(&dest_dir);
            let dest = dest_dir.join(&file_name);
            move_path(&final_src, &dest)?;
            mod_info.disabled_path = dest.to_string_lossy().to_string();
            mod_info.game_path = String::new();
        }
        mod_info.enabled = false;
    } else if mod_type == ModType::Pak || mod_type == ModType::LogicMods {
        let mod_info = &mut data.mods[mod_index];
        let src_path = PathBuf::from(&mod_info.game_path);
        if src_path.exists() {
            let mut moved_files = Vec::new();
            if let Some(parent) = src_path.parent() {
                let file_stem = src_path.file_stem().unwrap().to_string_lossy().to_string();
                let type_dir = if mod_type == ModType::LogicMods { "logicmods" } else { "pak" };
                let dest_dir = disabled_base.join(type_dir);
                let _ = fs::create_dir_all(&dest_dir);

                for ext in &["pak", "ucas", "utoc", "pak.pmm.json"] {
                    let companion = parent.join(format!("{}.{}", file_stem, ext));
                    if companion.exists() {
                        let dest = dest_dir.join(format!("{}.{}", file_stem, ext));
                        move_path(&companion, &dest)?;
                        moved_files.push(dest.to_string_lossy().to_string());
                    }
                }
            }
            mod_info.disabled_path = moved_files.first().cloned().unwrap_or_default();
            mod_info.extra_files = moved_files.into_iter().skip(1).collect();
            mod_info.game_path = String::new();
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
                    let sidecar = parent.join(format!("{}.pmm.json", file_name));
                    if sidecar.exists() {
                        let c_dest = dest_dir.join(format!("{}.pmm.json", file_name));
                        let _ = move_path(&sidecar, &c_dest);
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
        profile.enabled_mod_ids.retain(|id| {
            id.to_lowercase() != mod_id.to_lowercase() && 
            id.to_lowercase() != mod_name.to_lowercase()
        });
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
        return Ok(());
    }

    let game_paks = PathBuf::from(&data.settings.game_path).join("Pal").join("Content").join("Paks");

    if mod_type == ModType::Ue4ss {
        let mod_info = &mut data.mods[mod_index];
        let primary_disabled = PathBuf::from(&mod_info.disabled_path);
        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let dest_dir = gp.ue4ss_mods_dir.clone();

        if primary_disabled.exists() {
            let filename = primary_disabled.file_name().unwrap().to_string_lossy().to_string();
            let dest = dest_dir.join(&filename);
            let _ = fs::create_dir_all(&dest_dir);
            move_path(&primary_disabled, &dest)?;

            let force_order = data.settings.force_load_order.unwrap_or(false) && force_ue4ss_effective;
            let mods_txt = dest_dir.join("mods.txt");
            if mods_txt.exists() {
                let folder_name = get_mod_folder_name(mod_info);
                if force_order {
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
            } else {
                let _ = fs::write(&enabled_file, "");
            }

            mod_info.game_path = dest.to_string_lossy().to_string();
            mod_info.disabled_path = String::new();
        }
        mod_info.enabled = true;
    } else if mod_type == ModType::PalSchema {
        let mod_info = &mut data.mods[mod_index];
        let primary_disabled = PathBuf::from(&mod_info.disabled_path);
        
        let folder_name = get_mod_folder_name(mod_info);
        let force_order = data.settings.force_load_order.unwrap_or(false) && force_palschema_effective;

        let gp = crate::dependency_checker::build_game_profile(Path::new(&data.settings.game_path));
        let palschema_mods_dir = gp.palschema_mods_dir.clone();
        let palschema_storage_dir = gp.palschema_mods_dir.parent().unwrap().join("Storage");

        if primary_disabled.exists() {
            let _ = fs::create_dir_all(&palschema_mods_dir);
            if force_order {
                let storage_dest = palschema_storage_dir.join(&folder_name);
                let _ = fs::create_dir_all(&palschema_storage_dir);
                move_path(&primary_disabled, &storage_dest)?;

                let order = mod_info.mods_txt_order.unwrap_or(999);
                let link_name = format!("{:03}_{}", order, folder_name);
                let link_path = palschema_mods_dir.join(&link_name);
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
    } else if mod_type == ModType::Pak || mod_type == ModType::LogicMods {
        let mod_info = &mut data.mods[mod_index];
        let mut moved_back = Vec::new();
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
        for extra_disabled_str in &mod_info.extra_files {
            let extra_disabled = PathBuf::from(extra_disabled_str);
            if extra_disabled.exists() {
                let filename = extra_disabled.file_name().unwrap().to_string_lossy().to_string();
                let dest = dest_dir.join(&filename);
                move_path(&extra_disabled, &dest)?;
                moved_back.push(dest.to_string_lossy().to_string());
            }
        }
        mod_info.game_path = moved_back.first().cloned().unwrap_or_default();
        mod_info.extra_files = moved_back.into_iter().skip(1).collect();
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
                || primary_disabled.join("enabled.txt").exists();
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
                    let sidecar = parent.join(format!("{}.pmm.json", filename));
                    if sidecar.exists() {
                        let c_dest = dest_dir.join(format!("{}.pmm.json", filename));
                        let _ = move_path(&sidecar, &c_dest);
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

    Ok(())
}
