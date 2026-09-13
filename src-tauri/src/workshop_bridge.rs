use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::models::{ModInfo, ModType};
use crate::workshop::{
    read_pal_mod_settings, resolve_workshop_root, write_pal_mod_settings,
    WorkshopInfoJson,
};
use crate::zip_handler::workshop_rule::WorkshopInstallRule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMarker {
    pub mod_id: String,
    pub mod_name: String,
    pub workshop_id: u64,
    pub package_name: String,
    pub created_at: String,
}

/// Computes a deterministic pseudo-workshop ID in the 9,000,000,000+ range.
pub fn compute_pseudo_workshop_id(mod_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    mod_id.hash(&mut hasher);
    let hash = hasher.finish();
    9_000_000_000 + (hash % 1_000_000_000)
}

/// Sanitizes mod names/ids into valid Palworld package names (alphanumeric and underscores only).
pub fn sanitize_package_name(input: &str) -> String {
    let sanitized: String = input
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "PMM_WorkshopMod".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Recursively copies directory contents from src to dst.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Checks if a mod is already registered in Palworld's Workshop folder via PMM.
pub fn is_mod_bridged(game_path: &str, mod_id: &str) -> bool {
    let workshop_root = match resolve_workshop_root(game_path) {
        Some(r) => r,
        None => return false,
    };
    let pseudo_id = compute_pseudo_workshop_id(mod_id);
    let marker_path = workshop_root.join(pseudo_id.to_string()).join(".pmm_bridge.json");
    marker_path.exists()
}

/// Bridges an installed mod to Palworld's Steam Workshop directory with an artificial Info.json
/// and updates PalModSettings.ini so it shows up in Palworld's native in-game mod management menu.
pub fn bridge_mod_to_workshop(game_path: &str, mod_info: &ModInfo) -> Result<u64, String> {
    crate::logger::log(&format!(
        "workshop_bridge: Bridging mod '{}' (id: {}) to Workshop menu...",
        mod_info.name, mod_info.id
    ));

    let workshop_root = resolve_workshop_root(game_path).ok_or_else(|| {
        "Could not resolve Steam Workshop directory. Ensure Palworld is installed via Steam.".to_string()
    })?;

    let pseudo_id = compute_pseudo_workshop_id(&mod_info.id);
    let target_dir = workshop_root.join(pseudo_id.to_string());

    if target_dir.exists() {
        crate::logger::log(&format!(
            "workshop_bridge: Target directory {:?} already exists, cleaning before re-bridge...",
            target_dir
        ));
        let _ = fs::remove_dir_all(&target_dir);
    }
    fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create workshop directory: {}", e))?;

    let package_name = sanitize_package_name(&mod_info.name);

    // 1. Copy source files into the target workshop directory
    let mut files_copied = 0;
    let main_path = if !mod_info.game_path.is_empty() && Path::new(&mod_info.game_path).exists() {
        PathBuf::from(&mod_info.game_path)
    } else if !mod_info.disabled_path.is_empty() && Path::new(&mod_info.disabled_path).exists() {
        PathBuf::from(&mod_info.disabled_path)
    } else {
        PathBuf::new()
    };

    if main_path.exists() {
        if main_path.is_dir() {
            copy_dir_recursive(&main_path, &target_dir)
                .map_err(|e| format!("Failed to copy mod directory: {}", e))?;
            files_copied += 1;
        } else {
            // It's a file (.pak, etc.)
            let file_name = main_path.file_name().ok_or_else(|| "Invalid main path file name".to_string())?;
            fs::copy(&main_path, target_dir.join(file_name))
                .map_err(|e| format!("Failed to copy main mod file: {}", e))?;
            files_copied += 1;

            // Also check for accompanying pak companions (.ucas, .utoc, .sig)
            for comp_ext in &["ucas", "utoc", "sig"] {
                let comp = main_path.with_extension(comp_ext);
                if comp.exists() {
                    let comp_name = comp.file_name().unwrap();
                    let _ = fs::copy(&comp, target_dir.join(comp_name));
                }
            }
        }
    }

    // Copy any extra files
    for extra in &mod_info.extra_files {
        let extra_p = Path::new(extra);
        if extra_p.exists() {
            if extra_p.is_dir() {
                let _ = copy_dir_recursive(extra_p, &target_dir);
            } else if let Some(fname) = extra_p.file_name() {
                let _ = fs::copy(extra_p, target_dir.join(fname));
            }
            files_copied += 1;
        }
    }

    if files_copied == 0 {
        let _ = fs::remove_dir_all(&target_dir);
        return Err("No valid mod files found on disk to bridge into Workshop.".to_string());
    }

    // 2. Generate Info.json with appropriate InstallRule
    let mut rules = Vec::new();
    match mod_info.mod_type {
        ModType::Pak => {
            rules.push(WorkshopInstallRule {
                rule_type: "Pak".to_string(),
                is_server: false,
                targets: vec![".".to_string()],
            });
        }
        ModType::PalSchema => {
            rules.push(WorkshopInstallRule {
                rule_type: "PalSchema".to_string(),
                is_server: false,
                targets: vec![".".to_string()],
            });
        }
        ModType::Ue4ss | ModType::LogicMods => {
            rules.push(WorkshopInstallRule {
                rule_type: "UE4SSMod".to_string(),
                is_server: false,
                targets: vec![".".to_string()],
            });
        }
        ModType::Hybrid => {
            rules.push(WorkshopInstallRule {
                rule_type: "Pak".to_string(),
                is_server: false,
                targets: vec![".".to_string()],
            });
            rules.push(WorkshopInstallRule {
                rule_type: "UE4SSMod".to_string(),
                is_server: false,
                targets: vec![".".to_string()],
            });
        }
        _ => {
            rules.push(WorkshopInstallRule {
                rule_type: "Pak".to_string(),
                is_server: false,
                targets: vec![".".to_string()],
            });
        }
    }

    let workshop_info = WorkshopInfoJson {
        mod_name: mod_info.name.clone(),
        package_name: package_name.clone(),
        version: if mod_info.version.is_empty() { "1.0.0".to_string() } else { mod_info.version.clone() },
        author: mod_info.nexus_author.clone().unwrap_or_else(|| "Local / Nexus".to_string()),
        dependencies: None,
        install_rule: Some(rules),
        tags: vec!["PalModManager".to_string(), "BridgedMod".to_string()],
    };

    let info_path = target_dir.join("Info.json");
    let info_json = serde_json::to_string_pretty(&workshop_info)
        .map_err(|e| format!("Failed to serialize Info.json: {}", e))?;
    fs::write(&info_path, info_json)
        .map_err(|e| format!("Failed to write Info.json: {}", e))?;

    // 3. Write PMM marker file
    let marker = BridgeMarker {
        mod_id: mod_info.id.clone(),
        mod_name: mod_info.name.clone(),
        workshop_id: pseudo_id,
        package_name: package_name.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let marker_path = target_dir.join(".pmm_bridge.json");
    let marker_json = serde_json::to_string_pretty(&marker).unwrap_or_default();
    let _ = fs::write(&marker_path, marker_json);

    // 4. Synchronize with PalModSettings.ini ActiveModList
    let mut settings = read_pal_mod_settings(game_path);
    if mod_info.enabled && !settings.active_mod_list.contains(&package_name) {
        settings.active_mod_list.push(package_name.clone());
        let _ = write_pal_mod_settings(game_path, &settings);
        crate::logger::log(&format!(
            "workshop_bridge: Added '{}' to ActiveModList in PalModSettings.ini",
            package_name
        ));
    }

    crate::logger::log(&format!(
        "workshop_bridge: Successfully bridged '{}' as Workshop ID {}",
        mod_info.name, pseudo_id
    ));

    Ok(pseudo_id)
}

/// Unbridges a previously registered artificial Workshop mod, cleaning up its files and INI entry.
pub fn unbridge_mod_from_workshop(game_path: &str, mod_id: &str) -> Result<(), String> {
    crate::logger::log(&format!("workshop_bridge: Unbridging mod id: {}", mod_id));

    let workshop_root = match resolve_workshop_root(game_path) {
        Some(r) => r,
        None => return Ok(()),
    };

    let pseudo_id = compute_pseudo_workshop_id(mod_id);
    let target_dir = workshop_root.join(pseudo_id.to_string());

    if target_dir.exists() {
        let _ = fs::remove_dir_all(&target_dir);
        crate::logger::log(&format!(
            "workshop_bridge: Removed artificial workshop directory {:?}",
            target_dir
        ));
    }

    // Clean up ActiveModList from PalModSettings.ini
    let mut settings = read_pal_mod_settings(game_path);
    let initial_len = settings.active_mod_list.len();
    let sanitized_id = sanitize_package_name(mod_id);
    settings.active_mod_list.retain(|item| {
        item != mod_id && item != &sanitized_id
    });

    if settings.active_mod_list.len() != initial_len {
        let _ = write_pal_mod_settings(game_path, &settings);
        crate::logger::log("workshop_bridge: Removed package from PalModSettings.ini ActiveModList");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_pseudo_workshop_id() {
        let id1 = compute_pseudo_workshop_id("cool_mod_123");
        let id2 = compute_pseudo_workshop_id("cool_mod_123");
        assert_eq!(id1, id2);
        assert!(id1 >= 9_000_000_000 && id1 < 10_000_000_000);

        let id3 = compute_pseudo_workshop_id("another_mod_456");
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_sanitize_package_name() {
        assert_eq!(sanitize_package_name("My Super Mod! 1.0"), "My_Super_Mod__1_0");
        assert_eq!(sanitize_package_name("___"), "PMM_WorkshopMod");
        assert_eq!(sanitize_package_name("SimpleMod"), "SimpleMod");
    }
}
