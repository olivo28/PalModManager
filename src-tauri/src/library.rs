use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub mod_id: String,
    pub zip_name: String,
    pub zip_size: u64,
    pub installed_at: String,
    pub nexus_picture_url: Option<String>,
    pub nexus_name: Option<String>,
    pub nexus_author: Option<String>,
    pub nexus_summary: Option<String>,
    pub nexus_mod_id: Option<u32>,
    pub nexus_version: Option<String>,
}


pub fn get_library_path(program_path: &str, mod_id: &str) -> PathBuf {
    PathBuf::from(program_path)
        .join("mods-library")
        .join(mod_id)
}

pub fn library_dir(program_path: &str) -> PathBuf {
    PathBuf::from(program_path).join("mods-library")
}

pub fn copy_to_library(
    source_zip: &str,
    program_path: &str,
    mod_id: &str,
) -> Result<LibraryEntry, String> {
    let lib_path = get_library_path(program_path, mod_id);
    fs::create_dir_all(&lib_path).map_err(|e| format!("Cannot create library dir: {}", e))?;

    let zip_path = Path::new(source_zip);
    let zip_name = zip_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown.zip".to_string());
    let dest = lib_path.join(&zip_name);

    fs::copy(source_zip, &dest).map_err(|e| format!("Cannot copy to library: {}", e))?;

    let zip_size = fs::metadata(&dest)
        .map(|m| m.len())
        .unwrap_or(0);
    let now = chrono::Utc::now().to_rfc3339();

    Ok(LibraryEntry {
        mod_id: mod_id.to_string(),
        zip_name,
        zip_size,
        installed_at: now,
        nexus_picture_url: None,
        nexus_name: None,
        nexus_author: None,
        nexus_summary: None,
        nexus_mod_id: None,
        nexus_version: None,
    })
}

#[allow(dead_code)]
pub fn remove_from_library(
    program_path: &str,
    mod_id: &str,
    zip_name: Option<&str>,
) -> Result<(), String> {
    let lib_path = get_library_path(program_path, mod_id);
    if let Some(zip) = zip_name {
        let file_path = lib_path.join(zip);
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| format!("Cannot remove file: {}", e))?;
        }
        // If directory is now empty (or only contains .nexus.json), clean it up
        if let Ok(entries) = fs::read_dir(&lib_path) {
            let mut has_other_zips = false;
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != ".nexus.json" {
                    has_other_zips = true;
                    break;
                }
            }
            if !has_other_zips {
                let _ = fs::remove_dir_all(&lib_path);
            }
        }
    } else {
        if lib_path.exists() {
            fs::remove_dir_all(&lib_path).map_err(|e| format!("Cannot remove from library: {}", e))?;
        }
    }
    Ok(())
}

pub fn list_library(program_path: &str) -> Result<Vec<LibraryEntry>, String> {
    let lib_dir = library_dir(program_path);
    if !lib_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&lib_dir).map_err(|e| format!("Cannot read library dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        if !entry.file_type().map_or(false, |t| t.is_dir()) {
            continue;
        }
        let mod_id = entry.file_name().to_string_lossy().to_string();

        let mut nexus_picture_url = None;
        let mut nexus_name = None;
        let mut nexus_author = None;
        let mut nexus_summary = None;
        let mut nexus_mod_id = None;
        let mut nexus_version = None;

        let nexus_json_path = entry.path().join(".nexus.json");
        if nexus_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&nexus_json_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    nexus_picture_url = val.get("pictureUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
                    nexus_name = val.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                    nexus_author = val.get("author").and_then(|v| v.as_str()).map(|s| s.to_string());
                    nexus_summary = val.get("summary").and_then(|v| v.as_str()).map(|s| s.to_string());
                    nexus_mod_id = val.get("modId").and_then(|v| v.as_u64()).map(|v| v as u32);
                    nexus_version = val.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
            }
        }

        if let Ok(dir_entries) = fs::read_dir(entry.path()) {
            for zip_entry in dir_entries.filter_map(|e| e.ok()) {
                let path = zip_entry.path();
                if path.is_file() {
                    let zip_name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if zip_name == ".nexus.json" { continue; } // Skip cache file
                    let zip_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    let now = chrono::Utc::now().to_rfc3339();
                    entries.push(LibraryEntry {
                        mod_id: mod_id.clone(),
                        zip_name,
                        zip_size,
                        installed_at: now,
                        nexus_picture_url: nexus_picture_url.clone(),
                        nexus_name: nexus_name.clone(),
                        nexus_author: nexus_author.clone(),
                        nexus_summary: nexus_summary.clone(),
                        nexus_mod_id,
                        nexus_version: nexus_version.clone(),
                    });
                }
            }
        }
    }

    Ok(entries)
}
