use std::collections::HashMap;
use std::path::Path;
use tauri::State;
use crate::state::AppState;
use crate::usmap::{get_or_load_sdk_index, sync::get_active_usmap_path, parser::parse_usmap_file};
use super::types::EditorDiagnostic;
use super::linter_lua::lint_lua_syntax;
use super::linter_json::{lint_json_syntax, find_key_position};

#[tauri::command]
pub async fn validate_editor_code(
    file_path: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Vec<EditorDiagnostic>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();

    let mut diagnostics = Vec::new();
    let ext = Path::new(&file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Performance guard: Skip deep static analysis on massive data dumps (>2MB)
    if content.len() > 2 * 1024 * 1024 {
        return Ok(Vec::new());
    }

    let usmap_path = get_active_usmap_path(&program_path);
    let schema = if usmap_path.exists() {
        parse_usmap_file(&usmap_path).ok()
    } else {
        None
    };

    let sdk_index = get_or_load_sdk_index(&program_path, &game_path);

    if ext == "lua" {
        // -------------------------------------------------------------
        // Layer 1: Robust State-Machine Lua Syntax & Static Analysis
        // -------------------------------------------------------------
        lint_lua_syntax(&content, &mut diagnostics);

        // -------------------------------------------------------------
        // Layer 2: UE4SS Reflection & Hook Validation
        // -------------------------------------------------------------
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let line_num = (i + 1) as u32;

            let mut search_idx = 0;
            while let Some(start_pos) = find_hook_start(line, search_idx) {
                if let Some((target, _quote_char, col_offset)) = extract_string_literal(line, start_pos) {
                    let trimmed = target.trim();
                    let is_incomplete = trimmed.is_empty()
                        || trimmed == "/"
                        || trimmed == "/Script"
                        || trimmed == "/Script/"
                        || trimmed.ends_with('/')
                        || (trimmed.ends_with(':') && !trimmed.contains("__DelegateSignature"));

                    if !is_incomplete && (trimmed.contains('/') || trimmed.contains(':') || trimmed.starts_with("Pal") || trimmed.starts_with("APal") || trimmed.starts_with("UPal")) {
                        let is_builtin_engine = trimmed.starts_with("/Script/CoreUObject.")
                            || trimmed.starts_with("/Script/SlateCore.")
                            || trimmed.starts_with("/Script/Slate.");

                        if !is_builtin_engine {
                            if let Some(ref s) = schema {
                                let (_clean_class, _clean_func, status, reason, suggestion) =
                                        crate::commands::scanner::hook_validator::validate_hook_with_usmap(
                                        trimmed,
                                        s,
                                        sdk_index.as_ref(),
                                    );

                                if status == "broken_class" || status == "broken_function" {
                                    let severity = if status == "broken_class" { "error" } else { "warning" };
                                    diagnostics.push(EditorDiagnostic {
                                        line: line_num,
                                        column: col_offset as u32,
                                        end_line: line_num,
                                        end_column: (col_offset + trimmed.len()) as u32,
                                        severity: severity.to_string(),
                                        message: reason,
                                        target: trimmed.to_string(),
                                        suggestion,
                                        category: "ue4ss".to_string(),
                                    });
                                }
                            }
                        }
                    }
                    search_idx = start_pos + target.len() + 2;
                } else {
                    break;
                }
            }
        }
    } else if ext == "json" || ext == "jsonc" {
        // -------------------------------------------------------------
        // Layer 1: Universal JSON Syntax & Duplicate Key Detection
        // -------------------------------------------------------------
        let clean_content = content.strip_prefix("\u{feff}").unwrap_or(&content);
        lint_json_syntax(clean_content, &mut diagnostics);

        // -------------------------------------------------------------
        // Layer 2: PalSchema DataTables & USMAP Validation
        // -------------------------------------------------------------
        if !clean_content.trim().is_empty() {
            let stripped = crate::commands::scanner::utils::strip_jsonc_comments(clean_content);
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stripped) {
                if let Some(ref s) = schema {
                    let lines: Vec<&str> = clean_content.lines().collect();
                    if let Some(obj) = val.as_object() {
                        for (key, nested) in obj {
                            let is_dt = key.starts_with("DT_") || key.contains("DataTable");
                            let is_bp = key.starts_with("BP_") || key.ends_with("_C");
                            if (is_dt || is_bp) && nested.is_object() {
                                let (line_num, col_num) = find_key_position(&lines, key);

                                let (status, reason, suggestion) =
                                    crate::commands::scanner::palschema::validate_palschema_table_with_usmap(
                                        key,
                                        s,
                                        sdk_index.as_ref(),
                                        Path::new(&game_path),
                                    );

                                if status == "broken_table" || status == "broken_class" {
                                    diagnostics.push(EditorDiagnostic {
                                        line: line_num,
                                        column: col_num,
                                        end_line: line_num,
                                        end_column: col_num + key.len() as u32,
                                        severity: "error".to_string(),
                                        message: reason,
                                        target: key.clone(),
                                        suggestion,
                                        category: "palschema".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(diagnostics)
}

#[tauri::command]
pub async fn scan_workspace_problems(
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, Vec<EditorDiagnostic>>, String> {
    let (mod_info, program_path, game_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let mod_info = data.mods.iter().find(|m| m.id == mod_id).cloned().ok_or("Mod not found")?;
        (mod_info, data.settings.program_path.clone(), data.settings.game_path.clone())
    };

    let files = crate::commands::config_commands::list_mod_files(mod_id, state.clone())?;
    let mut results: HashMap<String, Vec<EditorDiagnostic>> = HashMap::new();

    let usmap_path = get_active_usmap_path(&program_path);
    let schema = if usmap_path.exists() {
        parse_usmap_file(&usmap_path).ok()
    } else {
        None
    };
    let sdk_index = get_or_load_sdk_index(&program_path, &game_path);

    for file_path in files {
        let ext = Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if ext != "lua" && ext != "json" && ext != "jsonc" {
            continue;
        }

        let full_path = match crate::commands::config_commands::get_full_mod_file_path(&mod_info, &file_path) {
            Ok(p) => p,
            Err(_) => continue,
        };

        if !full_path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if content.len() > 2 * 1024 * 1024 {
            continue;
        }

        let mut diagnostics = Vec::new();
        if ext == "lua" {
            lint_lua_syntax(&content, &mut diagnostics);

            let lines: Vec<&str> = content.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let line_num = (i + 1) as u32;
                let mut search_idx = 0;
                while let Some(start_pos) = find_hook_start(line, search_idx) {
                    if let Some((target, _quote_char, col_offset)) = extract_string_literal(line, start_pos) {
                        let trimmed = target.trim();
                        let is_incomplete = trimmed.is_empty()
                            || trimmed == "/"
                            || trimmed == "/Script"
                            || trimmed == "/Script/"
                            || trimmed.ends_with('/')
                            || (trimmed.ends_with(':') && !trimmed.contains("__DelegateSignature"));

                        if !is_incomplete && (trimmed.contains('/') || trimmed.contains(':') || trimmed.starts_with("Pal") || trimmed.starts_with("APal") || trimmed.starts_with("UPal")) {
                            let is_builtin_engine = trimmed.starts_with("/Script/CoreUObject.")
                                || trimmed.starts_with("/Script/SlateCore.")
                                || trimmed.starts_with("/Script/Slate.");

                            if !is_builtin_engine {
                                if let Some(ref s) = schema {
                                    let (_clean_class, _clean_func, status, reason, suggestion) =
                                            crate::commands::scanner::hook_validator::validate_hook_with_usmap(
                                            trimmed,
                                            s,
                                            sdk_index.as_ref(),
                                        );

                                    if status == "broken_class" || status == "broken_function" {
                                        let severity = if status == "broken_class" { "error" } else { "warning" };
                                        diagnostics.push(EditorDiagnostic {
                                            line: line_num,
                                            column: col_offset as u32,
                                            end_line: line_num,
                                            end_column: (col_offset + trimmed.len()) as u32,
                                            severity: severity.to_string(),
                                            message: reason,
                                            target: trimmed.to_string(),
                                            suggestion,
                                            category: "ue4ss".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                        search_idx = start_pos + target.len() + 2;
                    } else {
                        break;
                    }
                }
            }
        } else if ext == "json" || ext == "jsonc" {
            let clean_content = content.strip_prefix("\u{feff}").unwrap_or(&content);
            lint_json_syntax(clean_content, &mut diagnostics);
        }

        if !diagnostics.is_empty() {
            results.insert(file_path, diagnostics);
        }
    }

    Ok(results)
}

pub fn find_hook_start(line: &str, start_idx: usize) -> Option<usize> {
    if start_idx >= line.len() {
        return None;
    }
    let sub = &line[start_idx..];
    let lower = sub.to_ascii_lowercase();

    let keywords = ["registerhook(", "notifyonnewobject(", "staticfindobject(", ":registerhook(", "findfirstof(", "findallof("];
    let mut earliest: Option<usize> = None;

    for kw in &keywords {
        if let Some(pos) = lower.find(kw) {
            let actual_pos = start_idx + pos + kw.len();
            if earliest.is_none() || actual_pos < earliest.unwrap() {
                earliest = Some(actual_pos);
            }
        }
    }

    earliest
}

pub fn extract_string_literal(line: &str, start_pos: usize) -> Option<(String, char, usize)> {
    if start_pos >= line.len() {
        return None;
    }

    let remaining = &line[start_pos..];
    let mut chars = remaining.char_indices();

    let mut quote_char = None;
    let mut quote_col_offset = start_pos;

    for (idx, c) in chars.by_ref() {
        if c == '"' || c == '\'' {
            quote_char = Some(c);
            quote_col_offset = start_pos + idx + 1;
            break;
        } else if !c.is_whitespace() {
            return None;
        }
    }

    let q = quote_char?;
    let mut target = String::new();

    for (_idx, c) in chars {
        if c == q {
            return Some((target, q, quote_col_offset));
        }
        target.push(c);
    }

    None
}
