use std::path::Path;
use crate::models::{ModInfo, ModType};
use super::super::utils::get_physical_identity;

pub fn merge_scan_with_db(
    current_profile_id: &str,
    installed_ids: &[String],
    db_mods: &[ModInfo],
    fs_mods: &[ModInfo],
    workshop_package_names: &std::collections::HashSet<String>,
) -> Vec<ModInfo> {
    let mut consolidated_db: Vec<ModInfo> = Vec::new();
    for db_mod in db_mods {
        let db_path_norm = if !db_mod.game_path.is_empty() {
            db_mod.game_path.replace("\\", "/").to_lowercase()
        } else {
            db_mod.disabled_path.replace("\\", "/").to_lowercase()
        };

        if !db_path_norm.is_empty() {
            let is_extra_of_other = db_mods.iter().any(|other| {
                other.id != db_mod.id && other.extra_files.iter().any(|extra| {
                    extra.replace("\\", "/").to_lowercase() == db_path_norm
                })
            });
            if is_extra_of_other {
                crate::logger::log(&format!("merge_scan_with_db: Purging duplicate sub-component mod '{}' because its files are owned by another mod.", db_mod.name));
                continue;
            }
        }

        let db_id = get_physical_identity(&db_mod.game_path, &db_mod.disabled_path);
        if let Some(existing_idx) = consolidated_db.iter().position(|m| {
            if m.mod_type != db_mod.mod_type {
                return false;
            }
            let existing_id = get_physical_identity(&m.game_path, &m.disabled_path);
            existing_id == db_id
        }) {
            let existing = &mut consolidated_db[existing_idx];
            if existing.nexus_mod_id.is_none() && db_mod.nexus_mod_id.is_some() {
                existing.nexus_mod_id = db_mod.nexus_mod_id;
                existing.nexus_url = db_mod.nexus_url.clone();
                existing.nexus_author = db_mod.nexus_author.clone();
                existing.nexus_summary = db_mod.nexus_summary.clone();
                existing.nexus_picture_url = db_mod.nexus_picture_url.clone();
                existing.nexus_endorsements = db_mod.nexus_endorsements;
                existing.nexus_downloads = db_mod.nexus_downloads;
                existing.name = db_mod.name.clone();
            }
            if existing.game_path.is_empty() && !db_mod.game_path.is_empty() {
                existing.game_path = db_mod.game_path.clone();
                existing.enabled = db_mod.enabled;
            }
            if existing.disabled_path.is_empty() && !db_mod.disabled_path.is_empty() {
                existing.disabled_path = db_mod.disabled_path.clone();
            }
        } else {
            consolidated_db.push(db_mod.clone());
        }
    }

    let mut result: Vec<ModInfo> = Vec::new();
    let mut matched_db: Vec<bool> = vec![false; consolidated_db.len()];

    for fs_mod in fs_mods {
        let fs_path_norm = if !fs_mod.game_path.is_empty() {
            fs_mod.game_path.replace("\\", "/").to_lowercase()
        } else {
            fs_mod.disabled_path.replace("\\", "/").to_lowercase()
        };

        if !fs_path_norm.is_empty() {
            let is_registered_as_extra = consolidated_db.iter().any(|dm| {
                dm.extra_files.iter().any(|extra| {
                    extra.replace("\\", "/").to_lowercase() == fs_path_norm
                })
            });
            if is_registered_as_extra {
                continue;
            }
        }

        let fs_id = get_physical_identity(&fs_mod.game_path, &fs_mod.disabled_path);
        let mut matched_idx = None;
        for (i, dm) in consolidated_db.iter().enumerate() {
            if matched_db[i] {
                continue;
            }
            let db_id = get_physical_identity(&dm.game_path, &dm.disabled_path);
            let same_physical_path = !fs_path_norm.is_empty() && (
                (!dm.game_path.is_empty() && dm.game_path.replace('\\', "/").to_lowercase() == fs_path_norm) ||
                (!dm.disabled_path.is_empty() && dm.disabled_path.replace('\\', "/").to_lowercase() == fs_path_norm)
            );
            let same_physical_id = !db_id.is_empty() && !fs_id.is_empty() && db_id.to_lowercase() == fs_id.to_lowercase();
            let same_id = dm.id.to_lowercase() == fs_mod.id.to_lowercase();
            let same_name = !dm.name.is_empty() && dm.name.to_lowercase() == fs_mod.name.to_lowercase();

            if same_physical_path || same_physical_id || same_id || same_name {
                matched_idx = Some(i);
                break;
            }
        }

        if let Some(db_idx) = matched_idx {
            matched_db[db_idx] = true;
            let db_mod = &consolidated_db[db_idx];
            let mut merged = db_mod.clone();
            if fs_mod.mod_type == ModType::Altermatic || db_mod.mod_type == ModType::Altermatic {
                merged.mod_type = ModType::Altermatic;
            } else if fs_mod.mod_type == ModType::Hybrid || db_mod.mod_type == ModType::Hybrid {
                merged.mod_type = ModType::Hybrid;
            }
            merged.game_path = fs_mod.game_path.clone();
            merged.disabled_path = fs_mod.disabled_path.clone();
            merged.has_enabled_txt = fs_mod.has_enabled_txt;
            if merged.config_path.is_none() {
                merged.config_path = fs_mod.config_path.clone();
            }

            // Filter extra_files to only keep those that physically exist or belong to the active tree
            let mut valid_extras = Vec::new();
            for extra in &db_mod.extra_files {
                let extra_p = Path::new(extra);
                if extra_p.exists() {
                    valid_extras.push(extra.clone());
                }
            }
            for extra in &fs_mod.extra_files {
                if !valid_extras.contains(extra) {
                    valid_extras.push(extra.clone());
                }
            }
            merged.extra_files = valid_extras;

            merged.enabled = fs_mod.enabled;
            if fs_mod.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod")) {
                merged.nexus_summary = fs_mod.nexus_summary.clone();
                merged.has_pending_update = fs_mod.has_pending_update;
                merged.version = fs_mod.version.clone();
                merged.nexus_version_cached = fs_mod.nexus_version_cached.clone();
            }
            result.push(merged);
        } else {
            result.push(fs_mod.clone());
        }
    }

    for (i, dm) in consolidated_db.iter().enumerate() {
        if !matched_db[i] {
            let is_installed_in_current = installed_ids.iter().any(|entry| {
                crate::profiles::mod_matches_profile_entry(dm, entry)
            });

            let normalized_disabled = dm.disabled_path.replace('\\', "/");
            let is_disabled_in_other_profile = normalized_disabled.contains("/profiles/") 
                && !normalized_disabled.contains(&format!("/profiles/{}/", current_profile_id));

            if !is_installed_in_current || is_disabled_in_other_profile {
                let is_workshop = dm.nexus_summary.as_deref().map_or(false, |s| s.starts_with("Steam Workshop Mod"));
                if is_workshop {
                    if workshop_package_names.contains(&dm.id.to_lowercase()) {
                        let mut cleaned = dm.clone();
                        cleaned.game_path = String::new();
                        cleaned.enabled = false;
                        result.push(cleaned);
                    }
                } else {
                    result.push(dm.clone());
                }
                continue;
            }

            let has_game = !dm.game_path.is_empty() && Path::new(&dm.game_path).exists();
            let has_disabled = !dm.disabled_path.is_empty() && Path::new(&dm.disabled_path).exists();
            let has_metadata = dm.nexus_mod_id.is_some() || dm.library_zip.is_some();
 
            if has_game || has_disabled {
                result.push(dm.clone());
            } else if has_metadata {
                let mut kept = dm.clone();
                kept.enabled = false;
                kept.game_path = String::new();
                kept.disabled_path = String::new();
                kept.extra_files = Vec::new();
                result.push(kept);
            } else {
                crate::logger::log(&format!("merge_scan_with_db: Mod '{}' no longer exists on disk and has no metadata. Purging from database.", dm.name));
            }
        }
    }

    let mut final_deduped: Vec<ModInfo> = Vec::new();
    for m in result {
        let m_id = m.id.to_lowercase();
        let m_phys = get_physical_identity(&m.game_path, &m.disabled_path).to_lowercase();
        let m_game = m.game_path.replace('\\', "/").to_lowercase();
        let m_disabled = m.disabled_path.replace('\\', "/").to_lowercase();

        if let Some(existing_idx) = final_deduped.iter().position(|em| {
            let em_id = em.id.to_lowercase();
            let em_phys = get_physical_identity(&em.game_path, &em.disabled_path).to_lowercase();
            let em_game = em.game_path.replace('\\', "/").to_lowercase();
            let em_disabled = em.disabled_path.replace('\\', "/").to_lowercase();

            (em_id == m_id) ||
            (!m_phys.is_empty() && em_phys == m_phys) ||
            (!m_game.is_empty() && em_game == m_game) ||
            (!m_disabled.is_empty() && em_disabled == m_disabled) ||
            (!em.name.is_empty() && em.name.to_lowercase() == m.name.to_lowercase())
        }) {
            let existing = final_deduped[existing_idx].clone();
            let mut merged = existing.clone();
            if m.mod_type == ModType::Altermatic || existing.mod_type == ModType::Altermatic {
                merged.mod_type = ModType::Altermatic;
            } else if m.mod_type == ModType::Hybrid || existing.mod_type == ModType::Hybrid {
                merged.mod_type = ModType::Hybrid;
            }
            if merged.game_path.is_empty() && !m.game_path.is_empty() {
                merged.game_path = m.game_path.clone();
            }
            if merged.disabled_path.is_empty() && !m.disabled_path.is_empty() {
                merged.disabled_path = m.disabled_path.clone();
            }
            if merged.nexus_mod_id.is_none() && m.nexus_mod_id.is_some() {
                merged.nexus_mod_id = m.nexus_mod_id;
                merged.nexus_url = m.nexus_url.clone();
                merged.nexus_author = m.nexus_author.clone();
                merged.nexus_summary = m.nexus_summary.clone();
                merged.nexus_picture_url = m.nexus_picture_url.clone();
            }
            for extra in &m.extra_files {
                if !merged.extra_files.contains(extra) {
                    merged.extra_files.push(extra.clone());
                }
            }
            let is_strictly_disabled = (!merged.disabled_path.is_empty() && merged.game_path.is_empty())
                || (!m.disabled_path.is_empty() && m.game_path.is_empty())
                || (!existing.disabled_path.is_empty() && existing.game_path.is_empty());
            merged.enabled = if is_strictly_disabled {
                false
            } else {
                existing.enabled || m.enabled
            };
            if merged.config_path.is_none() && m.config_path.is_some() {
                merged.config_path = m.config_path.clone();
            }
            final_deduped[existing_idx] = merged;
        } else {
            final_deduped.push(m);
        }
    }

    final_deduped
}
