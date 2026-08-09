use crate::state::AppState;
use crate::models::{ModInfo, ModType};
use tauri::State;
use std::path::{Path, PathBuf};
use std::fs;

#[tauri::command]
pub fn get_ue4ss_load_order(state: State<AppState>) -> Result<Vec<ModInfo>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path = data.settings.game_path.clone();
    
    // Find current active profile
    let current_profile = data.profiles.iter()
        .find(|p| p.id == data.current_profile_id)
        .ok_or_else(|| "Current profile not found".to_string())?;

    // 1. Filter for active/enabled UE4SS and Hybrid mods in the current profile (excluding native ones)
    let mut target_mods: Vec<ModInfo> = data.mods.iter()
        .filter(|m| {
            current_profile.installed_mod_ids.contains(&m.id)
            && !m.game_path.is_empty()
            && (m.mod_type == ModType::Ue4ss || m.mod_type == ModType::Hybrid)
            && m.nexus_author.as_deref() != Some("UE4SS Native Mod")
        })
        .cloned()
        .collect();

    // 2. Try to read mods.txt to get order weights and enabled state
    let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let mods_txt = binaries_dir.join("ue4ss").join("Mods").join("mods.txt");
    
    let mut order_map = std::collections::HashMap::new();
    let mut enabled_map = std::collections::HashMap::new();
    
    if mods_txt.exists() {
        if let Ok(content) = fs::read_to_string(&mods_txt) {
            let mut custom_index = 0;
            for line in content.lines() {
                let line_clean = line.trim();
                if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
                    let (name, enabled) = if let Some(pos) = line_clean.find(':') {
                        let n = line_clean[..pos].trim().to_string();
                        let e_str = line_clean[pos+1..].trim();
                        (n, e_str != "0")
                    } else {
                        (line_clean.to_string(), true)
                    };
                    if !name.is_empty() {
                        let lower_name = name.to_lowercase();
                        order_map.insert(lower_name.clone(), custom_index);
                        enabled_map.insert(lower_name, enabled);
                        custom_index += 1;
                    }
                }
            }
        }
    }

    // Update the returned ModInfo enabled flag with the state parsed from mods.txt
    for m in &mut target_mods {
        let folder_name = crate::profiles::get_mod_folder_name(m).to_lowercase();
        if let Some(&is_enabled) = enabled_map.get(&folder_name) {
            m.enabled = is_enabled;
        } else if let Some(&is_enabled) = enabled_map.get(&m.name.to_lowercase()) {
            m.enabled = is_enabled;
        } else {
            m.enabled = true; // Default to true if not in mods.txt yet
        }
    }

    // 3. Sort by mods.txt weight, fallback to alphabetical
    target_mods.sort_by(|a, b| {
        let folder_a = crate::profiles::get_mod_folder_name(a).to_lowercase();
        let folder_b = crate::profiles::get_mod_folder_name(b).to_lowercase();

        let weight_a = order_map.get(&folder_a)
            .or_else(|| order_map.get(&a.name.to_lowercase()))
            .copied()
            .unwrap_or(usize::MAX);
        let weight_b = order_map.get(&folder_b)
            .or_else(|| order_map.get(&b.name.to_lowercase()))
            .copied()
            .unwrap_or(usize::MAX);
        
        if weight_a != weight_b {
            weight_a.cmp(&weight_b)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(target_mods)
}

#[tauri::command]
pub fn save_ue4ss_load_order(ordered_items: Vec<(String, bool)>, state: State<AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path = data.settings.game_path.clone();
    
    let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let ue4ss_mods_dir = binaries_dir.join("ue4ss").join("Mods");
    let mods_txt = ue4ss_mods_dir.join("mods.txt");
    if !mods_txt.exists() {
        return Err("mods.txt not found".to_string());
    }

    let content = fs::read_to_string(&mods_txt).map_err(|e| e.to_string())?;
    
    // Build list of custom mod lines to insert (including companion folders)
    let mut custom_lines = Vec::new();
    let ordered_ids: Vec<String> = ordered_items.iter().map(|(id, _)| id.clone()).collect();
    for (id, enabled) in &ordered_items {
        if let Some(m) = data.mods.iter().find(|m| &m.id == id) {
            let val = if *enabled { "1" } else { "0" };
            let folder_name = crate::profiles::get_mod_folder_name(m);
            custom_lines.push(format!("{} : {}", folder_name, val));
            
            // Add any secondary companion folders from extra_files
            for extra in &m.extra_files {
                let extra_path = PathBuf::from(extra);
                if extra_path.exists() && extra_path.is_dir() && extra_path.parent() == Some(&ue4ss_mods_dir) {
                    let extra_folder_name = extra_path.file_name().unwrap().to_string_lossy().to_string();
                    custom_lines.push(format!("{} : {}", extra_folder_name, val));
                }
            }
        }
    }

    // Process existing mods.txt and filter out any existing custom mod and companion folder references
    let mut lines_to_keep = Vec::new();
    for line in content.lines() {
        let line_clean = line.trim();
        if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
            let name = if let Some(pos) = line_clean.find(':') {
                line_clean[..pos].trim().to_lowercase()
            } else {
                line_clean.to_lowercase()
            };
            
            let is_matching_custom = ordered_ids.iter().any(|id| {
                if let Some(m) = data.mods.iter().find(|mod_item| &mod_item.id == id) {
                    // Check principal name or folder_name
                    if m.name.to_lowercase() == name || crate::profiles::get_mod_folder_name(m).to_lowercase() == name {
                        return true;
                    }
                    // Check companion folders in extra_files
                    for extra in &m.extra_files {
                        let extra_path = PathBuf::from(extra);
                        if extra_path.is_dir() && extra_path.parent() == Some(&ue4ss_mods_dir) {
                            let extra_folder_name = extra_path.file_name().unwrap().to_string_lossy().to_string().to_lowercase();
                            if extra_folder_name == name {
                                return true;
                            }
                        }
                    }
                }
                false
            });

            if is_matching_custom {
                continue;
            }
        }
        lines_to_keep.push(line.to_string());
    }

    // Find insertion index
    let mut insert_index = None;
    for (idx, line) in lines_to_keep.iter().enumerate() {
        let line_clean = line.trim();
        if line_clean.contains("BPModLoaderMod") {
            insert_index = Some(idx + 1);
        }
    }

    if insert_index.is_none() {
        for (idx, line) in lines_to_keep.iter().enumerate() {
            let line_clean = line.trim();
            if line_clean.contains("; Built-in keybinds") {
                insert_index = Some(idx);
            }
        }
    }

    let final_idx = insert_index.unwrap_or(lines_to_keep.len());
    
    // Insert ordered custom lines
    for (offset, custom_line) in custom_lines.into_iter().enumerate() {
        lines_to_keep.insert(final_idx + offset, custom_line);
    }

    fs::write(&mods_txt, lines_to_keep.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;

    // Save metadata to active profile
    let mut metadata = Vec::new();
    for (id, enabled) in &ordered_items {
        if let Some(m) = data.mods.iter().find(|m| &m.id == id) {
            let folder_name = crate::profiles::get_mod_folder_name(m);
            metadata.push((folder_name, *enabled));
        }
    }
    let current_id = data.current_profile_id.clone();
    if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_id) {
        profile.load_order_metadata = Some(metadata);
    }

    let data_clone = data.clone();
    drop(data);
    let _ = crate::db::save_db(&data_clone.settings.program_path, &data_clone);

    Ok(())
}
#[tauri::command]
pub fn get_palschema_load_order(state: State<AppState>) -> Result<Vec<ModInfo>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path = data.settings.game_path.clone();
    
    // Find current active profile
    let current_profile = data.profiles.iter()
        .find(|p| p.id == data.current_profile_id)
        .ok_or_else(|| "Current profile not found".to_string())?;

    // Filter for installed PalSchema and Hybrid mods in the current profile
    let mut target_mods: Vec<ModInfo> = data.mods.iter()
        .filter(|m| {
            current_profile.installed_mod_ids.contains(&m.id)
            && (m.mod_type == ModType::PalSchema || m.mod_type == ModType::Hybrid)
        })
        .cloned()
        .collect();

    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let palschema_mods_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("mods");

    // Read the physical mods/ directory to determine current order weights and enabled state
    let mut order_map = std::collections::HashMap::new();
    let mut enabled_map = std::collections::HashMap::new();

    if palschema_mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() || junction::exists(&path).unwrap_or(false) {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    // Parse optional "001_" prefix
                    let (clean_name, weight) = if name.len() > 4 && name.chars().take(3).all(|c| c.is_ascii_digit()) && name.chars().nth(3) == Some('_') {
                        let weight_str = &name[..3];
                        let w = weight_str.parse::<usize>().unwrap_or(999);
                        let rest = &name[4..];
                        (rest.to_string(), w)
                    } else {
                        (name.clone(), 999)
                    };
                    
                    order_map.insert(clean_name.to_lowercase(), weight);
                    // It is enabled if the link/directory exists under mods/
                    enabled_map.insert(clean_name.to_lowercase(), true);
                }
            }
        }
    }

    // Update ModInfo enabled status and temporary order weights
    let palschema_storage_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("Storage");
    let mut filtered_mods = Vec::new();
    for mut m in target_mods {
        let folder_name = crate::profiles::get_mod_folder_name(&m).to_lowercase();
        
        // For Hybrid mods, check if a PalSchema directory exists in mods/ or Storage/ to confirm they have a PalSchema component.
        if m.mod_type == ModType::Hybrid {
            let has_palschema = enabled_map.contains_key(&folder_name) || palschema_storage_dir.join(&m.name).exists() || palschema_storage_dir.join(&folder_name).exists();
            if !has_palschema {
                continue; // Skip hybrid mods with no PalSchema component
            }
        }

        // Checked in profiles enabled_mod_ids
        let profile_enabled = current_profile.enabled_mod_ids.iter().any(|id| {
            let id_lower = id.to_lowercase();
            id_lower == m.id.to_lowercase() || id_lower == m.name.to_lowercase() || id_lower == folder_name
        });
        
        m.enabled = profile_enabled;
        
        let weight = order_map.get(&folder_name).copied().unwrap_or(999);
        m.mods_txt_order = Some(weight as u32);
        filtered_mods.push(m);
    }
    target_mods = filtered_mods;

    // Sort by NTF Junction prefix weight, fallback to alphabetical
    target_mods.sort_by(|a, b| {
        let weight_a = a.mods_txt_order.unwrap_or(999);
        let weight_b = b.mods_txt_order.unwrap_or(999);
        
        if weight_a != weight_b {
            weight_a.cmp(&weight_b)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(target_mods)
}

#[tauri::command]
pub fn save_palschema_load_order(ordered_items: Vec<(String, bool)>, state: State<AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let game_path = data.settings.game_path.clone();
    let force_order = data.settings.force_load_order.unwrap_or(false);

    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let palschema_mods_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("mods");
    let palschema_storage_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("Storage");

    if !palschema_mods_dir.exists() {
        let _ = fs::create_dir_all(&palschema_mods_dir);
    }
    if !palschema_storage_dir.exists() {
        let _ = fs::create_dir_all(&palschema_storage_dir);
    }

    // Get current list of links in PalSchema/mods/
    let mut current_mods_links = Vec::new();
    if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            current_mods_links.push((path, name));
        }
    }

    let current_profile_id = data.current_profile_id.clone();
    let program_path = data.settings.program_path.clone();

    // Process and rename/recreate junctions according to the new order
    for (idx, (id, enabled)) in ordered_items.iter().enumerate() {
        let mut folder_name_opt = None;
        let mut mod_name_opt = None;

        if let Some(m) = data.mods.iter_mut().find(|mod_item| &mod_item.id == id) {
            let folder_name = crate::profiles::get_mod_folder_name(m);
            m.mods_txt_order = Some(idx as u32);
            m.enabled = *enabled;
            folder_name_opt = Some(folder_name.clone());
            mod_name_opt = Some(m.name.clone());

            // Remove any existing junction matching this mod (numbered prefix or clean)
            for (path, name) in &current_mods_links {
                if name == &folder_name || (name.len() > 4 && &name[4..] == &folder_name) {
                    let _ = crate::profiles::remove_junction_or_symlink(path);
                }
            }

            if *enabled {
                // Determine target link name
                let link_name = if force_order {
                    format!("{:03}_{}", idx, folder_name)
                } else {
                    folder_name.clone()
                };

                let target_storage = palschema_storage_dir.join(&folder_name);
                let link_path = palschema_mods_dir.join(&link_name);
                
                // If it's enabled but doesn't exist in Storage, try migrating from mods/ (or default disabled)
                if !target_storage.exists() {
                    // Check if it's currently sitting directly in mods/ (unmigrated)
                    let direct_mods_path = palschema_mods_dir.join(&folder_name);
                    if direct_mods_path.exists() && !junction::exists(&direct_mods_path).unwrap_or(false) {
                        let _ = fs::rename(&direct_mods_path, &target_storage);
                    }
                }

                if target_storage.exists() {
                    let _ = crate::profiles::create_junction_or_symlink(&target_storage, &link_path);
                    if m.mod_type == ModType::PalSchema {
                        m.game_path = link_path.to_string_lossy().to_string();
                        m.disabled_path = String::new();
                    }
                }
            } else {
                // If disabled, we move it back to disabled folder in profile
                let profile_dir = crate::profiles::get_profile_dir(&program_path, &current_profile_id);
                let disabled_base = profile_dir.join("disabled_mods");
                let storage_path = palschema_storage_dir.join(&folder_name);
                
                if storage_path.exists() {
                    let dest_dir = disabled_base.join("palschema");
                    let _ = fs::create_dir_all(&dest_dir);
                    let dest = dest_dir.join(&folder_name);
                    if fs::rename(&storage_path, &dest).is_ok() {
                        if m.mod_type == ModType::PalSchema {
                            m.disabled_path = dest.to_string_lossy().to_string();
                            m.game_path = String::new();
                        }
                    }
                }
            }
        }

        // Sync active profile lists (outside data.mods borrow)
        if let (Some(_), Some(mod_name)) = (folder_name_opt, mod_name_opt) {
            if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
                // Sync enabled_mod_ids
                let position = profile.enabled_mod_ids.iter().position(|name| name.to_lowercase() == id.to_lowercase() || name.to_lowercase() == mod_name.to_lowercase());
                if *enabled {
                    if position.is_none() {
                        profile.enabled_mod_ids.push(mod_name.clone());
                    }
                } else if let Some(pos) = position {
                    profile.enabled_mod_ids.remove(pos);
                }
            }
        }
    }

    let data_clone = data.clone();
    drop(data);
    let _ = crate::db::save_db(&data_clone.settings.program_path, &data_clone);

    Ok(())
}
