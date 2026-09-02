use base64::Engine as _;
use chrono::Utc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use crate::models::NexusAccountInfo;
use crate::protocol_handler;
use super::pkce::{generate_pkce, get_client_id, get_client_secret, get_redirect_uri};
use super::types::OAuthTokenResponse;
use super::profile::fetch_user_profile;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const OAUTH_AUTHORIZE_URL: &str = "https://users.nexusmods.com/oauth/authorize";
pub const OAUTH_TOKEN_URL: &str = "https://users.nexusmods.com/oauth/token";
pub const OAUTH_USERINFO_URL: &str = "https://users.nexusmods.com/oauth/userinfo";

#[derive(Debug, Clone)]
struct PendingOAuthState {
    pub state: String,
    pub code_verifier: String,
    pub created_at: Instant,
}

static PENDING_AUTH: Mutex<Option<PendingOAuthState>> = Mutex::new(None);

/// Start the OAuth flow: registers Windows protocol if needed, generates PKCE, and opens browser
pub fn start_oauth_flow() -> Result<String, String> {
    // 1. Ensure Windows custom URI scheme is registered to this executable
    let _ = protocol_handler::auto_register_if_needed();

    // 2. Generate PKCE & State
    let (verifier, challenge) = generate_pkce();
    let state = uuid::Uuid::new_v4().to_string();

    {
        let mut lock = PENDING_AUTH.lock().map_err(|e| e.to_string())?;
        *lock = Some(PendingOAuthState {
            state: state.clone(),
            code_verifier: verifier,
            created_at: Instant::now(),
        });
    }

    // 3. Build authorization URL
    let client_id = get_client_id();
    let redirect_uri = get_redirect_uri();

    let mut auth_url = url::Url::parse(OAUTH_AUTHORIZE_URL).map_err(|e| e.to_string())?;
    auth_url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid public")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", &state)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", &challenge);

    let url_str = auth_url.to_string();
    crate::logger::log(&format!("start_oauth_flow: Authorize URL: {}", url_str));

    // 4. Open default system browser
    let _ = open::that(&url_str).or_else(|_| {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", &url_str])
                .spawn()
                .map(|_| ())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to open browser"))
        }
    });

    Ok(url_str)
}

/// Helper to extract authorization code and optional state from various input formats:
/// 1. Nexus "HAVING ISSUES?" Base64 encoded payload (starts with eyJ...)
/// 2. Raw JSON payload ({"code": "...", "state": "..."})
/// 3. Deep link / callback URL (palmodmanager://oauth/callback?code=...&state=...)
/// 4. Query string (code=...&state=...)
/// 5. Raw code string
fn extract_code_and_state(input: &str) -> Result<(String, Option<String>), String> {
    let trimmed = input.trim();

    // 1. Try Base64 JSON decode (Nexus Mods "HAVING ISSUES?" code box)
    let b64_engines: [&base64::engine::GeneralPurpose; 4] = [
        &base64::engine::general_purpose::STANDARD,
        &base64::engine::general_purpose::URL_SAFE,
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &base64::engine::general_purpose::STANDARD_NO_PAD,
    ];

    for engine in b64_engines {
        if let Ok(bytes) = engine.decode(trimmed) {
            if let Ok(text) = String::from_utf8(bytes) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    let code = val.get("code")
                        .or_else(|| val.get("authorization_code"))
                        .or_else(|| val.get("data").and_then(|d| d.get("code")))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let state = val.get("state")
                        .or_else(|| val.get("data").and_then(|d| d.get("state")))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(c) = code {
                        return Ok((c, state));
                    }
                }
            }
        }
    }

    // 2. Try raw JSON
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let code = val.get("code")
            .or_else(|| val.get("authorization_code"))
            .or_else(|| val.get("data").and_then(|d| d.get("code")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let state = val.get("state")
            .or_else(|| val.get("data").and_then(|d| d.get("state")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(c) = code {
            return Ok((c, state));
        }
    }

    // 3. Try parsing as full URL
    if let Ok(parsed_url) = url::Url::parse(trimmed) {
        let mut code_opt = None;
        let mut state_opt = None;
        for (k, v) in parsed_url.query_pairs() {
            if k == "code" {
                code_opt = Some(v.to_string());
            } else if k == "state" {
                state_opt = Some(v.to_string());
            }
        }
        if let Some(c) = code_opt {
            return Ok((c, state_opt));
        }
    }

    // 4. Try parsing as query string (e.g. "code=xxx&state=yyy")
    let fake_url = format!("https://dummy.com/?{}", trimmed);
    if let Ok(parsed_url) = url::Url::parse(&fake_url) {
        let mut code_opt = None;
        let mut state_opt = None;
        for (k, v) in parsed_url.query_pairs() {
            if k == "code" {
                code_opt = Some(v.to_string());
            } else if k == "state" {
                state_opt = Some(v.to_string());
            }
        }
        if let Some(c) = code_opt {
            return Ok((c, state_opt));
        }
    }

    // 5. Raw code string (if non-empty and has no whitespace)
    if !trimmed.is_empty() && !trimmed.contains(' ') {
        return Ok((trimmed.to_string(), None));
    }

    Err("Could not extract authorization code from provided input".to_string())
}

/// Parse callback URL / manual code and exchange authorization code for access and refresh tokens
pub async fn handle_oauth_callback(callback_url: &str) -> Result<NexusAccountInfo, String> {
    crate::logger::log(&format!("handle_oauth_callback: Processing URL/Code '{}'", callback_url));

    let (code, state_opt) = extract_code_and_state(callback_url)?;

    // Verify state and retrieve verifier
    let verifier = {
        let mut lock = PENDING_AUTH.lock().map_err(|e| e.to_string())?;
        let pending = lock.take().ok_or("No pending OAuth login session found. Please try logging in again.")?;

        if pending.created_at.elapsed() > Duration::from_secs(600) {
            return Err("OAuth session expired (10 minutes limit). Please try again.".to_string());
        }

        if let Some(ref state) = state_opt {
            if &pending.state != state {
                return Err("OAuth state parameter mismatch (CSRF protection).".to_string());
            }
        }

        pending.code_verifier
    };

    // Exchange code for tokens
    let tokens = exchange_code_for_tokens(&code, &verifier).await?;
    let access_token = tokens.access_token.clone();
    let refresh_token = tokens.refresh_token.clone();
    let expires_in = tokens.expires_in.unwrap_or(3600);
    let token_expires_at = Utc::now().timestamp() + expires_in;

    // Fetch user profile
    let user_info = fetch_user_profile(&access_token).await?;

    let is_premium = user_info.roles.iter().any(|r| {
        let lr = r.to_lowercase();
        lr == "premium" || lr == "lifetimepremium"
    });
    let is_supporter = user_info.roles.iter().any(|r| r.to_lowercase() == "supporter");

    let account_info = NexusAccountInfo {
        user_id: user_info.user_id,
        username: Some(user_info.username),
        avatar_url: user_info.avatar_url,
        is_premium,
        is_supporter,
        roles: user_info.roles,
        access_token: Some(access_token),
        refresh_token,
        token_expires_at: Some(token_expires_at),
        kudos: user_info.kudos,
        profile_views: user_info.profile_views,
        endorsements_given: user_info.endorsements_given,
        joined_date: user_info.joined_date,
        last_active_date: user_info.last_active_date,
        about_me: user_info.about_me,
        mod_count: user_info.mod_count,
    };

    crate::logger::log(&format!(
        "handle_oauth_callback: Successfully authenticated user '{}' (Premium: {}, Kudos: {:?}, Views: {:?})",
        account_info.username.as_deref().unwrap_or("Unknown"),
        account_info.is_premium,
        account_info.kudos,
        account_info.profile_views
    ));

    Ok(account_info)
}

/// Exchange authorization code for token
async fn exchange_code_for_tokens(code: &str, code_verifier: &str) -> Result<OAuthTokenResponse, String> {
    let client_id = get_client_id();
    let client_secret = get_client_secret();
    let redirect_uri = get_redirect_uri();

    let mut params = std::collections::HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("client_id", client_id);
    if !client_secret.is_empty() {
        params.insert("client_secret", client_secret);
    }
    params.insert("code", code);
    params.insert("code_verifier", code_verifier);
    params.insert("redirect_uri", redirect_uri);

    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .post(OAUTH_TOKEN_URL)
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to send token request: {}", e))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        crate::logger::log(&format!("exchange_code_for_tokens: HTTP {} - Body: {}", status, body_text));
        return Err(format!("Token exchange failed (HTTP {}): {}", status, body_text));
    }

    let token_resp: OAuthTokenResponse = serde_json::from_str(&body_text)
        .map_err(|e| format!("Failed to parse token response: {} - Raw: {}", e, body_text))?;

    Ok(token_resp)
}

/// Refreshes an expired access token using the stored refresh_token
pub async fn refresh_access_token(refresh_token: &str) -> Result<OAuthTokenResponse, String> {
    let client_id = get_client_id();
    let client_secret = get_client_secret();

    let mut params = std::collections::HashMap::new();
    params.insert("grant_type", "refresh_token");
    params.insert("client_id", client_id);
    if !client_secret.is_empty() {
        params.insert("client_secret", client_secret);
    }
    params.insert("refresh_token", refresh_token);

    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let resp = client
        .post(OAUTH_TOKEN_URL)
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token refresh request error: {}", e))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Token refresh failed (HTTP {}): {}", status, body_text));
    }

    let token_resp: OAuthTokenResponse = serde_json::from_str(&body_text)
        .map_err(|e| format!("Failed to parse refresh response: {}", e))?;

    Ok(token_resp)
}

/// Automatically ensures the Nexus Mods access token is valid and fresh.
/// If expired or expiring within 120 seconds, automatically refreshes it via refresh_token,
/// updates AppState and saves the local database.
pub async fn ensure_valid_nexus_token(state: &tauri::State<'_, crate::AppState>) -> Option<String> {
    let current_account = {
        let data = state.data.lock().ok()?;
        data.settings.nexus_account.clone()
    };

    let mut account = current_account?;
    let now = chrono::Utc::now().timestamp();

    let access_tok = account.access_token.clone();
    let expires_at = account.token_expires_at;
    let refresh_tok = account.refresh_token.clone();

    // If token is present and still valid (with 2 min buffer), return it immediately
    if let (Some(token), Some(exp)) = (&access_tok, expires_at) {
        if now + 120 < exp {
            return Some(token.clone());
        }
    }

    // Token is expired (or missing expiration) and we have a refresh_token -> perform refresh
    if let Some(ref rt) = refresh_tok {
        crate::logger::log("ensure_valid_nexus_token: Token expired or expiring soon, auto-refreshing in background...");
        match refresh_access_token(rt).await {
            Ok(new_tokens) => {
                let exp_in = new_tokens.expires_in.unwrap_or(3600);
                account.access_token = Some(new_tokens.access_token.clone());
                if let Some(new_rt) = new_tokens.refresh_token {
                    account.refresh_token = Some(new_rt);
                }
                account.token_expires_at = Some(now + exp_in);

                // Persist new token in State and DB
                if let Ok(mut data) = state.data.lock() {
                    data.settings.nexus_account = Some(account.clone());
                    let prog_path = data.settings.program_path.clone();
                    let clone = data.clone();
                    drop(data);
                    let _ = crate::db::save_db(&prog_path, &clone);
                }
                crate::logger::log("ensure_valid_nexus_token: Token auto-refreshed successfully.");
                return Some(new_tokens.access_token);
            }
            Err(e) => {
                crate::logger::log(&format!("ensure_valid_nexus_token: Auto-refresh failed: {}", e));
            }
        }
    }

    access_tok
}

/// Decodes the payload segment of a JWT without external cryptography dependencies
pub fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload_b64 = parts[1];
    let b64_engines: [&base64::engine::GeneralPurpose; 4] = [
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &base64::engine::general_purpose::URL_SAFE,
        &base64::engine::general_purpose::STANDARD_NO_PAD,
        &base64::engine::general_purpose::STANDARD,
    ];
    for engine in b64_engines {
        if let Ok(bytes) = engine.decode(payload_b64) {
            if let Ok(json_str) = String::from_utf8(bytes) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    return Some(val);
                }
            }
        }
    }
    None
}
