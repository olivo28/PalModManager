// GamePass .pak to Zen format conversion commands
use std::path::{Path, PathBuf};
use tauri::State;
use crate::state::AppState;

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
