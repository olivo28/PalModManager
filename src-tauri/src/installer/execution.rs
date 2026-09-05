use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{ModInfo, ModType, RouteType};
use super::helpers::{
    copy_folder_contents, detect_config_local, determine_mod_id,
    get_palschema_component_root, get_ue4ss_component_root,
    move_path, normalize_path_separator,
};

pub fn execute_manifest(
    manifest: &crate::models::InstallManifest,
    extracted_dir: &Path,
    game_path: &Path,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    now: &str,
    force_load_order_ue4ss: bool,
    force_load_order_palschema: bool,
) -> Result<ModInfo, String> {
    let mut component_paths = Vec::new();
    let ue4ss_mods_dir = crate::dependency_checker::get_ue4ss_mods_dir(game_path);
    let ue4ss_component = normalize_path_separator(&ue4ss_mods_dir.join(&manifest.folder_name).to_string_lossy());
    let palschema_component = normalize_path_separator(&ue4ss_mods_dir.join("PalSchema").join("mods").join(&manifest.folder_name).to_string_lossy());

    let mut primary_path = String::new();
    let mut config_path = None;

    // 1. Copy/move files defined in the manifest routes
    for route in &manifest.routes {
        let src = extracted_dir.join(&route.zip_path);
        if !src.exists() {
            continue;
        }

        let dst = PathBuf::from(&route.dest_path);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create destination parent: {}", e))?;
        }

        if src.is_dir() {
            copy_folder_contents(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| format!("Failed to copy file from zip: {}", e))?;
        }

        let dst_str = dst.to_string_lossy().to_string();

        let comp_path = match route.route_type {
            RouteType::Ue4ss => get_ue4ss_component_root(&dst_str).unwrap_or_else(|| ue4ss_component.clone()),
            RouteType::PalSchema => get_palschema_component_root(&dst_str).unwrap_or_else(|| palschema_component.clone()),
            RouteType::Pak | RouteType::LogicMods | RouteType::Companion => normalize_path_separator(&dst_str),
            RouteType::Passthrough => {
                if manifest.has_ue4ss {
                    get_ue4ss_component_root(&dst_str).unwrap_or_else(|| ue4ss_component.clone())
                } else if manifest.has_palschema {
                    get_palschema_component_root(&dst_str).unwrap_or_else(|| palschema_component.clone())
                } else {
                    normalize_path_separator(&dst_str)
                }
            }
        };

        if !component_paths.contains(&comp_path) {
            component_paths.push(comp_path);
        }

        // Track config file (config normally only applies to UE4SS mods)
        if route.route_type == RouteType::Ue4ss && config_path.is_none() {
            let u_root = PathBuf::from(&ue4ss_component);
            config_path = detect_config_local(&u_root);
        }
    }

    if config_path.is_none() {
        for comp in &component_paths {
            let comp_lower = comp.to_lowercase();
            if comp_lower.contains("swapjson") && comp_lower.ends_with(".json") {
                config_path = Some(comp.clone());
                break;
            }
        }
    }

    if manifest.has_ue4ss {
        primary_path = ue4ss_component.clone();
    } else if manifest.has_palschema {
        primary_path = palschema_component.clone();
    } else if !component_paths.is_empty() {
        primary_path = component_paths[0].clone();
    }

    // 2. Write enabled.txt and mods.txt for UE4SS mods
    if manifest.has_ue4ss {
        let ue4ss_mods_dir = crate::dependency_checker::get_ue4ss_mods_dir(game_path);
        let mut ue4ss_mod_folders: Vec<String> = Vec::new();
        for comp in &component_paths {
            let p = Path::new(comp);
            if p.starts_with(&ue4ss_mods_dir) {
                if let Ok(rel) = p.strip_prefix(&ue4ss_mods_dir) {
                    if let Some(first_seg) = rel.iter().next() {
                        let name = first_seg.to_string_lossy().to_string();
                        if !name.is_empty() && name.to_lowercase() != "palschema" && !ue4ss_mod_folders.iter().any(|f| f.eq_ignore_ascii_case(&name)) {
                            ue4ss_mod_folders.push(name);
                        }
                    }
                }
            }
        }
        if ue4ss_mod_folders.is_empty() {
            ue4ss_mod_folders.push(manifest.folder_name.clone());
        }

        for folder in &ue4ss_mod_folders {
            let u_mod_dir = ue4ss_mods_dir.join(folder);
            if u_mod_dir.exists() {
                let enabled_file = u_mod_dir.join("enabled.txt");
                if force_load_order_ue4ss {
                    if enabled_file.exists() {
                        let _ = fs::remove_file(&enabled_file);
                    }
                } else {
                    if !enabled_file.exists() {
                        let _ = fs::write(&enabled_file, "");
                    }
                }
            }
            let mods_txt = ue4ss_mods_dir.join("mods.txt");
            if mods_txt.exists() {
                if force_load_order_ue4ss {
                    let _ = crate::profiles::update_mods_txt_load_order(&mods_txt, folder, true);
                } else {
                    let _ = crate::profiles::remove_from_mods_txt(&mods_txt, folder);
                }
            }
        }
    }

    // 3. For PalSchema mods: move the extracted folder from PalSchema/mods/ → PalSchema/Storage/
    //    and create an NTFS Junction back in PalSchema/mods/ (with numeric prefix if FLO is active)
    if manifest.has_palschema {
        let ue4ss_mods_dir = crate::dependency_checker::get_ue4ss_mods_dir(game_path);
        let palschema_mods_dir = ue4ss_mods_dir.join("PalSchema").join("mods");
        let extracted_mod_dir = palschema_mods_dir.join(&manifest.folder_name);

        if force_load_order_palschema {
            let palschema_storage_dir = ue4ss_mods_dir.join("PalSchema").join("Storage");
            if extracted_mod_dir.exists() {
                let storage_dest = palschema_storage_dir.join(&manifest.folder_name);
                let _ = fs::create_dir_all(&palschema_storage_dir);

                // Move extracted dir → Storage
                move_path(&extracted_mod_dir, &storage_dest)
                    .unwrap_or_else(|e| crate::logger::log(&format!("PalSchema Storage move failed: {}", e)));

                // Count existing numbered entries to assign the next available slot
                let next_order = palschema_mods_dir
                    .read_dir()
                    .map(|rd| {
                        rd.flatten()
                            .filter(|e| {
                                let n = e.file_name().to_string_lossy().to_string();
                                n.len() > 4 && n[..3].chars().all(|c| c.is_ascii_digit()) && n.as_bytes()[3] == b'_'
                            })
                            .count() as u32
                            + 1
                    })
                    .unwrap_or(1);
                let link_name = format!("{:03}_{}", next_order, &manifest.folder_name);
                let link_path = palschema_mods_dir.join(&link_name);
                if let Err(e) = crate::profiles::create_junction_or_symlink(&storage_dest, &link_path) {
                    crate::logger::log(&format!("PalSchema junction creation failed: {}", e));
                }

                let junction_str = normalize_path_separator(&link_path.to_string_lossy());
                if !manifest.has_ue4ss {
                    primary_path = junction_str.clone();
                }
                if let Some(pos) = component_paths.iter().position(|p| p == &palschema_component || p == &extracted_mod_dir.to_string_lossy()) {
                    component_paths[pos] = junction_str;
                } else if !component_paths.contains(&junction_str) {
                    component_paths.push(junction_str);
                }
            }
        } else {
            // Normal installation without symlinks / Storage
            if extracted_mod_dir.exists() {
                let normal_str = normalize_path_separator(&extracted_mod_dir.to_string_lossy());
                if !manifest.has_ue4ss {
                    primary_path = normal_str.clone();
                }
                if !component_paths.contains(&normal_str) {
                    component_paths.push(normal_str);
                }
            }
        }
    }

    let has_enabled_txt = manifest.has_ue4ss;

    // Separate primary path from extra files
    let mut extra_files = Vec::new();
    for file in component_paths {
        if file != primary_path {
            extra_files.push(file);
        }
    }

    let pak_destination = if manifest.mod_type == ModType::LogicMods {
        Some("LogicMods".to_string())
    } else if manifest.mod_type == ModType::Pak {
        Some("~mods".to_string())
    } else {
        None
    };

    Ok(ModInfo {
        id: determine_mod_id(manifest.nexus_mod_id, manifest.nexus_file_id.as_deref(), &manifest.folder_name, &manifest.mod_type),
        name: manifest.display_name.clone(),
        mod_type: manifest.mod_type.clone(),
        nexus_mod_id: manifest.nexus_mod_id,
        nexus_url: manifest.nexus_mod_id.map(|id| format!("https://www.nexusmods.com/palworld/mods/{}", id)),
        nexus_author,
        nexus_summary,
        nexus_picture_url,
        nexus_endorsements,
        nexus_downloads,
        version: manifest.version.clone(),
        install_date: now.to_string(),
        source_zip: String::new(),
        config_path,
        config_type: Some("auto".to_string()),
        enabled: true,
        game_path: primary_path,
        disabled_path: String::new(),
        pak_destination,
        has_enabled_txt,
        mods_txt_order: None,
        extra_files,
        nexus_description: None,
        nexus_version_cached: None,
        nexus_cached_at: None,
        nexus_category: None,
        nexus_tags: Vec::new(),
        github_repo: None,
        github_version: None,
        github_cached_at: None,
        update_date: None,
        library_zip: None,
        ignored_version: None,
        nexus_file_id: manifest.nexus_file_id.clone(),
        ignored_keys: None,
        has_pending_update: None,
        origin_load_method: if manifest.mod_type == ModType::Ue4ss { Some("enabled_txt".to_string()) } else { None },
        custom_notes: None,
        original_name: Some(manifest.display_name.clone()),
        custom_name: None,
    })
}
