// Conflict scanning for PalSchema and UE4SS mods
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use tauri::State;
use crate::state::AppState;
use crate::models::ModType;
use super::types::*;
use super::utils::{strip_jsonc_comments, collect_files_with_extensions};
use super::hook_validator::validate_hook_with_usmap;
use super::palschema::validate_palschema_table_with_usmap;

#[tauri::command]
pub async fn scan_conflicts(state: State<'_, AppState>) -> Result<ScanResult, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    
    // Obtain active mods for current profile
    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    
    let mut table_map: TableMap = HashMap::new();
    let mut hook_map: HookMap = HashMap::new();
    let mut palschema_tables: Vec<PalschemaTableEntry> = Vec::new();
    let mut deprecated_diagnostics: Vec<UsmapHookDiagnostic> = Vec::new();
    let mut warnings = Vec::new();
    
    let mut total_scanned = 0;
    let mut palschema_scanned = 0;
    let mut ue4ss_scanned = 0;

    for m in &profile_mods {
        // Skip disabled mods
        if !m.enabled || m.game_path.is_empty() {
            continue;
        }
        // Skip Native UE4SS mods
        if m.nexus_author.as_deref() == Some("UE4SS Native Mod") {
            continue;
        }

        total_scanned += 1;
        let mod_path = Path::new(&m.game_path);
        if !mod_path.exists() {
            continue;
        }

        let is_palschema = m.mod_type == ModType::PalSchema || m.mod_type == ModType::Hybrid;
        let is_ue4ss = m.mod_type == ModType::Ue4ss || m.mod_type == ModType::Hybrid;

        let conflict_info = ConflictingMod {
            mod_id: m.id.clone(),
            mod_name: m.name.clone(),
            file_path: String::new(),
            line_number: 0,
            detail: String::new(),
        };

        if is_palschema {
            palschema_scanned += 1;
            let mut target_palschema_path = mod_path.to_path_buf();
            if m.mod_type == ModType::Hybrid {
                for extra in &m.extra_files {
                    if extra.contains("PalSchema") {
                        target_palschema_path = PathBuf::from(extra);
                        break;
                    }
                }
            }
            if target_palschema_path.exists() {
                scan_palschema_mod(&target_palschema_path, &conflict_info, &mut table_map, &mut palschema_tables, &mut warnings);
            }
        }

        if is_ue4ss {
            ue4ss_scanned += 1;
            
            let mut paths_to_scan = Vec::new();
            if !m.game_path.is_empty() {
                paths_to_scan.push(PathBuf::from(&m.game_path));
            }
            for extra in &m.extra_files {
                if !extra.is_empty() {
                    paths_to_scan.push(PathBuf::from(extra));
                }
            }

            for base_path in paths_to_scan {
                if !base_path.exists() {
                    continue;
                }
                let scripts_path = if base_path.join("Scripts").exists() {
                    Some(base_path.join("Scripts"))
                } else if base_path.join("scripts").exists() {
                    Some(base_path.join("scripts"))
                } else if base_path.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("scripts".to_string()) && base_path.is_dir() {
                    Some(base_path.clone())
                } else {
                    None
                };

                if let Some(ref s_path) = scripts_path {
                    scan_ue4ss_mod(&base_path, s_path, &conflict_info, &mut hook_map, &mut deprecated_diagnostics, &mut warnings);
                }
            }
        }
    }

    // Filter conflicts
    let mut table_conflicts = Vec::new();
    let mut internal_table_conflicts = Vec::new();
    for (entry_key, mods) in table_map.clone() {
        if mods.len() > 1 {
            let parts: Vec<&str> = entry_key.split("::").collect();
            let table_name = parts.get(0).copied().unwrap_or("Unknown").to_string();
            let row_name = parts.get(1).copied().unwrap_or("Unknown").to_string();
            
            let first_id = &mods[0].mod_id;
            let is_internal = mods.iter().all(|m| &m.mod_id == first_id);
            
            let conflict = TableRowConflict {
                table_name,
                row_name,
                mods,
            };
            
            if is_internal {
                internal_table_conflicts.push(conflict);
            } else {
                table_conflicts.push(conflict);
            }
        }
    }

    let mut hook_conflicts = Vec::new();
    let mut internal_hook_conflicts = Vec::new();
    for (hook_target, (hook_fn, mods)) in hook_map.clone() {
        if mods.len() > 1 {
            let first_id = &mods[0].mod_id;
            let is_internal = mods.iter().all(|m| &m.mod_id == first_id);
            
            let conflict = HookConflict {
                hook_target,
                hook_fn,
                mods,
            };
            
            if is_internal {
                internal_hook_conflicts.push(conflict);
            } else {
                hook_conflicts.push(conflict);
            }
        }
    }

    // Build mod summaries map: mod_id -> (mod_name, mod_type, rows, hooks, pak_files)
    let mut summaries_map: HashMap<String, (String, String, Vec<String>, Vec<String>, Vec<String>)> = HashMap::new();
    for m in &profile_mods {
        if m.enabled && !m.game_path.is_empty() && m.nexus_author.as_deref() != Some("UE4SS Native Mod") {
            let type_str = format!("{:?}", m.mod_type);
            let mut pak_assets = Vec::new();

            // Collect any .pak, .utoc, .ucas files belonging to this mod
            let mut paks = Vec::new();
            let is_pak_or_zen = |path_str: &str| {
                let lower = path_str.to_lowercase();
                lower.ends_with(".pak") || lower.ends_with(".utoc") || lower.ends_with(".ucas")
            };

            if is_pak_or_zen(&m.game_path) {
                paks.push(PathBuf::from(&m.game_path));
            }
            for extra in &m.extra_files {
                if is_pak_or_zen(extra) {
                    paks.push(PathBuf::from(extra));
                }
            }

            for p in paks {
                if p.exists() {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext.eq_ignore_ascii_case("pak") {
                        if let Ok(entries) = crate::pak_scanner::list_pak_entries(&p) {
                            for entry in entries {
                                let entry_lower = entry.to_lowercase();
                                if entry_lower.ends_with(".uasset") {
                                    pak_assets.push(entry);
                                }
                            }
                        }
                    } else if ext.eq_ignore_ascii_case("utoc") || ext.eq_ignore_ascii_case("ucas") {
                        let filename = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        pak_assets.push(format!("[Zen Container] {}", filename));
                    }
                }
            }

            summaries_map.insert(m.id.clone(), (m.name.clone(), type_str, Vec::new(), Vec::new(), pak_assets));
        }
    }

    for (entry_key, mods) in &table_map {
        for m in mods {
            if let Some(summary) = summaries_map.get_mut(&m.mod_id) {
                let row_str = format!("{} ({})", entry_key, m.file_path);
                summary.2.push(row_str);
            }
        }
    }

    for (hook_target, (hook_fn, mods)) in &hook_map {
        for m in mods {
            if let Some(summary) = summaries_map.get_mut(&m.mod_id) {
                let hook_str = format!("{} [{}] ({}:L{})", hook_target, hook_fn, m.file_path, m.line_number);
                summary.3.push(hook_str);
            }
        }
    }

    let mut mod_summaries = Vec::new();
    for (mod_id, (mod_name, mod_type, mut rows, mut hooks, mut pak_files)) in summaries_map {
        // Skip if mod registry contains absolutely no rows, hooks, or pak assets
        if rows.is_empty() && hooks.is_empty() && pak_files.is_empty() {
            continue;
        }
        rows.sort();
        hooks.sort();
        pak_files.sort();
        mod_summaries.push(ModSummary {
            mod_id,
            mod_name,
            mod_type,
            palschema_rows: rows,
            ue4ss_hooks: hooks,
            pak_files,
        });
    }
    mod_summaries.sort_by(|a, b| a.mod_name.cmp(&b.mod_name));

    let (pak_conflicts, pak_scanned) = if !data.settings.game_path.is_empty() {
        crate::pak_scanner::scan_pak_conflicts(Path::new(&data.settings.game_path), &profile_mods)
    } else {
        (Vec::new(), 0)
    };

    let (is_gamepass, gamepass_notices) = if !data.settings.game_path.is_empty() {
        crate::pak_scanner::check_gamepass_pak_compatibility(Path::new(&data.settings.game_path), &profile_mods)
    } else {
        (false, Vec::new())
    };

    let schema_notices = crate::pak_scanner::check_mod_schema_compatibility(&profile_mods);

    // USMAP Unreal Engine Schema Hook Diagnostics & C++ SDK Headers
    let usmap_path = crate::usmap::sync::get_active_usmap_path(&data.settings.program_path);
    let sdk_index = crate::usmap::get_or_load_sdk_index(&data.settings.program_path, &data.settings.game_path);

    let usmap_diagnostics = if usmap_path.exists() {
        if let Ok(schema) = crate::usmap::parser::parse_usmap_file(&usmap_path) {
            let mut diagnostics = Vec::new();
            let mut valid_hooks = 0;
            let mut broken_hooks = 0;

            // 1. Validate UE4SS Lua Hooks
            for (hook_target, (_hook_fn, mods)) in &hook_map {
                let (target_class, target_function, status, reason, suggestion) = validate_hook_with_usmap(hook_target, &schema, sdk_index.as_deref());
                if status == "valid" || status == "blueprint_asset" {
                    valid_hooks += 1;
                } else if status == "broken_class" || status == "broken_function" {
                    broken_hooks += 1;
                }

                for m in mods {
                    diagnostics.push(UsmapHookDiagnostic {
                        mod_id: m.mod_id.clone(),
                        mod_name: m.mod_name.clone(),
                        file_path: m.file_path.clone(),
                        line_number: m.line_number,
                        hook_target: hook_target.clone(),
                        target_class: target_class.clone(),
                        target_function: target_function.clone(),
                        status: status.clone(),
                        reason: reason.clone(),
                        suggestion: suggestion.clone(),
                        category: "ue4ss".to_string(),
                    });
                }
            }

            // 2. Validate PalSchema DataTables & Structures
            let mut seen_palschema_tables = HashSet::new();
            for entry in &palschema_tables {
                let dedup = (entry.mod_id.clone(), entry.file_path.clone(), entry.table_name.clone());
                if !seen_palschema_tables.insert(dedup) {
                    continue;
                }

                let (status, reason, suggestion) = validate_palschema_table_with_usmap(&entry.table_name, &schema, sdk_index.as_deref(), Path::new(&data.settings.game_path));
                if status == "valid" || status == "blueprint_asset" {
                    valid_hooks += 1;
                } else if status == "broken_table" || status == "broken_class" {
                    broken_hooks += 1;
                }

                diagnostics.push(UsmapHookDiagnostic {
                    mod_id: entry.mod_id.clone(),
                    mod_name: entry.mod_name.clone(),
                    file_path: entry.file_path.clone(),
                    line_number: entry.line_number,
                    hook_target: entry.table_name.clone(),
                    target_class: entry.table_name.clone(),
                    target_function: String::new(),
                    status,
                    reason,
                    suggestion,
                    category: "palschema".to_string(),
                });
            }

            // 3. Validate Pak Assets & Schema Notices
            let mut seen_pak_assets = HashSet::new();
            let mut broken_pak_entries = HashSet::new();

            // First add broken schema notices
            for notice in &schema_notices {
                let suggestion = sdk_index.as_ref().and_then(|s| s.suggest_similar_class(&notice.struct_name));
                broken_hooks += 1;
                broken_pak_entries.insert((notice.mod_id.clone(), notice.asset_path.clone()));
                diagnostics.push(UsmapHookDiagnostic {
                    mod_id: notice.mod_id.clone(),
                    mod_name: notice.mod_name.clone(),
                    file_path: notice.asset_path.clone(),
                    line_number: 1,
                    hook_target: format!("{} ({})", notice.asset_path, notice.struct_name),
                    target_class: notice.struct_name.clone(),
                    target_function: String::new(),
                    status: "broken_struct".to_string(),
                    reason: notice.message.clone(),
                    suggestion,
                    category: "pak".to_string(),
                });
            }

            // Next validate all active pak assets
            for m in &profile_mods {
                if !m.enabled || m.game_path.is_empty() {
                    continue;
                }
                let mut paks = Vec::new();
                let is_pak_or_zen = |path_str: &str| {
                    let lower = path_str.to_lowercase();
                    lower.ends_with(".pak") || lower.ends_with(".utoc") || lower.ends_with(".ucas")
                };
                if is_pak_or_zen(&m.game_path) {
                    paks.push(PathBuf::from(&m.game_path));
                }
                for extra in &m.extra_files {
                    if is_pak_or_zen(extra) {
                        paks.push(PathBuf::from(extra));
                    }
                }

                for p in paks {
                    if p.exists() {
                        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if ext.eq_ignore_ascii_case("pak") {
                            if let Ok(entries) = crate::pak_scanner::list_pak_entries(&p) {
                                for entry in entries {
                                    let entry_lower = entry.to_lowercase();
                                    if entry_lower.ends_with(".uasset") {
                                        let dedup = (m.id.clone(), entry.clone());
                                        if seen_pak_assets.insert(dedup.clone()) && !broken_pak_entries.contains(&dedup) {
                                            valid_hooks += 1;
                                            diagnostics.push(UsmapHookDiagnostic {
                                                mod_id: m.id.clone(),
                                                mod_name: m.name.clone(),
                                                file_path: entry.clone(),
                                                line_number: 1,
                                                hook_target: entry.clone(),
                                                target_class: entry.clone(),
                                                target_function: String::new(),
                                                status: "valid".to_string(),
                                                reason: "Verified Pak game asset in Palworld package hierarchy".to_string(),
                                                suggestion: None,
                                                category: "pak".to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 4. Add Deprecated UE4SS APIs & Blind pcall Anti-Patterns
            for dep in deprecated_diagnostics {
                broken_hooks += 1;
                diagnostics.push(dep);
            }

            diagnostics.sort_by(|a, b| {
                let status_order = |s: &str| match s {
                    "broken_class" => 0,
                    "broken_function" => 1,
                    "broken_table" => 2,
                    "broken_struct" => 3,
                    "deprecated_api" => 4,
                    "blind_pcall" => 5,
                    "unknown" => 6,
                    "blueprint_asset" => 7,
                    "valid" => 8,
                    _ => 9,
                };
                status_order(&a.status)
                    .cmp(&status_order(&b.status))
                    .then_with(|| a.mod_name.cmp(&b.mod_name))
            });

            Some(UsmapDiagnosticSummary {
                has_usmap: true,
                usmap_version: schema.game_version,
                total_hooks_checked: diagnostics.len() as u32,
                valid_hooks,
                broken_hooks,
                diagnostics,
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok(ScanResult {
        total_scanned,
        palschema_scanned,
        ue4ss_scanned,
        pak_scanned,
        table_conflicts,
        hook_conflicts,
        pak_conflicts,
        internal_table_conflicts,
        internal_hook_conflicts,
        warnings,
        mod_summaries,
        gamepass_notices,
        schema_notices,
        usmap_diagnostics,
        is_gamepass,
    })
}

fn scan_palschema_mod(
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

fn extract_palschema_rows(
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

fn scan_ue4ss_mod(
    mod_path: &Path,
    scripts_path: &Path,
    conflict_info: &ConflictingMod,
    hook_map: &mut HookMap,
    deprecated_diagnostics: &mut Vec<UsmapHookDiagnostic>,
    _warnings: &mut Vec<String>,
) {
    let mut files_to_scan = Vec::new();
    collect_files_with_extensions(scripts_path, &["lua"], &mut files_to_scan);

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

    while line_num < lines.len() {
        let trimmed = lines[line_num].trim();
        line_num += 1;

        if trimmed.starts_with("--") {
            continue;
        }

        for api in &["RegisterHook", "NotifyOnNewObject"] {
            let mut search_from = 0;
            while let Some(rel_idx) = trimmed[search_from..].find(api) {
                let idx = search_from + rel_idx;
                search_from = idx + api.len();

                // Ensure boundary before api (avoid false positives on MyCustomRegisterHook)
                if idx > 0 {
                    let prev_byte = trimmed.as_bytes()[idx - 1];
                    if prev_byte.is_ascii_alphanumeric() || prev_byte == b'_' {
                        continue;
                    }
                }

                if let Some((target, decl_code)) = find_hook_target_string(&lines, line_num - 1, idx + api.len()) {
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
                        results.push((api.to_string(), clean_target.to_string(), line_num as u32, line_code));
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
