use crate::models::{AppData, ModInfo, ModType, Profile};
use std::fs;
use std::path::{Path, PathBuf};

/// Creates an NTFS Junction on Windows (zero-admin required) or a symlink on Unix
pub fn create_junction_or_symlink(target: &Path, link: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        // junction::create creates an NTFS junction
        junction::create(target, link).map_err(|e| format!("Failed to create junction from {:?} to {:?}: {}", target, link, e))
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(|e| format!("Failed to create symlink: {}", e))
    }
}

/// Safely removes a junction or directory link without deleting the contents of the target folder
pub fn remove_junction_or_symlink(link: &Path) -> Result<(), String> {
    if !link.exists() {
        return Ok(());
    }
    // On Windows, removing a junction can be done safely by removing the directory link entry itself
    // fs::remove_dir works on directory junctions/symlinks without deleting target content.
    fs::remove_dir(link).map_err(|e| format!("Failed to remove junction: {}", e))
}


pub fn sanitize_profile_id(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let trimmed = clean.trim_matches('_');
    if trimmed.is_empty() {
        "profile".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn get_profile_dir(program_path: &str, profile_id: &str) -> PathBuf {
    PathBuf::from(program_path).join("profiles").join(profile_id)
}

pub fn ensure_profile_structure(program_path: &str, profile_id: &str) -> PathBuf {
    let p_dir = get_profile_dir(program_path, profile_id);
    let _ = fs::create_dir_all(p_dir.join("ue4ss"));
    let _ = fs::create_dir_all(p_dir.join("palschema"));
    let _ = fs::create_dir_all(p_dir.join("paks"));
    let _ = fs::create_dir_all(p_dir.join("logicmods"));
    p_dir
}

pub fn cleanup_profile_mod_lists(data: &mut AppData) {
    for profile in &mut data.profiles {
        // Migrate old profiles: if installed_mod_ids is empty but enabled_mod_ids has content,
        // seed installed_mod_ids from enabled_mod_ids for backward compatibility
        if profile.installed_mod_ids.is_empty() && !profile.enabled_mod_ids.is_empty() {
            profile.installed_mod_ids = profile.enabled_mod_ids.clone();
        }
    }
}

/// Backward-compat alias
pub fn cleanup_profile_enabled_ids(data: &mut AppData) {
    cleanup_profile_mod_lists(data);
}

/// Returns the set of mod names installed in the given profile.
/// Used by get_mods/scan_mods to filter the global mod list to only profile-relevant mods.
#[allow(dead_code)]
pub fn get_profile_mod_names(data: &AppData, profile_id: &str) -> std::collections::HashSet<String> {
    if let Some(profile) = data.profiles.iter().find(|p| p.id == profile_id) {
        profile.installed_mod_ids.iter()
            .map(|s| s.to_lowercase())
            .collect()
    } else {
        std::collections::HashSet::new()
    }
}

pub fn ensure_default_profile(data: &mut AppData) {
    let program_path = data.settings.program_path.clone();
    let profiles_base = PathBuf::from(&program_path).join("profiles");
    let _ = fs::create_dir_all(&profiles_base);

    if !data.profiles.iter().any(|p| p.id == "default") {
        let now = chrono::Utc::now().to_rfc3339();
        let (ue4ss_installed, palschema_installed) = {
            let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&data.settings.game_path));
            (win64.join("dwmapi.dll").exists(), win64.join("ue4ss").join("Mods").join("PalSchema").join("dlls").join("main.dll").exists())
        };

        data.profiles.push(Profile {
            id: "default".to_string(),
            name: "Default".to_string(),
            created_at: now,
            installed_mod_ids: Vec::new(),
            enabled_mod_ids: Vec::new(),
            ue4ss_enabled: ue4ss_installed,
            palschema_enabled: palschema_installed,
            mod_folders: Vec::new(),
            load_order_metadata: None,
            force_load_order_ue4ss: None,
            force_load_order_palschema: None,
            hide_native_mods: None,
        });
    }

    // Migrate old profiles: if installed_mod_ids is empty but enabled_mod_ids has content,
    // seed installed_mod_ids for backward compatibility
    for profile in &mut data.profiles {
        if profile.installed_mod_ids.is_empty() && !profile.enabled_mod_ids.is_empty() {
            profile.installed_mod_ids = profile.enabled_mod_ids.clone();
        }
    }

    migrate_profile_uuids_to_stable_ids(data);

    if data.current_profile_id.is_empty() {
        data.current_profile_id = "default".to_string();
    }

    for p in &data.profiles {
        let p_dir = ensure_profile_structure(&program_path, &p.id);
        if let Ok(json) = serde_json::to_string_pretty(p) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }

    cleanup_profile_mod_lists(data);
}

pub fn migrate_profile_uuids_to_stable_ids(data: &mut AppData) {
    let mods = data.mods.clone();
    for profile in &mut data.profiles {
        for id_entry in &mut profile.installed_mod_ids {
            if let Some(matching_mod) = mods.iter().find(|m| {
                m.id.to_lowercase() == id_entry.to_lowercase() ||
                m.name.to_lowercase() == id_entry.to_lowercase()
            }) {
                if id_entry != &matching_mod.id {
                    crate::logger::log(&format!("Migrating profile installed ID '{}' -> stable ID '{}'", id_entry, matching_mod.id));
                    *id_entry = matching_mod.id.clone();
                }
            }
        }
        
        for id_entry in &mut profile.enabled_mod_ids {
            if let Some(matching_mod) = mods.iter().find(|m| {
                m.id.to_lowercase() == id_entry.to_lowercase() ||
                m.name.to_lowercase() == id_entry.to_lowercase()
            }) {
                if id_entry != &matching_mod.id {
                    crate::logger::log(&format!("Migrating profile enabled ID '{}' -> stable ID '{}'", id_entry, matching_mod.id));
                    *id_entry = matching_mod.id.clone();
                }
            }
        }

        profile.installed_mod_ids.dedup();
        profile.enabled_mod_ids.dedup();
    }
}


pub fn mod_matches_profile_entry(mod_info: &crate::models::ModInfo, entry: &str) -> bool {
    let entry_lower = entry.to_lowercase();
    if entry_lower == mod_info.id.to_lowercase() {
        return true;
    }
    if entry_lower == mod_info.name.to_lowercase() {
        return true;
    }
    // Check against game_path folder name
    if !mod_info.game_path.is_empty() {
        if let Some(filename) = std::path::Path::new(&mod_info.game_path).file_name() {
            let folder_name = filename.to_string_lossy().to_lowercase();
            if folder_name == entry_lower {
                return true;
            }
            // Strip .disabled if it got appended
            if let Some(stripped) = folder_name.strip_suffix(".disabled") {
                if stripped == entry_lower {
                    return true;
                }
            }
        }
    }
    // Check against disabled_path folder name
    if !mod_info.disabled_path.is_empty() {
        if let Some(filename) = std::path::Path::new(&mod_info.disabled_path).file_name() {
            let folder_name = filename.to_string_lossy().to_lowercase();
            if folder_name == entry_lower {
                return true;
            }
            if let Some(stripped) = folder_name.strip_suffix(".disabled") {
                if stripped == entry_lower {
                    return true;
                }
            }
        }
    }
    false
}

pub fn sync_current_profile_states(data: &mut AppData) {
    cleanup_profile_mod_lists(data);
    let current_id = data.current_profile_id.clone();
    if let Some(profile) = data.profiles.iter().find(|p| p.id == current_id).cloned() {
        for mod_info in &mut data.mods {
            if mod_info.nexus_author.as_deref() == Some("UE4SS Native Mod") {
                // Native mods: enabled state comes from mods.txt, not from profile
                continue;
            }
            // Enabled = matches any entry in this profile's enabled_mod_ids
            let is_enabled = profile.enabled_mod_ids.iter().any(|entry| {
                mod_matches_profile_entry(mod_info, entry)
            });
            mod_info.enabled = is_enabled;
        }
    }
}

pub fn set_profile_mod_state(
    data: &mut AppData,
    profile_id: &str,
    mod_id: &str,
    enabled: bool,
) -> Result<(), String> {
    let program_path = data.settings.program_path.clone();
    let mod_name = data.mods.iter().find(|m| m.id == mod_id).map(|m| m.name.clone());

    {
        let profile = data
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "Profile not found".to_string())?;

        if enabled {
            // Use mod name (stable across scans) not UUID (which changes if mod is re-scanned)
            if let Some(ref name) = mod_name {
                let already_present = profile.enabled_mod_ids.iter().any(|id| id.to_lowercase() == name.to_lowercase());
                if !already_present {
                    profile.enabled_mod_ids.push(name.clone());
                }
            }
        } else {
            profile.enabled_mod_ids.retain(|id| id != mod_id);
            if let Some(ref name) = mod_name {
                profile.enabled_mod_ids.retain(|id| id.to_lowercase() != name.to_lowercase());
            }
        }
    }

    cleanup_profile_enabled_ids(data);

    // Save individual profile metadata file
    let p_dir = get_profile_dir(&program_path, profile_id);
    if let Some(profile) = data.profiles.iter().find(|p| p.id == profile_id) {
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }

    // Apply physically if this is the currently active profile
    if profile_id == data.current_profile_id {
        crate::logger::log(&format!("set_profile_mod_state: Applying switch physically for mod = {} -> enabled = {}", mod_id, enabled));
        if enabled {
            enable_mod_internal(data, &program_path, mod_id)?;
        } else {
            disable_mod_internal(data, &program_path, mod_id)?;
        }
    }

    Ok(())
}

fn backup_game_files_to_profile(game_path: &str, profile_dir: &Path) {
    if game_path.is_empty() { return; }
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let ue4ss_game = win64.join("ue4ss");
    let dwmapi_game = win64.join("dwmapi.dll");

    let ue4ss_backup = profile_dir.join("ue4ss");
    if ue4ss_backup.exists() {
        let _ = fs::remove_dir_all(&ue4ss_backup);
    }
    if ue4ss_game.exists() {
        let _ = copy_dir_all(&ue4ss_game, &ue4ss_backup);
    }

    let dwmapi_backup = profile_dir.join("dwmapi.dll");
    if dwmapi_backup.exists() {
        let _ = fs::remove_file(&dwmapi_backup);
    }
    if dwmapi_game.exists() {
        let _ = fs::copy(&dwmapi_game, &dwmapi_backup);
    }

    let palschema_game = win64.join("ue4ss").join("Mods").join("PalSchema").join("mods");
    let palschema_backup = profile_dir.join("palschema");
    if palschema_backup.exists() {
        let _ = fs::remove_dir_all(&palschema_backup);
    }
    if palschema_game.exists() {
        let _ = copy_dir_all(&palschema_game, &palschema_backup);
    }

    let paks_game = PathBuf::from(game_path).join("Pal").join("Content").join("Paks").join("~mods");
    let paks_backup = profile_dir.join("paks");
    if paks_backup.exists() {
        let _ = fs::remove_dir_all(&paks_backup);
    }
    if paks_game.exists() {
        let _ = copy_dir_all(&paks_game, &paks_backup);
    }

    let logic_game = PathBuf::from(game_path).join("Pal").join("Content").join("Paks").join("LogicMods");
    let logic_backup = profile_dir.join("logicmods");
    if logic_backup.exists() {
        let _ = fs::remove_dir_all(&logic_backup);
    }
    if logic_game.exists() {
        let _ = copy_dir_all(&logic_game, &logic_backup);
    }
}

fn restore_profile_files_to_game(game_path: &str, profile_dir: &Path, target_profile: &Profile, program_path: &str) {
    if game_path.is_empty() { return; }
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let ue4ss_game = win64.join("ue4ss");
    let dwmapi_game = win64.join("dwmapi.dll");
    let palschema_game = win64.join("ue4ss").join("Mods").join("PalSchema").join("mods");
    let paks_game = PathBuf::from(game_path).join("Pal").join("Content").join("Paks").join("~mods");
    let logic_game = PathBuf::from(game_path).join("Pal").join("Content").join("Paks").join("LogicMods");

    // Clear game directory first
    if dwmapi_game.exists() { let _ = fs::remove_file(&dwmapi_game); }
    if ue4ss_game.exists() { let _ = fs::remove_dir_all(&ue4ss_game); }
    if paks_game.exists() { let _ = fs::remove_dir_all(&paks_game); }
    if logic_game.exists() { let _ = fs::remove_dir_all(&logic_game); }

    if target_profile.ue4ss_enabled {
        let ue4ss_backup = profile_dir.join("ue4ss");
        let dwmapi_backup = profile_dir.join("dwmapi.dll");
        if ue4ss_backup.exists() && fs::read_dir(&ue4ss_backup).map(|mut d| d.next().is_some()).unwrap_or(false) {
            let _ = copy_dir_all(&ue4ss_backup, &ue4ss_game);
            if dwmapi_backup.exists() {
                let _ = fs::copy(&dwmapi_backup, &dwmapi_game);
            }
        } else {
            let _ = sync_profile_dependencies(game_path, program_path, target_profile);
        }
    }

    if target_profile.palschema_enabled {
        let palschema_backup = profile_dir.join("palschema");
        if palschema_backup.exists() {
            let _ = copy_dir_all(&palschema_backup, &palschema_game);
        }
    }

    let paks_backup = profile_dir.join("paks");
    if paks_backup.exists() {
        let _ = copy_dir_all(&paks_backup, &paks_game);
    }

    let logic_backup = profile_dir.join("logicmods");
    if logic_backup.exists() {
        let _ = copy_dir_all(&logic_backup, &logic_game);
    }
}

pub fn switch_profile(
    data: &mut AppData,
    program_path: &str,
    target_profile: &Profile,
) -> Result<Vec<ModInfo>, String> {
    crate::logger::log(&format!("switch_profile: Switching to profile '{}' (folder: {})", target_profile.name, target_profile.id));

    let game_path = data.settings.game_path.clone();

    // 1. Backup physical files of current active profile + mods.txt snapshot
    let current_id = data.current_profile_id.clone();
    if !current_id.is_empty() {
        let current_dir = ensure_profile_structure(program_path, &current_id);
        backup_game_files_to_profile(&game_path, &current_dir);
        
        if !game_path.is_empty() {
            let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
            let mods_txt = binaries_dir.join("ue4ss").join("Mods").join("mods.txt");
            if mods_txt.exists() {
                let snapshot_path = current_dir.join("mods.txt.snapshot");
                let _ = fs::copy(&mods_txt, &snapshot_path);
            }
        }
    }

    // 2. Restore target profile physical files to game + mods.txt snapshot
    let target_dir = ensure_profile_structure(program_path, &target_profile.id);
    restore_profile_files_to_game(&game_path, &target_dir, target_profile, program_path);

    if !game_path.is_empty() {
        let binaries_dir = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
        let mods_txt = binaries_dir.join("ue4ss").join("Mods").join("mods.txt");
        let snapshot_path = target_dir.join("mods.txt.snapshot");
        if snapshot_path.exists() {
            let _ = fs::copy(&snapshot_path, &mods_txt);
        }
    }

    // 3. Update memory state
    data.current_profile_id = target_profile.id.clone();
    cleanup_profile_enabled_ids(data);
    sync_current_profile_states(data);

    // Save profile metadata
    if let Ok(json) = serde_json::to_string_pretty(target_profile) {
        let _ = fs::write(target_dir.join("profile.json"), json);
    }

    Ok(data.mods.clone())
}

pub fn create_profile(data: &mut AppData, name: String) -> Result<Profile, String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err("Invalid profile name".to_string());
    }

    let profile_id = sanitize_profile_id(&name);
    if data.profiles.iter().any(|p| p.id == profile_id) {
        return Err("A profile with a similar name already exists".to_string());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let profile = Profile {
        id: profile_id.clone(),
        name,
        created_at: now,
        installed_mod_ids: Vec::new(),
        enabled_mod_ids: Vec::new(),
        ue4ss_enabled: false,
        palschema_enabled: false,
        mod_folders: Vec::new(),
        load_order_metadata: None,
        force_load_order_ue4ss: None,
        force_load_order_palschema: None,
        hide_native_mods: None,
    };

    let program_path = data.settings.program_path.clone();
    let p_dir = ensure_profile_structure(&program_path, &profile.id);
    if let Ok(json) = serde_json::to_string_pretty(&profile) {
        let _ = fs::write(p_dir.join("profile.json"), json);
    }

    data.profiles.push(profile.clone());
    Ok(profile)
}

pub fn clone_profile(data: &mut AppData, source_profile_id: &str, new_name: String) -> Result<Profile, String> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() || new_name.len() > 100 {
        return Err("Invalid profile name".to_string());
    }

    let source_profile = data.profiles.iter().find(|p| p.id == source_profile_id)
        .ok_or_else(|| "Source profile not found".to_string())?.clone();

    let new_profile_id = sanitize_profile_id(&new_name);
    if data.profiles.iter().any(|p| p.id == new_profile_id) {
        return Err("A profile with a similar name already exists".to_string());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let new_profile = Profile {
        id: new_profile_id.clone(),
        name: new_name,
        created_at: now,
        installed_mod_ids: source_profile.installed_mod_ids.clone(),
        enabled_mod_ids: source_profile.enabled_mod_ids.clone(),
        ue4ss_enabled: source_profile.ue4ss_enabled,
        palschema_enabled: source_profile.palschema_enabled,
        mod_folders: source_profile.mod_folders.clone(),
        load_order_metadata: source_profile.load_order_metadata.clone(),
        force_load_order_ue4ss: source_profile.force_load_order_ue4ss,
        force_load_order_palschema: source_profile.force_load_order_palschema,
        hide_native_mods: source_profile.hide_native_mods,
    };

    let program_path = data.settings.program_path.clone();
    let src_dir = get_profile_dir(&program_path, source_profile_id);
    let dst_dir = ensure_profile_structure(&program_path, &new_profile.id);

    // If we are cloning the active profile, back up its active files first to keep the clone up-to-date
    if source_profile_id == data.current_profile_id {
        backup_game_files_to_profile(&data.settings.game_path, &src_dir);
    }

    // Copy physical backed-up mod folders/files from source profile directory to the new profile directory
    if src_dir.exists() {
        for folder in &["ue4ss", "palschema", "paks", "logicmods"] {
            let src_folder = src_dir.join(folder);
            let dst_folder = dst_dir.join(folder);
            if src_folder.exists() {
                let _ = copy_dir_all(&src_folder, &dst_folder);
            }
        }
        let dwmapi_src = src_dir.join("dwmapi.dll");
        if dwmapi_src.exists() {
            let _ = fs::copy(&dwmapi_src, dst_dir.join("dwmapi.dll"));
        }
    }

    if let Ok(json) = serde_json::to_string_pretty(&new_profile) {
        let _ = fs::write(dst_dir.join("profile.json"), json);
    }

    data.profiles.push(new_profile.clone());
    Ok(new_profile)
}

pub fn delete_profile(data: &mut AppData, profile_id: &str) -> Result<(), String> {
    if profile_id == "default" {
        return Err("Cannot delete the default profile".to_string());
    }

    let idx = data
        .profiles
        .iter()
        .position(|p| p.id == profile_id)
        .ok_or_else(|| "Profile not found".to_string())?;

    if data.current_profile_id == profile_id {
        return Err("Cannot delete the active profile. Switch profiles first.".to_string());
    }

    let program_path = data.settings.program_path.clone();
    let p_dir = get_profile_dir(&program_path, profile_id);
    if p_dir.exists() {
        let _ = fs::remove_dir_all(p_dir);
    }

    data.profiles.remove(idx);
    Ok(())
}

pub fn clear_profile(data: &mut AppData, profile_id: &str) -> Result<(), String> {
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();

    let profile = data
        .profiles
        .iter_mut()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| "Profile not found".to_string())?;

    // 1. Wipe mod listings and virtual folders
    profile.installed_mod_ids.clear();
    profile.enabled_mod_ids.clear();
    profile.mod_folders.clear();

    // 2. Wipe physical backup directories of this profile
    let p_dir = get_profile_dir(&program_path, profile_id);
    if p_dir.exists() {
        for folder in &["ue4ss", "palschema", "paks", "logicmods", "disabled_mods"] {
            let sub = p_dir.join(folder);
            if sub.exists() {
                let _ = fs::remove_dir_all(&sub);
            }
        }
        let dwmapi = p_dir.join("dwmapi.dll");
        if dwmapi.exists() {
            let _ = fs::remove_file(&dwmapi);
        }
        
        // Write the empty profile.json back
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }

    // 3. If it is the current active profile, clear the physical files from the game folder
    if profile_id == data.current_profile_id && !game_path.is_empty() {
        let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
        let ue4ss_game = win64.join("ue4ss");
        let dwmapi_game = win64.join("dwmapi.dll");
        let paks_game = PathBuf::from(&game_path).join("Pal").join("Content").join("Paks").join("~mods");
        let logic_game = PathBuf::from(&game_path).join("Pal").join("Content").join("Paks").join("LogicMods");

        if dwmapi_game.exists() { let _ = fs::remove_file(&dwmapi_game); }
        if ue4ss_game.exists() { let _ = fs::remove_dir_all(&ue4ss_game); }
        if paks_game.exists() { let _ = fs::remove_dir_all(&paks_game); }
        if logic_game.exists() { let _ = fs::remove_dir_all(&logic_game); }

        // Trigger restore dependency defaults to keep UE4SS and PalSchema active if configured
        restore_profile_files_to_game(&game_path, &p_dir, profile, &program_path);
    }

    Ok(())
}


pub fn rename_profile(data: &mut AppData, profile_id: &str, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err("Invalid profile name".to_string());
    }

    let profile = data
        .profiles
        .iter_mut()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| "Profile not found".to_string())?;

    profile.name = name;
    Ok(())
}

// Internal helpers

fn update_mods_txt_setting(mods_txt: &Path, mod_name: &str, enabled: bool) -> Result<(), String> {
    let content = fs::read_to_string(mods_txt).map_err(|e| e.to_string())?;
    let mut new_lines = Vec::new();
    let mut found = false;
    let target_val = if enabled { "1" } else { "0" };

    for line in content.lines() {
        let line_clean = line.trim();
        if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
            if let Some(pos) = line_clean.find(':') {
                let name = line_clean[..pos].trim();
                if name.to_lowercase() == mod_name.to_lowercase() {
                    new_lines.push(format!("{} : {}", name, target_val));
                    found = true;
                    continue;
                }
            }
        }
        new_lines.push(line.to_string());
    }

    if !found {
        new_lines.push(format!("{} : {}", mod_name, target_val));
    }

    fs::write(mods_txt, new_lines.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_mod_folder_name(mod_info: &ModInfo) -> String {
    if !mod_info.game_path.is_empty() {
        if let Some(name) = Path::new(&mod_info.game_path).file_name() {
            return name.to_string_lossy().to_string();
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
            update_mods_txt_setting(&path, &mod_info.name, false)?;
        } else {
            return Err("mods.txt not found relative to native mod game_path".to_string());
        }
        mod_info.enabled = false;
        // Native mods are not tracked in profile enabled_mod_ids
        return Ok(());
    }

    let current_id = data.current_profile_id.clone();
    let profile_dir = get_profile_dir(program_path, &current_id);
    let disabled_base = profile_dir.join("disabled_mods");

    if mod_type == ModType::Ue4ss {
        let mod_info = &mut data.mods[mod_index];
        let src_path = PathBuf::from(&mod_info.game_path);
        if src_path.exists() {
            if let Some(ue4ss_mods_dir) = src_path.parent() {
                let mods_txt = ue4ss_mods_dir.join("mods.txt");
                if mods_txt.exists() {
                    let folder_name = get_mod_folder_name(mod_info);
                    let _ = remove_from_mods_txt(&mods_txt, &folder_name);
                    let _ = remove_from_mods_txt(&mods_txt, &mod_info.name);
                    
                    for extra_path_str in &mod_info.extra_files {
                        let extra_path_lower = extra_path_str.to_lowercase();
                        if extra_path_lower.contains("ue4ss/mods/") {
                            let extra_path = Path::new(extra_path_str);
                            if let Some(extra_folder_name) = extra_path.file_name().map(|n| n.to_string_lossy().to_string()) {
                                let _ = remove_from_mods_txt(&mods_txt, &extra_folder_name);
                            }
                        }
                    }
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
        let src_path = PathBuf::from(&mod_info.game_path); // Could be the junction in mods/ or storage
        let folder_name = get_mod_folder_name(mod_info);

        let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&data.settings.game_path));
        let palschema_mods_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("mods");
        let palschema_storage_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("Storage");

        // 1. Remove junction from mods/ (either numbered prefix folder or standard folder)
        if palschema_mods_dir.exists() {
            if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    // Match either folder_name directly or with numerical prefix (e.g. 001_folder_name)
                    if name == folder_name || (name.len() > 4 && &name[4..] == folder_name) {
                        let _ = remove_junction_or_symlink(&path);
                    }
                }
            }
        }

        // 2. Move physical directory from Storage/ to disabled_mods/palschema
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
        
        // Remove secondary companion UE4SS folders from mods.txt before moving them
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
        profile.enabled_mod_ids.retain(|id| id != mod_id && id.to_lowercase() != mod_name.to_lowercase());
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
    let force_ue4ss_effective = effective_force_ue4ss(data);
    let force_palschema_effective = effective_force_palschema(data);

    let mod_index = data
        .mods
        .iter()
        .position(|m| m.id == mod_id)
        .ok_or("Mod not found")?;

    let is_native = data.mods[mod_index].nexus_author.as_deref() == Some("UE4SS Native Mod");
    let mod_type = data.mods[mod_index].mod_type.clone();

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
            update_mods_txt_setting(&path, &mod_info.name, true)?;
        } else {
            return Err("mods.txt not found relative to native mod game_path".to_string());
        }
        mod_info.enabled = true;
        // Native mods are not tracked in profile enabled_mod_ids
        return Ok(());
    }

    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&data.settings.game_path));
    let game_paks = PathBuf::from(&data.settings.game_path).join("Pal").join("Content").join("Paks");

    if mod_type == ModType::Ue4ss {
        let mod_info = &mut data.mods[mod_index];
        let primary_disabled = PathBuf::from(&mod_info.disabled_path);
        let mut dest_path = primary_disabled.clone();
        if primary_disabled.exists() {
            let filename = primary_disabled.file_name().unwrap().to_string_lossy().to_string();
            let dest = win64.join("ue4ss").join("Mods").join(&filename);
            let _ = fs::create_dir_all(dest.parent().unwrap());
            move_path(&primary_disabled, &dest)?;
            dest_path = dest;
        }
        mod_info.game_path = dest_path.to_string_lossy().to_string();
        mod_info.disabled_path = String::new();

        let force_order = data.settings.force_load_order.unwrap_or(false) && force_ue4ss_effective;
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
        mod_info.enabled = true;
    } else if mod_type == ModType::PalSchema {
        let mod_info = &mut data.mods[mod_index];
        let primary_disabled = PathBuf::from(&mod_info.disabled_path);
        
        let folder_name = get_mod_folder_name(mod_info);
        let force_order = data.settings.force_load_order.unwrap_or(false) && force_palschema_effective;

        let palschema_mods_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("mods");
        let palschema_storage_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("Storage");

        if primary_disabled.exists() {
            // 1. Move physical directory to Storage/
            let storage_dest = palschema_storage_dir.join(&folder_name);
            let _ = fs::create_dir_all(&palschema_storage_dir);
            move_path(&primary_disabled, &storage_dest)?;

            // 2. Create NTFS Junction in mods/
            let _ = fs::create_dir_all(&palschema_mods_dir);
            let link_name = if force_order {
                // Find order position (default to 999 if not set)
                let order = mod_info.mods_txt_order.unwrap_or(999);
                format!("{:03}_{}", order, folder_name)
            } else {
                folder_name.clone()
            };
            
            let link_path = palschema_mods_dir.join(&link_name);
            create_junction_or_symlink(&storage_dest, &link_path)?;

            mod_info.game_path = link_path.to_string_lossy().to_string();
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

        if primary_disabled.exists() {
            let filename = primary_disabled.file_name().unwrap().to_string_lossy().to_string();
            primary_has_scripts = primary_disabled.join("Scripts").exists()
                || primary_disabled.join("scripts").exists()
                || primary_disabled.join("enabled.txt").exists();
            let dest = if primary_has_scripts {
                win64.join("ue4ss").join("Mods").join(&filename)
            } else {
                win64.join("ue4ss").join("Mods").join("PalSchema").join("mods").join(&filename)
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
                    let dest_dir = win64.join("ue4ss").join("Mods").join("PalSchema").join("mods");
                    let _ = fs::create_dir_all(&dest_dir);
                    dest_dir.join(&filename)
                } else {
                    let dest_dir = win64.join("ue4ss").join("Mods");
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

        // 1. Register and setup companion UE4SS folders in mods.txt / enabled.txt FIRST
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

        // 2. Register and setup primary folder in mods.txt / enabled.txt LAST
        // (This ensures the primary folder is placed ABOVE/before the companion folders in mods.txt
        // because update_mods_txt_load_order inserts new items at the insertion index, pushing previous ones down)
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
    // Use mod name (stable across scans) instead of UUID (changes if mod is re-scanned)
    let current_id = data.current_profile_id.clone();
    let mod_name_for_profile = data.mods.iter().find(|m| m.id == mod_id).map(|m| m.name.clone());
    if let Some(ref name) = mod_name_for_profile {
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_id) {
            // Always ensure it's in installed_mod_ids
            let in_installed = profile.installed_mod_ids.iter().any(|id| id.to_lowercase() == name.to_lowercase());
            if !in_installed {
                profile.installed_mod_ids.push(name.clone());
            }
            // Add to enabled_mod_ids
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

pub fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Cannot create dest dir: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Cannot read source dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();
        let dest_path = dst.join(file_name);
        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| {
                format!("Cannot copy file {}: {}", file_name.to_string_lossy(), e)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn move_path(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    if src.is_dir() {
        copy_dir_all(src, dst)?;
        fs::remove_dir_all(src).map_err(|e| format!("Failed to remove source dir after cross-device copy: {}", e))?;
    } else {
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::copy(src, dst).map_err(|e| format!("Failed to copy source file during cross-device move: {}", e))?;
        fs::remove_file(src).map_err(|e| format!("Failed to remove source file after cross-device copy: {}", e))?;
    }
    Ok(())
}

fn save_pmm_meta_path(m: &crate::models::ModInfo, path_str: &str) -> Result<(), String> {
    if path_str.is_empty() {
        return Ok(());
    }
    let path = std::path::Path::new(path_str);
    if !path.exists() {
        return Ok(());
    }

    let pmm_path = if path.is_file() {
        std::path::PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()))
    } else {
        path.join(".pmm.json")
    };

    if let Ok(json) = serde_json::to_string_pretty(m) {
        let _ = std::fs::write(&pmm_path, json);
    }
    Ok(())
}

pub fn save_pmm_meta(m: &crate::models::ModInfo) -> Result<(), String> {
    let primary_path = if !m.game_path.is_empty() { &m.game_path } else { &m.disabled_path };
    let _ = save_pmm_meta_path(m, primary_path);

    for extra in &m.extra_files {
        if extra.to_lowercase().ends_with(".pak") {
            let _ = save_pmm_meta_path(m, extra);
        }
    }
    Ok(())
}

fn find_extracted_root(src: &std::path::Path) -> PathBuf {
    if src.is_dir() {
        let entries: Vec<_> = fs::read_dir(src)
            .ok()
            .into_iter()
            .flat_map(|rd| rd.filter_map(|e| e.ok()))
            .filter(|e| {
                let n = e.file_name();
                n != ".." && n != "." && n != "__MACOSX"
            })
            .collect();
        if entries.len() == 1 && entries[0].file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            return entries[0].path();
        }
    }
    src.to_path_buf()
}

fn sync_profile_dependencies(
    game_path: &str,
    program_path: &str,
    target_profile: &Profile,
) -> Result<(), String> {
    if game_path.is_empty() {
        return Ok(());
    }
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));

    // UE4SS
    let dwmapi = win64.join("dwmapi.dll");
    let ue4ss_dir = win64.join("ue4ss");
    if target_profile.ue4ss_enabled {
        let cached_zip = PathBuf::from(program_path)
            .join("mods-library")
            .join("dependencies")
            .join("ue4ss.zip");
        if cached_zip.exists() && (!dwmapi.exists() || !ue4ss_dir.exists()) {
            crate::logger::log("sync_profile_dependencies: Installing UE4SS from local library (offline)...");
            let temp_dir = std::env::temp_dir().join(format!("pmm_sync_ue4ss_{}", uuid::Uuid::new_v4()));
            if let Ok(extracted) = crate::zip_handler::extract_zip_to_temp(&cached_zip.to_string_lossy(), &temp_dir.join("extracted")) {
                let root = find_extracted_root(&extracted);
                let (framework_src, dwmapi_src) = {
                    let ue4ss_sub = root.join("ue4ss");
                    if ue4ss_sub.is_dir() {
                        (ue4ss_sub, root.join("dwmapi.dll"))
                    } else {
                        (root.clone(), root.join("dwmapi.dll"))
                    }
                };
                if dwmapi_src.exists() {
                    let _ = fs::copy(&dwmapi_src, &win64.join("dwmapi.dll"));
                }
                let _ = fs::create_dir_all(&ue4ss_dir);
                if let Ok(rd) = fs::read_dir(&framework_src) {
                    for entry in rd.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name == "dwmapi.dll" || name == "Mods" || name == "mods" {
                            continue;
                        }
                        let dst = ue4ss_dir.join(&name);
                        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            let _ = copy_dir_all(&entry.path(), &dst);
                        } else {
                            let _ = fs::copy(&entry.path(), &dst);
                        }
                    }
                }
                if let Ok(ver) = fs::read_to_string(PathBuf::from(program_path).join("mods-library").join("dependencies").join("ue4ss.version")) {
                    let _ = fs::write(ue4ss_dir.join("ue4ss.version"), ver.trim());
                }
            }
            let _ = fs::remove_dir_all(&temp_dir);
        }
    } else {
        if dwmapi.exists() {
            let _ = fs::remove_file(dwmapi);
        }
        if ue4ss_dir.exists() {
            let _ = fs::remove_dir_all(ue4ss_dir);
        }
    }

    // PalSchema
    let palschema_dir = win64.join("ue4ss").join("Mods").join("PalSchema");
    if target_profile.palschema_enabled {
        let cached_zip = PathBuf::from(program_path)
            .join("mods-library")
            .join("dependencies")
            .join("palschema.zip");
        if cached_zip.exists() && !palschema_dir.exists() {
            crate::logger::log("sync_profile_dependencies: Installing PalSchema from local library (offline)...");
            let temp_dir = std::env::temp_dir().join(format!("pmm_sync_ps_{}", uuid::Uuid::new_v4()));
            if let Ok(extracted) = crate::zip_handler::extract_zip_to_temp(&cached_zip.to_string_lossy(), &temp_dir.join("extracted")) {
                let root = find_extracted_root(&extracted);
                let _ = fs::create_dir_all(&palschema_dir);
                if let Ok(rd) = fs::read_dir(&root) {
                    for entry in rd.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name == "mods" || name == "Mods" {
                            continue;
                        }
                        let dst = palschema_dir.join(&name);
                        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            let _ = copy_dir_all(&entry.path(), &dst);
                        } else {
                            let _ = fs::copy(&entry.path(), &dst);
                        }
                    }
                }
                if let Ok(ver) = fs::read_to_string(PathBuf::from(program_path).join("mods-library").join("dependencies").join("palschema.version")) {
                    let _ = fs::write(palschema_dir.join("palschema.version"), ver.trim());
                }
            }
            let _ = fs::remove_dir_all(&temp_dir);
        }
    } else {
        if palschema_dir.exists() {
            let _ = fs::remove_dir_all(palschema_dir);
        }
    }

    Ok(())
}

pub fn auto_add_scanned_mods_to_profile(data: &mut AppData) {
    let current_profile_id = data.current_profile_id.clone();
    let program_path = data.settings.program_path.clone();
    let mut modified = false;

    if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
        let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&data.settings.game_path));
        let ue4ss_installed = win64.join("dwmapi.dll").exists();
        let palschema_installed = win64.join("ue4ss").join("Mods").join("PalSchema").join("dlls").join("main.dll").exists();

        if profile.ue4ss_enabled != ue4ss_installed {
            profile.ue4ss_enabled = ue4ss_installed;
            modified = true;
        }
        if profile.palschema_enabled != palschema_installed {
            profile.palschema_enabled = palschema_installed;
            modified = true;
        }

        for m in &data.mods {
            if m.nexus_author.as_deref() == Some("UE4SS Native Mod") {
                continue;
            }

            let is_in_game = !m.game_path.is_empty() && Path::new(&m.game_path).exists();
            let is_disabled = !m.disabled_path.is_empty()
                && m.disabled_path.replace("\\", "/").contains(&format!("/profiles/{}/disabled_mods/", profile.id))
                && Path::new(&m.disabled_path).exists();

            // If the mod is physically located in another profile's disabled directory, do not auto-add it to this one.
            let is_in_other_profile_disabled = !m.disabled_path.is_empty()
                && !m.disabled_path.replace("\\", "/").contains(&format!("/profiles/{}/disabled_mods/", profile.id))
                && Path::new(&m.disabled_path).exists();

            if (is_in_game && !is_in_other_profile_disabled) || is_disabled {
                let already_installed = profile.installed_mod_ids.iter().any(|id| {
                    id.to_lowercase() == m.id.to_lowercase() || id.to_lowercase() == m.name.to_lowercase()
                });
                if !already_installed {
                    profile.installed_mod_ids.push(m.id.clone());
                    modified = true;
                }

                if is_in_game {
                    let already_enabled = profile.enabled_mod_ids.iter().any(|id| {
                        id.to_lowercase() == m.id.to_lowercase() || id.to_lowercase() == m.name.to_lowercase()
                    });
                    if !already_enabled {
                        profile.enabled_mod_ids.push(m.id.clone());
                        modified = true;
                    }
                }
            }
        }

        if modified {
            let p_dir = get_profile_dir(&program_path, &current_profile_id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
    }
}

pub fn effective_force_ue4ss(data: &AppData) -> bool {
    let profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
    profile.and_then(|p| p.force_load_order_ue4ss)
        .or(data.settings.force_load_order_ue4ss)
        .unwrap_or(false)
}

pub fn effective_force_palschema(data: &AppData) -> bool {
    let profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
    profile.and_then(|p| p.force_load_order_palschema)
        .or(data.settings.force_load_order_palschema)
        .unwrap_or(false)
}

pub fn effective_hide_native_mods(data: &AppData) -> bool {
    let profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
    profile.and_then(|p| p.hide_native_mods)
        .or(data.settings.hide_native_mods)
        .unwrap_or(false)
}

