use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{AppData, Profile, DependencyMode, ModInfo};
use super::utils::{get_profile_dir, ensure_profile_structure, copy_dir_all, sanitize_profile_id};
use super::isolation::sync_profile_dependencies;
use super::actions::{enable_mod_internal, disable_mod_internal};
use super::core::{
    cleanup_profile_enabled_ids, sync_current_profile_states
};

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

    let p_dir = get_profile_dir(&program_path, profile_id);
    if let Some(profile) = data.profiles.iter().find(|p| p.id == profile_id) {
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }

    if profile_id == data.current_profile_id {
        let mod_info = data.mods.iter().find(|m| m.id == mod_id).cloned();
        let is_workshop = mod_info.as_ref().map(|m| {
            m.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod"))
        }).unwrap_or(false);

        if is_workshop {
            let game_path = data.settings.game_path.clone();
            let force_load_order_ue4ss = data.settings.force_load_order.unwrap_or(false) && data.settings.force_load_order_ue4ss.unwrap_or(false);
            let wmods = crate::workshop::scan_workshop_mods(&game_path);
            if let Some(target) = wmods.iter().find(|m| m.package_name == mod_id || m.package_name.to_lowercase() == mod_id.to_lowercase()) {
                crate::logger::log(&format!("set_profile_mod_state: Applying workshop change for mod = {} -> enabled = {}", mod_id, enabled));
                if enabled {
                    let _ = crate::workshop::activate_workshop_mod(&game_path, target, force_load_order_ue4ss);
                } else {
                    let _ = crate::workshop::deactivate_workshop_mod(&game_path, target, force_load_order_ue4ss);
                }
            }
        } else {
            crate::logger::log(&format!("set_profile_mod_state: Applying switch physically for mod = {} -> enabled = {}", mod_id, enabled));
            if enabled {
                enable_mod_internal(data, &program_path, mod_id)?;
            } else {
                disable_mod_internal(data, &program_path, mod_id)?;
            }
        }
    }

    Ok(())
}

fn backup_game_files_to_profile(game_path: &str, profile_dir: &Path, profile: &Profile) {
    if game_path.is_empty() { return; }
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let ue4ss_mods_dir = crate::dependency_checker::get_ue4ss_mods_dir(Path::new(game_path));
    let dwmapi_game = win64.join("dwmapi.dll");

    match profile.dependency_mode {
        DependencyMode::Workshop => {
            let workshop_root = game_path_to_workshop_dir(game_path);
            let root_backup = profile_dir.join("ue4ss_workshop_root");
            if root_backup.exists() {
                let _ = fs::remove_dir_all(&root_backup);
            }
            if workshop_root.exists() {
                let _ = copy_dir_all(&workshop_root, &root_backup);
            }
        }
        DependencyMode::Standard => {
            let ue4ss_game = win64.join("ue4ss");
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
        }
        DependencyMode::None => {}
    }

    // Backup user UE4SS mods from ue4ss_mods_dir
    let ue4ss_mods_backup = profile_dir.join("ue4ss_mods");
    if ue4ss_mods_backup.exists() {
        let _ = fs::remove_dir_all(&ue4ss_mods_backup);
    }
    if ue4ss_mods_dir.exists() {
        let _ = fs::create_dir_all(&ue4ss_mods_backup);
        if let Ok(entries) = fs::read_dir(&ue4ss_mods_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let lower = name.to_lowercase();
                if lower == "palschema" || lower == "shared" || lower == "bpmodloadermod" || lower == "linetracemod" || lower == "mods.txt" || lower == "ue4ss_signatures" {
                    continue;
                }
                let dst = ue4ss_mods_backup.join(&name);
                if entry.path().is_dir() {
                    let _ = copy_dir_all(&entry.path(), &dst);
                } else {
                    let _ = fs::copy(&entry.path(), &dst);
                }
            }
        }
    }

    // Backup PalSchema mods (from mods/ and Storage/)
    let palschema_game = ue4ss_mods_dir.join("PalSchema").join("mods");
    let palschema_storage = ue4ss_mods_dir.join("PalSchema").join("Storage");
    let palschema_backup = profile_dir.join("palschema");
    if palschema_backup.exists() {
        let _ = fs::remove_dir_all(&palschema_backup);
    }
    if palschema_game.exists() || palschema_storage.exists() {
        let _ = fs::create_dir_all(&palschema_backup);
        if palschema_storage.exists() {
            if let Ok(entries) = fs::read_dir(&palschema_storage) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let dst = palschema_backup.join(&name);
                    if entry.path().is_dir() {
                        let _ = copy_dir_all(&entry.path(), &dst);
                    }
                }
            }
        }
        if palschema_game.exists() {
            if let Ok(entries) = fs::read_dir(&palschema_game) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name();
                    let dst = palschema_backup.join(&name);
                    if !dst.exists() && path.is_dir() {
                        let _ = copy_dir_all(&path, &dst);
                    }
                }
            }
        }
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

    let mods_root = Path::new(game_path).join("Mods");
    let settings_ini = mods_root.join("PalModSettings.ini");
    let managed_mods = mods_root.join("ManagedMods");

    let settings_backup = profile_dir.join("PalModSettings.ini");
    if settings_backup.exists() {
        let _ = fs::remove_file(&settings_backup);
    }
    if settings_ini.exists() {
        let _ = fs::copy(&settings_ini, &settings_backup);
    }

    let managed_backup = profile_dir.join("ManagedMods");
    if managed_backup.exists() {
        let _ = fs::remove_dir_all(&managed_backup);
    }
    if managed_mods.exists() {
        let _ = copy_dir_all(&managed_mods, &managed_backup);
    }
}

fn game_path_to_workshop_dir(game_path: &str) -> PathBuf {
    Path::new(game_path).join("Mods").join("NativeMods").join("UE4SS")
}

fn restore_profile_files_to_game(
    game_path: &str,
    profile_dir: &Path,
    target_profile: &Profile,
    program_path: &str,
    force_load_order_palschema: bool,
) {
    if game_path.is_empty() { return; }
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let ue4ss_mods_dir = crate::dependency_checker::get_ue4ss_mods_dir(Path::new(game_path));
    let ue4ss_game = if win64.join("dwmapi.dll").exists() { win64.join("ue4ss") } else { game_path_to_workshop_dir(game_path) };
    let dwmapi_game = win64.join("dwmapi.dll");
    let palschema_game = ue4ss_mods_dir.join("PalSchema").join("mods");
    let palschema_storage = ue4ss_mods_dir.join("PalSchema").join("Storage");
    let paks_game = PathBuf::from(game_path).join("Pal").join("Content").join("Paks").join("~mods");
    let logic_game = PathBuf::from(game_path).join("Pal").join("Content").join("Paks").join("LogicMods");

    if dwmapi_game.exists() { let _ = fs::remove_file(&dwmapi_game); }

    // Safely clean PalSchema mods folder and Storage
    if palschema_game.exists() {
        if let Ok(entries) = fs::read_dir(&palschema_game) {
            for entry in entries.flatten() {
                let path = entry.path();
                let _ = crate::profiles::remove_junction_or_symlink(&path);
                if path.exists() {
                    if path.is_dir() {
                        let _ = fs::remove_dir_all(&path);
                    } else {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }
    if palschema_storage.exists() {
        let _ = fs::remove_dir_all(&palschema_storage);
    }

    // Clean up user UE4SS mods from ue4ss_mods_dir
    if ue4ss_mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(&ue4ss_mods_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let lower = name.to_lowercase();
                if lower == "palschema" || lower == "shared" || lower == "bpmodloadermod" || lower == "linetracemod" || lower == "mods.txt" || lower == "ue4ss_signatures" {
                    continue;
                }
                let path = entry.path();
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    let is_workshop = !win64.join("dwmapi.dll").exists() && game_path_to_workshop_dir(game_path).exists();

    let mods_root = Path::new(game_path).join("Mods");
    let settings_ini = mods_root.join("PalModSettings.ini");
    let managed_mods = mods_root.join("ManagedMods");
    if mods_root.exists() {
        if let Ok(entries) = fs::read_dir(&mods_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    let settings_backup = profile_dir.join("PalModSettings.ini");
    if settings_backup.exists() {
        let _ = fs::copy(&settings_backup, &settings_ini);
    } else if is_workshop {
        let default_workshop_root = Path::new(game_path)
            .parent().and_then(|p| p.parent()).and_then(|p| p.parent())
            .map(|p| p.join("workshop").join("content").join("1623730"))
            .unwrap_or_else(|| PathBuf::from(""));
            
        let ini_content = format!(
            "[PalModSettings]\r\nbGlobalEnableMod={}\r\nWorkshopRootDir={}\r\nConfigVersion=1.0\r\nbNeedShowErrorOnNextStart=False\r\n",
            if target_profile.ue4ss_enabled { "True" } else { "False" },
            default_workshop_root.to_string_lossy()
        );
        let _ = fs::create_dir_all(&mods_root);
        let _ = fs::write(&settings_ini, ini_content);
    }

    let managed_backup = profile_dir.join("ManagedMods");
    if managed_backup.exists() {
        let _ = copy_dir_all(&managed_backup, &managed_mods);
    }

    if ue4ss_game.exists() {
        let _ = fs::remove_dir_all(&ue4ss_game);
    }
    let ws_folder = game_path_to_workshop_dir(game_path);
    if ws_folder.exists() {
        let _ = fs::remove_dir_all(&ws_folder);
    }
    
    let native_mods_root = Path::new(game_path).join("Mods").join("NativeMods");
    if native_mods_root.exists() {
        if fs::read_dir(&native_mods_root).map(|mut d| d.next().is_none()).unwrap_or(false) {
            let _ = fs::remove_dir(&native_mods_root);
        }
    }
    if dwmapi_game.exists() {
        let _ = fs::remove_file(&dwmapi_game);
    }

    if paks_game.exists() { let _ = fs::remove_dir_all(&paks_game); }
    if logic_game.exists() { let _ = fs::remove_dir_all(&logic_game); }

    match target_profile.dependency_mode {
        DependencyMode::Workshop => {
            let root_backup = profile_dir.join("ue4ss_workshop_root");
            if root_backup.exists() {
                let _ = copy_dir_all(&root_backup, &ws_folder);
            } else {
                let mods_folder = ws_folder.join("Mods");
                let _ = fs::create_dir_all(&mods_folder);

                let workshop_backup = profile_dir.join("ue4ss_workshop_mods");
                if workshop_backup.exists() {
                    if let Ok(entries) = fs::read_dir(&workshop_backup) {
                        for entry in entries.flatten() {
                            let src = entry.path();
                            let dst = mods_folder.join(src.file_name().unwrap());
                            if src.is_dir() {
                                let _ = copy_dir_all(&src, &dst);
                            }
                        }
                    }
                }
            }

            let db = crate::db::load_db(program_path);
            let force_load_order_ue4ss = db.settings.force_load_order.unwrap_or(false) && db.settings.force_load_order_ue4ss.unwrap_or(false);
            let restored_settings = crate::workshop::read_pal_mod_settings(game_path);
            let wmods = crate::workshop::scan_workshop_mods(game_path);
            for package_name in &restored_settings.active_mod_list {
                if let Some(wmod) = wmods.iter().find(|m| &m.package_name == package_name) {
                    if !wmod.is_installed {
                        let _ = crate::workshop::activate_workshop_mod(game_path, wmod, force_load_order_ue4ss);
                    }
                }
            }
        }
        DependencyMode::Standard => {
            let ue4ss_backup = profile_dir.join("ue4ss");
            let dwmapi_backup = profile_dir.join("dwmapi.dll");
            let standard_ue4ss_game = win64.join("ue4ss");
            if ue4ss_backup.exists() && fs::read_dir(&ue4ss_backup).map(|mut d| d.next().is_some()).unwrap_or(false) {
                let _ = copy_dir_all(&ue4ss_backup, &standard_ue4ss_game);
                if dwmapi_backup.exists() {
                    let _ = fs::copy(&dwmapi_backup, &dwmapi_game);
                }
            } else {
                let _ = sync_profile_dependencies(game_path, program_path, target_profile);
            }
        }
        DependencyMode::None => {}
    }

    // Restore user UE4SS mods
    let ue4ss_mods_backup = profile_dir.join("ue4ss_mods");
    if ue4ss_mods_backup.exists() {
        let _ = fs::create_dir_all(&ue4ss_mods_dir);
        if let Ok(entries) = fs::read_dir(&ue4ss_mods_backup) {
            for entry in entries.flatten() {
                let src = entry.path();
                let dst = ue4ss_mods_dir.join(src.file_name().unwrap());
                if src.is_dir() {
                    let _ = copy_dir_all(&src, &dst);
                } else {
                    let _ = fs::copy(&src, &dst);
                }
            }
        }
    }

    // Restore PalSchema mods
    let palschema_backup = profile_dir.join("palschema");
    if palschema_backup.exists() {
        let _ = fs::create_dir_all(&palschema_game);
        if let Ok(entries) = fs::read_dir(&palschema_backup) {
            for entry in entries.flatten() {
                let src = entry.path();
                let dst = palschema_game.join(src.file_name().unwrap());
                if src.is_dir() {
                    let _ = copy_dir_all(&src, &dst);
                } else {
                    let _ = fs::copy(&src, &dst);
                }
            }
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

    // If PalSchema FLO is disabled, clean up any junctions restored from the backup and move folders out of Storage
    if !force_load_order_palschema {
        let palschema_mods = ue4ss_mods_dir.join("PalSchema").join("mods");
        let palschema_storage = ue4ss_mods_dir.join("PalSchema").join("Storage");

        if palschema_mods.exists() {
            // 1. Remove any junctions/symlinks
            if let Ok(entries) = fs::read_dir(&palschema_mods) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let _ = crate::profiles::remove_junction_or_symlink(&path);
                }
            }
        }

        // 2. Move actual directories from Storage back to mods/
        if palschema_storage.exists() {
            if let Ok(entries) = fs::read_dir(&palschema_storage) {
                for entry in entries.flatten() {
                    let src = entry.path();
                    if src.is_dir() {
                        let name = src.file_name().unwrap();
                        let dst = palschema_mods.join(name);
                        let _ = crate::profiles::move_path(&src, &dst);
                    }
                }
            }
            let _ = fs::remove_dir(&palschema_storage);
        }
    }
}

pub fn switch_profile(
    data: &mut AppData,
    program_path: &str,
    target_profile: &Profile,
) -> Result<Vec<ModInfo>, String> {
    crate::logger::log(&format!("switch_profile: Switching to profile '{}' (folder: {})", target_profile.name, target_profile.id));

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
        }
    }

    data.current_profile_id = target_profile.id.clone();
    cleanup_profile_enabled_ids(data);
    sync_current_profile_states(data);

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
        dependency_mode: DependencyMode::None,
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
        dependency_mode: source_profile.dependency_mode.clone(),
        mod_folders: source_profile.mod_folders.clone(),
        load_order_metadata: source_profile.load_order_metadata.clone(),
        force_load_order_ue4ss: source_profile.force_load_order_ue4ss,
        force_load_order_palschema: source_profile.force_load_order_palschema,
        hide_native_mods: source_profile.hide_native_mods,
    };

    let program_path = data.settings.program_path.clone();
    let src_dir = get_profile_dir(&program_path, source_profile_id);
    let dst_dir = ensure_profile_structure(&program_path, &new_profile.id);

    if source_profile_id == data.current_profile_id {
        backup_game_files_to_profile(&data.settings.game_path, &src_dir, &source_profile);
    }

    if src_dir.exists() {
        for folder in &["ue4ss", "ue4ss_workshop_root", "ue4ss_workshop_mods", "palschema", "paks", "logicmods"] {
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

    profile.installed_mod_ids.clear();
    profile.enabled_mod_ids.clear();
    profile.mod_folders.clear();

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
