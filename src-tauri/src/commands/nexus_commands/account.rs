use crate::db;
use crate::state::AppState;
use tauri::State;

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
