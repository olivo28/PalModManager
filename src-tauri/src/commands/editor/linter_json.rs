use std::collections::{HashMap, HashSet};
use super::types::EditorDiagnostic;

pub fn lint_json_syntax(clean_content: &str, diagnostics: &mut Vec<EditorDiagnostic>) {
    if clean_content.trim().is_empty() {
        return;
    }

    let stripped = crate::commands::scanner::utils::strip_jsonc_comments(clean_content);
    let lines: Vec<&str> = clean_content.lines().collect();

    // 1. Strict syntax parsing error
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&stripped) {
        let line = e.line() as u32;
        let col = e.column() as u32;
        diagnostics.push(EditorDiagnostic {
            line: if line == 0 { 1 } else { line },
            column: if col == 0 { 1 } else { col },
            end_line: if line == 0 { 1 } else { line },
            end_column: if col == 0 { 10 } else { col + 5 },
            severity: "error".to_string(),
            message: format!("JSON Syntax Error: {}", e),
            target: String::new(),
            suggestion: None,
            category: "syntax".to_string(),
        });
    }

    // 2. Duplicate key detector (ignores comments)
    let mut seen_keys_at_depth: HashMap<usize, HashSet<String>> = HashMap::new();
    let mut depth: usize = 0;

    for (i, line) in lines.iter().enumerate() {
        let line_num = (i + 1) as u32;
        let trimmed = line.trim();

        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        for c in trimmed.chars() {
            if c == '{' {
                depth += 1;
                seen_keys_at_depth.entry(depth).or_default().clear();
            } else if c == '}' && depth > 0 {
                seen_keys_at_depth.remove(&depth);
                depth -= 1;
            }
        }

        // Look for "key": pattern
        if let Some(colon_pos) = trimmed.find(':') {
            let before_colon = trimmed[..colon_pos].trim();
            if before_colon.starts_with('"') && before_colon.ends_with('"') && before_colon.len() >= 2 {
                let key_name = &before_colon[1..before_colon.len() - 1];
                let set = seen_keys_at_depth.entry(depth).or_default();
                if !set.insert(key_name.to_string()) {
                    let col = (line.find(before_colon).unwrap_or(0) + 1) as u32;
                    diagnostics.push(EditorDiagnostic {
                        line: line_num,
                        column: col,
                        end_line: line_num,
                        end_column: col + before_colon.len() as u32,
                        severity: "warning".to_string(),
                        message: format!("Duplicate key '{}' in JSON object (prior entry will be overwritten)", key_name),
                        target: key_name.to_string(),
                        suggestion: None,
                        category: "syntax".to_string(),
                    });
                }
            }
        }
    }
}

pub fn find_key_position(lines: &[&str], key: &str) -> (u32, u32) {
    let pattern = format!("\"{}\"", key);
    for (i, line) in lines.iter().enumerate() {
        if let Some(col) = line.find(&pattern) {
            return ((i + 1) as u32, (col + 1) as u32);
        }
    }
    (1, 1)
}
