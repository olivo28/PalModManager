use std::path::{Path, PathBuf};
use std::fs;

/// Palworld Steam Application ID
pub const PALWORLD_STEAM_APP_ID: &str = "1623730";

/// Validates whether a given path string points to a Palworld installation.
/// Returns Ok(canonical_path) if valid, or Err(descriptive_error) if not.
pub fn validate_game_path_str(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path is empty".to_string());
    }

    let p = PathBuf::from(trimmed);
    if !p.exists() {
        return Err("Path does not exist on disk".to_string());
    }

    match crate::dependency_checker::detect_game_root(&p) {
        Some(root) => Ok(root.to_string_lossy().into_owned()),
        None => Err("Path does not look like a Palworld installation (missing Pal/Content/Paks and Pal/Binaries)".to_string()),
    }
}

#[tauri::command]
pub fn validate_game_path(path: String) -> Result<String, String> {
    validate_game_path_str(&path)
}

#[tauri::command]
pub fn auto_detect_game_path() -> Result<Option<String>, String> {
    let detected = auto_detect_palworld_path();
    if let Some(ref p) = detected {
        crate::logger::log(&format!("game_detector: Auto-detected Palworld installation at '{}'", p));
    } else {
        crate::logger::log("game_detector: No Palworld installation found via auto-detection");
    }
    Ok(detected)
}

/// Automatically searches for Palworld installations across Steam libraries and default drive locations.
pub fn auto_detect_palworld_path() -> Option<String> {
    // 1. Scan Steam library folders from VDF configurations
    for vdf_path in get_steam_vdf_locations() {
        if vdf_path.is_file() {
            if let Some(found) = parse_vdf_and_find_palworld(&vdf_path) {
                return Some(found);
            }
        }
    }

    // 2. Scan standard drive paths and common library locations
    for candidate in get_candidate_game_paths() {
        if candidate.exists() {
            if let Some(canonical) = crate::dependency_checker::detect_game_root(&candidate) {
                return Some(canonical.to_string_lossy().into_owned());
            }
        }
    }

    None
}

/// Discovers common steamapps/libraryfolders.vdf paths across operating systems.
fn get_steam_vdf_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // Try reading Steam installation path from Windows Registry
        if let Ok(reg_path) = get_steam_path_from_registry() {
            locations.push(PathBuf::from(reg_path).join("steamapps").join("libraryfolders.vdf"));
        }

        // Standard Windows Steam directories
        locations.push(PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\libraryfolders.vdf"));
        locations.push(PathBuf::from(r"C:\Program Files\Steam\steamapps\libraryfolders.vdf"));
        locations.push(PathBuf::from(r"C:\Steam\steamapps\libraryfolders.vdf"));

        // Check root drives for Steam\steamapps\libraryfolders.vdf
        for drive in b'C'..=b'Z' {
            let drive_char = drive as char;
            locations.push(PathBuf::from(format!(r"{}:\Steam\steamapps\libraryfolders.vdf", drive_char)));
            locations.push(PathBuf::from(format!(r"{}:\SteamLibrary\steamapps\libraryfolders.vdf", drive_char)));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let home_p = PathBuf::from(home);
            locations.push(home_p.join(".local/share/Steam/steamapps/libraryfolders.vdf"));
            locations.push(home_p.join(".steam/steam/steamapps/libraryfolders.vdf"));
            locations.push(home_p.join(".steam/root/steamapps/libraryfolders.vdf"));
            locations.push(PathBuf::from("/home/deck/.local/share/Steam/steamapps/libraryfolders.vdf"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let home_p = PathBuf::from(home);
            locations.push(home_p.join("Library/Application Support/Steam/steamapps/libraryfolders.vdf"));
        }
    }

    locations
}

#[cfg(target_os = "windows")]
fn get_steam_path_from_registry() -> Result<String, ()> {
    use std::process::Command;
    // Query registry via reg.exe for HKCU\Software\Valve\Steam\SteamPath
    let output = Command::new("reg")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .output()
        .map_err(|_| ())?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("SteamPath") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let path = parts[2..].join(" ");
                    return Ok(path);
                }
            }
        }
    }
    Err(())
}

/// Parses a Steam libraryfolders.vdf file to extract library base directories.
fn parse_vdf_and_find_palworld(vdf_path: &Path) -> Option<String> {
    let content = fs::read_to_string(vdf_path).ok()?;
    let mut current_lib_path: Option<PathBuf> = None;
    let mut has_palworld_app = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Extract "path" "<path_string>"
        if trimmed.starts_with("\"path\"") {
            if let Some(path_val) = extract_vdf_quoted_value(trimmed) {
                let clean_path = path_val.replace(r"\\", r"\");
                current_lib_path = Some(PathBuf::from(clean_path));
                has_palworld_app = false;
            }
        }

        // Check if Palworld app ID is listed under this library
        if trimmed.contains(PALWORLD_STEAM_APP_ID) {
            has_palworld_app = true;
        }

        // If block closes or we detected Palworld, test the location
        if (trimmed == "}" || has_palworld_app) && current_lib_path.is_some() {
            if let Some(ref lib) = current_lib_path {
                let candidate = lib.join("steamapps").join("common").join("Palworld");
                if candidate.exists() {
                    if let Some(root) = crate::dependency_checker::detect_game_root(&candidate) {
                        return Some(root.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    None
}

/// Helper to extract the second quoted string in a line like: `"path" "D:\\SteamLibrary"`
fn extract_vdf_quoted_value(line: &str) -> Option<String> {
    let quotes: Vec<usize> = line.match_indices('"').map(|(i, _)| i).collect();
    if quotes.len() >= 4 {
        let val = &line[quotes[2] + 1..quotes[3]];
        return Some(val.to_string());
    }
    None
}

/// Common candidate directories across various drives and gaming platforms.
fn get_candidate_game_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "windows")]
    {
        for drive in b'C'..=b'Z' {
            let d = drive as char;
            // Steam library variations
            candidates.push(PathBuf::from(format!(r"{}:\SteamLibrary\steamapps\common\Palworld", d)));
            candidates.push(PathBuf::from(format!(r"{}:\Steam\steamapps\common\Palworld", d)));
            candidates.push(PathBuf::from(format!(r"{}:\Program Files (x86)\Steam\steamapps\common\Palworld", d)));
            candidates.push(PathBuf::from(format!(r"{}:\Program Files\Steam\steamapps\common\Palworld", d)));
            candidates.push(PathBuf::from(format!(r"{}:\Games\Steam\steamapps\common\Palworld", d)));
            candidates.push(PathBuf::from(format!(r"{}:\Games\SteamLibrary\steamapps\common\Palworld", d)));
            candidates.push(PathBuf::from(format!(r"{}:\Games\Palworld", d)));

            // Xbox Game Pass locations
            candidates.push(PathBuf::from(format!(r"{}:\XboxGames\Palworld\Content", d)));
            candidates.push(PathBuf::from(format!(r"{}:\XboxGames\Palworld", d)));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let home_p = PathBuf::from(home);
            candidates.push(home_p.join(".local/share/Steam/steamapps/common/Palworld"));
            candidates.push(home_p.join(".steam/steam/steamapps/common/Palworld"));
            candidates.push(PathBuf::from("/home/deck/.local/share/Steam/steamapps/common/Palworld"));
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_and_nonexistent_paths() {
        assert!(validate_game_path_str("").is_err());
        assert!(validate_game_path_str("   ").is_err());
        assert!(validate_game_path_str(r"Z:\Definitely\Does\Not\Exist\12345").is_err());
    }

    #[test]
    fn test_extract_vdf_quoted_value() {
        let line = r#"		"path"		"D:\\SteamLibrary""#;
        assert_eq!(extract_vdf_quoted_value(line), Some(r"D:\\SteamLibrary".to_string()));

        let app_line = r#"			"1623730"		"123456789""#;
        assert_eq!(extract_vdf_quoted_value(app_line), Some("123456789".to_string()));

        let invalid_line = r#"not quoted"#;
        assert_eq!(extract_vdf_quoted_value(invalid_line), None);
    }
}
