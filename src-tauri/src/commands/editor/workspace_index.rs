use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use serde_json::Value;
use crate::models::ModInfo;
use super::types::EditorCompletion;

#[derive(Debug, Clone)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub source_file: String,
    pub value_preview: Option<String>,
    pub kind: String, // "property" | "function" | "constant" | "field"
}

#[derive(Debug, Default, Clone)]
struct CachedModIndex {
    file_timestamps: HashMap<String, u64>,
    symbols: Vec<WorkspaceSymbol>,
}

static WORKSPACE_INDEX_CACHE: Mutex<Option<HashMap<String, CachedModIndex>>> = Mutex::new(None);

/// Invalidate cached symbols for a mod (e.g. on file save/reload)
#[allow(dead_code)]
pub fn invalidate_workspace_index(mod_id: &str) {
    if let Ok(mut guard) = WORKSPACE_INDEX_CACHE.lock() {
        if let Some(ref mut cache) = *guard {
            cache.remove(mod_id);
        }
    }
}

/// Strip comments from JSONC content
fn strip_json_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_string = false;
    let mut is_escaped = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if in_string {
            output.push(c);
            if is_escaped {
                is_escaped = false;
            } else if c == '\\' {
                is_escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else {
            if c == '"' {
                in_string = true;
                output.push(c);
            } else if c == '/' {
                if let Some(&next_c) = chars.peek() {
                    if next_c == '/' {
                        // Line comment: skip until newline
                        chars.next();
                        while let Some(&c_inner) = chars.peek() {
                            if c_inner == '\n' {
                                break;
                            }
                            chars.next();
                        }
                    } else if next_c == '*' {
                        // Block comment: skip until */
                        chars.next();
                        while let Some(c_inner) = chars.next() {
                            if c_inner == '*' && chars.peek() == Some(&'/') {
                                chars.next();
                                break;
                            }
                        }
                    } else {
                        output.push(c);
                    }
                } else {
                    output.push(c);
                }
            } else {
                output.push(c);
            }
        }
    }
    output
}

/// Recursively extract JSON keys and values into symbols
fn extract_json_symbols(val: &Value, source_file: &str, depth: usize, symbols: &mut Vec<WorkspaceSymbol>) {
    if depth > 5 {
        return;
    }

    if let Value::Object(map) = val {
        for (key, v) in map {
            if !key.trim().is_empty() {
                let preview = match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    Value::Bool(b) => Some(b.to_string()),
                    Value::Array(a) => Some(format!("[{} items]", a.len())),
                    Value::Object(o) => Some(format!("{{ {} fields }}", o.len())),
                    Value::Null => Some("null".to_string()),
                };

                symbols.push(WorkspaceSymbol {
                    name: key.clone(),
                    source_file: source_file.to_string(),
                    value_preview: preview,
                    kind: "property".to_string(),
                });
            }

            extract_json_symbols(v, source_file, depth + 1, symbols);
        }
    } else if let Value::Array(arr) = val {
        for v in arr {
            extract_json_symbols(v, source_file, depth + 1, symbols);
        }
    }
}

/// Extract function and table declarations from a Lua file
fn extract_lua_symbols(content: &str, source_file: &str, symbols: &mut Vec<WorkspaceSymbol>) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") {
            continue;
        }

        // 1. function Name( or function Module.Name( or function Module:Name(
        if trimmed.starts_with("function ") {
            let after_func = &trimmed[9..].trim_start();
            if let Some(paren_idx) = after_func.find('(') {
                let func_name = after_func[..paren_idx].trim();
                if !func_name.is_empty() && !func_name.contains(' ') {
                    symbols.push(WorkspaceSymbol {
                        name: func_name.to_string(),
                        source_file: source_file.to_string(),
                        value_preview: Some(format!("function {}", &after_func[..paren_idx + 1])),
                        kind: "function".to_string(),
                    });

                    // If func_name is Module.Method, also index the sub-method "Method"
                    if let Some(dot_idx) = func_name.rfind('.') {
                        let sub_name = &func_name[dot_idx + 1..];
                        if !sub_name.is_empty() {
                            symbols.push(WorkspaceSymbol {
                                name: sub_name.to_string(),
                                source_file: source_file.to_string(),
                                value_preview: Some(format!("function {}", func_name)),
                                kind: "function".to_string(),
                            });
                        }
                    } else if let Some(colon_idx) = func_name.rfind(':') {
                        let sub_name = &func_name[colon_idx + 1..];
                        if !sub_name.is_empty() {
                            symbols.push(WorkspaceSymbol {
                                name: sub_name.to_string(),
                                source_file: source_file.to_string(),
                                value_preview: Some(format!("function {}", func_name)),
                                kind: "function".to_string(),
                            });
                        }
                    }
                }
            }
        }
        // 2. Module = {} or local Module = {}
        else if trimmed.contains("= {}") || trimmed.contains("= { }") {
            let before_eq = trimmed.split('=').next().unwrap_or("").trim();
            let table_name = if before_eq.starts_with("local ") {
                before_eq[6..].trim()
            } else {
                before_eq
            };

            if !table_name.is_empty() && !table_name.contains(' ') && !table_name.contains('.') {
                symbols.push(WorkspaceSymbol {
                    name: table_name.to_string(),
                    source_file: source_file.to_string(),
                    value_preview: Some(format!("table {}", table_name)),
                    kind: "constant".to_string(),
                });
            }
        }
    }
}

/// Get or build the symbol index for the given mod workspace
fn is_internal_or_hidden_file(file_path: &str) -> bool {
    let lower = file_path.to_ascii_lowercase();
    lower.ends_with("modinfo.pmm.json")
        || lower.ends_with(".pmm.json")
        || lower.ends_with("modinfo.json")
        || lower.ends_with(".nexus.json")
        || lower.starts_with('.')
        || lower.contains("/.")
        || lower.contains("\\.")
}

pub fn get_or_build_mod_symbols(
    mod_info: &ModInfo,
    file_list: &[String],
) -> Vec<WorkspaceSymbol> {
    let mut guard = match WORKSPACE_INDEX_CACHE.lock() {
        Ok(c) => c,
        Err(poisoned) => poisoned.into_inner(),
    };

    let cache = guard.get_or_insert_with(HashMap::new);
    let mod_id = &mod_info.id;
    let mut needs_refresh = false;

    // Check if cached entry exists and timestamps are fresh
    if let Some(existing) = cache.get(mod_id) {
        if existing.file_timestamps.keys().any(|k| is_internal_or_hidden_file(k)) {
            needs_refresh = true;
        }

        for file_rel in file_list {
            if is_internal_or_hidden_file(file_rel) {
                continue;
            }

            let ext = Path::new(file_rel)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if ext != "json" && ext != "jsonc" && ext != "lua" {
                continue;
            }

            if let Ok(full_path) = crate::commands::config_commands::get_full_mod_file_path(mod_info, file_rel) {
                if let Ok(meta) = fs::metadata(&full_path) {
                    let mtime = meta.modified()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "")))
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    if existing.file_timestamps.get(file_rel).copied().unwrap_or(0) != mtime {
                        needs_refresh = true;
                        break;
                    }
                }
            }
        }

        if !needs_refresh {
            return existing.symbols.clone();
        }
    }

    // Index all mod files
    let mut new_timestamps = HashMap::new();
    let mut new_symbols = Vec::new();

    for file_rel in file_list {
        if is_internal_or_hidden_file(file_rel) {
            continue;
        }

        let ext = Path::new(file_rel)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if ext != "json" && ext != "jsonc" && ext != "lua" {
            continue;
        }

        let full_path = match crate::commands::config_commands::get_full_mod_file_path(mod_info, file_rel) {
            Ok(p) => p,
            Err(_) => continue,
        };

        if !full_path.exists() {
            continue;
        }

        if let Ok(meta) = fs::metadata(&full_path) {
            let mtime = meta.modified()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "")))
                .map(|d| d.as_secs())
                .unwrap_or(0);
            new_timestamps.insert(file_rel.clone(), mtime);
        }

        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if ext == "json" || ext == "jsonc" {
            let clean = if ext == "jsonc" {
                strip_json_comments(&content)
            } else {
                content
            };

            if let Ok(val) = serde_json::from_str::<Value>(&clean) {
                extract_json_symbols(&val, file_rel, 0, &mut new_symbols);
            }
        } else if ext == "lua" {
            extract_lua_symbols(&content, file_rel, &mut new_symbols);
        }
    }

    cache.insert(mod_id.clone(), CachedModIndex {
        file_timestamps: new_timestamps,
        symbols: new_symbols.clone(),
    });

    new_symbols
}

/// Find matching workspace symbols for a given query
pub fn find_workspace_completions(
    symbols: &[WorkspaceSymbol],
    query: &str,
    limit: usize,
) -> Vec<EditorCompletion> {
    let q_lower = query.trim().to_ascii_lowercase();
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sym in symbols {
        let name_lower = sym.name.to_ascii_lowercase();

        let is_match = if q_lower.is_empty() {
            true
        } else {
            name_lower.starts_with(&q_lower) || name_lower.contains(&q_lower)
        };

        if is_match && seen.insert(sym.name.clone()) {
            let doc_text = if let Some(ref prev) = sym.value_preview {
                format!("**Defined in**: `{}`\n\n```\n{}\n```", sym.source_file, prev)
            } else {
                format!("**Defined in**: `{}`", sym.source_file)
            };

            results.push(EditorCompletion {
                label: sym.name.clone(),
                insert_text: sym.name.clone(),
                kind: sym.kind.clone(),
                detail: Some(format!("📦 {}", sym.source_file)),
                documentation: Some(doc_text),
            });

            if results.len() >= limit {
                break;
            }
        }
    }

    results
}
