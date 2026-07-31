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
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ZipAnalysis {
    pub detected_type: DetectedModType,
    pub has_lua: bool,
    pub has_json: bool,
    pub has_pak: bool,
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
            // "Scripts" subfolder -> mod root is its parent
            for name in files {
                if !name.to_lowercase().ends_with(".lua") { continue; }
                let parts: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
                for (i, &part) in parts.iter().enumerate() {
                    if part.eq_ignore_ascii_case("scripts") && i > 0 {
                        let mod_name = parts[i - 1].to_string();
                        if !is_forbidden(&mod_name) {
                            let content_path = parts[..i].join("/");
                            return (Some(content_path), Some(mod_name));
                        }
                    }
                }
                // No Scripts subdir: .lua directly in mod folder
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
    let files = if lower_path.ends_with(".rar") {
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
    let mut has_info_json = false;
    let mut in_logicmods = false;
    let mut pak_destination_hint = None;

    for name in &files {
        let nl = name.to_lowercase();
        if nl.ends_with(".lua") { has_lua = true; }
        if nl.ends_with(".json") || nl.ends_with(".jsonc") {
            has_json = true;
            if nl.contains("info.json") { has_info_json = true; }
        }
        if nl.ends_with(".pak") { has_pak = true; }
        if nl.contains("logicmods") {
            in_logicmods = true;
            pak_destination_hint = Some("logicmods".to_string());
        }
    }

    // Determine type first, then run content detection with type hint
    let detected_type_pre = if has_lua {
        DetectedModType::Ue4ss
    } else if has_pak && in_logicmods {
        DetectedModType::LogicMods
    } else if has_pak {
        DetectedModType::Pak
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
        has_pak,
        has_info_json,
        pak_destination_hint,
        root_folder,
        files,
    })
}

pub fn extract_zip_to_temp(zip_path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    let lower_path = zip_path.to_lowercase();
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
