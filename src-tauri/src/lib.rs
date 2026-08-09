mod commands;
mod profiles;
pub mod db;
mod dependency_checker;
mod installer;
mod library;
mod models;
pub mod nexus;
mod state;
mod zip_handler;
mod logger;

use commands::mod_commands;
use commands::settings_commands;
use commands::install_commands;
use commands::config_commands;
use commands::nexus_commands;
use commands::library_commands;
use commands::profile_commands;
use commands::dependency_commands;
use commands::packer_commands;
use commands::scanner_commands;
use commands::db_commands;
use commands::load_order_commands;
use state::AppState;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init_logger();
    logger::log("=== APPLICATION STARTED (cargo run / .exe) ===");

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
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
            mod_commands::rename_mod,
            mod_commands::set_mod_version,
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
            library_commands::get_library,
            library_commands::install_mod_from_library,
            library_commands::remove_from_library,
            library_commands::get_library_zip_path,
            library_commands::copy_to_library_command,
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
            dependency_commands::check_dependencies,
            dependency_commands::check_ue4ss_latest,
            dependency_commands::check_palschema_latest,
            dependency_commands::check_dependencies_full,
            dependency_commands::install_ue4ss,
            dependency_commands::install_palschema,
            dependency_commands::uninstall_ue4ss,
            dependency_commands::uninstall_palschema,
            settings_commands::log_from_js,
            settings_commands::open_url,
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
            db_commands::db_get_all,
            db_commands::db_write_record,
            load_order_commands::get_ue4ss_load_order,
            load_order_commands::save_ue4ss_load_order,
            load_order_commands::get_palschema_load_order,
            load_order_commands::save_palschema_load_order,
        ])
        .setup(|app| {
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
            }
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
