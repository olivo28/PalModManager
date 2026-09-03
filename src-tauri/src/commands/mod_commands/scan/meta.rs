use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{ModInfo, ModType};
use super::super::utils::{detect_config, file_install_date};

pub fn load_pmm_meta(path: &Path) -> Option<ModInfo> {
    let path_str = path.to_string_lossy().to_string();
    let is_disabled = path_str.contains("disabled_mods");
    let game_path = if is_disabled { String::new() } else { path_str.clone() };
    let disabled_path = if is_disabled { path_str.clone() } else { String::new() };

    if path.is_dir() {
        if let Some(meta) = crate::profiles::utils::consolidate_mod_folder_metadata(path) {
            let folder_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Mod".to_string());
            let name = if !meta.name.is_empty() { meta.name } else { folder_name.clone() };
            let version = if !meta.version.is_empty() { meta.version } else { "1.0.0".to_string() };

            let mod_type = if meta.nexus_mod_id == Some(1626) || name.to_lowercase().contains("altermatic") || folder_name.to_lowercase().contains("altermatic") {
                ModType::Altermatic
            } else if meta.nexus_mod_id == Some(1894) || name.to_lowercase().contains("unipalui") || folder_name.to_lowercase().contains("unipalui") {
                ModType::Hybrid
            } else {
                match meta.mod_type.as_deref().unwrap_or("ue4ss").to_lowercase().as_str() {
                    "ue4ss" => ModType::Ue4ss,
                    "palschema" => ModType::PalSchema,
                    "pak" => ModType::Pak,
                    "logicmods" => ModType::LogicMods,
                    "altermatic" => ModType::Altermatic,
                    "hybrid" => ModType::Hybrid,
                    _ => ModType::Hybrid,
                }
            };

            return Some(ModInfo {
                id: folder_name,
                name,
                mod_type,
                nexus_mod_id: meta.nexus_mod_id,
                nexus_url: meta.nexus_url.or_else(|| meta.nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id))),
                nexus_author: meta.author,
                nexus_summary: meta.description,
                nexus_picture_url: meta.nexus_picture_url,
                nexus_endorsements: None,
                nexus_downloads: None,
                version,
                install_date: file_install_date(path),
                source_zip: String::new(),
                config_path: detect_config(path),
                config_type: Some("auto".to_string()),
                enabled: !is_disabled,
                game_path,
                disabled_path,
                pak_destination: None,
                has_enabled_txt: path.join("enabled.txt").exists(),
                mods_txt_order: None,
                extra_files: meta.installed_files.unwrap_or_default(),
                nexus_description: None,
                nexus_version_cached: None,
                nexus_cached_at: None,
                nexus_category: meta.category,
                nexus_tags: Vec::new(),
                github_repo: None,
                github_version: None,
                github_cached_at: None,
                update_date: None,
                library_zip: None,
                ignored_version: None,
                nexus_file_id: meta.nexus_file_id,
                ignored_keys: None,
                has_pending_update: None,
                origin_load_method: None,
                custom_notes: meta.custom_notes,
            });
        }
    } else if path.is_file() {
        let pmm_path = PathBuf::from(format!("{}.pmm.json", path.to_string_lossy()));
        let pmm_alt = path.parent().and_then(|p| path.file_stem().map(|s| p.join(format!("{}.pak.pmm.json", s.to_string_lossy()))));
        let target_pmm = if pmm_path.exists() { Some(pmm_path) } else if pmm_alt.as_ref().map_or(false, |p| p.exists()) { pmm_alt } else { None };

        if let Some(target) = target_pmm {
            if let Ok(content) = fs::read_to_string(&target) {
                if let Ok(meta) = serde_json::from_str::<crate::models::PmmMetadata>(&content) {
                    let file_stem = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Mod".to_string());
                    let name = if !meta.name.is_empty() { meta.name } else { file_stem.clone() };
                    let version = if !meta.version.is_empty() { meta.version } else { "1.0.0".to_string() };

                    let is_altermatic_skin = if let Some(parent) = path.parent() {
                        let swap_json_dir = parent.join("SwapJSON");
                        let alt_swap_json_dir = parent.parent().map(|p| p.join("~mods").join("SwapJSON"));
                        swap_json_dir.join(format!("{}.json", file_stem)).exists()
                            || swap_json_dir.join(format!("{}.json", file_stem.strip_suffix("_P").unwrap_or(&file_stem))).exists()
                            || alt_swap_json_dir.as_ref().map_or(false, |d| d.join(format!("{}.json", file_stem)).exists() || d.join(format!("{}.json", file_stem.strip_suffix("_P").unwrap_or(&file_stem))).exists())
                    } else {
                        false
                    };

                    let mod_type = if meta.nexus_mod_id == Some(1626) || name.to_lowercase().contains("altermatic") || file_stem.to_lowercase().contains("altermatic") || is_altermatic_skin {
                        ModType::Altermatic
                    } else if meta.nexus_mod_id == Some(1894) || name.to_lowercase().contains("unipalui") || file_stem.to_lowercase().contains("unipalui") {
                        ModType::Hybrid
                    } else {
                        match meta.mod_type.as_deref().unwrap_or("pak").to_lowercase().as_str() {
                            "logicmods" => ModType::LogicMods,
                            "palschema" => ModType::PalSchema,
                            "ue4ss" => ModType::Ue4ss,
                            "altermatic" => ModType::Altermatic,
                            "hybrid" => ModType::Hybrid,
                            _ => ModType::Pak,
                        }
                    };

                    return Some(ModInfo {
                        id: file_stem,
                        name,
                        mod_type,
                        nexus_mod_id: meta.nexus_mod_id,
                        nexus_url: meta.nexus_url.or_else(|| meta.nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id))),
                        nexus_author: meta.author,
                        nexus_summary: meta.description,
                        nexus_picture_url: meta.nexus_picture_url,
                        nexus_endorsements: None,
                        nexus_downloads: None,
                        version,
                        install_date: file_install_date(path),
                        source_zip: String::new(),
                        config_path: detect_config(path),
                        config_type: Some("auto".to_string()),
                        enabled: !is_disabled,
                        game_path,
                        disabled_path,
                        pak_destination: None,
                        has_enabled_txt: false,
                        mods_txt_order: None,
                        extra_files: meta.installed_files.unwrap_or_default(),
                        nexus_description: None,
                        nexus_version_cached: None,
                        nexus_cached_at: None,
                        nexus_category: meta.category,
                        nexus_tags: Vec::new(),
                        github_repo: None,
                        github_version: None,
                        github_cached_at: None,
                        update_date: None,
                        library_zip: None,
                        ignored_version: None,
                        nexus_file_id: meta.nexus_file_id,
                        ignored_keys: None,
                        has_pending_update: None,
                        origin_load_method: None,
                        custom_notes: meta.custom_notes,
                    });
                }
            }
        }
    }
    None
}
