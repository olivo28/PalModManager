use crate::models::ModInfo;
use crate::zip_handler::ZipAnalysis;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// Strip Nexus-style suffix from a zip filename to get a clean mod name.
///
/// Input:  "Fishing Pond HR (Palschema) 2631 3 2026-07-11T11-06Z vcf5 (1).zip"
/// Output: "Fishing Pond HR (Palschema)"
///
/// Rule: stop at the first token that is:
///   - purely numeric  (Nexus mod ID)
///   - starts with a digit followed by a hyphen that looks like a date
pub fn clean_zip_name(zip_filename: &str) -> String {
    let stem = Path::new(zip_filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let words: Vec<&str> = stem.split_whitespace().collect();
    
    // Find index of the last purely numeric token (excluding years)
    let mut id_index = None;
    for i in (0..words.len()).rev() {
        let clean_word: String = words[i].chars().filter(|c| c.is_ascii_digit()).collect();
        if !clean_word.is_empty() && words[i].chars().all(|c| c.is_ascii_digit() || c == '(' || c == ')') {
            if let Ok(num) = clean_word.parse::<u32>() {
                if !(num >= 2020 && num <= 2038) {
                    id_index = Some(i);
                    break;
                }
            }
        }
    }

    let clean_words = if let Some(idx) = id_index {
        words[..idx].to_vec()
    } else {
        // Fallback: slice before date-like token
        let mut idx = words.len();
        for i in 0..words.len() {
            let w = words[i];
            let starts_with_digit = w.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
            if starts_with_digit && w.contains('-') && w.len() > 6 {
                idx = i;
                break;
            }
        }
        words[..idx].to_vec()
    };

    let mut clean = Vec::new();
    for word in clean_words {
        let word_clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
        if ["gamepass", "steam", "gdk", "xbox", "singleplayer", "sp"].contains(&word_clean.as_str()) {
            continue;
        }
        clean.push(word);
    }

    let result = clean.join(" ");
    let final_result = result.trim_end_matches(|c: char| c == '-' || c == '_' || c == '(' || c == ' ' || c == ')').trim().to_string();
    if final_result.len() < 2 { stem } else { final_result }
}

fn parse_version_from_filename(zip_filename: &str) -> String {
    crate::nexus::parse_mod_filename(zip_filename)
        .version
        .unwrap_or_else(|| "unknown".to_string())
}


// ---------------------------------------------------------------------------
// Mod root detection  (works on the EXTRACTED filesystem, not zip file list)
// ---------------------------------------------------------------------------

const GAME_DIR_SEGMENTS: &[&str] = &[
    "pal", "binaries", "win64", "wingdk", "ue4ss", "mods", "palschema", "content", "paks"
];

/// Navigate down through game directories (like Pal/Binaries/Win64...) if present.
fn navigate_to_mod_root(dir: &Path) -> PathBuf {
    let mut current = dir.to_path_buf();
    loop {
        if let Ok(mut entries) = fs::read_dir(&current) {
            let mut subdirs = Vec::new();
            let mut has_files = false;
            while let Some(Ok(entry)) = entries.next() {
                let path = entry.path();
                if path.is_dir() {
                    subdirs.push(path);
                } else {
                    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    if !name.starts_with('.') && !name.contains("nexus") && name != "info.json" {
                        has_files = true;
                    }
                }
            }
            if subdirs.len() == 1 && !has_files {
                let dir_name = subdirs[0].file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                if GAME_DIR_SEGMENTS.contains(&dir_name.as_str()) {
                    current = subdirs[0].clone();
                    continue;
                }
            }
        }
        break;
    }
    current
}

pub fn install_mod(
    game_path: &str,
    extracted_dir: &Path,
    analysis: &ZipAnalysis,
    zip_filename: &str,
    _nexus_mod_id: Option<u32>,
    _nexus_name: Option<String>,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    pak_destination: Option<&str>,
    custom_name: Option<String>,
    _custom_type: Option<String>,
    nexus_category: Option<String>,
    nexus_tags: Vec<String>,
    force_load_order: bool,
) -> Result<ModInfo, String> {
    let game = Path::new(game_path);
    let now = Utc::now().to_rfc3339();

    // 1. Build manifest
    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            let full_path = extracted_dir.join(target_file);
            if full_path.exists() {
                if let Ok(content) = std::fs::read_to_string(full_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        modinfo_data = Some(val);
                    }
                }
            }
        }
    }

    let manifest = crate::zip_handler::build_manifest_from_files(
        &analysis.files,
        zip_filename,
        game,
        pak_destination,
        custom_name,
        modinfo_data.clone(),
    )?;

    let mut final_author = nexus_author;
    let mut final_summary = nexus_summary;

    if let Some(ref modinfo) = modinfo_data {
        if final_author.is_none() {
            if let Some(auth) = modinfo.get("author").and_then(|a| a.as_str()) {
                final_author = Some(auth.to_string());
            }
        }
        if final_summary.is_none() {
            if let Some(desc) = modinfo.get("description").and_then(|d| d.as_str()) {
                final_summary = Some(desc.to_string());
            }
        }
    }

    // 2. Execute manifest
    let mut mod_info = execute_manifest(
        &manifest,
        extracted_dir,
        game,
        final_author,
        final_summary,
        nexus_picture_url,
        nexus_downloads,
        nexus_endorsements,
        &now,
        force_load_order,
    )?;

    // Set other info
    mod_info.nexus_category = nexus_category;
    mod_info.nexus_tags = nexus_tags;
    mod_info.source_zip = zip_filename.to_string();

    Ok(mod_info)
}

fn normalize_path_separator(p: &str) -> String {
    let mut s = p.replace('/', "\\");
    while s.contains("\\\\") {
        s = s.replace("\\\\", "\\");
    }
    if s.ends_with('\\') && s.len() > 3 {
        s.pop();
    }
    s
}

fn get_ue4ss_component_root(dest_path: &str) -> Option<String> {
    let path = Path::new(dest_path);
    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("mods".to_string()) {
            if let Some(pparent) = parent.parent() {
                if pparent.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("ue4ss".to_string()) {
                    return Some(normalize_path_separator(&current.to_string_lossy()));
                }
            }
        }
        current = parent;
    }
    None
}

fn get_palschema_component_root(dest_path: &str) -> Option<String> {
    let path = Path::new(dest_path);
    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("mods".to_string()) {
            if let Some(pparent) = parent.parent() {
                if pparent.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("palschema".to_string()) {
                    return Some(normalize_path_separator(&current.to_string_lossy()));
                }
            }
        }
        current = parent;
    }
    None
}

pub fn execute_manifest(
    manifest: &crate::models::InstallManifest,
    extracted_dir: &Path,
    game_path: &Path,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    now: &str,
    force_load_order: bool,
) -> Result<ModInfo, String> {
    use crate::models::{ModInfo, RouteType, ModType};
    use std::fs;
    use std::path::PathBuf;

    let mut component_paths = Vec::new();
    let binaries_dir = crate::dependency_checker::get_binaries_dir(game_path);
    let ue4ss_component = normalize_path_separator(&binaries_dir.join("ue4ss").join("Mods").join(&manifest.folder_name).to_string_lossy());
    let palschema_component = normalize_path_separator(&binaries_dir.join("ue4ss").join("Mods").join("PalSchema").join("mods").join(&manifest.folder_name).to_string_lossy());

    let mut primary_path = String::new();
    let mut config_path = None;

    // 1. Copy/move files defined in the manifest routes
    for route in &manifest.routes {
        let src = extracted_dir.join(&route.zip_path);
        if !src.exists() {
            continue;
        }

        let dst = PathBuf::from(&route.dest_path);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create destination parent: {}", e))?;
        }

        if src.is_dir() {
            copy_folder_contents(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| format!("Failed to copy file from zip: {}", e))?;
        }

        let dst_str = dst.to_string_lossy().to_string();

        let comp_path = match route.route_type {
            RouteType::Ue4ss => get_ue4ss_component_root(&dst_str).unwrap_or_else(|| ue4ss_component.clone()),
            RouteType::PalSchema => get_palschema_component_root(&dst_str).unwrap_or_else(|| palschema_component.clone()),
            RouteType::Pak | RouteType::LogicMods | RouteType::Companion => normalize_path_separator(&dst_str),
            RouteType::Passthrough => {
                if manifest.has_ue4ss {
                    get_ue4ss_component_root(&dst_str).unwrap_or_else(|| ue4ss_component.clone())
                } else if manifest.has_palschema {
                    get_palschema_component_root(&dst_str).unwrap_or_else(|| palschema_component.clone())
                } else {
                    normalize_path_separator(&dst_str)
                }
            }
        };

        if !component_paths.contains(&comp_path) {
            component_paths.push(comp_path);
        }

        // Track config file
        match route.route_type {
            RouteType::Ue4ss => {
                if config_path.is_none() {
                    let u_root = PathBuf::from(&ue4ss_component);
                    config_path = detect_config_local(&u_root);
                }
            }
            RouteType::PalSchema => {
                if config_path.is_none() {
                    let p_root = PathBuf::from(&palschema_component);
                    config_path = detect_config_local(&p_root);
                }
            }
            _ => {}
        }
    }

    if manifest.has_ue4ss {
        primary_path = ue4ss_component.clone();
    } else if manifest.has_palschema {
        primary_path = palschema_component.clone();
    } else if !component_paths.is_empty() {
        primary_path = component_paths[0].clone();
    }

    // 2. Write enabled.txt and mods.txt for UE4SS mods
    if manifest.has_ue4ss {
        let binaries_dir = crate::dependency_checker::get_binaries_dir(game_path);
        let u_mod_dir = binaries_dir.join("ue4ss").join("Mods").join(&manifest.folder_name);
        if u_mod_dir.exists() {
            let enabled_file = u_mod_dir.join("enabled.txt");
            if force_load_order {
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
            } else {
                if !enabled_file.exists() {
                    let _ = fs::write(&enabled_file, "");
                }
            }
        }
        let mods_txt = binaries_dir.join("ue4ss").join("Mods").join("mods.txt");
        if mods_txt.exists() {
            if force_load_order {
                let _ = crate::profiles::update_mods_txt_load_order(&mods_txt, &manifest.folder_name, true);
            } else {
                let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &manifest.folder_name);
            }
        }
    }

    let has_enabled_txt = manifest.has_ue4ss;

    // Separate primary path from extra files
    let mut extra_files = Vec::new();
    for file in component_paths {
        if file != primary_path {
            extra_files.push(file);
        }
    }

    let pak_destination = if manifest.mod_type == ModType::LogicMods {
        Some("LogicMods".to_string())
    } else if manifest.mod_type == ModType::Pak {
        Some("~mods".to_string())
    } else {
        None
    };

    Ok(ModInfo {
        id: determine_mod_id(manifest.nexus_mod_id, manifest.nexus_file_id, &manifest.folder_name, &manifest.mod_type),
        name: manifest.display_name.clone(),
        mod_type: manifest.mod_type.clone(),
        nexus_mod_id: manifest.nexus_mod_id,
        nexus_url: manifest.nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id)),
        nexus_author,
        nexus_summary,
        nexus_picture_url,
        nexus_endorsements,
        nexus_downloads,
        version: manifest.version.clone(),
        install_date: now.to_string(),
        source_zip: String::new(),
        config_path,
        config_type: Some("auto".to_string()),
        enabled: true,
        game_path: primary_path,
        disabled_path: String::new(),
        pak_destination,
        has_enabled_txt,
        mods_txt_order: None,
        extra_files,
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
        nexus_file_id: manifest.nexus_file_id,
    })
}

// ---------------------------------------------------------------------------
// Filesystem helpers
// ---------------------------------------------------------------------------

fn copy_folder_contents(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("Cannot create dest dir: {}", e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("Cannot read source dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();
        let dest_path = dest.join(file_name);

        if path.is_dir() {
            copy_folder_contents(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)
                .map_err(|e| format!("Cannot copy file {}: {}", file_name.to_string_lossy(), e))?;
        }
    }
    Ok(())
}

fn detect_config_local(dir: &Path) -> Option<String> {
    for entry in walkdir::WalkDir::new(dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name == "config.json" || name == "config.jsonc" || name == "config.txt" || name == "config.cfg" || name == "settings.json" || name == "settings.txt" {
                return Some(entry.path().to_string_lossy().to_string());
            }
        }
    }
    None
}

pub fn determine_mod_id(
    nexus_mod_id: Option<u32>,
    nexus_file_id: Option<u32>,
    folder_name: &str,
    mod_type: &crate::models::ModType,
) -> String {
    let type_suffix = match mod_type {
        crate::models::ModType::Ue4ss => "ue4ss",
        crate::models::ModType::PalSchema => "palschema",
        crate::models::ModType::Pak => "pak",
        crate::models::ModType::LogicMods => "logicmods",
        crate::models::ModType::Hybrid => "hybrid",
    };
    if let Some(nexus_id) = nexus_mod_id {
        if let Some(file_id) = nexus_file_id {
            format!("{}-{}-{}-{}", nexus_id, file_id, folder_name, type_suffix)
        } else {
            format!("{}-{}-{}", nexus_id, folder_name, type_suffix)
        }
    } else {
        format!("{}-{}", folder_name, type_suffix)
    }
}

fn normalize_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn get_physical_identity(game_path: &str, disabled_path: &str) -> String {
    let path_str = if !game_path.is_empty() { game_path } else { disabled_path };
    if path_str.is_empty() {
        return String::new();
    }
    let path = std::path::Path::new(path_str);
    let mut name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    
    if name.ends_with(".disabled") {
        if let Some(stripped) = name.strip_suffix(".disabled") {
            name = stripped.to_string();
        }
    }
    if name.ends_with(".pak") {
        if let Some(stripped) = name.strip_suffix(".pak") {
            name = stripped.to_string();
        }
    }
    
    name.replace(' ', "").replace('-', "").replace('_', "").to_lowercase()
}

pub fn check_mod_exists(
    folder_name: &str,
    mod_type: &crate::models::ModType,
    nexus_id: Option<u32>,
    existing_mods: &[ModInfo],
) -> Option<ModInfo> {
    let zip_norm = folder_name.replace(' ', "").replace('-', "").replace('_', "").to_lowercase();
    
    existing_mods
        .iter()
        .find(|m| {
            if let (Some(nid1), Some(nid2)) = (nexus_id, m.nexus_mod_id) {
                if nid1 == nid2 {
                    return true;
                }
            }

            let db_id = get_physical_identity(&m.game_path, &m.disabled_path);
            if !db_id.is_empty() && db_id == zip_norm {
                return true;
            }

            if normalize_name(&m.name) == normalize_name(folder_name) {
                return true;
            }

            // Strict matching fallback (matching type and containing names for pak/logicmods)
            if m.mod_type == *mod_type {
                if let (Some(nid1), Some(nid2)) = (nexus_id, m.nexus_mod_id) {
                    if nid1 == nid2 {
                        if m.mod_type == crate::models::ModType::Pak || m.mod_type == crate::models::ModType::LogicMods {
                            let name_sim = db_id.contains(&zip_norm) || zip_norm.contains(&db_id);
                            if name_sim {
                                return true;
                            }
                        } else {
                            if db_id == zip_norm {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        })
        .cloned()
}

fn move_path(src: &Path, dst: &Path) -> Result<(), String> {
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    if src.is_dir() {
        copy_folder_contents(src, dst)?;
        fs::remove_dir_all(src).map_err(|e| format!("Failed to remove source dir after cross-device copy: {}", e))?;
    } else {
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::copy(src, dst).map_err(|e| format!("Failed to copy source file during cross-device move: {}", e))?;
        fs::remove_file(src).map_err(|e| format!("Failed to remove source file after cross-device copy: {}", e))?;
    }
    Ok(())
}

pub fn update_mod(
    existing: &mut ModInfo,
    game_path: &str,
    program_path: &str,
    current_profile_id: &str,
    extracted: &Path,
    analysis: &ZipAnalysis,
    zip_filename: &str,
    now: &str,
    force_load_order: bool,
) -> Result<(), String> {
    use crate::models::ModType;

    let delete_path_and_sidecar = |path_str: &str| {
        if path_str.is_empty() {
            return;
        }
        let p = Path::new(path_str);
        if p.exists() {
            if p.is_dir() {
                let _ = fs::remove_dir_all(p);
            } else {
                let _ = fs::remove_file(p);
                let sidecar = PathBuf::from(format!("{}.pmm.json", path_str));
                if sidecar.exists() {
                    let _ = fs::remove_file(sidecar);
                }
            }
        } else {
            let sidecar = PathBuf::from(format!("{}.pmm.json", path_str));
            if sidecar.exists() {
                let _ = fs::remove_file(sidecar);
            }
        }
    };

    let dest_deduced = if let Some(ref d) = existing.pak_destination {
        if !d.is_empty() {
            Some(d.clone())
        } else {
            None
        }
    } else {
        None
    }.or_else(|| {
        if existing.game_path.contains("LogicMods") || existing.disabled_path.contains("LogicMods") {
            Some("LogicMods".to_string())
        } else {
            None
        }
    });

    let snapshot = if !existing.game_path.is_empty() && Path::new(&existing.game_path).is_dir() {
        crate::config_merge::snapshot_configs(Path::new(&existing.game_path))
    } else if !existing.disabled_path.is_empty() && Path::new(&existing.disabled_path).is_dir() {
        crate::config_merge::snapshot_configs(Path::new(&existing.disabled_path))
    } else {
        crate::config_merge::ConfigSnapshot { entries: Vec::new() }
    };

    delete_path_and_sidecar(&existing.game_path);
    delete_path_and_sidecar(&existing.disabled_path);
    for extra in &existing.extra_files {
        delete_path_and_sidecar(extra);
    }

    let was_enabled = existing.enabled;

    let game = Path::new(game_path);
    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            let full_path = extracted.join(target_file);
            if full_path.exists() {
                if let Ok(content) = std::fs::read_to_string(full_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        modinfo_data = Some(val);
                    }
                }
            }
        }
    }

    let manifest = crate::zip_handler::build_manifest_from_files(
        &analysis.files,
        zip_filename,
        game,
        dest_deduced.as_deref(),
        Some(existing.name.clone()),
        modinfo_data,
    )?;

    let new_mod_info = execute_manifest(
        &manifest,
        extracted,
        game,
        existing.nexus_author.clone(),
        existing.nexus_summary.clone(),
        existing.nexus_picture_url.clone(),
        existing.nexus_downloads,
        existing.nexus_endorsements,
        now,
        force_load_order,
    )?;

    existing.name = new_mod_info.name;
    existing.mod_type = new_mod_info.mod_type;
    existing.version = new_mod_info.version;
    existing.source_zip = new_mod_info.source_zip;
    existing.config_path = new_mod_info.config_path;
    existing.config_type = new_mod_info.config_type;
    existing.game_path = new_mod_info.game_path;
    existing.disabled_path = new_mod_info.disabled_path;
    existing.pak_destination = new_mod_info.pak_destination;
    existing.has_enabled_txt = new_mod_info.has_enabled_txt;
    existing.extra_files = new_mod_info.extra_files;
    existing.update_date = Some(now.to_string());
    existing.enabled = true;

    let dest_dir = if !existing.game_path.is_empty() {
        Path::new(&existing.game_path)
    } else {
        Path::new(&existing.disabled_path)
    };
    if dest_dir.exists() && dest_dir.is_dir() {
        crate::config_merge::apply_config_merge(dest_dir, &snapshot);
    }

    if !was_enabled {
        let profile_dir = PathBuf::from(program_path).join("profiles").join(current_profile_id);
        let disabled_base = profile_dir.join("disabled_mods");

        if existing.mod_type == ModType::Ue4ss {
            let src_path = PathBuf::from(&existing.game_path);
            if src_path.exists() {
                let enabled_file = src_path.join("enabled.txt");
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
                let file_name = src_path.file_name().unwrap().to_string_lossy().to_string();
                let dest = disabled_base.join("ue4ss").join(&file_name);
                move_path(&src_path, &dest)?;
                existing.disabled_path = dest.to_string_lossy().to_string();
                existing.game_path = String::new();
            }
            existing.enabled = false;
        } else if existing.mod_type == ModType::PalSchema {
            let src_path = PathBuf::from(&existing.game_path);
            if src_path.exists() {
                let file_name = src_path.file_name().unwrap().to_string_lossy().to_string();
                let dest = disabled_base.join("palschema").join(&file_name);
                move_path(&src_path, &dest)?;
                existing.disabled_path = dest.to_string_lossy().to_string();
                existing.game_path = String::new();
            }
            existing.enabled = false;
        } else if existing.mod_type == ModType::Pak || existing.mod_type == ModType::LogicMods {
            let src_path = PathBuf::from(&existing.game_path);
            if src_path.exists() {
                let mut moved_files = Vec::new();
                if let Some(parent) = src_path.parent() {
                    let file_stem = src_path.file_stem().unwrap().to_string_lossy().to_string();
                    let type_dir = if existing.mod_type == ModType::LogicMods { "logicmods" } else { "pak" };
                    let dest_dir = disabled_base.join(type_dir);
                    let _ = fs::create_dir_all(&dest_dir);

                    for ext in &["pak", "ucas", "utoc", "pak.pmm.json"] {
                        let companion = parent.join(format!("{}.{}", file_stem, ext));
                        if companion.exists() {
                            let dest = dest_dir.join(format!("{}.{}", file_stem, ext));
                            move_path(&companion, &dest)?;
                            moved_files.push(dest.to_string_lossy().to_string());
                        }
                    }
                }
                existing.disabled_path = moved_files.first().cloned().unwrap_or_default();
                existing.extra_files = moved_files.into_iter().skip(1).collect();
                existing.game_path = String::new();
            }
            existing.enabled = false;
        } else if existing.mod_type == ModType::Hybrid {
            let mut moved_extras = Vec::new();
            for extra in &existing.extra_files {
                let extra_path = PathBuf::from(extra);
                if extra_path.exists() {
                    let file_name = extra_path.file_name().unwrap().to_string_lossy().to_string();
                    let dest_dir = disabled_base.join("hybrid").join("extras");
                    let _ = fs::create_dir_all(&dest_dir);
                    
                    if extra_path.is_dir() {
                        let dest = dest_dir.join(&file_name);
                        move_path(&extra_path, &dest)?;
                        moved_extras.push(dest.to_string_lossy().to_string());
                    } else {
                        let parent = extra_path.parent().unwrap();
                        let stem = extra_path.file_stem().unwrap().to_string_lossy().to_string();
                        let dest = dest_dir.join(&file_name);
                        move_path(&extra_path, &dest)?;
                        moved_extras.push(dest.to_string_lossy().to_string());
                        
                        for c_ext in &["ucas", "utoc"] {
                            let companion = parent.join(format!("{}.{}", stem, c_ext));
                            if companion.exists() {
                                let c_dest = dest_dir.join(format!("{}.{}", stem, c_ext));
                                let _ = move_path(&companion, &c_dest);
                            }
                        }
                        let sidecar = parent.join(format!("{}.pmm.json", file_name));
                        if sidecar.exists() {
                            let c_dest = dest_dir.join(format!("{}.pmm.json", file_name));
                            let _ = move_path(&sidecar, &c_dest);
                        }
                    }
                }
            }

            let src_path = PathBuf::from(&existing.game_path);
            if src_path.exists() {
                let enabled_file = src_path.join("enabled.txt");
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
                
                let file_name = src_path.file_name().unwrap().to_string_lossy().to_string();
                let dest_dir = disabled_base.join("hybrid");
                let _ = fs::create_dir_all(&dest_dir);
                let dest = dest_dir.join(&file_name);
                move_path(&src_path, &dest)?;
                
                existing.disabled_path = dest.to_string_lossy().to_string();
                existing.game_path = String::new();
            }
            existing.extra_files = moved_extras;
            existing.enabled = false;
        }
    }

    Ok(())
}
