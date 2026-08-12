use std::fs;
use std::path::Path;
use crate::models::{ModInfo, ModType, AppData};

pub fn get_physical_identity(game_path: &str, disabled_path: &str) -> String {
    let path_str = if !game_path.is_empty() { game_path } else { disabled_path };
    if path_str.is_empty() {
        return String::new();
    }
    let path = Path::new(path_str);
    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

pub fn file_install_date(path: &Path) -> String {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let naive = chrono::DateTime::from_timestamp(secs as i64, 0)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            naive
        })
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn detect_config(mod_path: &Path) -> Option<String> {
    let config_names = ["config.json", "settings.json", "options.json"];
    for name in &config_names {
        if mod_path.join(name).exists() {
            return Some(name.to_string());
        }
    }
    let config_subdirs = ["config", "settings"];
    for subdir in &config_subdirs {
        for name in &config_names {
            let full = mod_path.join(subdir).join(name);
            if full.exists() {
                return Some(format!("{}/{}", subdir, name));
            }
        }
    }
    if let Ok(rd) = fs::read_dir(mod_path) {
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                return Some(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
    }
    None
}

pub fn filter_mods_for_current_profile_pub(data: &AppData) -> Vec<ModInfo> {
    filter_mods_for_current_profile(data)
}

pub fn filter_mods_for_current_profile(data: &AppData) -> Vec<ModInfo> {
    let current_id = &data.current_profile_id;
    let profile = data.profiles.iter().find(|p| p.id == *current_id);
    let ue4ss_enabled = profile.map(|p| p.ue4ss_enabled).unwrap_or(false);
    let palschema_enabled = profile.map(|p| p.palschema_enabled).unwrap_or(false);

    data.mods.iter().filter(|m| {
        let is_native = m.nexus_author.as_deref() == Some("UE4SS Native Mod");
        if is_native {
            return ue4ss_enabled;
        }
        let is_workshop = m.game_path.to_lowercase().contains("nativemods") || m.game_path.to_lowercase().contains("workshop");
        if is_workshop {
            return ue4ss_enabled;
        }
        let is_palschema = m.mod_type == ModType::PalSchema;
        if is_palschema && !palschema_enabled && !ue4ss_enabled {
            return false;
        }
        if let Some(prof) = profile {
            prof.installed_mod_ids.iter().any(|entry| {
                crate::profiles::mod_matches_profile_entry(m, entry)
            })
        } else {
            false
        }
    }).cloned().collect()
}
