use std::path::Path;
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub async fn launch_game(state: State<'_, AppState>) -> Result<(), String> {
    let (game_path, enabled_mod_ids, all_mods, profile_name) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let current_profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
        let profile_name = current_profile.map(|p| p.name.clone()).unwrap_or_else(|| "Default".to_string());
        let enabled_ids = if let Some(p) = current_profile {
            p.enabled_mod_ids.clone()
        } else {
            data.mods.iter().filter(|m| m.enabled).map(|m| m.id.clone()).collect()
        };
        (data.settings.game_path.clone(), enabled_ids, data.mods.clone(), profile_name)
    };

    if game_path.is_empty() {
        return Err("Game path is not set in settings".to_string());
    }

    let path = Path::new(&game_path);
    let _ = crate::altermatic::sync_load_list(path, &enabled_mod_ids, &all_mods);
    let wingdk = path.join("Pal").join("Binaries").join("WinGDK");
    let is_xbox = wingdk.exists();

    let enabled_mods: Vec<_> = all_mods
        .iter()
        .filter(|m| enabled_mod_ids.contains(&m.id))
        .collect();

    crate::logger::log(&format!(
        "launch_game: Launching Palworld (Platform: {}, Profile: '{}', Active mods: {})...",
        if is_xbox { "Xbox/WinGDK" } else { "Steam/Win64" },
        profile_name,
        enabled_mods.len()
    ));

    if enabled_mods.is_empty() {
        crate::logger::log("launch_game: [Active Mods Launch List]: None (Vanilla launch)");
    } else {
        crate::logger::log("launch_game: [Active Mods Launch List]:");
        for (idx, m) in enabled_mods.iter().enumerate() {
            let ver = if m.version.trim().is_empty() { "1.0.0" } else { &m.version };
            crate::logger::log(&format!(
                "  [{:>2}] {} (v{}) [Type: {:?}]",
                idx + 1,
                m.name,
                ver,
                m.mod_type
            ));
        }
    }

    if is_xbox {
        let exe_path = wingdk.join("Palworld-WinGDK-Shipping.exe");
        if exe_path.exists() {
            std::process::Command::new(&exe_path)
                .current_dir(wingdk)
                .spawn()
                .map_err(|e| format!("Failed to launch Game Pass version: {}", e))?;
        } else {
            return Err("Palworld-WinGDK-Shipping.exe not found in Binaries/WinGDK".to_string());
        }
    } else {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("cmd")
                .args(&["/C", "start", "", "steam://run/1623730"])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn()
                .map_err(|e| format!("Failed to launch Steam version: {}", e))?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let launched = crate::system_open::open_url_in_system("steam://run/1623730");
            if let Err(e) = launched {
                crate::logger::log(&format!("launch_game: xdg-open steam protocol failed ({}), trying direct steam binary...", e));
                let mut cmd = std::process::Command::new("steam");
                cmd.arg("steam://run/1623730");
                cmd.env_remove("LD_LIBRARY_PATH");
                cmd.spawn().map_err(|e2| format!("Failed to launch game via Steam: {} (fallback: {})", e, e2))?;
            }
        }
    }

    crate::logger::log("launch_game: Game process spawned successfully");
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamProtocolStatus {
    pub registered: bool,
    pub handler: Option<String>,
    pub platform: String,
}

pub fn check_steam_protocol_status() -> SteamProtocolStatus {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

        let command_path = "Software\\Classes\\steam\\shell\\open\\command";
        let fallback_path = "steam\\shell\\open\\command";

        let handler = hkcu
            .open_subkey(command_path)
            .and_then(|k| k.get_value::<String, _>(""))
            .or_else(|_| {
                hkcr.open_subkey(fallback_path)
                    .and_then(|k| k.get_value::<String, _>(""))
            })
            .ok();

        SteamProtocolStatus {
            registered: handler.is_some(),
            handler,
            platform: "windows".to_string(),
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("xdg-mime")
            .args(["query", "default", "x-scheme-handler/steam"])
            .output()
            .ok();

        let handler_str = output
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty());

        if let Some(h) = handler_str {
            return SteamProtocolStatus {
                registered: h.contains("steam"),
                handler: Some(h),
                platform: "linux".to_string(),
            };
        }

        let local_desktop = std::env::var_os("HOME").map(|h| {
            std::path::PathBuf::from(h).join(".local/share/applications/steam.desktop")
        });
        let sys_desktop = std::path::Path::new("/usr/share/applications/steam.desktop");

        let exists = local_desktop.map(|p| p.exists()).unwrap_or(false) || sys_desktop.exists();
        SteamProtocolStatus {
            registered: exists,
            handler: if exists { Some("steam.desktop".to_string()) } else { None },
            platform: "linux".to_string(),
        }
    }

    #[cfg(target_os = "macos")]
    {
        let steam_app = std::path::Path::new("/Applications/Steam.app");
        let exists = steam_app.exists();
        SteamProtocolStatus {
            registered: exists,
            handler: if exists { Some("Steam.app".to_string()) } else { None },
            platform: "macos".to_string(),
        }
    }
}

#[tauri::command]
pub fn check_steam_protocol() -> SteamProtocolStatus {
    check_steam_protocol_status()
}
