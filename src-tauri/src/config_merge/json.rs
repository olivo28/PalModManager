use serde_json::Value;
use super::ChangedKeyDetail;

/// Strip comments and merge two JSON / JSONC contents.
pub fn merge_json(old: &str, new: &str, ignored_keys: &[String]) -> Option<String> {
    let old_clean = crate::commands::scanner::utils::strip_jsonc_comments(old);
    let new_clean = crate::commands::scanner::utils::strip_jsonc_comments(new);

    let old_json: Value = serde_json::from_str(&old_clean).ok()?;
    let new_json: Value = serde_json::from_str(&new_clean).ok()?;

    let merged_value = merge_json_values(&old_json, &new_json, "", ignored_keys);
    serde_json::to_string_pretty(&merged_value).ok()
}

/// Recursively merge two JSON values.
/// For matching keys, keep old_val (user edits) unless ignored.
/// If key only exists in new_val, keep it.
/// If key is an object, recurse.
pub fn merge_json_values(old_val: &Value, new_val: &Value, prefix: &str, ignored_keys: &[String]) -> Value {
    match (old_val, new_val) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            let mut merged_map = serde_json::Map::new();
            for (key, new_sub_val) in new_map {
                let full_key = if prefix.is_empty() { key.clone() } else { format!("{}.{}", prefix, key) };
                if ignored_keys.contains(&full_key) {
                    // Discard user old value, preserve author new default
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

/// Diff two JSON contents returning changed, added, and removed keys.
pub fn diff_json(old_content: &str, new_content: &str) -> Option<(Vec<ChangedKeyDetail>, Vec<String>, Vec<String>)> {
    let old_clean = crate::commands::scanner::utils::strip_jsonc_comments(old_content);
    let new_clean = crate::commands::scanner::utils::strip_jsonc_comments(new_content);

    let old_json: Value = serde_json::from_str(&old_clean).ok()?;
    let new_json: Value = serde_json::from_str(&new_clean).ok()?;

    let mut keys_user_changed = Vec::new();
    let mut keys_added_by_author = Vec::new();
    let mut keys_removed_by_author = Vec::new();

    diff_json_values(
        &old_json,
        &new_json,
        "",
        &mut keys_user_changed,
        &mut keys_added_by_author,
        &mut keys_removed_by_author,
    );

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
