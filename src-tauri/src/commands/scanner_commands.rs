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
    pub pak_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub total_scanned: u32,
    pub palschema_scanned: u32,
    pub ue4ss_scanned: u32,
    pub pak_scanned: u32,
    pub table_conflicts: Vec<TableRowConflict>,
    pub hook_conflicts: Vec<HookConflict>,
    pub pak_conflicts: Vec<crate::pak_scanner::PakConflict>,
    pub internal_table_conflicts: Vec<TableRowConflict>,
    pub internal_hook_conflicts: Vec<HookConflict>,
    pub warnings: Vec<String>,
    pub mod_summaries: Vec<ModSummary>,
    pub gamepass_notices: Vec<crate::pak_scanner::GamePassPakNotice>,
    pub is_gamepass: bool,
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
                for extra in &m.extra_files {
                    if extra.contains("PalSchema") {
                        target_palschema_path = PathBuf::from(extra);
                        break;
                    }
                }
            }
            if target_palschema_path.exists() {
                scan_palschema_mod(&target_palschema_path, &conflict_info, &mut table_map, &mut warnings);
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
                    scan_ue4ss_mod(&base_path, s_path, &conflict_info, &mut hook_map, &mut warnings);
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
        is_gamepass,
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
            let content_clean = content.strip_prefix("\u{feff}").unwrap_or(&content);
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
    scripts_path: &Path,
    conflict_info: &ConflictingMod,
    hook_map: &mut HashMap<String, (String, Vec<ConflictingMod>)>,
    _warnings: &mut Vec<String>,
) {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModHotkey {
    pub mod_id: String,
    pub mod_name: String,
    pub file_path: String,
    pub absolute_file_path: String,
    pub line_number: usize,
    pub keys: String,
    pub raw_line: String,
}

fn extract_keys_from_line(line: &str) -> Option<String> {
    let rkb = "RegisterKeyBind";
    let start = line.find(rkb)?;
    let after = &line[start + rkb.len()..];
    let open_paren = after.find('(')?;
    let content = &after[open_paren + 1..];
    
    let mut parts = Vec::new();
    let mut current_part = String::new();
    let mut paren_depth = 0;
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut quote_char = ' ';
    let mut chars = content.chars().peekable();
    
    while let Some(c) = chars.next() {
        if in_string {
            if c == '\\' {
                current_part.push(c);
                if let Some(next_c) = chars.next() {
                    current_part.push(next_c);
                }
            } else if c == quote_char {
                in_string = false;
                current_part.push(c);
            } else {
                current_part.push(c);
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    quote_char = c;
                    current_part.push(c);
                }
                '(' => {
                    paren_depth += 1;
                    current_part.push(c);
                }
                ')' => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        break;
                    }
                    current_part.push(c);
                }
                '{' | '[' => {
                    brace_depth += 1;
                    current_part.push(c);
                }
                '}' | ']' => {
                    brace_depth -= 1;
                    current_part.push(c);
                }
                ',' => {
                    if paren_depth == 0 && brace_depth == 0 {
                        parts.push(current_part.trim().to_string());
                        current_part = String::new();
                    } else {
                        current_part.push(c);
                    }
                }
                _ => {
                    current_part.push(c);
                }
            }
        }
    }
    
    if !parts.is_empty() {
        let keys = parts.join(", ");
        if !keys.is_empty() {
            return Some(keys);
        }
    } else {
        let single = current_part.trim().to_string();
        if !single.is_empty() {
            return Some(single);
        }
    }
    None
}

fn update_line_keybind(line: &str, new_keys: &str) -> Option<String> {
    let rkb = "RegisterKeyBind";
    let start = line.find(rkb)?;
    let after = &line[start + rkb.len()..];
    let open_paren = after.find('(')?;
    let content = &after[open_paren + 1..];
    
    let mut last_comma_idx = None;
    let mut paren_depth = 0;
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut quote_char = ' ';
    let mut chars = content.char_indices().peekable();
    
    while let Some((i, c)) = chars.next() {
        if in_string {
            if c == '\\' {
                let _ = chars.next();
            } else if c == quote_char {
                in_string = false;
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    quote_char = c;
                }
                '(' => {
                    paren_depth += 1;
                }
                ')' => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        break;
                    }
                }
                '{' | '[' => {
                    brace_depth += 1;
                }
                '}' | ']' => {
                    brace_depth -= 1;
                }
                ',' => {
                    if paren_depth == 0 && brace_depth == 0 {
                        last_comma_idx = Some(i);
                    }
                }
                _ => {}
            }
        }
    }
    
    if let Some(comma_idx) = last_comma_idx {
        let prefix = &line[..start + rkb.len() + open_paren + 1];
        let suffix = &content[comma_idx..];
        return Some(format!("{}{}{}", prefix, new_keys, suffix));
    }
    
    None
}

#[tauri::command]
pub fn scan_mod_hotkeys(state: State<'_, AppState>) -> Result<Vec<ModHotkey>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    let mut hotkeys = Vec::new();

    for m in &profile_mods {
        if !m.enabled {
            continue;
        }
        if m.nexus_author.as_deref() == Some("UE4SS Native Mod") {
            continue;
        }
        if m.mod_type != ModType::Ue4ss && m.mod_type != ModType::Hybrid && m.mod_type != ModType::PalSchema {
            continue;
        }

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

            let Some(scripts_path) = scripts_path else {
                continue;
            };

            let mut files_to_scan = Vec::new();
            collect_files_with_extensions(&scripts_path, &["lua"], &mut files_to_scan);

            for file_path in files_to_scan {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    let lines: Vec<&str> = content.lines().collect();
                    for (idx, line) in lines.iter().enumerate() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("--") {
                            continue;
                        }
                        if trimmed.contains("RegisterKeyBind") {
                            if let Some(keys) = extract_keys_from_line(line) {
                                let rel_path = match file_path.strip_prefix(&base_path) {
                                    Ok(rel) => rel.to_string_lossy().to_string(),
                                    Err(_) => file_path.to_string_lossy().to_string(),
                                };
                                hotkeys.push(ModHotkey {
                                    mod_id: m.id.clone(),
                                    mod_name: m.name.clone(),
                                    file_path: rel_path,
                                    absolute_file_path: file_path.to_string_lossy().to_string(),
                                    line_number: idx + 1,
                                    keys,
                                    raw_line: line.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(hotkeys)
}

#[tauri::command]
pub fn update_mod_hotkey(
    absolute_file_path: String,
    line_number: usize,
    new_keys: String,
) -> Result<(), String> {
    let path = Path::new(&absolute_file_path);
    if !path.exists() {
        return Err("File not found".to_string());
    }

    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    if line_number == 0 || line_number > lines.len() {
        return Err("Invalid line number".to_string());
    }

    let target_line = &lines[line_number - 1];
    if let Some(updated_line) = update_line_keybind(target_line, &new_keys) {
        lines[line_number - 1] = updated_line;
        let ending = if content.contains("\r\n") { "\r\n" } else { "\n" };
        let new_content = lines.join(ending) + ending;
        fs::write(path, new_content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    } else {
        Err("Failed to parse RegisterKeyBind on target line".to_string())
    }
}

#[tauri::command]
pub fn inspect_pak_file_tree(
    pak_path: String,
    zip_path: Option<String>,
) -> Result<crate::pak_scanner::PakInspectionResult, String> {
    let p = Path::new(&pak_path);
    if p.exists() {
        return crate::pak_scanner::list_pak_entries_detailed(p);
    }

    // If a parent zip_path is provided and the pak is inside an archive (installation preview)
    if let Some(zp) = zip_path {
        let zip_p = Path::new(&zp);
        if zip_p.exists() {
            let temp_dir = std::env::temp_dir().join("pmm_pak_inspect").join(uuid::Uuid::new_v4().to_string());
            fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

            let result = (|| {
                let target_name = Path::new(&pak_path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .unwrap_or_else(|| pak_path.to_lowercase());

                let lower = zp.to_lowercase();
                if lower.ends_with(".zip") {
                    let file = fs::File::open(&zip_p).map_err(|e| e.to_string())?;
                    let mut archive = zip::read::ZipArchive::new(file).map_err(|e| e.to_string())?;
                    
                    let mut found = false;
                    for i in 0..archive.len() {
                        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
                        let ename = entry.name().replace('\\', "/");
                        let ename_lower = ename.to_lowercase();
                        let entry_fname = Path::new(&ename)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_lowercase())
                            .unwrap_or_default();

                        if entry_fname == target_name
                            || ename_lower.ends_with(&format!("/{}", target_name))
                            || ename.eq_ignore_ascii_case(&pak_path)
                        {
                            let temp_pak = temp_dir.join("temp_inspect.pak");
                            let mut out = fs::File::create(&temp_pak).map_err(|e| e.to_string())?;
                            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(format!("Pak file '{}' not found inside archive", pak_path));
                    }
                } else if lower.ends_with(".7z") {
                    // Extract with sevenz
                    let reader = sevenz_rust::SevenZReader::open(&zip_p, sevenz_rust::Password::empty())
                        .map_err(|e| e.to_string())?;
                    let mut found = false;
                    for entry in reader.archive().files.iter() {
                        let ename = entry.name().replace('\\', "/");
                        let ename_lower = ename.to_lowercase();
                        let entry_fname = Path::new(&ename)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_lowercase())
                            .unwrap_or_default();

                        if entry_fname == target_name
                            || ename_lower.ends_with(&format!("/{}", target_name))
                            || ename.eq_ignore_ascii_case(&pak_path)
                        {
                            sevenz_rust::decompress_file(&zip_p, &temp_dir).map_err(|e| e.to_string())?;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(format!("Pak file '{}' not found in 7z", pak_path));
                    }
                } else {
                    // .rar or other supported archive formats
                    crate::zip_handler::extract_zip_to_temp(&zp, &temp_dir).map_err(|e| e.to_string())?;
                }

                let temp_pak = temp_dir.join("temp_inspect.pak");
                if temp_pak.exists() {
                    crate::pak_scanner::list_pak_entries_detailed(&temp_pak)
                } else {
                    // Search recursively in temp_dir for extracted pak
                    let mut found_pak: Option<PathBuf> = None;
                    for entry in walkdir::WalkDir::new(&temp_dir).into_iter().flatten() {
                        if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                            let fname = entry.file_name().to_string_lossy().to_lowercase();
                            if fname == target_name {
                                found_pak = Some(entry.path().to_path_buf());
                                break;
                            } else if found_pak.is_none() {
                                found_pak = Some(entry.path().to_path_buf());
                            }
                        }
                    }
                    if let Some(target) = found_pak {
                        crate::pak_scanner::list_pak_entries_detailed(&target)
                    } else {
                        Err(format!("Could not extract pak '{}'", pak_path))
                    }
                }
            })();

            let _ = fs::remove_dir_all(&temp_dir);
            return result;
        }
    }

    Err(format!("Pak file not found at '{}'", pak_path))
}

#[tauri::command]
pub fn inspect_mod_pak_contents(
    state: State<'_, AppState>,
    mod_id: String,
) -> Result<Vec<crate::pak_scanner::PakInspectionResult>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    let target_mod = profile_mods.into_iter().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;

    let mut candidate_paks = Vec::new();
    if target_mod.game_path.to_lowercase().ends_with(".pak") {
        candidate_paks.push(PathBuf::from(&target_mod.game_path));
    }
    for extra in &target_mod.extra_files {
        if extra.to_lowercase().ends_with(".pak") {
            candidate_paks.push(PathBuf::from(extra));
        }
    }

    let mut results = Vec::new();
    for pak in candidate_paks {
        if pak.exists() {
            if let Ok(info) = crate::pak_scanner::list_pak_entries_detailed(&pak) {
                results.push(info);
            }
        }
    }

    if results.is_empty() {
        return Err("No valid .pak files found for this mod on disk".to_string());
    }

    Ok(results)
}

#[tauri::command]
pub fn inspect_pak_asset(
    state: State<'_, AppState>,
    mod_id: String,
    asset_internal_path: String,
) -> Result<Vec<String>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    let target_mod = profile_mods.into_iter().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;

    let mut candidate_paks = Vec::new();
    if target_mod.game_path.to_lowercase().ends_with(".pak") {
        candidate_paks.push(PathBuf::from(&target_mod.game_path));
    }
    for extra in &target_mod.extra_files {
        if extra.to_lowercase().ends_with(".pak") {
            candidate_paks.push(PathBuf::from(extra));
        }
    }

    for pak in candidate_paks {
        if pak.exists() {
            if let Ok(entries) = crate::pak_scanner::list_pak_entries(&pak) {
                if entries.iter().any(|e| e.eq_ignore_ascii_case(&asset_internal_path)) {
                    return crate::pak_scanner::list_pak_entries(&pak);
                }
            }
        }
    }

    Err(format!("Asset '{}' not found in mod pak archives", asset_internal_path))
}

#[tauri::command]
pub fn inspect_uasset_deep_cmd(
    state: State<'_, AppState>,
    mod_id: Option<String>,
    pak_path: Option<String>,
    asset_internal_path: String,
    zip_path: Option<String>,
) -> Result<crate::pak_scanner::UAssetInspectionDetails, String> {
    // 1. Direct pak_path if provided
    if let Some(pp) = pak_path.as_deref() {
        let p = Path::new(pp);
        if p.exists() {
            return crate::pak_scanner::inspect_uasset_deep(p, &asset_internal_path);
        }
    }

    // 2. Mod ID search
    if let Some(mid) = mod_id.as_deref() {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
        if let Some(target_mod) = profile_mods.into_iter().find(|m| m.id == mid) {
            let mut candidate_paks = Vec::new();
            if target_mod.game_path.to_lowercase().ends_with(".pak") {
                candidate_paks.push(PathBuf::from(&target_mod.game_path));
            }
            for extra in &target_mod.extra_files {
                if extra.to_lowercase().ends_with(".pak") {
                    candidate_paks.push(PathBuf::from(extra));
                }
            }

            for pak in candidate_paks {
                if pak.exists() {
                    if let Ok(entries) = crate::pak_scanner::list_pak_entries(&pak) {
                        if entries.iter().any(|e| e.eq_ignore_ascii_case(&asset_internal_path)) {
                            return crate::pak_scanner::inspect_uasset_deep(&pak, &asset_internal_path);
                        }
                    }
                }
            }
        }
    }

    // 3. Zip preview path (Installer modal)
    if let Some(zp) = zip_path {
        let zip_p = Path::new(&zp);
        if zip_p.exists() {
            let temp_dir = std::env::temp_dir().join("pmm_uasset_inspect").join(uuid::Uuid::new_v4().to_string());
            fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

            let result = (|| {
                let lower = zp.to_lowercase();
                if lower.ends_with(".zip") {
                    let file = fs::File::open(&zip_p).map_err(|e| e.to_string())?;
                    let mut archive = zip::read::ZipArchive::new(file).map_err(|e| e.to_string())?;
                    for i in 0..archive.len() {
                        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
                        let ename = entry.name().replace('\\', "/");
                        if ename.to_lowercase().ends_with(".pak") {
                            let temp_pak = temp_dir.join("temp.pak");
                            let mut out = fs::File::create(&temp_pak).map_err(|e| e.to_string())?;
                            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                            if let Ok(details) = crate::pak_scanner::inspect_uasset_deep(&temp_pak, &asset_internal_path) {
                                return Ok(details);
                            }
                        }
                    }
                } else if lower.ends_with(".7z") {
                    if sevenz_rust::decompress_file(&zip_p, &temp_dir).is_ok() {
                        for entry in walkdir::WalkDir::new(&temp_dir).into_iter().flatten() {
                            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                                if let Ok(details) = crate::pak_scanner::inspect_uasset_deep(entry.path(), &asset_internal_path) {
                                    return Ok(details);
                                }
                            }
                        }
                    }
                } else {
                    if crate::zip_handler::extract_zip_to_temp(&zp, &temp_dir).is_ok() {
                        for entry in walkdir::WalkDir::new(&temp_dir).into_iter().flatten() {
                            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                                if let Ok(details) = crate::pak_scanner::inspect_uasset_deep(entry.path(), &asset_internal_path) {
                                    return Ok(details);
                                }
                            }
                        }
                    }
                }
                Err(format!("Asset '{}' not found inside archive pak files", asset_internal_path))
            })();

            let _ = fs::remove_dir_all(&temp_dir);
            return result;
        }
    }

    Err(format!("Could not locate pak containing asset '{}'", asset_internal_path))
}

#[tauri::command]
pub async fn convert_mod_to_gamepass(
    state: State<'_, AppState>,
    mod_id: String,
) -> Result<Vec<String>, String> {
    let (app_data_dir, candidate_paks, game_root) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let prog_path = if !data.settings.program_path.is_empty() {
            PathBuf::from(&data.settings.program_path)
        } else {
            PathBuf::from(".")
        };
        let root = PathBuf::from(&data.settings.game_path);
        let mut paks = Vec::new();
        if let Some(target_mod) = data.mods.iter().find(|m| m.id == mod_id) {
            if target_mod.game_path.to_lowercase().ends_with(".pak") {
                paks.push(PathBuf::from(&target_mod.game_path));
            }
            if target_mod.disabled_path.to_lowercase().ends_with(".pak") {
                paks.push(PathBuf::from(&target_mod.disabled_path));
            }
            for extra in &target_mod.extra_files {
                if extra.to_lowercase().ends_with(".pak") {
                    paks.push(PathBuf::from(extra));
                }
            }
        }
        (prog_path, paks, root)
    };

    if candidate_paks.is_empty() {
        return Err("No .pak files found for this mod".to_string());
    }

    let mut generated_files = Vec::new();
    for pak_path in candidate_paks {
        let full_path = if pak_path.is_absolute() {
            pak_path
        } else {
            game_root.join(&pak_path)
        };

        if full_path.exists() {
            let (utoc, ucas) = crate::retoc_runner::convert_pak_to_gamepass_zen(&full_path, &app_data_dir).await?;
            generated_files.push(utoc.to_string_lossy().to_string());
            generated_files.push(ucas.to_string_lossy().to_string());
        }
    }

    if generated_files.is_empty() {
        return Err("Could not find on-disk .pak files to convert".to_string());
    }

    // Register generated extra files in AppData
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let program_path = data.settings.program_path.clone();
        if let Some(m) = data.mods.iter_mut().find(|m| m.id == mod_id) {
            for gen in &generated_files {
                if !m.extra_files.contains(gen) {
                    m.extra_files.push(gen.clone());
                }
            }
        }
        let data_clone = data.clone();
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    Ok(generated_files)
}

#[tauri::command]
pub async fn convert_all_gamepass_mods(
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let (game_path, profile_mods) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let p_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
        (data.settings.game_path.clone(), p_mods)
    };

    if game_path.is_empty() {
        return Err("Game path is not configured".to_string());
    }

    let (_, notices) = crate::pak_scanner::check_gamepass_pak_compatibility(Path::new(&game_path), &profile_mods);
    if notices.is_empty() {
        return Ok(0);
    }

    let mut count = 0;
    for notice in notices {
        if notice.mod_id != "untracked" {
            let _ = convert_mod_to_gamepass(state.clone(), notice.mod_id).await;
            count += 1;
        } else {
            let app_data_dir = {
                let data = state.data.lock().map_err(|e| e.to_string())?;
                if !data.settings.program_path.is_empty() {
                    PathBuf::from(&data.settings.program_path)
                } else {
                    PathBuf::from(".")
                }
            };
            let pak_path = PathBuf::from(&notice.pak_path);
            if pak_path.exists() {
                if let Ok(_) = crate::retoc_runner::convert_pak_to_gamepass_zen(&pak_path, &app_data_dir).await {
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

#[tauri::command]
pub fn list_save_worlds_cmd(
    custom_dir: Option<String>,
) -> Result<Vec<crate::save_scanner::SaveWorldSummary>, String> {
    crate::save_scanner::list_save_worlds(custom_dir.as_deref())
}

#[tauri::command]
pub fn deep_scan_save_cmd(
    state: State<'_, AppState>,
    world_dir: String,
) -> Result<crate::save_scanner::SaveHealthReport, String> {
    let active_mod_names = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let p_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
        p_mods.into_iter().filter(|m| m.enabled).map(|m| m.name).collect::<Vec<String>>()
    };

    crate::save_scanner::deep_scan_save(&world_dir, &active_mod_names)
}

#[tauri::command]
pub fn repair_save_cmd(
    state: State<'_, AppState>,
    world_dir: String,
) -> Result<crate::save_scanner::SaveRepairResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };

    crate::save_scanner::repair_and_sanitize_save(&world_dir, &program_path)
}

#[tauri::command]
pub fn restore_save_backup_cmd(
    state: State<'_, AppState>,
    world_dir: String,
    backup_slot: String,
) -> Result<crate::save_scanner::SaveRepairResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };

    crate::save_scanner::restore_save_from_backup(&world_dir, &backup_slot, &program_path)
}

#[tauri::command]
pub fn create_world_backup_cmd(
    state: State<'_, AppState>,
    world_dir: String,
    custom_dest: Option<String>,
) -> Result<String, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    crate::save_scanner::create_manual_world_backup(&world_dir, &program_path, custom_dest.as_deref())
}

#[tauri::command]
pub fn open_world_folder_cmd(world_dir: String) -> Result<(), String> {
    crate::save_scanner::open_world_folder(&world_dir)
}

#[tauri::command]
pub fn export_world_zip_cmd(world_dir: String, target_path: String) -> Result<String, String> {
    crate::save_scanner::create_manual_world_backup(&world_dir, ".", Some(&target_path))
}

#[tauri::command]
pub fn prune_world_backups_cmd(world_dir: String, keep_count: usize) -> Result<usize, String> {
    crate::save_scanner::prune_world_backups(&world_dir, keep_count)
}

#[tauri::command]
pub fn save_world_custom_meta_cmd(world_dir: String, meta: crate::save_scanner::WorldCustomMeta) -> Result<(), String> {
    crate::save_scanner::save_world_custom_meta(Path::new(&world_dir), &meta)
}

#[tauri::command]
pub fn get_world_custom_meta_cmd(world_dir: String) -> Result<crate::save_scanner::WorldCustomMeta, String> {
    Ok(crate::save_scanner::load_world_custom_meta(Path::new(&world_dir)))
}

#[tauri::command]
pub fn inspect_snapshot_details_cmd(world_dir: String, slot_name: String) -> Result<crate::save_scanner::SaveBackupSnapshot, String> {
    crate::save_scanner::inspect_snapshot_details(Path::new(&world_dir), &slot_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_pak_from_rar() {
        let rar_file = "C:/Users/Antikux/Downloads/Modern Wooden Building - Steam Version(Emprxss) 5366 2 2026-08-27T18-44Z mu9Q1NbQr.rar";
        if !std::path::Path::new(rar_file).exists() {
            return;
        }
        let res = inspect_pak_file_tree("Emprxss_Wooden_Modern_Building_P.pak".to_string(), Some(rar_file.to_string()));
        println!("RAR Inspection result: {:?}", res);
        assert!(res.is_ok(), "Should inspect pak inside RAR");
        let inspection = res.unwrap();
        assert!(!inspection.files.is_empty(), "Files should not be empty");

        // Inspect uasset inside RAR pak
        let temp_dir = std::env::temp_dir().join("pmm_uasset_inspect_test");
        let _ = fs::create_dir_all(&temp_dir);
        let ext_res = crate::zip_handler::extract_zip_to_temp(rar_file, &temp_dir).expect("Should extract rar");
        let pak_path = ext_res.join("Pal/Content/Paks/~mods/Emprxss_Wooden_Modern_Building_P.pak");
        let pak_real = if pak_path.exists() {
            pak_path
        } else {
            walkdir::WalkDir::new(&temp_dir)
                .into_iter()
                .flatten()
                .find(|e| e.path().extension().map_or(false, |ext| ext == "pak"))
                .map(|e| e.path().to_path_buf())
                .unwrap()
        };

        let uasset_details = crate::pak_scanner::inspect_uasset_deep(&pak_real, "Model/Prop/Architecture/Architecture_Wood/Material/MI_PalProp_DoorBase_Wood.uasset");
        println!("Uasset details: {:?}", uasset_details);
        assert!(uasset_details.is_ok(), "Should parse uasset inside pak");
        let _ = fs::remove_dir_all(&temp_dir);
    }
}



