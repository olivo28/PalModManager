use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    /// relative path -> raw string content of the user's file
    pub entries: Vec<(PathBuf, String)>,
}

/// Walk the installed mod directory and collect all files matching config extensions
pub fn snapshot_configs(mod_dir: &Path) -> ConfigSnapshot {
    let mut entries = Vec::new();
    if !mod_dir.exists() {
        return ConfigSnapshot { entries };
    }
    
    fn walk(base: &Path, current: &Path, entries: &mut Vec<(PathBuf, String)>) {
        if let Ok(dir_entries) = fs::read_dir(current) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(base, &path, entries);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "json" || ext_lower == "jsonc" || ext_lower == "ini" || ext_lower == "cfg" || ext_lower == "txt" {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(rel) = path.strip_prefix(base) {
                                    entries.push((rel.to_path_buf(), content));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    walk(mod_dir, mod_dir, &mut entries);
    ConfigSnapshot { entries }
}

/// Apply the merging function to combine snapshot files back into the newly installed folder
pub fn apply_config_merge(mod_dir: &Path, snapshot: &ConfigSnapshot, ignored_keys: &[String]) {
    for (rel_path, old_content) in &snapshot.entries {
        let new_file = mod_dir.join(rel_path);
        if !new_file.exists() {
            continue; // File removed by author, skip
        }
        
        let ext = rel_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if let Ok(new_content) = fs::read_to_string(&new_file) {
            let merged = match ext.as_str() {
                "json" | "jsonc" => merge_json(old_content, &new_content, ignored_keys),
                "ini" | "cfg" | "txt" => merge_kv(old_content, &new_content, ignored_keys),
                _ => None,
            };
            if let Some(result) = merged {
                let _ = fs::write(&new_file, result);
            }
        }
    }
}

/// Recursively merge two JSON values.
/// For matching keys, keep old_val (user edits) unless ignored.
/// If key only exists in new_val, keep it.
/// If key is an object, recurse.
fn merge_json_values(old_val: &Value, new_val: &Value, prefix: &str, ignored_keys: &[String]) -> Value {
    match (old_val, new_val) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            let mut merged_map = serde_json::Map::new();
            for (key, new_sub_val) in new_map {
                let full_key = if prefix.is_empty() { key.clone() } else { format!("{}.{}", prefix, key) };
                if ignored_keys.contains(&full_key) {
                    // Ignored key: discard user old value, use author new default value
                    merged_map.insert(key.clone(), new_sub_val.clone());
                    continue;
                }
                if let Some(old_sub_val) = old_map.get(key) {
                    merged_map.insert(key.clone(), merge_json_values(old_sub_val, new_sub_val, &full_key, ignored_keys));
                } else {
                    merged_map.insert(key.clone(), new_sub_val.clone());
                }
            }
            Value::Object(merged_map)
        }
        (old_val, _) => old_val.clone(),
    }
}

fn merge_json(old: &str, new: &str, ignored_keys: &[String]) -> Option<String> {
    let old_clean = crate::commands::scanner_commands::strip_jsonc_comments(old);
    let new_clean = crate::commands::scanner_commands::strip_jsonc_comments(new);
    
    let old_json: Value = serde_json::from_str(&old_clean).ok()?;
    let new_json: Value = serde_json::from_str(&new_clean).ok()?;
    
    let merged_value = merge_json_values(&old_json, &new_json, "", ignored_keys);
    serde_json::to_string_pretty(&merged_value).ok()
}

/// Merge key-value settings flatly (INI/CFG/TXT).
/// Preserves comments and structure of the new file, replacing values of matching keys unless ignored.
fn merge_kv(old: &str, new: &str, ignored_keys: &[String]) -> Option<String> {
    let mut old_map = std::collections::HashMap::new();
    for line in old.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() || line_trimmed.starts_with(';') || line_trimmed.starts_with('#') || line_trimmed.starts_with("//") {
            continue;
        }
        if let Some(pos) = line_trimmed.find('=') {
            let k = line_trimmed[..pos].trim().to_string();
            let v = line_trimmed[pos + 1..].trim().to_string();
            if !k.is_empty() {
                old_map.insert(k, v);
            }
        } else if let Some(pos) = line_trimmed.find(':') {
            let k = line_trimmed[..pos].trim().to_string();
            let v = line_trimmed[pos + 1..].trim().to_string();
            if !k.is_empty() {
                old_map.insert(k, v);
            }
        }
    }
    
    let mut result_lines = Vec::new();
    for line in new.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() || line_trimmed.starts_with(';') || line_trimmed.starts_with('#') || line_trimmed.starts_with("//") {
            result_lines.push(line.to_string());
            continue;
        }
        
        let delimiter = if line_trimmed.contains('=') {
            Some('=')
        } else if line_trimmed.contains(':') {
            Some(':')
        } else {
            None
        };
        
        if let Some(delim) = delimiter {
            if let Some(pos) = line.find(delim) {
                let k = line[..pos].trim().to_string();
                if ignored_keys.contains(&k) {
                    // Ignored key: keep new author default line
                    result_lines.push(line.to_string());
                    continue;
                }
                if let Some(old_val) = old_map.get(&k) {
                    let leading_ws = &line[..line.len() - line.trim_start().len()];
                    result_lines.push(format!("{}{}{} {}", leading_ws, k, delim, old_val));
                    continue;
                }
            }
        }
        result_lines.push(line.to_string());
    }
    
    Some(result_lines.join("\r\n"))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangedKeyDetail {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
}

pub fn generate_config_diff(old_content: &str, new_content: &str, ext: &str) -> Option<(Vec<ChangedKeyDetail>, Vec<String>, Vec<String>)> {
    let mut keys_user_changed = Vec::new();
    let mut keys_added_by_author = Vec::new();
    let mut keys_removed_by_author = Vec::new();

    let ext_lower = ext.to_lowercase();
    if ext_lower == "json" || ext_lower == "jsonc" {
        let old_clean = crate::commands::scanner_commands::strip_jsonc_comments(old_content);
        let new_clean = crate::commands::scanner_commands::strip_jsonc_comments(new_content);
        
        let old_json: Value = serde_json::from_str(&old_clean).ok()?;
        let new_json: Value = serde_json::from_str(&new_clean).ok()?;
        
        diff_json_values(&old_json, &new_json, "", &mut keys_user_changed, &mut keys_added_by_author, &mut keys_removed_by_author);
    } else if ext_lower == "ini" || ext_lower == "cfg" || ext_lower == "txt" {
        let old_map = parse_kv_map(old_content);
        let new_map = parse_kv_map(new_content);
        
        for (k, new_v) in &new_map {
            if let Some(old_v) = old_map.get(k) {
                if old_v != new_v {
                    keys_user_changed.push(ChangedKeyDetail {
                        key: k.clone(),
                        old_value: old_v.clone(),
                        new_value: new_v.clone(),
                    });
                }
            } else {
                keys_added_by_author.push(k.clone());
            }
        }
        for (k, _) in &old_map {
            if !new_map.contains_key(k) {
                keys_removed_by_author.push(k.clone());
            }
        }
    } else {
        return None;
    }

    Some((keys_user_changed, keys_added_by_author, keys_removed_by_author))
}

fn diff_json_values(
    old_val: &Value,
    new_val: &Value,
    prefix: &str,
    keys_user_changed: &mut Vec<ChangedKeyDetail>,
    keys_added_by_author: &mut Vec<String>,
    keys_removed_by_author: &mut Vec<String>,
) {
    match (old_val, new_val) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            for (k, new_sub) in new_map {
                let full_k = if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
                if let Some(old_sub) = old_map.get(k) {
                    diff_json_values(old_sub, new_sub, &full_k, keys_user_changed, keys_added_by_author, keys_removed_by_author);
                } else {
                    keys_added_by_author.push(full_k);
                }
            }
            for (k, _) in old_map {
                if !new_map.contains_key(k) {
                    let full_k = if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
                    keys_removed_by_author.push(full_k);
                }
            }
        }
        (old_primitive, new_primitive) => {
            if old_primitive != new_primitive {
                let old_str = match old_primitive {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let new_str = match new_primitive {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                keys_user_changed.push(ChangedKeyDetail {
                    key: prefix.to_string(),
                    old_value: old_str,
                    new_value: new_str,
                });
            }
        }
    }
}

fn parse_kv_map(content: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() || line_trimmed.starts_with(';') || line_trimmed.starts_with('#') || line_trimmed.starts_with("//") {
            continue;
        }
        let delimiter = if line_trimmed.contains('=') {
            Some('=')
        } else if line_trimmed.contains(':') {
            Some(':')
        } else {
            None
        };
        if let Some(delim) = delimiter {
            if let Some(pos) = line_trimmed.find(delim) {
                let k = line_trimmed[..pos].trim().to_string();
                let v = line_trimmed[pos + 1..].trim().to_string();
                if !k.is_empty() {
                    map.insert(k, v);
                }
            }
        }
    }
    map
}
