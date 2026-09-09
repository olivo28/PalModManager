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

pub fn detect_configs(mod_path: &Path) -> (Option<String>, Option<Vec<String>>) {
    let mut configs: Vec<String> = Vec::new();
    let folder_name = mod_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let clean_folder = folder_name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");

    // 1. Check UE4SS shared/<ModName>/ directory first (high priority active user configs)
    if let Some(parent) = mod_path.parent() {
        let shared_root = parent.join("shared");
        if shared_root.is_dir() {
            let mut matching_dirs = Vec::new();
            let exact_dir = shared_root.join(folder_name);
            if exact_dir.is_dir() {
                matching_dirs.push(exact_dir);
            }
            if let Ok(entries) = fs::read_dir(&shared_root) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() && !matching_dirs.contains(&p) {
                        let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                        let clean_sub = name.replace(|c: char| !c.is_alphanumeric(), "");
                        if !clean_folder.is_empty() && clean_sub == clean_folder {
                            matching_dirs.push(p);
                        }
                    }
                }
            }

            for s_dir in matching_dirs {
                let subfolder_name = s_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
                if let Ok(files) = fs::read_dir(&s_dir) {
                    for file_res in files.flatten() {
                        let f_path = file_res.path();
                        if f_path.is_file() {
                            if let Some(ext) = f_path.extension().and_then(|e| e.to_str()) {
                                let ext_lower = ext.to_lowercase();
                                if ["lua", "json", "jsonc", "ini", "cfg", "txt"].contains(&ext_lower.as_str()) {
                                    let fname = f_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                    if !fname.ends_with(".bak") && !fname.starts_with('.') {
                                        let rel = format!("shared/{}/{}", subfolder_name, fname);
                                        if !configs.contains(&rel) {
                                            configs.push(rel);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Check mod_path internal directory
    let config_names = [
        "config.json",
        "config.jsonc",
        "settings.json",
        "options.json",
        "config.lua",
        "settings.lua",
        "config.cfg",
        "config.ini",
    ];
    for name in &config_names {
        if mod_path.join(name).exists() {
            let s = name.to_string();
            if !configs.contains(&s) {
                configs.push(s);
            }
        }
    }

    let config_subdirs = ["config", "settings", "scripts"];
    for subdir in &config_subdirs {
        let sub_path = mod_path.join(subdir);
        if sub_path.is_dir() {
            for name in &config_names {
                let full = sub_path.join(name);
                if full.exists() {
                    let rel = format!("{}/{}", subdir, name);
                    if !configs.iter().any(|c| c.eq_ignore_ascii_case(&rel)) {
                        configs.push(rel);
                    }
                }
            }

            if let Ok(entries) = fs::read_dir(&sub_path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let name_lower = name.to_lowercase();
                    if p.is_file() && (name_lower.ends_with(".lua") || name_lower.ends_with(".json") || name_lower.ends_with(".ini")) {
                        if (name_lower.contains("config") || name_lower.contains("setting") || name_lower.contains("option"))
                            && !name_lower.contains("do_not_edit")
                            && !name_lower.ends_with("manager.lua")
                            && !name_lower.ends_with("handler.lua")
                            && !name_lower.ends_with("helper.lua")
                            && !name_lower.ends_with("service.lua")
                        {
                            let rel = format!("{}/{}", subdir, name);
                            if !configs.iter().any(|c| c.eq_ignore_ascii_case(&rel)) {
                                configs.push(rel);
                            }
                        }
                    }
                }
            }
        }
    }

    if configs.is_empty() {
        (None, None)
    } else {
        let primary = configs[0].clone();
        (Some(primary), Some(configs))
    }
}

pub fn detect_config(mod_path: &Path) -> Option<String> {
    detect_configs(mod_path).0
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
        // Project mods created in Mod Studio are workspace projects and always accessible
        if m.origin_load_method.as_deref() == Some("project") || m.id.starts_with("project_") {
            return true;
        }

        let is_native = m.nexus_author.as_deref() == Some("UE4SS Native Mod");
        if is_native {
            return ue4ss_enabled;
        }
        let is_workshop = m.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod"))
            || m.game_path.to_lowercase().contains("nativemods")
            || m.game_path.to_lowercase().contains("workshop");
        if is_workshop && !ue4ss_enabled {
            return false;
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
