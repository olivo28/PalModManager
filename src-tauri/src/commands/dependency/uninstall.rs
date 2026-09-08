use std::fs;
use std::path::Path;
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn uninstall_ue4ss(state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() { return Err("Game path not set".to_string()); }

    // Guard: Workshop installations cannot be uninstalled from PMM
    let game_profile = crate::dependency_checker::build_game_profile(Path::new(&game_path));
    if game_profile.ue4ss_install_mode == crate::dependency_checker::UE4SSInstallMode::Workshop {
        return Err("UE4SS is managed by Steam Workshop. To uninstall, unsubscribe from the mod in Steam.".to_string());
    }

    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let dwmapi = win64.join("dwmapi.dll");
    let ue4ss_dir = win64.join("ue4ss");
    
    if dwmapi.exists() { let _ = fs::remove_file(dwmapi); }
    
    // Purge UE4SS runtime files without destroying user mods in ue4ss/Mods/
    if ue4ss_dir.exists() {
        let runtime_files = [
            "UE4SS.dll", "UE4SS-settings.ini", "MemberVariableLayout.ini",
            "LICENSE", "ue4ss.version", "ue4ss.pmm.json", "ue4ss.manifest.json"
        ];
        for f in runtime_files {
            let p = ue4ss_dir.join(f);
            if p.exists() { let _ = fs::remove_file(p); }
        }

        // Clean only default canonical system mods
        let mods_dir = ue4ss_dir.join("Mods");
        if mods_dir.exists() {
            for sys_mod in crate::dependency_manifest::CANONICAL_UE4SS_SYSTEM_MODS {
                let sys_p = mods_dir.join(sys_mod);
                if sys_p.exists() { let _ = fs::remove_dir_all(sys_p); }
            }
            let has_user_mods = fs::read_dir(&mods_dir)
                .map(|rd| rd.filter_map(|e| e.ok()).any(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    let lower = n.to_lowercase();
                    lower != "mods.txt" && lower != "mods.json" && lower != "palschema"
                }))
                .unwrap_or(false);

            if !has_user_mods {
                let _ = fs::remove_dir_all(&ue4ss_dir);
            }
        }
    }
    
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        // Remove only native system mods from data.mods
        let sys_mods = crate::dependency_manifest::CANONICAL_UE4SS_SYSTEM_MODS;
        let sys_mod_ids: Vec<String> = data.mods.iter()
            .filter(|m| sys_mods.iter().any(|&s| s.eq_ignore_ascii_case(&m.name)))
            .map(|m| m.id.clone())
            .collect();
        data.mods.retain(|m| !sys_mods.iter().any(|&s| s.eq_ignore_ascii_case(&m.name)));

        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.ue4ss_enabled = false;
            // Clean only system mod IDs from profile lists, preserving all user mods
            profile.installed_mod_ids.retain(|id| !sys_mod_ids.contains(id));
            profile.enabled_mod_ids.retain(|id| !sys_mod_ids.contains(id));

            let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    crate::logger::log("uninstall_ue4ss: UE4SS runtime desinstalado con éxito (mods de usuario preservados).");
    Ok("UE4SS uninstalled successfully".to_string())
}

#[tauri::command]
pub fn uninstall_palschema(state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() { return Err("Game path not set".to_string()); }

    let game_path_val = Path::new(&game_path);
    let game_profile = crate::dependency_checker::build_game_profile(game_path_val);

    let purge_ps_runtime = |dir: &Path| {
        if !dir.exists() { return; }
        let runtime_files = [
            "enabled.txt", "LICENSE", "palschema.version", "palschema.pmm.json", "palschema.manifest.json"
        ];
        for f in runtime_files {
            let p = dir.join(f);
            if p.exists() { let _ = fs::remove_file(p); }
        }
        let _ = fs::remove_dir_all(dir.join("dlls"));
        let _ = fs::remove_dir_all(dir.join("scripts"));

        // Preserve dir.join("mods") if it has user mods
        let has_user_mods = dir.join("mods").exists() && fs::read_dir(dir.join("mods"))
            .map(|rd| rd.filter_map(|e| e.ok()).count() > 0)
            .unwrap_or(false);
        if !has_user_mods {
            let _ = fs::remove_dir_all(dir);
        }
    };

    // Primary target: resolved profile palschema directory
    let palschema_dir = game_profile.ue4ss_mods_dir.join("PalSchema");
    purge_ps_runtime(&palschema_dir);

    // Secondary fallback: clean potential leftover native / workshop mods directory
    let native_ps_dir = game_path_val.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("PalSchema");
    purge_ps_runtime(&native_ps_dir);

    // Also clean standard binaries fallback if profile had resolved to workshop earlier
    let std_bin_ps = crate::dependency_checker::get_binaries_dir(game_path_val).join("ue4ss").join("Mods").join("PalSchema");
    purge_ps_runtime(&std_bin_ps);
    
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.palschema_enabled = false;
            
            let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    crate::logger::log("uninstall_palschema: PalSchema runtime desinstalado con éxito (mods de usuario preservados).");
    Ok("PalSchema uninstalled successfully".to_string())
}
