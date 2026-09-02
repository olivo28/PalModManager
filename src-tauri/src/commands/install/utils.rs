use std::path::PathBuf;
use crate::zip_handler;

pub fn check_mod_dependencies(game_path: &str, mod_type: &str, analysis: &zip_handler::ZipAnalysis) -> Result<(), String> {
    let dep_status = crate::dependency_checker::check_dependencies(game_path);
    
    // Check UE4SS dependency
    let ue4ss_required = match mod_type {
        "ue4ss" | "palschema" | "hybrid" => true,
        _ => false,
    };
    if ue4ss_required && !dep_status.ue4ss_installed {
        return Err("UE4SS is not installed. This mod requires UE4SS to operate. Please install UE4SS first.".to_string());
    }

    // Check PalSchema dependency
    let has_palschema_folder = analysis.files.iter().any(|f| f.to_lowercase().contains("palschema"));
    let palschema_required = match mod_type {
        "palschema" => true,
        "hybrid" if (analysis.has_palschema_json || has_palschema_folder) => true,
        _ => false,
    };
    if palschema_required && !dep_status.palschema_installed {
        return Err("PalSchema is not installed. This mod requires PalSchema to operate. Please install PalSchema first.".to_string());
    }

    Ok(())
}

pub fn sync_altermatic_helper(data: &crate::models::AppData) {
    if !data.settings.game_path.is_empty() {
        let game_path = PathBuf::from(&data.settings.game_path);
        let current_profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
        let enabled_ids: Vec<String> = if let Some(p) = current_profile {
            p.enabled_mod_ids.clone()
        } else {
            data.mods.iter().filter(|m| m.enabled).map(|m| m.id.clone()).collect()
        };
        let _ = crate::altermatic::sync_load_list(&game_path, &enabled_ids, &data.mods);
    }
}
