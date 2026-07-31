use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusModInfo {
    pub mod_id: u32,
    pub name: String,
    pub author: String,
    pub summary: String,
    pub description: String,
    pub version: String,
    pub downloads: u32,
    pub endorsements: u32,
    pub picture_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedModInfo {
    pub name: Option<String>,
    pub nexus_id: Option<u32>,
    pub version: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    #[serde(rename = "mod")]
    r#mod: Option<GraphQLMod>,
}

#[derive(Debug, Deserialize)]
struct GraphQLTag {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQLMod {
    name: Option<String>,
    author: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    version: Option<String>,
    downloads: Option<u32>,
    endorsements: Option<u32>,
    picture_url: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    category: Option<String>,
    #[serde(default)]
    tags: Option<Vec<GraphQLTag>>,
}

/// REST v1 file entry from /v1/games/{game}/mods/{id}/files.json
#[derive(Debug, Deserialize)]
struct NexusFileEntry {
    file_id: Option<i64>,
    version: Option<String>,
    category_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NexusFilesResponse {
    files: Option<Vec<NexusFileEntry>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

const GRAPHQL_ENDPOINT: &str = "https://api.nexusmods.com/v2/graphql";

/// Parse a mod filename and extract name, nexus ID, version, and date.
/// Supports patterns:
///   - `Name 4355 1 2026-07-27T06-25Z Hash.zip`
///   - `Name (Platform) 3866 2 2026-07-27T23-27Z Hash.zip`
///   - `Name-ID-Version-Timestamp.ext`
///   - `Name v2.0 657 2026-07-14T08-11Z Hash.zip`
pub fn parse_mod_filename(filename: &str) -> ParsedModInfo {
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.to_string());

    // 1. Detect hyphen-separated pattern: Name-ID-Version-Timestamp
    let hyphen_parts: Vec<&str> = stem.split('-').collect();
    if hyphen_parts.len() >= 4 {
        let mut nexus_id_idx = None;
        for i in 1..hyphen_parts.len() - 2 {
            if let Ok(id) = hyphen_parts[i].parse::<u32>() {
                if id >= 100 && id <= 9999999 {
                    nexus_id_idx = Some(i);
                    break;
                }
            }
        }

        if let Some(idx) = nexus_id_idx {
            let mut date_start_idx = None;
            for i in (idx + 1)..hyphen_parts.len() {
                let token = hyphen_parts[i];
                if (token.len() == 4 && (token.starts_with("202") || token.starts_with("203")))
                   || (token.len() >= 10 && token.chars().all(|c| c.is_ascii_digit())) {
                    date_start_idx = Some(i);
                    break;
                }
            }

            if let Some(ds_idx) = date_start_idx {
                let name = hyphen_parts[..idx].join(" ");
                let nexus_id = hyphen_parts[idx].parse::<u32>().unwrap();
                let version_parts = &hyphen_parts[idx + 1..ds_idx];
                let version = version_parts.join(".");
                let date_parts = &hyphen_parts[ds_idx..hyphen_parts.len() - 1];
                let date_str = date_parts.join("-");

                return ParsedModInfo {
                    name: Some(name),
                    nexus_id: Some(nexus_id),
                    version: Some(version),
                    date: Some(date_str),
                };
            }

            let last_token = hyphen_parts[hyphen_parts.len() - 1];
            let is_timestamp = last_token.len() >= 10 && last_token.chars().all(|c| c.is_ascii_digit());
            if is_timestamp {
                let name = hyphen_parts[..idx].join(" ");
                let nexus_id = hyphen_parts[idx].parse::<u32>().unwrap();
                let version_parts = &hyphen_parts[idx + 1..hyphen_parts.len() - 1];
                let version = version_parts.join(".");
                return ParsedModInfo {
                    name: Some(name),
                    nexus_id: Some(nexus_id),
                    version: Some(version),
                    date: Some(last_token.to_string()),
                };
            }
        }
    }

    // 2. Fallback to space/underscore/parenthesis separated pattern
    let parts: Vec<&str> = stem.split(&[' ', '_', '(', ')'][..])
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return ParsedModInfo { name: None, nexus_id: None, version: None, date: None };
    }

    let id_candidates: Vec<(usize, u32)> = parts.iter().enumerate()
        .filter_map(|(i, p)| {
            let num = p.parse::<u32>().ok()?;
            if num >= 100 && num <= 9999999
                && !(num >= 2020 && num <= 2038)
                && !p.contains('.')
            {
                Some((i, num))
            } else {
                None
            }
        })
        .collect();

    let best_id = id_candidates.into_iter()
        .min_by_key(|(i, _)| (parts.len() as isize / 2 - *i as isize).abs());

    if let Some((id_idx, nexus_id)) = best_id {
        let name = if id_idx > 0 {
            let name_parts: Vec<&str> = parts[..id_idx].to_vec();
            let clean: Vec<&str> = name_parts.into_iter()
                .filter(|p| !["steam", "singleplayer", "sp"].contains(&p.to_lowercase().as_str()))
                .collect();
            if clean.is_empty() { None } else { Some(clean.join(" ")) }
        } else {
            None
        };

        let mut date: Option<String> = None;
        let mut date_idx: Option<usize> = None;

        for i in (id_idx + 1)..parts.len() {
            let p = parts[i];
            let is_date = p.len() >= 10
                && p.as_bytes()[0] == b'2'
                && p.as_bytes()[1].is_ascii_digit()
                && p.as_bytes()[2].is_ascii_digit()
                && p.as_bytes()[3].is_ascii_digit()
                && (p.len() == 10 || p.as_bytes().get(4) == Some(&b'-'));
            let is_timestamp = p.len() >= 10 && p.chars().all(|c| c.is_ascii_digit());
            if is_date || is_timestamp {
                date = Some(p.to_string());
                date_idx = Some(i);
                break;
            }
        }

        let version = match date_idx {
            Some(di) if di > id_idx + 1 => {
                let ver_parts: Vec<&str> = parts[id_idx + 1..di].to_vec();
                let joined = ver_parts.join(".");
                if joined.is_empty() { None } else { Some(joined) }
            }
            _ => {
                if parts.len() > id_idx + 1 {
                    let ver_parts: Vec<&str> = parts[id_idx + 1..].to_vec();
                    let joined = ver_parts.join(".");
                    if joined.is_empty() { None } else { Some(joined) }
                } else {
                    None
                }
            }
        };

        let cleaned_version = version.map(|v| {
            let stripped = if v.starts_with('v') || v.starts_with('V') {
                &v[1..]
            } else {
                &v
            };
            stripped.trim_end_matches('.').to_string()
        });

        ParsedModInfo {
            name,
            nexus_id: Some(nexus_id),
            version: cleaned_version,
            date,
        }
    } else {
        ParsedModInfo { name: None, nexus_id: None, version: None, date: None }
    }
}

pub fn extract_nexus_id(filename: &str) -> Option<u32> {
    parse_mod_filename(filename).nexus_id
}

/// Fetch the latest MAIN/UPDATE file version for a mod via REST v1 API.
/// Returns None if no api_key or on error.
async fn fetch_latest_file_version(mod_id: u32, api_key: &str) -> Option<String> {
    if api_key.is_empty() {
        return None;
    }
    let url = format!(
        "https://api.nexusmods.com/v1/games/palworld/mods/{}/files.json",
        mod_id
    );
    let client = reqwest::Client::builder()
        .user_agent("PalModManager/1.0.0 (Tauri App)")
        .build()
        .ok()?;
    let resp = client
        .get(&url)
        .header("apikey", api_key)
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        crate::logger::log(&format!("fetch_latest_file_version: REST v1 returned HTTP {}", resp.status()));
        return None;
    }

    let files_resp: NexusFilesResponse = resp.json().await.ok()?;
    let files = files_resp.files?;

    let mut best_version: Option<String> = None;
    let mut max_file_id = 0i64;

    for entry in files {
        if let Some(cat) = &entry.category_name {
            if cat == "MAIN" || cat == "UPDATE" {
                if let Some(fid) = entry.file_id {
                    if fid > max_file_id {
                        max_file_id = fid;
                        if let Some(ver) = entry.version {
                            if !ver.is_empty() && ver != "unknown" {
                                best_version = Some(ver);
                            }
                        }
                    }
                }
            }
        }
    }

    best_version
}

pub async fn fetch_mod_info(mod_id: u32, api_key: Option<&str>) -> Result<NexusModInfo, String> {
    let query = r#"
query GetPalworldMod($modId: ID!) {
  mod(modId: $modId, gameId: "6063") {
    name
    author
    summary
    description
    version
    downloads
    endorsements
    pictureUrl
    createdAt
    updatedAt
    category
    tags {
      name
    }
  }
}
"#;

    let payload = serde_json::json!({
        "query": query,
        "variables": { "modId": mod_id.to_string() }
    });

    let client = reqwest::Client::builder()
        .user_agent("PalModManager/1.0.0 (Tauri App)")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut req = client
        .post(GRAPHQL_ENDPOINT)
        .header("Content-Type", "application/json")
        .json(&payload);

    if let Some(key) = api_key {
        if !key.is_empty() {
            req = req.header("apikey", key);
        }
    }

    let resp = req
        .send()
        .await
        .map_err(|e| {
            crate::logger::log(&format!("fetch_mod_info: Network error: {}", e));
            format!("Network error: {}", e)
        })?;

    crate::logger::log(&format!("fetch_mod_info: HTTP Status Code: {}", resp.status()));

    let text = resp
        .text()
        .await
        .map_err(|e| format!("Read text error: {}", e))?;

    let body: GraphQLResponse = match serde_json::from_str(&text) {
        Ok(b) => b,
        Err(e) => {
            crate::logger::log(&format!("fetch_mod_info: JSON parse failed. Error: {}. Body: {}", e, &text[..text.len().min(500)]));
            return Err(format!("Parse error: {}", e));
        }
    };

    if let Some(errors) = body.errors {
        if !errors.is_empty() {
            crate::logger::log(&format!("fetch_mod_info: GraphQL error: {}", errors[0].message));
            return Err(format!("GraphQL error: {}", errors[0].message));
        }
    }

    let data = body.data.ok_or("No data in response")?;
    let r#mod = data.r#mod.ok_or("Mod not found")?;

    // Try to get the real file version from REST v1 API
    let graphql_version = r#mod.version.clone().unwrap_or_default();
    let best_version = if let Some(key) = api_key {
        fetch_latest_file_version(mod_id, key).await
            .unwrap_or(graphql_version)
    } else {
        graphql_version
    };

    Ok(NexusModInfo {
        mod_id,
        name: r#mod.name.unwrap_or_default(),
        author: r#mod.author.unwrap_or_default(),
        summary: r#mod.summary.unwrap_or_default(),
        description: r#mod.description.unwrap_or_default(),
        version: best_version,
        downloads: r#mod.downloads.unwrap_or(0),
        endorsements: r#mod.endorsements.unwrap_or(0),
        picture_url: r#mod.picture_url.unwrap_or_default(),
        created_at: r#mod.created_at.unwrap_or_default(),
        updated_at: r#mod.updated_at.unwrap_or_default(),
        category: r#mod.category.unwrap_or_default(),
        tags: r#mod.tags.unwrap_or_default().into_iter().filter_map(|t| t.name).collect(),
    })
}
