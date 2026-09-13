use crate::db;
use crate::nexus;
use crate::state::AppState;
use chrono::Utc;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn fetch_nexus_info_async(mod_id: u32, _state: State<'_, AppState>) -> Result<Value, String> {
    let info = nexus::fetch_mod_info(mod_id).await?;
    Ok(serde_json::to_value(&info).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn refresh_nexus_cache(mod_id_str: String, state: State<'_, AppState>) -> Result<Value, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
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
    let info = nexus::fetch_mod_info(nexus_id).await?;

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
            || local_ver == "unknown";
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
        let _ = crate::profiles::save_pmm_meta(m);
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

    // Fetch official mod details from Nexus
    let info = crate::nexus::fetch_mod_info(nexus_id).await.ok();

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let m = &mut data.mods[mod_index];
        m.nexus_mod_id = Some(nexus_id);

        if let Some(ref info) = info {
            m.name = info.name.clone();
            m.nexus_author = Some(info.author.clone());
            m.nexus_summary = Some(info.summary.clone());
            m.nexus_picture_url = Some(info.picture_url.clone());
            m.nexus_downloads = Some(info.downloads);
            m.nexus_endorsements = Some(info.endorsements);
            m.nexus_description = Some(info.description.clone());
            m.nexus_version_cached = Some(info.version.clone());
            m.nexus_cached_at = Some(Utc::now().to_rfc3339());
            m.nexus_category = if info.category.is_empty() { None } else { Some(info.category.clone()) };
            m.nexus_tags = info.tags.clone();

            if m.version == "unknown" || m.version.is_empty() {
                m.version = info.version.clone();
            }

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
        }
        let _ = crate::profiles::save_pmm_meta(m);
    }

    let data = state.data.lock().map_err(|e| e.to_string())?;
    let result = serde_json::to_value(&data.mods[mod_index]).map_err(|e| e.to_string())?;
    let data_clone = data.clone();
    drop(data);
    let _ = db::save_db(&program_path, &data_clone);
    Ok(result)
}
