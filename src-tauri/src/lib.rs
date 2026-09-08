pub mod commands;
pub mod profiles;
pub mod db;
pub mod dependency_checker;
pub mod installer;
pub mod library;
pub mod models;
pub mod nexus;
pub mod nexus_oauth;
pub mod protocol_handler;
mod state;
pub mod zip_handler;
pub mod logger;
pub mod config_merge;
mod workshop;
mod watcher;
pub mod safety_backup;
pub mod image_proxy;
pub mod altermatic;
pub mod pak_scanner;
pub mod pak_patcher;
pub mod retoc_runner;
pub mod save_scanner;
pub mod usmap;
pub mod texture_decoder;
pub mod dependency_manifest;

use commands::mod_commands;
use commands::settings_commands;
use commands::install;
use commands::config_commands;
use commands::nexus_commands;
use commands::launch_commands;
use commands::library_commands;
use commands::profile_commands;
use commands::dependency;
use commands::packer_commands;
use commands::scanner;
use commands::db_commands;
use commands::load_order_commands;
use commands::workshop_commands;
use commands::discovery;
use commands::altermatic_commands;
use commands::usmap_commands;
use commands::sdk_commands;
use commands::editor;
use commands::config_archive;
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
            settings_commands::set_ue4ss_control_mode,
            settings_commands::set_ue4ss_build_flavor,
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
            mod_commands::merge_mods_as_hybrid,
            config_archive::check_archived_config,
            config_archive::apply_archived_config,
            config_archive::preview_archived_config_diff,
            mod_commands::save_mod_notes,
            mod_commands::set_mod_version,
            mod_commands::set_mod_ignored_keys,
            mod_commands::check_github_version,
            mod_commands::set_github_version,
            mod_commands::export_mods_json,
            mod_commands::get_game_version,
            install::analysis::analyze_zip,
            install::install_standard::install_mod_command,
            install::analysis::check_mod_exists_command,
            install::update::update_mod_command,
            install::install_manifest::build_install_manifest,
            install::install_manifest::install_mod_with_manifest,
            install::diff::preview_config_diff,
            config_commands::read_config,
            config_commands::save_config,
            config_commands::set_mod_config,
            config_commands::set_mod_configs,
            config_commands::list_mod_files,
            config_commands::read_mod_file,
            config_commands::save_mod_file,
            config_commands::delete_mod_file,
            config_commands::restore_mod_backup,
            config_commands::merge_mod_backup,
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
            settings_commands::set_ue4ss_control_mode,
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
            dependency::status::check_dependencies,
            dependency::status::clean_conflict_dlls,
            dependency::status::reset_workshop_cache,
            dependency::safety::get_safety_backup_info_command,
            dependency::safety::trigger_safety_backup_command,
            dependency::safety::restore_safety_backup_command,
            dependency::status::check_ue4ss_latest,
            dependency::status::check_palschema_latest,
            dependency::status::check_dependencies_full,
            dependency::install::install_ue4ss,
            dependency::install::install_palschema,
            dependency::uninstall::uninstall_ue4ss,
            dependency::uninstall::uninstall_palschema,
            dependency::vault::get_dependency_vault,
            dependency::vault::install_dependency_from_vault,
            dependency::vault::install_dependency_from_custom_zip,
            dependency::vault::delete_dependency_vault_entry,
            dependency::vault::open_dependency_vault_folder,
            dependency::storage::get_storage_usage_command,
            dependency::storage::clear_temp_downloads_command,
            dependency::storage::open_temp_folder_command,
            dependency::storage::open_library_folder_command,
            settings_commands::log_from_js,
            mod_commands::create_backup,
            mod_commands::change_pak_destination,
            mod_commands::export_profile_pack_cmd,
            mod_commands::import_profile_pack_cmd,

            mod_commands::restore_backup,
            mod_commands::analyze_backup,
            packer_commands::scan_paths_for_packing,
            packer_commands::pack_mod,
            packer_commands::save_packer_project,
            packer_commands::load_packer_projects,
            packer_commands::delete_packer_project,
            scanner::conflicts::scan_conflicts,
            scanner::hotkeys::scan_mod_hotkeys,
            scanner::hotkeys::update_mod_hotkey,
            scanner::pak_inspector::inspect_pak_asset,
            scanner::pak_inspector::inspect_uasset_deep_cmd,
            scanner::pak_inspector::decode_uasset_texture_cmd,
            scanner::pak_inspector::inspect_pak_file_tree,
            scanner::pak_inspector::inspect_mod_pak_contents,
            scanner::gamepass::convert_mod_to_gamepass,
            scanner::gamepass::convert_all_gamepass_mods,
            scanner::saves::list_save_worlds_cmd,
            scanner::saves::deep_scan_save_cmd,
            scanner::saves::repair_save_cmd,
            scanner::saves::restore_save_backup_cmd,
            scanner::saves::create_world_backup_cmd,
            scanner::saves::list_pmm_world_backups_cmd,
            scanner::saves::restore_pmm_world_backup_cmd,
            scanner::saves::delete_pmm_world_backup_cmd,
            scanner::saves::open_pmm_world_backups_folder_cmd,
            scanner::saves::open_world_folder_cmd,
            scanner::saves::export_world_zip_cmd,
            scanner::saves::prune_world_backups_cmd,
            scanner::saves::save_world_custom_meta_cmd,
            scanner::saves::get_world_custom_meta_cmd,
            scanner::saves::inspect_snapshot_details_cmd,
            scanner::patch_builder::build_compatibility_pak_cmd,
            scanner::patch_builder::list_generated_patches_cmd,
            scanner::patch_builder::delete_generated_patch_cmd,
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
            discovery::categories::get_discovery_categories,
            discovery::mods::get_discovery_mods,
            discovery::details::get_discovery_mod_details,
            discovery::actions::endorse_nexus_mod,
            discovery::actions::abstain_nexus_mod,
            discovery::actions::track_nexus_mod,
            discovery::actions::untrack_nexus_mod,
            discovery::actions::install_discovery_file,
            altermatic_commands::sync_altermatic_load_list,
            altermatic_commands::get_altermatic_dep_status,
            usmap_commands::get_mappings_status,
            usmap_commands::sync_mappings_now,
            usmap_commands::get_usmap_struct_info,
            usmap_commands::get_usmap_full_struct_details,
            usmap_commands::search_usmap_entries,
            usmap_commands::get_usmap_enum_info,
            usmap_commands::get_usmap_summary,
            sdk_commands::get_sdk_status,
            sdk_commands::import_local_sdk,
            sdk_commands::sync_sdk_from_repo,
            sdk_commands::purge_sdk_cache,
            editor::validation::validate_editor_code,
            editor::completions::get_editor_completions,
            editor::completions::get_reflection_catalogs_status,
            editor::sync_catalogs::sync_blueprints_catalog,
            editor::sync_catalogs::sync_datatables_catalog,
            editor::sync_catalogs::sync_palschema_schemas,
            editor::sync_catalogs::get_palschema_schemas_catalog,
            editor::sync_catalogs::get_palschema_monaco_definitions,
            editor::sync_catalogs::get_palschema_raw_schema,
            editor::validation::scan_workspace_problems,
            editor::scaffolding::create_mod_file,
            editor::scaffolding::create_editor_folder,
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
                if let (Some(x), Some(y)) = (settings.window_x, settings.window_y) {
                    let is_on_screen = if let Ok(monitors) = window.available_monitors() {
                        monitors.into_iter().any(|m| {
                            let m_pos = m.position();
                            let m_size = m.size();
                            let sf = m.scale_factor();
                            let m_log_x = m_pos.x as f64 / sf;
                            let m_log_y = m_pos.y as f64 / sf;
                            let m_log_w = m_size.width as f64 / sf;
                            let m_log_h = m_size.height as f64 / sf;
                            x >= m_log_x - 100.0
                                && x < m_log_x + m_log_w - 100.0
                                && y >= m_log_y - 50.0
                                && y < m_log_y + m_log_h - 50.0
                        })
                    } else {
                        false
                    };
                    if is_on_screen {
                        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
                    } else {
                        let _ = window.center();
                    }
                } else {
                    let _ = window.center();
                }
                if let Some(true) = settings.window_maximized {
                    let _ = window.maximize();
                }
                let _ = window.show();
            }

            // Asynchronously pre-warm all reflection databases (USMAP, SDK, DataTables, Blueprints)
            let prog_path = settings.program_path.clone();
            let g_path = settings.game_path.clone();
            std::thread::Builder::new()
                .name("pmm-reflection-prewarm".into())
                .spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    crate::logger::log("Pre-warming reflection databases in background...");
                    let _ = crate::usmap::get_or_load_schema(&prog_path);
                    let _ = crate::usmap::get_or_load_sdk_index(&prog_path, &g_path);
                    let _ = crate::usmap::get_or_load_datatable_index(&prog_path);
                    let _ = crate::usmap::get_or_load_blueprint_index(&prog_path);
                    let _ = crate::usmap::load_palschema_definitions_for_monaco(&prog_path, &g_path, true);
                    crate::logger::log("Reflection databases pre-warmed successfully.");
                })
                .ok();

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
                                if let Ok(pos) = window.outer_position() {
                                    if let Ok(scale_factor) = window.scale_factor() {
                                        let logical_pos = pos.to_logical::<f64>(scale_factor);
                                        data.settings.window_x = Some(logical_pos.x);
                                        data.settings.window_y = Some(logical_pos.y);
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
