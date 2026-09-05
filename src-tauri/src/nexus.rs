use serde::{Deserialize, Serialize};
use std::path::Path;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    pub nexus_file_id: Option<String>,
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
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct NexusFileEntry {
    file_id: Option<i64>,
    version: Option<String>,
    category_name: Option<String>,
}

#[allow(dead_code)]
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
fn is_version_token(s: &str) -> bool {
    let lower = s.to_lowercase();
    if lower.starts_with('v') && lower.len() > 1 && lower[1..].chars().next().map_or(false, |c| c.is_ascii_digit()) {
        return true;
    }
    if s.contains('.') && s.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return true;
    }
    false
}

fn clean_version_token(v: &str) -> String {
    let stripped = if v.starts_with('v') || v.starts_with('V') {
        &v[1..]
    } else {
        v
    };
    stripped.trim_end_matches('.').to_string()
}

pub fn parse_mod_filename(filename: &str) -> ParsedModInfo {
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.to_string());

    if stem.to_lowercase().starts_with("nexus_") || stem.to_lowercase().starts_with("nexus-") {
        return ParsedModInfo { name: None, nexus_id: None, nexus_file_id: None, version: None, date: None };
    }

    // 1. Detect hyphen-separated pattern: Name-ID-Version-Timestamp[-Token]
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
                let (name, version) = if idx > 0 && is_version_token(hyphen_parts[idx - 1]) {
                    let v = clean_version_token(hyphen_parts[idx - 1]);
                    let n = hyphen_parts[..idx - 1].join(" ");
                    (n, Some(v))
                } else {
                    let n = hyphen_parts[..idx].join(" ");
                    let v_parts = &hyphen_parts[idx + 1..ds_idx];
                    let v = if !v_parts.is_empty() { Some(clean_version_token(&v_parts.join("."))) } else { None };
                    (n, v)
                };

                let nexus_id = hyphen_parts[idx].parse::<u32>().unwrap();
                let last_token = hyphen_parts[hyphen_parts.len() - 1];
                let (date_str, nexus_file_id) = if hyphen_parts.len() > ds_idx + 1 {
                    let file_id = last_token.to_string();
                    let d = hyphen_parts[ds_idx..hyphen_parts.len() - 1].join("-");
                    (d, Some(file_id))
                } else {
                    (hyphen_parts[ds_idx..].join("-"), None)
                };

                return ParsedModInfo {
                    name: if name.is_empty() { None } else { Some(name) },
                    nexus_id: Some(nexus_id),
                    nexus_file_id,
                    version,
                    date: Some(date_str),
                };
            }

            let last_token = hyphen_parts[hyphen_parts.len() - 1];
            let is_timestamp = last_token.len() >= 10 && last_token.chars().all(|c| c.is_ascii_digit());
            if is_timestamp {
                let (name, version) = if idx > 0 && is_version_token(hyphen_parts[idx - 1]) {
                    (hyphen_parts[..idx - 1].join(" "), Some(clean_version_token(hyphen_parts[idx - 1])))
                } else {
                    (hyphen_parts[..idx].join(" "), Some(clean_version_token(&hyphen_parts[idx + 1..hyphen_parts.len() - 1].join("."))))
                };
                let nexus_id = hyphen_parts[idx].parse::<u32>().unwrap();
                return ParsedModInfo {
                    name: if name.is_empty() { None } else { Some(name) },
                    nexus_id: Some(nexus_id),
                    nexus_file_id: None,
                    version,
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
        return ParsedModInfo { name: None, nexus_id: None, nexus_file_id: None, version: None, date: None };
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
        let mut extracted_version = None;
        let name_end_idx = if id_idx > 0 && is_version_token(parts[id_idx - 1]) {
            extracted_version = Some(clean_version_token(parts[id_idx - 1]));
            id_idx - 1
        } else {
            id_idx
        };

        let name = if name_end_idx > 0 {
            let name_parts: Vec<&str> = parts[..name_end_idx].to_vec();
            let clean: Vec<&str> = name_parts.into_iter()
                .filter(|p| !["steam", "singleplayer", "sp"].contains(&p.to_lowercase().as_str()))
                .collect();
            if clean.is_empty() { None } else { Some(clean.join(" ")) }
        } else {
            None
        };

        let start_idx = id_idx + 1;

        let mut date: Option<String> = None;
        let mut date_idx: Option<usize> = None;

        for i in start_idx..parts.len() {
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

        let mut nexus_file_id: Option<String> = None;

        if let Some(di) = date_idx {
            // Check if there is an alphanumeric token immediately following the date/timestamp (e.g. QXTyhgia8, vcf5TPXlj)
            if di + 1 < parts.len() {
                nexus_file_id = Some(parts[di + 1].to_string());
            }

            if extracted_version.is_none() {
                if di > start_idx {
                    let ver_parts: Vec<&str> = parts[start_idx..di].to_vec();
                    let joined = ver_parts.join(".");
                    if !joined.is_empty() {
                        extracted_version = Some(clean_version_token(&joined));
                    }
                }
            } else if nexus_file_id.is_none() && di > start_idx {
                nexus_file_id = Some(parts[start_idx].to_string());
            }
        } else {
            if extracted_version.is_none() {
                if parts.len() > start_idx {
                    let ver_parts: Vec<&str> = parts[start_idx..].to_vec();
                    let joined = ver_parts.join(".");
                    if !joined.is_empty() {
                        extracted_version = Some(clean_version_token(&joined));
                    }
                }
            } else if parts.len() > start_idx {
                nexus_file_id = Some(parts[start_idx].to_string());
            }
        }

        ParsedModInfo {
            name,
            nexus_id: Some(nexus_id),
            nexus_file_id,
            version: extracted_version,
            date,
        }
    } else {
        ParsedModInfo { name: None, nexus_id: None, nexus_file_id: None, version: None, date: None }
    }
}

pub fn extract_nexus_id(filename: &str) -> Option<u32> {
    parse_mod_filename(filename).nexus_id
}

/// Fetch the latest MAIN/UPDATE file version for a mod via GraphQL v2 API anonymously.
async fn fetch_latest_file_version(mod_id: u32) -> Option<String> {
    let query = r#"
query GetModFiles($modId: ID!, $gameId: ID!) {
  modFiles(modId: $modId, gameId: $gameId) {
    category
    version
    fileId
  }
}
"#;

    let payload = serde_json::json!({
        "query": query,
        "variables": {
            "modId": mod_id.to_string(),
            "gameId": "6063"
        }
    });

    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let req = client
        .post(GRAPHQL_ENDPOINT)
        .header("Content-Type", "application/json")
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .json(&payload);

    let resp = req.send().await.ok()?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    


    if !status.is_success() {
        return None;
    }

    #[derive(Debug, Deserialize)]
    struct GqlFilesResponse {
        data: Option<GqlFilesData>,
    }
    #[derive(Debug, Deserialize)]
    struct GqlFilesData {
        #[serde(rename = "modFiles")]
        mod_files: Option<Vec<GqlModFile>>,
    }
    #[derive(Debug, Deserialize)]
    struct GqlModFile {
        category: Option<String>,
        version: Option<String>,
        #[serde(rename = "fileId")]
        file_id: Option<i64>,
    }

    let gql_resp: GqlFilesResponse = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            crate::logger::log(&format!("fetch_latest_file_version: JSON parse failed: {}", e));
            return None;
        }
    };
    let files = gql_resp.data?.mod_files?;

    let mut best_version: Option<String> = None;
    let mut max_file_id = 0i64;

    for entry in files {
        if let Some(cat) = &entry.category {
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

pub async fn fetch_mod_info(mod_id: u32) -> Result<NexusModInfo, String> {
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
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let req = client
        .post(GRAPHQL_ENDPOINT)
        .header("Content-Type", "application/json")
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .json(&payload);

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
        .map_err(|e| format!("Failed to get response text: {}", e))?;

    let body: GraphQLResponse = serde_json::from_str(&text).map_err(|e| {
        crate::logger::log(&format!("fetch_mod_info: Failed to parse body: {}", e));
        format!("JSON parse error: {}", e)
    })?;

    if let Some(errors) = body.errors {
        if !errors.is_empty() {
            crate::logger::log(&format!("fetch_mod_info: GraphQL error: {}", errors[0].message));
            return Err(format!("GraphQL error: {}", errors[0].message));
        }
    }

    let data = body.data.ok_or("No data in response")?;
    let r#mod = data.r#mod.ok_or("Mod not found")?;

    let graphql_version = r#mod.version.clone().unwrap_or_default();
    let best_version = fetch_latest_file_version(mod_id).await
        .unwrap_or(graphql_version);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_palvariety_filenames() {
        let file1 = "PalVariety Rarity 1to500 V1.0.0 4140 1 2026-08-12T20-44Z vcf5TPXlj.zip";
        let res1 = parse_mod_filename(file1);
        assert_eq!(res1.name, Some("PalVariety Rarity 1to500".to_string()));
        assert_eq!(res1.version, Some("1.0.0".to_string()));
        assert_eq!(res1.nexus_id, Some(4140));
        assert_eq!(res1.nexus_file_id, Some("vcf5TPXlj".to_string()));

        let file2 = "PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip";
        let res2 = parse_mod_filename(file2);
        assert_eq!(res2.name, Some("PalVariety Rarity Shiny4096".to_string()));
        assert_eq!(res2.version, Some("1.1.3".to_string()));
        assert_eq!(res2.nexus_id, Some(4140));
        assert_eq!(res2.nexus_file_id, Some("QXTyhgia8".to_string()));
    }
}
