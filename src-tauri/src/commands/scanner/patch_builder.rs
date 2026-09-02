// Compatibility patch building, listing, and deletion commands
use std::path::PathBuf;
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub async fn build_compatibility_pak_cmd(
    request: crate::pak_patcher::PatchBuildRequest,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<crate::pak_patcher::PatchBuildResult, String> {
    use tauri::Manager;
    let (game_path, program_path, current_profile_id) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.game_path.clone(),
            data.settings.program_path.clone(),
            data.current_profile_id.clone(),
        )
    };
    if game_path.is_empty() {
        return Err("Game path is not configured in Settings".to_string());
    }
    let app_data_dir = if !program_path.is_empty() {
        PathBuf::from(&program_path)
    } else {
        app_handle.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    crate::pak_patcher::build_compatibility_pak_internal(
        request,
        &game_path,
        &app_data_dir,
        &program_path,
        &current_profile_id,
    ).await
}

#[tauri::command]
pub fn list_generated_patches_cmd(state: State<'_, AppState>) -> Result<Vec<crate::pak_patcher::GeneratedPatchInfo>, String> {
    let (game_path, program_path, current_profile_id) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.game_path.clone(),
            data.settings.program_path.clone(),
            data.current_profile_id.clone(),
        )
    };
    if game_path.is_empty() {
        return Ok(Vec::new());
    }
    crate::pak_patcher::list_generated_patches_internal(&game_path, &program_path, &current_profile_id)
}

#[tauri::command]
pub fn delete_generated_patch_cmd(patch_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let (program_path, current_profile_id) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.program_path.clone(),
            data.current_profile_id.clone(),
        )
    };
    crate::pak_patcher::delete_generated_patch_internal(&patch_path, &program_path, &current_profile_id)
}
