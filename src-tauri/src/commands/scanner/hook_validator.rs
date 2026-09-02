// UE4SS Hook Validation against USMAP schema and C++ SDK

pub fn validate_hook_with_usmap(
    target: &str,
    schema: &crate::usmap::parser::UsmapSchema,
    sdk: Option<&crate::usmap::SdkIndex>,
) -> (String, String, String, String, Option<String>) {
    let raw_target = target.trim();
    
    // 1. Dynamic Blueprint Asset Hooks (/Game/...)
    if raw_target.starts_with("/Game/") {
        let (raw_class, raw_func) = if let Some(colon_idx) = raw_target.find(':') {
            (&raw_target[..colon_idx], &raw_target[colon_idx + 1..])
        } else {
            (raw_target, "")
        };

        let clean_class = raw_class
            .split('.')
            .last()
            .unwrap_or(raw_class)
            .split('/')
            .last()
            .unwrap_or(raw_class)
            .trim();

        let clean_func = raw_func.trim();

        if let Some(sdk_idx) = sdk {
            if sdk_idx.find_class(clean_class).is_some() {
                if !clean_func.is_empty() {
                    if sdk_idx.has_function(clean_class, clean_func) {
                        return (
                            clean_class.to_string(),
                            clean_func.to_string(),
                            "valid".to_string(),
                            format!("Verified Blueprint hook '{}:{}' in C++ SDK", clean_class, clean_func),
                            None,
                        );
                    } else {
                        let suggestion = sdk_idx.suggest_similar_function(clean_class, clean_func).map(|sug| {
                            if let Some((prefix, _)) = raw_target.split_once(':') {
                                format!("{}:{}", prefix, sug)
                            } else {
                                format!("{}:{}", clean_class, sug)
                            }
                        });
                        return (
                            clean_class.to_string(),
                            clean_func.to_string(),
                            "broken_function".to_string(),
                            format!("Function or event '{}' not found in Blueprint '{}' (checked C++ SDK)", clean_func, clean_class),
                            suggestion,
                        );
                    }
                } else {
                    return (
                        clean_class.to_string(),
                        clean_func.to_string(),
                        "valid".to_string(),
                        format!("Verified Blueprint class '{}' in C++ SDK", clean_class),
                        None,
                    );
                }
            }
        }

        return (
            clean_class.to_string(),
            clean_func.to_string(),
            "blueprint_asset".to_string(),
            "Dynamic Blueprint Asset Hook (loaded at runtime from game packages)".to_string(),
            None,
        );
    }

    // 2. Native C++ Engine Hooks (/Script/...)
    let (raw_class, raw_func) = if let Some(colon_idx) = raw_target.find(':') {
        (&raw_target[..colon_idx], &raw_target[colon_idx + 1..])
    } else {
        (raw_target, "")
    };

    let without_script = raw_class.trim_start_matches("/Script/");
    let mut clean_class = if let Some(dot_idx) = without_script.rfind('.') {
        &without_script[dot_idx + 1..]
    } else if without_script.starts_with("PalPal") {
        &without_script[3..]
    } else {
        without_script
    }.trim();

    if clean_class.starts_with("Default__") && clean_class.len() > 9 {
        clean_class = &clean_class[9..];
    }

    let clean_func = raw_func.trim();

    if clean_class.is_empty() {
        return (
            clean_class.to_string(),
            clean_func.to_string(),
            "unknown".to_string(),
            "Empty or unparseable hook target".to_string(),
            None,
        );
    }

    // Check if class is Blueprint by naming convention (starts with BP_ or ends with _C)
    if clean_class.starts_with("BP_") || clean_class.ends_with("_C") {
        if let Some(sdk_idx) = sdk {
            if sdk_idx.find_class(clean_class).is_some() {
                if !clean_func.is_empty() {
                    if sdk_idx.has_function(clean_class, clean_func) {
                        return (
                            clean_class.to_string(),
                            clean_func.to_string(),
                            "valid".to_string(),
                            format!("Verified Blueprint hook '{}:{}' in C++ SDK", clean_class, clean_func),
                            None,
                        );
                    } else {
                        let suggestion = sdk_idx.suggest_similar_function(clean_class, clean_func).map(|sug| {
                            if let Some((prefix, _)) = raw_target.split_once(':') {
                                format!("{}:{}", prefix, sug)
                            } else {
                                format!("{}:{}", clean_class, sug)
                            }
                        });
                        return (
                            clean_class.to_string(),
                            clean_func.to_string(),
                            "broken_function".to_string(),
                            format!("Function '{}' not found in Blueprint '{}' (checked C++ SDK)", clean_func, clean_class),
                            suggestion,
                        );
                    }
                } else {
                    return (
                        clean_class.to_string(),
                        clean_func.to_string(),
                        "valid".to_string(),
                        format!("Verified Blueprint class '{}' in C++ SDK", clean_class),
                        None,
                    );
                }
            }
        }

        return (
            clean_class.to_string(),
            clean_func.to_string(),
            "blueprint_asset".to_string(),
            "Dynamic Blueprint Asset Hook (loaded at runtime from game packages)".to_string(),
            None,
        );
    }

    // If C++ SDK is available, perform authoritative class and method resolution
    if let Some(sdk_idx) = sdk {
        let class_in_sdk = sdk_idx.find_class(clean_class);
        let class_struct = schema.find_struct(clean_class);
        let class_in_names = schema.names.iter().any(|n| {
            n.eq_ignore_ascii_case(clean_class)
                || n.eq_ignore_ascii_case(&format!("A{}", clean_class))
                || n.eq_ignore_ascii_case(&format!("U{}", clean_class))
        });

        if class_in_sdk.is_none() && class_struct.is_none() && !class_in_names {
            let suggestion = sdk_idx.suggest_similar_class(clean_class).map(|sug_cls| {
                if let Some((raw_cls_part, raw_func_part)) = raw_target.split_once(':') {
                    if let Some(dot_idx) = raw_cls_part.rfind('.') {
                        let prefix = &raw_cls_part[..=dot_idx];
                        format!("{}{}:{}", prefix, sug_cls, raw_func_part)
                    } else if raw_cls_part.starts_with("/Script/") {
                        format!("/Script/{}:{}", sug_cls, raw_func_part)
                    } else {
                        format!("{}:{}", sug_cls, raw_func_part)
                    }
                } else if let Some(dot_idx) = raw_target.rfind('.') {
                    let prefix = &raw_target[..=dot_idx];
                    format!("{}{}", prefix, sug_cls)
                } else {
                    sug_cls
                }
            });
            return (
                clean_class.to_string(),
                clean_func.to_string(),
                "broken_class".to_string(),
                format!(
                    "Native class '{}' does not exist in Palworld schema or C++ SDK",
                    clean_class
                ),
                suggestion,
            );
        }

        if class_in_sdk.is_some() {
            if !clean_func.is_empty() {
                if sdk_idx.has_function(clean_class, clean_func) {
                    return (
                        clean_class.to_string(),
                        clean_func.to_string(),
                        "valid".to_string(),
                        format!(
                            "Verified native hook '{}:{}' in Palworld C++ SDK",
                            clean_class, clean_func
                        ),
                        None,
                    );
                } else {
                    let suggestion = sdk_idx.suggest_similar_function(clean_class, clean_func)
                        .map(|sug| {
                            if let Some((prefix, _)) = raw_target.split_once(':') {
                                format!("{}:{}", prefix, sug)
                            } else {
                                format!("{}:{}", clean_class, sug)
                            }
                        })
                        .or_else(|| {
                            sdk_idx.find_class_with_function(clean_func).map(|correct_cls| {
                                if let Some((prefix, _)) = raw_target.split_once(':') {
                                    if let Some(dot_idx) = prefix.rfind('.') {
                                        format!("{}{}:{}", &prefix[..=dot_idx], correct_cls, clean_func)
                                    } else if prefix.starts_with("/Script/") {
                                        format!("/Script/{}:{}", correct_cls, clean_func)
                                    } else {
                                        format!("{}:{}", correct_cls, clean_func)
                                    }
                                } else {
                                    format!("{}:{}", correct_cls, clean_func)
                                }
                            })
                        });
                    return (
                        clean_class.to_string(),
                        clean_func.to_string(),
                        "broken_function".to_string(),
                        format!(
                            "Function or delegate '{}' not found in class '{}' (checked C++ SDK)",
                            clean_func, clean_class
                        ),
                        suggestion,
                    );
                }
            } else {
                return (
                    clean_class.to_string(),
                    clean_func.to_string(),
                    "valid".to_string(),
                    format!(
                        "Verified native class '{}' in Palworld C++ SDK",
                        clean_class
                    ),
                    None,
                );
            }
        }
    }

    // Fallback: Check class existence in USMAP
    let class_struct = schema.find_struct(clean_class);
    let class_in_names = schema.names.iter().any(|n| {
        n.eq_ignore_ascii_case(clean_class)
            || n.eq_ignore_ascii_case(&format!("A{}", clean_class))
            || n.eq_ignore_ascii_case(&format!("U{}", clean_class))
    });

    let class_exists = class_struct.is_some() || class_in_names;

    if !class_exists {
        return (
            clean_class.to_string(),
            clean_func.to_string(),
            "broken_class".to_string(),
            format!(
                "Native class '{}' does not exist in current Palworld schema",
                clean_class
            ),
            None,
        );
    }

    // If function is specified, validate function/delegate name in schema FNames
    if !clean_func.is_empty() {
        let func_in_struct_props = class_struct.map_or(false, |s| {
            s.properties.iter().any(|p| p.name.eq_ignore_ascii_case(clean_func))
        });
        let func_in_names = schema.names.iter().any(|n| n.eq_ignore_ascii_case(clean_func));

        if !func_in_struct_props && !func_in_names {
            return (
                clean_class.to_string(),
                clean_func.to_string(),
                "broken_function".to_string(),
                format!(
                    "Function or delegate '{}' not found in Palworld schema FNames",
                    clean_func
                ),
                None,
            );
        }
    }

    let clean_ver = schema.game_version.trim_start_matches(|c| c == 'v' || c == 'V');

    (
        clean_class.to_string(),
        clean_func.to_string(),
        "valid".to_string(),
        if clean_func.is_empty() {
            format!(
                "Verified native class '{}' in Palworld schema v{}",
                clean_class, clean_ver
            )
        } else {
            format!(
                "Verified native hook '{}:{}' in Palworld schema v{}",
                clean_class, clean_func, clean_ver
            )
        },
        None,
    )
}

pub fn scan_lua_deprecations(
    mod_id: &str,
    mod_name: &str,
    file_rel_path: &str,
    content: &str,
    diagnostics: &mut Vec<crate::commands::scanner::types::UsmapHookDiagnostic>,
) {
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let line_num = (i + 1) as u32;
        let trimmed = line.trim();
        if trimmed.starts_with("--") {
            continue;
        }

        // Mask out string literals and inline comments
        let code_only = crate::commands::editor::validation::mask_strings_and_comments(line);
        let code_trimmed = code_only.trim();

        let line_lower = code_only.to_ascii_lowercase();
        let deprecated_rules = [
            (
                "loopasync",
                "LoopAsync",
                "LoopAsync is deprecated in UE4SS. Use LoopInGameThreadWithDelay for cancellable and pauseable timers.",
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
                "ForEachProperty is deprecated on UStruct/UClass in UE4SS. Use direct field access (GetPropertyValue / __index).",
                None,
            ),
        ];

        for (token_lower, display_name, reason, suggestion) in &deprecated_rules {
            if let Some(pos) = line_lower.find(token_lower) {
                let is_prefix = pos > 0 && code_only.chars().nth(pos - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                let after_idx = pos + token_lower.len();
                let is_suffix = after_idx < code_only.len() && code_only.chars().nth(after_idx).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);

                if !is_prefix && !is_suffix {
                    diagnostics.push(crate::commands::scanner::types::UsmapHookDiagnostic {
                        mod_id: mod_id.to_string(),
                        mod_name: mod_name.to_string(),
                        file_path: file_rel_path.to_string(),
                        line_number: line_num,
                        hook_target: display_name.to_string(),
                        target_class: display_name.to_string(),
                        target_function: String::new(),
                        status: "deprecated_api".to_string(),
                        reason: reason.to_string(),
                        suggestion: suggestion.map(|s| s.to_string()),
                        category: "ue4ss_deprecated".to_string(),
                    });
                }
            }
        }

        // Blind pcall check
        if code_trimmed.starts_with("pcall(") || code_trimmed.starts_with("pcall ") {
            diagnostics.push(crate::commands::scanner::types::UsmapHookDiagnostic {
                mod_id: mod_id.to_string(),
                mod_name: mod_name.to_string(),
                file_path: file_rel_path.to_string(),
                line_number: line_num,
                hook_target: "pcall(...)".to_string(),
                target_class: "pcall".to_string(),
                target_function: "pcall".to_string(),
                status: "blind_pcall".to_string(),
                reason: "Blind pcall detected: suppressing errors silences runtime crashes and makes mods impossible to debug. Capture 'local ok, err = pcall(...)' and log errors.".to_string(),
                suggestion: Some("local ok, err = pcall(...)".to_string()),
                category: "anti_pattern".to_string(),
            });
        }
    }
}
