use super::types::EditorDiagnostic;

#[derive(Debug, PartialEq)]
pub enum LuaLexState {
    Code,
    SingleLineComment,
    MultiLineComment(usize),
    SingleQuoteString,
    DoubleQuoteString,
    MultiLineString(usize),
}

pub fn lint_lua_syntax(content: &str, diagnostics: &mut Vec<EditorDiagnostic>) {
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


