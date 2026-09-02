// World save scanning, backup, repair, and metadata commands
use std::path::Path;
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn list_save_worlds_cmd(
    state: State<'_, AppState>,
    custom_dir: Option<String>,
) -> Result<Vec<crate::save_scanner::SaveWorldSummary>, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    crate::save_scanner::list_save_worlds(custom_dir.as_deref(), Some(&program_path))
}

#[tauri::command]
pub fn deep_scan_save_cmd(
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

    crate::save_scanner::deep_scan_save(&world_dir, &active_mod_names, Some(&program_path))
}

#[tauri::command]
pub fn repair_save_cmd(
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

    crate::save_scanner::repair_and_sanitize_save(&world_dir, &program_path)
}

#[tauri::command]
pub fn restore_save_backup_cmd(
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

    crate::save_scanner::restore_save_from_backup(&world_dir, &backup_slot, &program_path)
}

#[tauri::command]
pub fn create_world_backup_cmd(
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
    crate::save_scanner::create_manual_world_backup(&world_dir, &program_path, custom_dest.as_deref())
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
pub fn restore_pmm_world_backup_cmd(
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
    crate::save_scanner::restore_pmm_world_backup(&world_dir, &backup_file_path, &program_path)
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
    open::that(&backups_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_world_folder_cmd(world_dir: String) -> Result<(), String> {
    crate::save_scanner::open_world_folder(&world_dir)
}

#[tauri::command]
pub fn export_world_zip_cmd(world_dir: String, target_path: String) -> Result<String, String> {
    crate::save_scanner::create_manual_world_backup(&world_dir, ".", Some(&target_path))
}

#[tauri::command]
pub fn prune_world_backups_cmd(world_dir: String, keep_count: usize) -> Result<usize, String> {
    crate::save_scanner::prune_world_backups(&world_dir, keep_count)
}

#[tauri::command]
pub fn save_world_custom_meta_cmd(world_dir: String, meta: crate::save_scanner::WorldCustomMeta) -> Result<(), String> {
    crate::save_scanner::save_world_custom_meta(Path::new(&world_dir), &meta)
}

#[tauri::command]
pub fn get_world_custom_meta_cmd(world_dir: String) -> Result<crate::save_scanner::WorldCustomMeta, String> {
    Ok(crate::save_scanner::load_world_custom_meta(Path::new(&world_dir)))
}

#[tauri::command]
pub fn inspect_snapshot_details_cmd(world_dir: String, slot_name: String) -> Result<crate::save_scanner::SaveBackupSnapshot, String> {
    crate::save_scanner::inspect_snapshot_details(Path::new(&world_dir), &slot_name)
}
