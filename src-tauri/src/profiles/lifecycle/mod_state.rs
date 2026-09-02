use std::fs;
use crate::models::AppData;
use super::super::utils::get_profile_dir;
use super::super::actions::{enable_mod_internal, disable_mod_internal};
use super::super::core::{cleanup_profile_enabled_ids, mod_matches_profile_entry};

pub fn set_profile_mod_state(
    data: &mut AppData,
    profile_id: &str,
    mod_id: &str,
    enabled: bool,
) -> Result<(), String> {
    let program_path = data.settings.program_path.clone();
    let mod_name = data.mods.iter().find(|m| m.id == mod_id).map(|m| m.name.clone());

    {
        let profile = data
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "Profile not found".to_string())?;

        if enabled {
            if let Some(ref name) = mod_name {
                let already_present = profile.enabled_mod_ids.iter().any(|id| id.to_lowercase() == name.to_lowercase());
                if !already_present {
                    profile.enabled_mod_ids.push(name.clone());
                }
            }
        } else {
            if let Some(ref m_info) = data.mods.iter().find(|m| m.id == mod_id) {
                profile.enabled_mod_ids.retain(|id| !mod_matches_profile_entry(m_info, id));
            } else {
                profile.enabled_mod_ids.retain(|id| id.to_lowercase() != mod_id.to_lowercase());
                if let Some(ref name) = mod_name {
                    profile.enabled_mod_ids.retain(|id| id.to_lowercase() != name.to_lowercase());
                }
            }
        }
    }

    cleanup_profile_enabled_ids(data);

    let p_dir = get_profile_dir(&program_path, profile_id);
    if let Some(profile) = data.profiles.iter().find(|p| p.id == profile_id) {
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }

    if profile_id == data.current_profile_id {
        let mod_info = data.mods.iter().find(|m| m.id == mod_id).cloned();
        let is_workshop = mod_info.as_ref().map(|m| {
            m.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod"))
        }).unwrap_or(false);

        if is_workshop {
            let game_path = data.settings.game_path.clone();
            let force_load_order_ue4ss = data.settings.force_load_order.unwrap_or(false) && data.settings.force_load_order_ue4ss.unwrap_or(false);
            let wmods = crate::workshop::scan_workshop_mods(&game_path);
            if let Some(target) = wmods.iter().find(|m| m.package_name == mod_id || m.package_name.to_lowercase() == mod_id.to_lowercase()) {
                crate::logger::log(&format!("set_profile_mod_state: Applying workshop change for mod = {} -> enabled = {}", mod_id, enabled));
                if enabled {
                    let _ = crate::workshop::activate_workshop_mod(&game_path, target, force_load_order_ue4ss);
                } else {
                    let _ = crate::workshop::deactivate_workshop_mod(&game_path, target, force_load_order_ue4ss);
                }
            }
        } else {
            crate::logger::log(&format!("set_profile_mod_state: Applying switch physically for mod = {} -> enabled = {}", mod_id, enabled));
            if enabled {
                enable_mod_internal(data, &program_path, mod_id)?;
            } else {
                disable_mod_internal(data, &program_path, mod_id)?;
            }
        }
    }

    Ok(())
}
