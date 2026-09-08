use std::path::Path;
use std::fs;
use super::models::InstalledBuildInfo;

pub fn detect_installed_game_build(game_path: &Path) -> InstalledBuildInfo {
    // 1. Check Steam appmanifest (e.g., steamapps/common/Palworld -> steamapps/appmanifest_1623730.acf)
    if let Some(steamapps_dir) = game_path.parent().and_then(|p| p.parent()) {
        let manifest_path = steamapps_dir.join("appmanifest_1623730.acf");
        if manifest_path.exists() {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                let mut app_id = None;
                let mut build_id = None;
                let mut last_updated = None;

                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("\"appid\"") {
                        if let Some(val) = extract_vdf_value(trimmed) {
                            app_id = val.parse::<u32>().ok();
                        }
                    } else if trimmed.starts_with("\"buildid\"") {
                        if let Some(val) = extract_vdf_value(trimmed) {
                            build_id = Some(val.to_string());
                        }
                    } else if trimmed.starts_with("\"LastUpdated\"") {
                        if let Some(val) = extract_vdf_value(trimmed) {
                            last_updated = val.parse::<u64>().ok();
                        }
                    }
                }

                if build_id.is_some() {
                    let master = super::master_manifest::get_or_load_master_manifest("");
                    let game_version = master.map(|m| super::master_manifest::resolve_game_version(&m, build_id.as_deref()));

                    return InstalledBuildInfo {
                        app_id: app_id.or(Some(1623730)),
                        build_id,
                        game_version,
                        source: "Steam appmanifest".to_string(),
                        is_steam: true,
                        is_gamepass: false,
                        last_updated_timestamp: last_updated,
                    };
                }
            }
        }
    }

    // 2. Check for Xbox Game Pass (WinGDK) indicators
    let is_gp = game_path.join("AppxManifest.xml").exists()
        || game_path.join("MicrosoftGame.config").exists()
        || game_path.to_string_lossy().to_lowercase().contains("xboxgames")
        || game_path.to_string_lossy().to_lowercase().contains("content");

    if is_gp {
        let master = super::master_manifest::get_or_load_master_manifest("");
        let game_version = master.map(|m| format!("{} (Game Pass)", m.latest_game_version));

        return InstalledBuildInfo {
            app_id: None,
            build_id: None,
            game_version,
            source: "Xbox Game Pass (WinGDK)".to_string(),
            is_steam: false,
            is_gamepass: true,
            last_updated_timestamp: None,
        };
    }

    // 3. Check for Palworld binary presence
    let has_exe = game_path.join("Pal").join("Binaries").join("Win64").join("Palworld-Win64-Shipping.exe").exists()
        || game_path.join("Palworld.exe").exists();

    let master = super::master_manifest::get_or_load_master_manifest("");
    let game_version = if has_exe { master.map(|m| m.latest_game_version) } else { None };

    InstalledBuildInfo {
        app_id: if has_exe { Some(1623730) } else { None },
        build_id: None,
        game_version,
        source: if has_exe { "Standalone / Non-Steam".to_string() } else { "Unknown Location".to_string() },
        is_steam: false,
        is_gamepass: false,
        last_updated_timestamp: None,
    }
}

fn extract_vdf_value(line: &str) -> Option<&str> {
    let mut parts = line.split('"').filter(|s| !s.trim().is_empty());
    let _key = parts.next()?;
    let val = parts.next()?;
    Some(val.trim())
}
