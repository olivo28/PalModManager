use crate::state::AppState;
use tauri::State;

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
    let app_handle_for_install = app_handle.clone();
    let download_id = uuid::Uuid::new_v4().to_string();
    let zip_str = download_nxm_file(nxm_url, download_id, app_handle, state.clone()).await?;
    let temp_path = std::path::PathBuf::from(&zip_str);

    let install_res = crate::commands::install::install_mod_command(
        app_handle_for_install,
        zip_str,
        None,
        None,
        None,
        state,
    ).await;

    let _ = std::fs::remove_file(&temp_path);
    install_res
}
