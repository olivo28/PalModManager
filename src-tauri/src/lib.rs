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
    
    logger::log(&format!("Loading database from {}", program_path.display()));
    let start_db = std::time::Instant::now();
    let mut data = db::load_db(&program_path.to_string_lossy());
    logger::log(&format!("Database loaded successfully in {:?}", start_db.elapsed()));

    if data.settings.program_path.is_empty() {
        data.settings.program_path = program_path.to_string_lossy().to_string();
        let _ = db::save_db(&data.settings.program_path, &data);
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
            settings_commands::set_nexus_api_key,
            settings_commands::set_hide_native_mods,
            settings_commands::set_debug_console,

            mod_commands::get_mods,
            mod_commands::scan_mods,
            mod_commands::remove_mod,
            mod_commands::disable_mod,
            mod_commands::enable_mod,
            mod_commands::disable_all_mods,
            mod_commands::enable_all_mods,
            mod_commands::open_folder,
            mod_commands::open_extra_folder,
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
            library_commands::get_library,
            library_commands::install_mod_from_library,
            library_commands::remove_from_library,
            library_commands::get_library_zip_path,
            library_commands::copy_to_library_command,
            profile_commands::get_profiles,
            profile_commands::get_current_profile,
            profile_commands::switch_profile_command,
            profile_commands::create_profile_command,
            profile_commands::delete_profile_command,
            profile_commands::rename_profile_command,
            profile_commands::set_mod_profile_state,
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
            mod_commands::restore_backup,
            mod_commands::analyze_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
