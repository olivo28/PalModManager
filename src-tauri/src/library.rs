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

fn sanitize_filename(name: &str) -> String {
    name.chars().map(|c| match c {
        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
        _ => c,
    }).collect()
}

fn infer_mod_type_from_name(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    if lower.contains("palschema") {
        Some("palschema".to_string())
    } else if lower.contains("altermatic") {
        Some("altermatic".to_string())
    } else if lower.contains("ue4ss") || lower.contains("lua") {
        Some("ue4ss".to_string())
    } else if lower.contains("logicmods") {
        Some("logicmods".to_string())
    } else if lower.contains("pak") {
        Some("pak".to_string())
    } else {
        None
    }
}

pub fn copy_to_library(
    source_zip: &str,
    program_path: &str,
    mod_id: &str,
    target_filename: Option<&str>,
    mod_version: Option<&str>,
) -> Result<LibraryEntry, String> {
    let safe_folder_name = sanitize_filename(mod_id);
    let lib_path = get_library_path(program_path, &safe_folder_name);
    fs::create_dir_all(&lib_path).map_err(|e| format!("Cannot create library dir: {}", e))?;

    let zip_path = Path::new(source_zip);
    let original_name = zip_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown.zip".to_string());

    let is_temp_or_nexus = |name: &str| -> bool {
        name.starts_with("nexus_") || name.starts_with("disc_") || name.starts_with("temp_") || name == "unknown.zip" || name.starts_with("download_")
    };

    let clean_version = mod_version
        .map(|v| v.trim())
        .filter(|v| !v.is_empty() && *v != "unknown");

    let orig_ext = zip_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_else(|| "zip".to_string());
    let clean_ext = if orig_ext == "rar" || orig_ext == "7z" || orig_ext == "pak" { orig_ext } else { "zip".to_string() };

    let zip_name = if let Some(target) = target_filename {
        let sanitized_target = sanitize_filename(target);
        if is_temp_or_nexus(&sanitized_target) {
            if let Some(v) = clean_version {
                format!("{} - {}.{}", safe_folder_name, sanitize_filename(v), clean_ext)
            } else {
                format!("{}.{}", safe_folder_name, clean_ext)
            }
        } else {
            sanitized_target
        }
    } else if is_temp_or_nexus(&original_name) {
        if let Some(v) = clean_version {
            format!("{} - {}.{}", safe_folder_name, sanitize_filename(v), clean_ext)
        } else {
            format!("{}.{}", safe_folder_name, clean_ext)
        }
    } else {
        sanitize_filename(&original_name)
    };
    
    let dest = lib_path.join(&zip_name);

    fs::copy(source_zip, &dest).map_err(|e| format!("Cannot copy to library: {}", e))?;

    // Clean up any obsolete temporary nexus_*.zip in this library folder
    if let Ok(entries) = fs::read_dir(&lib_path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
                if fname.starts_with("nexus_") && p != dest {
                    let _ = fs::remove_file(&p);
                    let sidecar = PathBuf::from(format!("{}.pmm.json", p.to_string_lossy()));
                    if sidecar.exists() {
                        let _ = fs::remove_file(sidecar);
                    }
                }
            }
        }
    }

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

            // Check if another folder exists sharing the same Nexus ID or normalized name
            let folder_nexus_id = if nexus_path.exists() {
                fs::read_to_string(&nexus_path)
                    .ok()
                    .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                    .and_then(|v| v.get("modId").and_then(|m| m.as_u64()))
                    .map(|id| id as u32)
            } else {
                None
            };

            let target_canonical = folder_paths.iter().find(|other| {
                if *other == folder { return false; }
                let other_nexus = other.join(".nexus.json");
                if !other_nexus.exists() { return false; }
                let other_name = other.file_name().unwrap_or_default().to_string_lossy().to_string();
                let norm_other = normalize(&other_name);

                // Only merge pure duplicates with matching normalized names (e.g. "Elemental Coatings" vs "ElementalCoatings").
                // Never merge distinct options or variants that share the same Nexus mod page.
                if norm_other == norm_folder {
                    if let Some(cur_id) = folder_nexus_id {
                        if let Ok(c) = fs::read_to_string(&other_nexus) {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&c) {
                                if let Some(oid) = v.get("modId").and_then(|m| m.as_u64()).map(|n| n as u32) {
                                    if oid == cur_id {
                                        return folder_name.contains(' ') <= other_name.contains(' ');
                                    }
                                }
                            }
                        }
                    }
                    if !nexus_path.exists() {
                        return true;
                    }
                }

                false
            });

            if let Some(target) = target_canonical {
                if let Ok(entries) = fs::read_dir(folder) {
                    for e in entries.filter_map(|e| e.ok()) {
                        let src_file = e.path();
                        if src_file.is_file() {
                            let dest_file = target.join(e.file_name());
                            if !dest_file.exists() {
                                let _ = fs::copy(&src_file, &dest_file);
                            }
                            let _ = fs::remove_file(&src_file);
                        }
                    }
                }
                let _ = fs::remove_dir_all(folder);
            } else if !nexus_path.exists() {
                if let Some(m) = installed_mods.iter().find(|m| {
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
                    folder_nexus_picture_url = val.get("pictureUrl").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                    folder_nexus_name = val.get("name").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                    folder_nexus_author = val.get("author").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                    folder_nexus_summary = val.get("summary").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                    folder_nexus_mod_id = val.get("modId").and_then(|v| v.as_u64()).map(|v| v as u32);
                    folder_nexus_version = val.get("version").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                }
            }
        }

        // Auto-heal temporary nexus_*.zip archives and eliminate duplicates
        if let Ok(files_read) = fs::read_dir(entry.path()) {
            let files: Vec<PathBuf> = files_read
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect();

            let nexus_zips: Vec<PathBuf> = files.iter()
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("nexus_") && !n.ends_with(".pmm.json"))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            let clean_zips: Vec<PathBuf> = files.iter()
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| !n.starts_with("nexus_") && n != ".nexus.json" && !n.ends_with(".pmm.json"))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            if !clean_zips.is_empty() {
                for nz in nexus_zips {
                    let _ = fs::remove_file(&nz);
                    let sc = PathBuf::from(format!("{}.pmm.json", nz.to_string_lossy()));
                    if sc.exists() { let _ = fs::remove_file(sc); }
                }
            } else if !nexus_zips.is_empty() {
                let mut sorted = nexus_zips;
                sorted.sort_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0));
                if let Some(best_zip) = sorted.pop() {
                    let canonical_name = format!("{}.zip", mod_id);
                    let target_dest = entry.path().join(&canonical_name);
                    let _ = fs::rename(&best_zip, &target_dest);
                    let old_sidecar = PathBuf::from(format!("{}.pmm.json", best_zip.to_string_lossy()));
                    let new_sidecar = PathBuf::from(format!("{}.pmm.json", target_dest.to_string_lossy()));
                    if old_sidecar.exists() {
                        let _ = fs::rename(old_sidecar, new_sidecar);
                    }
                    for dup in sorted {
                        let _ = fs::remove_file(&dup);
                        let sc = PathBuf::from(format!("{}.pmm.json", dup.to_string_lossy()));
                        if sc.exists() { let _ = fs::remove_file(sc); }
                    }
                }
            }
        }

        // Collect raw archive files in directory
        let mut raw_files: Vec<(PathBuf, String, u64)> = Vec::new();
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
                    raw_files.push((path, zip_name, zip_size));
                }
            }
        }

        struct SiblingItem {
            path: PathBuf,
            zip_name: String,
            zip_size: u64,
            author: Option<String>,
            description: Option<String>,
            version: Option<String>,
            mod_type: Option<String>,
            nexus_picture_url: Option<String>,
            nexus_name: Option<String>,
            nexus_mod_id: Option<u32>,
        }

        let mut sibling_items: Vec<SiblingItem> = Vec::new();

        for (path, zip_name, zip_size) in raw_files {
            let mut author = folder_nexus_author.clone();
            let mut description = folder_nexus_summary.clone();
            let mut version = folder_nexus_version.clone();
            let mut mod_type = None;
            let mut nexus_picture_url = folder_nexus_picture_url.clone();
            let mut nexus_name = folder_nexus_name.clone();
            let mut nexus_mod_id = folder_nexus_mod_id;

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
                    // Try typed PmmMetadata first with empty string guard
                    if let Ok(meta) = serde_json::from_str::<crate::models::PmmMetadata>(&content) {
                        if let Some(a) = meta.author.filter(|s| !s.trim().is_empty()) { author = Some(a); }
                        if let Some(d) = meta.description.filter(|s| !s.trim().is_empty()) { description = Some(d); }
                        if !meta.version.trim().is_empty() { version = Some(meta.version); }
                        if let Some(t) = meta.mod_type.filter(|s| !s.trim().is_empty()) { mod_type = Some(t.to_lowercase()); }
                        if let Some(p) = meta.nexus_picture_url.filter(|s| !s.trim().is_empty()) { nexus_picture_url = Some(p); }
                        if !meta.name.trim().is_empty() { nexus_name = Some(meta.name); }
                        if let Some(id) = meta.nexus_mod_id { nexus_mod_id = Some(id); }
                    }
                    // Robust Value fallback for legacy ModInfo dumps or alternate key names
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if author.is_none() {
                            author = val.get("author").or_else(|| val.get("nexusAuthor")).and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                        }
                        if description.is_none() {
                            description = val.get("description").or_else(|| val.get("nexusSummary")).or_else(|| val.get("nexusDescription")).and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                        }
                        if version.is_none() {
                            version = val.get("version").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                        }
                        if mod_type.is_none() {
                            mod_type = val.get("type").or_else(|| val.get("modType")).or_else(|| val.get("mod_type")).and_then(|v| v.as_str()).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
                        }
                        if nexus_picture_url.is_none() {
                            nexus_picture_url = val.get("nexusPictureUrl").or_else(|| val.get("pictureUrl")).and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                        }
                        if nexus_name.is_none() {
                            nexus_name = val.get("name").or_else(|| val.get("nexusName")).and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                        }
                        if nexus_mod_id.is_none() {
                            nexus_mod_id = val.get("nexusModId").or_else(|| val.get("modId")).and_then(|v| v.as_u64()).map(|n| n as u32);
                        }
                    }
                }
            }

            // Extract version from filename if not in sidecar
            if version.is_none() {
                let parsed = crate::nexus::parse_mod_filename(&zip_name);
                if let Some(v) = parsed.version {
                    version = Some(v);
                }
            }

            sibling_items.push(SiblingItem {
                path,
                zip_name,
                zip_size,
                author,
                description,
                version,
                mod_type,
                nexus_picture_url,
                nexus_name,
                nexus_mod_id,
            });
        }

        // Determine best shared metadata across sibling versions in the folder
        let best_author = folder_nexus_author.clone()
            .or_else(|| sibling_items.iter().find_map(|item| item.author.clone()));
        let best_description = folder_nexus_summary.clone()
            .or_else(|| sibling_items.iter().find_map(|item| item.description.clone()));
        let best_picture_url = folder_nexus_picture_url.clone()
            .or_else(|| sibling_items.iter().find_map(|item| item.nexus_picture_url.clone()));
        let best_nexus_id = folder_nexus_mod_id
            .or_else(|| sibling_items.iter().find_map(|item| item.nexus_mod_id));
        let best_nexus_name = folder_nexus_name.clone()
            .or_else(|| sibling_items.iter().find_map(|item| item.nexus_name.clone()));

        // Sibling mod_type resolution with fallback to installed mods and name inference
        let mut best_mod_type = sibling_items.iter().find_map(|item| item.mod_type.clone());
        if best_mod_type.is_none() {
            let norm_folder = normalize(&mod_id);
            let matched = installed_mods.iter().find(|m| {
                (best_nexus_id.is_some() && m.nexus_mod_id.is_some() && m.nexus_mod_id == best_nexus_id)
                    || normalize(&m.name) == norm_folder
                    || normalize(&m.id) == norm_folder
            });
            if let Some(m) = matched {
                best_mod_type = Some(format!("{:?}", m.mod_type).to_lowercase());
            }
        }
        if best_mod_type.is_none() {
            let name_to_check = best_nexus_name.as_deref().unwrap_or(&mod_id);
            best_mod_type = infer_mod_type_from_name(name_to_check);
        }

        let canonical_mod_id = best_nexus_name.as_deref().unwrap_or(&mod_id);
        let norm_canonical = normalize(canonical_mod_id);
        let now = chrono::Utc::now().to_rfc3339();

        for mut item in sibling_items {
            // Propagate best metadata to older / incomplete versions
            if item.author.is_none() { item.author = best_author.clone(); }
            if item.description.is_none() { item.description = best_description.clone(); }
            if item.nexus_picture_url.is_none() { item.nexus_picture_url = best_picture_url.clone(); }
            if item.nexus_mod_id.is_none() { item.nexus_mod_id = best_nexus_id; }
            if item.nexus_name.is_none() { item.nexus_name = best_nexus_name.clone(); }
            if item.mod_type.is_none() { item.mod_type = best_mod_type.clone(); }

            // Auto-heal sidecar on disk using standard PmmMetadata
            let pmm_data = crate::models::PmmMetadata {
                name: item.nexus_name.as_deref().unwrap_or(&mod_id).to_string(),
                version: item.version.clone().unwrap_or_default(),
                author: item.author.clone(),
                description: item.description.clone(),
                mod_type: item.mod_type.clone(),
                nexus_mod_id: item.nexus_mod_id,
                nexus_file_id: None,
                nexus_picture_url: item.nexus_picture_url.clone(),
                nexus_url: None,
                custom_notes: None,
                category: None,
                routes: None,
                original_name: None,
                custom_name: None,
                folder_name: Some(mod_id.clone()),
                installed_folders: None,
                source_zip: Some(item.zip_name.clone()),
                installed_files: None,
                extra_files: None,
                fomod_choices: None,
            };
            let sidecar_dest = PathBuf::from(format!("{}.pmm.json", item.path.to_string_lossy()));
            if let Ok(json) = serde_json::to_string_pretty(&pmm_data) {
                let _ = fs::write(&sidecar_dest, json);
            }

            // Check live installation status against active installed mods
            let matched_installed = installed_mods.iter().find(|m| {
                let norm_m_name = normalize(&m.name);
                let norm_m_id = normalize(&m.id);
                norm_m_name == norm_canonical
                    || norm_m_id == norm_canonical
                    || (item.nexus_mod_id.is_some() && m.nexus_mod_id.is_some() && m.nexus_mod_id == item.nexus_mod_id)
            });

            let is_installed = matched_installed.is_some();
            let installed_version = matched_installed.map(|m| m.version.clone());
            if item.mod_type.is_none() {
                if let Some(m) = matched_installed {
                    item.mod_type = Some(format!("{:?}", m.mod_type).to_lowercase());
                }
            }

            entries.push(LibraryEntry {
                mod_id: canonical_mod_id.to_string(),
                zip_name: item.zip_name,
                zip_size: item.zip_size,
                installed_at: now.clone(),
                nexus_picture_url: item.nexus_picture_url,
                nexus_name: item.nexus_name,
                nexus_author: item.author.clone(),
                nexus_summary: item.description.clone(),
                nexus_mod_id: item.nexus_mod_id,
                nexus_version: folder_nexus_version.clone(),
                author: item.author,
                description: item.description,
                version: item.version,
                mod_type: item.mod_type,
                is_installed,
                installed_version,
            });
        }
    }

    Ok(entries)
}


