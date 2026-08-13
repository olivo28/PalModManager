use std::fs;
use std::path::{Path, PathBuf};
use crate::models::ModInfo;

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
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub mod_type: Option<String>,
    pub is_installed: bool,
    pub installed_version: Option<String>,
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

    // Copy source sidecar if present
    let src_sidecar = PathBuf::from(format!("{}.pmm.json", source_zip));
    let dst_sidecar = PathBuf::from(format!("{}.pmm.json", dest.to_string_lossy()));
    if src_sidecar.exists() {
        let _ = fs::copy(&src_sidecar, &dst_sidecar);
    }

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
        author: None,
        description: None,
        version: None,
        mod_type: None,
        is_installed: false,
        installed_version: None,
    })
}

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
        let sidecar = PathBuf::from(format!("{}.pmm.json", file_path.to_string_lossy()));
        if sidecar.exists() {
            let _ = fs::remove_file(sidecar);
        }
        let sidecar2 = file_path.with_extension("pmm.json");
        if sidecar2.exists() {
            let _ = fs::remove_file(sidecar2);
        }
        // If directory is now empty (or only contains .nexus.json or leftover .pmm.json), clean it up
        if let Ok(entries) = fs::read_dir(&lib_path) {
            let mut has_other_zips = false;
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != ".nexus.json" && !name.ends_with(".pmm.json") {
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

pub fn list_library(program_path: &str, installed_mods: &[ModInfo]) -> Result<Vec<LibraryEntry>, String> {
    let lib_dir = library_dir(program_path);
    if !lib_dir.exists() {
        return Ok(Vec::new());
    }

    let normalize = |s: &str| -> String {
        s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
    };

    // First pass: consolidate fragmented folders without .nexus.json into canonical folders
    if let Ok(dir_entries) = fs::read_dir(&lib_dir) {
        let folder_paths: Vec<PathBuf> = dir_entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map_or(false, |t| t.is_dir()))
            .map(|e| e.path())
            .collect();

        for folder in &folder_paths {
            let nexus_path = folder.join(".nexus.json");
            let folder_name = folder.file_name().unwrap_or_default().to_string_lossy().to_string();
            let norm_folder = normalize(&folder_name);

            // If this folder has NO .nexus.json, check if another canonical folder exists
            if !nexus_path.exists() {
                let target_canonical = folder_paths.iter().find(|other| {
                    if *other == folder { return false; }
                    let other_nexus = other.join(".nexus.json");
                    if !other_nexus.exists() { return false; }
                    let other_name = other.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let norm_other = normalize(&other_name);
                    norm_other == norm_folder
                        || norm_folder.starts_with(&norm_other)
                        || norm_other.starts_with(&norm_folder)
                });

                if let Some(target) = target_canonical {
                    if let Ok(entries) = fs::read_dir(folder) {
                        for e in entries.filter_map(|e| e.ok()) {
                            let src_file = e.path();
                            if src_file.is_file() {
                                let dest_file = target.join(e.file_name());
                                let _ = fs::copy(&src_file, &dest_file);
                                let _ = fs::remove_file(&src_file);
                            }
                        }
                    }
                    let _ = fs::remove_dir_all(folder);
                } else if let Some(m) = installed_mods.iter().find(|m| {
                    normalize(&m.name) == norm_folder || normalize(&m.id) == norm_folder
                }) {
                    if m.nexus_mod_id.is_some() || m.nexus_picture_url.is_some() {
                        let cache_json = serde_json::json!({
                            "modId": m.nexus_mod_id,
                            "name": m.name,
                            "author": m.nexus_author.as_deref().unwrap_or(""),
                            "summary": m.nexus_summary.as_deref().unwrap_or(""),
                            "description": m.nexus_description.as_deref().unwrap_or(""),
                            "version": m.version,
                            "pictureUrl": m.nexus_picture_url.as_deref().unwrap_or(""),
                        });
                        let _ = fs::write(folder.join(".nexus.json"), serde_json::to_string_pretty(&cache_json).unwrap_or_default());
                    }
                }
            }
        }
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&lib_dir).map_err(|e| format!("Cannot read library dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        if !entry.file_type().map_or(false, |t| t.is_dir()) {
            continue;
        }
        let mod_id = entry.file_name().to_string_lossy().to_string();

        let mut folder_nexus_picture_url = None;
        let mut folder_nexus_name = None;
        let mut folder_nexus_author = None;
        let mut folder_nexus_summary = None;
        let mut folder_nexus_mod_id = None;
        let mut folder_nexus_version = None;

        let nexus_json_path = entry.path().join(".nexus.json");
        if nexus_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&nexus_json_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    folder_nexus_picture_url = val.get("pictureUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
                    folder_nexus_name = val.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                    folder_nexus_author = val.get("author").and_then(|v| v.as_str()).map(|s| s.to_string());
                    folder_nexus_summary = val.get("summary").and_then(|v| v.as_str()).map(|s| s.to_string());
                    folder_nexus_mod_id = val.get("modId").and_then(|v| v.as_u64()).map(|v| v as u32);
                    folder_nexus_version = val.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
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
                    if zip_name == ".nexus.json" || zip_name.ends_with(".pmm.json") {
                        continue;
                    }

                    let zip_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    let now = chrono::Utc::now().to_rfc3339();

                    // Check for individual .pmm.json sidecar next to this zip
                    let mut author = folder_nexus_author.clone();
                    let mut description = folder_nexus_summary.clone();
                    let mut version = folder_nexus_version.clone();
                    let mut mod_type = None;
                    let mut nexus_picture_url = folder_nexus_picture_url.clone();
                    let mut nexus_name = folder_nexus_name.clone();
                    let mut nexus_mod_id = folder_nexus_mod_id;
                    let nexus_version = folder_nexus_version.clone();

                    let pmm_sidecar = PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()));
                    let pmm_sidecar_alt = path.with_extension("pmm.json");
                    let pmm_path_to_read = if pmm_sidecar.exists() {
                        Some(pmm_sidecar)
                    } else if pmm_sidecar_alt.exists() {
                        Some(pmm_sidecar_alt)
                    } else {
                        None
                    };

                    if let Some(ref pmm_file) = pmm_path_to_read {
                        if let Ok(content) = fs::read_to_string(pmm_file) {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(a) = val.get("nexusAuthor").or_else(|| val.get("author")).and_then(|v| v.as_str()) {
                                    author = Some(a.to_string());
                                }
                                if let Some(d) = val.get("nexusSummary").or_else(|| val.get("nexusDescription")).or_else(|| val.get("description")).and_then(|v| v.as_str()) {
                                    description = Some(d.to_string());
                                }
                                if let Some(v) = val.get("version").and_then(|v| v.as_str()) {
                                    version = Some(v.to_string());
                                }
                                if let Some(t) = val.get("type").or_else(|| val.get("modType")).and_then(|v| v.as_str()) {
                                    mod_type = Some(t.to_string());
                                }
                                if let Some(p) = val.get("nexusPictureUrl").and_then(|v| v.as_str()) {
                                    nexus_picture_url = Some(p.to_string());
                                }
                                if let Some(n) = val.get("name").and_then(|v| v.as_str()) {
                                    nexus_name = Some(n.to_string());
                                }
                                if let Some(id) = val.get("nexusModId").and_then(|v| v.as_u64()) {
                                    nexus_mod_id = Some(id as u32);
                                }
                            }
                        }
                    } else {
                        // Extract version from filename if not in sidecar
                        let parsed = crate::nexus::parse_mod_filename(&zip_name);
                        if let Some(v) = parsed.version {
                            version = Some(v);
                        }

                        // Auto-generate .pmm.json sidecar
                        let pmm_data = serde_json::json!({
                            "name": nexus_name.as_deref().unwrap_or(&mod_id),
                            "nexusModId": nexus_mod_id,
                            "nexusAuthor": author.as_deref().unwrap_or(""),
                            "nexusSummary": description.as_deref().unwrap_or(""),
                            "nexusPictureUrl": nexus_picture_url.as_deref().unwrap_or(""),
                            "version": version.as_deref().unwrap_or(""),
                            "zipName": zip_name,
                        });
                        let sidecar_dest = PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()));
                        let _ = fs::write(&sidecar_dest, serde_json::to_string_pretty(&pmm_data).unwrap_or_default());
                    }

                    // Check live installation status against active installed mods
                    let norm_mod_id = normalize(&mod_id);
                    let norm_nexus_name = nexus_name.as_deref().map(normalize);
                    let matched_installed = installed_mods.iter().find(|m| {
                        let norm_m_name = normalize(&m.name);
                        let norm_m_id = normalize(&m.id);
                        norm_m_name == norm_mod_id
                            || norm_m_id == norm_mod_id
                            || norm_nexus_name.as_ref().map(|n| n == &norm_m_name || n == &norm_m_id).unwrap_or(false)
                            || (nexus_mod_id.is_some() && m.nexus_mod_id.is_some() && m.nexus_mod_id == nexus_mod_id)
                    });

                    let is_installed = matched_installed.is_some();
                    let installed_version = matched_installed.map(|m| m.version.clone());
                    if mod_type.is_none() {
                        if let Some(m) = matched_installed {
                            mod_type = Some(format!("{:?}", m.mod_type).to_lowercase());
                        }
                    }

                    entries.push(LibraryEntry {
                        mod_id: mod_id.clone(),
                        zip_name,
                        zip_size,
                        installed_at: now,
                        nexus_picture_url,
                        nexus_name,
                        nexus_author: author.clone(),
                        nexus_summary: description.clone(),
                        nexus_mod_id,
                        nexus_version,
                        author,
                        description,
                        version,
                        mod_type,
                        is_installed,
                        installed_version,
                    });
                }
            }
        }
    }

    Ok(entries)
}
