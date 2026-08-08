use crate::models::{ModInfo, ModType};
use crate::zip_handler;
use crate::zip_handler::{DetectedModType, ZipAnalysis};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
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
pub fn clean_zip_name(zip_filename: &str) -> String {
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
        let word_lower = word.to_lowercase();
        if ["gamepass", "steam", "gdk", "xbox"].contains(&word_lower.as_str()) {
            continue;
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

fn find_ue4ss_root_for_file(file_path: &Path, extracted: &Path) -> PathBuf {
    let mut current = file_path.to_path_buf();
    while let Some(parent) = current.parent() {
        if parent == extracted {
            break;
        }
        let name = parent.file_name().unwrap().to_string_lossy().to_lowercase();
        if name == "scripts" || name == "dlls" {
            if let Some(grandparent) = parent.parent() {
                return grandparent.to_path_buf();
            }
        }
        current = parent.to_path_buf();
    }
    file_path.parent().unwrap_or(extracted).to_path_buf()
}

/// Locate the UE4SS mod root inside the extracted directory.
fn find_ue4ss_mod_root(extracted: &Path, zip_filename: &str) -> (PathBuf, String) {
    let base = navigate_to_mod_root(extracted);
    
    // Find all .lua and .dll files recursively
    let mut files = find_files_with_ext(&base, "lua");
    files.extend(find_files_with_ext(&base, "dll"));

    // 1. Prioritize finding a file named exactly "main.lua" or "main.dll"
    let main_file = files.iter().find(|p| {
        p.file_name()
            .map(|n| {
                let name = n.to_string_lossy().to_lowercase();
                name == "main.lua" || name == "main.dll"
            })
            .unwrap_or(false)
    });

    // Use main file path if found, otherwise fallback to the first file
    let target_file = main_file.or_else(|| files.first());

    if let Some(f) = target_file {
        let mod_root = find_ue4ss_root_for_file(f, extracted);

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

    (base.to_path_buf(), clean_zip_name(zip_filename))
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
fn preprocess_extracted_dir(extracted: &Path, is_xbox: bool) -> Result<(), String> {
    // 1. Collect all file paths
    let mut all_files = Vec::new();
    fn collect_files(dir: &Path, list: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, list);
                } else {
                    list.push(path);
                }
            }
        }
    }
    collect_files(extracted, &mut all_files);

    // Check if the archive contains both platform-specific markers
    let has_steam_tags = all_files.iter().any(|p| {
        let rel = p.strip_prefix(extracted).unwrap().to_string_lossy().to_lowercase().replace('\\', "/");
        rel.contains("/(steam)/") || rel.starts_with("(steam)/") ||
        rel.contains("/steam/") || rel.starts_with("steam/") ||
        rel.contains("/win64/") || rel.starts_with("win64/")
    });

    let has_xbox_tags = all_files.iter().any(|p| {
        let rel = p.strip_prefix(extracted).unwrap().to_string_lossy().to_lowercase().replace('\\', "/");
        rel.contains("/(xbox)/") || rel.starts_with("(xbox)/") ||
        rel.contains("/xbox/") || rel.starts_with("xbox/") ||
        rel.contains("/(gdk)/") || rel.starts_with("(gdk)/") ||
        rel.contains("/gdk/") || rel.starts_with("gdk/") ||
        rel.contains("/wingdk/") || rel.starts_with("wingdk/")
    });

    let has_both_platforms = has_steam_tags && has_xbox_tags;

    // 2. Identify and delete files belonging to the inactive platform (only if both are present)
    if has_both_platforms {
        for file_path in &all_files {
            if !file_path.exists() { continue; }
            let rel_path = file_path.strip_prefix(extracted).unwrap();
            let rel_lower = rel_path.to_string_lossy().to_lowercase().replace('\\', "/");
            let segments: Vec<&str> = rel_lower.split('/').collect();

            let is_inactive = if is_xbox {
                segments.iter().any(|&s| {
                    s == "(steam)" || s == "steam" || s == "win64"
                })
            } else {
                segments.iter().any(|&s| {
                    s == "(xbox)" || s == "xbox" || s == "(gdk)" || s == "gdk" || s == "wingdk"
                })
            };

            if is_inactive {
                let _ = fs::remove_file(file_path);
            }
        }
    }

    // 3. Normalize wrapper folders for the active platform.
    // If a file resides inside a folder like (STEAM) or (XBOX) at the top level,
    // we want to move it to the root of the extracted folder.
    let mut remaining_files = Vec::new();
    collect_files(extracted, &mut remaining_files);

    for file_path in remaining_files {
        if !file_path.exists() { continue; }
        let rel_path = file_path.strip_prefix(extracted).unwrap();
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");
        let parts: Vec<&str> = rel_str.split('/').collect();
        if parts.len() > 1 {
            let first_lower = parts[0].to_lowercase();
            // If both are present, only strip our own platform wrapper.
            // If only one is present, we strip whatever wrapper is present (as a fallback).
            let is_wrapper = if has_both_platforms {
                if is_xbox {
                    first_lower == "(xbox)" || first_lower == "xbox" || first_lower == "(gdk)" || first_lower == "gdk" || first_lower == "wingdk"
                } else {
                    first_lower == "(steam)" || first_lower == "steam" || first_lower == "win64"
                }
            } else {
                first_lower == "(steam)" || first_lower == "steam" || first_lower == "win64" ||
                first_lower == "(xbox)" || first_lower == "xbox" || first_lower == "(gdk)" || first_lower == "gdk" || first_lower == "wingdk"
            };

            if is_wrapper {
                // Determine target path at the root of extracted
                let target_rel = parts[1..].join("/");
                let target_path = extracted.join(target_rel);
                if let Some(parent) = target_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Err(_) = fs::rename(&file_path, &target_path) {
                    if fs::copy(&file_path, &target_path).is_ok() {
                        let _ = fs::remove_file(&file_path);
                    }
                }
            }
        }
    }

    // 4. Clean up any empty directories inside extracted
    fn clean_empty_dirs(dir: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            let mut subdirs = Vec::new();
            let mut has_files = false;
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    subdirs.push(entry.path());
                } else {
                    has_files = true;
                }
            }
            for subdir in subdirs {
                clean_empty_dirs(&subdir);
            }
            if !has_files {
                if let Ok(mut check) = fs::read_dir(dir) {
                    if check.next().is_none() {
                        let _ = fs::remove_dir(dir);
                    }
                }
            }
        }
    }
    clean_empty_dirs(extracted);

    Ok(())
}

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
    pak_destination: Option<&str>,
    custom_name: Option<String>,
    custom_type: Option<String>,
    nexus_category: Option<String>,
    nexus_tags: Vec<String>,
) -> Result<ModInfo, String> {
    let game = PathBuf::from(game_path);
    let now = Utc::now().to_rfc3339();

    let win64 = crate::dependency_checker::get_binaries_dir(&game);
    let is_xbox = win64.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("wingdk".to_string());

    if let Err(e) = preprocess_extracted_dir(extracted_dir, is_xbox) {
        crate::logger::log(&format!("Warning: preprocess_extracted_dir failed: {}", e));
    }

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
            pak_destination, custom_name, &now, nexus_category, nexus_tags,
        ),
        DetectedModType::LogicMods => install_pak(
            &game, extracted_dir, zip_filename, nexus_mod_id, nexus_name, nexus_author,
            nexus_summary, nexus_picture_url, nexus_downloads, nexus_endorsements,
            pak_destination, custom_name, &now, nexus_category, nexus_tags,
        ),
        DetectedModType::Hybrid => install_hybrid(
            &game, extracted_dir, zip_filename, nexus_mod_id, nexus_name, nexus_author,
            nexus_summary, nexus_picture_url, nexus_downloads, nexus_endorsements,
            custom_name, &now, nexus_category, nexus_tags,
            pak_destination,
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
                    if m.mod_type == ModType::Pak || m.mod_type == ModType::LogicMods {
                        let name_sim = db_id.contains(&zip_norm) || zip_norm.contains(&db_id);
                        if name_sim {
                            return true;
                        }
                    } else {
                        // Folder-based mods: must have the exact same folder name
                        if db_id == zip_norm {
                            return true;
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

/// Re-install an existing mod from new ZIP contents.
pub fn update_mod(
    existing: &mut ModInfo,
    game_path: &str,
    program_path: &str,
    current_profile_id: &str,
    extracted: &Path,
    analysis: &ZipAnalysis,
    zip_filename: &str,
    now: &str,
) -> Result<(), String> {
    // Helper to delete path and its companion sidecar if it's a file
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

    // 0. Deduce/preserve existing pak destination before paths are cleared/deleted
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

    // 1. Physically delete previous mod files
    delete_path_and_sidecar(&existing.game_path);
    delete_path_and_sidecar(&existing.disabled_path);
    for extra in &existing.extra_files {
        delete_path_and_sidecar(extra);
    }

    // 2. Remember original state
    let was_enabled = existing.enabled;

    // 3. Call the corresponding installer function based on the mod's target type
    let game = Path::new(game_path);
    let new_mod_info = match existing.mod_type {
        ModType::Ue4ss => install_ue4ss(
            game,
            extracted,
            zip_filename,
            existing.nexus_mod_id,
            Some(existing.name.clone()),
            existing.nexus_author.clone(),
            existing.nexus_summary.clone(),
            existing.nexus_picture_url.clone(),
            existing.nexus_downloads,
            existing.nexus_endorsements,
            Some(existing.name.clone()),
            now,
            existing.nexus_category.clone(),
            existing.nexus_tags.clone(),
        )?,
        ModType::PalSchema => install_palschema(
            game,
            extracted,
            zip_filename,
            existing.nexus_mod_id,
            Some(existing.name.clone()),
            existing.nexus_author.clone(),
            existing.nexus_summary.clone(),
            existing.nexus_picture_url.clone(),
            existing.nexus_downloads,
            existing.nexus_endorsements,
            Some(existing.name.clone()),
            now,
            existing.nexus_category.clone(),
            existing.nexus_tags.clone(),
        )?,
        ModType::Pak => install_pak(
            game,
            extracted,
            zip_filename,
            existing.nexus_mod_id,
            Some(existing.name.clone()),
            existing.nexus_author.clone(),
            existing.nexus_summary.clone(),
            existing.nexus_picture_url.clone(),
            existing.nexus_downloads,
            existing.nexus_endorsements,
            dest_deduced.as_deref(),
            Some(existing.name.clone()),
            now,
            existing.nexus_category.clone(),
            existing.nexus_tags.clone(),
        )?,
        ModType::LogicMods => install_pak(
            game,
            extracted,
            zip_filename,
            existing.nexus_mod_id,
            Some(existing.name.clone()),
            existing.nexus_author.clone(),
            existing.nexus_summary.clone(),
            existing.nexus_picture_url.clone(),
            existing.nexus_downloads,
            existing.nexus_endorsements,
            dest_deduced.as_deref(),
            Some(existing.name.clone()),
            now,
            existing.nexus_category.clone(),
            existing.nexus_tags.clone(),
        )?,
        ModType::Hybrid => install_hybrid(
            game,
            extracted,
            zip_filename,
            existing.nexus_mod_id,
            Some(existing.name.clone()),
            existing.nexus_author.clone(),
            existing.nexus_summary.clone(),
            existing.nexus_picture_url.clone(),
            existing.nexus_downloads,
            existing.nexus_endorsements,
            Some(existing.name.clone()),
            now,
            existing.nexus_category.clone(),
            existing.nexus_tags.clone(),
            dest_deduced.as_deref(),
            analysis,
        )?,
    };

    // 4. Overwrite mod fields with new install details
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
    existing.enabled = true; // Temporary marked as enabled so we can disable if needed

    // 5. If it was disabled before, manually disable it now
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

fn find_palschema_root(json_path: &Path, extracted: &Path) -> PathBuf {
    for ancestor in json_path.ancestors() {
        if let Some(name) = ancestor.file_name().map(|n| n.to_string_lossy().to_lowercase()) {
            if ["pals", "enums", "translations", "raw"].contains(&name.as_str()) {
                if let Some(parent) = ancestor.parent() {
                    if parent != extracted {
                        return parent.to_path_buf();
                    }
                }
            }
        }
    }
    let mut current = json_path.parent().unwrap();
    while let Some(parent) = current.parent() {
        if parent == extracted {
            break;
        }
        let parent_name = parent.file_name().unwrap().to_string_lossy().to_lowercase();
        if ["palschema mods folder", "mods", "ue4ss"].contains(&parent_name.as_str()) {
            break;
        }
        current = parent;
    }
    current.to_path_buf()
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
    pak_destination: Option<&str>,
    _analysis: &ZipAnalysis,
) -> Result<ModInfo, String> {
    let win64 = crate::dependency_checker::get_binaries_dir(game);
    let clean_stem = clean_zip_name(zip_filename);

    // 1. Scan extracted files to dynamically discover component roots
    let mut all_files = Vec::new();
    fn collect_files(dir: &Path, list: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, list);
                } else {
                    list.push(path);
                }
            }
        }
    }
    collect_files(extracted, &mut all_files);

    // 2. Identify UE4SS roots (LUA files or DLL files)
    let mut ue4ss_roots = Vec::new();
    for file_path in &all_files {
        let ext = file_path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext == "lua" || ext == "dll" {
            let root = find_ue4ss_root_for_file(file_path, extracted);
            if !ue4ss_roots.contains(&root) {
                ue4ss_roots.push(root);
            }
        }
    }

    // 3. Identify PalSchema roots
    let mut palschema_roots = Vec::new();
    for file_path in &all_files {
        let ext = file_path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext == "json" || ext == "jsonc" {
            let rel = file_path.strip_prefix(extracted).unwrap().to_string_lossy().to_lowercase().replace('\\', "/");
            let is_palschema_json = rel.contains("palschema") || 
                rel.contains("/unique/") || rel.starts_with("unique/") ||
                rel.contains("/translations/") || rel.starts_with("translations/") ||
                rel.contains("/spawns/") || rel.starts_with("spawns/") ||
                rel.contains("/skins/") || rel.starts_with("skins/") ||
                rel.contains("/raw/") || rel.starts_with("raw/") ||
                rel.contains("/pals/") || rel.starts_with("pals/") ||
                rel.contains("/items/") || rel.starts_with("items/") ||
                rel.contains("/enums/") || rel.starts_with("enums/") ||
                rel.contains("/blueprints/") || rel.starts_with("blueprints/");
            if is_palschema_json {
                let root = find_palschema_root(file_path, extracted);
                if !palschema_roots.contains(&root) {
                    palschema_roots.push(root);
                }
            }
        }
    }

    // Determine target mod folder name
    let detected_folder_name = if ue4ss_roots.len() == 1 && palschema_roots.is_empty() {
        let name = ue4ss_roots[0].file_name().unwrap().to_string_lossy().to_string();
        if name.is_empty() || name.to_lowercase() == "temp_extracted" || name.starts_with("palmodmanager_") || ue4ss_roots[0] == extracted {
            clean_stem.clone()
        } else {
            name
        }
    } else if palschema_roots.len() == 1 && ue4ss_roots.is_empty() {
        let name = palschema_roots[0].file_name().unwrap().to_string_lossy().to_string();
        if name.is_empty() || name.to_lowercase() == "temp_extracted" || name.starts_with("palmodmanager_") || palschema_roots[0] == extracted {
            clean_stem.clone()
        } else {
            name
        }
    } else {
        clean_stem.clone()
    };

    let mod_name = custom_name.unwrap_or_else(|| detected_folder_name.clone());
    let safe_folder_name = sanitize_folder_name(&mod_name);

    let mut installed_extras = Vec::new();
    let mut primary_path = String::new();
    let mut config_path: Option<String> = None;

    let paks_dest_dir = game.join("Pal").join("Content").join("Paks").join("~mods");
    let logicmods_dest_dir = game.join("Pal").join("Content").join("Paks").join("LogicMods");
    let ue4ss_mods_dest = win64.join("ue4ss").join("Mods");
    let palschema_mods_dest = ue4ss_mods_dest.join("PalSchema").join("mods");

    let mut copied_palschema = false;
    let mut copied_ue4ss = false;
    let mut copied_ue4ss_base = PathBuf::new();
    let mut copied_palschema_base = PathBuf::new();

    // 4. Perform enrouting copy
    for file_path in &all_files {
        let rel_path = file_path.strip_prefix(extracted).unwrap();
        let rel_lower = rel_path.to_string_lossy().to_lowercase().replace('\\', "/");
        let file_basename = file_path.file_name().unwrap();

        // A. Pak files
        if rel_lower.ends_with(".pak") || rel_lower.ends_with(".ucas") || rel_lower.ends_with(".utoc") {
            let dest_dir = if let Some(dest) = pak_destination {
                if dest.to_lowercase() == "logicmods" {
                    &logicmods_dest_dir
                } else {
                    &paks_dest_dir
                }
            } else {
                if rel_lower.contains("logicmods") {
                    &logicmods_dest_dir
                } else {
                    &paks_dest_dir
                }
            };
            let dest_file = dest_dir.join(file_basename);
            let _ = fs::create_dir_all(dest_file.parent().unwrap());
            fs::copy(file_path, &dest_file).map_err(|e| format!("Cannot copy asset file: {}", e))?;
            installed_extras.push(dest_file.to_string_lossy().to_string());
            continue;
        }

        // Longest prefix match routing for UE4SS and PalSchema roots
        let matching_ue4ss = ue4ss_roots.iter().filter(|r| file_path.starts_with(r)).max_by_key(|r| r.as_path().components().count());
        let matching_palschema = palschema_roots.iter().filter(|r| file_path.starts_with(r)).max_by_key(|r| r.as_path().components().count());

        match (matching_ue4ss, matching_palschema) {
            (Some(u_root), Some(p_root)) => {
                let u_count = u_root.as_path().components().count();
                let p_count = p_root.as_path().components().count();
                if p_count > u_count {
                    let file_rel_to_root = file_path.strip_prefix(p_root).unwrap();
                    let dest_file = palschema_mods_dest.join(&safe_folder_name).join(file_rel_to_root);
                    let _ = fs::create_dir_all(dest_file.parent().unwrap());
                    fs::copy(file_path, &dest_file).map_err(|e| format!("Cannot copy PalSchema file: {}", e))?;
                    copied_palschema_base = palschema_mods_dest.join(&safe_folder_name);
                    copied_palschema = true;
                } else {
                    let file_rel_to_root = file_path.strip_prefix(u_root).unwrap();
                    let dest_file = ue4ss_mods_dest.join(&safe_folder_name).join(file_rel_to_root);
                    let _ = fs::create_dir_all(dest_file.parent().unwrap());
                    fs::copy(file_path, &dest_file).map_err(|e| format!("Cannot copy UE4SS file: {}", e))?;
                    copied_ue4ss_base = ue4ss_mods_dest.join(&safe_folder_name);
                    copied_ue4ss = true;
                }
            }
            (None, Some(p_root)) => {
                let file_rel_to_root = file_path.strip_prefix(p_root).unwrap();
                let dest_file = palschema_mods_dest.join(&safe_folder_name).join(file_rel_to_root);
                let _ = fs::create_dir_all(dest_file.parent().unwrap());
                fs::copy(file_path, &dest_file).map_err(|e| format!("Cannot copy PalSchema file: {}", e))?;
                copied_palschema_base = palschema_mods_dest.join(&safe_folder_name);
                copied_palschema = true;
            }
            (Some(u_root), None) => {
                let file_rel_to_root = file_path.strip_prefix(u_root).unwrap();
                let dest_file = ue4ss_mods_dest.join(&safe_folder_name).join(file_rel_to_root);
                let _ = fs::create_dir_all(dest_file.parent().unwrap());
                fs::copy(file_path, &dest_file).map_err(|e| format!("Cannot copy UE4SS file: {}", e))?;
                copied_ue4ss_base = ue4ss_mods_dest.join(&safe_folder_name);
                copied_ue4ss = true;
            }
            (None, None) => {}
        }
    }

    if copied_ue4ss {
        primary_path = copied_ue4ss_base.to_string_lossy().to_string();
        config_path = detect_config_local(&copied_ue4ss_base);
        let enabled_file = copied_ue4ss_base.join("enabled.txt");
        if !enabled_file.exists() {
            let _ = fs::write(&enabled_file, "");
        }
        if copied_palschema {
            installed_extras.push(copied_palschema_base.to_string_lossy().to_string());
        }
    } else if copied_palschema {
        primary_path = copied_palschema_base.to_string_lossy().to_string();
        config_path = detect_config_local(&copied_palschema_base);
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
        pak_destination: pak_destination.map(|d| d.to_string()),
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
        ignored_version: None,
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
    let safe_folder_name = sanitize_folder_name(&mod_name);
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
        ignored_version: None,
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
    let safe_folder_name = sanitize_folder_name(&mod_name);
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
        ignored_version: None,
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
        Some(d) if d.to_lowercase() == "logicmods" => "LogicMods",
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
        ignored_version: None,
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
