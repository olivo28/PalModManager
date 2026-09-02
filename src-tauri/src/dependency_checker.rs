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
    #[serde(default)]
    pub has_dll_conflict: bool,
    #[serde(default)]
    pub conflicting_dlls: Vec<String>,
    #[serde(default)]
    pub ue4ss_updated_from: Option<String>,
    #[serde(default)]
    pub palschema_updated_from: Option<String>,
    #[serde(default)]
    pub altermatic_installed: bool,
    #[serde(default)]
    pub unipalui_installed: bool,
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
    #[allow(dead_code)]
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

    let settings_ini = root.join("Mods").join("PalModSettings.ini");
    let is_workshop_active = if settings_ini.exists() {
        std::fs::read_to_string(&settings_ini).map(|c| c.contains("bGlobalEnableMod=True") || c.contains("bGlobalEnableMod = True")).unwrap_or(false)
    } else {
        false
    };

    let ue4ss_install_mode = if is_workshop_active || (workshop_ue4ss_dir.exists() && !standard_dwmapi.exists()) {
        UE4SSInstallMode::Workshop
    } else if standard_dwmapi.exists() || standard_ue4ss_dir.exists() {
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
                let mut ver = if version_file.exists() {
                    fs::read_to_string(&version_file).ok().map(|s| s.trim().to_string())
                } else {
                    None
                };

                // Fallback: check Info.json in ManagedMods, NativeMods, or Workshop staging
                if ver.is_none() {
                    let candidates = [
                        game_path_val.join("Mods").join("ManagedMods").join("UE4SSExperimentalPW").join("Info.json"),
                        workshop_dir.join("Info.json"),
                    ];
                    for c in candidates {
                        if c.exists() {
                            if let Ok(info_str) = fs::read_to_string(&c) {
                                if let Ok(info) = serde_json::from_str::<crate::workshop::WorkshopInfoJson>(&info_str) {
                                    if !info.version.is_empty() {
                                        ver = Some(info.version);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                (true, ver.or_else(|| Some("Workshop".to_string())))
            }
        }
        UE4SSInstallMode::NotFound => {
            (false, None)
        }
    };

    let ps_dll_std = profile.ue4ss_mods_dir.join("PalSchema").join("dlls").join("main.dll");
    let ps_dir_std = profile.ue4ss_mods_dir.join("PalSchema");
    let ps_ws_managed = game_path_val.join("Mods").join("ManagedMods").join("PalSchema");
    let ps_ws_native = game_path_val.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("PalSchema");
    let ws_settings = if profile.ue4ss_install_mode == UE4SSInstallMode::Workshop {
        Some(crate::workshop::read_pal_mod_settings(game_path))
    } else {
        None
    };

    let ps_exists = ps_dll_std.exists()
        || ps_dir_std.exists()
        || ps_ws_managed.exists()
        || ps_ws_native.exists()
        || ws_settings.as_ref().map_or(false, |s| s.active_mod_list.iter().any(|m| m.eq_ignore_ascii_case("PalSchema")));

    let (palschema_installed, palschema_version) = if ps_exists {
        let ps_active = if profile.ue4ss_install_mode == UE4SSInstallMode::Workshop {
            ws_settings.as_ref().map_or(true, |s| {
                s.global_enabled && (
                    s.active_mod_list.iter().any(|m| m.eq_ignore_ascii_case("PalSchema"))
                    || ps_ws_managed.exists()
                    || ps_ws_native.exists()
                )
            })
        } else {
            ps_dll_std.exists() || ps_dir_std.exists()
        };

        if !ps_active {
            (false, None)
        } else {
            let version_file = profile.ue4ss_mods_dir.join("PalSchema").join("palschema.version");
            let mut ver = if version_file.exists() {
                fs::read_to_string(&version_file).ok().map(|s| s.trim().to_string())
            } else {
                None
            };

            if ver.is_none() {
                let candidates = [
                    game_path_val.join("Mods").join("ManagedMods").join("PalSchema").join("Info.json"),
                    game_path_val.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("PalSchema").join("Info.json"),
                    profile.ue4ss_mods_dir.join("PalSchema").join("Info.json"),
                ];
                for c in candidates {
                    if c.exists() {
                        if let Ok(info_str) = fs::read_to_string(&c) {
                            if let Ok(info) = serde_json::from_str::<crate::workshop::WorkshopInfoJson>(&info_str) {
                                if !info.version.is_empty() {
                                    ver = Some(info.version);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            let final_ver = ver.or_else(|| match profile.ue4ss_install_mode {
                UE4SSInstallMode::Workshop => Some("Workshop".to_string()),
                _ => None,
            });
            (true, final_ver)
        }
    } else {
        (false, None)
    };

    let ue4ss_install_mode_str = match profile.ue4ss_install_mode {
        UE4SSInstallMode::Standard => "Standard".to_string(),
        UE4SSInstallMode::Workshop => "Workshop".to_string(),
        UE4SSInstallMode::NotFound => "NotFound".to_string(),
    };

    let (has_dll_conflict, conflicting_dlls) = if profile.ue4ss_install_mode == UE4SSInstallMode::Workshop {
        let candidate_dlls = ["dwmapi.dll", "xinput1_3.dll"];
        let found: Vec<String> = candidate_dlls.iter()
            .filter(|&&dll| profile.binaries_dir.join(dll).exists())
            .map(|&s| s.to_string())
            .collect();
        let conflict = !found.is_empty();
        (conflict, found)
    } else {
        (false, Vec::new())
    };

    // Helper to check prefix in a directory
    let has_pak_starting_with = |dir: &std::path::Path, prefix: &str| -> bool {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.starts_with(prefix) && (name.ends_with(".pak") || entry.path().is_dir()) {
                    return true;
                }
            }
        }
        false
    };

    // Check Altermatic Framework (Nexus #1626)
    let altermatic_installed = has_pak_starting_with(&profile.paks_dir, "altermatic")
        || has_pak_starting_with(&profile.paks_dir.join("~mods"), "altermatic")
        || has_pak_starting_with(&profile.logic_mods_dir, "altermatic")
        || profile.ue4ss_mods_dir.join("Altermatic").exists();

    // Check UniPalUI Framework (Nexus #1894)
    let unipalui_installed = has_pak_starting_with(&profile.paks_dir, "unipalui")
        || has_pak_starting_with(&profile.paks_dir.join("~mods"), "unipalui")
        || has_pak_starting_with(&profile.logic_mods_dir, "unipalui")
        || profile.ue4ss_mods_dir.join("UniPalUI").exists();

    if ue4ss_installed {
        crate::dependency_manifest::ensure_ue4ss_manifest(game_path, ue4ss_version.as_deref());
    }
    if palschema_installed {
        crate::dependency_manifest::ensure_palschema_manifest(game_path, palschema_version.as_deref());
    }

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
        has_dll_conflict,
        conflicting_dlls,
        ue4ss_updated_from: None,
        palschema_updated_from: None,
        altermatic_installed,
        unipalui_installed,
    }
}

/// Returns a tuple: (tag_name, iso_date_string) for the latest UE4SS release.
/// tag_name is what we show to the user; iso_date is used for update comparison.
pub async fn check_ue4ss_latest() -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .user_agent("PalModManager/1.7.0")
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let default_tag = "experimental-palworld".to_string();

    // Priority 1: GitHub API (inspects asset upload/update timestamps)
    let url = "https://api.github.com/repos/Okaetsu/RE-UE4SS/releases/tags/experimental-palworld";
    if let Ok(resp) = client.get(url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let api_tag = json["tag_name"].as_str().unwrap_or(&default_tag).to_string();
                let mut latest_asset_dt: Option<chrono::DateTime<chrono::Utc>> = None;

                if let Some(assets) = json["assets"].as_array() {
                    for asset in assets {
                        let name = asset["name"].as_str().unwrap_or("");
                        if name.starts_with("UE4SS-Palworld") {
                            if let Some(updated) = asset["updated_at"].as_str().or_else(|| asset["created_at"].as_str()) {
                                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(updated) {
                                    let dt_utc: chrono::DateTime<chrono::Utc> = dt.into();
                                    match latest_asset_dt {
                                        Some(cur) if dt_utc > cur => { latest_asset_dt = Some(dt_utc); }
                                        None => { latest_asset_dt = Some(dt_utc); }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(dt) = latest_asset_dt {
                    return Ok((api_tag, dt.format("%d.%m.%Y").to_string()));
                }

                if let Some(published) = json["published_at"].as_str() {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(published) {
                        return Ok((api_tag, dt.format("%d.%m.%Y").to_string()));
                    }
                }
            }
        }
    }

    // Priority 2: Fallback HTML scraping (scan ALL body dates and datetime tags, picking the latest)
    let release_urls = [
        "https://github.com/Okaetsu/RE-UE4SS/releases/latest",
        "https://github.com/Okaetsu/RE-UE4SS/releases/tag/experimental-palworld",
    ];

    for target_url in release_urls {
        if let Ok(resp) = client.get(target_url).send().await {
            if let Ok(html) = resp.text().await {
                let mut latest_dt: Option<chrono::NaiveDate> = None;

                // 1. Scan text dates in markdown body (e.g. "Updated on 28th of August 2026")
                let text_date_re = regex::Regex::new(r"(?i)(\d{1,2})(?:st|nd|rd|th)?\s+(?:of\s+)?(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{4})").unwrap();
                for cap in text_date_re.captures_iter(&html) {
                    if let (Some(d_match), Some(m_match), Some(y_match)) = (cap.get(1), cap.get(2), cap.get(3)) {
                        let month_opt = match m_match.as_str().to_lowercase().as_str() {
                            "january" | "jan" => Some(1),
                            "february" | "feb" => Some(2),
                            "march" | "mar" => Some(3),
                            "april" | "apr" => Some(4),
                            "may" => Some(5),
                            "june" | "jun" => Some(6),
                            "july" | "jul" => Some(7),
                            "august" | "aug" => Some(8),
                            "september" | "sep" | "sept" => Some(9),
                            "october" | "oct" => Some(10),
                            "november" | "nov" => Some(11),
                            "december" | "dec" => Some(12),
                            _ => None,
                        };
                        if let (Ok(day), Some(month), Ok(year)) = (
                            d_match.as_str().parse::<u32>(),
                            month_opt,
                            y_match.as_str().parse::<i32>(),
                        ) {
                            if let Some(nd) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
                                match latest_dt {
                                    Some(cur) if nd > cur => { latest_dt = Some(nd); }
                                    None => { latest_dt = Some(nd); }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                // 2. Scan datetime="..." attributes
                let mut cursor = 0;
                while let Some(pos) = html[cursor..].find("datetime=\"") {
                    let time_start = cursor + pos + 10;
                    if let Some(len) = html[time_start..].find('"') {
                        let dt_raw = &html[time_start..time_start + len];
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_raw) {
                            let nd = dt.date_naive();
                            match latest_dt {
                                Some(cur) if nd > cur => { latest_dt = Some(nd); }
                                None => { latest_dt = Some(nd); }
                                _ => {}
                            }
                        }
                        cursor = time_start + len;
                    } else {
                        break;
                    }
                }

                if let Some(nd) = latest_dt {
                    return Ok((default_tag, nd.format("%d.%m.%Y").to_string()));
                }
            }
        }
    }

    Err("Could not determine latest UE4SS date from GitHub".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_ue4ss_latest() {
        tauri::async_runtime::block_on(async {
            let result = check_ue4ss_latest().await;
            assert!(result.is_ok(), "check_ue4ss_latest failed: {:?}", result.err());
            let (tag, date_str) = result.unwrap();
            assert_eq!(tag, "experimental-palworld");
            assert!(date_str.ends_with("2026"), "Expected 2026 date for UE4SS, got: {}", date_str);
        });
    }
}


