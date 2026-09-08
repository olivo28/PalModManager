use std::collections::{HashMap, HashSet};
use super::ChangedKeyDetail;

const RESERVED_LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for",
    "function", "goto", "if", "in", "local", "nil", "not", "or",
    "repeat", "return", "then", "true", "until", "while",
];

/// Checks whether a Lua file represents a pure configuration file (e.g. settings table)
/// rather than an executable script with functions, UE4SS hooks, or runtime logic.
pub fn is_lua_config_content(content: &str) -> bool {
    let lower = content.to_lowercase();

    // Check for UE4SS hook registrations and engine threading calls
    let engine_hooks = [
        "registerhook",
        "registercustomevent",
        "registerkeybind",
        "registerbeginplay",
        "registerloadmap",
        "registerendplay",
        "executeingamethread",
        "executewithdelay",
        "staticfindobject",
        "findallof",
        "notifyonnewobject",
        "xpcall(",
        "pcall(",
    ];

    for hook in &engine_hooks {
        if lower.contains(hook) {
            return false;
        }
    }

    // Inspect line by line for procedural code, functions, loops, and control flow
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }

        let trimmed_lower = trimmed.to_lowercase();

        // Check for function definitions
        if trimmed_lower.starts_with("function ")
            || trimmed_lower.starts_with("function(")
            || trimmed_lower.starts_with("function{")
            || trimmed_lower.starts_with("local function ")
            || trimmed_lower.starts_with("local function(")
            || trimmed_lower.contains("= function(")
            || trimmed_lower.contains("= function (")
        {
            return false;
        }

        // Check for loops and module imports
        if (trimmed_lower.starts_with("while ") && trimmed_lower.ends_with(" do"))
            || (trimmed_lower.starts_with("for ") && trimmed_lower.contains(" do"))
            || trimmed_lower.starts_with("repeat")
            || trimmed_lower.starts_with("until ")
            || (trimmed_lower.starts_with("require(") || trimmed_lower.starts_with("require \"") || trimmed_lower.starts_with("require '"))
        {
            return false;
        }
    }

    true
}

/// Validates whether a raw LHS string is a valid Lua config key identifier or table path.
pub fn extract_lua_config_key(raw_key: &str) -> Option<String> {
    let mut k = raw_key.trim();
    if k.starts_with("local ") {
        k = k[6..].trim();
    }

    // Multiple assignments like `local a, b = ...` are procedural code, not configs
    if k.contains(',') {
        return None;
    }

    // Bracketed string key: ["Key"] or ['Key']
    if (k.starts_with("[\"") && k.ends_with("\"]")) || (k.starts_with("['") && k.ends_with("']")) {
        let inner = &k[2..k.len() - 2];
        if !inner.trim().is_empty() {
            return Some(inner.trim().to_string());
        }
        return None;
    }

    // Dotted identifier path or single identifier: e.g. Config.Speed or Speed
    let segments: Vec<&str> = k.split('.').collect();
    if segments.is_empty() {
        return None;
    }

    let reserved: HashSet<&str> = RESERVED_LUA_KEYWORDS.iter().copied().collect();

    for seg in &segments {
        let seg_trimmed = seg.trim();
        if seg_trimmed.is_empty() || reserved.contains(seg_trimmed) {
            return None;
        }

        let first_char = seg_trimmed.chars().next()?;
        if !first_char.is_ascii_alphabetic() && first_char != '_' {
            return None;
        }

        if !seg_trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
    }

    Some(k.to_string())
}

/// Parse valid Lua configuration assignments into a key-value dictionary.
pub fn parse_lua_map(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    // Guard: Never parse executable scripts
    if !is_lua_config_content(content) {
        return map;
    }

    for line in content.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() || line_trimmed.starts_with("--") {
            continue;
        }

        // Never treat conditional statements or comparison operators as key-value assignments
        if line_trimmed.contains("==")
            || line_trimmed.contains("~=")
            || line_trimmed.contains("<=")
            || line_trimmed.contains(">=")
            || line_trimmed.starts_with("if ")
            || line_trimmed.starts_with("elseif ")
            || line_trimmed.starts_with("else")
            || line_trimmed.starts_with("end")
        {
            continue;
        }

        if let Some(pos) = line_trimmed.find('=') {
            let raw_key = line_trimmed[..pos].trim();
            let key = match extract_lua_config_key(raw_key) {
                Some(k) => k,
                None => continue,
            };

            let mut val = line_trimmed[pos + 1..].trim();
            if let Some(comment_pos) = val.find("--") {
                val = val[..comment_pos].trim();
            }

            // Skip table headers like `local Config = {` or `return {`
            if val.ends_with('{') || val.is_empty() {
                continue;
            }

            // Skip procedural expressions
            let val_lower = val.to_lowercase();
            if val_lower.starts_with("function") || val_lower.starts_with("pcall") || val_lower.starts_with("xpcall") {
                continue;
            }

            if val.ends_with(',') {
                val = val[..val.len() - 1].trim();
            }

            if !val.is_empty() {
                map.insert(key, val.to_string());
            }
        }
    }

    map
}

/// Merge Lua table / config settings flatly.
/// Preserves comments, indentation, and structure of the new file, replacing values of matching keys unless ignored.
/// Refuses to touch files that contain executable procedural code or hooks to prevent corruption.
pub fn merge_lua(old: &str, new: &str, ignored_keys: &[String]) -> Option<String> {
    if !is_lua_config_content(new) {
        crate::logger::log("Config merge: Target Lua file contains executable code/hooks; refusing to merge as a config table.");
        return None;
    }
    if !is_lua_config_content(old) {
        crate::logger::log("Config merge: Snapshot Lua file contains executable code/hooks; refusing to merge.");
        return None;
    }

    let old_map = parse_lua_map(old);
    let mut result_lines = Vec::new();

    for line in new.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() || line_trimmed.starts_with("--") {
            result_lines.push(line.to_string());
            continue;
        }

        // Never treat conditional statements or comparison operators as key-value assignments
        if line_trimmed.contains("==")
            || line_trimmed.contains("~=")
            || line_trimmed.contains("<=")
            || line_trimmed.contains(">=")
            || line_trimmed.starts_with("if ")
            || line_trimmed.starts_with("elseif ")
            || line_trimmed.starts_with("else")
            || line_trimmed.starts_with("end")
        {
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

            // Skip table openings like `local Config = {` or `return {`
            if after_eq.ends_with('{') || after_eq.is_empty() {
                result_lines.push(line.to_string());
                continue;
            }

            let raw_key = line[..pos].trim();
            let key = match extract_lua_config_key(raw_key) {
                Some(k) => k,
                None => {
                    result_lines.push(line.to_string());
                    continue;
                }
            };

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

/// Diff two Lua config contents.
pub fn diff_lua(old_content: &str, new_content: &str) -> (Vec<ChangedKeyDetail>, Vec<String>, Vec<String>) {
    let old_map = parse_lua_map(old_content);
    let new_map = parse_lua_map(new_content);

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
