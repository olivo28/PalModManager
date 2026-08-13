use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::models::{ModInfo, ModType, WorkshopInstallType};
use super::utils::{get_physical_identity, file_install_date, detect_config};

fn load_from_modinfo_pmm_json(path: &Path) -> Option<ModInfo> {
    if !path.is_dir() {
        return None;
    }
    let pmm_path = path.join("modinfo.pmm.json");
    if !pmm_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&pmm_path).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    let name = meta.get("name").and_then(|n| n.as_str()).unwrap_or("Scanned Mod").to_string();
    let version = meta.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0").to_string();
    let description = meta.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string();
    let author = meta.get("author").and_then(|a| a.as_str()).map(|s| s.to_string());
    let nexus_id = meta.get("nexusModId").and_then(|id| id.as_u64()).map(|n| n as u32);
    let mod_type_str = meta.get("modType").and_then(|t| t.as_str()).unwrap_or("ue4ss");
    
    let mod_type = match mod_type_str.to_lowercase().as_str() {
        "ue4ss" => ModType::Ue4ss,
        "palschema" => ModType::PalSchema,
        "pak" => ModType::Pak,
        "logicmods" => ModType::LogicMods,
        _ => ModType::Hybrid,
    };

    let path_str = path.to_string_lossy().to_string();
    let is_disabled = path_str.contains("disabled_mods");
    
    let game_path = if is_disabled { String::new() } else { path_str.clone() };
    let disabled_path = if is_disabled { path_str.clone() } else { String::new() };

    Some(ModInfo {
        id: name.clone(),
        name: name.clone(),
        mod_type,
        nexus_mod_id: nexus_id,
        nexus_url: nexus_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id)),
        nexus_author: author,
        nexus_summary: Some(description),
        nexus_picture_url: None,
        nexus_endorsements: None,
        nexus_downloads: None,
        version,
        install_date: file_install_date(path),
        source_zip: String::new(),
        config_path: detect_config(path),
        config_type: Some("auto".to_string()),
        enabled: !is_disabled,
        game_path,
        disabled_path,
        pak_destination: None,
        has_enabled_txt: path.join("enabled.txt").exists(),
        mods_txt_order: None,
        extra_files: Vec::new(),
        nexus_description: None,
        nexus_version_cached: None,
        nexus_cached_at: None,
        nexus_category: None,
        nexus_tags: Vec::new(),
        github_repo: None,
        github_version: None,
        github_cached_at: None,
        update_date: None,
        library_zip: None,
        ignored_version: None,
        nexus_file_id: None,
        ignored_keys: None,
    })
}

fn load_pmm_meta(path: &Path) -> Option<ModInfo> {
    let pmm_path = if path.is_file() {
        PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()))
    } else {
        path.join(".pmm.json")
    };
    if pmm_path.exists() {
        if let Ok(content) = fs::read_to_string(&pmm_path) {
            if let Ok(mut mod_info) = serde_json::from_str::<ModInfo>(&content) {
                let path_str = path.to_string_lossy().to_string();
                let is_disabled = path_str.contains("disabled_mods");
                if is_disabled {
                    mod_info.disabled_path = path_str;
                    mod_info.game_path = String::new();
                    mod_info.enabled = false;
                } else {
                    mod_info.game_path = path_str;
                    mod_info.disabled_path = String::new();
                    if path.is_file() {
                        mod_info.enabled = true;
                    }
                }
                return Some(mod_info);
            }
        }
    }
    if path.is_dir() {
        if let Some(m) = load_from_modinfo_pmm_json(path) {
            return Some(m);
        }
    }
    None
}

fn scan_ue4ss_mods(dir: &Path, results: &mut Vec<ModInfo>, ignored_names: &std::collections::HashSet<String>) {
    if !dir.exists() { return; }

    let mods_txt_path = dir.join("mods.txt");
    let mut mods_txt_states: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    if mods_txt_path.exists() {
        if let Ok(content) = fs::read_to_string(&mods_txt_path) {
            for line in content.lines() {
                let line_clean = line.trim();
                if line_clean.starts_with(';') || line_clean.starts_with("//") {
                    continue;
                }
                if let Some(pos) = line_clean.find(':') {
                    let name = line_clean[..pos].trim().to_lowercase();
                    let val = line_clean[pos+1..].trim();
                    mods_txt_states.insert(name, val == "1");
                } else if !line_clean.is_empty() {
                    mods_txt_states.insert(line_clean.to_lowercase(), true);
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) { continue; }
            let mod_name = entry.file_name().to_string_lossy().to_string();
            if ignored_names.contains(&mod_name.to_lowercase()) { continue; }
            let mod_path = entry.path();
            if ["ConsoleUnlocker", "LuaPlugin", "PalSchema"].contains(&mod_name.as_str()) { continue; }

            let is_native_mod = ["BPModLoaderMod", "CheatManagerEnablerMod", "ConsoleCommandsMod", "ConsoleEnablerMod", "Keybinds", "LineTraceMod", "SplitScreenMod", "BPML_GenericFunctions", "shared", "adapters"].contains(&mod_name.as_str());

            let is_enabled = if let Some(&state) = mods_txt_states.get(&mod_name.to_lowercase()) {
                state
            } else {
                mod_path.join("enabled.txt").exists() || is_native_mod
            };

            if let Some(mut m) = load_pmm_meta(&mod_path) {
                m.enabled = is_enabled;
                results.push(m);
                continue;
            }

            let author = if is_native_mod { Some("UE4SS Native Mod".to_string()) } else { None };
            let summary = if is_native_mod { Some("Core dependency mod installed by UE4SS. Controlled by mods.txt.".to_string()) } else { None };

            results.push(ModInfo {
                id: mod_name.clone(),
                name: mod_name.clone(),
                mod_type: ModType::Ue4ss,
                nexus_mod_id: None, nexus_url: None, nexus_author: author, nexus_summary: summary,
                nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                version: "1.0.0".to_string(),
                install_date: file_install_date(&mod_path),
                source_zip: String::new(),
                config_path: detect_config(&mod_path),
                config_type: Some("auto".to_string()),
                enabled: is_enabled,
                game_path: mod_path.to_string_lossy().to_string(),
                disabled_path: String::new(),
                pak_destination: None,
                has_enabled_txt: mod_path.join("enabled.txt").exists(),
                mods_txt_order: None,
                extra_files: Vec::new(),
                nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                nexus_category: None, nexus_tags: Vec::new(),
                github_repo: None, github_version: None, github_cached_at: None,
                update_date: None, library_zip: None,
                ignored_version: None,
                nexus_file_id: None,
                ignored_keys: None,
            });
        }
    }
}

fn scan_palschema_mods(dir: &Path, results: &mut Vec<ModInfo>, ignored_names: &std::collections::HashSet<String>) {
    if !dir.exists() { return; }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) { continue; }
            let mod_name = entry.file_name().to_string_lossy().to_string();
            if ignored_names.contains(&mod_name.to_lowercase()) { continue; }
            let mod_path = entry.path();

            if let Some(m) = load_pmm_meta(&mod_path) {
                results.push(m);
                continue;
            }

            let has_json = WalkDir::new(&mod_path).max_depth(2).into_iter().filter_map(|e| e.ok()).any(|e| {
                e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "json" || ext == "jsonc")
            });

            if has_json {
                let install_date = file_install_date(&mod_path);
                results.push(ModInfo {
                    id: mod_name.clone(),
                    name: mod_name.clone(),
                    mod_type: ModType::PalSchema,
                    nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
                    nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                    version: "unknown".to_string(), install_date,
                    source_zip: String::new(), config_path: detect_config(&mod_path),
                    config_type: Some("auto".to_string()), enabled: true,
                    game_path: mod_path.to_string_lossy().to_string(),
                    disabled_path: String::new(),
                    pak_destination: None, has_enabled_txt: false, mods_txt_order: None,
                    extra_files: Vec::new(),
                    nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                    nexus_category: None, nexus_tags: Vec::new(),
                    github_repo: None, github_version: None, github_cached_at: None,
                    update_date: None, library_zip: None,
                    ignored_version: None,
                    nexus_file_id: None,
                    ignored_keys: None,
                });
            }
        }
    }
}

fn scan_pak_mods(dir: &Path, pak_type: &str, results: &mut Vec<ModInfo>) {
    if !dir.exists() { return; }
    for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        let ext = entry.path().extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        if ext != "pak" { continue; }

        let mod_path = entry.path();
        if let Some(m) = load_pmm_meta(&mod_path) {
            results.push(m);
            continue;
        }

        let file_stem = entry.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "unknown".to_string());
        let mod_name = file_stem.trim_end_matches("_P").to_string();
        let mod_path = entry.path();
        let install_date = file_install_date(mod_path);
        let mod_path_str = mod_path.to_string_lossy().to_string();
        let mut extra_files: Vec<String> = Vec::new();
        for companion_ext in &["ucas", "utoc"] {
            let companion_path = dir.join(format!("{}.{}", file_stem, companion_ext));
            if companion_path.exists() {
                extra_files.push(companion_path.to_string_lossy().to_string());
            }
        }
        let mt = if pak_type == "logicmods" { ModType::LogicMods } else { ModType::Pak };
        results.push(ModInfo {
            id: mod_name.clone(), name: mod_name.clone(), mod_type: mt,
            nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
            nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
            version: "unknown".to_string(), install_date,
            source_zip: String::new(), config_path: None, config_type: None,
            enabled: true, game_path: mod_path_str, disabled_path: String::new(),
            pak_destination: Some(pak_type.to_string()), has_enabled_txt: false,
            mods_txt_order: None, extra_files,
            nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
            nexus_category: None, nexus_tags: Vec::new(),
            github_repo: None,
            github_version: None,
            github_cached_at: None,
            update_date: None,
            library_zip: None,
            ignored_version: None,
            nexus_file_id: None,
            ignored_keys: None,
        });
    }
}

fn scan_disabled_mods(disabled_base: &Path, results: &mut Vec<ModInfo>) {
    let type_dirs = [("ue4ss", ModType::Ue4ss), ("palschema", ModType::PalSchema)];
    for (type_str, mod_type) in &type_dirs {
        let dir = disabled_base.join(type_str);
        if !dir.exists() { continue; }
        if let Ok(rd) = fs::read_dir(&dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if !entry.file_type().map_or(false, |ft| ft.is_dir()) { continue; }
                let mod_name = entry.file_name().to_string_lossy().to_string();
                let mod_path = entry.path();

                if let Some(m) = load_pmm_meta(&mod_path) {
                    results.push(m);
                    continue;
                }

                let install_date = file_install_date(&mod_path);
                results.push(ModInfo {
                    id: mod_name.clone(), name: mod_name.clone(), mod_type: mod_type.clone(),
                    nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
                    nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                    version: "unknown".to_string(), install_date,
                    source_zip: String::new(), config_path: detect_config(&mod_path),
                    config_type: Some("auto".to_string()), enabled: false,
                    game_path: String::new(), disabled_path: mod_path.to_string_lossy().to_string(),
                    pak_destination: None, has_enabled_txt: mod_path.join("enabled.txt").exists(),
                    mods_txt_order: None, extra_files: Vec::new(),
                    nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                    nexus_category: None, nexus_tags: Vec::new(),
                    github_repo: None,
                    github_version: None,
                    github_cached_at: None,
                    update_date: None,
                    library_zip: None,
                    ignored_version: None,
                    nexus_file_id: None,
                    ignored_keys: None,
                });
            }
        }
    }

    for (type_str, pak_type) in &[("pak", "pak"), ("logicmods", "logicmods")] {
        let dir = disabled_base.join(type_str);
        if !dir.exists() { continue; }
        if let Ok(rd) = fs::read_dir(&dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if !entry.file_type().map_or(false, |ft| ft.is_file()) { continue; }
                let ext = entry.path().extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                if ext != "pak" { continue; }

                let mod_path = entry.path();
                if let Some(m) = load_pmm_meta(&mod_path) {
                    results.push(m);
                    continue;
                }

                let file_stem = entry.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let mod_name = file_stem.trim_end_matches("_P").to_string();
                let install_date = file_install_date(&entry.path());
                let mt = if *pak_type == "logicmods" { ModType::LogicMods } else { ModType::Pak };
                results.push(ModInfo {
                    id: mod_name.clone(), name: mod_name.clone(), mod_type: mt,
                    nexus_mod_id: None, nexus_url: None, nexus_author: None, nexus_summary: None,
                    nexus_picture_url: None, nexus_endorsements: None, nexus_downloads: None,
                    version: "unknown".to_string(), install_date,
                    source_zip: String::new(), config_path: None, config_type: None,
                    enabled: false, game_path: String::new(),
                    disabled_path: entry.path().to_string_lossy().to_string(),
                    pak_destination: Some(pak_type.to_string()), has_enabled_txt: false,
                    mods_txt_order: None, extra_files: Vec::new(),
                    nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                    nexus_category: None, nexus_tags: Vec::new(),
                    github_repo: None,
                    github_version: None,
                    github_cached_at: None,
                    update_date: None,
                    library_zip: None,
                    ignored_version: None,
                    nexus_file_id: None,
                    ignored_keys: None,
                });
            }
        }
    }
}

pub fn scan_mods_internal(
    game_path: &str,
    program_path: &str,
    current_profile_id: &str,
    installed_ids: &[String],
    db_mods: &[ModInfo],
) -> Vec<ModInfo> {
    if game_path.is_empty() {
        return Vec::new();
    }
    let game = PathBuf::from(game_path);
    let mut fs_mods: Vec<ModInfo> = vec![];

    let wmods = crate::workshop::scan_workshop_mods(game_path);
    let workshop_package_names: std::collections::HashSet<String> = wmods.iter()
        .map(|m| m.package_name.to_lowercase())
        .collect();

    let gp = crate::dependency_checker::build_game_profile(&game);
    let ue4ss_mods_dir = gp.ue4ss_mods_dir.clone();
    if ue4ss_mods_dir.exists() {
        scan_ue4ss_mods(&ue4ss_mods_dir, &mut fs_mods, &workshop_package_names);
    }

    let palschema_dir = gp.palschema_mods_dir.clone();
    if palschema_dir.exists() {
        scan_palschema_mods(&palschema_dir, &mut fs_mods, &workshop_package_names);
    }

    let pak_mods_dir = game.join("Pal").join("Content").join("Paks").join("~mods");
    if pak_mods_dir.exists() {
        scan_pak_mods(&pak_mods_dir, "pak", &mut fs_mods);
    }

    let logic_mods_dir = game.join("Pal").join("Content").join("Paks").join("LogicMods");
    if logic_mods_dir.exists() {
        scan_pak_mods(&logic_mods_dir, "logicmods", &mut fs_mods);
    }

    let disabled_base = PathBuf::from(program_path)
        .join("profiles")
        .join(current_profile_id)
        .join("disabled_mods");
    if disabled_base.exists() {
        scan_disabled_mods(&disabled_base, &mut fs_mods);
    }

    for wmod in wmods.iter().filter(|m| !m.is_framework) {
        let game_mod_path = if wmod.install_type == WorkshopInstallType::PalSchemaMod {
            gp.palschema_mods_dir.join(&wmod.package_name)
        } else {
            gp.ue4ss_mods_dir.join(&wmod.package_name)
        };
        
        let mut details = format!(
            "Steam Workshop Mod\n\n• Workshop ID: {}\n• Author: {}\n• Package Name: {}\n• Install Type: {:?}",
            wmod.workshop_id,
            wmod.author,
            wmod.package_name,
            wmod.install_type
        );
        if !wmod.dependencies.is_empty() {
            details.push_str(&format!("\n• Dependencies: {}", wmod.dependencies.join(", ")));
        }

        let display_name = format!("{} (Workshop)", wmod.mod_name);
        let installed_version = if wmod.is_installed {
            let manifest_dir = game.join("Mods").join("ManagedMods").join(&wmod.package_name);
            let installed_info_path = manifest_dir.join("Info.json");
            let mut inst_ver = "unknown".to_string();
            if installed_info_path.exists() {
                if let Ok(inst_info_str) = fs::read_to_string(&installed_info_path) {
                    if let Ok(inst_info) = serde_json::from_str::<crate::workshop::WorkshopInfoJson>(&inst_info_str) {
                        inst_ver = inst_info.version;
                    }
                }
            }
            inst_ver
        } else {
            "unknown".to_string()
        };

        fs_mods.push(ModInfo {
            id: wmod.package_name.clone(),
            name: display_name,
            mod_type: match wmod.install_type {
                WorkshopInstallType::PalSchemaMod => ModType::PalSchema,
                _ => ModType::Ue4ss,
            },
            nexus_mod_id: None,
            nexus_url: Some(format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", wmod.workshop_id)),
            nexus_author: Some(wmod.author.clone()),
            nexus_summary: Some(details),
            nexus_picture_url: wmod.thumbnail_path.clone(),
            nexus_endorsements: None,
            nexus_downloads: None,
            version: installed_version,
            install_date: String::new(),
            source_zip: String::new(),
            config_path: None,
            config_type: Some("auto".to_string()),
            enabled: wmod.is_active,
            game_path: game_mod_path.to_string_lossy().to_string(),
            disabled_path: String::new(),
            pak_destination: None,
            has_enabled_txt: game_mod_path.join("enabled.txt").exists(),
            mods_txt_order: None,
            extra_files: Vec::new(),
            nexus_description: None,
            nexus_version_cached: Some(wmod.version.clone()),
            nexus_cached_at: None,
            nexus_category: None,
            nexus_tags: Vec::new(),
            github_repo: None,
            github_version: None,
            github_cached_at: None,
            update_date: None,
            library_zip: None,
            ignored_version: None,
            nexus_file_id: None,
            ignored_keys: None,
            has_pending_update: Some(wmod.has_pending_update),
        });
    }

    merge_scan_with_db(current_profile_id, installed_ids, db_mods, &fs_mods, &workshop_package_names)
}

fn merge_scan_with_db(
    current_profile_id: &str,
    installed_ids: &[String],
    db_mods: &[ModInfo],
    fs_mods: &[ModInfo],
    workshop_package_names: &std::collections::HashSet<String>,
) -> Vec<ModInfo> {
    let mut consolidated_db: Vec<ModInfo> = Vec::new();
    for db_mod in db_mods {
        let db_path_norm = if !db_mod.game_path.is_empty() {
            db_mod.game_path.replace("\\", "/").to_lowercase()
        } else {
            db_mod.disabled_path.replace("\\", "/").to_lowercase()
        };

        if !db_path_norm.is_empty() {
            let is_extra_of_other = db_mods.iter().any(|other| {
                other.id != db_mod.id && other.extra_files.iter().any(|extra| {
                    extra.replace("\\", "/").to_lowercase() == db_path_norm
                })
            });
            if is_extra_of_other {
                crate::logger::log(&format!("merge_scan_with_db: Purging duplicate sub-component mod '{}' because its files are owned by another mod.", db_mod.name));
                continue;
            }
        }

        let db_id = get_physical_identity(&db_mod.game_path, &db_mod.disabled_path);
        if let Some(existing_idx) = consolidated_db.iter().position(|m| {
            if m.mod_type != db_mod.mod_type {
                return false;
            }
            let existing_id = get_physical_identity(&m.game_path, &m.disabled_path);
            existing_id == db_id
        }) {
            let existing = &mut consolidated_db[existing_idx];
            if existing.nexus_mod_id.is_none() && db_mod.nexus_mod_id.is_some() {
                existing.nexus_mod_id = db_mod.nexus_mod_id;
                existing.nexus_url = db_mod.nexus_url.clone();
                existing.nexus_author = db_mod.nexus_author.clone();
                existing.nexus_summary = db_mod.nexus_summary.clone();
                existing.nexus_picture_url = db_mod.nexus_picture_url.clone();
                existing.nexus_endorsements = db_mod.nexus_endorsements;
                existing.nexus_downloads = db_mod.nexus_downloads;
                existing.name = db_mod.name.clone();
            }
            if existing.game_path.is_empty() && !db_mod.game_path.is_empty() {
                existing.game_path = db_mod.game_path.clone();
                existing.enabled = db_mod.enabled;
            }
            if existing.disabled_path.is_empty() && !db_mod.disabled_path.is_empty() {
                existing.disabled_path = db_mod.disabled_path.clone();
            }
        } else {
            consolidated_db.push(db_mod.clone());
        }
    }

    let mut result: Vec<ModInfo> = Vec::new();
    let mut matched_db: Vec<bool> = vec![false; consolidated_db.len()];

    for fs_mod in fs_mods {
        let fs_path_norm = if !fs_mod.game_path.is_empty() {
            fs_mod.game_path.replace("\\", "/").to_lowercase()
        } else {
            fs_mod.disabled_path.replace("\\", "/").to_lowercase()
        };

        if !fs_path_norm.is_empty() {
            let is_registered_as_extra = consolidated_db.iter().any(|dm| {
                dm.extra_files.iter().any(|extra| {
                    extra.replace("\\", "/").to_lowercase() == fs_path_norm
                })
            });
            if is_registered_as_extra {
                continue;
            }
        }

        let fs_id = get_physical_identity(&fs_mod.game_path, &fs_mod.disabled_path);
        if let Some(db_idx) = consolidated_db.iter().position(|dm| {
            let dm_idx = consolidated_db.iter().position(|x| x as *const _ == dm as *const _).unwrap_or(0);
            if matched_db[dm_idx] {
                return false;
            }
            let db_id = get_physical_identity(&dm.game_path, &dm.disabled_path);
            (db_id == fs_id && !db_id.is_empty() && dm.mod_type == fs_mod.mod_type) || 
            (dm.id.to_lowercase() == fs_mod.id.to_lowercase()) ||
            (dm.name.to_lowercase() == fs_mod.name.to_lowercase() && dm.mod_type == fs_mod.mod_type)
        }) {
            matched_db[db_idx] = true;
            let db_mod = &consolidated_db[db_idx];
            let mut merged = db_mod.clone();
            merged.game_path = fs_mod.game_path.clone();
            merged.disabled_path = fs_mod.disabled_path.clone();
            merged.has_enabled_txt = fs_mod.has_enabled_txt;
            if merged.config_path.is_none() {
                merged.config_path = fs_mod.config_path.clone();
            }
            if fs_mod.extra_files.len() > merged.extra_files.len() {
                merged.extra_files = fs_mod.extra_files.clone();
            }
            merged.enabled = fs_mod.enabled;
            if fs_mod.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod")) {
                merged.nexus_summary = fs_mod.nexus_summary.clone();
                merged.has_pending_update = fs_mod.has_pending_update;
            }
            result.push(merged);
        } else {
            result.push(fs_mod.clone());
        }
    }

    for (i, dm) in consolidated_db.iter().enumerate() {
        if !matched_db[i] {
            let is_installed_in_current = installed_ids.iter().any(|entry| {
                crate::profiles::mod_matches_profile_entry(dm, entry)
            });

            let normalized_disabled = dm.disabled_path.replace("\\", "/");
            let is_disabled_in_other_profile = normalized_disabled.contains("/profiles/") 
                && !normalized_disabled.contains(&format!("/profiles/{}/", current_profile_id));

            if !is_installed_in_current || is_disabled_in_other_profile {
                let is_workshop = dm.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod"));
                if is_workshop {
                    if workshop_package_names.contains(&dm.id.to_lowercase()) {
                        let mut cleaned = dm.clone();
                        cleaned.game_path = String::new();
                        cleaned.enabled = false;
                        result.push(cleaned);
                    }
                } else {
                    result.push(dm.clone());
                }
                continue;
            }

            let has_game = !dm.game_path.is_empty() && Path::new(&dm.game_path).exists();
            let has_disabled = !dm.disabled_path.is_empty() && Path::new(&dm.disabled_path).exists();
            let has_metadata = dm.nexus_mod_id.is_some() || dm.library_zip.is_some();
 
            if has_game || has_disabled {
                result.push(dm.clone());
            } else if has_metadata {
                let mut kept = dm.clone();
                kept.enabled = false;
                kept.game_path = String::new();
                kept.disabled_path = String::new();
                kept.extra_files = Vec::new();
                result.push(kept);
            } else {
                crate::logger::log(&format!("merge_scan_with_db: Mod '{}' no longer exists on disk and has no metadata. Purging from database.", dm.name));
            }
        }
    }

    let mut final_deduped: Vec<ModInfo> = Vec::new();
    for m in result {
        if let Some(existing_idx) = final_deduped.iter().position(|em| em.id == m.id) {
            let existing = final_deduped[existing_idx].clone();
            let mut merged = existing.clone();
            if merged.game_path.is_empty() && !m.game_path.is_empty() {
                merged.game_path = m.game_path.clone();
            }
            if merged.disabled_path.is_empty() && !m.disabled_path.is_empty() {
                merged.disabled_path = m.disabled_path.clone();
            }
            for extra in &m.extra_files {
                if !merged.extra_files.contains(extra) {
                    merged.extra_files.push(extra.clone());
                }
            }
            merged.enabled = existing.enabled || m.enabled;
            if merged.config_path.is_none() && m.config_path.is_some() {
                merged.config_path = m.config_path.clone();
            }
            final_deduped[existing_idx] = merged;
        } else {
            final_deduped.push(m);
        }
    }

    final_deduped
}
