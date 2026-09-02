use crate::state::AppState;
use crate::usmap::{get_or_load_sdk_index, sync::get_active_usmap_path, parser::parse_usmap_file};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorDiagnostic {
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub severity: String, // "error" | "warning" | "info"
    pub message: String,
    pub target: String,
    pub suggestion: Option<String>,
    pub category: String, // "ue4ss" | "palschema" | "syntax"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorCompletion {
    pub label: String,
    pub insert_text: String,
    pub kind: String, // "hook" | "class" | "function" | "delegate" | "table" | "struct" | "api" | "module"
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

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
                                    crate::commands::scanner_commands::validate_hook_with_usmap(
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
            let stripped = crate::commands::scanner_commands::strip_jsonc_comments(clean_content);
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
                                    crate::commands::scanner_commands::validate_palschema_table_with_usmap(
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
                                        crate::commands::scanner_commands::validate_hook_with_usmap(
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

#[derive(Debug, PartialEq)]
enum LuaLexState {
    Code,
    SingleLineComment,
    MultiLineComment(usize),
    SingleQuoteString,
    DoubleQuoteString,
    MultiLineString(usize),
}

fn lint_lua_syntax(content: &str, diagnostics: &mut Vec<EditorDiagnostic>) {
    let mut block_stack: Vec<(&'static str, u32, u32)> = Vec::new();
    let mut paren_stack: Vec<(char, u32, u32)> = Vec::new();

    let mut state = LuaLexState::Code;
    let mut is_escaped = false;
    let mut in_loop_header = false;
    let mut line_num: u32 = 1;
    let mut col_num: u32 = 1;

    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut idx = 0;

    let mut current_token = String::new();
    let mut token_start_col: u32 = 1;

    while idx < len {
        let c = chars[idx];
        let next_c = if idx + 1 < len { Some(chars[idx + 1]) } else { None };

        match state {
            LuaLexState::Code => {
                // Helper to flush current_token before state switches
                let flush_token = |t: &mut String, l: u32, col: u32, blocks: &mut Vec<(&'static str, u32, u32)>, loop_hdr: &mut bool, diags: &mut Vec<EditorDiagnostic>| {
                    if !t.is_empty() {
                        match t.as_str() {
                            "function" => {
                                *loop_hdr = false;
                                blocks.push(("function", l, col));
                            }
                            "if" => {
                                *loop_hdr = false;
                                blocks.push(("if", l, col));
                            }
                            "for" => {
                                *loop_hdr = true;
                                blocks.push(("for", l, col));
                            }
                            "while" => {
                                *loop_hdr = true;
                                blocks.push(("while", l, col));
                            }
                            "repeat" => {
                                *loop_hdr = false;
                                blocks.push(("repeat", l, col));
                            }
                            "do" => {
                                if *loop_hdr {
                                    *loop_hdr = false;
                                } else {
                                    blocks.push(("do", l, col));
                                }
                            }
                            "until" => {
                                *loop_hdr = false;
                                if let Some((top, _, _)) = blocks.last() {
                                    if *top == "repeat" {
                                        blocks.pop();
                                    }
                                }
                            }
                            "end" => {
                                *loop_hdr = false;
                                if let Some((top, _, _)) = blocks.pop() {
                                    if top == "repeat" {
                                        diags.push(EditorDiagnostic {
                                            line: l,
                                            column: col,
                                            end_line: l,
                                            end_column: col + 3,
                                            severity: "error".to_string(),
                                            message: "'repeat' block must be closed with 'until', not 'end'".to_string(),
                                            target: "end".to_string(),
                                            suggestion: Some("until".to_string()),
                                            category: "syntax".to_string(),
                                        });
                                    }
                                } else {
                                    diags.push(EditorDiagnostic {
                                        line: l,
                                        column: col,
                                        end_line: l,
                                        end_column: col + 3,
                                        severity: "error".to_string(),
                                        message: "Unexpected 'end' with no matching block to close".to_string(),
                                        target: "end".to_string(),
                                        suggestion: None,
                                        category: "syntax".to_string(),
                                    });
                                }
                            }
                            _ => {}
                        }
                        t.clear();
                    }
                };

                // Check comment start
                if c == '-' && next_c == Some('-') {
                    flush_token(&mut current_token, line_num, token_start_col, &mut block_stack, &mut in_loop_header, diagnostics);
                    // Check if multi-line comment: --[[ or --[=[
                    if idx + 2 < len && chars[idx + 2] == '[' {
                        let mut eq_count = 0;
                        let mut p = idx + 3;
                        while p < len && chars[p] == '=' {
                            eq_count += 1;
                            p += 1;
                        }
                        if p < len && chars[p] == '[' {
                            state = LuaLexState::MultiLineComment(eq_count);
                            idx = p;
                            continue;
                        }
                    }
                    state = LuaLexState::SingleLineComment;
                    idx += 1;
                    continue;
                }

                // Check string start
                if c == '\'' {
                    flush_token(&mut current_token, line_num, token_start_col, &mut block_stack, &mut in_loop_header, diagnostics);
                    state = LuaLexState::SingleQuoteString;
                    is_escaped = false;
                    idx += 1;
                    col_num += 1;
                    continue;
                }
                if c == '"' {
                    flush_token(&mut current_token, line_num, token_start_col, &mut block_stack, &mut in_loop_header, diagnostics);
                    state = LuaLexState::DoubleQuoteString;
                    is_escaped = false;
                    idx += 1;
                    col_num += 1;
                    continue;
                }
                if c == '[' && (next_c == Some('[') || next_c == Some('=')) {
                    flush_token(&mut current_token, line_num, token_start_col, &mut block_stack, &mut in_loop_header, diagnostics);
                    let mut eq_count = 0;
                    let mut p = idx + 1;
                    while p < len && chars[p] == '=' {
                        eq_count += 1;
                        p += 1;
                    }
                    if p < len && chars[p] == '[' {
                        state = LuaLexState::MultiLineString(eq_count);
                        idx = p;
                        continue;
                    }
                }

                // Check C-style operators
                if c == '!' && next_c == Some('=') {
                    diagnostics.push(EditorDiagnostic {
                        line: line_num,
                        column: col_num,
                        end_line: line_num,
                        end_column: col_num + 2,
                        severity: "warning".to_string(),
                        message: "Lua uses '~=' for inequality instead of '!='".to_string(),
                        target: "!=".to_string(),
                        suggestion: Some("~=".to_string()),
                        category: "syntax".to_string(),
                    });
                }
                if c == '&' && next_c == Some('&') {
                    diagnostics.push(EditorDiagnostic {
                        line: line_num,
                        column: col_num,
                        end_line: line_num,
                        end_column: col_num + 2,
                        severity: "warning".to_string(),
                        message: "Lua uses 'and' instead of '&&'".to_string(),
                        target: "&&".to_string(),
                        suggestion: Some("and".to_string()),
                        category: "syntax".to_string(),
                    });
                }
                if c == '|' && next_c == Some('|') {
                    diagnostics.push(EditorDiagnostic {
                        line: line_num,
                        column: col_num,
                        end_line: line_num,
                        end_column: col_num + 2,
                        severity: "warning".to_string(),
                        message: "Lua uses 'or' instead of '||'".to_string(),
                        target: "||".to_string(),
                        suggestion: Some("or".to_string()),
                        category: "syntax".to_string(),
                    });
                }

                // Check Parentheses & Brackets
                match c {
                    '(' | '[' | '{' => {
                        paren_stack.push((c, line_num, col_num));
                    }
                    ')' => {
                        if let Some((open_c, o_line, _)) = paren_stack.pop() {
                            if open_c != '(' {
                                diagnostics.push(EditorDiagnostic {
                                    line: line_num,
                                    column: col_num,
                                    end_line: line_num,
                                    end_column: col_num + 1,
                                    severity: "error".to_string(),
                                    message: format!("Mismatched closing ')' (opened with '{}' at line {})", open_c, o_line),
                                    target: ")".to_string(),
                                    suggestion: None,
                                    category: "syntax".to_string(),
                                });
                            }
                        } else {
                            diagnostics.push(EditorDiagnostic {
                                line: line_num,
                                column: col_num,
                                end_line: line_num,
                                end_column: col_num + 1,
                                severity: "error".to_string(),
                                message: "Unmatched closing ')'".to_string(),
                                target: ")".to_string(),
                                suggestion: None,
                                category: "syntax".to_string(),
                            });
                        }
                    }
                    ']' => {
                        if let Some((open_c, o_line, _)) = paren_stack.pop() {
                            if open_c != '[' {
                                diagnostics.push(EditorDiagnostic {
                                    line: line_num,
                                    column: col_num,
                                    end_line: line_num,
                                    end_column: col_num + 1,
                                    severity: "error".to_string(),
                                    message: format!("Mismatched closing ']' (opened with '{}' at line {})", open_c, o_line),
                                    target: "]".to_string(),
                                    suggestion: None,
                                    category: "syntax".to_string(),
                                });
                            }
                        } else {
                            diagnostics.push(EditorDiagnostic {
                                line: line_num,
                                column: col_num,
                                end_line: line_num,
                                end_column: col_num + 1,
                                severity: "error".to_string(),
                                message: "Unmatched closing ']'".to_string(),
                                target: "]".to_string(),
                                suggestion: None,
                                category: "syntax".to_string(),
                            });
                        }
                    }
                    '}' => {
                        if let Some((open_c, o_line, _)) = paren_stack.pop() {
                            if open_c != '{' {
                                diagnostics.push(EditorDiagnostic {
                                    line: line_num,
                                    column: col_num,
                                    end_line: line_num,
                                    end_column: col_num + 1,
                                    severity: "error".to_string(),
                                    message: format!("Mismatched closing '}}' (opened with '{}' at line {})", open_c, o_line),
                                    target: "}".to_string(),
                                    suggestion: None,
                                    category: "syntax".to_string(),
                                });
                            }
                        } else {
                            diagnostics.push(EditorDiagnostic {
                                line: line_num,
                                column: col_num,
                                end_line: line_num,
                                end_column: col_num + 1,
                                severity: "error".to_string(),
                                message: "Unmatched closing '}'".to_string(),
                                target: "}".to_string(),
                                suggestion: None,
                                category: "syntax".to_string(),
                            });
                        }
                    }
                    _ => {}
                }

                // Check identifier tokens & block keywords
                if c.is_alphanumeric() || c == '_' {
                    if current_token.is_empty() {
                        token_start_col = col_num;
                    }
                    current_token.push(c);
                } else {
                    flush_token(&mut current_token, line_num, token_start_col, &mut block_stack, &mut in_loop_header, diagnostics);
                }
            }
            LuaLexState::SingleLineComment => {
                if c == '\n' {
                    state = LuaLexState::Code;
                }
            }
            LuaLexState::MultiLineComment(eq_count) => {
                if c == ']' {
                    let mut count = 0;
                    let mut p = idx + 1;
                    while p < len && chars[p] == '=' {
                        count += 1;
                        p += 1;
                    }
                    if count == eq_count && p < len && chars[p] == ']' {
                        state = LuaLexState::Code;
                        idx = p;
                    }
                }
            }
            LuaLexState::SingleQuoteString => {
                if is_escaped {
                    is_escaped = false;
                } else if c == '\\' {
                    is_escaped = true;
                } else if c == '\'' {
                    state = LuaLexState::Code;
                }
            }
            LuaLexState::DoubleQuoteString => {
                if is_escaped {
                    is_escaped = false;
                } else if c == '\\' {
                    is_escaped = true;
                } else if c == '"' {
                    state = LuaLexState::Code;
                }
            }
            LuaLexState::MultiLineString(eq_count) => {
                if c == ']' {
                    let mut count = 0;
                    let mut p = idx + 1;
                    while p < len && chars[p] == '=' {
                        count += 1;
                        p += 1;
                    }
                    if count == eq_count && p < len && chars[p] == ']' {
                        state = LuaLexState::Code;
                        idx = p;
                    }
                }
            }
        }

        if c == '\n' {
            line_num += 1;
            col_num = 1;
        } else {
            col_num += 1;
        }
        idx += 1;
    }

    // Unclosed parens/brackets
    for (c, l, col) in paren_stack {
        diagnostics.push(EditorDiagnostic {
            line: l,
            column: col,
            end_line: l,
            end_column: col + 1,
            severity: "error".to_string(),
            message: format!("Unclosed opening '{}'", c),
            target: c.to_string(),
            suggestion: None,
            category: "syntax".to_string(),
        });
    }

    // Unclosed blocks
    for (block_type, l, col) in block_stack {
        diagnostics.push(EditorDiagnostic {
            line: l,
            column: col,
            end_line: l,
            end_column: col + block_type.len() as u32,
            severity: "error".to_string(),
            message: format!("Unclosed '{}' block (missing 'end' keyword)", block_type),
            target: block_type.to_string(),
            suggestion: None,
            category: "syntax".to_string(),
        });
    }
}

fn lint_json_syntax(clean_content: &str, diagnostics: &mut Vec<EditorDiagnostic>) {
    if clean_content.trim().is_empty() {
        return;
    }

    let stripped = crate::commands::scanner_commands::strip_jsonc_comments(clean_content);
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

fn find_key_position(lines: &[&str], key: &str) -> (u32, u32) {
    let pattern = format!("\"{}\"", key);
    for (i, line) in lines.iter().enumerate() {
        if let Some(col) = line.find(&pattern) {
            return ((i + 1) as u32, (col + 1) as u32);
        }
    }
    (1, 1)
}

#[tauri::command]
pub async fn get_editor_completions(
    file_path: String,
    query: String,
    line_prefix: String,
    state: State<'_, AppState>,
) -> Result<Vec<EditorCompletion>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();

    let mut completions = Vec::new();
    let ext = Path::new(&file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let q_lower = query.trim().to_ascii_lowercase();
    let prefix_lower = line_prefix.to_ascii_lowercase();

    let usmap_path = get_active_usmap_path(&program_path);
    let schema = if usmap_path.exists() {
        parse_usmap_file(&usmap_path).ok()
    } else {
        None
    };
    let sdk_index = get_or_load_sdk_index(&program_path, &game_path);

    if ext == "lua" {
        let is_in_path = prefix_lower.contains("registerhook")
            || prefix_lower.contains("notifyonnewobject")
            || prefix_lower.contains("staticfindobject")
            || prefix_lower.contains("findfirstof")
            || prefix_lower.contains("findallof")
            || prefix_lower.contains("/script/")
            || prefix_lower.contains("/game/")
            || q_lower.starts_with('/')
            || q_lower.starts_with("/script")
            || q_lower.starts_with("dt_")
            || q_lower.contains('.');

        if is_in_path {
            let mut seen = HashSet::new();

            // Check if user is typing a DataTable in Lua e.g. StaticFindObject("...DT_PalCharacterParameter")
            if q_lower.starts_with("dt_") {
                if let Some(ref s) = schema {
                    for name in &s.names {
                        let nl = name.to_ascii_lowercase();
                        if nl.starts_with("dt_") && (q_lower.is_empty() || nl.contains(&q_lower)) {
                            if seen.insert(name.clone()) {
                                completions.push(EditorCompletion {
                                    label: name.clone(),
                                    insert_text: name.clone(),
                                    kind: "table".to_string(),
                                    detail: Some("PalSchema DataTable".to_string()),
                                    documentation: Some(format!("Palworld reflection DataTable: {}", name)),
                                });
                            }
                        }
                        if completions.len() >= 60 {
                            break;
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // HIERARCHICAL LEVEL 1: Root Namespaces (/ ➔ /Script/, /Game/)
            // -----------------------------------------------------------------
            if q_lower == "/" || q_lower.is_empty() {
                completions.push(EditorCompletion {
                    label: "/Script/".to_string(),
                    insert_text: "/Script/".to_string(),
                    kind: "module".to_string(),
                    detail: Some("C++ Engine & Game Modules".to_string()),
                    documentation: Some("Root namespace for native C++ modules, classes, and managers.".to_string()),
                });
                completions.push(EditorCompletion {
                    label: "/Game/".to_string(),
                    insert_text: "/Game/".to_string(),
                    kind: "module".to_string(),
                    detail: Some("Unreal Engine Blueprint Assets".to_string()),
                    documentation: Some("Root namespace for cooked game Blueprints and UI widgets.".to_string()),
                });
            }

            // -----------------------------------------------------------------
            // HIERARCHICAL LEVEL 2: Modules (/Script/ ➔ Pal., UMG., Engine.)
            // -----------------------------------------------------------------
            if let Some(ref sdk) = sdk_index {
                let is_level_2 = (q_lower.starts_with("/script/") && !q_lower.contains('.'))
                    || q_lower == "/script"
                    || q_lower == "/script/";

                if is_level_2 {
                    let mut sorted_modules: Vec<&String> = sdk.modules.iter().collect();
                    sorted_modules.sort_by(|a, b| {
                        let is_pri = |m: &str| m == "Pal" || m == "UMG" || m == "Engine" || m == "CoreUObject";
                        let a_p = if is_pri(a) { 0 } else { 1 };
                        let b_p = if is_pri(b) { 0 } else { 1 };
                        a_p.cmp(&b_p).then_with(|| a.cmp(b))
                    });

                    let mod_filter = q_lower.trim_start_matches("/script/").trim_start_matches("/script");
                    for mod_name in sorted_modules {
                        let ml = mod_name.to_ascii_lowercase();
                        if mod_filter.is_empty() || ml.starts_with(mod_filter) || ml.contains(mod_filter) {
                            let script_module = format!("/Script/{}.", mod_name);
                            if seen.insert(script_module.clone()) {
                                completions.push(EditorCompletion {
                                    label: format!("/Script/{}.", mod_name),
                                    insert_text: script_module,
                                    kind: "module".to_string(),
                                    detail: Some(format!("Module ({}.hpp)", mod_name)),
                                    documentation: Some(format!("C++ SDK Module: {}\nContains native classes and reflection structs.", mod_name)),
                                });
                            }
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // HIERARCHICAL LEVEL 4: Methods & Delegates on a Specific Class (Class:)
            // -----------------------------------------------------------------
            if q_lower.contains(':') {
                if let Some((class_part, func_query)) = q_lower.split_once(':') {
                    let clean_cls = class_part
                        .split('.')
                        .last()
                        .unwrap_or(class_part)
                        .split('/')
                        .last()
                        .unwrap_or(class_part)
                        .trim();

                    if let Some(ref sdk) = sdk_index {
                        if let Some(cinfo) = sdk.find_class(clean_cls) {
                            let all_funcs = sdk.get_all_class_functions_and_properties(clean_cls);
                            for func in all_funcs {
                                let fl = func.to_ascii_lowercase();
                                if func_query.is_empty() || fl.contains(func_query) {
                                    let is_delegate = func.ends_with("__DelegateSignature") || func.contains("Delegate");
                                    let full_sig = format!("{}:{}", class_part, func);
                                    if seen.insert(full_sig.clone()) {
                                        completions.push(EditorCompletion {
                                            label: format!("{}:{}", cinfo.clean_name, func),
                                            insert_text: full_sig,
                                            kind: if is_delegate { "delegate".to_string() } else { "function".to_string() },
                                            detail: Some(if is_delegate { "Delegate Signature".to_string() } else { "Class Method".to_string() }),
                                            documentation: Some(format!("Member of class {}\nModule: {}", cinfo.clean_name, cinfo.module_name)),
                                        });
                                    }
                                }
                                if completions.len() >= 80 {
                                    break;
                                }
                            }
                        }
                    }
                }
            } else {
                // -----------------------------------------------------------------
                // HIERARCHICAL LEVEL 3: Classes in Specific Module (/Script/Pal. ➔ Classes)
                // -----------------------------------------------------------------
                let (target_module, class_query) = if q_lower.starts_with("/script/") {
                    let rest = q_lower.trim_start_matches("/script/");
                    if let Some((m, c)) = rest.split_once('.') {
                        (Some(m.to_string()), c.to_string())
                    } else {
                        (None, rest.to_string())
                    }
                } else if let Some((m, c)) = q_lower.split_once('.') {
                    (Some(m.to_string()), c.to_string())
                } else {
                    (None, q_lower.clone())
                };

                if let Some(ref sdk) = sdk_index {
                    for (cname, cinfo) in &sdk.classes {
                        if let Some(ref tm) = target_module {
                            if !cinfo.module_name.eq_ignore_ascii_case(tm) && !cinfo.module_name.to_ascii_lowercase().starts_with(tm) {
                                continue;
                            }
                        }

                        let clean = &cinfo.clean_name;
                        let clean_lower = clean.to_ascii_lowercase();

                        if !class_query.is_empty() && !cname.contains(&class_query) && !clean_lower.contains(&class_query) {
                            continue;
                        }

                        let script_path = format!("/Script/{}.{}", cinfo.module_name, clean);
                        if seen.insert(script_path.clone()) {
                            completions.push(EditorCompletion {
                                label: format!("/Script/{}.{}", cinfo.module_name, clean),
                                insert_text: script_path,
                                kind: "class".to_string(),
                                detail: Some(format!("{}.hpp ({} funcs)", cinfo.module_name, cinfo.functions.len())),
                                documentation: Some(format!("Native C++ class {}\nModule: {}\nSuperclass: {}", clean, cinfo.module_name, cinfo.clean_super_class.as_deref().unwrap_or("UObject"))),
                            });
                        }

                        if completions.len() >= 100 {
                            break;
                        }
                    }
                }
            }
        } else {
            // Comprehensive UE4SS Global APIs & Hooks (Official UE4SS Lua API)
            let standard_apis = [
                // Hooking & Lifecycle
                ("RegisterHook", "RegisterHook(${1:\"/Script/ModuleName.ClassName:FunctionName\"}, function(${2:Context})\n\t$0\nend)", "UE4SS Function Hook\nRegisters a pre/post hook on an Unreal Engine function."),
                ("UnregisterHook", "UnregisterHook(${1:\"/Script/ModuleName.ClassName:FunctionName\"}, ${2:PreCallbackId}, ${3:PostCallbackId})", "UE4SS Unregister Hook\nRemoves an active function hook by ID."),
                ("RegisterBeginPlayHook", "RegisterBeginPlayHook(function(${1:Context})\n\t$0\nend)", "UE4SS BeginPlay Hook\nExecutes when AActor::BeginPlay is called by Unreal Engine."),
                ("RegisterCustomEvent", "RegisterCustomEvent(${1:\"EventName\"}, function(${2:Context})\n\t$0\nend)", "UE4SS Custom Event\nRegisters a custom event hook callback."),
                ("RegisterDefaultConstructorHook", "RegisterDefaultConstructorHook(${1:\"/Script/ModuleName.ClassName\"}, function(${2:Context})\n\t$0\nend)", "UE4SS Constructor Hook\nExecutes when an object's default C++ constructor runs."),
                ("RegisterProcessConsoleExecHook", "RegisterProcessConsoleExecHook(function(${1:Context})\n\t$0\nend)", "UE4SS Console Exec Hook\nIntercepts in-game console commands before execution."),
                ("RegisterConsoleCommandHandler", "RegisterConsoleCommandHandler(${1:\"CommandName\"}, function(${2:FullCommand}, ${3:Parameters}, ${4:OutputDevice})\n\t$0\n\treturn true\nend)", "UE4SS Custom Console Command\nRegisters a custom in-game console command handler."),
                ("RegisterKeyBind", "RegisterKeyBind(${1:Key.F1}, ${2:{ ModifierKey.CONTROL \\}}, function()\n\t$0\nend)", "UE4SS Keybinding\nBinds a hotkey sequence with optional modifier keys."),
                ("NotifyOnNewObject", "NotifyOnNewObject(${1:\"/Script/ModuleName.ClassName\"}, function(${2:ConstructedObject})\n\t$0\nend)", "UE4SS Object Lifecycle Hook\nFires whenever a new instance of the specified class is allocated in memory."),

                // Object & World Discovery
                ("StaticFindObject", "StaticFindObject(${1:\"/Script/ModuleName.ClassName\"})", "Find UObject in global engine memory table by full path."),
                ("FindObject", "FindObject(${1:\"ClassName\"}, ${2:\"OuterName\"}, ${3:\"ObjectName\"})", "Find UObject by class name, outer name, and object name."),
                ("FindFirstOf", "FindFirstOf(${1:\"ClassName\"})", "Find the first active instance of a class in the loaded world."),
                ("FindAllOf", "FindAllOf(${1:\"ClassName\"})", "Find all active instances of a class in the loaded world (returns a table)."),
                ("StaticConstructObject", "StaticConstructObject(${1:Class}, ${2:Outer}, ${3:Name})", "Allocate and construct a new instance of an Unreal Engine class."),
                ("CreateInvalidObject", "CreateInvalidObject()", "Creates an invalid UObject reference placeholder."),

                // Threading & Execution
                ("ExecuteInGameThread", "ExecuteInGameThread(function()\n\t$0\nend)", "Execute code synchronously inside the main game thread."),
                ("ExecuteWithDelay", "ExecuteWithDelay(${1:1000}, function()\n\t$0\nend)", "Execute a callback after a specified delay in milliseconds."),
                ("ExecuteAsync", "ExecuteAsync(function()\n\t$0\nend)", "Execute a callback asynchronously on a background worker thread."),
                ("IsKeySequenceValid", "IsKeySequenceValid(${1:KeySequence})", "Validates whether a given key sequence is valid."),

                // Engine & Kismet Subsystems
                ("GetEngineVersion", "GetEngineVersion()", "Returns the current Unreal Engine version string."),
                ("GetWorld", "GetWorld()", "Returns the current active UWorld instance."),
                ("GetGameInstance", "GetGameInstance()", "Returns the current active UGameInstance."),
                ("GetPlayerController", "GetPlayerController()", "Returns the primary APlayerController instance."),
                ("GetKismetSystemLibrary", "GetKismetSystemLibrary()", "Returns the global UKismetSystemLibrary static helper."),
                ("GetKismetMathLibrary", "GetKismetMathLibrary()", "Returns the global UKismetMathLibrary static helper."),
                ("GetKismetStringLibrary", "GetKismetStringLibrary()", "Returns the global UKismetStringLibrary static helper."),
                ("GetKismetArrayLibrary", "GetKismetArrayLibrary()", "Returns the global UKismetArrayLibrary static helper."),
                ("GetKismetTextLibrary", "GetKismetTextLibrary()", "Returns the global UKismetTextLibrary static helper."),
                ("GetKismetGuidLibrary", "GetKismetGuidLibrary()", "Returns the global UKismetGuidLibrary static helper."),

                // Enums & Types
                ("Key", "Key.${1:F1}", "UE4SS Key Enum (e.g. Key.F1, Key.SPACE, Key.ENTER)"),
                ("ModifierKey", "ModifierKey.${1:CONTROL}", "UE4SS Modifier Key Enum (ModifierKey.CONTROL, ModifierKey.SHIFT, ModifierKey.ALT)"),
                ("FVector", "FVector(${1:0.0}, ${2:0.0}, ${3:0.0})", "Unreal Engine 3D Vector constructor"),
                ("FRotator", "FRotator(${1:0.0}, ${2:0.0}, ${3:0.0})", "Unreal Engine 3D Rotator constructor (Pitch, Yaw, Roll)"),
                ("FQuat", "FQuat(${1:0.0}, ${2:0.0}, ${3:0.0}, ${4:1.0})", "Unreal Engine Quaternion constructor"),
                ("FTransform", "FTransform(${1:Rotator}, ${2:Translation}, ${3:Scale3D})", "Unreal Engine Transform constructor"),
                ("FLinearColor", "FLinearColor(${1:1.0}, ${2:1.0}, ${3:1.0}, ${4:1.0})", "Unreal Engine RGBA Linear Color constructor"),
                ("FColor", "FColor(${1:255}, ${2:255}, ${3:255}, ${4:255})", "Unreal Engine 8-bit RGBA Color constructor"),
                ("FText", "FText(${1:\"Text\"})", "Unreal Engine Localized FText constructor"),
                ("FName", "FName(${1:\"Name\"})", "Unreal Engine FName identifier constructor"),
                ("FString", "FString(${1:\"String\"})", "Unreal Engine FString constructor"),
            ];

            for (name, snippet, doc) in &standard_apis {
                if q_lower.is_empty() || name.to_ascii_lowercase().contains(&q_lower) {
                    completions.push(EditorCompletion {
                        label: name.to_string(),
                        insert_text: snippet.to_string(),
                        kind: "api".to_string(),
                        detail: Some("UE4SS API".to_string()),
                        documentation: Some(doc.to_string()),
                    });
                }
            }
        }
    } else if ext == "json" || ext == "jsonc" {
        // -------------------------------------------------------------
        // PalSchema Dynamic DataTables, Blueprint Assets & Struct Fields
        // -------------------------------------------------------------
        let mut seen = HashSet::new();

        // 1. DataTables (DT_...)
        if let Some(ref s) = schema {
            for name in &s.names {
                let nl = name.to_ascii_lowercase();
                if (nl.starts_with("dt_") || nl.contains("datatable")) && (q_lower.is_empty() || nl.contains(&q_lower)) {
                    if seen.insert(name.clone()) {
                        completions.push(EditorCompletion {
                            label: name.clone(),
                            insert_text: name.clone(),
                            kind: "table".to_string(),
                            detail: Some("PalSchema DataTable".to_string()),
                            documentation: Some(format!("Palworld reflection DataTable: {}", name)),
                        });
                    }
                }
                if completions.len() >= 60 {
                    break;
                }
            }
        }

        // 2. Blueprint Asset Classes & Mod Paths (/Game/... or BP_...)
        if q_lower.starts_with("bp_") || q_lower.starts_with("/game/") || q_lower.contains("blueprint") || q_lower.is_empty() {
            if let Some(ref sdk) = sdk_index {
                for (cname, cinfo) in &sdk.classes {
                    if cname.starts_with("bp_") || cname.starts_with("wbp_") {
                        if q_lower.is_empty() || cname.contains(&q_lower) {
                            if seen.insert(cinfo.clean_name.clone()) {
                                completions.push(EditorCompletion {
                                    label: cinfo.clean_name.clone(),
                                    insert_text: cinfo.clean_name.clone(),
                                    kind: "class".to_string(),
                                    detail: Some("Blueprint Asset Class".to_string()),
                                    documentation: Some(format!("Game Blueprint: {}\nModule: {}", cinfo.clean_name, cinfo.module_name)),
                                });
                            }
                        }
                    }
                    if completions.len() >= 100 {
                        break;
                    }
                }
            }
        }

        // 3. Struct Properties & Row Fields
        if let Some(ref s) = schema {
            for (sname, ustruct) in &s.structs {
                let sl = sname.to_ascii_lowercase();
                let is_row_struct = sl.ends_with("row") || sl.contains("parameter") || sl.contains("data");

                if is_row_struct || q_lower.is_empty() || sl.contains(&q_lower) {
                    for prop in &ustruct.properties {
                        let pl = prop.name.to_ascii_lowercase();
                        if q_lower.is_empty() || pl.contains(&q_lower) {
                            if seen.insert(prop.name.clone()) {
                                completions.push(EditorCompletion {
                                    label: prop.name.clone(),
                                    insert_text: prop.name.clone(),
                                    kind: "struct".to_string(),
                                    detail: Some(format!("Field of {}", sname)),
                                    documentation: Some(format!("Property: {}\nParent Struct: {}\nType: {}", prop.name, sname, prop.type_name)),
                                });
                            }
                        }
                        if completions.len() >= 100 {
                            break;
                        }
                    }
                }
                if completions.len() >= 100 {
                    break;
                }
            }
        }
    }

    Ok(completions)
}

fn find_hook_start(line: &str, start_idx: usize) -> Option<usize> {
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

fn extract_string_literal(line: &str, start_pos: usize) -> Option<(String, char, usize)> {
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
