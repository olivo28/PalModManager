use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;
use crate::models::ModType;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConflictingMod {
    pub mod_id: String,
    pub mod_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub detail: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TableRowConflict {
    pub table_name: String,
    pub row_name: String,
    pub mods: Vec<ConflictingMod>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HookConflict {
    pub hook_target: String,
    pub hook_fn: String,
    pub mods: Vec<ConflictingMod>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModSummary {
    pub mod_id: String,
    pub mod_name: String,
    pub mod_type: String,
    pub palschema_rows: Vec<String>,
    pub ue4ss_hooks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub total_scanned: u32,
    pub palschema_scanned: u32,
    pub ue4ss_scanned: u32,
    pub table_conflicts: Vec<TableRowConflict>,
    pub hook_conflicts: Vec<HookConflict>,
    pub warnings: Vec<String>,
    pub mod_summaries: Vec<ModSummary>,
}

#[tauri::command]
pub async fn scan_conflicts(state: State<'_, AppState>) -> Result<ScanResult, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    
    // Obtain active mods for current profile
    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    
    let mut table_map: HashMap<String, Vec<ConflictingMod>> = HashMap::new();
    let mut hook_map: HashMap<String, (String, Vec<ConflictingMod>)> = HashMap::new();
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
                if let Some(parent) = mod_path.parent() {
                    target_palschema_path = parent.join("PalSchema").join("mods").join(mod_path.file_name().unwrap());
                }
            }
            if target_palschema_path.exists() {
                scan_palschema_mod(&target_palschema_path, &conflict_info, &mut table_map, &mut warnings);
            }
        }

        if is_ue4ss {
            ue4ss_scanned += 1;
            scan_ue4ss_mod(mod_path, &conflict_info, &mut hook_map, &mut warnings);
        }
    }

    // Filter conflicts
    let mut table_conflicts = Vec::new();
    for (entry_key, mods) in table_map.clone() {
        if mods.len() > 1 {
            let parts: Vec<&str> = entry_key.split("::").collect();
            let table_name = parts.get(0).copied().unwrap_or("Unknown").to_string();
            let row_name = parts.get(1).copied().unwrap_or("Unknown").to_string();
            table_conflicts.push(TableRowConflict {
                table_name,
                row_name,
                mods,
            });
        }
    }

    let mut hook_conflicts = Vec::new();
    for (hook_target, (hook_fn, mods)) in hook_map.clone() {
        if mods.len() > 1 {
            hook_conflicts.push(HookConflict {
                hook_target,
                hook_fn,
                mods,
            });
        }
    }

    // Build mod summaries map
    let mut summaries_map: HashMap<String, (String, String, Vec<String>, Vec<String>)> = HashMap::new();
    for m in &profile_mods {
        if m.enabled && !m.game_path.is_empty() && m.nexus_author.as_deref() != Some("UE4SS Native Mod") {
            if m.mod_type == ModType::PalSchema || m.mod_type == ModType::Ue4ss || m.mod_type == ModType::Hybrid {
                let type_str = format!("{:?}", m.mod_type);
                summaries_map.insert(m.id.clone(), (m.name.clone(), type_str, Vec::new(), Vec::new()));
            }
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
    for (mod_id, (mod_name, mod_type, mut rows, mut hooks)) in summaries_map {
        // Skip if mod registry contains absolutely no rows and no hooks
        if rows.is_empty() && hooks.is_empty() {
            continue;
        }
        rows.sort();
        hooks.sort();
        mod_summaries.push(ModSummary {
            mod_id,
            mod_name,
            mod_type,
            palschema_rows: rows,
            ue4ss_hooks: hooks,
        });
    }
    mod_summaries.sort_by(|a, b| a.mod_name.cmp(&b.mod_name));

    Ok(ScanResult {
        total_scanned,
        palschema_scanned,
        ue4ss_scanned,
        table_conflicts,
        hook_conflicts,
        warnings,
        mod_summaries,
    })
}

fn scan_palschema_mod(
    mod_path: &Path,
    conflict_info: &ConflictingMod,
    table_map: &mut HashMap<String, Vec<ConflictingMod>>,
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
            let stripped = strip_jsonc_comments(&content);
            match serde_json::from_str::<serde_json::Value>(&stripped) {
                Ok(val) => {
                    let mut file_info = conflict_info.clone();
                    if let Ok(rel) = file_path.strip_prefix(mod_path) {
                        file_info.file_path = rel.to_string_lossy().to_string();
                    } else {
                        file_info.file_path = file_path.to_string_lossy().to_string();
                    }
                    extract_palschema_rows(&file_path, &val, &file_info, table_map, warnings);
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
    mod_info: &ConflictingMod,
    table_map: &mut HashMap<String, Vec<ConflictingMod>>,
    _warnings: &mut Vec<String>,
) {
    let filename = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    if let Some(obj) = json_val.as_object() {
        for (key, val) in obj {
            let is_dt = key.starts_with("DT_") || key.contains("DataTable");
            let is_bp = key.starts_with("BP_") || key.ends_with("_C");
            
            if (is_dt || is_bp) && val.is_object() {
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
    conflict_info: &ConflictingMod,
    hook_map: &mut HashMap<String, (String, Vec<ConflictingMod>)>,
    _warnings: &mut Vec<String>,
) {
    let scripts_path = mod_path.join("Scripts");
    if !scripts_path.exists() {
        return;
    }

    let mut files_to_scan = Vec::new();
    collect_files_with_extensions(&scripts_path, &["lua"], &mut files_to_scan);

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
        }
    }
}

fn extract_literal_hooks(content: &str) -> Vec<(String, String, u32, String)> {
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
            if let Some(idx) = trimmed.find(api) {
                let rest = &trimmed[idx + api.len()..];
                if let Some(start_paren) = rest.find('(') {
                    let arg_part = rest[start_paren + 1..].trim();
                    let quote = if arg_part.starts_with('"') {
                        Some('"')
                    } else if arg_part.starts_with('\'') {
                        Some('\'')
                    } else {
                        None
                    };
                    
                    if let Some(q) = quote {
                        let quote_str = &arg_part[1..];
                        if let Some(end_quote) = quote_str.find(q) {
                            let target = &quote_str[..end_quote];
                            if target.contains('/') || target.contains(':') {
                                let mut code = trimmed.to_string();
                                if !trimmed.contains("function") && !trimmed.contains(')') && line_num < lines.len() {
                                    let next_trimmed = lines[line_num].trim();
                                    code = format!("{} {}", trimmed, next_trimmed);
                                }
                                let line_code = if code.len() > 120 {
                                    format!("{}...", &code[..120])
                                } else {
                                    code
                                };
                                results.push((api.to_string(), target.to_string(), line_num as u32, line_code));
                            }
                        }
                    }
                }
            }
        }
    }
    results
}


fn collect_files_with_extensions(dir: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_files_with_extensions(&path, extensions, files);
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    files.push(path);
                }
            }
        }
    }
}

pub fn strip_jsonc_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            output.push(c);
        } else {
            if c == '"' {
                in_string = true;
                output.push(c);
            } else if c == '/' {
                if let Some(&next_c) = chars.peek() {
                    if next_c == '/' {
                        chars.next();
                        while let Some(nc) = chars.next() {
                            if nc == '\n' || nc == '\r' {
                                output.push(nc);
                                break;
                            }
                        }
                    } else if next_c == '*' {
                        chars.next();
                        while let Some(nc) = chars.next() {
                            if nc == '*' {
                                if let Some(&next_nc) = chars.peek() {
                                    if next_nc == '/' {
                                        chars.next();
                                        break;
                                    }
                                }
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
