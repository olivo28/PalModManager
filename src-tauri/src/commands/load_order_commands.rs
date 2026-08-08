use crate::state::AppState;
use crate::models::{ModInfo, ModType};
use tauri::State;
use std::path::Path;
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
        if let Some(&is_enabled) = enabled_map.get(&m.name.to_lowercase()) {
            m.enabled = is_enabled;
        } else {
            m.enabled = true; // Default to true if not in mods.txt yet
        }
    }

    // 3. Sort by mods.txt weight, fallback to alphabetical
    target_mods.sort_by(|a, b| {
        let weight_a = order_map.get(&a.name.to_lowercase()).copied().unwrap_or(usize::MAX);
        let weight_b = order_map.get(&b.name.to_lowercase()).copied().unwrap_or(usize::MAX);
        
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
    let mods_txt = binaries_dir.join("ue4ss").join("Mods").join("mods.txt");
    if !mods_txt.exists() {
        return Err("mods.txt not found".to_string());
    }

    let content = fs::read_to_string(&mods_txt).map_err(|e| e.to_string())?;
    
    // Build list of custom mod lines to insert
    let mut custom_lines = Vec::new();
    let ordered_ids: Vec<String> = ordered_items.iter().map(|(id, _)| id.clone()).collect();
    for (id, enabled) in &ordered_items {
        if let Some(m) = data.mods.iter().find(|m| &m.id == id) {
            let val = if *enabled { "1" } else { "0" };
            let folder_name = crate::profiles::get_mod_folder_name(m);
            custom_lines.push(format!("{} : {}", folder_name, val));
        }
    }

    // Process existing mods.txt and filter out any existing custom mod references
    let mut lines_to_keep = Vec::new();
    for line in content.lines() {
        let line_clean = line.trim();
        if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
            if let Some(pos) = line_clean.find(':') {
                let name = line_clean[..pos].trim();
                if ordered_ids.iter().any(|id| data.mods.iter().any(|m| &m.id == id && (crate::profiles::get_mod_folder_name(m).to_lowercase() == name.to_lowercase() || m.name.to_lowercase() == name.to_lowercase()))) {
                    continue;
                }
            } else {
                let name = line_clean;
                if ordered_ids.iter().any(|id| data.mods.iter().any(|m| &m.id == id && (crate::profiles::get_mod_folder_name(m).to_lowercase() == name.to_lowercase() || m.name.to_lowercase() == name.to_lowercase()))) {
                    continue;
                }
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
