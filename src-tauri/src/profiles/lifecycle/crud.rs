use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{AppData, DependencyMode, Profile};
use super::super::utils::{copy_dir_all, ensure_profile_structure, get_profile_dir, sanitize_profile_id};
use super::backup_restore::{backup_game_files_to_profile, restore_profile_files_to_game};

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
        dependency_mode: DependencyMode::None,
        mod_folders: Vec::new(),
        load_order_metadata: None,
        force_load_order_ue4ss: None,
        force_load_order_palschema: None,
        hide_native_mods: None,
        ue4ss_version: None,
        palschema_version: None,
        altermatic_version: None,
        unipalui_version: None,
        compatibility_patches: None,
        ue4ss_control_mode: Some("enabled_txt".to_string()),
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
        dependency_mode: source_profile.dependency_mode.clone(),
        mod_folders: source_profile.mod_folders.clone(),
        load_order_metadata: source_profile.load_order_metadata.clone(),
        force_load_order_ue4ss: source_profile.force_load_order_ue4ss,
        force_load_order_palschema: source_profile.force_load_order_palschema,
        hide_native_mods: source_profile.hide_native_mods,
        ue4ss_version: source_profile.ue4ss_version.clone(),
        palschema_version: source_profile.palschema_version.clone(),
        altermatic_version: source_profile.altermatic_version.clone(),
        unipalui_version: source_profile.unipalui_version.clone(),
        compatibility_patches: source_profile.compatibility_patches.clone(),
        ue4ss_control_mode: source_profile.ue4ss_control_mode.clone(),
    };

    let program_path = data.settings.program_path.clone();
    let src_dir = get_profile_dir(&program_path, source_profile_id);
    let dst_dir = ensure_profile_structure(&program_path, &new_profile.id);

    if source_profile_id == data.current_profile_id {
        backup_game_files_to_profile(&data.settings.game_path, &src_dir, &source_profile);
    }

    if src_dir.exists() {
        for folder in &[
            "ue4ss",
            "ue4ss_mods",
            "ue4ss_workshop_root",
            "ue4ss_workshop_mods",
            "palschema",
            "paks",
            "logicmods",
            "disabled_mods",
            "ManagedMods",
        ] {
            let src_folder = src_dir.join(folder);
            let dst_folder = dst_dir.join(folder);
            if src_folder.exists() {
                let _ = copy_dir_all(&src_folder, &dst_folder);
            }
        }
        for file in &["dwmapi.dll", "mods.txt.snapshot", "PalModSettings.ini"] {
            let src_file = src_dir.join(file);
            let dst_file = dst_dir.join(file);
            if src_file.exists() {
                let _ = fs::copy(&src_file, &dst_file);
            }
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

    profile.installed_mod_ids.clear();
    profile.enabled_mod_ids.clear();
    profile.mod_folders.clear();

    let p_dir = get_profile_dir(&program_path, profile_id);
    if p_dir.exists() {
        for folder in &[
            "ue4ss",
            "ue4ss_mods",
            "ue4ss_workshop_root",
            "ue4ss_workshop_mods",
            "palschema",
            "paks",
            "logicmods",
            "disabled_mods",
            "ManagedMods",
        ] {
            let sub = p_dir.join(folder);
            if sub.exists() {
                let _ = fs::remove_dir_all(&sub);
            }
        }
        for file in &["dwmapi.dll", "mods.txt.snapshot", "PalModSettings.ini"] {
            let f = p_dir.join(file);
            if f.exists() {
                let _ = fs::remove_file(&f);
            }
        }
        
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }

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

        let force_palschema = data.settings.force_load_order.unwrap_or(false) && profile.force_load_order_palschema
            .or(data.settings.force_load_order_palschema)
            .unwrap_or(false);
        restore_profile_files_to_game(&game_path, &p_dir, profile, &program_path, force_palschema);
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
