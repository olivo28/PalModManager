use crate::models::{ModInfo, ModType};
use crate::zip_handler;
use crate::zip_handler::{DetectedModType, ZipAnalysis};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

/// Recursively collect all files with the given extension (case-insensitive).
fn find_files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                result.extend(find_files_with_ext(&path, ext));
            } else if path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase() == ext)
                .unwrap_or(false)
            {
                result.push(path);
            }
        }
    }
    result
}

/// Strip Nexus-style suffix from a zip filename to get a clean mod name.
///
/// Input:  "Fishing Pond HR (Palschema) 2631 3 2026-07-11T11-06Z vcf5 (1).zip"
/// Output: "Fishing Pond HR (Palschema)"
///
/// Rule: stop at the first token that is:
///   - purely numeric  (Nexus mod ID)
///   - starts with a digit followed by a hyphen that looks like a date
fn clean_zip_name(zip_filename: &str) -> String {
    let stem = Path::new(zip_filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let words: Vec<&str> = stem.split_whitespace().collect();
    let mut clean: Vec<&str> = Vec::new();
    for word in &words {
        // Pure number == Nexus mod ID
        if word.chars().all(|c| c.is_ascii_digit()) {
            break;
        }
        // Date-like token (e.g. "2026-07-11T11-06Z")
        let starts_with_digit = word.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
        if starts_with_digit && word.contains('-') && word.len() > 6 {
            break;
        }
        clean.push(word);
    }

    let result = clean.join(" ").trim_end_matches(|c: char| c == '(' || c == ' ').trim().to_string();
    if result.len() < 2 { stem } else { result }
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

/// Locate the UE4SS mod root inside the extracted directory.
fn find_ue4ss_mod_root(extracted: &Path, zip_filename: &str) -> (PathBuf, String) {
    let base = navigate_to_mod_root(extracted);
    
    // Find all .lua files recursively
    let lua_files = find_files_with_ext(&base, "lua");

    // 1. Prioritize finding a file named exactly "main.lua"
    let main_lua = lua_files.iter().find(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_lowercase() == "main.lua")
            .unwrap_or(false)
    });

    // Use main.lua path if found, otherwise fallback to the first lua file
    let target_lua = main_lua.or_else(|| lua_files.first());

    if let Some(lua) = target_lua {
        if let Some(parent) = lua.parent() {
            let parent_lower = parent
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            let mod_root = if parent_lower == "scripts" {
                match parent.parent() {
                    Some(p) => p.to_path_buf(),
                    None => continue_fallback(extracted, zip_filename),
                }
            } else {
                parent.to_path_buf()
            };

            if !mod_root.starts_with(&base) {
                return (base.to_path_buf(), clean_zip_name(zip_filename));
            }

            if mod_root == base {
                return (base.to_path_buf(), clean_zip_name(zip_filename));
            }

            let name = mod_root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| clean_zip_name(zip_filename));

            return (mod_root, name);
        }
    }

    (base.to_path_buf(), clean_zip_name(zip_filename))
}

// Helper to keep compiler happy in fallback option
fn continue_fallback(extracted: &Path, _zip_filename: &str) -> PathBuf {
    extracted.to_path_buf()
}

/// Locate the PalSchema mod root inside the extracted directory.
fn find_palschema_mod_root(extracted: &Path, zip_filename: &str) -> (PathBuf, String) {
    let base = navigate_to_mod_root(extracted);

    if let Ok(mut entries) = fs::read_dir(&base) {
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
            let mod_root = subdirs[0].clone();
            let name = mod_root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| clean_zip_name(zip_filename));
            
            let name_lower = name.to_lowercase();
            if !GAME_DIR_SEGMENTS.contains(&name_lower.as_str()) {
                return (mod_root, name);
            }
        }
    }

    (base.to_path_buf(), clean_zip_name(zip_filename))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn install_mod(
    game_path: &str,
    extracted_dir: &Path,
    analysis: &ZipAnalysis,
    zip_filename: &str,
    nexus_mod_id: Option<u32>,
    nexus_name: Option<String>,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    _pak_destination: Option<&str>,
    custom_name: Option<String>,
    custom_type: Option<String>,
    nexus_category: Option<String>,
    nexus_tags: Vec<String>,
) -> Result<ModInfo, String> {
    let game = PathBuf::from(game_path);
    let now = Utc::now().to_rfc3339();

    // Determine the type to install: check custom override, fallback to analyzed type
    let target_type = if let Some(ref t) = custom_type {
        let t_lower = t.to_lowercase();
        if t_lower == "ue4ss" {
            DetectedModType::Ue4ss
        } else if t_lower == "palschema" {
            DetectedModType::PalSchema
        } else if t_lower == "pak" {
            DetectedModType::Pak
        } else if t_lower == "logicmods" {
            DetectedModType::LogicMods
        } else if t_lower == "hybrid" {
            DetectedModType::Hybrid
        } else {
            analysis.detected_type.clone()
        }
    } else {
        analysis.detected_type.clone()
    };

    match target_type {
        DetectedModType::Ue4ss => install_ue4ss(
            &game, extracted_dir, zip_filename, nexus_mod_id, nexus_name, nexus_author,
            nexus_summary, nexus_picture_url, nexus_downloads, nexus_endorsements,
            custom_name, &now, nexus_category, nexus_tags,
        ),
        DetectedModType::PalSchema => install_palschema(
            &game, extracted_dir, zip_filename, nexus_mod_id, nexus_name, nexus_author,
            nexus_summary, nexus_picture_url, nexus_downloads, nexus_endorsements,
            custom_name, &now, nexus_category, nexus_tags,
        ),
        DetectedModType::Pak => install_pak(
            &game, extracted_dir, zip_filename, nexus_mod_id, nexus_name, nexus_author,
            nexus_summary, nexus_picture_url, nexus_downloads, nexus_endorsements,
            Some("~mods"), custom_name, &now, nexus_category, nexus_tags,
        ),
        DetectedModType::LogicMods => install_pak(
            &game, extracted_dir, zip_filename, nexus_mod_id, nexus_name, nexus_author,
            nexus_summary, nexus_picture_url, nexus_downloads, nexus_endorsements,
            Some("logicmods"), custom_name, &now, nexus_category, nexus_tags,
        ),
        DetectedModType::Hybrid => install_hybrid(
            &game, extracted_dir, zip_filename, nexus_mod_id, nexus_name, nexus_author,
            nexus_summary, nexus_picture_url, nexus_downloads, nexus_endorsements,
            custom_name, &now, nexus_category, nexus_tags,
            analysis,
        ),
        DetectedModType::Unknown => {
            Err("Cannot determine mod type. Please specify manually.".to_string())
        }
    }
}

/// Check if a mod with the given folder name already exists.
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

pub fn determine_mod_id(nexus_mod_id: Option<u32>, folder_name: &str) -> String {
    if let Some(nexus_id) = nexus_mod_id {
        format!("{}-{}", nexus_id, folder_name)
    } else {
        folder_name.to_string()
    }
}

pub fn check_mod_exists(folder_name: &str, nexus_id: Option<u32>, existing_mods: &[ModInfo]) -> Option<ModInfo> {
    let zip_norm = folder_name.replace(' ', "").replace('-', "").replace('_', "").to_lowercase();
    
    existing_mods
        .iter()
        .find(|m| {
            let db_id = get_physical_identity(&m.game_path, &m.disabled_path);
            if !db_id.is_empty() && db_id == zip_norm {
                return true;
            }
            if normalize_name(&m.name) == normalize_name(folder_name) {
                return true;
            }
            if let (Some(nid1), Some(nid2)) = (nexus_id, m.nexus_mod_id) {
                if nid1 == nid2 {
                    let name_sim = db_id.contains(&zip_norm) || zip_norm.contains(&db_id);
                    if name_sim {
                        return true;
                    }
                }
            }
            false
        })
        .cloned()
}



/// Re-install an existing mod from new ZIP contents.
pub fn update_mod(
    existing: &mut ModInfo,
    _game_path: &str,
    extracted: &Path,
    analysis: &ZipAnalysis,
    zip_filename: &str,
    now: &str,
) -> Result<(), String> {
    let dest = if existing.enabled {
        PathBuf::from(&existing.game_path)
    } else {
        PathBuf::from(&existing.disabled_path)
    };

    if dest.is_dir() {
        let _ = fs::remove_dir_all(&dest);
    } else if dest.is_file() {
        let _ = fs::remove_file(&dest);
    }

    // Re-detect mod root and copy
    match &analysis.detected_type {
        DetectedModType::Ue4ss => {
            let mod_root = find_ue4ss_mod_root(extracted, zip_filename).0;
            let has_scripts_dir = mod_root.join("Scripts").exists() || mod_root.join("scripts").exists();
            if has_scripts_dir {
                copy_folder_contents(&mod_root, &dest)?;
            } else {
                fs::create_dir_all(dest.join("Scripts")).map_err(|e| format!("Cannot create Scripts dir: {}", e))?;
                if let Ok(entries) = fs::read_dir(&mod_root) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        let file_name = path.file_name().unwrap();
                        if path.is_dir() {
                            let dest_path = dest.join(file_name);
                            copy_folder_contents(&path, &dest_path)?;
                        } else {
                            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                            if ext == "lua" {
                                fs::copy(&path, dest.join("Scripts").join(file_name))
                                    .map_err(|e| format!("Cannot copy lua script: {}", e))?;
                            } else {
                                fs::copy(&path, dest.join(file_name))
                                    .map_err(|e| format!("Cannot copy asset: {}", e))?;
                            }
                        }
                    }
                }
            }
            // Ensure enabled.txt exists
            let enabled_file = dest.join("enabled.txt");
            if !enabled_file.exists() {
                let _ = fs::write(&enabled_file, "");
            }
        }

        DetectedModType::PalSchema => {
            let mod_root = find_palschema_mod_root(extracted, zip_filename).0;
            copy_folder_contents(&mod_root, &dest)?;
        }
        _ => {
            copy_folder_contents(extracted, &dest)?;
        }
    }

    existing.update_date = Some(now.to_string());
    existing.source_zip = zip_filename.to_string();

    if existing.enabled {
        existing.game_path = dest.to_string_lossy().to_string();
    } else {
        existing.disabled_path = dest.to_string_lossy().to_string();
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal install helpers
// ---------------------------------------------------------------------------

fn sanitize_folder_name(name: &str) -> String {
    let mut cleaned = name.replace(|c: char| {
        c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|'
    }, "");
    cleaned = cleaned.trim().to_string();
    cleaned
}

fn detect_config_local(dir: &Path) -> Option<String> {
    for entry in WalkDir::new(dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name == "config.json" || name == "config.jsonc" || name == "config.txt" || name == "config.cfg" || name == "settings.json" || name == "settings.txt" {
                return Some(entry.path().to_string_lossy().to_string());
            }
        }
    }
    None
}

pub fn install_hybrid(
    game: &Path,
    extracted: &Path,
    zip_filename: &str,
    nexus_mod_id: Option<u32>,
    _nexus_name: Option<String>,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    custom_name: Option<String>,
    now: &str,
    nexus_category: Option<String>,
    nexus_tags: Vec<String>,
    _analysis: &ZipAnalysis,
) -> Result<ModInfo, String> {
    let win64 = crate::dependency_checker::get_binaries_dir(game);
    let clean_stem = clean_zip_name(zip_filename);
    let mod_name = custom_name.unwrap_or(clean_stem.clone());
    let safe_folder_name = sanitize_folder_name(&clean_stem);

    let mut installed_extras = Vec::new();
    let mut primary_path = String::new();
    let mut config_path: Option<String> = None;

    let palschema_base_dest = win64.join("ue4ss").join("Mods").join("PalSchema").join("mods").join(&safe_folder_name);
    let ue4ss_base_dest = win64.join("ue4ss").join("Mods").join(&safe_folder_name);
    let paks_dest_dir = game.join("Pal").join("Content").join("Paks").join("~mods");
    let logicmods_dest_dir = game.join("Pal").join("Content").join("Paks").join("LogicMods");

    let mut copied_palschema = false;
    let mut copied_ue4ss = false;

    for entry in WalkDir::new(extracted).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let rel_path = path.strip_prefix(extracted).unwrap_or(path);
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");
        let rel_lower = rel_str.to_lowercase();
        let file_basename = path.file_name().unwrap();

        // 1. Is it a Pak file/companion?
        if rel_lower.ends_with(".pak") || rel_lower.ends_with(".ucas") || rel_lower.ends_with(".utoc") {
            let dest_dir = if rel_lower.contains("logicmods") {
                &logicmods_dest_dir
            } else {
                &paks_dest_dir
            };
            let dest_file = dest_dir.join(file_basename);
            let _ = fs::create_dir_all(dest_file.parent().unwrap());
            fs::copy(path, &dest_file).map_err(|e| format!("Cannot copy asset file: {}", e))?;
            installed_extras.push(dest_file.to_string_lossy().to_string());
            continue;
        }

        // Skip READMEs or other junk files at the root of the ZIP
        if rel_lower.starts_with("readme") || (rel_lower.ends_with(".txt") && !rel_str.contains('/')) {
            continue;
        }

        // 2. Is it a PalSchema file?
        if rel_lower.contains("palschema") {
            let parts: Vec<&str> = rel_str.split('/').collect();
            let mut relative_parts = Vec::new();
            for (idx, part) in parts.iter().enumerate() {
                if part.to_lowercase().contains("palschema") {
                    if idx + 2 < parts.len() {
                        relative_parts = parts[idx + 2..].to_vec();
                    }
                    break;
                }
            }

            let dest_file = if !relative_parts.is_empty() {
                palschema_base_dest.join(relative_parts.join("/"))
            } else {
                palschema_base_dest.join(file_basename)
            };

            let _ = fs::create_dir_all(dest_file.parent().unwrap());
            fs::copy(path, &dest_file).map_err(|e| format!("Cannot copy PalSchema file: {}", e))?;
            copied_palschema = true;
            continue;
        }

        // 3. Otherwise, it belongs to UE4SS component
        let parts: Vec<&str> = rel_str.split('/').collect();
        let mut relative_parts = Vec::new();
        let mut found_mods = false;
        for (idx, part) in parts.iter().enumerate() {
            if part.to_lowercase() == "mods" {
                found_mods = true;
                if idx + 2 < parts.len() {
                    relative_parts = parts[idx + 2..].to_vec();
                }
                break;
            }
        }

        if !found_mods {
            for (idx, part) in parts.iter().enumerate() {
                if part.to_lowercase() == "scripts" {
                    relative_parts = parts[idx..].to_vec();
                    break;
                }
            }
        }

        let dest_file = if !relative_parts.is_empty() {
            ue4ss_base_dest.join(relative_parts.join("/"))
        } else {
            ue4ss_base_dest.join(file_basename)
        };

        let _ = fs::create_dir_all(dest_file.parent().unwrap());
        fs::copy(path, &dest_file).map_err(|e| format!("Cannot copy UE4SS file: {}", e))?;
        copied_ue4ss = true;
    }

    if copied_ue4ss {
        primary_path = ue4ss_base_dest.to_string_lossy().to_string();
        config_path = detect_config_local(&ue4ss_base_dest);
        let enabled_file = ue4ss_base_dest.join("enabled.txt");
        if !enabled_file.exists() {
            let _ = fs::write(&enabled_file, "");
        }
        if copied_palschema {
            installed_extras.push(palschema_base_dest.to_string_lossy().to_string());
        }
    } else if copied_palschema {
        primary_path = palschema_base_dest.to_string_lossy().to_string();
        config_path = detect_config_local(&palschema_base_dest);
    } else {
        if !installed_extras.is_empty() {
            primary_path = installed_extras.remove(0);
        }
    }

    let has_enabled_txt = copied_ue4ss;

    Ok(ModInfo {
        id: determine_mod_id(nexus_mod_id, &safe_folder_name),
        name: mod_name,
        mod_type: ModType::Hybrid,
        nexus_mod_id,
        nexus_url: nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id)),
        nexus_author,
        nexus_summary,
        nexus_picture_url,
        nexus_endorsements,
        nexus_downloads,
        version: parse_version_from_filename(zip_filename),
        install_date: now.to_string(),
        source_zip: zip_filename.to_string(),
        config_path,
        config_type: Some("auto".to_string()),
        enabled: true,
        game_path: primary_path,
        disabled_path: String::new(),
        pak_destination: None,
        has_enabled_txt,
        mods_txt_order: None,
        extra_files: installed_extras,
        nexus_description: None,
        nexus_version_cached: None,
        nexus_cached_at: None,
        nexus_category,
        nexus_tags,
        github_repo: None,
        github_version: None,
        github_cached_at: None,
        update_date: None,
        library_zip: None,
    })
}

pub fn install_ue4ss(
    game: &Path,
    extracted: &Path,
    zip_filename: &str,
    nexus_mod_id: Option<u32>,
    _nexus_name: Option<String>,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    custom_name: Option<String>,
    now: &str,
    nexus_category: Option<String>,
    nexus_tags: Vec<String>,
) -> Result<ModInfo, String> {
    let mods_dir = crate::dependency_checker::get_binaries_dir(game)
        .join("ue4ss")
        .join("Mods");

    let (mod_root, detected_name) = find_ue4ss_mod_root(extracted, zip_filename);
    let mod_name = custom_name.unwrap_or_else(|| detected_name.clone());
    let safe_folder_name = sanitize_folder_name(&detected_name);
    let dest = mods_dir.join(&safe_folder_name);

    if dest.exists() {
        return Err(format!("Mod folder '{}' already exists at destination. Remove it first.", safe_folder_name));
    }

    crate::logger::log(&format!(
        "install_ue4ss: mod_root='{}' -> dest='{}'",
        mod_root.display(),
        dest.display()
    ));

    let has_scripts_dir = mod_root.join("Scripts").exists() || mod_root.join("scripts").exists();


    if has_scripts_dir {
        copy_folder_contents(&mod_root, &dest)?;
    } else {
        fs::create_dir_all(dest.join("Scripts")).map_err(|e| format!("Cannot create Scripts dir: {}", e))?;
        if let Ok(entries) = fs::read_dir(&mod_root) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let file_name = path.file_name().unwrap();
                if path.is_dir() {
                    let dest_path = dest.join(file_name);
                    copy_folder_contents(&path, &dest_path)?;
                } else {
                    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    if ext == "lua" {
                        fs::copy(&path, dest.join("Scripts").join(file_name))
                            .map_err(|e| format!("Cannot copy lua script: {}", e))?;
                    } else {
                        fs::copy(&path, dest.join(file_name))
                            .map_err(|e| format!("Cannot copy asset: {}", e))?;
                    }
                }
            }
        }
    }

    let enabled_file = dest.join("enabled.txt");
    if !enabled_file.exists() {
        let _ = fs::write(&enabled_file, "");
    }
    let has_enabled_txt = true;

    Ok(ModInfo {
        id: determine_mod_id(nexus_mod_id, &safe_folder_name),
        name: mod_name,

        mod_type: ModType::Ue4ss,
        nexus_mod_id,
        nexus_url: nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id)),
        nexus_author,
        nexus_summary,
        nexus_picture_url,
        nexus_endorsements,
        nexus_downloads,
        version: parse_version_from_filename(zip_filename),
        install_date: now.to_string(),
        source_zip: zip_filename.to_string(),
        config_path: None,
        config_type: Some("auto".to_string()),
        enabled: true,
        game_path: dest.to_string_lossy().to_string(),
        disabled_path: String::new(),
        pak_destination: None,
        has_enabled_txt,
        mods_txt_order: None,
        extra_files: Vec::new(),
        nexus_description: None,
        nexus_version_cached: None,
        nexus_cached_at: None,
        nexus_category,
        nexus_tags,
        github_repo: None,
        github_version: None,
        github_cached_at: None,
        update_date: None,
        library_zip: None,
    })
}


fn install_palschema(
    game: &Path,
    extracted: &Path,
    zip_filename: &str,
    nexus_mod_id: Option<u32>,
    _nexus_name: Option<String>,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    custom_name: Option<String>,
    now: &str,
    nexus_category: Option<String>,
    nexus_tags: Vec<String>,
) -> Result<ModInfo, String> {
    let mods_dir = crate::dependency_checker::get_binaries_dir(game)
        .join("ue4ss")
        .join("Mods")
        .join("PalSchema")
        .join("mods");

    let (mod_root, detected_name) = find_palschema_mod_root(extracted, zip_filename);
    let mod_name = custom_name.unwrap_or_else(|| detected_name.clone());
    let safe_folder_name = sanitize_folder_name(&detected_name);
    let dest = mods_dir.join(&safe_folder_name);

    if dest.exists() {
        return Err(format!("Mod folder '{}' already exists at destination. Remove it first.", safe_folder_name));
    }

    crate::logger::log(&format!(
        "install_palschema: mod_root='{}' -> dest='{}'",
        mod_root.display(),
        dest.display()
    ));

    copy_folder_contents(&mod_root, &dest)?;


    Ok(ModInfo {
        id: determine_mod_id(nexus_mod_id, &safe_folder_name),
        name: mod_name,
        mod_type: ModType::PalSchema,
        nexus_mod_id,
        nexus_url: nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id)),
        nexus_author,
        nexus_summary,
        nexus_picture_url,
        nexus_endorsements,
        nexus_downloads,
        version: parse_version_from_filename(zip_filename),
        install_date: now.to_string(),
        source_zip: zip_filename.to_string(),
        config_path: None,
        config_type: Some("auto".to_string()),
        enabled: true,
        game_path: dest.to_string_lossy().to_string(),
        disabled_path: String::new(),
        pak_destination: None,
        has_enabled_txt: false,
        mods_txt_order: None,
        extra_files: Vec::new(),
        nexus_description: None,
        nexus_version_cached: None,
        nexus_cached_at: None,
        nexus_category,
        nexus_tags,
        github_repo: None,
        github_version: None,
        github_cached_at: None,
        update_date: None,
        library_zip: None,
    })
}

fn install_pak(
    game: &Path,
    extracted: &Path,
    zip_filename: &str,
    nexus_mod_id: Option<u32>,
    _nexus_name: Option<String>,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    pak_destination: Option<&str>,
    custom_name: Option<String>,
    now: &str,
    nexus_category: Option<String>,
    nexus_tags: Vec<String>,
) -> Result<ModInfo, String> {
    let dest_subdir = match pak_destination {
        Some("logicmods") => "LogicMods",
        _ => "~mods",
    };

    let paks_dir = game
        .join("Pal")
        .join("Content")
        .join("Paks")
        .join(dest_subdir);

    fs::create_dir_all(&paks_dir).map_err(|e| format!("Cannot create paks dir: {}", e))?;

    let pak_files = zip_handler::find_pak_files_recursive(extracted);

    if pak_files.is_empty() {
        return Err("No .pak files found in the zip archive.".to_string());
    }

    let mut installed_paks: Vec<String> = Vec::new();

    for pak_path in &pak_files {
        let companions = zip_handler::find_pak_companions(pak_path);
        let target_stem = pak_path.file_stem().unwrap().to_string_lossy().to_string();

        for companion in companions {
            let ext = companion.extension().unwrap().to_string_lossy();
            let dest_file = paks_dir.join(format!("{}.{}", target_stem, ext));
            
            fs::copy(&companion, &dest_file)
                .map_err(|e| format!("Cannot copy pak: {}", e))?;
                
            if ext == "pak" {
                installed_paks.push(dest_file.to_string_lossy().to_string());
            }
        }

    }

    if installed_paks.is_empty() {
        return Err("No .pak files installed.".to_string());
    }

    let first_pak = Path::new(&installed_paks[0]);
    let clean_stem = first_pak
        .file_stem()
        .map(|s| {
            let s = s.to_string_lossy();
            s.strip_suffix("_P").unwrap_or(&s).to_string()
        })
        .unwrap_or_else(|| clean_zip_name(zip_filename));

    let mod_name = custom_name.unwrap_or(clean_stem.clone());


    let mod_type = if dest_subdir == "LogicMods" {
        ModType::LogicMods
    } else {
        ModType::Pak
    };

    Ok(ModInfo {
        id: determine_mod_id(nexus_mod_id, &clean_stem),
        name: mod_name,
        mod_type,
        nexus_mod_id,
        nexus_url: nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id)),
        nexus_author,
        nexus_summary,
        nexus_picture_url,
        nexus_endorsements,
        nexus_downloads,
        version: parse_version_from_filename(zip_filename),
        install_date: now.to_string(),
        source_zip: zip_filename.to_string(),
        config_path: None,
        config_type: None,
        enabled: true,
        game_path: installed_paks.first().cloned().unwrap_or_default(),
        disabled_path: String::new(),
        pak_destination: Some(dest_subdir.to_string()),
        has_enabled_txt: false,
        mods_txt_order: None,
        extra_files: installed_paks.into_iter().skip(1).collect(),
        nexus_description: None,
        nexus_version_cached: None,
        nexus_cached_at: None,
        nexus_category,
        nexus_tags,
        github_repo: None,
        github_version: None,
        github_cached_at: None,
        update_date: None,
        library_zip: None,
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
