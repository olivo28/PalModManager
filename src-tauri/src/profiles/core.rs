use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{AppData, Profile, DependencyMode};
use super::utils::{get_profile_dir, ensure_profile_structure};

fn dedup_vec(vec: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    vec.retain(|item| {
        let key = item.trim().to_lowercase();
        if key.is_empty() {
            return false;
        }
        seen.insert(key)
    });
}

pub fn migrate_profile_uuids_to_stable_ids(data: &mut AppData) {
    let mods = data.mods.clone();
    for profile in &mut data.profiles {
        for id_entry in &mut profile.installed_mod_ids {
            if let Some(matching_mod) = mods.iter().find(|m| {
                m.id.to_lowercase() == id_entry.to_lowercase() ||
                m.name.to_lowercase() == id_entry.to_lowercase()
            }) {
                if id_entry != &matching_mod.id {
                    crate::logger::log(&format!("Migrating profile installed ID '{}' -> stable ID '{}'", id_entry, matching_mod.id));
                    *id_entry = matching_mod.id.clone();
                }
            }
        }
        
        for id_entry in &mut profile.enabled_mod_ids {
            if let Some(matching_mod) = mods.iter().find(|m| {
                m.id.to_lowercase() == id_entry.to_lowercase() ||
                m.name.to_lowercase() == id_entry.to_lowercase()
            }) {
                if id_entry != &matching_mod.id {
                    crate::logger::log(&format!("Migrating profile enabled ID '{}' -> stable ID '{}'", id_entry, matching_mod.id));
                    *id_entry = matching_mod.id.clone();
                }
            }
        }

        dedup_vec(&mut profile.installed_mod_ids);
        dedup_vec(&mut profile.enabled_mod_ids);
    }
}

pub fn mod_matches_profile_entry(mod_info: &crate::models::ModInfo, entry: &str) -> bool {
    let entry_lower = entry.to_lowercase();
    let mod_id_lower = mod_info.id.to_lowercase();
    let mod_name_lower = mod_info.name.to_lowercase();
    if entry_lower == mod_id_lower || entry_lower == mod_name_lower {
        return true;
    }
    if let Some(stripped) = mod_name_lower.strip_suffix(" (workshop)") {
        if stripped == entry_lower {
            return true;
        }
    }
    if let Some(stripped) = entry_lower.strip_suffix(" (workshop)") {
        if stripped == mod_name_lower || stripped == mod_id_lower {
            return true;
        }
    }
    if !mod_info.game_path.is_empty() {
        if let Some(filename) = Path::new(&mod_info.game_path).file_name() {
            let folder_name = filename.to_string_lossy().to_lowercase();
            if folder_name == entry_lower {
                return true;
            }
            if let Some(stripped) = folder_name.strip_suffix(".disabled") {
                if stripped == entry_lower {
                    return true;
                }
            }
        }
    }
    if !mod_info.disabled_path.is_empty() {
        if let Some(filename) = Path::new(&mod_info.disabled_path).file_name() {
            let folder_name = filename.to_string_lossy().to_lowercase();
            if folder_name == entry_lower {
                return true;
            }
            if let Some(stripped) = folder_name.strip_suffix(".disabled") {
                if stripped == entry_lower {
                    return true;
                }
            }
        }
    }
    false
}

pub fn sync_current_profile_states(data: &mut AppData) {
    cleanup_profile_mod_lists(data);
    let current_id = data.current_profile_id.clone();
    if let Some(profile) = data.profiles.iter().find(|p| p.id == current_id).cloned() {
        for mod_info in &mut data.mods {
            if mod_info.nexus_author.as_deref() == Some("UE4SS Native Mod") {
                continue;
            }
            let is_enabled = profile.enabled_mod_ids.iter().any(|entry| {
                mod_matches_profile_entry(mod_info, entry)
            });
            mod_info.enabled = is_enabled;
        }
    }
}

pub fn cleanup_profile_mod_lists(data: &mut AppData) {
    let dep = if !data.settings.game_path.is_empty() {
        Some(crate::dependency_checker::check_dependencies(&data.settings.game_path))
    } else {
        None
    };

    let program_path = data.settings.program_path.clone();

    for profile in &mut data.profiles {
        dedup_vec(&mut profile.installed_mod_ids);
        dedup_vec(&mut profile.enabled_mod_ids);

        if profile.installed_mod_ids.is_empty() && !profile.enabled_mod_ids.is_empty() {
            profile.installed_mod_ids = profile.enabled_mod_ids.clone();
        }

        if let Some(ref d) = dep {
            if profile.id == "default" && !profile.ue4ss_enabled {
                if d.ue4ss_installed {
                    profile.ue4ss_enabled = true;
                    if d.palschema_installed {
                        profile.palschema_enabled = true;
                    }
                }
            }
            if profile.dependency_mode == DependencyMode::None && profile.ue4ss_enabled {
                profile.dependency_mode = if d.ue4ss_install_mode == "Workshop" {
                    DependencyMode::Workshop
                } else {
                    DependencyMode::Standard
                };
            }

            let is_current = profile.id == data.current_profile_id;

            // Only overwrite UE4SS/PalSchema versions for the ACTIVE profile.
            // Non-active profiles already have their own stored versions — don't clobber them
            // with whatever happens to be on disk right now (which reflects the active profile's install).
            if is_current {
                if profile.ue4ss_enabled && d.ue4ss_installed {
                    profile.ue4ss_version = d.ue4ss_version.clone();
                } else {
                    profile.ue4ss_version = None;
                }
                if profile.palschema_enabled && d.palschema_installed {
                    profile.palschema_version = d.palschema_version.clone();
                } else {
                    profile.palschema_version = None;
                }
            } else {
                // For non-active profiles, just clear version if the dep is marked disabled.
                if !profile.ue4ss_enabled {
                    profile.ue4ss_version = None;
                }
                if !profile.palschema_enabled {
                    profile.palschema_version = None;
                }
            }

            let alt_mod = data.mods.iter().find(|m| {
                (m.nexus_mod_id == Some(1626) || m.name.to_lowercase().contains("altermatic")) &&
                (
                    if is_current {
                        m.enabled
                    } else {
                        profile.enabled_mod_ids.iter().any(|id| id == &m.id || id.eq_ignore_ascii_case(&m.name))
                    }
                )
            });
            profile.altermatic_version = alt_mod.map(|m| m.version.clone());

            let uni_mod = data.mods.iter().find(|m| {
                (m.nexus_mod_id == Some(1894) || m.name.to_lowercase().contains("unipalui")) &&
                (
                    if is_current {
                        m.enabled
                    } else {
                        profile.enabled_mod_ids.iter().any(|id| id == &m.id || id.eq_ignore_ascii_case(&m.name))
                    }
                )
            });
            profile.unipalui_version = uni_mod.map(|m| m.version.clone());
        }

        // Sync compatibility patches for this profile (compact filenames only)
        let patches = crate::pak_patcher::load_profile_patches_registry(&program_path, &profile.id);
        profile.compatibility_patches = if patches.is_empty() {
            None
        } else {
            Some(patches.into_iter().map(|p| p.pak_filename).collect())
        };

        // Persist clean profile.json to disk
        if !program_path.is_empty() {
            let p_dir = get_profile_dir(&program_path, &profile.id);
            if p_dir.exists() {
                if let Ok(json) = serde_json::to_string_pretty(profile) {
                    let _ = fs::write(p_dir.join("profile.json"), json);
                }
            }
        }
    }
}

pub fn cleanup_profile_enabled_ids(data: &mut AppData) {
    cleanup_profile_mod_lists(data);
}


pub fn ensure_default_profile(data: &mut AppData) {
    let program_path = data.settings.program_path.clone();
    let profiles_base = PathBuf::from(&program_path).join("profiles");
    let _ = fs::create_dir_all(&profiles_base);

    if !data.profiles.iter().any(|p| p.id == "default") {
        let dep = if !data.settings.game_path.is_empty() {
            Some(crate::dependency_checker::check_dependencies(&data.settings.game_path))
        } else {
            None
        };

        let now = chrono::Utc::now().to_rfc3339();
        let dependency_mode = if let Some(ref d) = dep {
            if d.ue4ss_installed {
                if d.ue4ss_install_mode == "Workshop" {
                    DependencyMode::Workshop
                } else {
                    DependencyMode::Standard
                }
            } else {
                DependencyMode::None
            }
        } else {
            DependencyMode::None
        };

        let ue4ss_on = dep.as_ref().map(|d| d.ue4ss_installed).unwrap_or(false);
        let ps_on = dep.as_ref().map(|d| d.palschema_installed).unwrap_or(false);

        data.profiles.push(Profile {
            id: "default".to_string(),
            name: "Default".to_string(),
            created_at: now,
            installed_mod_ids: Vec::new(),
            enabled_mod_ids: Vec::new(),
            ue4ss_enabled: ue4ss_on,
            palschema_enabled: ps_on,
            dependency_mode,
            mod_folders: Vec::new(),
            load_order_metadata: None,
            force_load_order_ue4ss: None,
            force_load_order_palschema: None,
            hide_native_mods: None,
            ue4ss_version: None,
            palschema_version: None,
            altermatic_version: None,
            unipalui_version: None,
            compatibility_patches: None,
            ue4ss_control_mode: Some("enabled_txt".to_string()),
        });
    }

    migrate_profile_uuids_to_stable_ids(data);

    if data.current_profile_id.is_empty() {
        data.current_profile_id = "default".to_string();
    }

    // Migration: clear UE4SS/PalSchema versions from non-active profiles.
    // The old bug caused every profile switch to overwrite ALL profiles with the
    // active profile's disk-scanned versions. This one-time pass resets non-active
    // profiles so their versions get correctly populated when they are activated.
    let current_id = data.current_profile_id.clone();
    for profile in data.profiles.iter_mut() {
        if profile.id != current_id {
            // Reset to None so the correct value loads from disk snapshot when switching
            profile.ue4ss_version = None;
            profile.palschema_version = None;
        }
    }

    for p in &data.profiles {
        let p_dir = ensure_profile_structure(&program_path, &p.id);
        if let Ok(json) = serde_json::to_string_pretty(p) {
            let _ = fs::write(p_dir.join("profile.json"), json);
        }
    }


    cleanup_profile_mod_lists(data);
}

pub fn auto_add_scanned_mods_to_profile(data: &mut AppData) {
    let current_profile_id = data.current_profile_id.clone();
    let program_path = data.settings.program_path.clone();
    let mut modified = false;

    if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
        let ue4ss_active = profile.ue4ss_enabled;
        let palschema_active = profile.palschema_enabled;

        for m in &data.mods {
            if m.nexus_author.as_deref() == Some("UE4SS Native Mod") {
                continue;
            }

            let is_workshop = m.nexus_summary.as_deref()
                .map_or(false, |s| s.starts_with("Steam Workshop Mod"));

            if is_workshop {
                if (m.mod_type == crate::models::ModType::Ue4ss || m.mod_type == crate::models::ModType::Hybrid) && !ue4ss_active {
                    continue;
                }
                if m.mod_type == crate::models::ModType::PalSchema && !palschema_active {
                    continue;
                }
                let already_installed = profile.installed_mod_ids.iter().any(|id| {
                    mod_matches_profile_entry(m, id)
                });
                if !already_installed {
                    profile.installed_mod_ids.push(m.id.clone());
                    modified = true;
                }
                if m.enabled {
                    let already_enabled = profile.enabled_mod_ids.iter().any(|id| {
                        mod_matches_profile_entry(m, id)
                    });
                    if !already_enabled {
                        profile.enabled_mod_ids.push(m.id.clone());
                        modified = true;
                    }
                }
                continue;
            }

            if (m.mod_type == crate::models::ModType::Ue4ss || m.mod_type == crate::models::ModType::Hybrid) && !ue4ss_active {
                continue;
            }
            if m.mod_type == crate::models::ModType::PalSchema && !palschema_active {
                continue;
            }

            let is_in_game = !m.game_path.is_empty() && Path::new(&m.game_path).exists();
            let is_disabled = !m.disabled_path.is_empty()
                && m.disabled_path.replace("\\", "/").contains(&format!("/profiles/{}/disabled_mods/", profile.id))
                && Path::new(&m.disabled_path).exists();

            let is_in_other_profile_disabled = !m.disabled_path.is_empty()
                && !m.disabled_path.replace("\\", "/").contains(&format!("/profiles/{}/disabled_mods/", profile.id))
                && Path::new(&m.disabled_path).exists();

            if (is_in_game && !is_in_other_profile_disabled) || is_disabled {
                let already_installed = profile.installed_mod_ids.iter().any(|id| {
                    mod_matches_profile_entry(m, id)
                });
                if !already_installed {
                    profile.installed_mod_ids.push(m.id.clone());
                    modified = true;
                }

                if is_in_game && m.enabled {
                    let already_enabled = profile.enabled_mod_ids.iter().any(|id| {
                        mod_matches_profile_entry(m, id)
                    });
                    if !already_enabled {
                        profile.enabled_mod_ids.push(m.id.clone());
                        modified = true;
                    }
                }
            }
        }

        let orig_inst_len = profile.installed_mod_ids.len();
        profile.installed_mod_ids.retain(|id| !id.trim().is_empty());
        let mut seen_inst = std::collections::HashSet::new();
        profile.installed_mod_ids.retain(|id| seen_inst.insert(id.to_lowercase()));

        let orig_enb_len = profile.enabled_mod_ids.len();
        profile.enabled_mod_ids.retain(|id| !id.trim().is_empty());
        let mut seen_enb = std::collections::HashSet::new();
        profile.enabled_mod_ids.retain(|id| seen_enb.insert(id.to_lowercase()));

        if modified || profile.installed_mod_ids.len() != orig_inst_len || profile.enabled_mod_ids.len() != orig_enb_len {
            let p_dir = get_profile_dir(&program_path, &current_profile_id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
    }
}

