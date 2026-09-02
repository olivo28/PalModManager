use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
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
pub struct UsmapHookDiagnostic {
    pub mod_id: String,
    pub mod_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub hook_target: String,
    pub target_class: String,
    pub target_function: String,
    pub status: String,
    pub reason: String,
    pub suggestion: Option<String>,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct PalschemaTableEntry {
    pub mod_id: String,
    pub mod_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub table_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsmapDiagnosticSummary {
    pub has_usmap: bool,
    pub usmap_version: String,
    pub total_hooks_checked: u32,
    pub valid_hooks: u32,
    pub broken_hooks: u32,
    pub diagnostics: Vec<UsmapHookDiagnostic>,
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
    pub schema_notices: Vec<crate::pak_scanner::DeprecatedSchemaNotice>,
    pub usmap_diagnostics: Option<UsmapDiagnosticSummary>,
    pub is_gamepass: bool,
}

#[tauri::command]
pub async fn scan_conflicts(state: State<'_, AppState>) -> Result<ScanResult, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    
    // Obtain active mods for current profile
    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    
    let mut table_map: HashMap<String, Vec<ConflictingMod>> = HashMap::new();
    let mut hook_map: HashMap<String, (String, Vec<ConflictingMod>)> = HashMap::new();
    let mut palschema_tables: Vec<PalschemaTableEntry> = Vec::new();
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
                let (target_class, target_function, status, reason, suggestion) = validate_hook_with_usmap(hook_target, &schema, sdk_index.as_ref());
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

                let (status, reason, suggestion) = validate_palschema_table_with_usmap(&entry.table_name, &schema, sdk_index.as_ref());
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

            // 3. Validate Pak Assets & Deprecated Schema Notices
            for notice in &schema_notices {
                let suggestion = sdk_index.as_ref().and_then(|s| s.suggest_similar_class(&notice.struct_name));
                broken_hooks += 1;
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

            diagnostics.sort_by(|a, b| {
                let status_order = |s: &str| match s {
                    "broken_class" => 0,
                    "broken_function" => 1,
                    "broken_table" => 2,
                    "broken_struct" => 3,
                    "unknown" => 4,
                    "blueprint_asset" => 5,
                    "valid" => 6,
                    _ => 7,
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

fn validate_hook_with_usmap(
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
    let clean_class = if let Some(dot_idx) = without_script.rfind('.') {
        &without_script[dot_idx + 1..]
    } else if without_script.starts_with("PalPal") {
        &without_script[3..]
    } else {
        without_script
    }.trim();

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

fn validate_palschema_table_with_usmap(
    table_name: &str,
    schema: &crate::usmap::parser::UsmapSchema,
    sdk: Option<&crate::usmap::SdkIndex>,
) -> (String, String, Option<String>) {
    let raw = table_name.trim();
    if raw.is_empty() {
        return ("unknown".to_string(), "Empty table name".to_string(), None);
    }

    // 1. Direct match in USMAP schema FNames (e.g. DT_PalCharacterParameter, DT_ItemData)
    let in_schema_names = schema.names.iter().any(|n| n.eq_ignore_ascii_case(raw));
    let in_schema_structs = schema.find_struct(raw).is_some();

    // 2. Check stripped name without DT_ (e.g. PalIndividualCharacterParameter, ItemData)
    let clean = if raw.starts_with("DT_") {
        &raw[3..]
    } else {
        raw
    };
    let clean_in_structs = schema.find_struct(clean).is_some();
    let clean_in_names = schema.names.iter().any(|n| n.eq_ignore_ascii_case(clean));

    // 3. Check in C++ SDK
    let class_in_sdk = sdk.and_then(|s| s.find_class(clean).or_else(|| s.find_class(raw)));

    if in_schema_names || in_schema_structs || clean_in_structs || clean_in_names || class_in_sdk.is_some() {
        return (
            "valid".to_string(),
            format!("Verified DataTable '{}' in Palworld reflection schema", raw),
            None,
        );
    }

    // If not found, it's a broken table name / obsolete table
    // Try fuzzy match in schema.names (prioritizing DT_ entries or Pal... entries)
    let raw_norm = raw.replace('_', "").to_ascii_lowercase();
    let mut best_candidate: Option<String> = None;
    let mut highest_score: f64 = 0.0;

    for name in &schema.names {
        if !name.starts_with("DT_") && !name.contains("DataTable") && !name.starts_with("Pal") {
            continue;
        }
        let cand_norm = name.replace('_', "").to_ascii_lowercase();
        if raw_norm == cand_norm {
            best_candidate = Some(name.clone());
            break;
        }
        let max_len = raw_norm.len().max(cand_norm.len());
        if max_len == 0 {
            continue;
        }
        let dist = crate::usmap::sdk_parser::levenshtein_distance(&raw_norm, &cand_norm);
        let score = 1.0 - (dist as f64 / max_len as f64);
        if score > highest_score && score >= 0.55 {
            highest_score = score;
            best_candidate = Some(name.clone());
        }
    }

    // If still no candidate, check SDK classes
    if best_candidate.is_none() {
        if let Some(sdk_idx) = sdk {
            if let Some(sug) = sdk_idx.suggest_similar_class(clean) {
                best_candidate = Some(if raw.starts_with("DT_") { format!("DT_{}", sug) } else { sug });
            }
        }
    }

    (
        "broken_table".to_string(),
        format!("DataTable '{}' does not exist in Palworld reflection schema", raw),
        best_candidate,
    )
}

fn scan_palschema_mod(
    mod_path: &Path,
    conflict_info: &ConflictingMod,
    table_map: &mut HashMap<String, Vec<ConflictingMod>>,
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
    table_map: &mut HashMap<String, Vec<ConflictingMod>>,
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
    pub variable_name: Option<String>,
    pub definition_file_path: Option<String>,
    pub definition_absolute_path: Option<String>,
    pub definition_line_number: Option<usize>,
    pub is_variable: bool,
}

fn find_variable_in_lua_files(
    var_name: &str,
    lua_files: &[(PathBuf, String)],
    depth: usize,
) -> Option<(String, PathBuf, usize)> {
    if depth > 5 || var_name.is_empty() {
        return None;
    }

    let clean_var = var_name.trim();

    if clean_var.contains("Key.") || clean_var.contains("ModifierKey.") {
        return None;
    }

    let (table_prefix, field_name) = if let Some(dot_idx) = clean_var.find('.') {
        (&clean_var[..dot_idx], &clean_var[dot_idx + 1..])
    } else {
        ("", clean_var)
    };

    for (file_path, content) in lua_files {
        let lines: Vec<&str> = content.lines().collect();
        for (line_idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("--") {
                continue;
            }

            // Pattern 1: Direct assignment e.g. "Config.OpenMenuKey = Key.F5" or "local OpenMenuKey = Key.F5"
            let is_match = if !table_prefix.is_empty() {
                trimmed.starts_with(&format!("{}.{} ", table_prefix, field_name))
                    || trimmed.starts_with(&format!("{}.{}=", table_prefix, field_name))
                    || trimmed.starts_with(&format!("{}.{}\t", table_prefix, field_name))
            } else {
                trimmed.starts_with(&format!("local {} ", field_name))
                    || trimmed.starts_with(&format!("local {}=", field_name))
                    || trimmed.starts_with(&format!("local {}\t", field_name))
                    || trimmed.starts_with(&format!("{} ", field_name))
                    || trimmed.starts_with(&format!("{}=", field_name))
                    || trimmed.starts_with(&format!("{}\t", field_name))
            };

            if is_match {
                if let Some(eq_pos) = trimmed.find('=') {
                    let mut rhs = trimmed[eq_pos + 1..].trim();
                    if let Some(c_pos) = rhs.find("--") {
                        rhs = rhs[..c_pos].trim();
                    }
                    rhs = rhs.trim_end_matches(',').trim_end_matches(';').trim();

                    if !rhs.is_empty() {
                        if rhs.contains("Key.") || rhs.contains("ModifierKey.") || rhs.starts_with('"') || rhs.starts_with('\'') || rhs.starts_with('{') {
                            return Some((rhs.to_string(), file_path.clone(), line_idx + 1));
                        } else if !rhs.contains('(') && !rhs.contains(' ') {
                            if let Some(sub_res) = find_variable_in_lua_files(rhs, lua_files, depth + 1) {
                                return Some(sub_res);
                            }
                        }
                    }
                }
            }

            // Pattern 2: Field inside table constructor e.g. OpenMenuKey = Key.F5 inside Config = { ... }
            if !table_prefix.is_empty() {
                if trimmed.starts_with(&format!("{} =", field_name))
                    || trimmed.starts_with(&format!("{}=", field_name))
                    || trimmed.starts_with(&format!("{}\t=", field_name))
                {
                    if let Some(eq_pos) = trimmed.find('=') {
                        let mut rhs = trimmed[eq_pos + 1..].trim();
                        if let Some(c_pos) = rhs.find("--") {
                            rhs = rhs[..c_pos].trim();
                        }
                        rhs = rhs.trim_end_matches(',').trim_end_matches(';').trim();

                        if rhs.contains("Key.") || rhs.contains("ModifierKey.") || rhs.starts_with('"') || rhs.starts_with('\'') || rhs.starts_with('{') {
                            return Some((rhs.to_string(), file_path.clone(), line_idx + 1));
                        }
                    }
                }
            }
        }
    }

    None
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

            // Preload all .lua files and their contents for this mod
            let mut mod_lua_files: Vec<(PathBuf, String)> = Vec::new();
            for file_path in &files_to_scan {
                if let Ok(content) = fs::read_to_string(file_path) {
                    mod_lua_files.push((file_path.clone(), content));
                }
            }

            for (file_path, content) in &mod_lua_files {
                let lines: Vec<&str> = content.lines().collect();
                for (idx, line) in lines.iter().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("--") {
                        continue;
                    }
                    if trimmed.contains("RegisterKeyBind") {
                        if let Some(raw_keys) = extract_keys_from_line(line) {
                            let rel_path = match file_path.strip_prefix(&base_path) {
                                Ok(rel) => rel.to_string_lossy().to_string(),
                                Err(_) => file_path.to_string_lossy().to_string(),
                            };

                            let is_direct_key = raw_keys.contains("Key.")
                                || raw_keys.contains("ModifierKey.")
                                || raw_keys.starts_with('"')
                                || raw_keys.starts_with('\'');

                            let (final_keys, var_name, def_file_rel, def_file_abs, def_line, is_var) = if !is_direct_key {
                                if let Some((resolved, def_path, def_line_num)) = find_variable_in_lua_files(&raw_keys, &mod_lua_files, 0) {
                                    let def_rel = match def_path.strip_prefix(&base_path) {
                                        Ok(rel) => rel.to_string_lossy().to_string(),
                                        Err(_) => def_path.to_string_lossy().to_string(),
                                    };
                                    (resolved, Some(raw_keys.clone()), Some(def_rel), Some(def_path.to_string_lossy().to_string()), Some(def_line_num), true)
                                } else {
                                    (raw_keys.clone(), Some(raw_keys.clone()), None, None, None, false)
                                }
                            } else {
                                (raw_keys.clone(), None, None, None, None, false)
                            };

                            hotkeys.push(ModHotkey {
                                mod_id: m.id.clone(),
                                mod_name: m.name.clone(),
                                file_path: rel_path,
                                absolute_file_path: file_path.to_string_lossy().to_string(),
                                line_number: idx + 1,
                                keys: final_keys,
                                raw_line: line.to_string(),
                                variable_name: var_name,
                                definition_file_path: def_file_rel,
                                definition_absolute_path: def_file_abs,
                                definition_line_number: def_line,
                                is_variable: is_var,
                            });
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
    is_variable: Option<bool>,
    _variable_name: Option<String>,
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

    let updated_line = if is_variable == Some(true) {
        if let Some(eq_idx) = target_line.find('=') {
            let left = &target_line[..=eq_idx];
            let comment_part = if let Some(c_idx) = target_line.find("--") {
                if c_idx > eq_idx {
                    format!(" {}", &target_line[c_idx..])
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let trimmed_end = target_line.trim_end();
            let ends_with_comma = trimmed_end.ends_with(',') || (trimmed_end.contains("--") && trimmed_end.split("--").next().unwrap_or("").trim().ends_with(','));
            let comma_suffix = if ends_with_comma { "," } else { "" };
            format!("{} {}{}{}", left, new_keys, comma_suffix, comment_part)
        } else {
            update_line_keybind(target_line, &new_keys).unwrap_or_else(|| target_line.clone())
        }
    } else {
        match update_line_keybind(target_line, &new_keys) {
            Some(line) => line,
            None => return Err("Failed to parse RegisterKeyBind on target line".to_string()),
        }
    };

    lines[line_number - 1] = updated_line;
    let ending = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let new_content = lines.join(ending) + ending;
    fs::write(path, new_content).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
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
pub fn decode_uasset_texture_cmd(
    state: State<'_, AppState>,
    mod_id: Option<String>,
    pak_path: Option<String>,
    asset_internal_path: String,
    zip_path: Option<String>,
) -> Result<crate::texture_decoder::TexturePreviewInfo, String> {
    let target_base = {
        let l = asset_internal_path.to_lowercase();
        if l.ends_with(".uasset") {
            l[..l.len() - 7].to_string()
        } else if l.ends_with(".ubulk") || l.ends_with(".uptnl") {
            l[..l.len() - 6].to_string()
        } else if l.ends_with(".uexp") {
            l[..l.len() - 5].to_string()
        } else {
            l
        }
    };

    // 1. Direct pak_path if provided
    if let Some(pp) = pak_path.as_deref() {
        let p = Path::new(pp);
        if p.exists() {
            let names = crate::pak_scanner::list_pak_entries(p).unwrap_or_default();
            return crate::texture_decoder::extract_and_decode_texture(p, &asset_internal_path, &names);
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
                        if entries.iter().any(|e| {
                            let el = e.to_lowercase();
                            el == asset_internal_path.to_lowercase() || el.starts_with(&target_base)
                        }) {
                            return crate::texture_decoder::extract_and_decode_texture(&pak, &asset_internal_path, &entries);
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
            let temp_dir = std::env::temp_dir().join("pmm_tex_decode").join(uuid::Uuid::new_v4().to_string());
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
                            let names = crate::pak_scanner::list_pak_entries(&temp_pak).unwrap_or_default();
                            if names.iter().any(|e| e.to_lowercase().starts_with(&target_base)) {
                                if let Ok(tex) = crate::texture_decoder::extract_and_decode_texture(&temp_pak, &asset_internal_path, &names) {
                                    return Ok(tex);
                                }
                            }
                        }
                    }
                } else if lower.ends_with(".7z") {
                    if sevenz_rust::decompress_file(&zip_p, &temp_dir).is_ok() {
                        for entry in walkdir::WalkDir::new(&temp_dir).into_iter().flatten() {
                            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                                let names = crate::pak_scanner::list_pak_entries(entry.path()).unwrap_or_default();
                                if names.iter().any(|e| e.to_lowercase().starts_with(&target_base)) {
                                    if let Ok(tex) = crate::texture_decoder::extract_and_decode_texture(entry.path(), &asset_internal_path, &names) {
                                        return Ok(tex);
                                    }
                                }
                            }
                        }
                    }
                }
                Err("Texture not found in package".to_string())
            })();
            let _ = fs::remove_dir_all(&temp_dir);
            return result;
        }
    }

    Err(format!("Texture asset '{}' not found in mod pak archives", asset_internal_path))
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
    state: State<'_, AppState>,
    custom_dir: Option<String>,
) -> Result<Vec<crate::save_scanner::SaveWorldSummary>, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    crate::save_scanner::list_save_worlds(custom_dir.as_deref(), Some(&program_path))
}

#[tauri::command]
pub fn deep_scan_save_cmd(
    state: State<'_, AppState>,
    world_dir: String,
) -> Result<crate::save_scanner::SaveHealthReport, String> {
    let (active_mod_names, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let p_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
        let names = p_mods.into_iter().filter(|m| m.enabled).map(|m| m.name).collect::<Vec<String>>();
        let prog_p = if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        };
        (names, prog_p)
    };

    crate::save_scanner::deep_scan_save(&world_dir, &active_mod_names, Some(&program_path))
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
pub fn list_pmm_world_backups_cmd(
    state: State<'_, AppState>,
    world_name_filter: Option<String>,
) -> Result<Vec<crate::save_scanner::PmmWorldBackup>, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    Ok(crate::save_scanner::list_pmm_world_backups(&program_path, world_name_filter.as_deref()))
}

#[tauri::command]
pub fn restore_pmm_world_backup_cmd(
    state: State<'_, AppState>,
    world_dir: String,
    backup_file_path: String,
) -> Result<crate::save_scanner::SaveRepairResult, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    crate::save_scanner::restore_pmm_world_backup(&world_dir, &backup_file_path, &program_path)
}

#[tauri::command]
pub fn delete_pmm_world_backup_cmd(
    backup_file_path: String,
) -> Result<(), String> {
    crate::save_scanner::delete_pmm_world_backup(&backup_file_path)
}

#[tauri::command]
pub fn open_pmm_world_backups_folder_cmd(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        if !data.settings.program_path.is_empty() {
            data.settings.program_path.clone()
        } else {
            ".".to_string()
        }
    };
    let backups_dir = Path::new(&program_path).join("backups").join("worlds");
    if !backups_dir.exists() {
        let _ = std::fs::create_dir_all(&backups_dir);
    }
    open::that(&backups_dir).map_err(|e| e.to_string())
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

#[tauri::command]
pub async fn build_compatibility_pak_cmd(
    request: crate::pak_patcher::PatchBuildRequest,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<crate::pak_patcher::PatchBuildResult, String> {
    use tauri::Manager;
    let (game_path, program_path, current_profile_id) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.game_path.clone(),
            data.settings.program_path.clone(),
            data.current_profile_id.clone(),
        )
    };
    if game_path.is_empty() {
        return Err("Game path is not configured in Settings".to_string());
    }
    let app_data_dir = if !program_path.is_empty() {
        PathBuf::from(&program_path)
    } else {
        app_handle.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    crate::pak_patcher::build_compatibility_pak_internal(
        request,
        &game_path,
        &app_data_dir,
        &program_path,
        &current_profile_id,
    ).await
}

#[tauri::command]
pub fn list_generated_patches_cmd(state: State<'_, AppState>) -> Result<Vec<crate::pak_patcher::GeneratedPatchInfo>, String> {
    let (game_path, program_path, current_profile_id) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.game_path.clone(),
            data.settings.program_path.clone(),
            data.current_profile_id.clone(),
        )
    };
    if game_path.is_empty() {
        return Ok(Vec::new());
    }
    crate::pak_patcher::list_generated_patches_internal(&game_path, &program_path, &current_profile_id)
}

#[tauri::command]
pub fn delete_generated_patch_cmd(patch_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let (program_path, current_profile_id) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (
            data.settings.program_path.clone(),
            data.current_profile_id.clone(),
        )
    };
    crate::pak_patcher::delete_generated_patch_internal(&patch_path, &program_path, &current_profile_id)
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



