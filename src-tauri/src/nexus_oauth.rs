use crate::models::NexusAccountInfo;
use crate::protocol_handler;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const OAUTH_AUTHORIZE_URL: &str = "https://users.nexusmods.com/oauth/authorize";
const OAUTH_TOKEN_URL: &str = "https://users.nexusmods.com/oauth/token";
const OAUTH_USERINFO_URL: &str = "https://users.nexusmods.com/oauth/userinfo";

pub fn get_client_id() -> &'static str {
    option_env!("NEXUS_CLIENT_ID").unwrap_or("pal_mod_manager")
}

pub fn get_client_secret() -> &'static str {
    option_env!("NEXUS_CLIENT_SECRET").unwrap_or("")
}

pub fn get_redirect_uri() -> &'static str {
    option_env!("NEXUS_REDIRECT_URI").unwrap_or("palmodmanager://oauth/callback")
}

#[derive(Debug, Clone)]
struct PendingOAuthState {
    pub state: String,
    pub code_verifier: String,
    pub created_at: Instant,
}

static PENDING_AUTH: Mutex<Option<PendingOAuthState>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusUserInfoResponse {
    pub sub: Option<serde_json::Value>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub membership_roles: Option<Vec<String>>,
    pub premium_expiry: Option<serde_json::Value>,
    pub user_id: Option<u64>,
    pub user: Option<NexusUserNested>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusUserNested {
    pub id: Option<u64>,
    pub username: Option<String>,
    pub membership_roles: Option<Vec<String>>,
    pub premium_expiry: Option<serde_json::Value>,
}

/// Generate a cryptographically random code_verifier and S256 code_challenge
pub fn generate_pkce() -> (String, String) {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
    let verifier = URL_SAFE_NO_PAD.encode(&random_bytes);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(&hash);

    (verifier, challenge)
}

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

pub struct FetchedProfile {
    pub user_id: Option<u64>,
    pub username: String,
    pub avatar_url: Option<String>,
    pub roles: Vec<String>,
    pub kudos: Option<u32>,
    pub profile_views: Option<u32>,
    pub endorsements_given: Option<u32>,
    pub joined_date: Option<String>,
    pub last_active_date: Option<String>,
    pub about_me: Option<String>,
    pub mod_count: Option<u32>,
}

/// Fetch complete user profile information:
/// 1. Directly from the decoded JWT access_token claims
/// 2. From Nexus Mods GraphQL API v2 (user / userByName queries)
/// 3. From OpenID /oauth/userinfo
pub async fn fetch_user_profile(access_token: &str) -> Result<FetchedProfile, String> {
    let mut username = String::new();
    let mut avatar_url: Option<String> = None;
    let mut roles: Vec<String> = Vec::new();
    let mut user_id: Option<u64> = None;
    let mut kudos: Option<u32> = None;
    let mut profile_views: Option<u32> = None;
    let mut endorsements_given: Option<u32> = None;
    let mut joined_date: Option<String> = None;
    let mut last_active_date: Option<String> = None;
    let mut about_me: Option<String> = None;
    let mut mod_count: Option<u32> = None;

    // 0. Extract user info directly from the JWT access_token payload
    if let Some(jwt_payload) = decode_jwt_payload(access_token) {
        crate::logger::log(&format!("fetch_user_profile: Decoded JWT payload: {}", jwt_payload));

        if let Some(user_obj) = jwt_payload.get("user") {
            if let Some(n) = user_obj.get("username").or_else(|| user_obj.get("name")).and_then(|v| v.as_str()) {
                username = n.to_string();
            }
            if let Some(id) = user_obj.get("id").or_else(|| user_obj.get("user_id")).and_then(|v| v.as_u64()) {
                user_id = Some(id);
            }
            if let Some(r_arr) = user_obj.get("membership_roles").and_then(|v| v.as_array()) {
                for r in r_arr {
                    if let Some(s) = r.as_str() {
                        roles.push(s.to_string());
                    }
                }
            }
            if let Some(av) = user_obj.get("avatar").or_else(|| user_obj.get("picture")).and_then(|v| v.as_str()) {
                if !av.is_empty() {
                    avatar_url = Some(av.to_string());
                }
            }
        }

        if username.is_empty() {
            if let Some(n) = jwt_payload.get("name").or_else(|| jwt_payload.get("username")).and_then(|v| v.as_str()) {
                username = n.to_string();
            }
        }
        if user_id.is_none() {
            if let Some(sub) = jwt_payload.get("sub").and_then(|v| {
                if let Some(s) = v.as_str() { s.parse::<u64>().ok() } else { v.as_u64() }
            }) {
                user_id = Some(sub);
            }
        }
    }

    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    // 1. Fetch rich profile via Nexus Mods GraphQL v2 API
    // We try query GetUser($id: Int!) first if user_id is known, or fallback to GetUserByName($name: String!)
    let mut gql_success = false;

    let queries_to_try: Vec<(&str, serde_json::Value)> = {
        let mut q = Vec::new();
        if let Some(uid) = user_id {
            q.push((
                "query GetUser($id: Int!) { user(id: $id) { memberId name avatar about kudos views endorsementsGiven joined lastActive modCount membershipRoles roles } }",
                serde_json::json!({ "id": uid })
            ));
        }
        if !username.is_empty() {
            q.push((
                "query GetUserByName($name: String!) { userByName(name: $name) { memberId name avatar about kudos views endorsementsGiven joined lastActive modCount membershipRoles roles } }",
                serde_json::json!({ "name": username })
            ));
        }
        q
    };

    for (gql_query, gql_vars) in queries_to_try {
        if gql_success {
            break;
        }

        let gql_body = serde_json::json!({
            "query": gql_query,
            "variables": gql_vars
        });

        crate::logger::log(&format!("fetch_user_profile: Sending GraphQL query with vars: {}", gql_vars));

        match client
            .post("https://api.nexusmods.com/v2/graphql")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Application-Name", "PalModManager")
            .header("Application-Version", APP_VERSION)
            .json(&gql_body)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                crate::logger::log(&format!("fetch_user_profile (graphql): HTTP {} - Body: {}", status, body_text));

                if status.is_success() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_text) {
                        let user_node = val.get("data")
                            .and_then(|d| d.get("user").or_else(|| d.get("userByName")))
                            .filter(|u| !u.is_null());

                        if let Some(user) = user_node {
                            gql_success = true;
                            if let Some(n) = user.get("name").and_then(|v| v.as_str()) {
                                if !n.is_empty() { username = n.to_string(); }
                            }
                            if let Some(mid) = user.get("memberId").and_then(|v| v.as_u64()) {
                                user_id = Some(mid);
                            }
                            if let Some(av) = user.get("avatar").and_then(|v| v.as_str()) {
                                if !av.is_empty() { avatar_url = Some(av.to_string()); }
                            }
                            if let Some(ab) = user.get("about").and_then(|v| v.as_str()) {
                                if !ab.is_empty() { about_me = Some(ab.to_string()); }
                            }
                            if let Some(k) = user.get("kudos").and_then(|v| v.as_u64()) {
                                kudos = Some(k as u32);
                            }
                            if let Some(v) = user.get("views").and_then(|v| v.as_u64()) {
                                profile_views = Some(v as u32);
                            }
                            if let Some(e) = user.get("endorsementsGiven").and_then(|v| v.as_u64()) {
                                endorsements_given = Some(e as u32);
                            }
                            if let Some(j) = user.get("joined").and_then(|v| v.as_str()) {
                                joined_date = Some(j.to_string());
                            }
                            if let Some(la) = user.get("lastActive").and_then(|v| v.as_str()) {
                                last_active_date = Some(la.to_string());
                            }
                            if let Some(mc) = user.get("modCount").and_then(|v| v.as_u64()) {
                                mod_count = Some(mc as u32);
                            }
                            if let Some(r_arr) = user.get("membershipRoles").or_else(|| user.get("roles")).and_then(|v| v.as_array()) {
                                for r in r_arr {
                                    if let Some(s) = r.as_str() {
                                        if !roles.contains(&s.to_string()) {
                                            roles.push(s.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                crate::logger::log(&format!("fetch_user_profile GraphQL request error: {}", e));
            }
        }
    }

    // 2. Try OpenID userinfo endpoint fallback
    if avatar_url.is_none() {
        if let Ok(resp) = client
            .get(OAUTH_USERINFO_URL)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Application-Name", "PalModManager")
            .header("Application-Version", APP_VERSION)
            .send()
            .await
        {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            crate::logger::log(&format!("fetch_user_profile (userinfo): HTTP {} - Body: {}", status, body_text));

            if status.is_success() {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body_text) {
                    if username.is_empty() {
                        if let Some(n) = parsed.get("name").or_else(|| parsed.get("username")).and_then(|v| v.as_str()) {
                            username = n.to_string();
                        }
                    }
                    if avatar_url.is_none() {
                        if let Some(av) = parsed.get("avatar").or_else(|| parsed.get("picture")).and_then(|v| v.as_str()) {
                            if !av.is_empty() {
                                avatar_url = Some(av.to_string());
                            }
                        }
                    }
                    if user_id.is_none() {
                        if let Some(sub) = parsed.get("sub").and_then(|v| {
                            if let Some(s) = v.as_str() { s.parse::<u64>().ok() } else { v.as_u64() }
                        }) {
                            user_id = Some(sub);
                        }
                    }
                }
            }
        }
    }

    if username.is_empty() {
        if let Some(uid) = user_id {
            username = format!("Nexus User #{}", uid);
        } else {
            username = "Nexus User".to_string();
        }
    }

    Ok(FetchedProfile {
        user_id,
        username,
        avatar_url,
        roles,
        kudos,
        profile_views,
        endorsements_given,
        joined_date,
        last_active_date,
        about_me,
        mod_count,
    })
}

/// Fetch user endorsements from Nexus Mods REST API v1
pub async fn fetch_user_endorsements(access_token: &str) -> Result<Vec<crate::models::NexusUserEndorsement>, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let resp = client
        .get("https://api.nexusmods.com/v1/user/endorsements.json")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch endorsements: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    crate::logger::log(&format!("fetch_user_endorsements: HTTP {} - Body len: {}", status, body.len()));

    if !status.is_success() {
        return Err(format!("Nexus API returned HTTP {}: {}", status, body));
    }

    let items: Vec<crate::models::NexusUserEndorsement> = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse endorsements JSON: {}", e))?;

    Ok(items)
}

/// Fetch user tracked mods from Nexus Mods REST API v1
pub async fn fetch_user_tracked_mods(access_token: &str) -> Result<Vec<crate::models::NexusUserTrackedMod>, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let resp = client
        .get("https://api.nexusmods.com/v1/user/tracked_mods.json")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch tracked mods: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    crate::logger::log(&format!("fetch_user_tracked_mods: HTTP {} - Body len: {}", status, body.len()));

    if !status.is_success() {
        return Err(format!("Nexus API returned HTTP {}: {}", status, body));
    }

    let items: Vec<crate::models::NexusUserTrackedMod> = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse tracked mods JSON: {}", e))?;

    Ok(items)
}

/// Fetch mods created/published by the user on Nexus Mods via GraphQL v2
pub async fn fetch_user_authored_mods(access_token: &str, user_id: u64) -> Result<Vec<crate::models::NexusUserAuthoredMod>, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let gql_query = r#"
query GetUserAuthoredMods($userId: String!) {
  mods(filter: { uploaderId: [{ value: $userId }] }, count: 50) {
    nodes {
      modId
      name
      summary
      version
      downloads
      endorsements
      pictureUrl
      game {
        domainName
        name
      }
      createdAt
      updatedAt
    }
  }
}
"#;

    let payload = serde_json::json!({
        "query": gql_query,
        "variables": {
            "userId": user_id.to_string()
        }
    });

    let resp = client
        .post("https://api.nexusmods.com/v2/graphql")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch user authored mods: {}", e))?;

    let body = resp.text().await.unwrap_or_default();
    crate::logger::log(&format!("fetch_user_authored_mods: body: {}", body));

    let mut result = Vec::new();
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Some(nodes) = val.get("data")
            .and_then(|d| d.get("mods"))
            .and_then(|m| m.get("nodes"))
            .and_then(|n| n.as_array())
        {
            for node in nodes {
                let mod_id = node.get("modId").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                if mod_id == 0 { continue; }
                let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed Mod").to_string();
                let summary = node.get("summary").and_then(|v| v.as_str()).map(|s| s.to_string());
                let version = node.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
                let downloads = node.get("downloads").and_then(|v| v.as_u64()).map(|d| d as u32);
                let endorsements = node.get("endorsements").and_then(|v| v.as_u64()).map(|e| e as u32);
                let picture_url = node.get("pictureUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
                let game_name = node.get("game").and_then(|g| g.get("name")).and_then(|v| v.as_str()).map(|s| s.to_string());
                let domain_name = node.get("game").and_then(|g| g.get("domainName")).and_then(|v| v.as_str()).map(|s| s.to_string());
                let created_at = node.get("createdAt").and_then(|v| v.as_str()).map(|s| s.to_string());
                let updated_at = node.get("updatedAt").and_then(|v| v.as_str()).map(|s| s.to_string());

                result.push(crate::models::NexusUserAuthoredMod {
                    mod_id,
                    name,
                    summary,
                    version,
                    downloads,
                    endorsements,
                    picture_url,
                    game_name,
                    domain_name,
                    created_at,
                    updated_at,
                });
            }
        }
    }

    Ok(result)
}

#[derive(Debug, Clone)]
pub struct ModBasicInfo {
    pub name: String,
    pub summary: Option<String>,
    pub picture_url: Option<String>,
    pub version: Option<String>,
}

/// Batch fetch details for a list of (domain, mod_id) tuples from GraphQL v2
pub async fn batch_fetch_legacy_mods_info(
    domain_and_ids: &[(&str, u32)],
    access_token: Option<&str>,
) -> std::collections::HashMap<(String, u32), ModBasicInfo> {
    let mut map = std::collections::HashMap::new();
    if domain_and_ids.is_empty() {
        return map;
    }

    let client = match reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build() {
            Ok(c) => c,
            Err(_) => return map,
        };

    let ids_payload: Vec<serde_json::Value> = domain_and_ids
        .iter()
        .map(|(domain, mod_id)| serde_json::json!({
            "gameDomain": domain,
            "modId": mod_id
        }))
        .collect();

    let gql_query = r#"
query LegacyModsByDomain($ids: [CompositeDomainWithIdInput!]!) {
  legacyModsByDomain(ids: $ids) {
    nodes {
      modId
      name
      summary
      pictureUrl
      version
      game {
        domainName
      }
    }
  }
}
"#;

    let payload = serde_json::json!({
        "query": gql_query,
        "variables": {
            "ids": ids_payload
        }
    });

    let mut req = client.post("https://api.nexusmods.com/v2/graphql")
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION);

    if let Some(token) = access_token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    if let Ok(resp) = req.json(&payload).send().await {
        let body = resp.text().await.unwrap_or_default();
        crate::logger::log(&format!("legacyModsByDomain HTTP response len: {}", body.len()));

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(nodes) = val.get("data")
                .and_then(|d| d.get("legacyModsByDomain"))
                .and_then(|l| l.get("nodes"))
                .and_then(|n| n.as_array())
            {
                for node in nodes {
                    let mod_id = node.get("modId").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    if mod_id == 0 { continue; }
                    let domain = node.get("game")
                        .and_then(|g| g.get("domainName"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();

                    let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let summary = node.get("summary").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let picture_url = node.get("pictureUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let version = node.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());

                    map.insert((domain, mod_id), ModBasicInfo {
                        name,
                        summary,
                        picture_url,
                        version,
                    });
                }
            }
        }
    }

    map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxmLinkInfo {
    pub raw_url: String,
    pub game_domain: String,
    pub mod_id: u32,
    pub file_id: u64,
    pub query: String,
}

/// Parse and validate an nxm:// URL
pub fn parse_nxm_url(raw_url: &str) -> Result<NxmLinkInfo, String> {
    let clean = raw_url.trim().trim_matches('"');
    let parsed = url::Url::parse(clean).map_err(|e| format!("Invalid NXM link: {}", e))?;

    if parsed.scheme() != "nxm" {
        return Err(format!("Invalid URL scheme '{}' (expected 'nxm')", parsed.scheme()));
    }

    // Host might be the game domain, or the path could contain it
    let host = parsed.host_str().unwrap_or("").to_lowercase();
    let path_segments: Vec<&str> = parsed.path_segments().map(|s| s.collect()).unwrap_or_default();

    let mut game_domain = host.clone();
    let mut mod_id = 0u32;
    let mut file_id = 0u64;

    // Cases:
    // 1. nxm://palworld/mods/5321/files/23121?key=...
    // 2. nxm:///palworld/mods/5321/files/23121?key=...
    if !host.is_empty() && host != "mods" {
        game_domain = host;
        let mut i = 0;
        while i < path_segments.len() {
            if path_segments[i] == "mods" && i + 1 < path_segments.len() {
                mod_id = path_segments[i + 1].parse().unwrap_or(0);
                i += 2;
            } else if path_segments[i] == "files" && i + 1 < path_segments.len() {
                file_id = path_segments[i + 1].parse().unwrap_or(0);
                i += 2;
            } else {
                i += 1;
            }
        }
    } else {
        let mut i = 0;
        if !path_segments.is_empty() && path_segments[0] != "mods" {
            game_domain = path_segments[0].to_lowercase();
            i = 1;
        }
        while i < path_segments.len() {
            if path_segments[i] == "mods" && i + 1 < path_segments.len() {
                mod_id = path_segments[i + 1].parse().unwrap_or(0);
                i += 2;
            } else if path_segments[i] == "files" && i + 1 < path_segments.len() {
                file_id = path_segments[i + 1].parse().unwrap_or(0);
                i += 2;
            } else {
                i += 1;
            }
        }
    }

    if game_domain != "palworld" {
        return Err(format!("PalModManager only handles Palworld mods (received link for '{}')", game_domain));
    }

    if mod_id == 0 || file_id == 0 {
        return Err(format!("Invalid NXM link structure: missing mod ID ({}) or file ID ({})", mod_id, file_id));
    }

    let query = parsed.query().unwrap_or("").to_string();

    Ok(NxmLinkInfo {
        raw_url: raw_url.to_string(),
        game_domain,
        mod_id,
        file_id,
        query,
    })
}

/// Request direct CDN download URL from Nexus Mods REST API v1
pub async fn fetch_nxm_direct_download_url(access_token: &str, nxm: &NxmLinkInfo) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let api_url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}/files/{}/download_link.json?{}",
        nxm.game_domain, nxm.mod_id, nxm.file_id, nxm.query
    );

    crate::logger::log(&format!("fetch_nxm_direct_download_url: Calling {}", api_url));

    let resp = client.get(&api_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .send()
        .await
        .map_err(|e| format!("Failed to request download link: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Nexus Mods API error ({}): {}", status, body));
    }

    let val: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Nexus download response: {}", e))?;

    if let Some(arr) = val.as_array() {
        if let Some(first) = arr.first() {
            if let Some(uri) = first.get("URI").and_then(|u| u.as_str()) {
                return Ok(uri.to_string());
            }
        }
    }

    Err("No valid CDN download URI found in Nexus response.".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxmModMetadata {
    pub mod_id: u32,
    pub name: String,
    pub summary: Option<String>,
    pub picture_url: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxmDownloadProgressEvent {
    pub download_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f32,
}

/// Fetch mod metadata (name, summary, picture) from Nexus REST API v1
pub async fn fetch_nxm_mod_metadata(
    access_token: &str,
    game_domain: &str,
    mod_id: u32,
) -> Result<NxmModMetadata, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let api_url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}.json",
        game_domain, mod_id
    );

    let resp = client.get(&api_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .send()
        .await
        .map_err(|e| format!("Failed to request mod metadata: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Nexus metadata error: {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await
        .map_err(|e| format!("Failed to parse mod metadata: {}", e))?;

    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Mod").to_string();
    let summary = val.get("summary").and_then(|v| v.as_str()).map(|s| s.to_string());
    let picture_url = val.get("picture_url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let version = val.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
    let author = val.get("author").and_then(|v| v.as_str()).map(|s| s.to_string());

    Ok(NxmModMetadata {
        mod_id,
        name,
        summary,
        picture_url,
        version,
        author,
    })
}

/// Download file from CDN and save as a temporary zip file with progress events
pub async fn download_file_to_temp_with_progress(
    app_handle: &tauri::AppHandle,
    download_url: &str,
    file_id: u64,
    download_id: &str,
) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    use tauri::Emitter;

    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let mut resp = client.get(download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to CDN: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("CDN returned status {}", resp.status()));
    }

    let total_bytes = resp.content_length();
    let temp_dir = std::env::temp_dir().join("PalModManager_Downloads");
    let _ = std::fs::create_dir_all(&temp_dir);
    let target_file = temp_dir.join(format!("nexus_{}_{}.zip", file_id, uuid::Uuid::new_v4()));

    let mut file = std::fs::File::create(&target_file)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;

    let mut downloaded_bytes = 0u64;

    // Initial 0% event
    let _ = app_handle.emit("nxm-download-progress", NxmDownloadProgressEvent {
        download_id: download_id.to_string(),
        bytes_downloaded: 0,
        total_bytes,
        percentage: 0.0,
    });

    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Download stream error: {}", e))? {
        file.write_all(&chunk).map_err(|e| format!("Failed to write data chunk: {}", e))?;
        downloaded_bytes += chunk.len() as u64;

        let percentage = match total_bytes {
            Some(total) if total > 0 => ((downloaded_bytes as f64 / total as f64) * 100.0) as f32,
            _ => 0.0,
        };

        let _ = app_handle.emit("nxm-download-progress", NxmDownloadProgressEvent {
            download_id: download_id.to_string(),
            bytes_downloaded: downloaded_bytes,
            total_bytes,
            percentage,
        });
    }

    // Flush and release file handle before returning
    file.flush().map_err(|e| format!("Failed to flush temporary file: {}", e))?;
    drop(file);

    // Final 100% event
    let _ = app_handle.emit("nxm-download-progress", NxmDownloadProgressEvent {
        download_id: download_id.to_string(),
        bytes_downloaded: downloaded_bytes,
        total_bytes: Some(downloaded_bytes),
        percentage: 100.0,
    });

    Ok(target_file)
}

/// Download file from CDN and save as a temporary zip file (legacy fallback)
pub async fn download_file_to_temp(download_url: &str, file_id: u64) -> Result<std::path::PathBuf, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to CDN: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("CDN returned status {}", resp.status()));
    }

    let temp_dir = std::env::temp_dir().join("PalModManager_Downloads");
    let _ = std::fs::create_dir_all(&temp_dir);
    let target_file = temp_dir.join(format!("nexus_{}_{}.zip", file_id, uuid::Uuid::new_v4()));

    let bytes = resp.bytes().await.map_err(|e| format!("Failed to download file stream: {}", e))?;
    std::fs::write(&target_file, bytes).map_err(|e| format!("Failed to write downloaded file: {}", e))?;

    Ok(target_file)
}



