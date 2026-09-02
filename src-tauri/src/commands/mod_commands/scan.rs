use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::models::{ModInfo, ModType, WorkshopInstallType};
use super::utils::{get_physical_identity, file_install_date, detect_config};

fn load_pmm_meta(path: &Path) -> Option<ModInfo> {
    let path_str = path.to_string_lossy().to_string();
    let is_disabled = path_str.contains("disabled_mods");
    let game_path = if is_disabled { String::new() } else { path_str.clone() };
    let disabled_path = if is_disabled { path_str.clone() } else { String::new() };

    if path.is_dir() {
        if let Some(meta) = crate::profiles::utils::consolidate_mod_folder_metadata(path) {
            let folder_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Mod".to_string());
            let name = if !meta.name.is_empty() { meta.name } else { folder_name.clone() };
            let version = if !meta.version.is_empty() { meta.version } else { "1.0.0".to_string() };

            let mod_type = if meta.nexus_mod_id == Some(1626) || name.to_lowercase().contains("altermatic") || folder_name.to_lowercase().contains("altermatic") {
                ModType::Altermatic
            } else if meta.nexus_mod_id == Some(1894) || name.to_lowercase().contains("unipalui") || folder_name.to_lowercase().contains("unipalui") {
                ModType::Hybrid
            } else {
                match meta.mod_type.as_deref().unwrap_or("ue4ss").to_lowercase().as_str() {
                    "ue4ss" => ModType::Ue4ss,
                    "palschema" => ModType::PalSchema,
                    "pak" => ModType::Pak,
                    "logicmods" => ModType::LogicMods,
                    "altermatic" => ModType::Altermatic,
                    "hybrid" => ModType::Hybrid,
                    _ => ModType::Hybrid,
                }
            };

            return Some(ModInfo {
                id: folder_name,
                name,
                mod_type,
                nexus_mod_id: meta.nexus_mod_id,
                nexus_url: meta.nexus_url.or_else(|| meta.nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id))),
                nexus_author: meta.author,
                nexus_summary: meta.description,
                nexus_picture_url: meta.nexus_picture_url,
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
                extra_files: meta.installed_files.unwrap_or_default(),
                nexus_description: None,
                nexus_version_cached: None,
                nexus_cached_at: None,
                nexus_category: meta.category,
                nexus_tags: Vec::new(),
                github_repo: None,
                github_version: None,
                github_cached_at: None,
                update_date: None,
                library_zip: None,
                ignored_version: None,
                nexus_file_id: meta.nexus_file_id,
                ignored_keys: None,
                has_pending_update: None,
                origin_load_method: None,
                custom_notes: meta.custom_notes,
            });
        }
    } else if path.is_file() {
        let pmm_path = PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()));
        let pmm_alt = path.parent().and_then(|p| path.file_stem().map(|s| p.join(format!("{}.pak.pmm.json", s.to_string_lossy()))));
        let target_pmm = if pmm_path.exists() { Some(pmm_path) } else if pmm_alt.as_ref().map_or(false, |p| p.exists()) { pmm_alt } else { None };

        if let Some(target) = target_pmm {
            if let Ok(content) = fs::read_to_string(&target) {
                if let Ok(meta) = serde_json::from_str::<crate::models::PmmMetadata>(&content) {
                    let file_stem = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Mod".to_string());
                    let name = if !meta.name.is_empty() { meta.name } else { file_stem.clone() };
                    let version = if !meta.version.is_empty() { meta.version } else { "1.0.0".to_string() };

                    let is_altermatic_skin = if let Some(parent) = path.parent() {
                        let swap_json_dir = parent.join("SwapJSON");
                        let alt_swap_json_dir = parent.parent().map(|p| p.join("~mods").join("SwapJSON"));
                        swap_json_dir.join(format!("{}.json", file_stem)).exists()
                            || swap_json_dir.join(format!("{}.json", file_stem.trim_end_matches("_P"))).exists()
                            || alt_swap_json_dir.as_ref().map_or(false, |d| d.join(format!("{}.json", file_stem)).exists() || d.join(format!("{}.json", file_stem.trim_end_matches("_P"))).exists())
                    } else {
                        false
                    };

                    let mod_type = if meta.nexus_mod_id == Some(1626) || name.to_lowercase().contains("altermatic") || file_stem.to_lowercase().contains("altermatic") || is_altermatic_skin {
                        ModType::Altermatic
                    } else if meta.nexus_mod_id == Some(1894) || name.to_lowercase().contains("unipalui") || file_stem.to_lowercase().contains("unipalui") {
                        ModType::Hybrid
                    } else {
                        match meta.mod_type.as_deref().unwrap_or("pak").to_lowercase().as_str() {
                            "logicmods" => ModType::LogicMods,
                            "palschema" => ModType::PalSchema,
                            "ue4ss" => ModType::Ue4ss,
                            "altermatic" => ModType::Altermatic,
                            "hybrid" => ModType::Hybrid,
                            _ => ModType::Pak,
                        }
                    };

                    return Some(ModInfo {
                        id: file_stem,
                        name,
                        mod_type,
                        nexus_mod_id: meta.nexus_mod_id,
                        nexus_url: meta.nexus_url.or_else(|| meta.nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id))),
                        nexus_author: meta.author,
                        nexus_summary: meta.description,
                        nexus_picture_url: meta.nexus_picture_url,
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
                        has_enabled_txt: false,
                        mods_txt_order: None,
                        extra_files: meta.installed_files.unwrap_or_default(),
                        nexus_description: None,
                        nexus_version_cached: None,
                        nexus_cached_at: None,
                        nexus_category: meta.category,
                        nexus_tags: Vec::new(),
                        github_repo: None,
                        github_version: None,
                        github_cached_at: None,
                        update_date: None,
                        library_zip: None,
                        ignored_version: None,
                        nexus_file_id: meta.nexus_file_id,
                        ignored_keys: None,
                        has_pending_update: None,
                        origin_load_method: None,
                        custom_notes: meta.custom_notes,
                    });
                }
            }
        }
    }
    None
}

fn scan_ue4ss_mods(dir: &Path, results: &mut Vec<ModInfo>, ignored_names: &std::collections::HashSet<String>) {
    if !dir.exists() { return; }

    let mods_txt_path = dir.join("mods.txt");
    let mut mods_txt_states: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut mods_txt_positions: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    if mods_txt_path.exists() {
        if let Ok(content) = fs::read_to_string(&mods_txt_path) {
            for (line_idx, line) in content.lines().enumerate() {
                let line_clean = line.trim();
                if line_clean.starts_with(';') || line_clean.starts_with("//") {
                    continue;
                }
                if let Some(pos) = line_clean.find(':') {
                    let name = line_clean[..pos].trim().to_lowercase();
                    let val = line_clean[pos+1..].trim();
                    mods_txt_states.insert(name.clone(), val == "1");
                    mods_txt_positions.insert(name, line_idx as u32);
                } else if !line_clean.is_empty() {
                    let name = line_clean.to_lowercase();
                    mods_txt_states.insert(name.clone(), true);
                    mods_txt_positions.insert(name, line_idx as u32);
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

            let name_lower = mod_name.to_lowercase();
            let is_in_mods_txt = mods_txt_states.contains_key(&name_lower);
            let is_enabled = if let Some(&state) = mods_txt_states.get(&name_lower) {
                state
            } else {
                mod_path.join("enabled.txt").exists() || is_native_mod
            };

            let order_pos = mods_txt_positions.get(&name_lower).copied();
            let origin_load = if is_in_mods_txt {
                Some("mods_txt".to_string())
            } else if mod_path.join("enabled.txt").exists() {
                Some("enabled_txt".to_string())
            } else {
                None
            };

            if let Some(mut m) = load_pmm_meta(&mod_path) {
                m.enabled = is_enabled;
                if m.mods_txt_order.is_none() && order_pos.is_some() {
                    m.mods_txt_order = order_pos;
                }
                if m.origin_load_method.is_none() && origin_load.is_some() {
                    m.origin_load_method = origin_load;
                }
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
                mods_txt_order: order_pos,
                extra_files: Vec::new(),
                nexus_description: None, nexus_version_cached: None, nexus_cached_at: None,
                nexus_category: None, nexus_tags: Vec::new(),
                github_repo: None, github_version: None, github_cached_at: None,
                update_date: None, library_zip: None,
                ignored_version: None,
                nexus_file_id: None,
                ignored_keys: None,
                has_pending_update: None,
                origin_load_method: origin_load,
                custom_notes: None,
            });
        }
    }
}

fn scan_palschema_mods(dir: &Path, results: &mut Vec<ModInfo>, ignored_names: &std::collections::HashSet<String>) {
    if !dir.exists() { return; }
    let storage_dir = dir.parent().map(|p| p.join("Storage"));

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) { continue; }
            let raw_name = entry.file_name().to_string_lossy().to_string();
            let clean_name = if raw_name.len() > 4 && raw_name[..3].chars().all(|c| c.is_ascii_digit()) && raw_name.as_bytes()[3] == b'_' {
                raw_name[4..].to_string()
            } else {
                raw_name.clone()
            };

            if ignored_names.contains(&clean_name.to_lowercase()) || ignored_names.contains(&raw_name.to_lowercase()) {
                continue;
            }
            let mod_path = entry.path();

            if let Some(mut m) = load_pmm_meta(&mod_path) {
                if m.name.len() > 4 && m.name[..3].chars().all(|c| c.is_ascii_digit()) && m.name.as_bytes()[3] == b'_' {
                    m.name = m.name[4..].to_string();
                }
                results.push(m);
                continue;
            }

            // Check if storage folder has .pmm.json
            if let Some(ref s_dir) = storage_dir {
                let storage_mod_dir = s_dir.join(&clean_name);
                if storage_mod_dir.exists() {
                    if let Some(mut m) = load_pmm_meta(&storage_mod_dir) {
                        m.game_path = mod_path.to_string_lossy().to_string();
                        m.enabled = true;
                        if m.name.len() > 4 && m.name[..3].chars().all(|c| c.is_ascii_digit()) && m.name.as_bytes()[3] == b'_' {
                            m.name = m.name[4..].to_string();
                        }
                        results.push(m);
                        continue;
                    }
                }
            }

            let has_json = WalkDir::new(&mod_path).max_depth(2).into_iter().filter_map(|e| e.ok()).any(|e| {
                e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "json" || ext == "jsonc")
            });

            if has_json {
                let install_date = file_install_date(&mod_path);
                results.push(ModInfo {
                    id: clean_name.clone(),
                    name: clean_name.clone(),
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
                    has_pending_update: None,
                    origin_load_method: None,
                    custom_notes: None,
                });
            }
        }
    }
}

fn scan_pak_mods(
    dir: &Path,
    pak_type: &str,
    results: &mut Vec<ModInfo>,
    registered_patches: &[crate::pak_patcher::RegisteredPatch],
) {
    if !dir.exists() { return; }
    for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        let fname = entry.path().file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if fname.ends_with(".pmm.json.pmm.json") || fname.ends_with(".json.pmm.json") {
            let _ = fs::remove_file(entry.path());
            continue;
        }
        let ext = entry.path().extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        if ext != "pak" { continue; }

        let mod_path = entry.path();
        if let Some(m) = load_pmm_meta(&mod_path) {
            results.push(m);
            continue;
        }

        let file_stem = entry.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "unknown".to_string());
        let filename = entry.path().file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let path_str = entry.path().to_string_lossy().to_string();

        // Ignore internal compatibility patches (e.g. zzz_PMM_Patch_*, zzz_MergedMods_*, or in registry) from main mods list
        if file_stem.starts_with("zzz_") || registered_patches.iter().any(|p| p.pak_filename == filename || p.pak_path == path_str) {
            continue;
        }

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
        let is_altermatic = mod_name.to_lowercase().contains("altermatic")
            || file_stem.to_lowercase().contains("altermatic")
            || dir.join("SwapJSON").join(format!("{}.json", file_stem)).exists()
            || dir.join("SwapJSON").join(format!("{}.json", mod_name)).exists()
            || dir.parent().map_or(false, |p| p.join("~mods").join("SwapJSON").join(format!("{}.json", file_stem)).exists() || p.join("~mods").join("SwapJSON").join(format!("{}.json", mod_name)).exists());

        let is_unipalui = mod_name.to_lowercase().contains("unipalui") || file_stem.to_lowercase().contains("unipalui");

        let mt = if is_altermatic {
            ModType::Altermatic
        } else if is_unipalui {
            ModType::Hybrid
        } else if pak_type == "logicmods" {
            ModType::LogicMods
        } else {
            ModType::Pak
        };
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
            has_pending_update: None,
            origin_load_method: None,
            custom_notes: None,
        });
    }
}

fn scan_disabled_mods(disabled_base: &Path, results: &mut Vec<ModInfo>) {
    let type_dirs = [
        ("ue4ss", ModType::Ue4ss),
        ("palschema", ModType::PalSchema),
        ("hybrid", ModType::Hybrid),
    ];
    for (type_str, mod_type) in &type_dirs {
        let dir = disabled_base.join(type_str);
        if !dir.exists() { continue; }
        if let Ok(rd) = fs::read_dir(&dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if !entry.file_type().map_or(false, |ft| ft.is_dir()) { continue; }
                let mod_name = entry.file_name().to_string_lossy().to_string();
                if type_str == &"hybrid" && ["logicmods", "palschema", "pak", "ue4ss", "extras"].contains(&mod_name.to_lowercase().as_str()) {
                    continue;
                }
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
                    has_pending_update: None,
                    origin_load_method: None,
                    custom_notes: None,
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
                    has_pending_update: None,
                    origin_load_method: None,
                    custom_notes: None,
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
    let _guard = crate::watcher::pause_watcher_guard();
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

    let registered_patches = crate::pak_patcher::load_profile_patches_registry(program_path, current_profile_id);

    let pak_mods_dir = gp.paks_dir.clone();
    if pak_mods_dir.exists() {
        scan_pak_mods(&pak_mods_dir, "pak", &mut fs_mods, &registered_patches);
    }

    let logic_mods_dir = gp.logic_mods_dir.clone();
    if logic_mods_dir.exists() {
        scan_pak_mods(&logic_mods_dir, "logicmods", &mut fs_mods, &registered_patches);
    }

    let disabled_base = PathBuf::from(program_path)
        .join("profiles")
        .join(current_profile_id)
        .join("disabled_mods");
    if disabled_base.exists() {
        scan_disabled_mods(&disabled_base, &mut fs_mods);
    }

    for wmod in wmods.iter().filter(|m| !m.is_framework && (m.is_installed || m.is_active)) {
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
            origin_load_method: None,
            custom_notes: None,
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
        let mut matched_idx = None;
        for (i, dm) in consolidated_db.iter().enumerate() {
            if matched_db[i] {
                continue;
            }
            let db_id = get_physical_identity(&dm.game_path, &dm.disabled_path);
            let same_physical_path = !fs_path_norm.is_empty() && (
                (!dm.game_path.is_empty() && dm.game_path.replace('\\', "/").to_lowercase() == fs_path_norm) ||
                (!dm.disabled_path.is_empty() && dm.disabled_path.replace('\\', "/").to_lowercase() == fs_path_norm)
            );
            let same_physical_id = !db_id.is_empty() && !fs_id.is_empty() && db_id.to_lowercase() == fs_id.to_lowercase();
            let same_id = dm.id.to_lowercase() == fs_mod.id.to_lowercase();
            let same_name = !dm.name.is_empty() && dm.name.to_lowercase() == fs_mod.name.to_lowercase();

            if same_physical_path || same_physical_id || same_id || same_name {
                matched_idx = Some(i);
                break;
            }
        }

        if let Some(db_idx) = matched_idx {
            matched_db[db_idx] = true;
            let db_mod = &consolidated_db[db_idx];
            let mut merged = db_mod.clone();
            if fs_mod.mod_type == ModType::Altermatic || db_mod.mod_type == ModType::Altermatic {
                merged.mod_type = ModType::Altermatic;
            } else if fs_mod.mod_type == ModType::Hybrid || db_mod.mod_type == ModType::Hybrid {
                merged.mod_type = ModType::Hybrid;
            }
            merged.game_path = fs_mod.game_path.clone();
            merged.disabled_path = fs_mod.disabled_path.clone();
            merged.has_enabled_txt = fs_mod.has_enabled_txt;
            if merged.config_path.is_none() {
                merged.config_path = fs_mod.config_path.clone();
            }

            // Filter extra_files to only keep those that physically exist or belong to the active tree
            let mut valid_extras = Vec::new();
            for extra in &db_mod.extra_files {
                let extra_p = Path::new(extra);
                if extra_p.exists() {
                    valid_extras.push(extra.clone());
                }
            }
            for extra in &fs_mod.extra_files {
                if !valid_extras.contains(extra) {
                    valid_extras.push(extra.clone());
                }
            }
            merged.extra_files = valid_extras;

            merged.enabled = fs_mod.enabled;
            if fs_mod.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod")) {
                merged.nexus_summary = fs_mod.nexus_summary.clone();
                merged.has_pending_update = fs_mod.has_pending_update;
                merged.version = fs_mod.version.clone();
                merged.nexus_version_cached = fs_mod.nexus_version_cached.clone();
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

            let normalized_disabled = dm.disabled_path.replace('\\', "/");
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
        let m_id = m.id.to_lowercase();
        let m_phys = get_physical_identity(&m.game_path, &m.disabled_path).to_lowercase();
        let m_game = m.game_path.replace('\\', "/").to_lowercase();
        let m_disabled = m.disabled_path.replace('\\', "/").to_lowercase();

        if let Some(existing_idx) = final_deduped.iter().position(|em| {
            let em_id = em.id.to_lowercase();
            let em_phys = get_physical_identity(&em.game_path, &em.disabled_path).to_lowercase();
            let em_game = em.game_path.replace('\\', "/").to_lowercase();
            let em_disabled = em.disabled_path.replace('\\', "/").to_lowercase();

            (em_id == m_id) ||
            (!m_phys.is_empty() && em_phys == m_phys) ||
            (!m_game.is_empty() && em_game == m_game) ||
            (!m_disabled.is_empty() && em_disabled == m_disabled) ||
            (!em.name.is_empty() && em.name.to_lowercase() == m.name.to_lowercase())
        }) {
            let existing = final_deduped[existing_idx].clone();
            let mut merged = existing.clone();
            if m.mod_type == ModType::Altermatic || existing.mod_type == ModType::Altermatic {
                merged.mod_type = ModType::Altermatic;
            } else if m.mod_type == ModType::Hybrid || existing.mod_type == ModType::Hybrid {
                merged.mod_type = ModType::Hybrid;
            }
            if merged.game_path.is_empty() && !m.game_path.is_empty() {
                merged.game_path = m.game_path.clone();
            }
            if merged.disabled_path.is_empty() && !m.disabled_path.is_empty() {
                merged.disabled_path = m.disabled_path.clone();
            }
            if merged.nexus_mod_id.is_none() && m.nexus_mod_id.is_some() {
                merged.nexus_mod_id = m.nexus_mod_id;
                merged.nexus_url = m.nexus_url.clone();
                merged.nexus_author = m.nexus_author.clone();
                merged.nexus_summary = m.nexus_summary.clone();
                merged.nexus_picture_url = m.nexus_picture_url.clone();
            }
            for extra in &m.extra_files {
                if !merged.extra_files.contains(extra) {
                    merged.extra_files.push(extra.clone());
                }
            }
            let is_strictly_disabled = (!merged.disabled_path.is_empty() && merged.game_path.is_empty())
                || (!m.disabled_path.is_empty() && m.game_path.is_empty())
                || (!existing.disabled_path.is_empty() && existing.game_path.is_empty());
            merged.enabled = if is_strictly_disabled {
                false
            } else {
                existing.enabled || m.enabled
            };
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
