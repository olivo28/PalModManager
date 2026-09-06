use std::time::Duration;
use super::flow::APP_VERSION;
use super::types::{NxmDownloadProgressEvent, NxmLinkInfo, NxmModMetadata};

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

    // Scrub query tokens from log to avoid writing sensitive credentials to disk
    crate::logger::log(&format!("fetch_nxm_direct_download_url: Calling download link endpoint for game={}, mod_id={}, file_id={}", nxm.game_domain, nxm.mod_id, nxm.file_id));

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

/// Fetch mod file details (file_name, version, etc.) from Nexus REST API v1
pub async fn fetch_nxm_file_details(
    access_token: &str,
    game_domain: &str,
    mod_id: u32,
    file_id: u64,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;

    let api_url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}/files/{}.json",
        game_domain, mod_id, file_id
    );

    let resp = client.get(&api_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Application-Name", "PalModManager")
        .header("Application-Version", APP_VERSION)
        .send()
        .await
        .map_err(|e| format!("Failed to request file details: {}", e))?;

    if resp.status().is_success() {
        let val: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(file_name) = val.get("file_name").and_then(|v| v.as_str()) {
            if !file_name.trim().is_empty() {
                return Ok(file_name.trim().to_string());
            }
        }
    }
    Err("Could not retrieve file_name from Nexus API".to_string())
}

/// Download file from CDN and save with its real archive filename with progress events
pub async fn download_file_to_temp_with_progress(
    app_handle: &tauri::AppHandle,
    download_url: &str,
    file_id: u64,
    download_id: &str,
    preferred_filename: Option<&str>,
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

    // Extract real filename: 1. Preferred from Nexus API, 2. Content-Disposition header, 3. URL path, 4. fallback
    let mut detected_filename: Option<String> = preferred_filename.map(|s| s.to_string());

    if detected_filename.is_none() {
        if let Some(cd) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION).and_then(|h| h.to_str().ok()) {
            if let Some(idx) = cd.find("filename=") {
                let fn_str = &cd[idx + 9..];
                if fn_str.starts_with('"') {
                    if let Some(end_quote) = fn_str[1..].find('"') {
                        let name = fn_str[1..=end_quote].trim().to_string();
                        if !name.is_empty() {
                            detected_filename = Some(name);
                        }
                    }
                } else {
                    let end = fn_str.find(';').unwrap_or(fn_str.len());
                    let name = fn_str[..end].trim().to_string();
                    if !name.is_empty() {
                        detected_filename = Some(name);
                    }
                }
            }
        }
    }

    if detected_filename.is_none() {
        if let Ok(parsed_url) = reqwest::Url::parse(download_url) {
            if let Some(path_seg) = parsed_url.path_segments().and_then(|mut s| s.next_back()) {
                let decoded = match url::form_urlencoded::parse(format!("k={path_seg}").as_bytes()).next() {
                    Some((_, v)) => v.to_string(),
                    None => path_seg.to_string(),
                };
                let lower = decoded.to_lowercase();
                if lower.ends_with(".zip") || lower.ends_with(".7z") || lower.ends_with(".rar") {
                    detected_filename = Some(decoded);
                }
            }
        }
    }

    let raw_name = detected_filename.unwrap_or_else(|| {
        if file_id > 0 {
            format!("nexus_{}.zip", file_id)
        } else {
            format!("download_{}.zip", uuid::Uuid::new_v4())
        }
    });

    // Sanitize filename to prevent invalid OS characters
    let clean_filename = raw_name
        .replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_")
        .trim()
        .to_string();

    let target_file = temp_dir.join(&clean_filename);

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
