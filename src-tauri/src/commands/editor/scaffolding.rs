use std::fs;
use tauri::State;
use crate::state::AppState;

/// Create a new file within an existing mod workspace
#[tauri::command]
pub fn create_mod_file(
    mod_id: String,
    relative_path: String,
    initial_content: Option<String>,
    state: State<AppState>,
) -> Result<String, String> {
    crate::logger::log(&format!("create_mod_file: Creating '{}' for mod '{}'", relative_path, mod_id));
    let clean_rel = relative_path.trim().replace('\\', "/");
    if clean_rel.is_empty() || clean_rel.starts_with('/') || clean_rel.contains("..") {
        return Err("Invalid file path".to_string());
    }

    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;

    let full_path = crate::commands::config_commands::get_full_mod_file_path(mod_info, &clean_rel)?;

    if full_path.exists() {
        return Err("File already exists".to_string());
    }

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create directory structure: {}", e))?;
    }

    let content = initial_content.unwrap_or_default();
    fs::write(&full_path, &content).map_err(|e| format!("Cannot write file: {}", e))?;

    // Invalidate symbol index so Monaco immediately discovers any new symbols
    super::workspace_index::invalidate_workspace_index(&mod_id);

    crate::logger::log(&format!("create_mod_file: Successfully created '{}'", full_path.display()));
    Ok(clean_rel)
}

/// Create a new folder within an existing mod workspace
#[tauri::command]
pub fn create_editor_folder(
    mod_id: String,
    relative_path: String,
    state: State<AppState>,
) -> Result<String, String> {
    crate::logger::log(&format!("create_editor_folder: Creating folder '{}' for mod '{}'", relative_path, mod_id));
    let clean_rel = relative_path.trim().replace('\\', "/");
    if clean_rel.is_empty() || clean_rel.starts_with('/') || clean_rel.contains("..") {
        return Err("Invalid folder path".to_string());
    }

    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mod_info = data.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;

    let full_path = crate::commands::config_commands::get_full_mod_file_path(mod_info, &clean_rel)?;

    fs::create_dir_all(&full_path).map_err(|e| format!("Cannot create folder: {}", e))?;

    crate::logger::log(&format!("create_editor_folder: Successfully created folder '{}'", full_path.display()));
    Ok(clean_rel)
}
