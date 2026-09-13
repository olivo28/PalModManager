use crate::db;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub mod_id: String,
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub nexus_mod_id: u32,
}

pub fn is_version_newer(local: &str, remote: &str) -> bool {
    let local_parts: Vec<u32> = local.split('.')
        .map(|p| p.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))
        .collect();
    let remote_parts: Vec<u32> = remote.split('.')
        .map(|p| p.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))
        .collect();
    
    let max_len = std::cmp::max(local_parts.len(), remote_parts.len());
    for i in 0..max_len {
        let l = *local_parts.get(i).unwrap_or(&0);
        let r = *remote_parts.get(i).unwrap_or(&0);
        if r > l {
            return true;
        }
        if l > r {
            return false;
        }
    }
    false
}

#[tauri::command]
pub async fn check_for_updates(state: State<'_, AppState>) -> Result<Vec<UpdateCheckResult>, String> {
    let mods_to_check: Vec<(String, String, String, u32, Option<String>)> = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let current_profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
        let installed_ids = current_profile.map(|p| &p.installed_mod_ids);

        data.mods.iter()
            .filter(|m| {
                if let Some(ids) = installed_ids {
                    ids.iter().any(|id| crate::profiles::mod_matches_profile_entry(m, id))
                } else {
                    true
                }
            })
            .filter_map(|m| {
                if let Some(nid) = m.nexus_mod_id {
                    Some((m.id.clone(), m.name.clone(), m.version.clone(), nid, m.ignored_version.clone()))
                } else {
                    None
                }
            })
            .collect()
    };

    let mut results = Vec::new();

    for (mod_id, name, local_ver, nexus_id, ignored_ver) in mods_to_check {
        crate::logger::log(&format!("check_for_updates: Checking '{}' (NexusID {})", name, nexus_id));
        match crate::nexus::fetch_mod_info(nexus_id).await {
            Ok(info) => {
                let norm_local = local_ver.trim_start_matches(|c| c == 'v' || c == 'V').trim().to_lowercase();
                let norm_latest = info.version.trim_start_matches(|c| c == 'v' || c == 'V').trim().to_lowercase();

                if norm_latest != "unknown" && norm_local != "unknown" && is_version_newer(&norm_local, &norm_latest) {
                    // Check if this latest version has been ignored
                    if let Some(ref ignored) = ignored_ver {
                        let norm_ignored = ignored.trim_start_matches(|c| c == 'v' || c == 'V').trim().to_lowercase();
                        if norm_ignored == norm_latest {
                            continue;
                        }
                    }

                    results.push(UpdateCheckResult {
                        mod_id,
                        name,
                        current_version: local_ver,
                        latest_version: info.version,
                        nexus_mod_id: nexus_id,
                    });
                }
            }
            Err(_) => {}
        }
    }

    Ok(results)
}

#[tauri::command]
pub fn ignore_mod_version(
    mod_id: String,
    version: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    if let Some(m) = data.mods.iter_mut().find(|m| m.id == mod_id) {
        m.ignored_version = version;
    } else {
        return Err("Mod not found".to_string());
    }

    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    Ok(())
}
