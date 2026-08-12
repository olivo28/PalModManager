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
    if !link.exists() {
        return Ok(());
    }
    // On Windows, removing a junction can be done safely by removing the directory link entry itself
    // fs::remove_dir works on directory junctions/symlinks without deleting target content.
    fs::remove_dir(link).map_err(|e| format!("Failed to remove junction: {}", e))
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
                if path.is_dir() {
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
            fs::copy(&path, &dest_path).map_err(|e| {
                format!("Cannot copy file {}: {}", file_name.to_string_lossy(), e)
            })?;
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

fn save_pmm_meta_path(m: &ModInfo, path_str: &str) -> Result<(), String> {
    if path_str.is_empty() {
        return Ok(());
    }
    let path = Path::new(path_str);
    if !path.exists() {
        return Ok(());
    }

    let pmm_path = if path.is_file() {
        PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()))
    } else {
        path.join(".pmm.json")
    };

    if let Ok(json) = serde_json::to_string_pretty(m) {
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
