// Conflict scanning for PalSchema and UE4SS mods
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use tauri::State;
use crate::state::AppState;
use crate::models::ModType;
use super::types::*;
use super::conflict_parsers::{scan_palschema_mod, scan_ue4ss_mod};
pub use super::conflict_parsers::extract_literal_hooks;
use super::hook_validator::validate_hook_with_usmap;
use super::palschema::validate_palschema_table_with_usmap;

#[tauri::command]
pub async fn scan_conflicts(state: State<'_, AppState>) -> Result<ScanResult, String> {
    let (profile_mods, game_path, program_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
        (profile_mods, data.settings.game_path.clone(), data.settings.program_path.clone())
    };

    tauri::async_runtime::spawn_blocking(move || {
        scan_conflicts_internal(profile_mods, &game_path, &program_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn scan_conflicts_internal(
    profile_mods: Vec<crate::models::ModInfo>,
    game_path: &str,
    program_path: &str,
) -> Result<ScanResult, String> {
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

    let (pak_conflicts, pak_scanned) = if !game_path.is_empty() {
        crate::pak_scanner::scan_pak_conflicts(Path::new(game_path), &profile_mods)
    } else {
        (Vec::new(), 0)
    };

    let (is_gamepass, gamepass_notices) = if !game_path.is_empty() {
        crate::pak_scanner::check_gamepass_pak_compatibility(Path::new(game_path), &profile_mods)
    } else {
        (false, Vec::new())
    };

    let schema_notices = crate::pak_scanner::check_mod_schema_compatibility(&profile_mods);
    let patch_risk_notices = crate::pak_scanner::check_patch_risk_compatibility(&profile_mods);

    // USMAP Unreal Engine Schema Hook Diagnostics & C++ SDK Headers
    let usmap_path = crate::usmap::sync::get_active_usmap_path(program_path);
    let sdk_index = crate::usmap::get_or_load_sdk_index(program_path, game_path);

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

                let (status, reason, suggestion) = validate_palschema_table_with_usmap(&entry.table_name, &schema, sdk_index.as_deref(), Path::new(game_path));
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
        patch_risk_notices,
        usmap_diagnostics,
        is_gamepass,
    })
}
