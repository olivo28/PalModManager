use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;
use crate::models::{ModInfo, AppSettings, Profile};

/// Full in-memory database snapshot returned by db_get_all
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbSnapshot {
    pub mods: Vec<ModInfo>,
    pub profiles: Vec<Profile>,
    pub current_profile_id: String,
    pub settings: AppSettings,
}

/// Returns the full in-memory AppData as a structured snapshot for the DB Inspector.
#[tauri::command]
pub async fn db_get_all(state: State<'_, AppState>) -> Result<DbSnapshot, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    Ok(DbSnapshot {
        mods: data.mods.clone(),
        profiles: data.profiles.clone(),
        current_profile_id: data.current_profile_id.clone(),
        settings: data.settings.clone(),
    })
}

/// Writes a modified JSON record back to in-memory state and flushes the DB to disk.
/// record_type: "mod" | "profile" | "settings"
/// record_id:   the `id` field (unused for "settings")
/// json:        the raw modified JSON string — validated server-side before write
#[tauri::command]
pub async fn db_write_record(
    state: State<'_, AppState>,
    record_type: String,
    record_id: String,
    json: String,
) -> Result<(), String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        match record_type.as_str() {
            "mod" => {
                let updated: ModInfo = serde_json::from_str(&json)
                    .map_err(|e| format!("Invalid mod JSON: {}", e))?;
                let pos = data.mods.iter().position(|m| m.id == record_id)
                    .ok_or_else(|| format!("Mod '{}' not found", record_id))?;
                data.mods[pos] = updated;
            }
            "profile" => {
                let updated: Profile = serde_json::from_str(&json)
                    .map_err(|e| format!("Invalid profile JSON: {}", e))?;
                let pos = data.profiles.iter().position(|p| p.id == record_id)
                    .ok_or_else(|| format!("Profile '{}' not found", record_id))?;
                data.profiles[pos] = updated;
            }
            "settings" => {
                let updated: AppSettings = serde_json::from_str(&json)
                    .map_err(|e| format!("Invalid settings JSON: {}", e))?;
                data.settings = updated;
            }
            _ => return Err(format!("Unknown record_type '{}'", record_type)),
        }
    }

    let data = state.data.lock().map_err(|e| e.to_string())?;
    crate::db::save_db(&program_path, &data).map_err(|e| e.to_string())?;
    Ok(())
}
