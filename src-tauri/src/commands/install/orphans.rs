use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use uuid::Uuid;
use crate::models::{UpgradeOrphanFile, UpgradeOrphansResult};
use crate::state::AppState;
use crate::zip_handler;

fn normalize_for_compare(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    #[cfg(target_os = "windows")]
    {
        s.to_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        s
    }
}

/// Inspects local files on disk for a mod being upgraded vs what the incoming zip ships.
/// Classifies files not in the incoming zip as configs (default keep) or stale files (default purge).
#[tauri::command]
pub async fn inspect_upgrade_orphans(
    zip_path: String,
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<UpgradeOrphansResult, String> {
    let (game_path, mod_info) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let m = data
            .mods
            .iter()
            .find(|m| m.id == mod_id)
            .cloned()
            .ok_or_else(|| "Mod not found".to_string())?;
        (data.settings.game_path.clone(), m)
    };

    if game_path.is_empty() {
        return Err("Game path is not configured".to_string());
    }

    let game = Path::new(&game_path);

    // Collect all local directories where mod files live
    let mut installed_roots = Vec::new();
    if !mod_info.game_path.is_empty() {
        let p = crate::config_merge::resolve_path_in_game(game, &mod_info.game_path);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            installed_roots.push(r);
        }
    }
    if !mod_info.disabled_path.is_empty() {
        let p = crate::config_merge::resolve_path_in_game(game, &mod_info.disabled_path);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            if !installed_roots.contains(&r) {
                installed_roots.push(r);
            }
        }
    }
    for extra in &mod_info.extra_files {
        let p = crate::config_merge::resolve_path_in_game(game, extra);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            if !installed_roots.contains(&r) {
                installed_roots.push(r);
            }
        }
    }

    if installed_roots.is_empty() {
        return Ok(UpgradeOrphansResult {
            mod_id: mod_info.id,
            mod_name: mod_info.name,
            orphan_files: Vec::new(),
        });
    }

    // Extract incoming zip to temporary folder to build manifest and calculate installed destinations
    let temp_dir = std::env::temp_dir().join(format!("palmodmanager_orphans_{}", Uuid::new_v4()));
    let extracted = zip_handler::extract_zip_to_temp(&zip_path, &temp_dir)?;

    let zip_filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f: &&String| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            let full_path = extracted.join(target_file);
            if full_path.exists() {
                if let Ok(content) = fs::read_to_string(full_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        modinfo_data = Some(val);
                    }
                }
            }
        }
    }

    let manifest = zip_handler::build_manifest_from_files(
        &analysis.files,
        &zip_filename,
        game,
        None,
        Some(mod_info.name.clone()),
        modinfo_data,
    );

    let manifest = match manifest {
        Ok(m) => m,
        Err(err) => {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(err);
        }
    };

    let mut manifest_installed_files = std::collections::HashSet::new();
    for route in &manifest.routes {
        let src = crate::installer::resolve_source_path(&extracted, &route.zip_path)
            .unwrap_or_else(|| extracted.join(&route.zip_path));
        let dst = PathBuf::from(&route.dest_path);
        if src.is_dir() {
            for entry in walkdir::WalkDir::new(&src).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Ok(rel) = entry.path().strip_prefix(&src) {
                        let full_dst = dst.join(rel);
                        manifest_installed_files.insert(normalize_for_compare(&full_dst));
                    }
                }
            }
        } else {
            manifest_installed_files.insert(normalize_for_compare(&dst));
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);

    // Snapshot existing user configs
    let mut all_snapshots = crate::config_merge::ConfigSnapshot { entries: Vec::new() };
    for root in &installed_roots {
        let snap = crate::config_merge::snapshot_configs(root, mod_info.config_path.as_deref());
        for entry in snap.entries {
            all_snapshots.entries.push(entry);
        }
    }

    let mut orphan_files = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for root in &installed_roots {
        if !root.is_dir() {
            continue;
        }
        let dir_name = root.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        if matches!(dir_name.as_str(), "paks" | "logicmods" | "mods" | "win64" | "binaries" | "pal" | "content") {
            continue;
        }

        for entry in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let file_path = entry.path();
            let fname = file_path.file_name().unwrap_or_default().to_string_lossy();
            let fname_lower = fname.to_lowercase();

            // Skip protected internal metadata
            if fname_lower.ends_with(".pmm.json")
                || fname_lower == "enabled.txt"
                || fname.starts_with('.')
                || fname_lower.ends_with(".bak")
                || file_path.components().any(|c| c.as_os_str().to_string_lossy().to_lowercase() == "shared")
            {
                continue;
            }

            let normalized = normalize_for_compare(file_path);
            if seen_paths.contains(&normalized) {
                continue;
            }
            seen_paths.insert(normalized.clone());

            // If file is part of incoming update, it's not orphaned
            if manifest_installed_files.contains(&normalized) {
                continue;
            }

            // Check if captured by config snapshot
            let is_snapshotted = all_snapshots.entries.iter().any(|(rel, _)| {
                let snap_norm = normalize_for_compare(rel);
                normalized.ends_with(&snap_norm)
            });

            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let path_str_lower = file_path.to_string_lossy().to_lowercase();

            let is_runtime_data = path_str_lower.contains("/saved/")
                || path_str_lower.contains("\\saved\\")
                || path_str_lower.contains("/storage/")
                || path_str_lower.contains("\\storage\\")
                || path_str_lower.contains("/data/")
                || path_str_lower.contains("\\data\\")
                || path_str_lower.contains("/cache/")
                || path_str_lower.contains("\\cache\\");

            let is_heuristic_config = is_snapshotted
                || is_runtime_data
                || ext == "ini"
                || ext == "cfg"
                || ext == "toml"
                || ext == "yaml"
                || ext == "yml"
                || ((ext == "json" || ext == "jsonc" || ext == "txt" || ext == "lua")
                    && (fname_lower.contains("config")
                        || fname_lower.contains("setting")
                        || fname_lower.contains("option")
                        || fname_lower.contains("pref")
                        || path_str_lower.contains("config")
                        || path_str_lower.contains("setting")));

            let rel_str = file_path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| fname.to_string());

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

            orphan_files.push(UpgradeOrphanFile {
                relative_path: rel_str,
                absolute_path: file_path.to_string_lossy().to_string(),
                file_name: fname.to_string(),
                is_config: is_heuristic_config,
                default_keep: is_heuristic_config,
                size_bytes: size,
            });
        }
    }

    Ok(UpgradeOrphansResult {
        mod_id: mod_info.id,
        mod_name: mod_info.name,
        orphan_files,
    })
}
