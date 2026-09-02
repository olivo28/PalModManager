use std::path::Path;
use crate::models::ModInfo;

pub fn get_mod_folder_name(mod_info: &ModInfo) -> String {
    if !mod_info.game_path.is_empty() {
        if let Some(name) = Path::new(&mod_info.game_path).file_name() {
            let mut name_str = name.to_string_lossy().to_string();
            if let Some(stripped) = name_str.strip_suffix(".disabled") {
                name_str = stripped.to_string();
            }
            if name_str.len() > 4 && name_str[..3].chars().all(|c| c.is_ascii_digit()) && name_str.as_bytes()[3] == b'_' {
                name_str = name_str[4..].to_string();
            }
            return name_str;
        }
    }
    if !mod_info.disabled_path.is_empty() {
        if let Some(name) = Path::new(&mod_info.disabled_path).file_name() {
            let mut name_str = name.to_string_lossy().to_string();
            if let Some(stripped) = name_str.strip_suffix(".disabled") {
                name_str = stripped.to_string();
            }
            if name_str.len() > 4 && name_str[..3].chars().all(|c| c.is_ascii_digit()) && name_str.as_bytes()[3] == b'_' {
                name_str = name_str[4..].to_string();
            }
            return name_str;
        }
    }
    let mut clean_name = mod_info.name.clone();
    if clean_name.len() > 4 && clean_name[..3].chars().all(|c| c.is_ascii_digit()) && clean_name.as_bytes()[3] == b'_' {
        clean_name = clean_name[4..].to_string();
    }
    clean_name
}
