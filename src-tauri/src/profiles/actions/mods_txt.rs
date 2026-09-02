use std::fs;
use std::path::Path;

pub fn update_mods_txt_load_order(mods_txt: &Path, mod_name: &str, enabled: bool) -> Result<(), String> {
    let content = fs::read_to_string(mods_txt).map_err(|e| e.to_string())?;
    let target_val = if enabled { "1" } else { "0" };
    let mod_name_lower = mod_name.to_lowercase();

    let mut found = false;
    let mut lines_to_process: Vec<String> = Vec::new();

    for line in content.lines() {
        let line_clean = line.trim();
        if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
            let matches = if let Some(pos) = line_clean.find(':') {
                let name = line_clean[..pos].trim();
                name.to_lowercase() == mod_name_lower
            } else {
                line_clean.to_lowercase() == mod_name_lower
            };

            if matches {
                found = true;
                lines_to_process.push(format!("{} : {}", mod_name, target_val));
                continue;
            }
        }
        lines_to_process.push(line.to_string());
    }

    if !found {
        let mut insert_index = None;
        for (idx, line) in lines_to_process.iter().enumerate() {
            let line_clean = line.trim();
            if line_clean.contains("BPModLoaderMod") {
                insert_index = Some(idx + 1);
            }
        }

        if insert_index.is_none() {
            for (idx, line) in lines_to_process.iter().enumerate() {
                let line_clean = line.trim();
                if line_clean.contains("; Built-in keybinds") {
                    insert_index = Some(idx);
                }
            }
        }

        let final_idx = insert_index.unwrap_or(lines_to_process.len());
        let new_entry = format!("{} : {}", mod_name, target_val);
        lines_to_process.insert(final_idx, new_entry);
    }

    fs::write(mods_txt, lines_to_process.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_from_mods_txt(mods_txt: &Path, mod_name: &str) -> Result<(), String> {
    let content = fs::read_to_string(mods_txt).map_err(|e| e.to_string())?;
    let mut new_lines = Vec::new();
    let mut changed = false;

    for line in content.lines() {
        let line_clean = line.trim();
        if !line_clean.starts_with(';') && !line_clean.starts_with("//") {
            if let Some(pos) = line_clean.find(':') {
                let name = line_clean[..pos].trim();
                if name.to_lowercase() == mod_name.to_lowercase() {
                    changed = true;
                    continue;
                }
            } else if line_clean.to_lowercase() == mod_name.to_lowercase() {
                changed = true;
                continue;
            }
        }
        new_lines.push(line.to_string());
    }

    if changed {
        fs::write(mods_txt, new_lines.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}
