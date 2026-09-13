use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_nexus_user_endorsements(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<crate::models::NexusUserEndorsement>, String> {
    let now = chrono::Utc::now().timestamp();
    let is_forced = force_refresh.unwrap_or(false);

    // 1. Check local cache (valid for 4h)
    if !is_forced {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if let (Some(cache), Some(ts)) = (&data.settings.nexus_endorsements_cache, data.settings.nexus_cache_timestamp) {
            if now - ts < 14400 && !cache.is_empty() {
                return Ok(cache.clone());
            }
        }
    }

    let access_token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("No active Nexus Mods session. Please log in first.")?;
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let mut endorsements = crate::nexus_oauth::fetch_user_endorsements(&access_token).await?;

    let local_mods = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.clone()
    };

    let mut missing_tuples = Vec::new();
    for item in &mut endorsements {
        if let Some(m) = local_mods.iter().find(|m| m.nexus_mod_id == Some(item.mod_id)) {
            item.mod_title = Some(m.name.clone());
        } else {
            missing_tuples.push((item.domain_name.as_str(), item.mod_id));
        }
    }

    if !missing_tuples.is_empty() {
        let fetched = crate::nexus_oauth::batch_fetch_legacy_mods_info(&missing_tuples, Some(&access_token)).await;
        for item in &mut endorsements {
            let key = (item.domain_name.to_lowercase(), item.mod_id);
            if let Some(info) = fetched.get(&key) {
                if !info.name.is_empty() {
                    item.mod_title = Some(info.name.clone());
                }
                item.picture_url = info.picture_url.clone();
                item.summary = info.summary.clone();
            }
        }
    }

    // Persist to local database
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.nexus_endorsements_cache = Some(endorsements.clone());
        data.settings.nexus_cache_timestamp = Some(now);
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    Ok(endorsements)
}

#[tauri::command]
pub async fn get_nexus_user_tracked_mods(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<crate::models::NexusUserTrackedMod>, String> {
    let now = chrono::Utc::now().timestamp();
    let is_forced = force_refresh.unwrap_or(false);

    // 1. Check local cache (valid for 4h)
    if !is_forced {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if let (Some(cache), Some(ts)) = (&data.settings.nexus_tracked_cache, data.settings.nexus_cache_timestamp) {
            if now - ts < 14400 && !cache.is_empty() {
                return Ok(cache.clone());
            }
        }
    }

    let access_token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("No active Nexus Mods session. Please log in first.")?;
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let mut tracked = crate::nexus_oauth::fetch_user_tracked_mods(&access_token).await?;

    let local_mods = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.clone()
    };

    let mut missing_tuples = Vec::new();
    for item in &mut tracked {
        if let Some(m) = local_mods.iter().find(|m| m.nexus_mod_id == Some(item.mod_id)) {
            item.mod_title = Some(m.name.clone());
        } else {
            missing_tuples.push((item.domain_name.as_str(), item.mod_id));
        }
    }

    if !missing_tuples.is_empty() {
        let fetched = crate::nexus_oauth::batch_fetch_legacy_mods_info(&missing_tuples, Some(&access_token)).await;
        for item in &mut tracked {
            let key = (item.domain_name.to_lowercase(), item.mod_id);
            if let Some(info) = fetched.get(&key) {
                if !info.name.is_empty() {
                    item.mod_title = Some(info.name.clone());
                }
                item.picture_url = info.picture_url.clone();
                item.summary = info.summary.clone();
            }
        }
    }

    // Persist to local database
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.nexus_tracked_cache = Some(tracked.clone());
        data.settings.nexus_cache_timestamp = Some(now);
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    Ok(tracked)
}

#[tauri::command]
pub async fn get_nexus_user_authored_mods(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<crate::models::NexusUserAuthoredMod>, String> {
    let now = chrono::Utc::now().timestamp();
    let is_forced = force_refresh.unwrap_or(false);

    // 1. Check local cache (valid for 4h)
    if !is_forced {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if let (Some(cache), Some(ts)) = (&data.settings.nexus_authored_cache, data.settings.nexus_cache_timestamp) {
            if now - ts < 14400 && !cache.is_empty() {
                return Ok(cache.clone());
            }
        }
    }

    let access_token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("No active Nexus Mods session. Please log in first.")?;
    let (user_id, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let acc = data.settings.nexus_account.as_ref().ok_or("No active Nexus Mods session. Please log in first.")?;
        let uid = acc.user_id.ok_or("Missing user ID.")?;
        (uid, data.settings.program_path.clone())
    };

    let authored = crate::nexus_oauth::fetch_user_authored_mods(&access_token, user_id).await?;

    // Persist to local database
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.nexus_authored_cache = Some(authored.clone());
        data.settings.nexus_cache_timestamp = Some(now);
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    Ok(authored)
}

