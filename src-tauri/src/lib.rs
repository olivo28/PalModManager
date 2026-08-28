mod commands;
mod profiles;
pub mod db;
mod dependency_checker;
mod installer;
mod library;
mod models;
pub mod nexus;
pub mod nexus_oauth;
pub mod protocol_handler;
mod state;
mod zip_handler;
mod logger;
pub mod config_merge;
mod workshop;
mod watcher;
pub mod safety_backup;
pub mod image_proxy;
pub mod altermatic;
pub mod pak_scanner;
pub mod retoc_runner;
pub mod save_scanner;

use commands::mod_commands;
use commands::settings_commands;
use commands::install_commands;
use commands::config_commands;
use commands::nexus_commands;
use commands::launch_commands;
use commands::library_commands;
use commands::profile_commands;
use commands::dependency_commands;
use commands::packer_commands;
use commands::scanner_commands;
use commands::db_commands;
use commands::load_order_commands;
use commands::workshop_commands;
use commands::discovery_commands;
use commands::altermatic_commands;
use state::AppState;

use tauri::{Manager, Emitter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init_logger();
    logger::log("=== APPLICATION STARTED (cargo run / .exe) ===");
    logger::log(&format!("PMM-Core Engine: Initializing desktop runtime v{}", env!("CARGO_PKG_VERSION")));

    #[cfg(target_os = "windows")]
    let program_path = std::env::var("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("PalModManager");

    #[cfg(not(target_os = "windows"))]
    let program_path = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local").join("share"))
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("PalModManager");
    
    let default_program_path = program_path.clone();
    let mut active_program_path = default_program_path.clone();

    logger::log(&format!("Loading default database from {}", default_program_path.display()));
    let start_db = std::time::Instant::now();
    let mut data = db::load_db(&default_program_path.to_string_lossy());
    
    // Auto-create initial snapshot if game path is configured
    if !data.settings.game_path.is_empty() {
        let _ = safety_backup::create_initial_safety_backup(&data.settings.game_path, &default_program_path.to_string_lossy(), false);
    }

    logger::log(&format!("Default database loaded successfully in {:?}", start_db.elapsed()));

    if let Some(ref custom_path) = data.settings.custom_data_path {
        if !custom_path.is_empty() {
            let target_dir = if custom_path == "__portable__" {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
                    .unwrap_or_else(|| default_program_path.clone())
            } else {
                std::path::PathBuf::from(custom_path)
            };
            logger::log(&format!("Redirecting database location to custom path: {}", target_dir.display()));
            active_program_path = target_dir;
            let custom_data = db::load_db(&active_program_path.to_string_lossy());
            data = custom_data;
        }
    }

    if data.settings.program_path != active_program_path.to_string_lossy().to_string() {
        data.settings.program_path = active_program_path.to_string_lossy().to_string();
        let _ = db::save_db(&active_program_path.to_string_lossy(), &data);
    }

    let is_debug = data.settings.debug_console.unwrap_or(false);
    logger::set_console_visibility(is_debug);

    let state = AppState {
        data: std::sync::Mutex::new(data),
    };

    logger::log("Initializing Tauri builder...");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            crate::logger::log(&format!("single_instance: Captured args: {:?}", args));
            for arg in args {
                if arg.starts_with("palmodmanager://") {
                    crate::logger::log(&format!("single_instance: Captured deep link: {}", arg));
                    let _ = app.emit("nexus-oauth-deep-link", arg.clone());
                } else if arg.starts_with("nxm://") {
                    crate::logger::log(&format!("single_instance: Captured NXM URL: {}", arg));
                    let _ = app.emit("nexus-nxm-download", arg.clone());
                }
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            settings_commands::get_settings,
            settings_commands::set_game_path,
            settings_commands::set_hide_native_mods,
            settings_commands::set_debug_console,
            settings_commands::set_force_load_order,
            settings_commands::set_force_load_order_ue4ss,
            settings_commands::set_force_load_order_palschema,
            settings_commands::set_custom_data_path,
            settings_commands::set_toolbar_scale,
            settings_commands::set_language,
            settings_commands::set_dns_resolver,
            settings_commands::set_cache_remote_images,
            settings_commands::set_folder_expand_mode,
            image_proxy::fetch_and_cache_image,
            image_proxy::get_image_cache_size,
            image_proxy::purge_image_cache,

            mod_commands::get_mods,
            mod_commands::scan_mods,
            mod_commands::remove_mod,
            mod_commands::disable_mod,
            mod_commands::enable_mod,
            mod_commands::disable_all_mods,
            mod_commands::enable_all_mods,
            mod_commands::open_folder,
            mod_commands::open_extra_folder,
            mod_commands::open_folder_by_type,
            mod_commands::open_path,
            mod_commands::rename_mod,
            mod_commands::set_mod_version,
            mod_commands::set_mod_ignored_keys,
            mod_commands::check_github_version,
            mod_commands::set_github_version,
            mod_commands::export_mods_json,
            mod_commands::get_game_version,
            install_commands::analyze_zip,
            install_commands::install_mod_command,
            install_commands::check_mod_exists_command,
            install_commands::update_mod_command,
            install_commands::build_install_manifest,
            install_commands::install_mod_with_manifest,
            install_commands::preview_config_diff,
            config_commands::read_config,
            config_commands::save_config,
            config_commands::set_mod_config,
            config_commands::list_mod_files,
            config_commands::read_mod_file,
            config_commands::save_mod_file,
            nexus_commands::fetch_nexus_info_async,
            nexus_commands::refresh_nexus_cache,
            nexus_commands::set_nexus_mod_id,
            nexus_commands::check_for_updates,
            nexus_commands::ignore_mod_version,
            nexus_commands::start_nexus_oauth,
            nexus_commands::handle_nexus_oauth_callback,
            nexus_commands::get_nexus_account_status,
            nexus_commands::refresh_nexus_account_profile,
            nexus_commands::logout_nexus_account,
            nexus_commands::check_nexus_protocol_status,
            nexus_commands::register_nexus_protocol,
            nexus_commands::unregister_nexus_protocol,
            nexus_commands::get_nexus_user_endorsements,
            nexus_commands::get_nexus_user_tracked_mods,
            nexus_commands::get_nexus_user_authored_mods,
            nexus_commands::parse_nxm_link,
            nexus_commands::get_nxm_mod_metadata,
            nexus_commands::download_nxm_file,
            nexus_commands::handle_nxm_download,
            settings_commands::open_url,
            library_commands::get_library,
            library_commands::install_mod_from_library,
            library_commands::remove_from_library,
            library_commands::get_library_zip_path,
            library_commands::copy_to_library_command,
            library_commands::check_library_updates,
            profile_commands::get_profiles,
            profile_commands::get_current_profile,
            profile_commands::switch_profile_command,
            profile_commands::create_profile_command,
            profile_commands::clone_profile_command,
            profile_commands::delete_profile_command,
            profile_commands::rename_profile_command,
            profile_commands::clear_profile_command,
            profile_commands::set_mod_profile_state,
            profile_commands::create_mod_folder_command,
            profile_commands::delete_mod_folder_command,
            profile_commands::rename_mod_folder_command,
            profile_commands::add_mod_to_folder_command,
            profile_commands::toggle_folder_mods_command,
            profile_commands::reorder_mod_folders_command,
            dependency_commands::check_dependencies,
            dependency_commands::clean_conflict_dlls,
            dependency_commands::reset_workshop_cache,
            dependency_commands::get_safety_backup_info_command,
            dependency_commands::trigger_safety_backup_command,
            dependency_commands::restore_safety_backup_command,
            dependency_commands::check_ue4ss_latest,
            dependency_commands::check_palschema_latest,
            dependency_commands::check_dependencies_full,
            dependency_commands::install_ue4ss,
            dependency_commands::install_palschema,
            dependency_commands::uninstall_ue4ss,
            dependency_commands::uninstall_palschema,
            dependency_commands::get_storage_usage_command,
            dependency_commands::clear_temp_downloads_command,
            dependency_commands::open_temp_folder_command,
            dependency_commands::open_library_folder_command,
            settings_commands::log_from_js,
            mod_commands::create_backup,
            mod_commands::change_pak_destination,

            mod_commands::restore_backup,
            mod_commands::analyze_backup,
            packer_commands::scan_paths_for_packing,
            packer_commands::pack_mod,
            packer_commands::save_packer_project,
            packer_commands::load_packer_projects,
            packer_commands::delete_packer_project,
            scanner_commands::scan_conflicts,
            scanner_commands::scan_mod_hotkeys,
            scanner_commands::update_mod_hotkey,
            scanner_commands::inspect_pak_asset,
            scanner_commands::inspect_uasset_deep_cmd,
            scanner_commands::inspect_pak_file_tree,
            scanner_commands::inspect_mod_pak_contents,
            scanner_commands::convert_mod_to_gamepass,
            scanner_commands::convert_all_gamepass_mods,
            scanner_commands::list_save_worlds_cmd,
            scanner_commands::deep_scan_save_cmd,
            scanner_commands::repair_save_cmd,
            scanner_commands::restore_save_backup_cmd,
            scanner_commands::create_world_backup_cmd,
            scanner_commands::open_world_folder_cmd,
            scanner_commands::export_world_zip_cmd,
            scanner_commands::prune_world_backups_cmd,
            scanner_commands::save_world_custom_meta_cmd,
            scanner_commands::get_world_custom_meta_cmd,
            scanner_commands::inspect_snapshot_details_cmd,
            db_commands::db_get_all,
            db_commands::db_write_record,
            load_order_commands::get_ue4ss_load_order,
            load_order_commands::save_ue4ss_load_order,
            load_order_commands::get_palschema_load_order,
            load_order_commands::save_palschema_load_order,
            workshop_commands::get_workshop_mods,
            workshop_commands::get_workshop_state,
            workshop_commands::activate_workshop_mod_cmd,
            workshop_commands::deactivate_workshop_mod_cmd,
            workshop_commands::set_workshop_global_enabled,
            workshop_commands::prepare_workshop_update_zip,
            workshop_commands::check_workshop_updates_online_cmd,
            workshop_commands::trigger_steam_validation_cmd,
            launch_commands::launch_game,
            discovery_commands::get_discovery_categories,
            discovery_commands::get_discovery_mods,
            discovery_commands::get_discovery_mod_details,
            discovery_commands::endorse_nexus_mod,
            discovery_commands::abstain_nexus_mod,
            discovery_commands::track_nexus_mod,
            discovery_commands::untrack_nexus_mod,
            discovery_commands::install_discovery_file,
            altermatic_commands::sync_altermatic_load_list,
            altermatic_commands::get_altermatic_dep_status,
        ])
        .setup(move |app| {
            let state = app.state::<AppState>();
            let settings = {
                let data = state.data.lock().unwrap();
                data.settings.clone()
            };
            if let Some(window) = app.get_webview_window("main") {
                if let (Some(w), Some(h)) = (settings.window_width, settings.window_height) {
                    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)));
                }
                if let Some(true) = settings.window_maximized {
                    let _ = window.maximize();
                }
                let _ = window.show();
            }

            // Check if launched directly with palmodmanager:// or nxm:// deep link
            for arg in std::env::args() {
                if arg.starts_with("palmodmanager://") {
                    crate::logger::log(&format!("setup: Application launched directly with deep link: {}", arg));
                    let app_handle = app.handle().clone();
                    let url_clone = arg.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(1200));
                        let _ = app_handle.emit("nexus-oauth-deep-link", url_clone);
                    });
                } else if arg.starts_with("nxm://") {
                    crate::logger::log(&format!("setup: Application launched directly with NXM link: {}", arg));
                    let app_handle = app.handle().clone();
                    let url_clone = arg.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(1200));
                        let _ = app_handle.emit("nexus-nxm-download", url_clone);
                    });
                }
            }

            // Spawn background thread to watch the Steam Workshop mods folder for changes
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_mods_hash = String::new();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let state = handle.state::<AppState>();
                    let game_path = {
                        match state.data.lock() {
                            Ok(data) => data.settings.game_path.clone(),
                            Err(_) => continue,
                        }
                    };
                    if game_path.is_empty() {
                        continue;
                    }

                    let settings = crate::workshop::read_pal_mod_settings(&game_path);
                    if settings.workshop_root.is_empty() {
                        continue;
                    }

                    let path = std::path::Path::new(&settings.workshop_root);
                    if !path.exists() {
                        continue;
                    }

                    let mut current_hash = String::new();
                    if let Ok(entries) = std::fs::read_dir(path) {
                        let mut entries_vec = Vec::new();
                        for entry in entries.flatten() {
                            if let Ok(meta) = entry.metadata() {
                                if let Ok(mtime) = meta.modified() {
                                    entries_vec.push(format!("{}:{:?}", entry.file_name().to_string_lossy(), mtime));
                                }
                            }
                        }
                        entries_vec.sort();
                        current_hash = entries_vec.join("|");
                    }

                    if !current_hash.is_empty() && current_hash != last_mods_hash {
                        if !last_mods_hash.is_empty() {
                            let _ = handle.emit("workshop-directory-changed", ());
                        }
                        last_mods_hash = current_hash;
                    }
                }
            });

            // Start reactive filesystem watcher on all active mod directories
            let (game_dir, prog_path) = {
                let state = app.state::<AppState>();
                let lock_res = state.data.lock();
                lock_res.map(|d| (d.settings.game_path.clone(), d.settings.program_path.clone())).unwrap_or_default()
            };

            let mut watch_paths = Vec::new();
            if !game_dir.is_empty() {
                let gp = std::path::Path::new(&game_dir);
                let game_profile = crate::dependency_checker::build_game_profile(gp);

                // Watch standard / detected UE4SS mods directory (and its mods.txt)
                if game_profile.ue4ss_mods_dir.exists() {
                    watch_paths.push(game_profile.ue4ss_mods_dir.clone());
                } else if game_profile.binaries_dir.exists() {
                    watch_paths.push(game_profile.binaries_dir.clone());
                }

                // Also ensure standard win64 ue4ss folder is watched if present
                let win64_ue4ss = gp.join("Pal").join("Binaries").join("Win64").join("ue4ss");
                if win64_ue4ss.exists() && !watch_paths.contains(&win64_ue4ss) {
                    watch_paths.push(win64_ue4ss);
                }

                // Watch Paks (~mods, LogicMods)
                let paks_dir = gp.join("Pal").join("Content").join("Paks");
                if paks_dir.exists() {
                    watch_paths.push(paks_dir);
                }

                // Watch NativeMods
                let native_mods = gp.join("Mods").join("NativeMods");
                if native_mods.exists() {
                    watch_paths.push(native_mods);
                }

                // Native Steam Workshop content directory (steamapps/workshop/content/1623730)
                let ws_settings = crate::workshop::read_pal_mod_settings(&game_dir);
                if !ws_settings.workshop_root.is_empty() {
                    let ws_path = std::path::PathBuf::from(&ws_settings.workshop_root);
                    if ws_path.exists() {
                        watch_paths.push(ws_path);
                    }
                } else {
                    let default_ws = gp
                        .parent().and_then(|p| p.parent()).and_then(|p| p.parent())
                        .map(|p| p.join("workshop").join("content").join("1623730"));
                    if let Some(ws_path) = default_ws {
                        if ws_path.exists() {
                            watch_paths.push(ws_path);
                        }
                    }
                }
            }
            if !prog_path.is_empty() {
                let p = std::path::Path::new(&prog_path);
                watch_paths.push(p.join("mods-library"));
            }

            watcher::start_fs_watcher(app.handle().clone(), watch_paths);

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                match event {
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                        let is_maximized = window.is_maximized().unwrap_or(false);
                        let state = window.state::<AppState>();
                        let lock_res = state.data.lock();
                        if let Ok(mut data) = lock_res {
                            data.settings.window_maximized = Some(is_maximized);
                            if !is_maximized {
                                if let Ok(size) = window.inner_size() {
                                    if let Ok(scale_factor) = window.scale_factor() {
                                        let logical = size.to_logical::<f64>(scale_factor);
                                        if logical.width > 100.0 && logical.height > 100.0 {
                                            data.settings.window_width = Some(logical.width);
                                            data.settings.window_height = Some(logical.height);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed => {
                        let state = window.state::<AppState>();
                        let lock_res = state.data.lock();
                        if let Ok(data) = lock_res {
                            let data_clone = data.clone();
                            drop(data);
                            let _ = db::save_db(&data_clone.settings.program_path, &data_clone);
                        }
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
