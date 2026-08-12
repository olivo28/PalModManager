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
    /// How UE4SS was installed: "Standard", "Workshop", or "NotFound"
    pub ue4ss_install_mode: String,
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

pub fn get_ue4ss_mods_dir(game_path: &Path) -> std::path::PathBuf {
    let profile = build_game_profile(game_path);
    profile.ue4ss_mods_dir
}

pub fn get_shipping_exe_path(game_path: &Path) -> std::path::PathBuf {
    let profile = build_game_profile(game_path);
    profile.exe_path
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UE4SSInstallMode {
    Standard,
    Workshop,
    NotFound,
}

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub game_root: std::path::PathBuf,
    pub binaries_dir: std::path::PathBuf,
    pub platform: String,
    pub ue4ss_install_mode: UE4SSInstallMode,
    pub ue4ss_mods_dir: std::path::PathBuf,
    pub mods_txt_path: std::path::PathBuf,
    pub paks_dir: std::path::PathBuf,
    pub logic_mods_dir: std::path::PathBuf,
    pub palschema_mods_dir: std::path::PathBuf,
    pub palschema_storage_dir: std::path::PathBuf,
    pub exe_path: std::path::PathBuf,
}

pub fn detect_game_root(path: &Path) -> Option<std::path::PathBuf> {
    let mut current = path.to_path_buf();
    for _ in 0..6 {
        let has_paks = current.join("Pal/Content/Paks").exists();
        let has_win64_exe = current.join("Pal/Binaries/Win64/Palworld-Win64-Shipping.exe").exists();
        let has_wingdk_exe = current.join("Pal/Binaries/WinGDK/Palworld-WinGDK-Shipping.exe").exists();
        
        if has_paks && (has_win64_exe || has_wingdk_exe) {
            return Some(current);
        }
        
        // Also fallback validation for simple game structure (just folder matching)
        if has_paks && (current.join("Pal/Binaries/Win64").exists() || current.join("Pal/Binaries/WinGDK").exists()) {
            return Some(current);
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

pub fn build_game_profile(game_root: &Path) -> GameProfile {
    let root = detect_game_root(game_root).unwrap_or_else(|| game_root.to_path_buf());
    let wingdk = root.join("Pal").join("Binaries").join("WinGDK");
    let is_xbox = wingdk.exists();
    
    let binaries_dir = if is_xbox {
        wingdk
    } else {
        root.join("Pal").join("Binaries").join("Win64")
    };

    let platform = if is_xbox {
        "Xbox".to_string()
    } else if root.join("Pal").join("Binaries").join("Win64").exists() {
        "Steam".to_string()
    } else {
        "Unknown".to_string()
    };

    let standard_dwmapi = binaries_dir.join("dwmapi.dll");
    let standard_ue4ss_dir = binaries_dir.join("ue4ss");
    let workshop_ue4ss_dir = root.join("Mods").join("NativeMods").join("UE4SS");

    let ue4ss_install_mode = if standard_dwmapi.exists() && standard_ue4ss_dir.exists() {
        UE4SSInstallMode::Standard
    } else if standard_dwmapi.exists() {
        UE4SSInstallMode::Standard
    } else if workshop_ue4ss_dir.exists() {
        UE4SSInstallMode::Workshop
    } else {
        UE4SSInstallMode::NotFound
    };

    let ue4ss_mods_dir = match ue4ss_install_mode {
        UE4SSInstallMode::Workshop => workshop_ue4ss_dir.join("Mods"),
        _ => standard_ue4ss_dir.join("Mods"),
    };

    let exe_path = if is_xbox {
        binaries_dir.join("Palworld-WinGDK-Shipping.exe")
    } else {
        binaries_dir.join("Palworld-Win64-Shipping.exe")
    };

    GameProfile {
        game_root: root.clone(),
        binaries_dir: binaries_dir.clone(),
        platform,
        ue4ss_mods_dir: ue4ss_mods_dir.clone(),
        mods_txt_path: ue4ss_mods_dir.join("mods.txt"),
        paks_dir: root.join("Pal").join("Content").join("Paks").join("~mods"),
        logic_mods_dir: root.join("Pal").join("Content").join("Paks").join("LogicMods"),
        palschema_mods_dir: ue4ss_mods_dir.join("PalSchema").join("mods"),
        palschema_storage_dir: ue4ss_mods_dir.join("PalSchema").join("Storage"),
        ue4ss_install_mode,
        exe_path,
    }
}

pub fn check_dependencies(game_path: &str) -> DependencyStatus {
    let game_path_val = Path::new(game_path);
    let profile = build_game_profile(game_path_val);

    let (ue4ss_installed, ue4ss_version) = match profile.ue4ss_install_mode {
        UE4SSInstallMode::Standard => {
            let version_file = profile.binaries_dir.join("ue4ss").join("ue4ss.version");
            let ver = if version_file.exists() {
                fs::read_to_string(&version_file).ok().map(|s| s.trim().to_string())
            } else {
                get_file_date(&profile.binaries_dir.join("dwmapi.dll").to_string_lossy())
            };
            (true, ver)
        }
        UE4SSInstallMode::Workshop => {
            let settings = crate::workshop::read_pal_mod_settings(game_path);
            let is_active_in_ini = settings.global_enabled
                && settings.active_mod_list.iter().any(|m| {
                    m.eq_ignore_ascii_case("UE4SSExperimentalPW") ||
                    m.to_lowercase().contains("ue4ss")
                });

            if !is_active_in_ini {
                (false, None)
            } else {
                let workshop_dir = game_path_val.join("Mods").join("NativeMods").join("UE4SS");
                let version_file = workshop_dir.join("ue4ss.version");
                let ver = if version_file.exists() {
                    fs::read_to_string(&version_file).ok().map(|s| s.trim().to_string())
                } else {
                    Some("Workshop".to_string())
                };
                (true, ver)
            }
        }
        UE4SSInstallMode::NotFound => {
            (false, None)
        }
    };

    let ps_dll = profile.ue4ss_mods_dir.join("PalSchema").join("dlls").join("main.dll");
    let (palschema_installed, palschema_version) = if ps_dll.exists() {
        let ps_active = if profile.ue4ss_install_mode == UE4SSInstallMode::Workshop {
            let settings = crate::workshop::read_pal_mod_settings(game_path);
            settings.global_enabled &&
            settings.active_mod_list.iter().any(|m| m.eq_ignore_ascii_case("PalSchema"))
        } else {
            true
        };

        if !ps_active {
            (false, None)
        } else {
            let version_file = profile.ue4ss_mods_dir.join("PalSchema").join("palschema.version");
            let ver = if version_file.exists() {
                fs::read_to_string(&version_file).ok().map(|s| s.trim().to_string())
            } else {
                match profile.ue4ss_install_mode {
                    UE4SSInstallMode::Workshop => Some("Workshop".to_string()),
                    _ => None,
                }
            };
            (true, ver)
        }
    } else {
        (false, None)
    };

    let ue4ss_install_mode_str = match profile.ue4ss_install_mode {
        UE4SSInstallMode::Standard => "Standard".to_string(),
        UE4SSInstallMode::Workshop => "Workshop".to_string(),
        UE4SSInstallMode::NotFound => "NotFound".to_string(),
    };

    DependencyStatus {
        ue4ss_installed,
        ue4ss_version,
        ue4ss_latest_tag: None,
        ue4ss_latest_date: None,
        ue4ss_needs_update: false,
        ue4ss_install_mode: ue4ss_install_mode_str,
        palschema_installed,
        palschema_version,
        palschema_latest_version: None,
        palschema_needs_update: false,
        game_platform: profile.platform,
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
