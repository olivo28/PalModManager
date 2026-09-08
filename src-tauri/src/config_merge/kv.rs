use std::collections::HashMap;
use super::ChangedKeyDetail;

/// Merge key-value settings flatly (INI/CFG/TXT/TOML).
/// Preserves comments and structure of the new file, replacing values of matching keys unless ignored.
pub fn merge_kv(old: &str, new: &str, ignored_keys: &[String]) -> Option<String> {
    let old_map = parse_kv_map(old);
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

/// Diff two key-value string contents.
pub fn diff_kv(old_content: &str, new_content: &str) -> (Vec<ChangedKeyDetail>, Vec<String>, Vec<String>) {
    let old_map = parse_kv_map(old_content);
    let new_map = parse_kv_map(new_content);

    let mut keys_user_changed = Vec::new();
    let mut keys_added_by_author = Vec::new();
    let mut keys_removed_by_author = Vec::new();

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

    (keys_user_changed, keys_added_by_author, keys_removed_by_author)
}

/// Parse flat key-value pairs separated by '=' or ':'.
pub fn parse_kv_map(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
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
