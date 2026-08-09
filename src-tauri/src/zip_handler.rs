use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::read::ZipArchive;

pub fn extract_nexus_id_from_path(zip_path: &str) -> Option<u32> {
    let filename = Path::new(zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    crate::nexus::extract_nexus_id(&filename)
}

#[derive(Debug, Clone)]
pub enum DetectedModType {
    Ue4ss,
    PalSchema,
    Pak,
    LogicMods,
    Hybrid,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ZipAnalysis {
    pub detected_type: DetectedModType,
    pub has_lua: bool,
    pub has_json: bool,
    pub has_palschema_json: bool,
    pub has_pak: bool,
    #[allow(dead_code)]
    pub has_dll: bool,
    pub has_info_json: bool,
    pub pak_destination_hint: Option<String>,
    pub root_folder: Option<String>,
    pub files: Vec<String>,
}

/// Names that must not be used as mod folder names.
const FORBIDDEN_MOD_NAMES: &[&str] = &[
    "pal", "mods", "win64", "wingdk", "binaries", "content", "paks", "~mods",
    "logicmods", "ue4ss", "palschema", "plugins", "scripts",
    "blueprints", "translations",
];

/// Markers that identify the boundary before mod folders.
/// Using substring matching (lower.find()) so they work at ANY depth in the path.
/// Order matters: most specific first.
const MODS_DIR_MARKERS: &[&str] = &[
    "ue4ss/mods/palschema/mods/",
    "ue4ss/mods/palschema/",
    "palschema/mods/",
    "ue4ss/mods/",
    "ue4ss/mods/",
    "binaries/win64/ue4ss/mods/",
    "binaries/wingdk/ue4ss/mods/",
];

fn is_forbidden(name: &str) -> bool {
    let lower = name.to_lowercase();
    FORBIDDEN_MOD_NAMES.contains(&lower.as_str())
}

/// Core detection: find the mod root path within the ZIP using substring matching.
/// Works for direct paths, Nexus-wrapped paths, and double-wrapped paths.
/// Returns (mod_content_path, mod_folder_name).
fn detect_mod_content_and_name(
    files: &[String],
    detected_type: &DetectedModType,
) -> (Option<String>, Option<String>) {

    // --- Strategy 1: marker-based (handles any depth of wrapping) ---
    // Find "ue4ss/Mods/", "PalSchema/mods/", etc. ANYWHERE in the path.
    for name in files {
        if name.ends_with('/') { continue; }
        let lower = name.to_lowercase();

        for marker in MODS_DIR_MARKERS {
            if let Some(pos) = lower.find(marker) {
                let after = &name[pos + marker.len()..];
                let mod_name = after.split('/').next().unwrap_or("").to_string();
                if !mod_name.is_empty() && !is_forbidden(&mod_name) {
                    // mod_content_path = everything up to and including mod_name
                    let content_path = format!("{}{}", &name[..pos + marker.len()], mod_name);
                    return (Some(content_path), Some(mod_name));
                }
            }
        }
    }

    // --- Strategy 2: content-based structural heuristics ---
    match detected_type {
        DetectedModType::Ue4ss => {
            // "dlls/main.dll" or "Scripts" subfolder -> mod root is its parent
            for name in files {
                let lower = name.to_lowercase();
                if !lower.ends_with(".lua") && !lower.ends_with(".dll") { continue; }
                let parts: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
                for (i, &part) in parts.iter().enumerate() {
                    let part_lower = part.to_lowercase();
                    if (part_lower == "scripts" || part_lower == "dlls") && i > 0 {
                        let mod_name = parts[i - 1].to_string();
                        if !is_forbidden(&mod_name) {
                            let content_path = parts[..i].join("/");
                            return (Some(content_path), Some(mod_name));
                        }
                    }
                }
                // No Scripts/dlls subdir: file directly in mod folder
                let parts: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
                if parts.len() >= 2 {
                    let mod_name = parts[parts.len() - 2].to_string();
                    if !is_forbidden(&mod_name) {
                        let content_path = parts[..parts.len() - 1].join("/");
                        return (Some(content_path), Some(mod_name));
                    }
                }
            }
        }

        DetectedModType::PalSchema => {
            // "raw" or "translations" subfolder -> mod root is their parent
            for name in files {
                if name.ends_with('/') { continue; }
                let parts: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
                for (i, &part) in parts.iter().enumerate() {
                    let pl = part.to_lowercase();
                    if (pl == "raw" || pl == "translations") && i > 0 {
                        let mod_name = parts[i - 1].to_string();
                        if !is_forbidden(&mod_name) {
                            let content_path = parts[..i].join("/");
                            return (Some(content_path), Some(mod_name));
                        }
                    }
                }
            }
            // Flat case (blueprints at root): no mod root detected.
            // Caller must create a folder from the zip filename.
        }

        _ => {}
    }

    (None, None)
}

fn find_root_folder(files: &[String]) -> Option<String> {
    for name in files {
        if name.contains('/') {
            let first = name.split('/').next().unwrap_or("");
            let first_lower = first.to_lowercase();
            if !first.is_empty() && !FORBIDDEN_MOD_NAMES.contains(&first_lower.as_str()) {
                return Some(first.to_string());
            }
        }
    }
    None
}



/// Legacy: kept for install_commands.rs call site.
pub fn detect_mod_folder(files: &[String]) -> (Option<String>, bool, Option<String>, Option<String>) {
    let (content_path, mod_name) = detect_mod_content_and_name(files, &DetectedModType::Unknown);
    let has_game_path = content_path.is_some();
    (mod_name, has_game_path, None, content_path)
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

fn list_rar_files(path: &str) -> Result<Vec<String>, String> {
    let mut cmd = std::process::Command::new("tar");
    cmd.args(&["-tf", path]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd.output().map_err(|e| format!("Failed to run tar: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tar command failed: {}", err));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(|l| l.trim().replace('\\', "/"))
        .filter(|l| !l.is_empty())
        .collect())
}

fn extract_rar_to_temp(path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(temp_dir).map_err(|e| format!("Cannot create temp dir: {}", e))?;
    let mut cmd = std::process::Command::new("tar");
    cmd.args(&["-xf", path, "-C", &temp_dir.to_string_lossy()]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd.output().map_err(|e| format!("Failed to run tar: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tar extraction failed: {}", err));
    }
    Ok(temp_dir.to_path_buf())
}

pub fn analyze_zip(zip_path: &str) -> Result<ZipAnalysis, String> {
    let lower_path = zip_path.to_lowercase();
    let files = if lower_path.ends_with(".rar") || lower_path.ends_with(".7z") {
        list_rar_files(zip_path)?
    } else {
        let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
        let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;
        let mut list = Vec::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| format!("Cannot read entry: {}", e))?;
            list.push(entry.name().to_string());
        }
        list
    };

    let mut has_lua = false;
    let mut has_json = false;
    let mut has_pak = false;
    let mut has_dll = false;
    let mut has_info_json = false;
    let mut in_logicmods = false;
    let mut pak_destination_hint = None;

    for name in &files {
        let nl = name.to_lowercase();
        if nl.ends_with(".lua") { has_lua = true; }
        if nl.ends_with(".dll") { has_dll = true; }
        if nl.ends_with(".json") || nl.ends_with(".jsonc") {
            has_json = true;
            if nl.contains("info.json") || nl.contains("modinfo.pmm.json") { has_info_json = true; }
        }
        if nl.ends_with(".pak") { has_pak = true; }
        if nl.contains("logicmods") {
            in_logicmods = true;
            pak_destination_hint = Some("logicmods".to_string());
        }
    }

    // Determine type first, then run content detection with type hint
    let has_palschema_folder = files.iter().any(|f| f.to_lowercase().contains("palschema"));
    let has_palschema_json = files.iter().any(|f| {
        let fl = f.to_lowercase();
        if (fl.ends_with(".json") || fl.ends_with(".jsonc"))
            && !fl.ends_with("info.json")
            && !fl.ends_with("modinfo.json")
            && !fl.ends_with("modinfo.pmm.json")
            && !fl.ends_with("manifest.json")
            && !fl.ends_with("metadata.json")
        {
            let parts: Vec<&str> = fl.split('/').collect();
            parts.iter().any(|part| {
                matches!(
                    *part,
                    "pals"
                        | "spawns"
                        | "items"
                        | "blueprints"
                        | "unique"
                        | "enums"
                        | "skins"
                        | "translations"
                        | "raw"
                )
            })
        } else {
            false
        }
    });
    let has_ue4ss = has_lua || has_dll;
    let has_palschema = has_palschema_folder || has_palschema_json;
    let is_hybrid = (has_ue4ss && has_palschema) || (has_ue4ss && has_pak);

    let detected_type_pre = if is_hybrid {
        DetectedModType::Hybrid
    } else if has_lua || has_dll {
        DetectedModType::Ue4ss
    } else if has_palschema_folder {
        DetectedModType::PalSchema
    } else if has_pak {
        if in_logicmods {
            DetectedModType::LogicMods
        } else {
            DetectedModType::Pak
        }
    } else if has_json {
        DetectedModType::PalSchema
    } else {
        DetectedModType::Unknown
    };

    let root_folder = find_root_folder(&files);

    Ok(ZipAnalysis {
        detected_type: detected_type_pre,
        has_lua,
        has_json,
        has_palschema_json,
        has_pak,
        has_dll,
        has_info_json,
        pak_destination_hint,
        root_folder,
        files,
    })
}

pub fn extract_zip_to_temp(zip_path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    let lower_path = zip_path.to_lowercase();
    if lower_path.ends_with(".rar") || lower_path.ends_with(".7z") {
        return extract_rar_to_temp(zip_path, temp_dir);
    }
    let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;
    fs::create_dir_all(temp_dir).map_err(|e| format!("Cannot create temp dir: {}", e))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("Cannot read entry: {}", e))?;
        let outpath = temp_dir.join(entry.mangled_name());
        if entry.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| format!("Cannot create dir: {}", e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Cannot create parent dir: {}", e))?;
                }
            }
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Cannot create file: {}", e))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| format!("Cannot read entry: {}", e))?;
            std::io::Write::write_all(&mut outfile, &buf)
                .map_err(|e| format!("Cannot write file: {}", e))?;
        }
    }
    Ok(temp_dir.to_path_buf())
}

/// Recursively find all .pak files in a directory tree.
pub fn find_pak_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut paks = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                paks.extend(find_pak_files_recursive(&path));
            } else if path.extension().map(|e| e == "pak").unwrap_or(false) {
                paks.push(path);
            }
        }
    }
    paks
}

/// Find all companion files (.pak, .ucas, .utoc) for a given .pak stem.
pub fn find_pak_companions(pak_path: &Path) -> Vec<PathBuf> {
    let mut companions = Vec::new();
    if let (Some(parent), Some(stem)) = (pak_path.parent(), pak_path.file_stem()) {
        let stem_str = stem.to_string_lossy();
        for ext in &["pak", "ucas", "utoc"] {
            let companion = parent.join(format!("{}.{}", stem_str, ext));
            if companion.exists() {
                companions.push(companion);
            }
        }
    }
    companions
}

pub fn read_archive_file(zip_path: &str, target_file: &str) -> Option<String> {
    let lower_target = target_file.to_lowercase();
    let lower_zip = zip_path.to_lowercase();
    if lower_zip.ends_with(".rar") || lower_zip.ends_with(".7z") {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let temp_dir = std::env::temp_dir().join(format!("pmm_temp_{}", timestamp));
        let _ = fs::create_dir_all(&temp_dir);
        let mut cmd = std::process::Command::new("tar");
        cmd.args(&["-xf", zip_path, "-C", &temp_dir.to_string_lossy(), target_file]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);
        
        let mut result = None;
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let extracted_path = temp_dir.join(target_file);
                if let Ok(content) = fs::read_to_string(&extracted_path) {
                    result = Some(content);
                }
            }
        }
        let _ = fs::remove_dir_all(&temp_dir);
        result
    } else {
        if let Ok(file) = fs::File::open(zip_path) {
            if let Ok(mut archive) = ZipArchive::new(file) {
                for i in 0..archive.len() {
                    if let Ok(mut entry) = archive.by_index(i) {
                        if entry.name().to_lowercase() == lower_target {
                            let mut buf = String::new();
                            if entry.read_to_string(&mut buf).is_ok() {
                                return Some(buf);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

const GAME_PATH_SEGMENTS: &[&str] = &[
    "pal", "content", "paks", "~mods", "logicmods",
    "binaries", "win64", "wingdk",
    "ue4ss", "mods",
    "palschema",
];

const PALSCHEMA_FOLDERS: &[&str] = &[
    "resources", "enums", "pals", "npcs", "items", "skins",
    "appearance", "buildings", "raw", "blueprints", "helpguide",
    "spawns", "translations", "paks"
];

fn detect_folder_name_from_files(files: &[String], zip_filename: &str) -> String {
    for file in files {
        if file.ends_with('/') {
            continue;
        }
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        for segment in segments {
            let p = Path::new(segment);
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let stem_lower = stem.to_lowercase();
            let lower = segment.to_lowercase();
            if GAME_PATH_SEGMENTS.contains(&lower.as_str()) || GAME_PATH_SEGMENTS.contains(&stem_lower.as_str()) {
                continue;
            }
            // Skip platform wrappers
            if lower == "(steam)" || lower == "steam" || lower == "(xbox)" || lower == "xbox" ||
               lower == "(gdk)" || lower == "gdk" || lower == "wingdk" ||
               stem_lower == "(steam)" || stem_lower == "steam" || stem_lower == "(xbox)" || stem_lower == "xbox" ||
               stem_lower == "(gdk)" || stem_lower == "gdk" || stem_lower == "wingdk" {
                continue;
            }
            // Skip common wrapper/instruction directories
            let contains_wrapper_words = lower.contains("mods folder") || lower.contains("mod folder") ||
                                         lower.contains("ue4ss mods") || lower.contains("palschema mods") ||
                                         lower.contains("mods directory") || lower.contains("mod directory");
            if contains_wrapper_words || stem_lower.contains("mods folder") || stem_lower.contains("mod folder") ||
               stem_lower.contains("ue4ss mods") || stem_lower.contains("palschema mods") ||
               stem_lower.contains("mods directory") || stem_lower.contains("mod directory") {
                continue;
            }
            // Skip generic metadata/documentation files and images
            if lower == "license" || lower == "readme" || lower == "changelog" ||
               lower.ends_with(".txt") || lower.ends_with(".md") || lower.ends_with(".png") ||
               lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".git") ||
               lower.ends_with(".gitattributes") || lower.ends_with(".gitignore") {
                continue;
            }
            if PALSCHEMA_FOLDERS.contains(&lower.as_str()) || PALSCHEMA_FOLDERS.contains(&stem_lower.as_str()) {
                break;
            }
            if lower == "scripts" || lower == "dlls" || stem_lower == "scripts" || stem_lower == "dlls" {
                break;
            }
            if lower.ends_with(".dll") {
                return stem;
            }
            if lower.ends_with(".json") || lower.ends_with(".jsonc") || lower.ends_with(".pak") {
                continue;
            }
            return segment.to_string();
        }
    }
    crate::installer::clean_zip_name(zip_filename)
}

pub fn build_manifest_from_files(
    files: &[String],
    filename: &str,
    game_path: &Path,
    pak_destination: Option<&str>,
    custom_display_name: Option<String>,
    modinfo_data: Option<serde_json::Value>,
) -> Result<crate::models::InstallManifest, String> {
    use crate::models::{InstallManifest, FileRoute, RouteType, ModType};

    let mut custom_routes_map = std::collections::HashMap::new();
    if let Some(ref modinfo) = modinfo_data {
        if let Some(routes_arr) = modinfo.get("routes").and_then(|r| r.as_array()) {
            for route_val in routes_arr {
                if let (Some(zip_path), Some(route_type_str)) = (
                    route_val.get("zipPath").and_then(|z| z.as_str()),
                    route_val.get("routeType").and_then(|t| t.as_str()),
                ) {
                    let rtype = match route_type_str.to_lowercase().as_str() {
                        "ue4ss" => RouteType::Ue4ss,
                        "palschema" => RouteType::PalSchema,
                        "pak" => RouteType::Pak,
                        "logicmods" => RouteType::LogicMods,
                        "passthrough" => RouteType::Passthrough,
                        _ => continue,
                    };
                    custom_routes_map.insert(zip_path.replace('\\', "/").to_lowercase(), rtype);
                }
            }
        }
    }

    let parsed_nexus = crate::nexus::parse_mod_filename(filename);
    let binaries_dir = crate::dependency_checker::get_binaries_dir(game_path);
    let is_xbox = binaries_dir.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("wingdk".to_string());

    // 1. Detect if the ZIP contains both steam and xbox tags
    let has_steam_tags = files.iter().any(|f| {
        let fl = f.to_lowercase().replace('\\', "/");
        fl.contains("/(steam)/") || fl.starts_with("(steam)/") ||
        fl.contains("/steam/") || fl.starts_with("steam/") ||
        fl.contains("/win64/") || fl.starts_with("win64/")
    });
    let has_xbox_tags = files.iter().any(|f| {
        let fl = f.to_lowercase().replace('\\', "/");
        fl.contains("/(xbox)/") || fl.starts_with("(xbox)/") ||
        fl.contains("/xbox/") || fl.starts_with("xbox/") ||
        fl.contains("/(gdk)/") || fl.starts_with("(gdk)/") ||
        fl.contains("/gdk/") || fl.starts_with("gdk/") ||
        fl.contains("/wingdk/") || fl.starts_with("wingdk/")
    });
    let has_both_platforms = has_steam_tags && has_xbox_tags;

    let folder_name = detect_folder_name_from_files(files, filename);
    let folder_name_lower = folder_name.to_lowercase();

    let mut routes = Vec::new();
    let mut has_ue4ss = false;
    let mut has_palschema = false;
    let mut has_pak = false;

    // First pass: classify files and build relative paths
    let mut temp_routes = Vec::new();
    for file in files {
        if file.ends_with('/') {
            continue;
        }

        let normalized = file.replace('\\', "/");
        let lower = normalized.to_lowercase();

        // Skip inactive platform wrapper files
        if has_both_platforms {
            let segments: Vec<&str> = lower.split('/').collect();
            let is_inactive = if is_xbox {
                segments.iter().any(|&s| s == "(steam)" || s == "steam" || s == "win64")
            } else {
                segments.iter().any(|&s| s == "(xbox)" || s == "xbox" || s == "(gdk)" || s == "gdk" || s == "wingdk")
            };
            if is_inactive {
                continue;
            }
        }

        // Determine relative path from folder_name
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        let folder_idx = segments.iter().position(|s| s.to_lowercase() == folder_name_lower);

        let mut relative_path = if let Some(idx) = folder_idx {
            segments[idx + 1..].join("/")
        } else {
            // Strip any platform wrapper if present in the first segment
            let parts: Vec<&str> = normalized.split('/').collect();
            if parts.len() > 1 {
                let first_lower = parts[0].to_lowercase();
                let is_wrapper = first_lower == "(steam)" || first_lower == "steam" || first_lower == "win64" ||
                                 first_lower == "(xbox)" || first_lower == "xbox" || first_lower == "(gdk)" || first_lower == "gdk" || first_lower == "wingdk" ||
                                 first_lower.contains("mods folder") || first_lower.contains("mod folder") ||
                                 first_lower.contains("ue4ss mods") || first_lower.contains("palschema mods") ||
                                 first_lower.contains("mods directory") || first_lower.contains("mod directory");
                if is_wrapper {
                    parts[1..].join("/")
                } else {
                    normalized.clone()
                }
            } else {
                normalized.clone()
            }
        };

        let rel_lower_check = relative_path.to_lowercase();
        if rel_lower_check.starts_with("palschema/") {
            relative_path = relative_path["palschema/".len()..].to_string();
        }

        let rel_lower = relative_path.to_lowercase();
        let rel_segments: Vec<&str> = rel_lower.split('/').collect();

        // Classify RouteType
        let route_type = if let Some(rtype) = custom_routes_map.get(&lower)
            .or_else(|| custom_routes_map.get(&normalized.to_lowercase()))
            .or_else(|| custom_routes_map.get(&rel_lower))
        {
            match rtype {
                RouteType::Ue4ss => has_ue4ss = true,
                RouteType::PalSchema => has_palschema = true,
                RouteType::Pak => has_pak = true,
                RouteType::LogicMods => has_pak = true,
                _ => {}
            }
            rtype.clone()
        } else if rel_lower.ends_with(".pak") || rel_lower.ends_with(".ucas") || rel_lower.ends_with(".utoc") {
            // Check if any ancestor is a PalSchema folder
            let is_standard_game_path = rel_segments.contains(&"content") || rel_segments.contains(&"~mods") || rel_segments.contains(&"logicmods");
            let is_palschema_pak = if is_standard_game_path {
                false
            } else {
                rel_lower.contains("palschema/") || rel_segments.iter().any(|seg| PALSCHEMA_FOLDERS.contains(seg))
            };

            if is_palschema_pak {
                has_palschema = true;
                RouteType::PalSchema
            } else {
                has_pak = true;
                if rel_lower.contains("logicmods") || pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
                    RouteType::LogicMods
                } else {
                    RouteType::Pak
                }
            }
        } else if rel_lower.ends_with(".lua") || rel_lower.ends_with(".dll") {
            let is_ue4ss_code = rel_segments.contains(&"scripts") || rel_segments.contains(&"dlls");
            if is_ue4ss_code {
                has_ue4ss = true;
                RouteType::Ue4ss
            } else {
                has_ue4ss = true;
                RouteType::Ue4ss
            }
        } else {
            // Check if any ancestor is a PalSchema folder
            let mut is_palschema_asset = false;
            let is_standard_game_path = rel_segments.contains(&"content") || rel_segments.contains(&"~mods") || rel_segments.contains(&"logicmods");
            if !is_standard_game_path {
                for seg in &rel_segments {
                    if PALSCHEMA_FOLDERS.contains(seg) {
                        is_palschema_asset = true;
                        break;
                    }
                }
            }

            if is_palschema_asset || lower.contains("palschema/") {
                has_palschema = true;
                RouteType::PalSchema
            } else {
                RouteType::Passthrough
            }
        };

        temp_routes.push((file.clone(), relative_path, route_type));
    }

    // Determine primary destination type
    let primary_route_type = if has_ue4ss {
        RouteType::Ue4ss
    } else if has_palschema {
        RouteType::PalSchema
    } else if has_pak {
        if pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
            RouteType::LogicMods
        } else {
            RouteType::Pak
        }
    } else {
        RouteType::Passthrough
    };

    // Second pass: build absolute dest_path
    let paks_dest_dir = game_path.join("Pal").join("Content").join("Paks").join("~mods");
    let logicmods_dest_dir = game_path.join("Pal").join("Content").join("Paks").join("LogicMods");
    let ue4ss_mods_dest = binaries_dir.join("ue4ss").join("Mods");
    let palschema_mods_dest = ue4ss_mods_dest.join("PalSchema").join("mods");

    for (zip_path, relative_path, route_type) in temp_routes {
        let rel_lower = relative_path.to_lowercase();
        let game_subpath = if let Some(idx) = rel_lower.find("pal/binaries/win64/ue4ss/mods/") {
            Some(&relative_path[idx..])
        } else if let Some(idx) = rel_lower.find("pal/binaries/win64/") {
            Some(&relative_path[idx..])
        } else if let Some(idx) = rel_lower.find("pal/content/paks/") {
            Some(&relative_path[idx..])
        } else {
            None
        };

        let dest_path = if let Some(subpath) = game_subpath {
            game_path.join(subpath)
        } else {
            match route_type {
                RouteType::Ue4ss => {
                    let final_rel = if rel_lower.ends_with(".lua") && !relative_path.contains('/') {
                        format!("Scripts/{}", relative_path)
                    } else {
                        relative_path.clone()
                    };
                    ue4ss_mods_dest.join(&folder_name).join(final_rel)
                }
                RouteType::PalSchema => {
                    palschema_mods_dest.join(&folder_name).join(&relative_path)
                }
                RouteType::Pak => {
                    let filename = Path::new(&relative_path).file_name().unwrap();
                    paks_dest_dir.join(filename)
                }
                RouteType::LogicMods => {
                    let filename = Path::new(&relative_path).file_name().unwrap();
                    logicmods_dest_dir.join(filename)
                }
                RouteType::Companion => {
                    let filename = Path::new(&relative_path).file_name().unwrap();
                    let target_dir = if pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
                        &logicmods_dest_dir
                    } else {
                        &paks_dest_dir
                    };
                    target_dir.join(filename)
                }
                RouteType::Passthrough => {
                    match primary_route_type {
                        RouteType::Ue4ss => ue4ss_mods_dest.join(&folder_name).join(&relative_path),
                        RouteType::PalSchema => palschema_mods_dest.join(&folder_name).join(&relative_path),
                        RouteType::Pak | RouteType::LogicMods | RouteType::Companion | RouteType::Passthrough => {
                            let target_dir = if primary_route_type == RouteType::LogicMods {
                                &logicmods_dest_dir
                            } else {
                                &paks_dest_dir
                            };
                            let filename = Path::new(&relative_path).file_name().unwrap();
                            target_dir.join(filename)
                        }
                    }
                }
            }
        };

        let dest_str = dest_path.to_string_lossy().to_string();
        #[cfg(windows)]
        let dest_str = dest_str.replace('/', "\\");
        #[cfg(not(windows))]
        let dest_str = dest_str.replace('\\', "/");

        routes.push(FileRoute {
            zip_path,
            dest_path: dest_str,
            route_type,
        });
    }

    // Derive global mod type
    let mod_type = match (has_ue4ss, has_palschema, has_pak) {
        (true, false, false) => ModType::Ue4ss,
        (false, true, false) => ModType::PalSchema,
        (false, false, true) => {
            if pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
                ModType::LogicMods
            } else {
                ModType::Pak
            }
        }
        _ => ModType::Hybrid,
    };

    let mut display_name = custom_display_name
        .or(parsed_nexus.name)
        .unwrap_or_else(|| crate::installer::clean_zip_name(filename));

    let mut version = parsed_nexus.version.unwrap_or_else(|| "1.0".to_string());
    let mut nexus_mod_id = parsed_nexus.nexus_id;

    if let Some(ref modinfo) = modinfo_data {
        if let Some(n) = modinfo.get("name").and_then(|n| n.as_str()) {
            display_name = n.to_string();
        }
        if let Some(v) = modinfo.get("version").and_then(|v| v.as_str()) {
            version = v.to_string();
        } else if let Some(v) = modinfo.get("version").and_then(|v| v.as_f64()) {
            version = v.to_string();
        }
        if let Some(id) = modinfo.get("nexusModId").and_then(|id| id.as_u64()) {
            nexus_mod_id = Some(id as u32);
        }
    }

    Ok(InstallManifest {
        folder_name,
        display_name,
        mod_type,
        routes,
        nexus_mod_id,
        nexus_file_id: parsed_nexus.nexus_file_id,
        has_pak,
        has_ue4ss,
        has_palschema,
        version,
    })
}

pub fn build_install_manifest(
    zip_path: &str,
    game_path: &Path,
    pak_destination: Option<&str>,
    custom_display_name: Option<String>,
) -> Result<crate::models::InstallManifest, String> {
    let analysis = analyze_zip(zip_path)?;
    let filename = Path::new(zip_path).file_name().unwrap().to_string_lossy().to_string();
    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            if let Some(content) = read_archive_file(zip_path, target_file) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    modinfo_data = Some(val);
                }
            }
        }
    }
    build_manifest_from_files(&analysis.files, &filename, game_path, pak_destination, custom_display_name, modinfo_data)
}

