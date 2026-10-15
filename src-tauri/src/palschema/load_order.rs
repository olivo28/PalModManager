use std::fs;
use std::path::{Path, PathBuf};

/// Parses a semver string (e.g. "0.7.0", "v0.7.0-beta", "1.0.0") to check if it is >= 0.7.0.
pub fn is_version_at_least_0_7_0(ver_str: &str) -> bool {
    let clean = ver_str.trim().trim_start_matches('v').trim_start_matches('V');
    let parts: Vec<&str> = clean.split('.').collect();
    if let Some(major) = parts.first().and_then(|p| p.parse::<u32>().ok()) {
        if major > 0 {
            return true;
        }
        if let Some(minor_part) = parts.get(1) {
            let num_str: String = minor_part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(minor) = num_str.parse::<u32>() {
                return minor >= 7;
            }
        }
    }
    false
}

/// Checks whether PalSchema in the configured game installation supports native load order.
/// Returns true if `load_order.json` already exists or if PalSchema version is >= 0.7.0.
pub fn is_palschema_native_flo(game_path: &Path) -> bool {
    let gp = crate::dependency_checker::build_game_profile(game_path);
    let palschema_base = match gp.palschema_mods_dir.parent() {
        Some(base) => base,
        None => return false,
    };

    // 1. Direct indicator: load_order.json already exists in PalSchema folder
    let load_order_file = palschema_base.join("load_order.json");
    if load_order_file.exists() {
        return true;
    }

    // 2. Version check: palschema.version file
    let version_file = palschema_base.join("palschema.version");
    if version_file.exists() {
        if let Ok(ver) = fs::read_to_string(&version_file) {
            if is_version_at_least_0_7_0(&ver) {
                return true;
            }
        }
    }

    // 3. Version check: Info.json (used in Workshop or standalone installs)
    let info_file = palschema_base.join("Info.json");
    if info_file.exists() {
        if let Ok(content) = fs::read_to_string(&info_file) {
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(ver) = info.get("version").and_then(|v| v.as_str()) {
                    if is_version_at_least_0_7_0(ver) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Returns the path to `<UE4SS>/Mods/PalSchema/load_order.json`.
pub fn get_load_order_file_path(game_path: &Path) -> Option<PathBuf> {
    let gp = crate::dependency_checker::build_game_profile(game_path);
    gp.palschema_mods_dir.parent().map(|base| base.join("load_order.json"))
}

/// Reads the current native `load_order.json` from the game installation.
/// All entries are converted to lowercase matching PalSchema's `ToLowerCase(LoadOrderList)` logic.
pub fn read_load_order(game_path: &Path) -> Option<Vec<String>> {
    let file_path = get_load_order_file_path(game_path)?;
    if !file_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&file_path).ok()?;
    let list: Vec<String> = serde_json::from_str(&content).ok()?;
    Some(list.into_iter().map(|id| id.trim().to_lowercase()).collect())
}

/// Writes an ordered list of mod IDs to `<UE4SS>/Mods/PalSchema/load_order.json`.
pub fn write_load_order(game_path: &Path, ordered_ids: &[String]) -> Result<(), String> {
    let file_path = get_load_order_file_path(game_path)
        .ok_or_else(|| "Failed to resolve PalSchema directory".to_string())?;

    if let Some(parent) = file_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let lowercased: Vec<String> = ordered_ids
        .iter()
        .map(|id| id.trim().to_lowercase())
        .filter(|id| !id.is_empty())
        .collect();

    let json_str = serde_json::to_string_pretty(&lowercased)
        .map_err(|e| format!("Failed to serialize load_order.json: {}", e))?;

    fs::write(&file_path, json_str)
        .map_err(|e| format!("Failed to write {}: {}", file_path.display(), e))?;

    crate::logger::log(&format!(
        "PalSchema: Successfully updated load_order.json with {} items",
        lowercased.len()
    ));

    Ok(())
}

/// Auto-migrates legacy PalSchema FLO setups (`Storage/` + NTFS Junctions) to clean native folders.
/// Restores directories from `PalSchema/Storage/` back to `PalSchema/mods/`, purges junctions,
/// deletes `FLOPalSchema.json`, and removes the empty `Storage/` directory.
pub fn migrate_legacy_storage(game_path: &Path) -> Result<(), String> {
    let gp = crate::dependency_checker::build_game_profile(game_path);
    let palschema_mods_dir = gp.palschema_mods_dir.clone();
    let palschema_storage_dir = gp.palschema_storage_dir.clone();
    let palschema_base = match palschema_mods_dir.parent() {
        Some(base) => base.to_path_buf(),
        None => return Ok(()),
    };

    let legacy_map = palschema_base.join("FLOPalSchema.json");
    if legacy_map.exists() {
        crate::logger::log("PalSchema: Cleaning up obsolete FLOPalSchema.json map");
        let _ = fs::remove_file(&legacy_map);
    }

    if !palschema_mods_dir.exists() {
        return Ok(());
    }

    // 1. Remove all NTFS junctions and numbered prefixed directories in mods/
    if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.eq_ignore_ascii_case("Storage") {
                continue;
            }

            if crate::profiles::is_junction_or_symlink(&path) {
                let _ = crate::profiles::remove_junction_or_symlink(&path);
            } else if path.is_dir() {
                let is_prefixed = name.len() > 4
                    && name.chars().take(3).all(|c| c.is_ascii_digit())
                    && name.chars().nth(3) == Some('_');
                if is_prefixed {
                    let _ = crate::profiles::remove_junction_or_symlink(&path);
                    if path.exists() {
                        let _ = fs::remove_dir_all(&path);
                    }
                }
            }
        }
    }

    // 2. Move real mod folders from Storage/ back to mods/
    if palschema_storage_dir.exists() {
        crate::logger::log("PalSchema: Migrating mod folders from Storage/ back to mods/");
        if let Ok(entries) = fs::read_dir(&palschema_storage_dir) {
            for entry in entries.flatten() {
                let src = entry.path();
                if src.is_dir() {
                    let name = match src.file_name() {
                        Some(n) => n,
                        None => continue,
                    };
                    let dst = palschema_mods_dir.join(name);
                    let _ = crate::profiles::move_path(&src, &dst);
                }
            }
        }
        let _ = fs::remove_dir(&palschema_storage_dir);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_at_least_0_7_0() {
        assert!(is_version_at_least_0_7_0("0.7.0"));
        assert!(is_version_at_least_0_7_0("v0.7.0"));
        assert!(is_version_at_least_0_7_0("0.7.1"));
        assert!(is_version_at_least_0_7_0("0.8.0"));
        assert!(is_version_at_least_0_7_0("1.0.0"));
        assert!(!is_version_at_least_0_7_0("0.6.9"));
        assert!(!is_version_at_least_0_7_0("0.5.2"));
        assert!(!is_version_at_least_0_7_0("unknown"));
    }

    #[test]
    fn test_read_write_load_order() {
        let temp_file = std::env::temp_dir().join(format!("pmm_test_lo_{}.json", uuid::Uuid::new_v4()));
        let items = vec!["mod_b".to_string(), "mod_a".to_string()];

        assert!(write_load_order(&temp_file, &items).is_ok());
        let read = read_load_order(&temp_file).unwrap();
        assert_eq!(read, vec!["mod_b".to_string(), "mod_a".to_string()]);

        let _ = fs::remove_file(&temp_file);
    }
}
