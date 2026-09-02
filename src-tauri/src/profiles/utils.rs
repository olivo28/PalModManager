use std::fs;
use std::path::{Path, PathBuf};
use crate::models::ModInfo;

/// Creates an NTFS Junction on Windows (zero-admin required) or a symlink on Unix
pub fn create_junction_or_symlink(target: &Path, link: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        // junction::create creates an NTFS junction
        junction::create(target, link).map_err(|e| format!("Failed to create junction from {:?} to {:?}: {}", target, link, e))
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(|e| format!("Failed to create symlink: {}", e))
    }
}

/// Safely removes a junction or directory link without deleting the contents of the target folder
pub fn remove_junction_or_symlink(link: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        if junction::exists(link).unwrap_or(false) {
            if junction::delete(link).is_err() {
                // Fallback to remove_dir if delete fails
                let _ = fs::remove_dir(link);
            }
            return Ok(());
        }
    }

    if let Ok(meta) = fs::symlink_metadata(link) {
        if meta.is_dir() {
            fs::remove_dir(link).map_err(|e| format!("Failed to remove directory link: {}", e))
        } else {
            fs::remove_file(link).map_err(|e| format!("Failed to remove file link: {}", e))
        }
    } else {
        Ok(())
    }
}

pub fn sanitize_profile_id(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let trimmed = clean.trim_matches('_');
    if trimmed.is_empty() {
        "profile".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn get_profile_dir(program_path: &str, profile_id: &str) -> PathBuf {
    PathBuf::from(program_path).join("profiles").join(profile_id)
}

pub fn ensure_profile_structure(program_path: &str, profile_id: &str) -> PathBuf {
    let p_dir = get_profile_dir(program_path, profile_id);
    let _ = fs::create_dir_all(p_dir.join("ue4ss"));
    let _ = fs::create_dir_all(p_dir.join("ue4ss_mods"));
    let _ = fs::create_dir_all(p_dir.join("ue4ss_workshop_mods"));
    let _ = fs::create_dir_all(p_dir.join("palschema"));
    let _ = fs::create_dir_all(p_dir.join("paks"));
    let _ = fs::create_dir_all(p_dir.join("logicmods"));
    p_dir
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Cannot create dest dir: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Cannot read source dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();
        let dest_path = dst.join(file_name);
        
        let is_symlink = path.is_symlink();
        let is_junction = if path.is_dir() {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if let Ok(meta) = fs::symlink_metadata(&path) {
                    (meta.file_attributes() & 0x400) != 0 // FILE_ATTRIBUTE_REPARSE_POINT
                } else {
                    false
                }
            }
            #[cfg(not(windows))]
            false
        } else {
            false
        };

        if is_symlink || is_junction {
            // Re-create the symlink or junction pointing to the same target instead of copying the content
            if let Ok(target) = fs::read_link(&path) {
                let is_dir_link = fs::symlink_metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
                if is_dir_link {
                    let _ = create_junction_or_symlink(&target, &dest_path);
                } else {
                    #[cfg(windows)]
                    {
                        let _ = std::os::windows::fs::symlink_file(&target, &dest_path);
                    }
                    #[cfg(not(windows))]
                    {
                        let _ = std::os::unix::fs::symlink(&target, &dest_path);
                    }
                }
            }
        } else if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            if dest_path.exists() {
                let _ = fs::remove_file(&dest_path);
            }
            if fs::hard_link(&path, &dest_path).is_err() {
                fs::copy(&path, &dest_path).map_err(|e| {
                    format!("Cannot copy file {}: {}", file_name.to_string_lossy(), e)
                })?;
            }
        }
    }
    Ok(())
}

pub fn move_path(src: &Path, dst: &Path) -> Result<(), String> {
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    if src.is_dir() {
        copy_dir_all(src, dst)?;
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

pub fn consolidate_mod_folder_metadata(folder: &Path) -> Option<crate::models::PmmMetadata> {
    if !folder.is_dir() {
        return None;
    }

    let modinfo_path = folder.join("modinfo.pmm.json");
    let legacy_dot_path = folder.join(".pmm.json");

    let mut meta = crate::models::PmmMetadata::default();
    let mut found = false;
    let mut needs_write = false;

    if modinfo_path.exists() {
        if let Ok(content) = fs::read_to_string(&modinfo_path) {
            if let Ok(parsed) = serde_json::from_str::<crate::models::PmmMetadata>(&content) {
                meta = parsed;
                found = true;
            }
        }
    }

    if legacy_dot_path.exists() {
        if let Ok(content) = fs::read_to_string(&legacy_dot_path) {
            if let Ok(legacy_parsed) = serde_json::from_str::<crate::models::PmmMetadata>(&content) {
                if !found {
                    meta = legacy_parsed;
                    found = true;
                } else {
                    if meta.name.is_empty() && !legacy_parsed.name.is_empty() { meta.name = legacy_parsed.name; }
                    if meta.version.is_empty() && !legacy_parsed.version.is_empty() { meta.version = legacy_parsed.version; }
                    if meta.author.is_none() && legacy_parsed.author.is_some() { meta.author = legacy_parsed.author; }
                    if meta.description.is_none() && legacy_parsed.description.is_some() { meta.description = legacy_parsed.description; }
                    if meta.nexus_picture_url.is_none() && legacy_parsed.nexus_picture_url.is_some() { meta.nexus_picture_url = legacy_parsed.nexus_picture_url; }
                    if meta.nexus_url.is_none() && legacy_parsed.nexus_url.is_some() { meta.nexus_url = legacy_parsed.nexus_url; }
                    if meta.nexus_mod_id.is_none() && legacy_parsed.nexus_mod_id.is_some() { meta.nexus_mod_id = legacy_parsed.nexus_mod_id; }
                    if meta.custom_notes.is_none() && legacy_parsed.custom_notes.is_some() { meta.custom_notes = legacy_parsed.custom_notes; }
                    if meta.category.is_none() && legacy_parsed.category.is_some() { meta.category = legacy_parsed.category; }
                    if meta.routes.is_none() && legacy_parsed.routes.is_some() { meta.routes = legacy_parsed.routes; }
                    if meta.installed_files.is_none() && legacy_parsed.installed_files.is_some() { meta.installed_files = legacy_parsed.installed_files; }
                }
            }
        }
        // Safely remove redundant legacy .pmm.json
        let _ = fs::remove_file(&legacy_dot_path);
        needs_write = true;
    }

    // Populate installed_files if empty or None
    if meta.installed_files.as_ref().map_or(true, |f| f.is_empty()) {
        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(rel) = entry.path().strip_prefix(folder) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    if rel_str != "modinfo.pmm.json" && rel_str != ".pmm.json" {
                        files.push(rel_str);
                    }
                }
            }
        }
        if !files.is_empty() {
            meta.installed_files = Some(files);
            needs_write = true;
        }
    }

    if found {
        if needs_write || !modinfo_path.exists() {
            if let Ok(json) = serde_json::to_string_pretty(&meta) {
                let _ = fs::write(&modinfo_path, json);
            }
        }
        Some(meta)
    } else {
        None
    }
}

fn save_pmm_meta_path(m: &ModInfo, path_str: &str) -> Result<(), String> {
    if path_str.is_empty() {
        return Ok(());
    }
    let path = Path::new(path_str);
    if !path.exists() {
        return Ok(());
    }

    let installed_files = if !m.extra_files.is_empty() {
        Some(m.extra_files.clone())
    } else if path.is_dir() {
        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(rel) = entry.path().strip_prefix(path) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    if rel_str != "modinfo.pmm.json" && rel_str != ".pmm.json" {
                        files.push(rel_str);
                    }
                }
            }
        }
        if !files.is_empty() { Some(files) } else { None }
    } else {
        None
    };

    let meta = crate::models::PmmMetadata {
        name: m.name.clone(),
        version: m.version.clone(),
        author: m.nexus_author.clone(),
        description: m.nexus_summary.clone(),
        mod_type: Some(format!("{:?}", m.mod_type).to_lowercase()),
        nexus_mod_id: m.nexus_mod_id,
        nexus_file_id: m.nexus_file_id,
        nexus_picture_url: m.nexus_picture_url.clone(),
        nexus_url: m.nexus_url.clone(),
        custom_notes: m.custom_notes.clone(),
        category: m.nexus_category.clone(),
        routes: None,
        installed_files,
    };

    let pmm_path = if path.is_file() {
        PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()))
    } else {
        let legacy_file = path.join(".pmm.json");
        if legacy_file.exists() {
            let _ = fs::remove_file(&legacy_file);
        }
        path.join("modinfo.pmm.json")
    };

    if let Ok(json) = serde_json::to_string_pretty(&meta) {
        let _ = fs::write(&pmm_path, json);
    }
    Ok(())
}

pub fn save_pmm_meta(m: &ModInfo) -> Result<(), String> {
    let primary_path = if !m.game_path.is_empty() { &m.game_path } else { &m.disabled_path };
    let _ = save_pmm_meta_path(m, primary_path);

    for extra in &m.extra_files {
        if extra.to_lowercase().ends_with(".pak") {
            let _ = save_pmm_meta_path(m, extra);
        }
    }
    Ok(())
}

pub fn find_extracted_root(src: &Path) -> PathBuf {
    if src.is_dir() {
        let entries: Vec<_> = fs::read_dir(src)
            .ok()
            .into_iter()
            .flat_map(|rd| rd.filter_map(|e| e.ok()))
            .filter(|e| {
                let n = e.file_name();
                n != ".." && n != "." && n != "__MACOSX"
            })
            .collect();
        if entries.len() == 1 && entries[0].file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            return entries[0].path();
        }
    }
    src.to_path_buf()
}
