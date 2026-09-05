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

pub(crate) fn is_version_newer(local: &str, remote: &str) -> bool {
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

#[tauri::command]
pub fn start_nexus_oauth() -> Result<String, String> {
    crate::nexus_oauth::start_oauth_flow()
}

#[tauri::command]
pub async fn handle_nexus_oauth_callback(
    callback_url: String,
    state: State<'_, AppState>,
) -> Result<crate::models::NexusAccountInfo, String> {
    let account = crate::nexus_oauth::handle_oauth_callback(&callback_url).await?;

    let program_path = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.nexus_account = Some(account.clone());
        data.settings.program_path.clone()
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    let _ = db::save_db(&program_path, &data_clone);

    Ok(account)
}

#[tauri::command]
pub async fn get_nexus_account_status(
    state: State<'_, AppState>,
) -> Result<Option<crate::models::NexusAccountInfo>, String> {
    let current_account = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.nexus_account.clone()
    };

    let mut account = match current_account {
        Some(a) => a,
        None => return Ok(None),
    };

    // Check if OAuth token needs refresh
    if let (Some(expires_at), Some(ref refresh_tok)) = (account.token_expires_at, &account.refresh_token) {
        let now = chrono::Utc::now().timestamp();
        // If token expires in less than 5 minutes
        if now + 300 >= expires_at {
            crate::logger::log("get_nexus_account_status: Access token expiring soon, refreshing...");
            match crate::nexus_oauth::refresh_access_token(refresh_tok).await {
                Ok(new_tokens) => {
                    let exp_in = new_tokens.expires_in.unwrap_or(3600);
                    account.access_token = Some(new_tokens.access_token.clone());
                    if let Some(rt) = new_tokens.refresh_token {
                        account.refresh_token = Some(rt);
                    }
                    account.token_expires_at = Some(now + exp_in);

                    // Save updated tokens
                    let program_path = {
                        let mut data = state.data.lock().map_err(|e| e.to_string())?;
                        data.settings.nexus_account = Some(account.clone());
                        data.settings.program_path.clone()
                    };
                    let data_clone = {
                        let data = state.data.lock().map_err(|e| e.to_string())?;
                        data.clone()
                    };
                    let _ = db::save_db(&program_path, &data_clone);
                }
                Err(e) => {
                    crate::logger::log(&format!("get_nexus_account_status: Refresh token failed: {}", e));
                }
            }
        }
    }

    Ok(Some(account))
}

#[tauri::command]
pub fn logout_nexus_account(state: State<'_, AppState>) -> Result<(), String> {
    let program_path = {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.nexus_account = None;
        data.settings.program_path.clone()
    };

    let data_clone = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.clone()
    };
    let _ = db::save_db(&program_path, &data_clone);
    crate::logger::log("logout_nexus_account: Cleared Nexus account from settings");
    Ok(())
}

#[tauri::command]
pub async fn refresh_nexus_account_profile(state: State<'_, AppState>) -> Result<Option<crate::models::NexusAccountInfo>, String> {
    let (access_tok_opt, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.nexus_account.as_ref().and_then(|a| a.access_token.clone()),
            data.settings.program_path.clone()
        )
    };

    let token = match access_tok_opt {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };

    let profile = crate::nexus_oauth::fetch_user_profile(&token).await?;

    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut account) = data.settings.nexus_account {
        account.username = Some(profile.username);
        account.user_id = profile.user_id;
        account.avatar_url = profile.avatar_url;
        account.kudos = profile.kudos;
        account.profile_views = profile.profile_views;
        account.endorsements_given = profile.endorsements_given;
        account.joined_date = profile.joined_date;
        account.last_active_date = profile.last_active_date;
        account.about_me = profile.about_me;
        account.mod_count = profile.mod_count;
        account.roles = profile.roles.clone();
        account.is_premium = profile.roles.iter().any(|r| {
            let lr = r.to_lowercase();
            lr == "premium" || lr == "lifetimepremium"
        });
        account.is_supporter = profile.roles.iter().any(|r| r.to_lowercase() == "supporter");
    }

    let updated_account = data.settings.nexus_account.clone();
    let data_clone = data.clone();
    drop(data);

    let _ = db::save_db(&program_path, &data_clone);
    Ok(updated_account)
}

#[tauri::command]
pub fn check_nexus_protocol_status() -> Result<crate::protocol_handler::DetailedProtocolInfo, String> {
    Ok(crate::protocol_handler::get_detailed_protocol_info())
}

#[tauri::command]
pub fn register_nexus_protocol(scheme: Option<String>) -> Result<String, String> {
    let s = scheme.as_deref().unwrap_or("all");
    crate::protocol_handler::register_specific_scheme(s)
}

#[tauri::command]
pub fn unregister_nexus_protocol(scheme: Option<String>) -> Result<(), String> {
    let s = scheme.as_deref().unwrap_or("all");
    crate::protocol_handler::unregister_specific_scheme(s)
}

#[tauri::command]
pub async fn get_nexus_user_endorsements(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<crate::models::NexusUserEndorsement>, String> {
    let now = chrono::Utc::now().timestamp();
    let is_forced = force_refresh.unwrap_or(false);

    // 1. Check local cache (valid for 24h)
    if !is_forced {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if let (Some(cache), Some(ts)) = (&data.settings.nexus_endorsements_cache, data.settings.nexus_cache_timestamp) {
            if now - ts < 86400 && !cache.is_empty() {
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

    // 1. Check local cache (valid for 24h)
    if !is_forced {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if let (Some(cache), Some(ts)) = (&data.settings.nexus_tracked_cache, data.settings.nexus_cache_timestamp) {
            if now - ts < 86400 && !cache.is_empty() {
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

    // 1. Check local cache (valid for 24h)
    if !is_forced {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if let (Some(cache), Some(ts)) = (&data.settings.nexus_authored_cache, data.settings.nexus_cache_timestamp) {
            if now - ts < 86400 && !cache.is_empty() {
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

#[tauri::command]
pub fn parse_nxm_link(nxm_url: String) -> Result<crate::nexus_oauth::NxmLinkInfo, String> {
    crate::nexus_oauth::parse_nxm_url(&nxm_url)
}

#[tauri::command]
pub async fn get_nxm_mod_metadata(
    game_domain: String,
    mod_id: u32,
    state: State<'_, AppState>,
) -> Result<crate::nexus_oauth::NxmModMetadata, String> {
    let access_token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("No active Nexus Mods session.")?;

    crate::nexus_oauth::fetch_nxm_mod_metadata(&access_token, &game_domain, mod_id).await
}

#[tauri::command]
pub async fn download_nxm_file(
    nxm_url: String,
    download_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    crate::logger::log(&format!("download_nxm_file: Processing URL: {} (id: {})", nxm_url, download_id));

    let (cdn_url, file_id, preferred_filename) = if nxm_url.starts_with("http://") || nxm_url.starts_with("https://") {
        (nxm_url, 0u64, None)
    } else {
        let nxm = crate::nexus_oauth::parse_nxm_url(&nxm_url)?;
        let access_token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
            .ok_or("No active Nexus Mods session. Please connect your Nexus Mods account in Settings / Profile first.")?;
        
        let file_name = crate::nexus_oauth::fetch_nxm_file_details(
            &access_token,
            &nxm.game_domain,
            nxm.mod_id as u32,
            nxm.file_id,
        ).await.ok();

        let url = crate::nexus_oauth::fetch_nxm_direct_download_url(&access_token, &nxm).await?;
        (url, nxm.file_id, file_name)
    };

    // Stream download file to temp directory with progress events
    let temp_zip = crate::nexus_oauth::download_file_to_temp_with_progress(
        &app_handle,
        &cdn_url,
        file_id,
        &download_id,
        preferred_filename.as_deref(),
    ).await?;

    Ok(temp_zip.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn handle_nxm_download(
    nxm_url: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let download_id = uuid::Uuid::new_v4().to_string();
    let zip_str = download_nxm_file(nxm_url, download_id, app_handle, state.clone()).await?;
    let temp_path = std::path::PathBuf::from(&zip_str);

    let install_res = crate::commands::install::install_mod_command(
        zip_str,
        None,
        None,
        None,
        state,
    ).await;

    let _ = std::fs::remove_file(&temp_path);
    install_res
}




