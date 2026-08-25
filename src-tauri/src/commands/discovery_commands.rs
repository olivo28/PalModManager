use crate::state::AppState;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const GRAPHQL_ENDPOINT: &str = "https://api.nexusmods.com/v2/graphql";
const REST_BASE_URL: &str = "https://api.nexusmods.com/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryModItem {
    pub mod_id: u32,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub version: String,
    pub downloads: u32,
    pub endorsements: u32,
    pub picture_url: String,
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub contains_adult_content: bool,
    pub is_endorsed: bool,
    pub is_tracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCategory {
    pub category_id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryFileItem {
    pub file_id: u64,
    pub name: String,
    pub version: String,
    pub category_id: u32,
    pub category_name: String,
    pub is_primary: bool,
    pub size_in_bytes: u64,
    pub size_formatted: String,
    pub uploaded_at: String,
    pub uploaded_timestamp: Option<i64>,
    pub description: String,
    pub unique_downloads: Option<u32>,
    pub total_downloads: Option<u32>,
    pub scan_status: Option<String>,
    pub changelog_entries: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryModDetails {
    pub mod_id: u32,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub downloads: u32,
    pub endorsements: u32,
    pub picture_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub contains_adult_content: bool,
    pub files: Vec<DiscoveryFileItem>,
    pub images: Vec<String>,
    pub is_endorsed: bool,
    pub is_tracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResponse {
    pub mods: Vec<DiscoveryModItem>,
    pub total_count: u32,
    pub page: u32,
    pub page_size: u32,
}

fn get_client(token: Option<&str>) -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Application-Name",
        HeaderValue::from_static("PalModManager"),
    );
    headers.insert(
        "Application-Version",
        HeaderValue::from_static(APP_VERSION),
    );
    if let Some(t) = token {
        if !t.is_empty() {
            if let Ok(hv) = HeaderValue::from_str(&format!("Bearer {}", t)) {
                headers.insert(AUTHORIZATION, hv);
            }
        }
    }
    reqwest::Client::builder()
        .default_headers(headers)
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Query categories for Palworld
#[tauri::command]
pub async fn get_discovery_categories(state: State<'_, AppState>) -> Result<Vec<DiscoveryCategory>, String> {
    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await;

    let client = get_client(token.as_deref());
    let url = format!("{}/games/palworld/categories.json", REST_BASE_URL);
    let mut resp = client.get(&url).send().await;

    if let Ok(ref r) = resp {
        if r.status() == StatusCode::UNAUTHORIZED || r.status().as_u16() == 402 {
            let anon_client = get_client(None);
            resp = anon_client.get(&url).send().await;
        }
    }

    let resp = resp.map_err(|e| format!("Failed to fetch categories: {}", e))?;

    let full_default_categories = vec![
        DiscoveryCategory { category_id: 1, name: "Animations".to_string() },
        DiscoveryCategory { category_id: 2, name: "Audio".to_string() },
        DiscoveryCategory { category_id: 3, name: "Characters".to_string() },
        DiscoveryCategory { category_id: 4, name: "Gameplay".to_string() },
        DiscoveryCategory { category_id: 5, name: "Miscellaneous".to_string() },
        DiscoveryCategory { category_id: 6, name: "Outfits".to_string() },
        DiscoveryCategory { category_id: 7, name: "Pals".to_string() },
        DiscoveryCategory { category_id: 8, name: "Palworld".to_string() },
        DiscoveryCategory { category_id: 9, name: "Scripts".to_string() },
        DiscoveryCategory { category_id: 10, name: "User Interface".to_string() },
        DiscoveryCategory { category_id: 11, name: "Utilities".to_string() },
        DiscoveryCategory { category_id: 12, name: "Visuals".to_string() },
        DiscoveryCategory { category_id: 13, name: "Weapons".to_string() },
    ];

    if !resp.status().is_success() {
        return Ok(full_default_categories);
    }

    #[derive(Deserialize)]
    struct RestCategoryItem {
        category_id: u32,
        name: String,
    }

    let items: Vec<RestCategoryItem> = resp.json().await.unwrap_or_default();
    if items.is_empty() {
        return Ok(full_default_categories);
    }

    let categories = items
        .into_iter()
        .map(|c| DiscoveryCategory {
            category_id: c.category_id,
            name: c.name,
        })
        .collect();

    Ok(categories)
}

/// Fetch list of mods for the discovery tab with search, sorting, time range, and adult content filters
#[tauri::command]
pub async fn get_discovery_mods(
    query: Option<String>,
    description_query: Option<String>,
    author_query: Option<String>,
    uploader_query: Option<String>,
    category_id: Option<u32>,
    category_name: Option<String>,
    adult_filter: Option<String>,
    supports_vortex: Option<bool>,
    has_updated: Option<bool>,
    language_name: Option<String>,
    language_names: Option<Vec<String>>,
    include_tags: Option<Vec<String>>,
    exclude_tags: Option<Vec<String>>,
    hide_translations: Option<bool>,
    sort_by: Option<String>,
    time_range: Option<String>,
    include_adult: Option<bool>,
    page: Option<u32>,
    page_size: Option<u32>,
    state: State<'_, AppState>,
) -> Result<DiscoveryResponse, String> {
    let current_page = page.unwrap_or(1).max(1);
    let limit = page_size.unwrap_or(36).clamp(6, 100);
    let offset = (current_page - 1) * limit;
    let sort = sort_by.unwrap_or_else(|| "date_published".to_string());
    let allow_adult = include_adult.unwrap_or(false);

    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await;
    let client = get_client(token.as_deref());

    // 1. Try GraphQL v2 query
    let gql_sort = match sort.as_str() {
        "date_published" | "newest" | "latest_added" => "[{ createdAt: { direction: DESC } }]",
        "endorsements" => "[{ endorsements: { direction: DESC } }]",
        "downloads" => "[{ downloads: { direction: DESC } }]",
        "unique_downloads" => "[{ downloads: { direction: DESC } }]",
        "updated" | "last_updated" | "latest_updated" => "[{ updatedAt: { direction: DESC } }]",
        "name" | "mod_name" => "[{ name: { direction: ASC } }]",
        "file_size" => "[{ downloads: { direction: DESC } }]",
        "last_comment" => "[{ updatedAt: { direction: DESC } }]",
        "surprise" => "[{ createdAt: { direction: ASC } }]",
        _ => "[{ createdAt: { direction: DESC } }]",
    };

    let mut filter_obj = serde_json::json!({
        "gameId": [{ "value": "6063" }]
    });

    if let Some(ref q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            filter_obj["nameStemmed"] = serde_json::json!([{ "value": trimmed }]);
        }
    }

    if let Some(ref d) = description_query {
        let trimmed = d.trim();
        if !trimmed.is_empty() {
            filter_obj["description"] = serde_json::json!([{ "value": trimmed, "op": "MATCHES" }]);
        }
    }

    if let Some(ref a) = author_query {
        let trimmed = a.trim();
        if !trimmed.is_empty() {
            filter_obj["author"] = serde_json::json!([{ "value": trimmed }]);
        }
    }

    if let Some(ref u) = uploader_query {
        let trimmed = u.trim();
        if !trimmed.is_empty() {
            filter_obj["uploader"] = serde_json::json!([{ "value": trimmed }]);
        }
    }

    if let Some(ref af) = adult_filter {
        match af.as_str() {
            "hide" => {
                filter_obj["adultContent"] = serde_json::json!([{ "value": false }]);
            }
            "only" => {
                filter_obj["adultContent"] = serde_json::json!([{ "value": true }]);
            }
            _ => {}
        }
    } else if !allow_adult {
        filter_obj["adultContent"] = serde_json::json!([{ "value": false }]);
    }

    if let Some(vortex) = supports_vortex {
        if vortex {
            filter_obj["supportsVortex"] = serde_json::json!([{ "value": true }]);
        }
    }

    if let Some(updated) = has_updated {
        if updated {
            filter_obj["hasUpdated"] = serde_json::json!([{ "value": true }]);
        }
    }

    if let Some(ref tags) = include_tags {
        let clean_tags: Vec<&str> = tags.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if !clean_tags.is_empty() {
            let tag_arr: Vec<_> = clean_tags.iter().map(|t| serde_json::json!({ "value": t })).collect();
            filter_obj["tag"] = serde_json::json!(tag_arr);
        }
    }

    if let Some(ref langs) = language_names {
        let clean_langs: Vec<&str> = langs.iter().map(|s| s.trim()).filter(|s| !s.is_empty() && *s != "all").collect();
        if !clean_langs.is_empty() {
            let lang_arr: Vec<_> = clean_langs.iter().map(|l| serde_json::json!({ "value": l })).collect();
            filter_obj["languageName"] = serde_json::json!(lang_arr);
        }
    } else if let Some(ref lang) = language_name {
        let trimmed = lang.trim();
        if !trimmed.is_empty() && trimmed != "all" {
            filter_obj["languageName"] = serde_json::json!([{ "value": trimmed }]);
        }
    }

    if let Some(ref cat_n) = category_name {
        let trimmed = cat_n.trim();
        if !trimmed.is_empty() && trimmed != "All Categories" {
            filter_obj["categoryName"] = serde_json::json!([{ "value": trimmed }]);
        }
    }

    if let Some(ref cat_n) = category_name {
        let trimmed = cat_n.trim();
        if !trimmed.is_empty() && trimmed != "All Categories" {
            filter_obj["categoryName"] = serde_json::json!([{ "value": trimmed }]);
        }
    }

    let gql_query = format!(
        r#"
query GetPalworldDiscovery($filter: ModsFilter, $count: Int, $offset: Int) {{
  mods(filter: $filter, sort: {}, count: $count, offset: $offset) {{
    nodes {{
      modId
      name
      summary
      author
      version
      downloads
      endorsements
      pictureUrl
      createdAt
      updatedAt
      category
    }}
    totalCount
  }}
}}
"#,
        gql_sort
    );

    let gql_payload = serde_json::json!({
        "query": gql_query,
        "variables": {
            "filter": filter_obj,
            "count": limit,
            "offset": offset,
        }
    });

    let mut resp = client
        .post(GRAPHQL_ENDPOINT)
        .header(CONTENT_TYPE, "application/json")
        .json(&gql_payload)
        .send()
        .await;

    if let Ok(ref r) = resp {
        if r.status() == StatusCode::UNAUTHORIZED || r.status().as_u16() == 402 {
            let anon_client = get_client(None);
            resp = anon_client
                .post(GRAPHQL_ENDPOINT)
                .header(CONTENT_TYPE, "application/json")
                .json(&gql_payload)
                .send()
                .await;
        }
    }

    if let Ok(response) = resp {
        if response.status().is_success() {
            #[derive(Deserialize)]
            struct GqlNode {
                #[serde(rename = "modId")]
                mod_id: Option<u32>,
                name: Option<String>,
                summary: Option<String>,
                author: Option<String>,
                version: Option<String>,
                downloads: Option<u32>,
                endorsements: Option<u32>,
                #[serde(rename = "pictureUrl")]
                picture_url: Option<String>,
                #[serde(rename = "createdAt")]
                created_at: Option<String>,
                #[serde(rename = "updatedAt")]
                updated_at: Option<String>,
                category: Option<String>,
            }
            #[derive(Deserialize)]
            struct GqlModsData {
                nodes: Option<Vec<GqlNode>>,
                #[serde(rename = "totalCount")]
                total_count: Option<u32>,
            }
            #[derive(Deserialize)]
            struct GqlResponseData {
                mods: Option<GqlModsData>,
            }
            #[derive(Deserialize)]
            struct GqlWrapper {
                data: Option<GqlResponseData>,
            }

            if let Ok(parsed) = response.json::<GqlWrapper>().await {
                if let Some(d) = parsed.data {
                    if let Some(mods_data) = d.mods {
                        let total = mods_data.total_count.unwrap_or(0);
                        if let Some(nodes) = mods_data.nodes {
                            if !nodes.is_empty() {
                                let mut items: Vec<DiscoveryModItem> = nodes
                                    .into_iter()
                                    .filter_map(|n| {
                                        let mid = n.mod_id?;
                                        let name = n.name.unwrap_or_default();
                                        let summary = n.summary.unwrap_or_default();
                                        let cat = n.category.clone().unwrap_or_default();
                                        
                                        let is_nsfw = cat.to_lowercase().contains("adult")
                                            || cat.to_lowercase().contains("nsfw")
                                            || name.to_lowercase().contains("nsfw")
                                            || name.to_lowercase().contains("nude")
                                            || name.to_lowercase().contains("18+")
                                            || summary.to_lowercase().contains("nsfw");

                                        if !allow_adult && is_nsfw {
                                            return None;
                                        }

                                        Some(DiscoveryModItem {
                                            mod_id: mid,
                                            name,
                                            summary,
                                            author: n.author.unwrap_or_else(|| "Unknown".to_string()),
                                            version: n.version.unwrap_or_else(|| "1.0".to_string()),
                                            downloads: n.downloads.unwrap_or(0),
                                            endorsements: n.endorsements.unwrap_or(0),
                                            picture_url: n.picture_url.unwrap_or_default(),
                                            category_id: None,
                                            category_name: n.category,
                                            created_at: n.created_at.unwrap_or_default(),
                                            updated_at: n.updated_at.unwrap_or_default(),
                                            contains_adult_content: is_nsfw,
                                            is_endorsed: false,
                                            is_tracked: false,
                                        })
                                    })
                                    .collect();

                                if let Some(ref tr) = time_range {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64;
                                    let max_age_secs = match tr.as_str() {
                                        "24h" => 86400,
                                        "7d" => 7 * 86400,
                                        "14d" => 14 * 86400,
                                        "28d" => 28 * 86400,
                                        "1y" => 365 * 86400,
                                        _ => 0,
                                    };
                                    if max_age_secs > 0 {
                                        let cutoff = now - max_age_secs;
                                        items.retain(|m| {
                                            let ts = chrono::DateTime::parse_from_rfc3339(&m.created_at)
                                                .map(|dt| dt.timestamp())
                                                .unwrap_or(0);
                                            ts >= cutoff
                                        });
                                    }
                                }

                                return Ok(DiscoveryResponse {
                                    mods: items,
                                    total_count: total,
                                    page: current_page,
                                    page_size: limit,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback: REST API v1
    let endpoint = match sort.as_str() {
        "newest" | "latest_added" | "date_published" => "latest_added.json",
        "updated" | "latest_updated" => "latest_updated.json",
        _ => "trending.json",
    };

    let url = format!("{}/games/palworld/mods/{}", REST_BASE_URL, endpoint);
    let mut rest_resp = client.get(&url).send().await;

    if let Ok(ref r) = rest_resp {
        if r.status() == StatusCode::UNAUTHORIZED || r.status().as_u16() == 402 {
            let anon_client = get_client(None);
            rest_resp = anon_client.get(&url).send().await;
        }
    }

    let rest_resp = rest_resp.map_err(|e| format!("Failed to fetch discovery mods: {}", e))?;

    if !rest_resp.status().is_success() {
        return Err(format!("Nexus API returned HTTP {}", rest_resp.status()));
    }

    #[derive(Deserialize)]
    struct RestModItem {
        mod_id: u32,
        name: String,
        summary: Option<String>,
        author: Option<String>,
        version: Option<String>,
        picture_url: Option<String>,
        endorsement_count: Option<u32>,
        category_id: Option<u32>,
        created_time: Option<String>,
        updated_time: Option<String>,
        created_timestamp: Option<i64>,
        updated_timestamp: Option<i64>,
        contains_adult_content: Option<bool>,
    }

    let all_mods: Vec<RestModItem> = rest_resp.json().await.unwrap_or_default();
    let mut filtered: Vec<RestModItem> = all_mods;

    if !allow_adult {
        filtered.retain(|m| !m.contains_adult_content.unwrap_or(false));
    }

    if let Some(ref tr) = time_range {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let max_age_secs = match tr.as_str() {
            "24h" => 86400,
            "7d" => 7 * 86400,
            "14d" => 14 * 86400,
            "28d" => 28 * 86400,
            "1y" => 365 * 86400,
            _ => 0,
        };
        if max_age_secs > 0 {
            let cutoff = now - max_age_secs;
            filtered.retain(|m| m.created_timestamp.unwrap_or(0) >= cutoff);
        }
    }

    if let Some(ref q) = query {
        let q_lower = q.trim().to_lowercase();
        if !q_lower.is_empty() {
            filtered.retain(|m| {
                m.name.to_lowercase().contains(&q_lower)
                    || m.summary.as_deref().unwrap_or("").to_lowercase().contains(&q_lower)
                    || m.author.as_deref().unwrap_or("").to_lowercase().contains(&q_lower)
            });
        }
    }

    if let Some(cat) = category_id {
        if cat > 0 {
            filtered.retain(|m| m.category_id == Some(cat));
        }
    }

    let total = filtered.len() as u32;
    let paginated: Vec<DiscoveryModItem> = filtered
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|m| DiscoveryModItem {
            mod_id: m.mod_id,
            name: m.name,
            summary: m.summary.unwrap_or_default(),
            author: m.author.unwrap_or_else(|| "Unknown".to_string()),
            version: m.version.unwrap_or_else(|| "1.0".to_string()),
            downloads: 0,
            endorsements: m.endorsement_count.unwrap_or(0),
            picture_url: m.picture_url.unwrap_or_default(),
            category_id: m.category_id,
            category_name: None,
            created_at: m.created_time.unwrap_or_default(),
            updated_at: m.updated_time.unwrap_or_default(),
            contains_adult_content: m.contains_adult_content.unwrap_or(false),
            is_endorsed: false,
            is_tracked: false,
        })
        .collect();

    Ok(DiscoveryResponse {
        mods: paginated,
        total_count: total,
        page: current_page,
        page_size: limit,
    })
}

/// Fetch full details for a single mod in Discovery view
#[tauri::command]
pub async fn get_discovery_mod_details(mod_id: u32, state: State<'_, AppState>) -> Result<DiscoveryModDetails, String> {
    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await;
    let client = get_client(token.as_deref());

    // 1. Fetch Mod Details, Endorsement/Track Status & Files via GraphQL v2
    let gql_query = r#"
query GetModFullDetails($modId: ID!) {
  mod(modId: $modId, gameId: "6063") {
    name
    summary
    description
    author
    version
    downloads
    endorsements
    pictureUrl
    createdAt
    updatedAt
    category
    viewerEndorsed
    viewerTracked
  }
  modFiles(modId: $modId, gameId: "6063") {
    fileId
    name
    version
    category
    categoryId
    primary
    date
    sizeInBytes
    size
    uniqueDownloads
    totalDownloads
    scanned
    scannedV2
    changelogText
    description
  }
}
"#;

    let payload = serde_json::json!({
        "query": gql_query,
        "variables": { "modId": mod_id.to_string() }
    });

    let mut name = String::new();
    let mut summary = String::new();
    let mut description = String::new();
    let mut author = String::new();
    let mut version = "1.0".to_string();
    let mut downloads = 0u32;
    let mut endorsements = 0u32;
    let mut picture_url = String::new();
    let mut created_at = String::new();
    let mut updated_at = String::new();
    let mut category_name: Option<String> = None;
    let mut contains_adult_content = false;
    let mut is_endorsed = false;
    let mut is_tracked = false;
    let mut images: Vec<String> = Vec::new();
    let mut files_list: Vec<DiscoveryFileItem> = Vec::new();

    let mut gql_resp = client
        .post(GRAPHQL_ENDPOINT)
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await;

    if let Ok(ref r) = gql_resp {
        if r.status() == StatusCode::UNAUTHORIZED || r.status().as_u16() == 402 {
            let anon_client = get_client(None);
            gql_resp = anon_client
                .post(GRAPHQL_ENDPOINT)
                .header(CONTENT_TYPE, "application/json")
                .json(&payload)
                .send()
                .await;
        }
    }

    if let Ok(resp) = gql_resp {
        if resp.status().is_success() {
            #[derive(Deserialize)]
            struct GqlFile {
                #[serde(rename = "fileId")]
                file_id: Option<u64>,
                name: Option<String>,
                version: Option<String>,
                category: Option<String>,
                #[serde(rename = "categoryId")]
                category_id: Option<u32>,
                primary: Option<serde_json::Value>,
                date: Option<i64>,
                #[serde(rename = "sizeInBytes")]
                size_in_bytes: Option<serde_json::Value>,
                #[serde(rename = "uniqueDownloads")]
                unique_downloads: Option<u32>,
                #[serde(rename = "totalDownloads")]
                total_downloads: Option<u32>,
                #[serde(rename = "scannedV2")]
                scanned_v2: Option<String>,
                #[serde(rename = "changelogText")]
                changelog_text: Option<Vec<String>>,
                description: Option<String>,
            }

            #[derive(Deserialize)]
            struct GqlMod {
                name: Option<String>,
                summary: Option<String>,
                description: Option<String>,
                author: Option<String>,
                version: Option<String>,
                downloads: Option<u32>,
                endorsements: Option<u32>,
                #[serde(rename = "pictureUrl")]
                picture_url: Option<String>,
                #[serde(rename = "createdAt")]
                created_at: Option<String>,
                #[serde(rename = "updatedAt")]
                updated_at: Option<String>,
                category: Option<String>,
                #[serde(rename = "viewerEndorsed")]
                viewer_endorsed: Option<bool>,
                #[serde(rename = "viewerTracked")]
                viewer_tracked: Option<bool>,
            }
            #[derive(Deserialize)]
            struct GqlData {
                #[serde(rename = "mod")]
                r#mod: Option<GqlMod>,
                #[serde(rename = "modFiles")]
                mod_files: Option<Vec<GqlFile>>,
            }
            #[derive(Deserialize)]
            struct GqlResp {
                data: Option<GqlData>,
            }

            if let Ok(body) = resp.json::<GqlResp>().await {
                if let Some(d) = body.data {
                    if let Some(m) = d.r#mod {
                        name = m.name.unwrap_or_default();
                        summary = m.summary.unwrap_or_default();
                        description = m.description.unwrap_or_default();
                        author = m.author.unwrap_or_default();
                        version = m.version.unwrap_or_else(|| "1.0".to_string());
                        downloads = m.downloads.unwrap_or(0);
                        endorsements = m.endorsements.unwrap_or(0);
                        picture_url = m.picture_url.unwrap_or_default();
                        created_at = m.created_at.unwrap_or_default();
                        updated_at = m.updated_at.unwrap_or_default();
                        category_name = m.category;
                        is_endorsed = m.viewer_endorsed.unwrap_or(false);
                        is_tracked = m.viewer_tracked.unwrap_or(false);
                        if !picture_url.is_empty() {
                            images.push(picture_url.clone());
                        }
                    }

                    if let Some(gfiles) = d.mod_files {
                        for gf in gfiles {
                            let fid = gf.file_id.unwrap_or(0);
                            if fid == 0 {
                                continue;
                            }
                            let fname = gf.name.unwrap_or_else(|| "Mod File".to_string());
                            let fver = gf.version.unwrap_or_else(|| "1.0".to_string());
                            let cat_id = gf.category_id.unwrap_or(1);
                            let cat_name = gf.category.unwrap_or_else(|| match cat_id {
                                1 => "MAIN".to_string(),
                                2 => "UPDATE".to_string(),
                                3 => "OPTIONAL".to_string(),
                                4 => "OLD_VERSION".to_string(),
                                _ => "MISC".to_string(),
                            });

                            let is_primary = match gf.primary {
                                Some(serde_json::Value::Bool(b)) => b,
                                Some(serde_json::Value::Number(n)) => n.as_i64() == Some(1),
                                _ => cat_id == 1,
                            };

                            let bytes_num = if let Some(ref sb) = gf.size_in_bytes {
                                if let Some(n) = sb.as_u64() {
                                    n
                                } else if let Some(s) = sb.as_str() {
                                    s.parse::<u64>().unwrap_or(0)
                                } else {
                                    0
                                }
                            } else {
                                0
                            };

                            let uploaded_ts = gf.date;
                            let uploaded_formatted = if let Some(ts) = uploaded_ts {
                                chrono::DateTime::from_timestamp(ts, 0)
                                    .map(|dt| dt.format("%d %b %Y, %H:%M").to_string())
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };

                            files_list.push(DiscoveryFileItem {
                                file_id: fid,
                                name: fname,
                                version: fver,
                                category_id: cat_id,
                                category_name: cat_name,
                                is_primary,
                                size_in_bytes: bytes_num,
                                size_formatted: format_file_size(bytes_num),
                                uploaded_at: uploaded_formatted,
                                uploaded_timestamp: uploaded_ts,
                                description: gf.description.unwrap_or_default(),
                                unique_downloads: gf.unique_downloads,
                                total_downloads: gf.total_downloads,
                                scan_status: gf.scanned_v2,
                                changelog_entries: gf.changelog_text,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. If files_list is empty, fallback to REST API v1
    if files_list.is_empty() {
        let files_url = format!("{}/games/palworld/mods/{}/files.json", REST_BASE_URL, mod_id);
        let mut files_resp = client.get(&files_url).send().await;

        if let Ok(ref r) = files_resp {
            if r.status() == StatusCode::UNAUTHORIZED || r.status().as_u16() == 402 {
                let anon_client = get_client(None);
                files_resp = anon_client.get(&files_url).send().await;
            }
        }

        if let Ok(files_resp) = files_resp {
            if files_resp.status().is_success() {
                #[derive(Deserialize)]
                struct RestFilesPayload {
                    files: Option<Vec<serde_json::Value>>,
                }

                if let Ok(payload) = files_resp.json::<RestFilesPayload>().await {
                    if let Some(files) = payload.files {
                        for f in files {
                            let fid = if let Some(id_num) = f.get("file_id").and_then(|v| v.as_u64()) {
                                id_num
                            } else if let Some(arr) = f.get("file_id").and_then(|v| v.as_array()) {
                                arr.first().and_then(|v| v.as_u64()).unwrap_or(0)
                            } else {
                                0
                            };

                            if fid == 0 {
                                continue;
                            }

                            let file_name = f.get("name").and_then(|v| v.as_str()).or_else(|| f.get("file_name").and_then(|v| v.as_str())).unwrap_or("Mod File").to_string();
                            let file_version = f.get("version").and_then(|v| v.as_str()).unwrap_or("1.0").to_string();
                            let cat_id = f.get("category_id").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                            let cat_name = f.get("category_name").and_then(|v| v.as_str()).unwrap_or_else(|| match cat_id {
                                1 => "MAIN",
                                2 => "UPDATE",
                                3 => "OPTIONAL",
                                4 => "OLD_VERSION",
                                5 => "MISCELLANEOUS",
                                _ => "MISC",
                            }).to_string();

                            let is_primary = f.get("is_primary").and_then(|v| v.as_bool()).unwrap_or(cat_id == 1);
                            let size_bytes = if let Some(b) = f.get("size_in_bytes").and_then(|v| v.as_u64()) {
                                b
                            } else if let Some(kb) = f.get("size_kb").and_then(|v| v.as_u64()) {
                                kb * 1024
                            } else {
                                0
                            };

                            let uploaded = f.get("uploaded_time").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let uploaded_ts = f.get("uploaded_timestamp").and_then(|v| v.as_i64());
                            let desc = f.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();

                            files_list.push(DiscoveryFileItem {
                                file_id: fid,
                                name: file_name,
                                version: file_version,
                                category_id: cat_id,
                                category_name: cat_name,
                                is_primary,
                                size_in_bytes: size_bytes,
                                size_formatted: format_file_size(size_bytes),
                                uploaded_at: uploaded,
                                uploaded_timestamp: uploaded_ts,
                                description: desc,
                                unique_downloads: None,
                                total_downloads: None,
                                scan_status: Some("VERIFIED".to_string()),
                                changelog_entries: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Extract all screenshots from the Nexus mod page & description
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0".parse().unwrap());
    headers.insert(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".parse().unwrap());
    headers.insert(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.5".parse().unwrap());
    headers.insert("Sec-Fetch-Dest", "document".parse().unwrap());
    headers.insert("Sec-Fetch-Mode", "navigate".parse().unwrap());
    headers.insert("Sec-Fetch-Site", "none".parse().unwrap());
    headers.insert("Sec-Fetch-User", "?1".parse().unwrap());
    headers.insert("Priority", "u=1".parse().unwrap());

    let web_client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_else(|_| client.clone());

    let mod_id_str = mod_id.to_string();
    let urls_to_check = vec![
        format!("https://www.nexusmods.com/palworld/mods/{}", mod_id),
        format!("https://www.nexusmods.com/palworld/mods/{}?tab=images", mod_id),
    ];

    let re_pattern = format!(r#"https://(?:staticdelivery|images)\.nexusmods\.com/[^\s"'<>\\]*?/{}/[a-zA-Z0-9_\-\.]+\.(?:png|jpg|jpeg|webp)"#, mod_id_str);
    if let Ok(re) = regex::Regex::new(&re_pattern) {
        for page_url in urls_to_check {
            if let Ok(page_resp) = web_client.get(&page_url).send().await {
                if let Ok(html) = page_resp.text().await {
                    for cap in re.find_iter(&html) {
                        let matched_url = cap.as_str();
                        let full_res = matched_url.replace("/images/thumbnails/", "/images/").replace("/thumbnails/", "/");
                        if !images.contains(&full_res) {
                            images.push(full_res);
                        }
                    }
                }
            }
        }
    }

    println!("[Discovery] Mod {} extracted {} gallery images", mod_id, images.len());

    // Extract any screenshots embedded in description (BBCode, Markdown or HTML)
    for line in description.split(|c| c == '\n' || c == '[' || c == ']' || c == '"' || c == '\'' || c == '(' || c == ')') {
        let trimmed = line.trim();
        if (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
            && (trimmed.ends_with(".png") || trimmed.ends_with(".jpg") || trimmed.ends_with(".jpeg") || trimmed.ends_with(".webp") || trimmed.contains("staticdelivery.nexusmods.com") || trimmed.contains("images.nexusmods.com"))
        {
            let clean_url = trimmed.replace("/thumbnails/", "/");
            if !images.contains(&clean_url) {
                images.push(clean_url);
            }
        }
    }

    Ok(DiscoveryModDetails {
        mod_id,
        name,
        summary,
        description,
        author,
        version,
        downloads,
        endorsements,
        picture_url,
        created_at,
        updated_at,
        category_id: None,
        category_name,
        contains_adult_content,
        files: files_list,
        images,
        is_endorsed,
        is_tracked,
    })
}

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
