use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub ue4ss_installed: bool,
    pub ue4ss_version: Option<String>,
    /// Human-readable tag of the latest UE4SS release (e.g. "experimental-palworld")
    pub ue4ss_latest_tag: Option<String>,
    /// ISO date of the latest UE4SS release — used internally for comparison
    pub ue4ss_latest_date: Option<String>,
    pub ue4ss_needs_update: bool,
    pub palschema_installed: bool,
    pub palschema_version: Option<String>,
    pub palschema_latest_version: Option<String>,
    pub palschema_needs_update: bool,
    pub game_platform: String,
}

fn get_file_date(path: &str) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    let secs = duration.as_secs() as i64;
    let dt = chrono::DateTime::from_timestamp(secs, 0)?;
    Some(dt.format("%d.%m.%Y").to_string())
}

pub fn get_binaries_dir(game_path: &Path) -> std::path::PathBuf {
    let wingdk = game_path.join("Pal").join("Binaries").join("WinGDK");
    if wingdk.exists() {
        wingdk
    } else {
        game_path.join("Pal").join("Binaries").join("Win64")
    }
}

pub fn get_shipping_exe_path(game_path: &Path) -> std::path::PathBuf {
    let binaries = get_binaries_dir(game_path);
    if binaries.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("wingdk".to_string()) {
        binaries.join("Palworld-WinGDK-Shipping.exe")
    } else {
        binaries.join("Palworld-Win64-Shipping.exe")
    }
}

pub fn check_dependencies(game_path: &str) -> DependencyStatus {
    let game_path_val = Path::new(game_path);
    let binaries = get_binaries_dir(game_path_val);

    let game_platform = if binaries.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("wingdk".to_string()) {
        "Xbox".to_string()
    } else if binaries.exists() {
        "Steam".to_string()
    } else {
        "Unknown".to_string()
    };

    // UE4SS check — detect by dwmapi.dll presence, version = file date or ue4ss.version
    let dwmapi = binaries.join("dwmapi.dll");
    let ue4ss_installed = dwmapi.exists();
    let ue4ss_version = if ue4ss_installed {
        let version_file = binaries.join("ue4ss").join("ue4ss.version");
        if version_file.exists() {
            fs::read_to_string(version_file).ok().map(|s| s.trim().to_string())
        } else {
            get_file_date(&dwmapi.to_string_lossy())
        }
    } else {
        None
    };

    // PalSchema check — detect by dlls/main.dll, version = palschema.version if available
    let ps_dll = binaries.join("ue4ss").join("Mods").join("PalSchema").join("dlls").join("main.dll");
    let palschema_installed = ps_dll.exists();
    let palschema_version = if palschema_installed {
        let version_file = binaries.join("ue4ss").join("Mods").join("PalSchema").join("palschema.version");
        if version_file.exists() {
            fs::read_to_string(version_file).ok().map(|s| s.trim().to_string())
        } else {
            None
        }
    } else {
        None
    };

    DependencyStatus {
        ue4ss_installed,
        ue4ss_version,
        ue4ss_latest_tag: None,
        ue4ss_latest_date: None,
        ue4ss_needs_update: false,
        palschema_installed,
        palschema_version,
        palschema_latest_version: None,
        palschema_needs_update: false,
        game_platform,
    }
}

/// Returns a tuple: (tag_name, iso_date_string) for the latest UE4SS release.
/// tag_name is what we show to the user; iso_date is used for update comparison.
pub async fn check_ue4ss_latest() -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let tag_name = "experimental-palworld".to_string();

    // Try HTML scraping first to avoid API rate limit
    if let Ok(resp) = client.get("https://github.com/Okaetsu/RE-UE4SS/releases/tag/experimental-palworld").send().await {
        if let Ok(html) = resp.text().await {
            if let Some(time_pos) = html.find("datetime=") {
                let time_start = time_pos + 10;
                if let Some(time_end) = html[time_start..].find('"') {
                    let dt_raw = &html[time_start..time_start + time_end];
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_raw) {
                        let iso_date = dt.format("%d.%m.%Y").to_string();
                        return Ok((tag_name, iso_date));
                    }
                }
            }
        }
    }

    // Fallback: GitHub API
    let url = "https://api.github.com/repos/Okaetsu/RE-UE4SS/releases/tags/experimental-palworld";
    let resp = client.get(url)
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let api_tag = json["tag_name"].as_str().unwrap_or("experimental-palworld").to_string();
    let mut latest: Option<chrono::DateTime<chrono::Utc>> = None;
    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            if let Some(updated) = asset["updated_at"].as_str() {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(updated) {
                    match latest {
                        Some(current) if dt > current => { latest = Some(dt.into()); }
                        None => { latest = Some(dt.into()); }
                        _ => {}
                    }
                }
            }
        }
    }
    if latest.is_none() {
        if let Some(published) = json["published_at"].as_str() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(published) {
                latest = Some(dt.into());
            }
        }
    }
    match latest {
        Some(dt) => Ok((api_tag, dt.format("%d.%m.%Y").to_string())),
        None => Err("Could not determine latest UE4SS date".to_string()),
    }
}

pub async fn check_palschema_latest() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Try HTML redirect first to avoid API rate limit!
    if let Ok(resp) = client.get("https://github.com/Okaetsu/PalSchema/releases/latest").send().await {
        let final_url = resp.url().as_str();
        if final_url.contains("/releases/tag/") {
            if let Some(tag) = final_url.split("/releases/tag/").last() {
                if !tag.is_empty() {
                    return Ok(tag.to_string());
                }
            }
        }
    }

    let url = "https://api.github.com/repos/Okaetsu/PalSchema/releases/latest";
    let resp = client.get(url)
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    let tag = json["tag_name"].as_str()
        .ok_or_else(|| "No tag_name in response".to_string())?;
    Ok(tag.to_string())
}
