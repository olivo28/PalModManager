use crate::models::ModInfo;
use super::helpers::{get_physical_identity, normalize_name};

pub fn check_mod_exists(
    folder_name: &str,
    mod_type: &crate::models::ModType,
    nexus_id: Option<u32>,
    existing_mods: &[ModInfo],
) -> Option<ModInfo> {
    let zip_norm = folder_name.replace(' ', "").replace('-', "").replace('_', "").to_lowercase();
    let norm_incoming_name = normalize_name(folder_name);
    
    existing_mods
        .iter()
        .find(|m| {
            let is_type_compatible = m.mod_type == *mod_type
                || m.mod_type == crate::models::ModType::Hybrid
                || *mod_type == crate::models::ModType::Hybrid;

            if let (Some(nid1), Some(nid2)) = (nexus_id, m.nexus_mod_id) {
                if nid1 == nid2 {
                    let db_id = get_physical_identity(&m.game_path, &m.disabled_path);
                    // Same Nexus page alone is not a conflict — a mod page can ship multiple
                    // independent file variants (e.g. body type 1 and body type 2). Only treat
                    // as a duplicate when they map to the same physical file stem on disk.
                    if !db_id.is_empty() && db_id == zip_norm && is_type_compatible {
                        return true;
                    }
                }
            }

            let db_id = get_physical_identity(&m.game_path, &m.disabled_path);
            if !db_id.is_empty() && db_id == zip_norm && is_type_compatible {
                return true;
            }

            if normalize_name(&m.name) == norm_incoming_name && is_type_compatible {
                return true;
            }
            false
        })
        .cloned()
}
