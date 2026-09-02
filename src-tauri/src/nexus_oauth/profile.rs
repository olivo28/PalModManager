use std::time::Duration;
use super::flow::{decode_jwt_payload, APP_VERSION, OAUTH_USERINFO_URL};
use super::types::{FetchedProfile, ModBasicInfo};

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
