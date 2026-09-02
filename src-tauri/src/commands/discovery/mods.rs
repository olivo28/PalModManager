use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;
use serde::Deserialize;
use tauri::State;
use crate::state::AppState;
use super::client::{get_client, GRAPHQL_ENDPOINT, REST_BASE_URL};
use super::types::{DiscoveryModItem, DiscoveryResponse};

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
    _exclude_tags: Option<Vec<String>>,
    _hide_translations: Option<bool>,
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
        #[allow(dead_code)]
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
