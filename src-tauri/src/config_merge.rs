use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    /// relative path -> raw string content of the user's file
    pub entries: Vec<(PathBuf, String)>,
}

/// Walk the installed mod directory and collect all files matching config extensions,
/// plus any custom config file specified by the user in mod_info.config_path.
/// Resolves a raw mod or config path against the game root or dependency folders.
pub fn resolve_path_in_game(game_path: &Path, raw_path: &str) -> PathBuf {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return PathBuf::new();
    }

    // Strip [UE4SS] or [PalSchema] prefix if present
    let clean = if trimmed.starts_with('[') {
        let p_obj = Path::new(trimmed);
        let comps: Vec<&str> = p_obj.iter().filter_map(|p| p.to_str()).collect();
        if comps.len() > 1 {
            comps[1..].join("/")
        } else {
            trimmed.to_string()
        }
    } else {
        trimmed.to_string()
    };

    let p = Path::new(&clean);
    if p.is_absolute() && p.exists() {
        return p.to_path_buf();
    }

    // Direct join with game_path
    let candidate1 = game_path.join(&clean);
    if candidate1.exists() {
        return candidate1;
    }

    // If game_path ends with "Pal" and clean starts with "Pal/" or "Pal\"
    let clean_norm = clean.replace('\\', "/");
    if let Some(stripped) = clean_norm.strip_prefix("Pal/").or_else(|| clean_norm.strip_prefix("pal/")) {
        let candidate2 = game_path.join(stripped);
        if candidate2.exists() {
            return candidate2;
        }
        if let Some(parent) = game_path.parent() {
            let candidate3 = parent.join(&clean);
            if candidate3.exists() {
                return candidate3;
            }
        }
    } else if let Some(parent) = game_path.parent() {
        let candidate4 = parent.join(&clean);
        if candidate4.exists() {
            return candidate4;
        }
    }

    // Search under ue4ss/Mods and PalSchema/mods
    if let Some(filename) = p.file_name() {
        let ue4ss_dir = crate::dependency_checker::get_ue4ss_mods_dir(game_path);
        if ue4ss_dir.exists() {
            for entry in walkdir::WalkDir::new(&ue4ss_dir).max_depth(4).into_iter().flatten() {
                if entry.file_name() == filename {
                    return entry.path().to_path_buf();
                }
            }
        }
    }

    candidate1
}

/// Create a snapshot of all user-editable configuration files in a mod folder.
pub fn snapshot_configs(mod_dir: &Path, custom_config: Option<&str>) -> ConfigSnapshot {
    let mut entries = Vec::new();
    if !mod_dir.exists() {
        return ConfigSnapshot { entries };
    }

    let custom_filename = custom_config.and_then(|c| {
        let t = c.trim();
        if t.is_empty() { None } else { Path::new(t).file_name().map(|f| f.to_os_string()) }
    });

    fn walk(base: &Path, current: &Path, entries: &mut Vec<(PathBuf, String)>, custom_fname: &Option<std::ffi::OsString>) {
        if let Ok(dir_entries) = fs::read_dir(current) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(base, &path, entries, custom_fname);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "json" || ext_lower == "jsonc" || ext_lower == "ini" || ext_lower == "cfg" || ext_lower == "txt" || ext_lower == "lua" {
                            let fname_str = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                            // Skip metadata, dotfiles, and manifests from config snapshots
                            if fname_str.starts_with('.')
                                || fname_str.eq_ignore_ascii_case("modinfo.pmm.json")
                                || fname_str.eq_ignore_ascii_case("modinfo.json")
                                || fname_str.eq_ignore_ascii_case("enabled.txt")
                                || fname_str.ends_with(".manifest.json")
                            {
                                continue;
                            }

                            // Rule: .lua files ONLY merge if they are explicitly configured as the mod's config
                            let is_lua_config = if ext_lower == "lua" {
                                custom_fname.as_ref() == Some(&path.file_name().unwrap_or_default().to_os_string())
                            } else {
                                true
                            };

                            if is_lua_config {
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
    }

    walk(mod_dir, mod_dir, &mut entries, &custom_filename);

    // If a custom config was explicitly specified and not already snapshotted, include it
    if let Some(ref custom_str) = custom_config {
        let custom_trimmed = custom_str.trim();
        if !custom_trimmed.is_empty() {
            let path_obj = Path::new(custom_trimmed);
            let fname = path_obj.file_name();
            let already_present = entries.iter().any(|(rel, _)| {
                rel == path_obj || (fname.is_some() && rel.file_name() == fname)
            });

            if !already_present {
                if path_obj.is_absolute() && path_obj.is_file() {
                    if let Ok(content) = fs::read_to_string(path_obj) {
                        let rel = fname.map(PathBuf::from).unwrap_or_else(|| path_obj.to_path_buf());
                        entries.push((rel, content));
                    }
                } else if let Some(target_name) = fname {
                    fn find_file_rec(dir: &Path, target: &std::ffi::OsStr, base: &Path) -> Option<(PathBuf, String)> {
                        if let Ok(rd) = fs::read_dir(dir) {
                            for e in rd.flatten() {
                                let p = e.path();
                                if p.is_dir() {
                                    if let Some(found) = find_file_rec(&p, target, base) {
                                        return Some(found);
                                    }
                                } else if p.is_file() && p.file_name() == Some(target) {
                                    if let Ok(content) = fs::read_to_string(&p) {
                                        let rel = p.strip_prefix(base).map(|r| r.to_path_buf()).unwrap_or_else(|_| PathBuf::from(target));
                                        return Some((rel, content));
                                    }
                                }
                            }
                        }
                        None
                    }
                    if let Some(found) = find_file_rec(mod_dir, target_name, mod_dir) {
                        entries.push(found);
                    }
                }
            }
        }
    }

    ConfigSnapshot { entries }
}

/// Apply the merging function to combine snapshot files back into the newly installed folder
pub fn apply_config_merge(mod_dir: &Path, snapshot: &ConfigSnapshot, ignored_keys: &[String]) {
    for (rel_path, old_content) in &snapshot.entries {
        let fname_str = rel_path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        let rel_path_str = rel_path.to_string_lossy();
        if ignored_keys.iter().any(|k| k.eq_ignore_ascii_case(fname_str) || k.eq_ignore_ascii_case(&rel_path_str)) {
            crate::logger::log(&format!("Config merge: Skipping explicitly ignored file {}", fname_str));
            continue;
        }

        let mut target_file: Option<PathBuf> = None;
        let direct = mod_dir.join(rel_path);
        if direct.exists() && direct.is_file() {
            target_file = Some(direct);
        } else {
            let candidate_scripts = mod_dir.join("Scripts").join(rel_path);
            if candidate_scripts.exists() && candidate_scripts.is_file() {
                target_file = Some(candidate_scripts);
            } else if let Some(fname) = rel_path.file_name() {
                let cand_root = mod_dir.join(fname);
                if cand_root.exists() && cand_root.is_file() {
                    target_file = Some(cand_root);
                } else {
                    let cand_s = mod_dir.join("Scripts").join(fname);
                    if cand_s.exists() && cand_s.is_file() {
                        target_file = Some(cand_s);
                    } else {
                        fn find_target(dir: &Path, name: &std::ffi::OsStr) -> Option<PathBuf> {
                            if let Ok(rd) = fs::read_dir(dir) {
                                for entry in rd.flatten() {
                                    let p = entry.path();
                                    if p.is_dir() {
                                        if let Some(found) = find_target(&p, name) {
                                            return Some(found);
                                        }
                                    } else if p.is_file() && p.file_name() == Some(name) {
                                        return Some(p);
                                    }
                                }
                            }
                            None
                        }
                        target_file = find_target(mod_dir, fname);
                    }
                }
            }
        }

        let new_file = match target_file {
            Some(f) => f,
            None => continue,
        };

        // Always save a safe pre-update backup of the user's previous file so it can never be lost
        let ext_str = rel_path.extension().and_then(|e| e.to_str()).unwrap_or("lua");
        let pre_update_bak = new_file.with_extension(format!("{}.pre-update.bak", ext_str));
        let _ = fs::write(&pre_update_bak, old_content);

        let ext = rel_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if let Ok(new_content) = fs::read_to_string(&new_file) {
            let merged = merge_file_contents(old_content, &new_content, &ext, ignored_keys);
            if let Some(result) = merged {
                let _ = fs::write(&new_file, result);
            }
        }
    }
}

pub fn merge_file_contents(old_content: &str, new_content: &str, ext: &str, ignored_keys: &[String]) -> Option<String> {
    match ext.to_lowercase().as_str() {
        "json" | "jsonc" => merge_json(old_content, new_content, ignored_keys),
        "ini" | "cfg" | "txt" | "toml" | "yaml" | "yml" => merge_kv(old_content, new_content, ignored_keys),
        "lua" => merge_lua(old_content, new_content, ignored_keys),
        _ => None,
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
    let old_clean = crate::commands::scanner::utils::strip_jsonc_comments(old);
    let new_clean = crate::commands::scanner::utils::strip_jsonc_comments(new);
    
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

/// Merge Lua table / config settings flatly.
/// Preserves comments, indentation, and structure of the new file, replacing values of matching keys unless ignored.
fn merge_lua(old: &str, new: &str, ignored_keys: &[String]) -> Option<String> {
    let old_map = parse_lua_map(old);
    let mut result_lines = Vec::new();

    for line in new.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() || line_trimmed.starts_with("--") {
            result_lines.push(line.to_string());
            continue;
        }

        if let Some(pos) = line.find('=') {
            let mut after_eq = line[pos + 1..].trim_start();
            let mut comment = "";
            if let Some(c_pos) = after_eq.find("--") {
                comment = &after_eq[c_pos..];
                after_eq = after_eq[..c_pos].trim_end();
            }

            // Skip table openings like `local Config = {`
            if after_eq.ends_with('{') {
                result_lines.push(line.to_string());
                continue;
            }

            let raw_key = line[..pos].trim();
            let key = raw_key.trim_matches(|c| c == '[' || c == ']' || c == '"' || c == '\'').trim().to_string();

            if ignored_keys.iter().any(|k| k == &key || k == raw_key || (raw_key.contains('.') && raw_key.ends_with(&format!(".{}", k)))) {
                result_lines.push(line.to_string());
                continue;
            }

            if let Some(old_val) = old_map.get(&key).or_else(|| old_map.get(raw_key)) {
                let leading_ws = &line[..line.len() - line.trim_start().len()];
                let has_comma = after_eq.ends_with(',');
                let comma_str = if has_comma { "," } else { "" };
                let comment_prefix = if !comment.is_empty() { " " } else { "" };

                result_lines.push(format!("{}{} = {}{}{}{}", leading_ws, raw_key, old_val, comma_str, comment_prefix, comment));
                continue;
            }
        }

        result_lines.push(line.to_string());
    }

    Some(result_lines.join("\r\n"))
}

fn parse_lua_map(content: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() || line_trimmed.starts_with("--") {
            continue;
        }
        if let Some(pos) = line_trimmed.find('=') {
            let mut val = line_trimmed[pos + 1..].trim();
            if let Some(comment_pos) = val.find("--") {
                val = val[..comment_pos].trim();
            }
            if val.ends_with('{') {
                // Table header
                continue;
            }
            if val.ends_with(',') {
                val = val[..val.len() - 1].trim();
            }

            let raw_key = line_trimmed[..pos].trim();
            let key = raw_key.trim_matches(|c| c == '[' || c == ']' || c == '"' || c == '\'').trim().to_string();
            if !key.is_empty() {
                map.insert(key, val.to_string());
            }
        }
    }
    map
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
        let old_clean = crate::commands::scanner::utils::strip_jsonc_comments(old_content);
        let new_clean = crate::commands::scanner::utils::strip_jsonc_comments(new_content);
        
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
    } else if ext_lower == "lua" {
        let old_map = parse_lua_map(old_content);
        let new_map = parse_lua_map(new_content);
        
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_lua_preserves_user_settings() {
        let old_lua = r#"
local Config = {
    ["ShowTimer"] = false,
    ["Scale"] = 1.5,
    ["PosX"] = 250,
    ["PosY"] = 400,
}
return Config
"#;

        let new_lua = r#"
local Config = {
    ["ShowTimer"] = true,
    ["Scale"] = 1.0,
    ["PosX"] = 100,
    ["PosY"] = 200,
    ["NewAuthorFeature"] = true,
}
return Config
"#;

        let diff = generate_config_diff(old_lua, new_lua, "lua").expect("diff should succeed");
        let (user_changed, added, removed) = diff;
        assert_eq!(user_changed.len(), 4);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0], "NewAuthorFeature");
        assert_eq!(removed.len(), 0);

        let merged = merge_lua(old_lua, new_lua, &[]).expect("merge should succeed");
        assert!(merged.contains("[\"ShowTimer\"] = false"));
        assert!(merged.contains("[\"Scale\"] = 1.5"));
        assert!(merged.contains("[\"PosX\"] = 250"));
        assert!(merged.contains("[\"PosY\"] = 400"));
        assert!(merged.contains("[\"NewAuthorFeature\"] = true"));
    }

    #[test]
    fn test_merge_lua_respects_ignored_keys() {
        let old_lua = r#"
Config = {}
Config.Enabled = false
Config.Version = "1.0.0"
"#;
        let new_lua = r#"
Config = {}
Config.Enabled = true
Config.Version = "2.0.0"
"#;
        let ignored = vec!["Version".to_string(), "Config.Version".to_string()];
        let merged = merge_lua(old_lua, new_lua, &ignored).expect("merge should succeed");
        assert!(merged.contains("Config.Enabled = false"));
        assert!(merged.contains("Config.Version = \"2.0.0\""));
    }

    #[test]
    fn test_snapshot_configs_lua_only_when_configured() {
        let temp_dir = std::env::temp_dir().join(format!("pmm_test_{}", uuid::Uuid::new_v4()));
        let scripts_dir = temp_dir.join("Scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();

        // Create main.lua, config.lua, and .nexus.json
        std::fs::write(scripts_dir.join("main.lua"), "-- main script").unwrap();
        std::fs::write(scripts_dir.join("config.lua"), "return { Setting = 1 }").unwrap();
        std::fs::write(temp_dir.join(".nexus.json"), "{}").unwrap();

        // Scenario 1: No custom_config configured -> main.lua and config.lua must NOT be snapshotted
        let snap1 = snapshot_configs(&temp_dir, None);
        assert!(!snap1.entries.iter().any(|(p, _)| p.to_string_lossy().contains("main.lua")));
        assert!(!snap1.entries.iter().any(|(p, _)| p.to_string_lossy().contains("config.lua")));
        assert!(!snap1.entries.iter().any(|(p, _)| p.to_string_lossy().contains(".nexus.json")));

        // Scenario 2: custom_config set to "config.lua" -> ONLY config.lua snapshotted, NEVER main.lua
        let snap2 = snapshot_configs(&temp_dir, Some("Scripts/config.lua"));
        assert!(!snap2.entries.iter().any(|(p, _)| p.to_string_lossy().contains("main.lua")));
        assert!(!snap2.entries.iter().any(|(p, _)| p.to_string_lossy().contains(".nexus.json")));
        assert!(snap2.entries.iter().any(|(p, _)| p.to_string_lossy().contains("config.lua")));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_apply_config_merge_skips_ignored_file() {
        let temp_dir = std::env::temp_dir().join(format!("pmm_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let cfg_file = temp_dir.join("config.json");
        std::fs::write(&cfg_file, r#"{"setting": "new_from_author"}"#).unwrap();

        let snap = ConfigSnapshot {
            entries: vec![(PathBuf::from("config.json"), r#"{"setting": "old_user_value"}"#.to_string())],
        };

        // If "config.json" is in ignored_keys, it must NOT be merged (keeps new_from_author)
        apply_config_merge(&temp_dir, &snap, &["config.json".to_string()]);
        let content = std::fs::read_to_string(&cfg_file).unwrap();
        assert!(content.contains("new_from_author"));
        assert!(!content.contains("old_user_value"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

