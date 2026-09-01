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
    Altermatic,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ZipAnalysis {
    pub detected_type: DetectedModType,
    pub has_lua: bool,
    pub has_json: bool,
    pub has_palschema_json: bool,
    pub has_pak: bool,
    pub has_altermatic: bool,
    #[allow(dead_code)]
    pub has_dll: bool,
    pub has_info_json: bool,
    pub pak_destination_hint: Option<String>,
    pub root_folder: Option<String>,
    pub files: Vec<String>,
}



fn find_root_folder(files: &[String]) -> Option<String> {
    for name in files {
        if name.contains('/') {
            let first = name.split('/').next().unwrap_or("");
            if !first.is_empty() && !is_forbidden(first) {
                return Some(first.to_string());
            }
        }
    }
    None
}

fn list_7z_files(path: &str) -> Result<Vec<String>, String> {
    let reader = sevenz_rust::SevenZReader::open(Path::new(path), sevenz_rust::Password::empty())
        .map_err(|e| format!("Cannot open .7z archive: {}", e))?;
    let mut files = Vec::new();
    for entry in reader.archive().files.iter() {
        if !entry.is_directory() {
            files.push(entry.name().replace('\\', "/"));
        }
    }
    Ok(files)
}

fn extract_7z_to_temp(path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(temp_dir).map_err(|e| format!("Cannot create temp dir: {}", e))?;
    sevenz_rust::decompress_file(Path::new(path), temp_dir)
        .map_err(|e| format!("Failed to extract .7z archive: {}", e))?;
    Ok(temp_dir.to_path_buf())
}

fn read_7z_file(path: &str, target_file: &str) -> Option<String> {
    let lower_target = target_file.to_lowercase();
    let mut reader = sevenz_rust::SevenZReader::open(Path::new(path), sevenz_rust::Password::empty()).ok()?;
    let mut content = None;
    let _ = reader.for_each_entries(|entry, reader| {
        let entry_name = entry.name().replace('\\', "/").to_lowercase();
        if entry_name == lower_target || entry_name.ends_with(&format!("/{}", lower_target)) {
            let mut buf = Vec::new();
            if reader.read_to_end(&mut buf).is_ok() {
                if let Ok(s) = String::from_utf8(buf) {
                    content = Some(s);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    });
    content
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
        return Err(format!("RAR archive reading failed: {}", err.trim()));
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
        return Err(format!("RAR extraction failed: {}", err.trim()));
    }
    Ok(temp_dir.to_path_buf())
}

pub fn analyze_zip(zip_path: &str) -> Result<ZipAnalysis, String> {
    let lower_path = zip_path.to_lowercase();
    let files = if lower_path.ends_with(".7z") {
        list_7z_files(zip_path)?
    } else if lower_path.ends_with(".rar") {
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

    let has_altermatic = files.iter().any(|f| {
        let fl = f.to_lowercase();
        fl.contains("swapjson")
            || fl.contains("alterconfig")
            || fl.ends_with(".swap.json")
            || (fl.ends_with(".json") && (fl.contains("skelmesh") || fl.contains("matreplace") || fl.contains("altermatic")))
    });

    let detected_type_pre = if is_hybrid {
        DetectedModType::Hybrid
    } else if has_altermatic {
        DetectedModType::Altermatic
    } else if has_palschema_folder || has_palschema || has_palschema_json {
        DetectedModType::PalSchema
    } else if has_lua || has_dll {
        DetectedModType::Ue4ss
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
        has_altermatic,
        has_dll,
        has_info_json,
        pak_destination_hint,
        root_folder,
        files,
    })
}

pub fn extract_zip_to_temp(zip_path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    let lower_path = zip_path.to_lowercase();
    if lower_path.ends_with(".7z") {
        return extract_7z_to_temp(zip_path, temp_dir);
    }
    if lower_path.ends_with(".rar") {
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
    if lower_zip.ends_with(".7z") {
        return read_7z_file(zip_path, target_file);
    }
    if lower_zip.ends_with(".rar") {
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

const FORBIDDEN_MOD_NAMES: &[&str] = &[
    "pal", "palworld", "mods", "win64", "wingdk", "binaries", "content", "paks", "~mods",
    "logicmods", "ue4ss", "palschema", "plugins", "scripts", "nativemods",
    "blueprints", "translations", "steam", "(steam)", "xbox", "(xbox)", "gdk", "(gdk)",
    "gamepass", "(gamepass)", "swapjson", "alterconfig", "release", "build", "dist",
];

fn is_forbidden(name: &str) -> bool {
    let lower = name.to_lowercase();
    let trimmed = lower.trim_matches(|c: char| c == '(' || c == ')' || c == '[' || c == ']' || c.is_whitespace());
    FORBIDDEN_MOD_NAMES.contains(&lower.as_str())
        || FORBIDDEN_MOD_NAMES.contains(&trimmed)
        || lower.is_empty()
        || lower.starts_with("(steam)")
        || lower.starts_with("(xbox)")
        || lower.starts_with("(gdk)")
        || lower.starts_with("(gamepass)")
        || lower.contains("mods folder")
        || lower.contains("mod folder")
        || lower.contains("ue4ss mods")
        || lower.contains("palschema mods")
        || lower.contains("mods directory")
        || lower.contains("mod directory")
}

pub fn detect_folder_name_from_files(files: &[String], zip_filename: &str) -> String {
    // Strategy 1a: Scan strictly for UE4SS / LogicMods markers (scripts, dlls, logicmods)
    // to ensure the UE4SS mod name is prioritized in hybrid mods (important for mods.txt).
    for file in files {
        if file.ends_with('/') {
            continue;
        }
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                let lower = segment.to_lowercase();
                if lower == "scripts" || lower == "dlls" || lower == "logicmods" {
                    // Climb up candidate parents to find a non-forbidden name
                    let mut j = i as isize - 1;
                    while j >= 0 {
                        let candidate = segments[j as usize];
                        if !is_forbidden(candidate) {
                            return candidate.to_string();
                        }
                        j -= 1;
                    }
                }
            }
        }
    }

    // Strategy 1b: Fallback scan for PalSchema boundary markers.
    for file in files {
        if file.ends_with('/') {
            continue;
        }
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                let lower = segment.to_lowercase();
                if PALSCHEMA_FOLDERS.contains(&lower.as_str()) {
                    // Climb up candidate parents to find a non-forbidden name
                    let mut j = i as isize - 1;
                    while j >= 0 {
                        let candidate = segments[j as usize];
                        if !is_forbidden(candidate) {
                            return candidate.to_string();
                        }
                        j -= 1;
                    }
                }
            }
        }
    }

    // Strategy 2: Fallback to the first non-forbidden, non-file segment
    for file in files {
        if file.ends_with('/') {
            continue;
        }
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        let num_segments = segments.len();
        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == num_segments - 1;
            
            // Skip the last segment (leaf file name) to avoid treating batch/txt/png/lua files as folder names
            if is_last {
                let lower = segment.to_lowercase();
                let stem = Path::new(segment).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if lower.ends_with(".dll") {
                    return stem;
                }
                continue;
            }

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
               lower.ends_with(".gitattributes") || lower.ends_with(".gitignore") ||
               lower.ends_with(".bat") || lower.ends_with(".sh") || lower.ends_with(".py") {
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

    // Strategy 3: Check for .pak file stem (for pure PAK mods)
    for file in files {
        let normalized = file.replace('\\', "/");
        let lower = normalized.to_lowercase();
        if lower.ends_with(".pak") {
            if let Some(leaf) = normalized.split('/').last() {
                let pak_stem = Path::new(leaf).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let cleaned_pak = crate::installer::clean_zip_name(&pak_stem);
                if !cleaned_pak.is_empty() && cleaned_pak != "unknown" && !cleaned_pak.starts_with("nexus_") {
                    return cleaned_pak;
                }
                if !pak_stem.is_empty() {
                    return pak_stem;
                }
            }
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

    let mut folder_name = detect_folder_name_from_files(files, filename);
    let is_forbidden_folder = is_forbidden(&folder_name);
    let is_uuid = folder_name.len() >= 32 && folder_name.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if folder_name.is_empty() || folder_name == "unknown" || folder_name.starts_with("nexus_") || is_uuid || is_forbidden_folder {
        if let Some(ref disp) = custom_display_name {
            let cleaned = crate::installer::clean_zip_name(disp);
            if !cleaned.is_empty() && cleaned != "unknown" && !is_forbidden(&cleaned) {
                folder_name = cleaned;
            }
        }
        if folder_name.is_empty() || is_forbidden(&folder_name) {
            if let Some(ref n_name) = parsed_nexus.name {
                if !n_name.is_empty() && !is_forbidden(n_name) {
                    folder_name = n_name.clone();
                }
            }
        }
        if folder_name.is_empty() || is_forbidden(&folder_name) {
            let fallback = crate::installer::clean_zip_name(filename);
            if !fallback.is_empty() && !is_forbidden(&fallback) {
                folder_name = fallback;
            }
        }
    }
    let folder_name_lower = folder_name.to_lowercase();

    // Collect all distinct UE4SS mod root directory names from the files list
    let mut detected_ue4ss_roots: Vec<String> = Vec::new();
    for file in files {
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        for (i, segment) in segments.iter().enumerate() {
            let lower = segment.to_lowercase();
            if (lower == "scripts" || lower == "dlls" || lower == "enabled.txt") && i > 0 {
                let candidate = segments[i - 1];
                if !is_forbidden(candidate) && !detected_ue4ss_roots.iter().any(|r| r.eq_ignore_ascii_case(candidate)) {
                    detected_ue4ss_roots.push(candidate.to_string());
                }
            }
        }
    }

    let mut routes = Vec::new();
    let mut has_ue4ss = false;
    let mut has_palschema = false;
    let mut has_pak = false;

    // First pass: classify files and build relative paths
    let mut temp_routes = Vec::new();
    for file in files {
        let normalized = file.replace('\\', "/");
        let lower = normalized.to_lowercase();

        // Skip directory-only paths and folders (must contain a file extension to be mapped as a file)
        if normalized.ends_with('/') || normalized.split('/').last().map(|s| !s.contains('.')).unwrap_or(true) {
            continue;
        }

        // Handle the new Workshop route wrapping (Mods/NativeMods/UE4SS/Mods/)
        let normalized_clean = if lower.contains("mods/nativemods/ue4ss/mods/") {
            let idx = lower.find("mods/nativemods/ue4ss/mods/").unwrap();
            normalized[idx + "mods/nativemods/ue4ss/mods/".len()..].to_string()
        } else {
            normalized.clone()
        };
        let lower_clean = normalized_clean.to_lowercase();
        let segments: Vec<&str> = normalized_clean.split('/').filter(|s| !s.is_empty()).collect();

        // Skip inactive platform wrapper files
        if has_both_platforms {
            let is_inactive = if is_xbox {
                segments.iter().any(|&s| {
                    let sl = s.to_lowercase();
                    sl == "(steam)" || sl == "steam" || sl == "win64"
                })
            } else {
                segments.iter().any(|&s| {
                    let sl = s.to_lowercase();
                    sl == "(xbox)" || sl == "xbox" || sl == "(gdk)" || sl == "gdk" || sl == "wingdk"
                })
            };
            if is_inactive {
                continue;
            }
        }

        // Find the folder_name marker.
        // It must be an ancestor root wrapper, NOT an internal child folder inside "scripts", "dlls", "palschema", etc.
        let folder_idx = segments.iter().position(|s| s.to_lowercase() == folder_name_lower);
        let is_valid_root_folder = if let Some(idx) = folder_idx {
            let prior_segments = &segments[..idx];
            let is_inside_subfolder = prior_segments.iter().any(|s| {
                let sl = s.to_lowercase();
                sl == "scripts" || sl == "dlls" || sl == "mods" || sl == "palschema" || PALSCHEMA_FOLDERS.contains(&sl.as_str())
            });
            !is_inside_subfolder
        } else {
            false
        };

        let mut relative_path = if is_valid_root_folder {
            let idx = folder_idx.unwrap();
            segments[idx + 1..].join("/")
        } else {
            // Strip any platform wrapper if present in the first segment
            let parts: Vec<&str> = normalized_clean.split('/').collect();
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
                    normalized_clean.clone()
                }
            } else {
                normalized_clean.clone()
            }
        };

        // If this is a PalSchema folder path, we must extract relative path starting AFTER the mods/ segment to isolate the schema subdirectory
        if lower_clean.contains("palschema/mods/") {
            if let Some(pos) = lower_clean.find("palschema/mods/") {
                let schema_subpath = &normalized_clean[pos + "palschema/mods/".len()..];
                relative_path = schema_subpath.to_string();
            }
        } else if lower_clean.contains("ue4ss/mods/") {
            if let Some(pos) = lower_clean.find("ue4ss/mods/") {
                let ue4ss_subpath = &normalized_clean[pos + "ue4ss/mods/".len()..];
                let sub_lower = ue4ss_subpath.to_lowercase();
                if sub_lower.starts_with(&format!("{}/", folder_name_lower)) {
                    relative_path = ue4ss_subpath[folder_name_lower.len() + 1..].to_string();
                } else {
                    relative_path = ue4ss_subpath.to_string();
                }
            }
        } else {
            let rel_lower_check = relative_path.to_lowercase();
            if rel_lower_check.starts_with("ue4ss/mods/") {
                relative_path = relative_path["ue4ss/mods/".len()..].to_string();
                let sub_lower = relative_path.to_lowercase();
                if sub_lower.starts_with(&format!("{}/", folder_name_lower)) {
                    relative_path = relative_path[folder_name_lower.len() + 1..].to_string();
                }
            } else if rel_lower_check.starts_with("mods/") {
                relative_path = relative_path["mods/".len()..].to_string();
                let sub_lower = relative_path.to_lowercase();
                if sub_lower.starts_with(&format!("{}/", folder_name_lower)) {
                    relative_path = relative_path[folder_name_lower.len() + 1..].to_string();
                }
            } else if rel_lower_check.starts_with("ue4ss/") {
                relative_path = relative_path["ue4ss/".len()..].to_string();
            } else if rel_lower_check.starts_with("palschema/mods/") {
                relative_path = relative_path["palschema/mods/".len()..].to_string();
            } else if rel_lower_check.starts_with("palschema/") {
                relative_path = relative_path["palschema/".len()..].to_string();
            }
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

            let is_ue4ss_asset = rel_segments.iter().any(|seg| detected_ue4ss_roots.iter().any(|r| r.eq_ignore_ascii_case(seg)))
                || lower.contains("ue4ss/mods/")
                || rel_lower.ends_with("enabled.txt");

            if is_palschema_asset || lower.contains("palschema/") {
                has_palschema = true;
                RouteType::PalSchema
            } else if is_ue4ss_asset {
                has_ue4ss = true;
                RouteType::Ue4ss
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
    let ue4ss_mods_dest = crate::dependency_checker::get_ue4ss_mods_dir(game_path);
    let palschema_mods_dest = ue4ss_mods_dest.join("PalSchema").join("mods");

    for (zip_path, relative_path, route_type) in temp_routes {
        if relative_path.is_empty() {
            continue;
        }
        let file_name_opt = Path::new(&relative_path).file_name();
        if file_name_opt.is_none() {
            continue;
        }
        let filename = file_name_opt.unwrap();
        let rel_lower = relative_path.to_lowercase();
        let binaries_name = binaries_dir.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| "win64".to_string());
        
        let norm_zip = zip_path.replace('\\', "/");
        let zip_lower = norm_zip.to_lowercase();

        let game_subpath = if let Some(idx) = zip_lower.find("pal/content/paks/") {
            Some(norm_zip[idx..].to_string())
        } else if let Some(idx) = zip_lower.find("pal/binaries/") {
            let sub = &norm_zip[idx..];
            let sub_lower = sub.to_lowercase();
            // Adapt win64/wingdk to target system
            if sub_lower.starts_with("pal/binaries/win64/") && binaries_name == "wingdk" {
                Some(format!("Pal/Binaries/WinGDK/{}", &sub["pal/binaries/win64/".len()..]))
            } else if sub_lower.starts_with("pal/binaries/wingdk/") && binaries_name == "win64" {
                Some(format!("Pal/Binaries/Win64/{}", &sub["pal/binaries/wingdk/".len()..]))
            } else {
                Some(sub.to_string())
            }
        } else if let Some(idx) = rel_lower.find("mods/nativemods/ue4ss/mods/") {
            Some(relative_path[idx..].to_string())
        } else {
            None
        };

        let dest_path = if let Some(ref subpath) = game_subpath {
            game_path.join(subpath)
        } else {
            match route_type {
                RouteType::Ue4ss => {
                    let norm_file = zip_path.replace('\\', "/");
                    let norm_parts: Vec<&str> = norm_file.split('/').filter(|s| !s.is_empty()).collect();

                    let mut matched_root = None;
                    let mut matched_subpath = None;

                    for root in &detected_ue4ss_roots {
                        let root_lower = root.to_lowercase();
                        if let Some(pos) = norm_parts.iter().position(|p| p.to_lowercase() == root_lower) {
                            matched_root = Some(root.clone());
                            matched_subpath = Some(norm_parts[pos + 1..].join("/"));
                            break;
                        }
                    }

                    if let (Some(root), Some(subpath)) = (matched_root, matched_subpath) {
                        let final_sub = if subpath.to_lowercase().ends_with(".lua") && !subpath.contains('/') {
                            format!("Scripts/{}", subpath)
                        } else {
                            subpath
                        };
                        ue4ss_mods_dest.join(&root).join(final_sub)
                    } else {
                        let final_rel = if rel_lower.ends_with(".lua") && !relative_path.contains('/') {
                            format!("Scripts/{}", relative_path)
                        } else {
                            relative_path.clone()
                        };
                        ue4ss_mods_dest.join(&folder_name).join(final_rel)
                    }
                }
                RouteType::PalSchema => {
                    let rel_segments: Vec<&str> = relative_path.split('/').collect();
                    let first_seg_lower = rel_segments.first().map(|s| s.to_lowercase()).unwrap_or_default();
                    
                    // If the relative path starts directly with a PalSchema loader folder (e.g. items/, raw/, pals/), 
                    // it needs the mod folder prepended.
                    // Otherwise, the first segment is ALREADY the mod's specific PalSchema folder (e.g. 000_PassiveTraitExtraction/)!
                    if PALSCHEMA_FOLDERS.contains(&first_seg_lower.as_str()) {
                        palschema_mods_dest.join(&folder_name).join(&relative_path)
                    } else {
                        palschema_mods_dest.join(&relative_path)
                    }
                }
                RouteType::Pak => {
                    paks_dest_dir.join(filename)
                }
                RouteType::LogicMods => {
                    logicmods_dest_dir.join(filename)
                }
                RouteType::Companion => {
                    let target_dir = if pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
                        &logicmods_dest_dir
                    } else {
                        &paks_dest_dir
                    };
                    target_dir.join(filename)
                }
                RouteType::Passthrough => {
                    let rel_lower = relative_path.to_lowercase();
                    if rel_lower.contains("swapjson") {
                        paks_dest_dir.join("SwapJSON").join(filename)
                    } else if rel_lower.contains("alterconfig") {
                        paks_dest_dir.join("AlterConfig").join(filename)
                    } else if rel_lower.contains("json_templates") || rel_lower.contains("jsontemplates") {
                        paks_dest_dir.join("JSON_Templates").join(filename)
                    } else {
                        match primary_route_type {
                            RouteType::Ue4ss => {
                                let norm_file = zip_path.replace('\\', "/");
                                let norm_parts: Vec<&str> = norm_file.split('/').filter(|s| !s.is_empty()).collect();

                                let mut matched_root = None;
                                let mut matched_subpath = None;

                                for root in &detected_ue4ss_roots {
                                    let root_lower = root.to_lowercase();
                                    if let Some(pos) = norm_parts.iter().position(|p| p.to_lowercase() == root_lower) {
                                        matched_root = Some(root.clone());
                                        matched_subpath = Some(norm_parts[pos + 1..].join("/"));
                                        break;
                                    }
                                }

                                if let (Some(root), Some(subpath)) = (matched_root, matched_subpath) {
                                    ue4ss_mods_dest.join(&root).join(subpath)
                                } else {
                                    ue4ss_mods_dest.join(&folder_name).join(&relative_path)
                                }
                            }
                            RouteType::PalSchema => palschema_mods_dest.join(&folder_name).join(&relative_path),
                            RouteType::Pak | RouteType::LogicMods | RouteType::Companion | RouteType::Passthrough => {
                                let target_dir = if primary_route_type == RouteType::LogicMods {
                                    &logicmods_dest_dir
                                } else {
                                    &paks_dest_dir
                                };
                                target_dir.join(filename)
                            }
                        }
                    }
                }
            }
        };

        let is_doc_or_image = {
            let fl = rel_lower.as_str();
            (fl.ends_with(".txt") && !fl.ends_with("enabled.txt") && !fl.ends_with("mod.txt") && !fl.ends_with("info.txt"))
                || fl.ends_with(".md")
                || fl.ends_with(".url")
                || fl.ends_with(".png")
                || fl.ends_with(".jpg")
                || fl.ends_with(".jpeg")
                || fl.ends_with(".gif")
                || fl.ends_with(".pdf")
        };

        if is_doc_or_image && (route_type == RouteType::Pak || route_type == RouteType::LogicMods || route_type == RouteType::Passthrough) {
            continue;
        }

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

    let has_altermatic = files.iter().any(|f| {
        let fl = f.to_lowercase();
        fl.contains("swapjson")
            || fl.contains("alterconfig")
            || fl.ends_with(".swap.json")
            || (fl.ends_with(".json") && (fl.contains("skelmesh") || fl.contains("matreplace") || fl.contains("altermatic")))
    });

    // Derive global mod type
    let mod_type = if has_altermatic {
        ModType::Altermatic
    } else {
        match (has_ue4ss, has_palschema, has_pak) {
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
        }
    };

    let mut display_name = custom_display_name
        .filter(|n| !n.trim().is_empty() && !n.starts_with("Mod #") && !n.starts_with("nexus_") && n != "unknown")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_pal_insight_manifest_routing() {
        let zip_file = "C:/Users/Antikux/Downloads/Pal Insight 1.6.0 4638 1.6.0 2026-08-26T17-37Z 5V0tE6iaU.zip";
        if !std::path::Path::new(zip_file).exists() {
            return;
        }
        let game_path = PathBuf::from("C:/FakeGamePath");
        let manifest = build_install_manifest(zip_file, &game_path, None, None).expect("Should build manifest");

        println!("Pal Insight Manifest: folder_name={}, version={}, mod_type={:?}", manifest.folder_name, manifest.version, manifest.mod_type);
        for r in &manifest.routes {
            println!("  ROUTE: {} -> {}", r.zip_path, r.dest_path);
        }

        // Verify PalInsight and PalInsightSettings are routed separately without nesting
        let has_mangled_nesting = manifest.routes.iter().any(|r| {
            r.dest_path.contains("PalInsightSettings\\PalInsight") || r.dest_path.contains("PalInsightSettings/PalInsight")
        });
        assert!(!has_mangled_nesting, "Should not nest PalInsight under PalInsightSettings!");

        let has_palinsight_lua = manifest.routes.iter().any(|r| {
            r.dest_path.ends_with("PalInsight\\Scripts\\main.lua") || r.dest_path.ends_with("PalInsight/Scripts/main.lua")
        });
        assert!(has_palinsight_lua, "PalInsight main.lua must be in PalInsight/Scripts/main.lua");

        let has_settings_lua = manifest.routes.iter().any(|r| {
            r.dest_path.ends_with("PalInsightSettings\\Scripts\\main.lua") || r.dest_path.ends_with("PalInsightSettings/Scripts/main.lua")
        });
        assert!(has_settings_lua, "PalInsightSettings main.lua must be in PalInsightSettings/Scripts/main.lua");

        let has_pak = manifest.routes.iter().any(|r| {
            r.dest_path.contains("LogicMods") && r.dest_path.ends_with("PalInsightX.pak")
        });
        assert!(has_pak, "PalInsightX.pak must be in LogicMods");
    }

    #[test]
    fn test_expedition_timer_hud_manifest_routing() {
        let zip_file = "C:/Users/Antikux/Downloads/ExpeditionTimerHUD 4687 8 2026-08-26T20-02Z tlYGMmRGl.zip";
        if !std::path::Path::new(zip_file).exists() {
            return;
        }
        let game_path = PathBuf::from("C:/FakeGamePath");
        let analysis = analyze_zip(zip_file).expect("Should analyze zip");
        println!("Analysis: {:?}", analysis);
        let manifest = build_install_manifest(zip_file, &game_path, None, None).expect("Should build manifest");

        println!("ExpeditionTimerHUD Manifest: folder_name={}, version={}, mod_type={:?}", manifest.folder_name, manifest.version, manifest.mod_type);
        for r in &manifest.routes {
            println!("  ROUTE: {} -> {} (type: {:?})", r.zip_path, r.dest_path, r.route_type);
        }
    }
}

