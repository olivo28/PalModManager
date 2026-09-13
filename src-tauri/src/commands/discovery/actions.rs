use serde::Deserialize;
use tauri::State;
use crate::state::AppState;
use super::client::{get_client, REST_BASE_URL};

/// Endorse a mod on Nexus Mods
#[tauri::command]
pub async fn endorse_nexus_mod(mod_id: u32, version: Option<String>, state: State<'_, AppState>) -> Result<bool, String> {
    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("Not authenticated with Nexus Mods")?;

    let client = get_client(Some(&token));
    let url = format!("{}/games/palworld/mods/{}/endorse.json", REST_BASE_URL, mod_id);
    let mut params = std::collections::HashMap::new();
    if let Some(ref v) = version {
        params.insert("version", v.as_str());
    }

    let resp = client.post(&url).form(&params).send().await.map_err(|e| format!("Failed to endorse mod: {}", e))?;
    if resp.status().is_success() {
        let (mod_name, program_path) = {
            let data = state.data.lock().map_err(|e| e.to_string())?;
            let name = data.mods.iter().find(|m| m.nexus_mod_id == Some(mod_id)).map(|m| m.name.clone());
            (name, data.settings.program_path.clone())
        };
        {
            let mut data = state.data.lock().map_err(|e| e.to_string())?;
            let cache = data.settings.nexus_endorsements_cache.get_or_insert_with(Vec::new);
            if let Some(existing) = cache.iter_mut().find(|e| e.mod_id == mod_id) {
                existing.status = Some("Endorsed".to_string());
                existing.version = version.clone();
                existing.date = Some(chrono::Utc::now().to_rfc3339());
            } else {
                cache.push(crate::models::NexusUserEndorsement {
                    mod_id,
                    domain_name: "palworld".to_string(),
                    date: Some(chrono::Utc::now().to_rfc3339()),
                    version,
                    status: Some("Endorsed".to_string()),
                    mod_title: mod_name,
                    picture_url: None,
                    summary: None,
                });
            }
            let data_clone = data.clone();
            drop(data);
            let _ = crate::db::save_db(&program_path, &data_clone);
        }
        Ok(true)
    } else {
        let err_text = resp.text().await.unwrap_or_default();
        if err_text.contains("IS_OWN_MOD") {
            Err("IS_OWN_MOD".to_string())
        } else {
            Err(format!("Endorsement failed: {}", err_text))
        }
    }
}

/// Abstain / remove endorsement for a mod
#[tauri::command]
pub async fn abstain_nexus_mod(mod_id: u32, version: Option<String>, state: State<'_, AppState>) -> Result<bool, String> {
    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("Not authenticated with Nexus Mods")?;

    let client = get_client(Some(&token));
    let url = format!("{}/games/palworld/mods/{}/abstain.json", REST_BASE_URL, mod_id);
    let mut params = std::collections::HashMap::new();
    if let Some(ref v) = version {
        params.insert("version", v.as_str());
    }

    let resp = client.post(&url).form(&params).send().await.map_err(|e| format!("Failed to abstain endorsement: {}", e))?;
    if resp.status().is_success() {
        let program_path = {
            let mut data = state.data.lock().map_err(|e| e.to_string())?;
            if let Some(ref mut cache) = data.settings.nexus_endorsements_cache {
                cache.retain(|e| e.mod_id != mod_id);
            }
            data.settings.program_path.clone()
        };
        let data_clone = {
            let data = state.data.lock().map_err(|e| e.to_string())?;
            data.clone()
        };
        let _ = crate::db::save_db(&program_path, &data_clone);
        Ok(true)
    } else {
        let err_text = resp.text().await.unwrap_or_default();
        Err(format!("Abstaining endorsement failed: {}", err_text))
    }
}

/// Track a mod on Nexus Mods
#[tauri::command]
pub async fn track_nexus_mod(mod_id: u32, state: State<'_, AppState>) -> Result<bool, String> {
    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("Not authenticated with Nexus Mods")?;

    let client = get_client(Some(&token));
    let url = format!("{}/user/tracked_mods.json?domain_name=palworld", REST_BASE_URL);
    let mut params = std::collections::HashMap::new();
    params.insert("mod_id", mod_id.to_string());

    let resp = client.post(&url).form(&params).send().await.map_err(|e| format!("Failed to track mod: {}", e))?;
    if resp.status().is_success() {
        let (mod_name, program_path) = {
            let data = state.data.lock().map_err(|e| e.to_string())?;
            let name = data.mods.iter().find(|m| m.nexus_mod_id == Some(mod_id)).map(|m| m.name.clone());
            (name, data.settings.program_path.clone())
        };
        {
            let mut data = state.data.lock().map_err(|e| e.to_string())?;
            let cache = data.settings.nexus_tracked_cache.get_or_insert_with(Vec::new);
            if !cache.iter().any(|m| m.mod_id == mod_id) {
                cache.push(crate::models::NexusUserTrackedMod {
                    mod_id,
                    domain_name: "palworld".to_string(),
                    mod_title: mod_name,
                    picture_url: None,
                    summary: None,
                });
            }
            let data_clone = data.clone();
            drop(data);
            let _ = crate::db::save_db(&program_path, &data_clone);
        }
        Ok(true)
    } else {
        let err_text = resp.text().await.unwrap_or_default();
        Err(format!("Tracking failed: {}", err_text))
    }
}

/// Untrack a mod on Nexus Mods
#[tauri::command]
pub async fn untrack_nexus_mod(mod_id: u32, state: State<'_, AppState>) -> Result<bool, String> {
    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("Not authenticated with Nexus Mods")?;

    let client = get_client(Some(&token));
    let url = format!("{}/user/tracked_mods.json?domain_name=palworld&mod_id={}", REST_BASE_URL, mod_id);

    let resp = client.delete(&url).send().await.map_err(|e| format!("Failed to untrack mod: {}", e))?;
    if resp.status().is_success() {
        let program_path = {
            let mut data = state.data.lock().map_err(|e| e.to_string())?;
            if let Some(ref mut cache) = data.settings.nexus_tracked_cache {
                cache.retain(|m| m.mod_id != mod_id);
            }
            data.settings.program_path.clone()
        };
        let data_clone = {
            let data = state.data.lock().map_err(|e| e.to_string())?;
            data.clone()
        };
        let _ = crate::db::save_db(&program_path, &data_clone);
        Ok(true)
    } else {
        let err_text = resp.text().await.unwrap_or_default();
        Err(format!("Untracking failed: {}", err_text))
    }
}

/// Download and install a specific file directly from Discovery
#[tauri::command]
pub async fn install_discovery_file(
    mod_id: u32,
    file_id: u64,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await
        .ok_or("Not authenticated with Nexus Mods")?;

    let client = get_client(Some(&token));
    let download_url_endpoint = format!(
        "{}/games/palworld/mods/{}/files/{}/download_link.json",
        REST_BASE_URL, mod_id, file_id
    );

    let resp = client.get(&download_url_endpoint).send().await.map_err(|e| format!("Failed to fetch download link: {}", e))?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to obtain download URL: {}", err_body));
    }

    #[derive(Deserialize)]
    struct DownloadLinkItem {
        #[serde(rename = "URI")]
        uri: Option<String>,
    }

    let links: Vec<DownloadLinkItem> = resp.json().await.map_err(|e| format!("Failed to parse download link: {}", e))?;
    let direct_url = links.into_iter().find_map(|l| l.uri).ok_or("No download link URI provided by Nexus")?;

    Ok(direct_url)
}
