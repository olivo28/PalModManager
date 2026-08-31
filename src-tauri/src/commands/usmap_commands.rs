use tauri::State;
use crate::state::AppState;
use crate::usmap::{UsmapStatus, UsmapStruct};

#[tauri::command]
pub fn get_mappings_status(state: State<'_, AppState>) -> Result<UsmapStatus, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();
    drop(data);

    Ok(crate::usmap::get_mappings_status(&program_path, &game_path))
}

#[tauri::command]
pub async fn sync_mappings_now(state: State<'_, AppState>) -> Result<UsmapStatus, String> {
    let (program_path, game_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.settings.game_path.clone())
    };

    crate::usmap::invalidate_schema_cache();
    crate::usmap::sync_mappings_async(program_path, game_path).await
}

#[tauri::command]
pub fn get_usmap_struct_info(struct_name: String, state: State<'_, AppState>) -> Result<Option<UsmapStruct>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    if let Some(schema) = crate::usmap::get_or_load_schema(&program_path) {
        Ok(schema.get_struct(&struct_name).cloned())
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn get_usmap_summary(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    if let Some(schema) = crate::usmap::get_or_load_schema(&program_path) {
        Ok(serde_json::json!({
            "loaded": true,
            "totalStructs": schema.total_structs,
            "totalEnums": schema.total_enums,
            "totalNames": schema.total_names,
            "gameVersion": schema.game_version
        }))
    } else {
        Ok(serde_json::json!({
            "loaded": false,
            "totalStructs": 0,
            "totalEnums": 0,
            "totalNames": 0
        }))
    }
}
