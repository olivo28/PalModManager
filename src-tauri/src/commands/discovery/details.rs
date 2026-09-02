use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;
use serde::Deserialize;
use tauri::State;
use crate::state::AppState;
use super::client::{format_file_size, get_client, GRAPHQL_ENDPOINT, REST_BASE_URL};
use super::types::{DiscoveryFileItem, DiscoveryModDetails};

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
    let contains_adult_content = false;
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
                                5 => "MISCELLANEOUS".to_string(),
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
