// Parsers and AST string extractors for PalSchema and UE4SS conflicts
use std::fs;
use std::path::Path;
use super::types::*;
use super::utils::{strip_jsonc_comments, collect_files_with_extensions};

pub fn scan_palschema_mod(
    mod_path: &Path,
    conflict_info: &ConflictingMod,
    table_map: &mut TableMap,
    palschema_tables: &mut Vec<PalschemaTableEntry>,
    warnings: &mut Vec<String>,
) {
    let mut files_to_scan = Vec::new();
    collect_files_with_extensions(mod_path, &["json", "jsonc"], &mut files_to_scan);

    for file_path in files_to_scan {
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name.ends_with(".pmm.json") {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&file_path) {
            let content_clean = content.strip_prefix('\u{feff}').unwrap_or(&content);
            if content_clean.trim().is_empty() {
                continue;
            }
            let stripped = strip_jsonc_comments(content_clean);
            match serde_json::from_str::<serde_json::Value>(&stripped) {
                Ok(val) => {
                    let mut file_info = conflict_info.clone();
                    if let Ok(rel) = file_path.strip_prefix(mod_path) {
                        file_info.file_path = rel.to_string_lossy().to_string();
                    } else {
                        file_info.file_path = file_path.to_string_lossy().to_string();
                    }
                    extract_palschema_rows(&file_path, &val, content_clean, &file_info, table_map, palschema_tables, warnings);
                }
                Err(e) => {
                    warnings.push(format!(
                        "Mod '{}': Failed to parse JSON file '{}': {}",
                        conflict_info.mod_name,
                        file_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"),
                        e
                    ));
                }
            }
        }
    }
}

pub fn extract_palschema_rows(
    file_path: &Path,
    json_val: &serde_json::Value,
    raw_content: &str,
    mod_info: &ConflictingMod,
    table_map: &mut TableMap,
    palschema_tables: &mut Vec<PalschemaTableEntry>,
    _warnings: &mut Vec<String>,
) {
    let filename = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    let content_lines: Vec<&str> = raw_content.lines().collect();

    if let Some(obj) = json_val.as_object() {
        for (key, val) in obj {
            let is_dt = key.starts_with("DT_") || key.contains("DataTable");
            let is_bp = key.starts_with("BP_") || key.ends_with("_C");
            
            if (is_dt || is_bp) && val.is_object() {
                let line_num = content_lines
                    .iter()
                    .position(|l| l.contains(&format!("\"{}\"", key)))
                    .map(|p| p as u32 + 1)
                    .unwrap_or(1);

                palschema_tables.push(PalschemaTableEntry {
                    mod_id: mod_info.mod_id.clone(),
                    mod_name: mod_info.mod_name.clone(),
                    file_path: mod_info.file_path.clone(),
                    line_number: line_num,
                    table_name: key.clone(),
                });

                if let Some(nested_obj) = val.as_object() {
                    for (row_key, row_val) in nested_obj {
                        if row_val.is_object() || row_val.is_array() {
                            let val_str = serde_json::to_string(row_val).unwrap_or_else(|_| row_val.to_string());
                            let detail = if val_str.len() > 140 {
                                format!("{}...", &val_str[..140])
                            } else {
                                val_str
                            };

                            let mut info = mod_info.clone();
                            info.detail = detail;

                            let entry_key = format!("{}::{}", key, row_key);
                            table_map.entry(entry_key).or_default().push(info);
                        }
                    }
                }
            } else if (filename.starts_with("DT_") || filename.contains("DataTable") || filename.starts_with("BP_") || filename.ends_with("_C")) && (val.is_object() || val.is_array()) {
                palschema_tables.push(PalschemaTableEntry {
                    mod_id: mod_info.mod_id.clone(),
                    mod_name: mod_info.mod_name.clone(),
                    file_path: mod_info.file_path.clone(),
                    line_number: 1,
                    table_name: filename.to_string(),
                });

                let val_str = serde_json::to_string(val).unwrap_or_else(|_| val.to_string());
                let detail = if val_str.len() > 140 {
                    format!("{}...", &val_str[..140])
                } else {
                    val_str
                };

                let mut info = mod_info.clone();
                info.detail = detail;

                let entry_key = format!("{}::{}", filename, key);
                table_map.entry(entry_key).or_default().push(info);
            }
        }
    }
}

pub fn scan_ue4ss_mod(
    mod_path: &Path,
    scripts_path: &Path,
    conflict_info: &ConflictingMod,
    hook_map: &mut HookMap,
    deprecated_diagnostics: &mut Vec<UsmapHookDiagnostic>,
    _warnings: &mut Vec<String>,
) {
    let mut files_to_scan = Vec::new();
    collect_files_with_extensions(scripts_path, &["lua"], &mut files_to_scan);

    // Also collect root Lua files directly in mod_path (e.g. main.lua alongside scripts/)
    if mod_path != scripts_path && mod_path.is_dir() {
        if let Ok(entries) = fs::read_dir(mod_path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("lua")).unwrap_or(false) {
                    if !files_to_scan.contains(&p) {
                        files_to_scan.push(p);
                    }
                }
            }
        }
    }

    for file_path in files_to_scan {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let mut file_info = conflict_info.clone();
            if let Ok(rel) = file_path.strip_prefix(mod_path) {
                file_info.file_path = rel.to_string_lossy().to_string();
            } else {
                file_info.file_path = file_path.to_string_lossy().to_string();
            }

            let hooks = extract_literal_hooks(&content);
            for (hook_fn, target, line_num, line_code) in hooks {
                let entry = hook_map.entry(target).or_insert_with(|| (hook_fn.clone(), Vec::new()));
                
                let mut current_info = file_info.clone();
                current_info.line_number = line_num;
                current_info.detail = line_code;

                // Deduplicate same hook function within same mod
                if !entry.1.iter().any(|m| m.mod_id == current_info.mod_id && m.line_number == current_info.line_number) {
                    entry.1.push(current_info);
                }
            }

            // Scan for Deprecated UE4SS APIs & Blind pcall Anti-Patterns
            crate::commands::scanner::hook_validator::scan_lua_deprecations(
                &file_info.mod_id,
                &file_info.mod_name,
                &file_info.file_path,
                &content,
                deprecated_diagnostics,
            );
        }
    }
}

pub fn extract_literal_hooks(content: &str) -> Vec<(String, String, u32, String)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut line_num = 0;

    // Supported hook registration APIs and wrapper patterns
    let api_patterns = [
        ("RegisterHook", "RegisterHook"),
        ("registerHook", "RegisterHook"),
        ("register_hook", "RegisterHook"),
        ("Register_Hook", "RegisterHook"),
        ("NotifyOnNewObject", "NotifyOnNewObject"),
        ("notifyOnNewObject", "NotifyOnNewObject"),
        ("notify_on_new_object", "NotifyOnNewObject"),
        ("Notify_On_New_Object", "NotifyOnNewObject"),
    ];

    while line_num < lines.len() {
        let trimmed = lines[line_num].trim();
        line_num += 1;

        if trimmed.starts_with("--") {
            continue;
        }

        for (pattern, canonical_api) in &api_patterns {
            let mut search_from = 0;
            while let Some(rel_idx) = trimmed[search_from..].find(pattern) {
                let idx = search_from + rel_idx;
                let end_idx = idx + pattern.len();
                search_from = end_idx;

                // 1. Ensure word boundary after pattern (must not be followed by alphanumeric or _)
                if end_idx < trimmed.len() {
                    let next_byte = trimmed.as_bytes()[end_idx];
                    if next_byte.is_ascii_alphanumeric() || next_byte == b'_' {
                        continue;
                    }
                }

                // 2. Identify the prefix before pattern
                let mut id_start = idx;
                while id_start > 0 {
                    let b = trimmed.as_bytes()[id_start - 1];
                    if b.is_ascii_alphanumeric() || b == b'_' {
                        id_start -= 1;
                    } else {
                        break;
                    }
                }

                // Skip function declarations (e.g. "local function safeRegisterHook(...)")
                let before_id = trimmed[..id_start].trim_end();
                if before_id.ends_with("function") {
                    continue;
                }

                if let Some((target, decl_code)) = find_hook_target_string(&lines, line_num - 1, end_idx) {
                    let clean_target = target.trim();
                    let is_valid_target = !clean_target.is_empty()
                        && !clean_target.starts_with(':')
                        && !clean_target.ends_with(':')
                        && !clean_target.starts_with('.')
                        && clean_target.len() >= 3
                        && (clean_target.contains('/') || clean_target.contains(':') || clean_target.starts_with("Pal") || clean_target.starts_with("APal") || clean_target.starts_with("UPal") || clean_target.starts_with("BP_"));

                    if is_valid_target {
                        let line_code = if decl_code.len() > 120 {
                            format!("{}...", &decl_code[..120])
                        } else {
                            decl_code
                        };
                        results.push((canonical_api.to_string(), clean_target.to_string(), line_num as u32, line_code));
                    }
                }
            }
        }
    }
    results
}

fn find_hook_target_string(
    lines: &[&str],
    start_line_idx: usize,
    char_offset: usize,
) -> Option<(String, String)> {
    let mut combined_code = String::new();
    let max_lookahead = (start_line_idx + 4).min(lines.len());

    for i in start_line_idx..max_lookahead {
        let l = lines[i].trim();
        if l.starts_with("--") {
            continue;
        }
        if combined_code.is_empty() {
            combined_code.push_str(l);
        } else {
            combined_code.push(' ');
            combined_code.push_str(l);
        }

        let slice = if i == start_line_idx {
            if char_offset < lines[i].len() {
                &lines[i][char_offset..]
            } else {
                ""
            }
        } else {
            l
        };

        if let Some(target) = extract_first_quoted_target(slice) {
            return Some((target, combined_code));
        }

        if l.contains(';') || (l.contains(')') && !l.contains('(')) {
            break;
        }
    }

    None
}

fn extract_first_quoted_target(text: &str) -> Option<String> {
    let clean_text = if let Some(c_idx) = text.find("--") {
        &text[..c_idx]
    } else {
        text
    };

    let mut quote_char = None;
    let mut start_idx = 0;

    for (i, c) in clean_text.char_indices() {
        if quote_char.is_none() {
            if c == '"' || c == '\'' {
                quote_char = Some(c);
                start_idx = i + c.len_utf8();
            }
        } else if Some(c) == quote_char {
            let target = &clean_text[start_idx..i];
            return Some(target.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_literal_hooks_with_safe_register_hook_and_custom_wrappers() {
        let lua_code = r#"
-- PalWorldBedtimeExtended realistic snippet
local function safeRegisterHook(funcName, preCallback, postCallback)
    local ok, err = pcall(function()
        if postCallback then
            RegisterHook(funcName, preCallback, postCallback)
        else
            RegisterHook(funcName, preCallback)
        end
    end)
end

function Hooks.Init()
    -- Multiline safeRegisterHook 1
    safeRegisterHook(
        "/Game/Pal/Blueprint/Controller/Monster/BP_MonsterAIController_BaseCamp.BP_MonsterAIController_BaseCamp_C:InterruptSleepActivelyAction",
        function(self, Parameter) end
    )

    -- Multiline safeRegisterHook 2
    safeRegisterHook(
        "/Game/Pal/Blueprint/Controller/Monster/BP_MonsterAIController_BaseCamp.BP_MonsterAIController_BaseCamp_C:SetBaseCampActionSleep",
        function(self) end
    )

    -- Snake_case wrapper
    safe_register_hook('/Script/Pal.PalPlayerCharacter:OnJump', on_jump)

    -- PascalCase SafeRegisterHook
    SafeRegisterHook("/Script/Pal.PalUtility:SpawnPal", function() end)

    -- CamelCase direct registerHook
    registerHook("/Script/Engine.PlayerController:ClientRestart", restart)

    -- Wrapper for NotifyOnNewObject
    safeNotifyOnNewObject("/Script/Pal.PalMapObjectSpawner", function() end)
end
"#;

        let hooks = extract_literal_hooks(lua_code);
        let targets: Vec<String> = hooks.iter().map(|(_, t, _, _)| t.clone()).collect();
        let apis: Vec<String> = hooks.iter().map(|(api, _, _, _)| api.clone()).collect();

        // 1. Definition must NOT produce a false hook
        assert!(!targets.iter().any(|t| t.contains("funcName")), "Function definition argument must not be detected as hook");

        // 2. Both BedtimeExtended hooks must be accurately extracted
        assert!(targets.contains(&"/Game/Pal/Blueprint/Controller/Monster/BP_MonsterAIController_BaseCamp.BP_MonsterAIController_BaseCamp_C:InterruptSleepActivelyAction".to_string()));
        assert!(targets.contains(&"/Game/Pal/Blueprint/Controller/Monster/BP_MonsterAIController_BaseCamp.BP_MonsterAIController_BaseCamp_C:SetBaseCampActionSleep".to_string()));

        // 3. Other wrappers extracted
        assert!(targets.contains(&"/Script/Pal.PalPlayerCharacter:OnJump".to_string()));
        assert!(targets.contains(&"/Script/Pal.PalUtility:SpawnPal".to_string()));
        assert!(targets.contains(&"/Script/Engine.PlayerController:ClientRestart".to_string()));
        assert!(targets.contains(&"/Script/Pal.PalMapObjectSpawner".to_string()));

        assert_eq!(targets.len(), 6, "Must extract exactly 6 active hooks from wrappers and variants");

        // 4. APIs canonicalized
        assert_eq!(apis.iter().filter(|a| *a == "RegisterHook").count(), 5);
        assert_eq!(apis.iter().filter(|a| *a == "NotifyOnNewObject").count(), 1);
    }
}

