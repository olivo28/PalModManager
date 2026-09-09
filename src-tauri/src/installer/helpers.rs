use std::fs;
use std::path::Path;

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
    
    // Find index of the last purely numeric token (excluding years)
    let mut id_index = None;
    for i in (0..words.len()).rev() {
        let clean_word: String = words[i].chars().filter(|c| c.is_ascii_digit()).collect();
        if !clean_word.is_empty() && words[i].chars().all(|c| c.is_ascii_digit() || c == '(' || c == ')') {
            if let Ok(num) = clean_word.parse::<u32>() {
                if !(num >= 2020 && num <= 2038) {
                    id_index = Some(i);
                    break;
                }
            }
        }
    }

    let clean_words = if let Some(idx) = id_index {
        words[..idx].to_vec()
    } else {
        // Fallback: slice before date-like token
        let mut idx = words.len();
        for i in 0..words.len() {
            let w = words[i];
            let starts_with_digit = w.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
            if starts_with_digit && w.contains('-') && w.len() > 6 {
                idx = i;
                break;
            }
        }
        words[..idx].to_vec()
    };

    let mut clean = Vec::new();
    for word in clean_words {
        let word_clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
        if ["gamepass", "steam", "gdk", "xbox", "singleplayer", "sp"].contains(&word_clean.as_str()) {
            continue;
        }
        clean.push(word);
    }

    let result = clean.join(" ");
    let final_result = result.trim_end_matches(|c: char| c == '-' || c == '_' || c == '(' || c == ' ' || c == ')').trim().to_string();
    let stem_clean = stem.trim_matches(|c: char| c == '(' || c == ')' || c == '[' || c == ']' || c.is_whitespace()).to_lowercase();
    if ["gamepass", "steam", "gdk", "xbox", "singleplayer", "sp"].contains(&stem_clean.as_str()) {
        return "unknown".to_string();
    }
    if final_result.len() < 2 { stem } else { final_result }
}

pub fn normalize_path_separator(p: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        let mut s = p.replace('/', "\\");
        while s.contains("\\\\") {
            s = s.replace("\\\\", "\\");
        }
        if s.ends_with('\\') && s.len() > 3 {
            s.pop();
        }
        s
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut s = p.replace('\\', "/");
        while s.contains("//") {
            s = s.replace("//", "/");
        }
        if s.ends_with('/') && s.len() > 1 {
            s.pop();
        }
        s
    }
}

pub fn get_ue4ss_component_root(dest_path: &str) -> Option<String> {
    let path = Path::new(dest_path);
    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("mods".to_string()) {
            if let Some(pparent) = parent.parent() {
                if pparent.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("ue4ss".to_string()) {
                    return Some(normalize_path_separator(&current.to_string_lossy()));
                }
            }
        }
        current = parent;
    }
    None
}

pub fn get_palschema_component_root(dest_path: &str) -> Option<String> {
    let path = Path::new(dest_path);
    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("mods".to_string()) {
            if let Some(pparent) = parent.parent() {
                if pparent.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("palschema".to_string()) {
                    return Some(normalize_path_separator(&current.to_string_lossy()));
                }
            }
        }
        current = parent;
    }
    None
}

pub fn copy_folder_contents(src: &Path, dest: &Path) -> Result<(), String> {
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

pub fn detect_config_local(dir: &Path) -> Option<String> {
    for entry in walkdir::WalkDir::new(dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name == "config.json" || name == "config.jsonc" || name == "config.txt" || name == "config.cfg" || name == "settings.json" || name == "settings.txt" {
                return Some(entry.path().to_string_lossy().to_string());
            }
        }
    }
    None
}

pub fn determine_mod_id(
    nexus_mod_id: Option<u32>,
    nexus_file_id: Option<&str>,
    folder_name: &str,
    mod_type: &crate::models::ModType,
) -> String {
    let type_suffix = match mod_type {
        crate::models::ModType::Ue4ss => "ue4ss",
        crate::models::ModType::PalSchema => "palschema",
        crate::models::ModType::Pak => "pak",
        crate::models::ModType::LogicMods => "logicmods",
        crate::models::ModType::Hybrid => "hybrid",
        crate::models::ModType::Altermatic => "altermatic",
    };
    if let Some(nexus_id) = nexus_mod_id {
        if let Some(file_id) = nexus_file_id {
            if !file_id.trim().is_empty() {
                return format!("{}-{}-{}-{}", nexus_id, file_id.trim(), folder_name, type_suffix);
            }
        }
        format!("{}-{}-{}", nexus_id, folder_name, type_suffix)
    } else {
        format!("{}-{}", folder_name, type_suffix)
    }
}

pub fn normalize_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

pub fn get_physical_identity(game_path: &str, disabled_path: &str) -> String {
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

pub fn move_path(src: &Path, dst: &Path) -> Result<(), String> {
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
