// World save scanning, backup, repair, and metadata commands
use std::path::Path;
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub async fn list_save_worlds_cmd(
    state: State<'_, AppState>,
    custom_dir: Option<String>,
    force_refresh: Option<bool>,
) -> Result<Vec<crate::save_scanner::SaveWorldSummary>, String> {
    if force_refresh.unwrap_or(false) {
        crate::save_scanner::invalidate_save_world_cache(None);
    }
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        crate::save_scanner::list_save_worlds(custom_dir.as_deref(), Some(&program_path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn deep_scan_save_cmd(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    world_dir: String,
) -> Result<crate::save_scanner::SaveHealthReport, String> {
    let (active_mod_names, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let p_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
        let names = p_mods.into_iter().filter(|m| m.enabled).map(|m| m.name).collect::<Vec<String>>();
        let prog_p = if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        };
        (names, prog_p)
    };

    let task = crate::worker::WorkerTask::SaveDeepScan {
        world_dir,
        active_mod_names,
        program_path: Some(program_path),
    };

    crate::worker::run_worker_task(Some(std::sync::Arc::new(app)), task, "save-scan-progress").await
}

#[tauri::command]
pub async fn repair_save_cmd(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    world_dir: String,
) -> Result<crate::save_scanner::SaveRepairResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };

    let world_dir_clone = world_dir.clone();
    let task = crate::worker::WorkerTask::SaveRepair {
        world_dir: world_dir_clone,
        program_path,
    };

    let res = crate::worker::run_worker_task(Some(std::sync::Arc::new(app)), task, "save-repair-progress").await?;
    crate::save_scanner::invalidate_save_world_cache(Some(&world_dir));
    Ok(res)
}

#[tauri::command]
pub async fn restore_save_backup_cmd(
    state: State<'_, AppState>,
    world_dir: String,
    backup_slot: String,
) -> Result<crate::save_scanner::SaveRepairResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };

    let world_dir_clone = world_dir.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        crate::save_scanner::restore_save_from_backup(&world_dir_clone, &backup_slot, &program_path)
    })
    .await
    .map_err(|e| e.to_string())??;

    crate::save_scanner::invalidate_save_world_cache(Some(&world_dir));
    Ok(res)
}

#[tauri::command]
pub async fn create_world_backup_cmd(
    state: State<'_, AppState>,
    world_dir: String,
    custom_dest: Option<String>,
) -> Result<String, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };

    tauri::async_runtime::spawn_blocking(move || {
        crate::save_scanner::create_manual_world_backup(&world_dir, &program_path, custom_dest.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn list_pmm_world_backups_cmd(
    state: State<'_, AppState>,
    world_name_filter: Option<String>,
) -> Result<Vec<crate::save_scanner::PmmWorldBackup>, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    Ok(crate::save_scanner::list_pmm_world_backups(&program_path, world_name_filter.as_deref()))
}

#[tauri::command]
pub async fn restore_pmm_world_backup_cmd(
    state: State<'_, AppState>,
    world_dir: String,
    backup_file_path: String,
) -> Result<crate::save_scanner::SaveRepairResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };

    let world_dir_clone = world_dir.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        crate::save_scanner::restore_pmm_world_backup(&world_dir_clone, &backup_file_path, &program_path)
    })
    .await
    .map_err(|e| e.to_string())??;

    crate::save_scanner::invalidate_save_world_cache(Some(&world_dir));
    Ok(res)
}

#[tauri::command]
pub fn delete_pmm_world_backup_cmd(
    backup_file_path: String,
) -> Result<(), String> {
    crate::save_scanner::delete_pmm_world_backup(&backup_file_path)
}

#[tauri::command]
pub fn open_pmm_world_backups_folder_cmd(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    let backups_dir = Path::new(&program_path).join("backups").join("worlds");
    if !backups_dir.exists() {
        let _ = std::fs::create_dir_all(&backups_dir);
    }
    crate::system_open::open_path_in_system(&backups_dir)
}

#[tauri::command]
pub fn open_world_folder_cmd(world_dir: String) -> Result<(), String> {
    crate::save_scanner::open_world_folder(&world_dir)
}

#[tauri::command]
pub async fn export_world_zip_cmd(world_dir: String, target_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::save_scanner::create_manual_world_backup(&world_dir, ".", Some(&target_path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn prune_world_backups_cmd(world_dir: String, keep_count: usize) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::save_scanner::prune_world_backups(&world_dir, keep_count)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn save_world_custom_meta_cmd(world_dir: String, meta: crate::save_scanner::WorldCustomMeta) -> Result<(), String> {
    crate::save_scanner::save_world_custom_meta(Path::new(&world_dir), &meta)?;
    crate::save_scanner::invalidate_save_world_cache(Some(&world_dir));
    Ok(())
}

#[tauri::command]
pub fn get_world_custom_meta_cmd(world_dir: String) -> Result<crate::save_scanner::WorldCustomMeta, String> {
    Ok(crate::save_scanner::load_world_custom_meta(Path::new(&world_dir)))
}

#[tauri::command]
pub async fn inspect_snapshot_details_cmd(world_dir: String, slot_name: String) -> Result<crate::save_scanner::SaveBackupSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::save_scanner::inspect_snapshot_details(Path::new(&world_dir), &slot_name)
    })
    .await
    .map_err(|e| e.to_string())?
}
