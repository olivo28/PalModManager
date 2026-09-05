use std::collections::HashMap;
use std::path::Path;
use tauri::State;
use crate::state::AppState;
use crate::usmap::{get_or_load_sdk_index, get_or_load_schema};
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

    let schema = get_or_load_schema(&program_path);
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
                                        sdk_index.as_deref(),
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

        // -------------------------------------------------------------
        // Layer 3: UE4SS Deprecated APIs & Blind pcall Anti-Patterns
        // -------------------------------------------------------------
        lint_deprecated_and_antipatterns(&content, &mut diagnostics);
    } else if ext == "json" || ext == "jsonc" {
        // -------------------------------------------------------------
        // Layer 1: Universal JSON Syntax & Duplicate Key Detection
        // -------------------------------------------------------------
        let clean_content = content.strip_prefix("\u{feff}").unwrap_or(&content);
        lint_json_syntax(clean_content, &mut diagnostics);

        // -------------------------------------------------------------
        // Layer 2: PalSchema DataTables & USMAP Validation (PalSchema files only)
        // -------------------------------------------------------------
        let clean_path = file_path.replace('\\', "/");
        let clean_path_lower = clean_path.to_lowercase();
        let is_palschema_file = clean_path.starts_with("[PalSchema]")
            || clean_path_lower.contains("/palschema/")
            || clean_path_lower.contains("palschema/mods/")
            || clean_path_lower.ends_with(".palschema.json")
            || clean_path_lower.ends_with(".palschema.jsonc");

        let is_translation_file = clean_path_lower.contains("/translations/") || clean_path_lower.starts_with("translations/");

        if is_palschema_file && !clean_content.trim().is_empty() {
            let stripped = crate::commands::scanner::utils::strip_jsonc_comments(clean_content);
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stripped) {
                let lines: Vec<&str> = clean_content.lines().collect();
                if let Some(obj) = val.as_object() {
                    for (key, nested) in obj {
                        let is_dt = key.starts_with("DT_") || key.contains("DataTable");
                        let is_bp = key.starts_with("BP_") || key.ends_with("_C");
                        if (is_dt || is_bp) && nested.is_object() {
                            let (line_num, col_num) = find_key_position(&lines, key);

                            // For translation files, check if valid PalSchema localization table
                            if is_translation_file {
                                let is_valid_text = crate::usmap::is_valid_palschema_table_name(key, &program_path, &game_path);
                                if !is_valid_text && !key.ends_with("Text") && !key.ends_with("TextData") {
                                    diagnostics.push(EditorDiagnostic {
                                        line: line_num,
                                        column: col_num,
                                        end_line: line_num,
                                        end_column: col_num + key.len() as u32,
                                        severity: "warning".to_string(),
                                        message: format!("Unrecognized translation table '{}'. PalSchema translations typically target DT_*Text tables.", key),
                                        target: key.clone(),
                                        suggestion: Some("DT_ItemNameText".to_string()),
                                        category: "palschema".to_string(),
                                    });
                                }
                                continue;
                            }

                            if let Some(ref s) = schema {
                                let (status, reason, suggestion) =
                                    crate::commands::scanner::palschema::validate_palschema_table_with_usmap(
                                        key,
                                        s,
                                        sdk_index.as_deref(),
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

    let schema = get_or_load_schema(&program_path);
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
                                            sdk_index.as_deref(),
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

            // Layer 3: Deprecated UE4SS APIs & Blind pcall Anti-Patterns
            lint_deprecated_and_antipatterns(&content, &mut diagnostics);
        } else if ext == "json" || ext == "jsonc" {
            let clean_content = content.strip_prefix("\u{feff}").unwrap_or(&content);
            lint_json_syntax(clean_content, &mut diagnostics);

            let clean_path = file_path.replace('\\', "/");
            let clean_path_lower = clean_path.to_lowercase();
            let is_palschema_file = clean_path.starts_with("[PalSchema]")
                || clean_path_lower.contains("/palschema/")
                || clean_path_lower.contains("palschema/mods/")
                || clean_path_lower.ends_with(".palschema.json")
                || clean_path_lower.ends_with(".palschema.jsonc");

            let is_translation_file = clean_path_lower.contains("/translations/") || clean_path_lower.starts_with("translations/");

            if is_palschema_file && !clean_content.trim().is_empty() {
                let stripped = crate::commands::scanner::utils::strip_jsonc_comments(clean_content);
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stripped) {
                    let lines: Vec<&str> = clean_content.lines().collect();
                    if let Some(obj) = val.as_object() {
                        for (key, nested) in obj {
                            let is_dt = key.starts_with("DT_") || key.contains("DataTable");
                            let is_bp = key.starts_with("BP_") || key.ends_with("_C");
                            if (is_dt || is_bp) && nested.is_object() {
                                let (line_num, col_num) = find_key_position(&lines, key);

                                if is_translation_file {
                                    let is_valid_text = crate::usmap::is_valid_palschema_table_name(key, &program_path, &game_path);
                                    if !is_valid_text && !key.ends_with("Text") && !key.ends_with("TextData") {
                                        diagnostics.push(EditorDiagnostic {
                                            line: line_num,
                                            column: col_num,
                                            end_line: line_num,
                                            end_column: col_num + key.len() as u32,
                                            severity: "warning".to_string(),
                                            message: format!("Unrecognized translation table '{}'. PalSchema translations typically target DT_*Text tables.", key),
                                            target: key.clone(),
                                            suggestion: Some("DT_ItemNameText".to_string()),
                                            category: "palschema".to_string(),
                                        });
                                    }
                                    continue;
                                }

                                if let Some(ref s) = schema {
                                    let (status, reason, suggestion) =
                                        crate::commands::scanner::palschema::validate_palschema_table_with_usmap(
                                            key,
                                            s,
                                            sdk_index.as_deref(),
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

    let keywords = [
        "registerhook",
        "notifyonnewobject",
        "staticfindobject",
        "findfirstof",
        "findallof",
    ];
    let mut earliest: Option<usize> = None;

    for kw in &keywords {
        let mut pos_search = 0;
        while let Some(rel_pos) = lower[pos_search..].find(kw) {
            let pos = pos_search + rel_pos;
            let after_pos = pos + kw.len();

            // Ensure boundary before keyword (avoid false positives on MyCustomRegisterHook)
            let is_valid_boundary = if pos == 0 {
                true
            } else {
                let prev = lower.as_bytes()[pos - 1];
                !prev.is_ascii_alphanumeric() && prev != b'_'
            };

            if is_valid_boundary {
                let actual_pos = start_idx + after_pos;
                if earliest.is_none() || actual_pos < earliest.unwrap() {
                    earliest = Some(actual_pos);
                }
            }
            pos_search = after_pos;
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
        } else if c == '-' && remaining[idx..].starts_with("--") {
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

pub fn mask_strings_and_comments(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        let next_c = if i + 1 < len { Some(chars[i + 1]) } else { None };

        if in_single {
            if escaped {
                escaped = false;
                out.push(' ');
            } else if c == '\\' {
                escaped = true;
                out.push(' ');
            } else if c == '\'' {
                in_single = false;
                out.push(' ');
            } else {
                out.push(' ');
            }
        } else if in_double {
            if escaped {
                escaped = false;
                out.push(' ');
            } else if c == '\\' {
                escaped = true;
                out.push(' ');
            } else if c == '"' {
                in_double = false;
                out.push(' ');
            } else {
                out.push(' ');
            }
        } else {
            if c == '-' && next_c == Some('-') {
                while i < len {
                    out.push(' ');
                    i += 1;
                }
                break;
            } else if c == '\'' {
                in_single = true;
                out.push(' ');
            } else if c == '"' {
                in_double = true;
                out.push(' ');
            } else {
                out.push(c);
            }
        }
        i += 1;
    }

    out
}

pub fn lint_deprecated_and_antipatterns(content: &str, diagnostics: &mut Vec<EditorDiagnostic>) {
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let line_num = (i + 1) as u32;
        let trimmed = line.trim();

        // Skip full comment lines
        if trimmed.starts_with("--") {
            continue;
        }

        // Mask out string literals and inline comments so we only inspect active code tokens
        let code_only = mask_strings_and_comments(line);
        let code_trimmed = code_only.trim();

        // -------------------------------------------------------------
        // 1. Deprecated UE4SS APIs
        // -------------------------------------------------------------
        let deprecated_rules = [
            (
                "loopasync",
                "LoopAsync",
                "LoopAsync is deprecated in UE4SS. Use LoopInGameThreadWithDelay from the Delayed Action System for cancellable and pauseable timers.",
                Some("LoopInGameThreadWithDelay"),
            ),
            (
                "executeasync",
                "ExecuteAsync",
                "ExecuteAsync is deprecated in UE4SS. Use ExecuteInGameThread or ExecuteInGameThreadWithDelay for thread safety.",
                Some("ExecuteInGameThreadWithDelay"),
            ),
            (
                "executewithdelay",
                "ExecuteWithDelay",
                "ExecuteWithDelay is deprecated in UE4SS. Use ExecuteInGameThreadWithDelay from the Delayed Action System.",
                Some("ExecuteInGameThreadWithDelay"),
            ),
            (
                "getchartarray",
                "GetCharTArray",
                "GetCharTArray() is deprecated in UE4SS. Use GetCharArray() instead.",
                Some("GetCharArray"),
            ),
            (
                "foreachproperty",
                "ForEachProperty",
                "ForEachProperty is deprecated on UStruct/UClass in UE4SS. Use direct field access (GetPropertyValue / __index) or metadata iteration.",
                None,
            ),
        ];

        let line_lower = code_only.to_ascii_lowercase();
        for (token_lower, display_name, reason, suggestion) in &deprecated_rules {
            if let Some(pos) = line_lower.find(token_lower) {
                let is_prefix_char = pos > 0 && code_only.chars().nth(pos - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                let after_idx = pos + token_lower.len();
                let is_suffix_char = after_idx < code_only.len() && code_only.chars().nth(after_idx).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);

                if !is_prefix_char && !is_suffix_char {
                    diagnostics.push(EditorDiagnostic {
                        line: line_num,
                        column: (pos + 1) as u32,
                        end_line: line_num,
                        end_column: (pos + 1 + display_name.len()) as u32,
                        severity: "warning".to_string(),
                        message: reason.to_string(),
                        target: display_name.to_string(),
                        suggestion: suggestion.map(|s| s.to_string()),
                        category: "ue4ss_deprecated".to_string(),
                    });
                }
            }
        }

        // -------------------------------------------------------------
        // 2. Anti-Pattern: Blind pcall / Error Silencing
        // -------------------------------------------------------------
        // Case A: Orphan `pcall(...)` statement without variable assignment
        let is_pcall_start = code_trimmed.starts_with("pcall(") || code_trimmed.starts_with("pcall ");
        if is_pcall_start {
            if let Some(pos) = code_only.find("pcall") {
                diagnostics.push(EditorDiagnostic {
                    line: line_num,
                    column: (pos + 1) as u32,
                    end_line: line_num,
                    end_column: (pos + 6) as u32,
                    severity: "warning".to_string(),
                    message: "Blind pcall detected: suppressing errors silences runtime crashes and makes mods impossible to debug. Capture 'local ok, err = pcall(...)' and log errors, or use direct 'if obj and obj:IsValid()' checks.".to_string(),
                    target: "pcall".to_string(),
                    suggestion: Some("local ok, err = pcall".to_string()),
                    category: "anti_pattern".to_string(),
                });
            }
        } else if code_trimmed.starts_with("local ") && code_trimmed.contains("= pcall") {
            // Case B: `local ok = pcall(` capturing only 1 variable (no comma before `=`)
            if let Some(eq_idx) = code_trimmed.find('=') {
                let vars_part = code_trimmed[6..eq_idx].trim();
                if !vars_part.contains(',') {
                    if let Some(pos) = code_only.find("pcall") {
                        diagnostics.push(EditorDiagnostic {
                            line: line_num,
                            column: (pos + 1) as u32,
                            end_line: line_num,
                            end_column: (pos + 6) as u32,
                            severity: "warning".to_string(),
                            message: format!("Unhandled pcall error: only '{}' is captured while the error message is discarded. Capture 'local {}, err = pcall(...)' and log failures when '{} == false'.", vars_part, vars_part, vars_part),
                            target: "pcall".to_string(),
                            suggestion: Some(format!("local {}, err = pcall", vars_part)),
                            category: "anti_pattern".to_string(),
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_hook_start_and_extract_string_literal() {
        let line1 = r#"local ok, err = pcall(RegisterHook, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter", function(self) end)"#;
        let start1 = find_hook_start(line1, 0).expect("Should find hook start");
        let (target1, _, _) = extract_string_literal(line1, start1).expect("Should extract string literal");
        assert_eq!(target1, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter");

        let line2 = r#"RegisterHook('/Script/Engine.Actor:K2_DestroyActor', callback)"#;
        let start2 = find_hook_start(line2, 0).expect("Should find hook start");
        let (target2, _, _) = extract_string_literal(line2, start2).expect("Should extract string literal");
        assert_eq!(target2, "/Script/Engine.Actor:K2_DestroyActor");

        let line3 = r#"xpcall(RegisterHook, debug.traceback, "/Script/Pal.PalCharacter:Die", on_die)"#;
        let start3 = find_hook_start(line3, 0).expect("Should find hook start");
        let (target3, _, _) = extract_string_literal(line3, start3).expect("Should extract string literal");
        assert_eq!(target3, "/Script/Pal.PalCharacter:Die");
    }
}
