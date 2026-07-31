use crate::db;
use crate::nexus;
use crate::state::AppState;
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
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

#[tauri::command]
pub async fn fetch_nexus_info_async(mod_id: u32, state: State<'_, AppState>) -> Result<Value, String> {
    let api_key = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.nexus_api_key.clone()
    };
    let info = nexus::fetch_mod_info(mod_id, api_key.as_deref()).await?;
    Ok(serde_json::to_value(&info).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn refresh_nexus_cache(mod_id_str: String, state: State<'_, AppState>) -> Result<Value, String> {
    let (api_key, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.nexus_api_key.clone(), data.settings.program_path.clone())
    };

    let mod_index = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.iter().position(|m| m.id == mod_id_str).ok_or("Mod not found")?
    };

    let (nexus_mod_id, cached_version, cached_picture, current_local_version) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let m = &data.mods[mod_index];
        (m.nexus_mod_id, m.nexus_version_cached.clone(), m.nexus_picture_url.clone(), m.version.clone())
    };

    let nexus_id = nexus_mod_id.ok_or("No NexusMods ID for this mod")?;
    crate::logger::log(&format!("refresh_nexus_cache: Fetching NexusID {} for mod '{}'", nexus_id, {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods[mod_index].name.clone()
    }));
    let info = nexus::fetch_mod_info(nexus_id, api_key.as_deref()).await?;

    let needs_refresh = match &cached_version {
        Some(v) => v != &info.version,
        None => true,
    } || cached_picture.as_deref().unwrap_or("").is_empty()
      || current_local_version == "unknown"
      || current_local_version.is_empty();

    if !needs_refresh {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        return Ok(serde_json::to_value(&data.mods[mod_index]).map_err(|e| e.to_string())?);
    }

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let m = &mut data.mods[mod_index];
    m.nexus_author = Some(info.author.clone());
    m.nexus_summary = Some(info.summary.clone());
    m.nexus_picture_url = Some(info.picture_url.clone());
    m.nexus_downloads = Some(info.downloads);
    m.nexus_endorsements = Some(info.endorsements);
    m.nexus_description = Some(info.description.clone());
    m.nexus_version_cached = Some(info.version.clone());
    m.nexus_cached_at = Some(Utc::now().to_rfc3339());
    // Only overwrite local version if it's missing/unknown — zip version takes priority
    {
        let local_ver = m.version.trim().to_lowercase();
        let is_missing = local_ver.is_empty()
            || local_ver == "unknown"
            || local_ver == "1.0"
            || local_ver == "v1.0"
            || local_ver == "1"
            || local_ver == "v1";
        if is_missing && !info.version.is_empty() && info.version != "unknown" {
            crate::logger::log(&format!(
                "refresh_nexus_cache: version was '{}', updating to Nexus version '{}'",
                m.version, info.version
            ));
            m.version = info.version.clone();
        } else {
            crate::logger::log(&format!(
                "refresh_nexus_cache: keeping local version '{}', Nexus reports '{}'",
                m.version, info.version
            ));
        }
    }
    m.nexus_category = if info.category.is_empty() { None } else { Some(info.category.clone()) };
    m.nexus_tags = info.tags.clone();

    let cache_dir = if m.enabled {
        PathBuf::from(&m.game_path)
    } else {
        PathBuf::from(&m.disabled_path)
    };
    if cache_dir.exists() {
        let cache_json = serde_json::json!({
            "modId": nexus_id,
            "name": info.name,
            "author": info.author,
            "summary": info.summary,
            "description": info.description,
            "version": info.version,
            "downloads": info.downloads,
            "endorsements": info.endorsements,
            "pictureUrl": info.picture_url,
            "createdAt": info.created_at,
            "updatedAt": info.updated_at,
            "category": info.category,
            "tags": info.tags,
        });
        let _ = fs::write(cache_dir.join(".nexus.json"), serde_json::to_string_pretty(&cache_json).unwrap_or_default());
    }

    let result = serde_json::to_value(&data.mods[mod_index]).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    Ok(result)
}

#[tauri::command]
pub async fn set_nexus_mod_id(mod_id_str: String, nexus_id: u32, state: State<'_, AppState>) -> Result<Value, String> {
    let program_path: String;
    let mod_index: usize;
    {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        program_path = data.settings.program_path.clone();
        mod_index = data.mods.iter().position(|m| m.id == mod_id_str).ok_or("Mod not found")?;
    }

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let m = &mut data.mods[mod_index];
        m.nexus_mod_id = Some(nexus_id);
    }

    let data = state.data.lock().map_err(|e| e.to_string())?;
    let result = serde_json::to_value(&data.mods[mod_index]).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    Ok(result)
}

#[tauri::command]
pub async fn check_for_updates(state: State<'_, AppState>) -> Result<Vec<UpdateCheckResult>, String> {
    let api_key = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.nexus_api_key.clone()
    };

    let mods_to_check: Vec<(String, String, String, u32)> = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.iter()
            .filter_map(|m| {
                if let (Some(nid), Some(ref cached_ver)) = (m.nexus_mod_id, &m.nexus_version_cached) {
                    Some((m.id.clone(), m.name.clone(), cached_ver.clone(), nid))
                } else {
                    None
                }
            })
            .collect()
    };

    let mut results = Vec::new();

    for (mod_id, name, cached_ver, nexus_id) in mods_to_check {
        crate::logger::log(&format!("check_for_updates: Checking '{}' (NexusID {})", name, nexus_id));
        match crate::nexus::fetch_mod_info(nexus_id, api_key.as_deref()).await {
            Ok(info) => {
                let norm_cached = cached_ver.trim_start_matches(|c| c == 'v' || c == 'V').trim().to_lowercase();
                let norm_latest = info.version.trim_start_matches(|c| c == 'v' || c == 'V').trim().to_lowercase();

                let is_different = !norm_latest.is_empty() 
                    && norm_latest != "unknown" 
                    && norm_cached != "unknown"
                    && norm_latest != norm_cached
                    && !(norm_cached.starts_with(&norm_latest) && norm_cached.len() > norm_latest.len());

                if is_different {
                    results.push(UpdateCheckResult {
                        mod_id,
                        name,
                        current_version: cached_ver,
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
