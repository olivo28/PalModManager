pub mod types;
pub mod xml_parser;
pub mod dmp_parser;
pub mod heuristics;
pub mod log_correlator;
pub mod support_bundle;
pub mod watcher;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use tauri::State;
use types::{ActiveModSnapshot, CrashHistoryItem, SupportBundleResult, UnifiedCrashReport};
use crate::AppState;

// Helper to resolve Unreal crash directory
fn get_crash_directory(game_root: &Path) -> Option<PathBuf> {
    watcher::get_unreal_crash_dir(game_root)
        .or_else(|| {
            #[cfg(target_os = "windows")]
            {
                std::env::var("LOCALAPPDATA")
                    .ok()
                    .map(|l| PathBuf::from(l).join("Pal").join("Saved").join("Crashes"))
            }
            #[cfg(not(target_os = "windows"))]
            {
                None
            }
        })
}

// List all detected crash dumps and incidents on disk sorted by most recent
#[tauri::command]
pub async fn list_all_crashes(
    game_path: String,
) -> Result<Vec<CrashHistoryItem>, String> {
    let game_root = PathBuf::from(&game_path);
    let mut results: Vec<(CrashHistoryItem, std::time::SystemTime)> = Vec::new();

    if let Some(dir) = get_crash_directory(&game_root) {
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                        if name.starts_with("UECC-") || name.starts_with("crash") {
                            let meta = entry.metadata().ok();
                            let mod_time = meta.and_then(|m| m.modified().ok()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                            let dt: chrono::DateTime<chrono::Local> = mod_time.into();
                            let timestamp = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                            let has_xml = path.join("CrashContext.runtime-xml").exists();
                            let has_dmp = path.join("UEMinidump.dmp").exists() || path.join("UE4SS_Crash.dmp").exists();

                            let mut origin = "game_engine".to_string();
                            if has_xml {
                                if let Ok(xml_head) = std::fs::read_to_string(path.join("CrashContext.runtime-xml")) {
                                    let head_lower = xml_head.to_lowercase();
                                    if head_lower.contains("iplatformfilepak.cpp") || head_lower.contains("retry was not successful") {
                                        origin = "pak_mod".to_string();
                                    } else if head_lower.contains("ue4ss") {
                                        origin = "ue4ss".to_string();
                                    } else if head_lower.contains("outofmemory") || head_lower.contains("out of memory") {
                                        origin = "out_of_memory".to_string();
                                    }
                                }
                            }

                            results.push((
                                CrashHistoryItem {
                                    id: name.clone(),
                                    folder_path: path.to_string_lossy().to_string(),
                                    timestamp,
                                    has_xml,
                                    has_dmp,
                                    folder_name: name,
                                    origin,
                                },
                                mod_time,
                            ));
                        }
                    }
                }
            }
        }
    }

    // Check standalone UE4SS minidump in Pal/Binaries/Win64
    let ue4ss_dmp = game_root.join("Pal").join("Binaries").join("Win64").join("UE4SS_Crash.dmp");
    if ue4ss_dmp.exists() {
        if let Ok(meta) = ue4ss_dmp.metadata() {
            let mod_time = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let dt: chrono::DateTime<chrono::Local> = mod_time.into();
            let parent = ue4ss_dmp.parent().unwrap_or(&game_root);
            results.push((
                CrashHistoryItem {
                    id: "ue4ss_crash_standalone".to_string(),
                    folder_path: parent.to_string_lossy().to_string(),
                    timestamp: dt.format("%Y-%m-%d %H:%M:%S").to_string(),
                    has_xml: false,
                    has_dmp: true,
                    folder_name: "UE4SS_Crash.dmp".to_string(),
                    origin: "ue4ss".to_string(),
                },
                mod_time,
            ));
        }
    }

    // Sort descending by modified date
    results.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(results.into_iter().map(|(item, _)| item).collect())
}

// Scan and analyze the latest crash dump on disk
#[tauri::command]
pub async fn check_latest_crash(
    state: State<'_, AppState>,
    game_path: String,
) -> Result<Option<UnifiedCrashReport>, String> {
    let game_root = PathBuf::from(&game_path);
    let crash_dir = get_crash_directory(&game_root);

    let mut latest_folder: Option<(PathBuf, std::time::SystemTime)> = None;

    if let Some(ref dir) = crash_dir {
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if name.starts_with("UECC-") || name.starts_with("crash") {
                            if let Ok(meta) = entry.metadata() {
                                if let Ok(modified) = meta.modified() {
                                    if latest_folder.as_ref().map(|(_, t)| modified > *t).unwrap_or(true) {
                                        latest_folder = Some((path, modified));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some((folder, _)) = latest_folder {
        return analyze_crash_folder_internal(&state, &folder, &game_root).map(Some);
    }

    // Check standalone UE4SS minidump in Pal/Binaries/Win64/UE4SS_Crash.dmp
    let ue4ss_dmp = game_root.join("Pal").join("Binaries").join("Win64").join("UE4SS_Crash.dmp");
    if ue4ss_dmp.exists() {
        let parent = ue4ss_dmp.parent().unwrap_or(&game_root);
        return analyze_crash_folder_internal(&state, parent, &game_root).map(Some);
    }

    Ok(None)
}

// Analyze specific crash directory
#[tauri::command]
pub async fn analyze_crash_folder(
    state: State<'_, AppState>,
    folder_path: String,
    game_path: String,
) -> Result<UnifiedCrashReport, String> {
    let folder = PathBuf::from(&folder_path);
    let game_root = PathBuf::from(&game_path);
    analyze_crash_folder_internal(&state, &folder, &game_root)
}

// Export support bundle ZIP for Discord / GitHub support
#[tauri::command]
pub async fn export_crash_support_bundle(
    report: UnifiedCrashReport,
    output_destination: String,
    game_path: String,
) -> Result<SupportBundleResult, String> {
    let out_dest = PathBuf::from(&output_destination);
    let game_root = PathBuf::from(&game_path);
    Ok(support_bundle::generate_support_bundle(&report, &out_dest, &game_root))
}

// Toggle background crash watcher
#[tauri::command]
pub async fn toggle_crash_watcher(enabled: bool) -> Result<bool, String> {
    watcher::set_crash_watcher_enabled(enabled);
    crate::logger::log(&format!("Crash Doctor: Real-time watcher enabled set to: {}", enabled));
    Ok(enabled)
}

fn analyze_crash_folder_internal(
    state: &State<'_, AppState>,
    crash_folder: &Path,
    game_root: &Path,
) -> Result<UnifiedCrashReport, String> {
    // Read engine XML metadata if present
    let xml_path = crash_folder.join("CrashContext.runtime-xml");
    let xml_context = xml_parser::parse_crash_context_file(&xml_path);

    // 2. Leer Minidump si existe
    let dmp_candidates = [
        crash_folder.join("UEMinidump.dmp"),
        crash_folder.join("UE4SS_Crash.dmp"),
        game_root.join("Pal").join("Binaries").join("Win64").join("UE4SS_Crash.dmp"),
    ];

    let mut minidump_info = None;
    for dmp_path in &dmp_candidates {
        if dmp_path.exists() {
            if let Some(info) = dmp_parser::parse_minidump_file(dmp_path) {
                minidump_info = Some(info);
                break;
            }
        }
    }

    // 3. Obtener mods activos del perfil actual
    let active_mods = {
        let data = state.data.lock().unwrap();
        let mut list = Vec::new();
        for m in &data.mods {
            if m.enabled {
                list.push(ActiveModSnapshot {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    version: m.version.clone(),
                    mod_type: format!("{:?}", m.mod_type),
                    author: m.nexus_author.clone(),
                    nexus_id: m.nexus_mod_id,
                    is_enabled: m.enabled,
                });
            }
        }
        list
    };

    // 4. Extraer errores recientes de logs
    let recent_errors = log_correlator::extract_recent_errors(game_root, 30);

    // 5. Evaluar heurísticas y construir reporte unificado
    let report = heuristics::evaluate_crash_heuristics(
        xml_context.as_ref(),
        minidump_info.as_ref(),
        &active_mods,
        &recent_errors,
        crash_folder,
    );

    Ok(report)
}
