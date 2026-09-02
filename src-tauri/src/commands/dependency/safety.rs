use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn get_safety_backup_info_command(state: State<AppState>) -> Result<crate::safety_backup::SafetyBackupInfo, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    crate::safety_backup::get_safety_backup_info(&program_path, &game_path)
}

#[tauri::command]
pub fn trigger_safety_backup_command(state: State<AppState>) -> Result<bool, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    crate::safety_backup::create_initial_safety_backup(&game_path, &program_path, true)
}

#[tauri::command]
pub fn restore_safety_backup_command(state: State<AppState>) -> Result<(), String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    crate::safety_backup::restore_initial_safety_backup(&game_path, &program_path)
}
