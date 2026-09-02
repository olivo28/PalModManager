use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{DependencyMode, Profile};
use super::super::utils::copy_dir_all;
use super::super::isolation::sync_profile_dependencies;

pub fn game_path_to_workshop_dir(game_path: &str) -> PathBuf {
    Path::new(game_path).join("Mods").join("NativeMods").join("UE4SS")
}

pub fn backup_game_files_to_profile(game_path: &str, profile_dir: &Path, profile: &Profile) {
    if game_path.is_empty() { return; }
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let dwmapi_game = win64.join("dwmapi.dll");

    let active_mode = match profile.dependency_mode {
        DependencyMode::None => {
            if dwmapi_game.exists() || win64.join("ue4ss").exists() {
                DependencyMode::Standard
            } else if game_path_to_workshop_dir(game_path).exists() {
                DependencyMode::Workshop
            } else {
                DependencyMode::None
            }
        }
        ref other => other.clone(),
    };

    match active_mode {
        DependencyMode::Workshop => {
            let ws_folder = game_path_to_workshop_dir(game_path);

            // Backup user UE4SS mods from workshop
            let ws_mods_dir = ws_folder.join("Mods");
            let ws_mods_backup = profile_dir.join("ue4ss_workshop_mods");
            if ws_mods_backup.exists() {
                let _ = fs::remove_dir_all(&ws_mods_backup);
            }
            if ws_mods_dir.exists() {
                let _ = fs::create_dir_all(&ws_mods_backup);
                if let Ok(entries) = fs::read_dir(&ws_mods_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let lower = name.to_lowercase();
                        if lower == "palschema" || lower == "shared" || lower == "bpmodloadermod" || lower == "linetracemod" || lower == "mods.txt" || lower == "ue4ss_signatures" {
                            continue;
                        }
                        let dst = ws_mods_backup.join(&name);
                        if entry.path().is_dir() {
                            let _ = copy_dir_all(&entry.path(), &dst);
                        } else {
                            let _ = fs::copy(&entry.path(), &dst);
                        }
                    }
                }
                let mods_txt = ws_mods_dir.join("mods.txt");
                if mods_txt.exists() {
                    let _ = fs::copy(&mods_txt, ws_mods_backup.join("mods.txt"));
                }
            }

            // Backup PalSchema from workshop
            let palschema_game = ws_mods_dir.join("PalSchema").join("mods");
            let palschema_storage = ws_mods_dir.join("PalSchema").join("Storage");
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

            let std_ue4ss_mods_dir = if win64.join("ue4ss").join("Mods").exists() {
                win64.join("ue4ss").join("Mods")
            } else {
                win64.join("Mods")
            };

            let ue4ss_mods_backup = profile_dir.join("ue4ss_mods");
            if ue4ss_mods_backup.exists() {
                let _ = fs::remove_dir_all(&ue4ss_mods_backup);
            }
            if std_ue4ss_mods_dir.exists() {
                let _ = fs::create_dir_all(&ue4ss_mods_backup);
                if let Ok(entries) = fs::read_dir(&std_ue4ss_mods_dir) {
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
                let mods_txt = std_ue4ss_mods_dir.join("mods.txt");
                if mods_txt.exists() {
                    let _ = fs::copy(&mods_txt, ue4ss_mods_backup.join("mods.txt"));
                }
            }

            // Backup PalSchema from standard
            let palschema_game = std_ue4ss_mods_dir.join("PalSchema").join("mods");
            let palschema_storage = std_ue4ss_mods_dir.join("PalSchema").join("Storage");
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
        }
        DependencyMode::None => {}
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

pub fn restore_profile_files_to_game(
    game_path: &str,
    profile_dir: &Path,
    target_profile: &Profile,
    program_path: &str,
    force_load_order_palschema: bool,
) {
    if game_path.is_empty() { return; }
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let dwmapi_game = win64.join("dwmapi.dll");
    let ue4ss_std_dir = win64.join("ue4ss");
    let win64_mods_dir = win64.join("Mods");
    let ws_folder = game_path_to_workshop_dir(game_path);
    let paks_game = PathBuf::from(game_path).join("Pal").join("Content").join("Paks").join("~mods");
    let logic_game = PathBuf::from(game_path).join("Pal").join("Content").join("Paks").join("LogicMods");
    let mods_root = Path::new(game_path).join("Mods");
    let settings_ini = mods_root.join("PalModSettings.ini");
    let managed_mods = mods_root.join("ManagedMods");

    // 1. CLEANUP ALL EXISTING FILES ACROSS BOTH STANDARD AND WORKSHOP LOCATIONS:
    if dwmapi_game.exists() { let _ = fs::remove_file(&dwmapi_game); }
    if ue4ss_std_dir.exists() { let _ = fs::remove_dir_all(&ue4ss_std_dir); }
    if win64_mods_dir.exists() { let _ = fs::remove_dir_all(&win64_mods_dir); }
    if ws_folder.exists() { let _ = fs::remove_dir_all(&ws_folder); }
    
    let native_mods_root = Path::new(game_path).join("Mods").join("NativeMods");
    if native_mods_root.exists() {
        let _ = fs::remove_dir_all(&native_mods_root);
    }

    if managed_mods.exists() { let _ = fs::remove_dir_all(&managed_mods); }
    if settings_ini.exists() { let _ = fs::remove_file(&settings_ini); }
    if paks_game.exists() { let _ = fs::remove_dir_all(&paks_game); }
    if logic_game.exists() { let _ = fs::remove_dir_all(&logic_game); }

    // 2. RESTORE DEPENDENCIES AND MODS FOR THE TARGET PROFILE:
    let target_mode = match target_profile.dependency_mode {
        DependencyMode::None => {
            if target_profile.ue4ss_enabled {
                DependencyMode::Standard
            } else if profile_dir.join("ue4ss").exists() && fs::read_dir(profile_dir.join("ue4ss")).map(|mut d| d.next().is_some()).unwrap_or(false) {
                DependencyMode::Standard
            } else if profile_dir.join("ue4ss_workshop_root").exists() {
                DependencyMode::Workshop
            } else {
                DependencyMode::None
            }
        }
        ref other => other.clone(),
    };

    match target_mode {
        DependencyMode::Workshop => {
            let settings_backup = profile_dir.join("PalModSettings.ini");
            if settings_backup.exists() {
                let _ = fs::copy(&settings_backup, &settings_ini);
            } else {
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

            let target_ws_mods_dir = ws_folder.join("Mods");
            let _ = fs::create_dir_all(&target_ws_mods_dir);

            // Restore user workshop mods
            let workshop_backup = profile_dir.join("ue4ss_workshop_mods");
            if workshop_backup.exists() {
                if let Ok(entries) = fs::read_dir(&workshop_backup) {
                    for entry in entries.flatten() {
                        let src = entry.path();
                        let dst = target_ws_mods_dir.join(src.file_name().unwrap());
                        if src.is_dir() {
                            let _ = copy_dir_all(&src, &dst);
                        } else {
                            let _ = fs::copy(&src, &dst);
                        }
                    }
                }
            }

            // Restore PalSchema into Workshop
            if target_profile.palschema_enabled {
                let palschema_backup = profile_dir.join("palschema");
                let palschema_dest = target_ws_mods_dir.join("PalSchema").join("mods");
                if palschema_backup.exists() {
                    let _ = fs::create_dir_all(&palschema_dest);
                    if let Ok(entries) = fs::read_dir(&palschema_backup) {
                        for entry in entries.flatten() {
                            let src = entry.path();
                            let dst = palschema_dest.join(src.file_name().unwrap());
                            if src.is_dir() {
                                let _ = copy_dir_all(&src, &dst);
                            } else {
                                let _ = fs::copy(&src, &dst);
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
            if ue4ss_backup.exists() && fs::read_dir(&ue4ss_backup).map(|mut d| d.next().is_some()).unwrap_or(false) {
                let _ = copy_dir_all(&ue4ss_backup, &ue4ss_std_dir);
                if dwmapi_backup.exists() {
                    let _ = fs::copy(&dwmapi_backup, &dwmapi_game);
                }
            } else {
                let _ = sync_profile_dependencies(game_path, program_path, target_profile);
            }

            let target_std_mods_dir = ue4ss_std_dir.join("Mods");
            let _ = fs::create_dir_all(&target_std_mods_dir);

            // Restore user standard UE4SS mods
            let ue4ss_mods_backup = profile_dir.join("ue4ss_mods");
            if ue4ss_mods_backup.exists() {
                if let Ok(entries) = fs::read_dir(&ue4ss_mods_backup) {
                    for entry in entries.flatten() {
                        let src = entry.path();
                        let dst = target_std_mods_dir.join(src.file_name().unwrap());
                        if src.is_dir() {
                            let _ = copy_dir_all(&src, &dst);
                        } else {
                            let _ = fs::copy(&src, &dst);
                        }
                    }
                }
            }

            // Restore PalSchema into Standard
            if target_profile.palschema_enabled {
                let palschema_backup = profile_dir.join("palschema");
                let palschema_dest = target_std_mods_dir.join("PalSchema").join("mods");
                if palschema_backup.exists() {
                    let _ = fs::create_dir_all(&palschema_dest);
                    if let Ok(entries) = fs::read_dir(&palschema_backup) {
                        for entry in entries.flatten() {
                            let src = entry.path();
                            let dst = palschema_dest.join(src.file_name().unwrap());
                            if src.is_dir() {
                                let _ = copy_dir_all(&src, &dst);
                            } else {
                                let _ = fs::copy(&src, &dst);
                            }
                        }
                    }
                }
            }
        }
        DependencyMode::None => {}
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
        let palschema_mods = match target_profile.dependency_mode {
            DependencyMode::Workshop => ws_folder.join("Mods").join("PalSchema").join("mods"),
            _ => ue4ss_std_dir.join("Mods").join("PalSchema").join("mods"),
        };
        let palschema_storage = match target_profile.dependency_mode {
            DependencyMode::Workshop => ws_folder.join("Mods").join("PalSchema").join("Storage"),
            _ => ue4ss_std_dir.join("Mods").join("PalSchema").join("Storage"),
        };

        if palschema_mods.exists() {
            if let Ok(entries) = fs::read_dir(&palschema_mods) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let _ = crate::profiles::remove_junction_or_symlink(&path);
                }
            }
        }

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
