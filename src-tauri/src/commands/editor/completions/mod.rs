pub mod common;
pub mod lua;
pub mod json;

use std::collections::HashSet;
use std::path::Path;
use tauri::State;
use crate::state::AppState;
use crate::usmap::{get_or_load_sdk_index, get_or_load_schema, get_or_load_datatable_index, get_or_load_blueprint_index};
use crate::commands::editor::types::EditorCompletion;

#[tauri::command]
pub async fn get_editor_completions(
    file_path: String,
    query: String,
    line_prefix: String,
    mod_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<EditorCompletion>, String> {
    let (program_path, game_path, mod_info_opt) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let mod_info = mod_id.as_ref().and_then(|mid| data.mods.iter().find(|m| m.id == *mid).cloned());
        (data.settings.program_path.clone(), data.settings.game_path.clone(), mod_info)
    };

    let ext = Path::new(&file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut completions = Vec::new();

    // 1. Workspace Content-Aware Mod Symbols (Strictly for Lua cross-script navigation)
    // For JSON/JSONC data files, cross-file key scraping leaks unrelated symbols and is excluded
    if ext == "lua" {
        if let Some(ref mod_info) = mod_info_opt {
            if let Ok(file_list) = crate::commands::config_commands::list_mod_files(mod_info.id.clone(), state.clone()) {
                if !file_list.is_empty() {
                    let symbols = crate::commands::editor::workspace_index::get_or_build_mod_symbols(mod_info, &file_list);
                    let ws_completions = crate::commands::editor::workspace_index::find_workspace_completions(&symbols, &query, 60);
                    completions.extend(ws_completions);
                }
            }
        }
    }

    let schema = get_or_load_schema(&program_path);
    let sdk_index = get_or_load_sdk_index(&program_path, &game_path);
    let dt_index = get_or_load_datatable_index(&program_path);
    let bp_index = get_or_load_blueprint_index(&program_path);
    let palschema_tables = crate::usmap::get_or_load_palschema_table_names(&program_path, &game_path);

    let mut seen = HashSet::new();

    let clean_path = file_path.replace('\\', "/");
    let clean_path_lower = clean_path.to_lowercase();

    let is_palschema_file = clean_path.starts_with("[PalSchema]")
        || clean_path_lower.contains("/palschema/")
        || clean_path_lower.contains("palschema/mods/")
        || clean_path_lower.ends_with(".palschema.json")
        || clean_path_lower.ends_with(".palschema.jsonc")
        || mod_info_opt.as_ref().map(|m| {
            let is_ue4ss_comp = clean_path.starts_with("[UE4SS]") || clean_path_lower.starts_with("scripts/");
            let is_ps_mod = m.game_path.replace('\\', "/").to_lowercase().contains("palschema");
            is_ps_mod && !is_ue4ss_comp
        }).unwrap_or(false);

    if ext == "lua" {
        let lua_signatures = crate::usmap::get_or_load_lua_signatures(&game_path, &program_path);
        lua::populate_lua_completions(
            &query,
            &line_prefix,
            schema.as_ref(),
            sdk_index.as_ref(),
            dt_index.as_ref(),
            lua_signatures.as_ref(),
            &mut completions,
            &mut seen,
        );
    } else if ext == "json" || ext == "jsonc" {
        json::populate_json_completions(
            &file_path,
            &query,
            &line_prefix,
            schema.as_ref(),
            sdk_index.as_ref(),
            dt_index.as_ref(),
            bp_index.as_ref(),
            Some(&palschema_tables),
            is_palschema_file,
            &mut completions,
            &mut seen,
        );
    }

    Ok(completions)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflectionCatalogsStatus {
    pub total_datatables: usize,
    pub total_datatable_rows: usize,
    pub datatables_active_file: String,
    pub datatables_game_ver: String,
    pub datatables_build_id: String,
    pub datatables_sha256: String,
    pub datatables_size: u64,
    pub datatables_is_build_matched: bool,
    pub total_blueprints: usize,
    pub blueprints_build_id: String,
    pub blueprints_game_ver: String,
    pub blueprints_filename: String,
    pub blueprints_sha256: String,
    pub blueprints_size: u64,
    pub blueprints_is_build_matched: bool,
}

#[tauri::command]
pub async fn get_reflection_catalogs_status(
    state: State<'_, AppState>,
) -> Result<ReflectionCatalogsStatus, String> {
    let (program_path, game_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.settings.game_path.clone())
    };

    let installed_build = crate::usmap::detect_installed_game_build(std::path::Path::new(&game_path));
    let detected_build = installed_build.build_id;

    let dt = crate::usmap::get_or_load_datatable_index(&program_path);
    let bp = crate::usmap::get_or_load_blueprint_index(&program_path);

    // 1. Blueprints manifest & disk inspection (100% dynamic)
    let bp_dir = crate::usmap::get_blueprints_dir(&program_path);
    let mut bp_ver = String::new();
    let mut bp_build = String::new();
    let mut bp_file = String::new();
    let mut bp_hash = String::new();
    let mut bp_size: u64 = 0;

    let bp_manifest_path = crate::usmap::sync::find_bundled_resource("resources/blueprints/manifest.json")
        .or_else(|| {
            let p = bp_dir.join("manifest.json");
            if p.exists() { Some(p) } else { None }
        });

    if let Some(mp) = bp_manifest_path {
        if let Ok(m_str) = std::fs::read_to_string(&mp) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&m_str) {
                if let Some(list) = v.get("blueprints").and_then(|a| a.as_array()) {
                    let matched_entry = list.iter().find(|e| {
                        if let Some(b) = detected_build.as_deref() {
                            e.get("steam_build_id").and_then(|s| s.as_str()) == Some(b)
                        } else {
                            false
                        }
                    })
                    .or_else(|| list.iter().find(|e| e.get("is_latest").and_then(|b| b.as_bool()).unwrap_or(false)))
                    .or_else(|| list.first());

                    if let Some(entry) = matched_entry {
                        if let Some(fname) = entry.get("blueprints_filename").and_then(|s| s.as_str()) {
                            bp_file = fname.to_string();
                        }
                        if let Some(ver) = entry.get("game_version").and_then(|s| s.as_str()) {
                            bp_ver = ver.to_string();
                        }
                        if let Some(b) = entry.get("steam_build_id").and_then(|s| s.as_str()) {
                            bp_build = b.to_string();
                        }
                        if let Some(h) = entry.get("sha256").and_then(|s| s.as_str()) {
                            bp_hash = h.to_string();
                        }
                        if let Some(sz) = entry.get("file_size_bytes").and_then(|s| s.as_u64()) {
                            bp_size = sz;
                        }
                    }
                }
            }
        }
    }

    // Disk fallback if manifest gave no file
    if bp_file.is_empty() {
        let scan_dirs = [
            Some(bp_dir.clone()),
            crate::usmap::sync::find_bundled_resource("resources/blueprints"),
        ];
        for d_opt in scan_dirs.into_iter().flatten() {
            if let Ok(entries) = std::fs::read_dir(&d_opt) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
                            if fname.starts_with("Palworld_Blueprints_") && fname.ends_with(".json") {
                                bp_file = fname.to_string();
                                bp_build = fname
                                    .trim_start_matches("Palworld_Blueprints_")
                                    .trim_end_matches(".json")
                                    .to_string();
                                bp_size = p.metadata().map(|m| m.len()).unwrap_or(0);
                                break;
                            }
                        }
                    }
                }
            }
            if !bp_file.is_empty() {
                break;
            }
        }
    }

    // 2. DataTables manifest & disk inspection (100% dynamic)
    let dt_dir = crate::usmap::get_datatables_dir(&program_path);
    let mut dt_ver = String::new();
    let mut dt_build = String::new();
    let mut dt_file = String::new();
    let mut dt_hash = String::new();
    let mut dt_size: u64 = 0;

    let dt_manifest_path = crate::usmap::sync::find_bundled_resource("resources/datatables/manifest.json")
        .or_else(|| {
            let p = dt_dir.join("manifest.json");
            if p.exists() { Some(p) } else { None }
        });

    if let Some(mp) = dt_manifest_path {
        if let Ok(m_str) = std::fs::read_to_string(&mp) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&m_str) {
                if let Some(list) = v.get("datatables").and_then(|a| a.as_array()) {
                    let matched_entry = list.iter().find(|e| {
                        if let Some(b) = detected_build.as_deref() {
                            e.get("steam_build_id").and_then(|s| s.as_str()) == Some(b)
                        } else {
                            false
                        }
                    })
                    .or_else(|| list.iter().find(|e| e.get("is_latest").and_then(|b| b.as_bool()).unwrap_or(false)))
                    .or_else(|| list.first());

                    if let Some(entry) = matched_entry {
                        if let Some(fname) = entry.get("datatables_filename").and_then(|s| s.as_str()) {
                            dt_file = fname.to_string();
                        }
                        if let Some(ver) = entry.get("game_version").and_then(|s| s.as_str()) {
                            dt_ver = ver.to_string();
                        }
                        if let Some(b) = entry.get("steam_build_id").and_then(|s| s.as_str()) {
                            dt_build = b.to_string();
                        }
                        if let Some(h) = entry.get("sha256").and_then(|s| s.as_str()) {
                            dt_hash = h.to_string();
                        }
                        if let Some(sz) = entry.get("file_size_bytes").and_then(|s| s.as_u64()) {
                            dt_size = sz;
                        }
                    }
                }
            }
        }
    }

    // Disk fallback if manifest gave no file
    if dt_file.is_empty() {
        let scan_dirs = [
            Some(dt_dir.clone()),
            crate::usmap::sync::find_bundled_resource("resources/datatables"),
        ];
        for d_opt in scan_dirs.into_iter().flatten() {
            if let Ok(entries) = std::fs::read_dir(&d_opt) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
                            if fname.starts_with("Palworld_DataTables_") && fname.ends_with(".json") {
                                dt_file = fname.to_string();
                                dt_build = fname
                                    .trim_start_matches("Palworld_DataTables_")
                                    .trim_end_matches(".json")
                                    .to_string();
                                dt_size = p.metadata().map(|m| m.len()).unwrap_or(0);
                                break;
                            }
                        }
                    }
                }
            }
            if !dt_file.is_empty() {
                break;
            }
        }
    }

    let blueprints_is_build_matched = match (&detected_build, &bp_build) {
        (Some(det), b) if !det.is_empty() && !b.is_empty() => det == b,
        _ => true,
    };

    let datatables_is_build_matched = match (&detected_build, &dt_build) {
        (Some(det), b) if !det.is_empty() && !b.is_empty() => det == b,
        _ => true,
    };

    Ok(ReflectionCatalogsStatus {
        total_datatables: dt.as_ref().map(|d| d.total_tables).unwrap_or(0),
        total_datatable_rows: dt.as_ref().map(|d| d.total_rows).unwrap_or(0),
        datatables_active_file: dt_file,
        datatables_game_ver: dt_ver,
        datatables_build_id: dt_build,
        datatables_sha256: dt_hash,
        datatables_size: dt_size,
        datatables_is_build_matched,
        total_blueprints: bp.as_ref().map(|b| b.total_blueprints).unwrap_or(0),
        blueprints_build_id: bp_build,
        blueprints_game_ver: bp_ver,
        blueprints_filename: bp_file,
        blueprints_sha256: bp_hash,
        blueprints_size: bp_size,
        blueprints_is_build_matched,
    })
}
