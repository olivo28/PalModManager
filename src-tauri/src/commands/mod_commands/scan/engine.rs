use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{ModInfo, ModType, WorkshopInstallType};
use super::ue4ss::scan_ue4ss_mods;
use super::palschema::scan_palschema_mods;
use super::paks::scan_pak_mods;
use super::disabled::scan_disabled_mods;
use super::merge::merge_scan_with_db;

pub fn scan_mods_internal(
    game_path: &str,
    program_path: &str,
    current_profile_id: &str,
    installed_ids: &[String],
    db_mods: &[ModInfo],
) -> Vec<ModInfo> {
    if game_path.is_empty() {
        return Vec::new();
    }
    let _guard = crate::watcher::pause_watcher_guard();
    let game = PathBuf::from(game_path);
    let mut fs_mods: Vec<ModInfo> = vec![];

    let wmods = crate::workshop::scan_workshop_mods(game_path);
    let workshop_package_names: std::collections::HashSet<String> = wmods.iter()
        .map(|m| m.package_name.to_lowercase())
        .collect();

    let gp = crate::dependency_checker::build_game_profile(&game);
    let ue4ss_mods_dir = gp.ue4ss_mods_dir.clone();
    if ue4ss_mods_dir.exists() {
        scan_ue4ss_mods(&ue4ss_mods_dir, &mut fs_mods, &workshop_package_names);
    }

    let palschema_dir = gp.palschema_mods_dir.clone();
    if palschema_dir.exists() {
        scan_palschema_mods(&palschema_dir, &mut fs_mods, &workshop_package_names);
    }

    let registered_patches = crate::pak_patcher::load_profile_patches_registry(program_path, current_profile_id);

    let pak_mods_dir = gp.paks_dir.clone();
    if pak_mods_dir.exists() {
        scan_pak_mods(&pak_mods_dir, "pak", &mut fs_mods, &registered_patches);
    }

    let logic_mods_dir = gp.logic_mods_dir.clone();
    if logic_mods_dir.exists() {
        scan_pak_mods(&logic_mods_dir, "logicmods", &mut fs_mods, &registered_patches);
    }

    let disabled_base = PathBuf::from(program_path)
        .join("profiles")
        .join(current_profile_id)
        .join("disabled_mods");
    if disabled_base.exists() {
        scan_disabled_mods(&disabled_base, &mut fs_mods);
    }

    for wmod in wmods.iter().filter(|m| !m.is_framework && (m.is_installed || m.is_active)) {
        let game_mod_path = if wmod.install_type == WorkshopInstallType::PalSchemaMod {
            gp.palschema_mods_dir.join(&wmod.package_name)
        } else {
            gp.ue4ss_mods_dir.join(&wmod.package_name)
        };
        
        let mut details = format!(
            "Steam Workshop Mod\n\n• Workshop ID: {}\n• Author: {}\n• Package Name: {}\n• Install Type: {:?}",
            wmod.workshop_id,
            wmod.author,
            wmod.package_name,
            wmod.install_type
        );
        if !wmod.dependencies.is_empty() {
            details.push_str(&format!("\n• Dependencies: {}", wmod.dependencies.join(", ")));
        }

        let display_name = format!("{} (Workshop)", wmod.mod_name);
        let installed_version = if wmod.is_installed {
            let manifest_dir = game.join("Mods").join("ManagedMods").join(&wmod.package_name);
            let installed_info_path = manifest_dir.join("Info.json");
            let mut inst_ver = "unknown".to_string();
            if installed_info_path.exists() {
                if let Ok(inst_info_str) = fs::read_to_string(&installed_info_path) {
                    if let Ok(inst_info) = serde_json::from_str::<crate::workshop::WorkshopInfoJson>(&inst_info_str) {
                        inst_ver = inst_info.version;
                    }
                }
            }
            inst_ver
        } else {
            "unknown".to_string()
        };

        fs_mods.push(ModInfo {
            id: wmod.package_name.clone(),
            name: display_name,
            mod_type: match wmod.install_type {
                WorkshopInstallType::PalSchemaMod => ModType::PalSchema,
                _ => ModType::Ue4ss,
            },
            nexus_mod_id: None,
            nexus_url: Some(format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", wmod.workshop_id)),
            nexus_author: Some(wmod.author.clone()),
            nexus_summary: Some(details),
            nexus_picture_url: wmod.thumbnail_path.clone(),
            nexus_endorsements: None,
            nexus_downloads: None,
            version: installed_version,
            install_date: String::new(),
            source_zip: String::new(),
            config_path: None,
            config_type: Some("auto".to_string()),
            enabled: wmod.is_active,
            game_path: game_mod_path.to_string_lossy().to_string(),
            disabled_path: String::new(),
            pak_destination: None,
            has_enabled_txt: game_mod_path.join("enabled.txt").exists(),
            mods_txt_order: None,
            extra_files: Vec::new(),
            nexus_description: None,
            nexus_version_cached: Some(wmod.version.clone()),
            nexus_cached_at: None,
            nexus_category: None,
            nexus_tags: Vec::new(),
            github_repo: None,
            github_version: None,
            github_cached_at: None,
            update_date: None,
            library_zip: None,
            ignored_version: None,
            nexus_file_id: None,
            ignored_keys: None,
            has_pending_update: Some(wmod.has_pending_update),
            origin_load_method: None,
            custom_notes: None,
            original_name: None,
            custom_name: None,
        });
    }

    merge_scan_with_db(current_profile_id, installed_ids, db_mods, &fs_mods, &workshop_package_names)
}
