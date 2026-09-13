// Mod hotkey scanning and updating for UE4SS Lua mods
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;
use crate::models::ModType;
use super::utils::collect_files_with_extensions;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModHotkey {
    pub mod_id: String,
    pub mod_name: String,
    pub file_path: String,
    pub absolute_file_path: String,
    pub line_number: usize,
    pub keys: String,
    pub raw_line: String,
    pub variable_name: Option<String>,
    pub definition_file_path: Option<String>,
    pub definition_absolute_path: Option<String>,
    pub definition_line_number: Option<usize>,
    pub is_variable: bool,
}

pub fn find_variable_in_lua_files(
    var_name: &str,
    lua_files: &[(PathBuf, String)],
    depth: usize,
) -> Option<(String, PathBuf, usize)> {
    if depth > 5 || var_name.is_empty() {
        return None;
    }

    let clean_var = var_name.trim();

    if clean_var.contains("Key.") || clean_var.contains("ModifierKey.") {
        return None;
    }

    let parts: Vec<&str> = clean_var.split('.').collect();
    let field_name = parts.last().unwrap_or(&clean_var).trim();
    let (target_table, target_index) = if parts.len() > 1 {
        let parent = parts[parts.len() - 2].trim();
        if let Some(bracket_pos) = parent.find('[') {
            let table_base = parent[..bracket_pos].trim();
            let idx_str = parent[bracket_pos + 1..].trim_end_matches(']').trim();
            let idx = idx_str.parse::<usize>().ok();
            (table_base.to_string(), idx)
        } else {
            (parent.to_string(), None)
        }
    } else {
        (String::new(), None)
    };

    for (file_path, content) in lua_files {
        let lines: Vec<&str> = content.lines().collect();
        let mut table_stack: Vec<String> = Vec::new();
        let mut current_array_index: usize = 0;

        for (line_idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("--") {
                continue;
            }

            // Pattern 1: Direct assignment e.g. "Config.OpenMenuKey = Key.F5" or "local OpenMenuKey = Key.F5"
            let is_match = trimmed.starts_with(&format!("{} ", clean_var))
                || trimmed.starts_with(&format!("{}=", clean_var))
                || trimmed.starts_with(&format!("{}\t", clean_var))
                || (!target_table.is_empty() && (
                    trimmed.starts_with(&format!("{}.{} ", target_table, field_name))
                        || trimmed.starts_with(&format!("{}.{}=", target_table, field_name))
                        || trimmed.starts_with(&format!("{}.{}\t", target_table, field_name))
                ))
                || (target_table.is_empty() && (
                    trimmed.starts_with(&format!("local {} ", field_name))
                        || trimmed.starts_with(&format!("local {}=", field_name))
                        || trimmed.starts_with(&format!("local {}\t", field_name))
                        || trimmed.starts_with(&format!("{} ", field_name))
                        || trimmed.starts_with(&format!("{}=", field_name))
                        || trimmed.starts_with(&format!("{}\t", field_name))
                ));

            if is_match {
                if let Some(eq_pos) = trimmed.find('=') {
                    let mut rhs = trimmed[eq_pos + 1..].trim();
                    if let Some(c_pos) = rhs.find("--") {
                        rhs = rhs[..c_pos].trim();
                    }
                    rhs = rhs.trim_end_matches(',').trim_end_matches(';').trim();

                    if !rhs.is_empty() {
                        if rhs.contains("Key.") || rhs.contains("ModifierKey.") || rhs.starts_with('"') || rhs.starts_with('\'') || rhs.starts_with('{') {
                            return Some((rhs.to_string(), file_path.clone(), line_idx + 1));
                        } else if !rhs.contains('(') && !rhs.contains(' ') {
                            if let Some(sub_res) = find_variable_in_lua_files(rhs, lua_files, depth + 1) {
                                return Some(sub_res);
                            }
                        }
                    }
                }
            }

            // Update table stack based on table declaration
            if let Some(open_brace) = trimmed.find('{') {
                let lhs = trimmed[..open_brace].trim();
                let table_name = if let Some(eq_pos) = lhs.rfind('=') {
                    let name_part = lhs[..eq_pos].trim().trim_start_matches("local ").trim();
                    name_part.trim_matches(|c| c == '\'' || c == '"' || c == '[' || c == ']').to_string()
                } else {
                    String::new()
                };
                table_stack.push(table_name);
                current_array_index = 0;
            }

            if trimmed.starts_with('{') {
                current_array_index += 1;
            }

            // Pattern 2: Field inside table constructor e.g. OpenMenuKey = Key.F5 inside Config = { ... }
            let in_target_table = if !target_table.is_empty() {
                table_stack.iter().any(|t| t == &target_table)
                    || trimmed.contains(&format!("{} =", target_table))
                    || trimmed.contains(&format!("{}=", target_table))
                    || trimmed.contains(&format!("\"{}\"", target_table))
                    || trimmed.contains(&format!("'{}'", target_table))
                    || (target_index.is_some() && target_index == Some(current_array_index))
            } else {
                true
            };

            if in_target_table {
                let has_field = trimmed.starts_with(&format!("{} =", field_name))
                    || trimmed.starts_with(&format!("{}=", field_name))
                    || trimmed.starts_with(&format!("{}\t=", field_name))
                    || trimmed.contains(&format!("{} = Key.", field_name))
                    || trimmed.contains(&format!("{} = ModifierKey.", field_name))
                    || trimmed.contains(&format!("{}=Key.", field_name))
                    || trimmed.contains(&format!("{}=ModifierKey.", field_name))
                    || trimmed.contains(&format!("{}: Key.", field_name));

                if has_field {
                    let rhs_str = if let Some(eq_pos) = trimmed.find(&format!("{} =", field_name))
                        .or_else(|| trimmed.find(&format!("{}=", field_name)))
                    {
                        let rest = &trimmed[eq_pos + field_name.len()..];
                        rest.trim_start().trim_start_matches('=').trim()
                    } else if let Some(pos) = trimmed.find("Key.") {
                        &trimmed[pos..]
                    } else if let Some(pos) = trimmed.find("ModifierKey.") {
                        &trimmed[pos..]
                    } else {
                        ""
                    };

                    let mut rhs = rhs_str;
                    if let Some(c_pos) = rhs.find("--") {
                        rhs = rhs[..c_pos].trim();
                    }
                    if let Some(comma_pos) = rhs.find(',') {
                        rhs = rhs[..comma_pos].trim();
                    }
                    rhs = rhs.trim_end_matches(',').trim_end_matches(';').trim_end_matches('}').trim();

                    if !rhs.is_empty() {
                        if rhs.contains("Key.") || rhs.contains("ModifierKey.") || rhs.starts_with('"') || rhs.starts_with('\'') || rhs.starts_with('{') {
                            return Some((rhs.to_string(), file_path.clone(), line_idx + 1));
                        }
                    }
                }
            }

            if trimmed.contains('}') {
                let close_count = trimmed.chars().filter(|&c| c == '}').count();
                for _ in 0..close_count {
                    table_stack.pop();
                }
            }
        }
    }

    None
}

pub fn find_table_array_keys(
    table_name: &str,
    field_name: &str,
    lua_files: &[(PathBuf, String)],
) -> Vec<(String, PathBuf, usize, String)> {
    let mut results = Vec::new();
    let clean_table = table_name.trim();
    let clean_field = field_name.trim();
    let target_table_base = clean_table.split('.').last().unwrap_or(clean_table).trim();

    for (file_path, content) in lua_files {
        let lines: Vec<&str> = content.lines().collect();
        let mut inside_table = false;
        let mut brace_depth: i32 = 0;
        let mut table_start_depth: i32 = 0;
        let mut item_index: usize = 0;

        for (line_idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("--") {
                continue;
            }

            if !inside_table {
                let is_table_start = trimmed.starts_with(&format!("{} =", target_table_base))
                    || trimmed.starts_with(&format!("{}=", target_table_base))
                    || trimmed.starts_with(&format!("local {} =", target_table_base))
                    || trimmed.starts_with(&format!("local {}=", target_table_base))
                    || trimmed.contains(&format!("{}.{} =", clean_table, target_table_base))
                    || trimmed.contains(&format!("{}.{}=", clean_table, target_table_base));

                if is_table_start && trimmed.contains('{') {
                    inside_table = true;
                    table_start_depth = brace_depth;
                }
            }

            let open_braces = trimmed.chars().filter(|&c| c == '{').count() as i32;
            let close_braces = trimmed.chars().filter(|&c| c == '}').count() as i32;

            if inside_table {
                let has_field = trimmed.starts_with(&format!("{} =", clean_field))
                    || trimmed.starts_with(&format!("{}=", clean_field))
                    || trimmed.contains(&format!("{} = Key.", clean_field))
                    || trimmed.contains(&format!("{} = ModifierKey.", clean_field))
                    || trimmed.contains(&format!("{}=Key.", clean_field))
                    || trimmed.contains(&format!("{}=ModifierKey.", clean_field))
                    || trimmed.contains(&format!("{}: Key.", clean_field))
                    || trimmed.contains(&format!("{}:ModifierKey.", clean_field));

                if has_field {
                    item_index += 1;
                    let mut rhs = if let Some(pos) = trimmed.find("Key.") {
                        &trimmed[pos..]
                    } else if let Some(pos) = trimmed.find("ModifierKey.") {
                        &trimmed[pos..]
                    } else if let Some(eq_pos) = trimmed.find('=') {
                        trimmed[eq_pos + 1..].trim()
                    } else {
                        ""
                    };

                    if let Some(c_pos) = rhs.find("--") {
                        rhs = rhs[..c_pos].trim();
                    }
                    if let Some(comma_pos) = rhs.find(',') {
                        rhs = rhs[..comma_pos].trim();
                    }
                    let cleaned_rhs = rhs.trim_end_matches(',').trim_end_matches(';').trim_end_matches('}').trim();

                    if cleaned_rhs.contains("Key.") || cleaned_rhs.contains("ModifierKey.") || cleaned_rhs.starts_with('"') || cleaned_rhs.starts_with('\'') {
                        let label = format!("{}[{}].{}", target_table_base, item_index, clean_field);
                        results.push((cleaned_rhs.to_string(), file_path.clone(), line_idx + 1, label));
                    }
                }
            }

            brace_depth += open_braces - close_braces;
            if inside_table && brace_depth <= table_start_depth && close_braces > 0 {
                inside_table = false;
            }
        }
    }

    results
}

fn extract_keys_from_line(line: &str) -> Option<String> {
    let rkb = "RegisterKeyBind";
    let start = line.find(rkb)?;
    let after = &line[start + rkb.len()..];
    let open_paren = after.find('(')?;
    let content = &after[open_paren + 1..];
    
    let mut parts = Vec::new();
    let mut current_part = String::new();
    let mut paren_depth = 0;
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut quote_char = ' ';
    let mut chars = content.chars().peekable();
    
    while let Some(c) = chars.next() {
        if in_string {
            if c == '\\' {
                current_part.push(c);
                if let Some(next_c) = chars.next() {
                    current_part.push(next_c);
                }
            } else if c == quote_char {
                in_string = false;
                current_part.push(c);
            } else {
                current_part.push(c);
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    quote_char = c;
                    current_part.push(c);
                }
                '(' => {
                    paren_depth += 1;
                    current_part.push(c);
                }
                ')' => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        break;
                    }
                    current_part.push(c);
                }
                '{' | '[' => {
                    brace_depth += 1;
                    current_part.push(c);
                }
                '}' | ']' => {
                    brace_depth -= 1;
                    current_part.push(c);
                }
                ',' => {
                    if paren_depth == 0 && brace_depth == 0 {
                        parts.push(current_part.trim().to_string());
                        current_part = String::new();
                    } else {
                        current_part.push(c);
                    }
                }
                _ => {
                    current_part.push(c);
                }
            }
        }
    }
    
    if !parts.is_empty() {
        let keys = parts.join(", ");
        if !keys.is_empty() {
            return Some(keys);
        }
    } else {
        let single = current_part.trim().to_string();
        if !single.is_empty() {
            return Some(single);
        }
    }
    None
}

fn update_line_keybind(line: &str, new_keys: &str) -> Option<String> {
    let rkb = "RegisterKeyBind";
    let start = line.find(rkb)?;
    let after = &line[start + rkb.len()..];
    let open_paren = after.find('(')?;
    let content = &after[open_paren + 1..];
    
    let mut last_comma_idx = None;
    let mut paren_depth = 0;
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut quote_char = ' ';
    let mut chars = content.char_indices().peekable();
    
    while let Some((i, c)) = chars.next() {
        if in_string {
            if c == '\\' {
                let _ = chars.next();
            } else if c == quote_char {
                in_string = false;
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    quote_char = c;
                }
                '(' => {
                    paren_depth += 1;
                }
                ')' => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        break;
                    }
                }
                '{' | '[' => {
                    brace_depth += 1;
                }
                '}' | ']' => {
                    brace_depth -= 1;
                }
                ',' => {
                    if paren_depth == 0 && brace_depth == 0 {
                        last_comma_idx = Some(i);
                    }
                }
                _ => {}
            }
        }
    }
    
    if let Some(comma_idx) = last_comma_idx {
        let prefix = &line[..start + rkb.len() + open_paren + 1];
        let suffix = &content[comma_idx..];
        return Some(format!("{}{}{}", prefix, new_keys, suffix));
    }
    
    None
}

#[tauri::command]
pub async fn scan_mod_hotkeys(state: State<'_, AppState>) -> Result<Vec<ModHotkey>, String> {
    let profile_mods = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data)
    };

    tauri::async_runtime::spawn_blocking(move || {
        scan_mod_hotkeys_internal(&profile_mods)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn scan_mod_hotkeys_internal(profile_mods: &[crate::models::ModInfo]) -> Result<Vec<ModHotkey>, String> {
    let mut hotkeys = Vec::new();

    for m in profile_mods {
        if !m.enabled {
            continue;
        }
        if m.nexus_author.as_deref() == Some("UE4SS Native Mod") {
            continue;
        }
        if m.mod_type != ModType::Ue4ss && m.mod_type != ModType::Hybrid && m.mod_type != ModType::PalSchema {
            continue;
        }

        let mut paths_to_scan = Vec::new();
        if !m.game_path.is_empty() {
            paths_to_scan.push(PathBuf::from(&m.game_path));
        }
        for extra in &m.extra_files {
            if !extra.is_empty() {
                paths_to_scan.push(PathBuf::from(extra));
            }
        }

        for base_path in paths_to_scan {
            if !base_path.exists() {
                continue;
            }

            let scripts_path = if base_path.join("Scripts").exists() {
                Some(base_path.join("Scripts"))
            } else if base_path.join("scripts").exists() {
                Some(base_path.join("scripts"))
            } else if base_path.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("scripts".to_string()) && base_path.is_dir() {
                Some(base_path.clone())
            } else {
                None
            };

            let Some(scripts_path) = scripts_path else {
                continue;
            };

            let mut files_to_scan = Vec::new();
            collect_files_with_extensions(&scripts_path, &["lua"], &mut files_to_scan);

            // Preload all .lua files and their contents for this mod
            let mut mod_lua_files: Vec<(PathBuf, String)> = Vec::new();
            for file_path in &files_to_scan {
                if let Ok(content) = fs::read_to_string(file_path) {
                    mod_lua_files.push((file_path.clone(), content));
                }
            }

            for (file_path, content) in &mod_lua_files {
                let lines: Vec<&str> = content.lines().collect();
                for (idx, line) in lines.iter().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("--") {
                        continue;
                    }
                    if trimmed.contains("RegisterKeyBind") {
                        if let Some(raw_keys) = extract_keys_from_line(line) {
                            let rel_path = match file_path.strip_prefix(&base_path) {
                                Ok(rel) => rel.to_string_lossy().to_string(),
                                Err(_) => file_path.to_string_lossy().to_string(),
                            };

                            let is_direct_key = raw_keys.contains("Key.")
                                || raw_keys.contains("ModifierKey.")
                                || raw_keys.starts_with('"')
                                || raw_keys.starts_with('\'');

                            let mut loop_items = Vec::new();
                            if !is_direct_key {
                                let start_search = idx.saturating_sub(35);
                                for prev_idx in (start_search..idx).rev() {
                                    let prev_line = lines[prev_idx].trim();
                                    if prev_line.starts_with("for ") && (prev_line.contains("ipairs(") || prev_line.contains("pairs(")) {
                                        if let Some(in_pos) = prev_line.find(" in ") {
                                            let var_part = prev_line[4..in_pos].trim();
                                            let item_var = var_part.split(',').last().unwrap_or(var_part).trim();
                                            if raw_keys.starts_with(&format!("{}.", item_var)) {
                                                let field = raw_keys[item_var.len() + 1..].trim();
                                                if let Some(open_p) = prev_line.find('(') {
                                                    if let Some(close_p) = prev_line[open_p..].find(')') {
                                                        let tbl = prev_line[open_p + 1..open_p + close_p].trim();
                                                        let items = find_table_array_keys(tbl, field, &mod_lua_files);
                                                        if !items.is_empty() {
                                                            loop_items = items;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !loop_items.is_empty() {
                                for (resolved_key, def_path, def_line_num, var_label) in loop_items {
                                    let def_rel = match def_path.strip_prefix(&base_path) {
                                        Ok(rel) => rel.to_string_lossy().to_string(),
                                        Err(_) => def_path.to_string_lossy().to_string(),
                                    };
                                    hotkeys.push(ModHotkey {
                                        mod_id: m.id.clone(),
                                        mod_name: m.name.clone(),
                                        file_path: rel_path.clone(),
                                        absolute_file_path: file_path.to_string_lossy().to_string(),
                                        line_number: idx + 1,
                                        keys: resolved_key,
                                        raw_line: line.to_string(),
                                        variable_name: Some(var_label),
                                        definition_file_path: Some(def_rel),
                                        definition_absolute_path: Some(def_path.to_string_lossy().to_string()),
                                        definition_line_number: Some(def_line_num),
                                        is_variable: true,
                                    });
                                }
                                continue;
                            }

                            let (final_keys, var_name, def_file_rel, def_file_abs, def_line, is_var) = if !is_direct_key {
                                if let Some((resolved, def_path, def_line_num)) = find_variable_in_lua_files(&raw_keys, &mod_lua_files, 0) {
                                    let def_rel = match def_path.strip_prefix(&base_path) {
                                        Ok(rel) => rel.to_string_lossy().to_string(),
                                        Err(_) => def_path.to_string_lossy().to_string(),
                                    };
                                    (resolved, Some(raw_keys.clone()), Some(def_rel), Some(def_path.to_string_lossy().to_string()), Some(def_line_num), true)
                                } else {
                                    (raw_keys.clone(), Some(raw_keys.clone()), None, None, None, false)
                                }
                            } else {
                                (raw_keys.clone(), None, None, None, None, false)
                            };

                            hotkeys.push(ModHotkey {
                                mod_id: m.id.clone(),
                                mod_name: m.name.clone(),
                                file_path: rel_path,
                                absolute_file_path: file_path.to_string_lossy().to_string(),
                                line_number: idx + 1,
                                keys: final_keys,
                                raw_line: line.to_string(),
                                variable_name: var_name,
                                definition_file_path: def_file_rel,
                                definition_absolute_path: def_file_abs,
                                definition_line_number: def_line,
                                is_variable: is_var,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(hotkeys)
}

#[tauri::command]
pub fn update_mod_hotkey(
    absolute_file_path: String,
    line_number: usize,
    new_keys: String,
    is_variable: Option<bool>,
    _variable_name: Option<String>,
) -> Result<(), String> {
    let path = Path::new(&absolute_file_path);
    if !path.exists() {
        return Err("File not found".to_string());
    }

    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    if line_number == 0 || line_number > lines.len() {
        return Err("Invalid line number".to_string());
    }

    let target_line = &lines[line_number - 1];

    let updated_line = if is_variable == Some(true) {
        if let Some(eq_idx) = target_line.find('=') {
            let left = &target_line[..=eq_idx];
            let comment_part = if let Some(c_idx) = target_line.find("--") {
                if c_idx > eq_idx {
                    format!(" {}", &target_line[c_idx..])
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let trimmed_end = target_line.trim_end();
            let ends_with_comma = trimmed_end.ends_with(',') || (trimmed_end.contains("--") && trimmed_end.split("--").next().unwrap_or("").trim().ends_with(','));
            let comma_suffix = if ends_with_comma { "," } else { "" };
            format!("{} {}{}{}", left, new_keys, comma_suffix, comment_part)
        } else {
            update_line_keybind(target_line, &new_keys).unwrap_or_else(|| target_line.clone())
        }
    } else {
        match update_line_keybind(target_line, &new_keys) {
            Some(line) => line,
            None => return Err("Failed to parse RegisterKeyBind on target line".to_string()),
        }
    };

    lines[line_number - 1] = updated_line;
    let ending = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let new_content = lines.join(ending) + ending;
    fs::write(path, new_content).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}
