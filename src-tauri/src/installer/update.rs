use std::fs;
use std::path::{Path, PathBuf};
use crate::models::{ModInfo, ModType};
use crate::zip_handler::ZipAnalysis;
use super::execution::execute_manifest;
use super::helpers::move_path;

pub fn update_mod(
    existing: &mut ModInfo,
    game_path: &str,
    program_path: &str,
    current_profile_id: &str,
    extracted: &Path,
    analysis: &ZipAnalysis,
    zip_filename: &str,
    now: &str,
    force_load_order_ue4ss: bool,
    force_load_order_palschema: bool,
) -> Result<(), String> {
    let delete_path_and_sidecar = |path_str: &str| {
        if path_str.is_empty() {
            return;
        }
        let p = Path::new(path_str);
        if p.exists() {
            if p.is_dir() {
                let _ = fs::remove_dir_all(p);
            } else {
                let _ = fs::remove_file(p);
                let sidecar = PathBuf::from(format!("{}.pmm.json", path_str));
                if sidecar.exists() {
                    let _ = fs::remove_file(sidecar);
                }
            }
        } else {
            let sidecar = PathBuf::from(format!("{}.pmm.json", path_str));
            if sidecar.exists() {
                let _ = fs::remove_file(sidecar);
            }
        }
    };

    let dest_deduced = if let Some(ref d) = existing.pak_destination {
        if !d.is_empty() {
            Some(d.clone())
        } else {
            None
        }
    } else {
        None
    }.or_else(|| {
        if existing.game_path.to_lowercase().contains("logicmods") || existing.disabled_path.to_lowercase().contains("logicmods") {
            Some("LogicMods".to_string())
        } else {
            None
        }
    });

    let game = Path::new(game_path);
    let mut all_existing_dirs = Vec::new();
    if !existing.game_path.is_empty() {
        let p = crate::config_merge::resolve_path_in_game(game, &existing.game_path);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            all_existing_dirs.push(r);
        }
    }
    if !existing.disabled_path.is_empty() {
        let p = crate::config_merge::resolve_path_in_game(game, &existing.disabled_path);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            if !all_existing_dirs.contains(&r) {
                all_existing_dirs.push(r);
            }
        }
    }
    for extra in &existing.extra_files {
        let p = crate::config_merge::resolve_path_in_game(game, extra);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            if !all_existing_dirs.contains(&r) {
                all_existing_dirs.push(r);
            }
        }
    }

    let mut snapshot = crate::config_merge::ConfigSnapshot { entries: Vec::new() };
    for dir in &all_existing_dirs {
        let s = crate::config_merge::snapshot_configs(dir, existing.config_path.as_deref());
        for entry in s.entries {
            if !snapshot.entries.iter().any(|(rel, _)| rel == &entry.0) {
                snapshot.entries.push(entry);
            }
        }
    }

    if let Some(ref custom_str) = existing.config_path {
        let cp = crate::config_merge::resolve_path_in_game(game, custom_str);
        if cp.exists() && cp.is_file() {
            if let Ok(content) = fs::read_to_string(&cp) {
                let filename = cp.file_name().unwrap_or_default();
                let rel = PathBuf::from(filename);
                if !snapshot.entries.iter().any(|(r, _)| r == &rel) {
                    snapshot.entries.push((rel, content));
                }
            }
        }
    }

    let old_game_path = existing.game_path.clone();
    let old_disabled_path = existing.disabled_path.clone();
    let old_extras = existing.extra_files.clone();
    let was_enabled = existing.enabled;

    let game = Path::new(game_path);
    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            let full_path = extracted.join(target_file);
            if full_path.exists() {
                if let Ok(content) = std::fs::read_to_string(full_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        modinfo_data = Some(val);
                    }
                }
            }
        }
    }

    let manifest = crate::zip_handler::build_manifest_from_files(
        &analysis.files,
        zip_filename,
        game,
        dest_deduced.as_deref(),
        Some(existing.name.clone()),
        modinfo_data,
    )?;

    let new_mod_info = execute_manifest(
        &manifest,
        extracted,
        game,
        existing.nexus_author.clone(),
        existing.nexus_summary.clone(),
        existing.nexus_picture_url.clone(),
        existing.nexus_downloads,
        existing.nexus_endorsements,
        now,
        force_load_order_ue4ss,
        force_load_order_palschema,
    )?;

    // Clean up old paths that differ from newly installed paths
    if !old_game_path.is_empty() && old_game_path != new_mod_info.game_path {
        delete_path_and_sidecar(&old_game_path);
    }
    if !old_disabled_path.is_empty() && old_disabled_path != new_mod_info.disabled_path {
        delete_path_and_sidecar(&old_disabled_path);
    }
    for extra in &old_extras {
        if !new_mod_info.extra_files.contains(extra) && extra != &new_mod_info.game_path {
            delete_path_and_sidecar(extra);
        }
    }

    existing.name = new_mod_info.name;
    existing.mod_type = new_mod_info.mod_type;
    existing.version = new_mod_info.version;
    existing.source_zip = new_mod_info.source_zip;
    if !(existing.config_type.as_deref() == Some("manual") && existing.config_path.is_some()) {
        existing.config_path = new_mod_info.config_path;
        existing.config_type = new_mod_info.config_type;
    }
    existing.game_path = new_mod_info.game_path;
    existing.disabled_path = new_mod_info.disabled_path;
    existing.pak_destination = new_mod_info.pak_destination;
    existing.has_enabled_txt = new_mod_info.has_enabled_txt;
    existing.extra_files = new_mod_info.extra_files;
    existing.update_date = Some(now.to_string());
    existing.enabled = true;
    if existing.origin_load_method.is_none() {
        existing.origin_load_method = new_mod_info.origin_load_method;
    }

    let mut all_dest_dirs = Vec::new();
    if !existing.game_path.is_empty() {
        let p = crate::config_merge::resolve_path_in_game(game, &existing.game_path);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            all_dest_dirs.push(r);
        }
    }
    if !existing.disabled_path.is_empty() {
        let p = crate::config_merge::resolve_path_in_game(game, &existing.disabled_path);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            if !all_dest_dirs.contains(&r) {
                all_dest_dirs.push(r);
            }
        }
    }
    for extra in &existing.extra_files {
        let p = crate::config_merge::resolve_path_in_game(game, extra);
        if p.exists() {
            let r = if p.is_dir() { p } else { p.parent().unwrap_or(&p).to_path_buf() };
            if !all_dest_dirs.contains(&r) {
                all_dest_dirs.push(r);
            }
        }
    }
    let ignored = existing.ignored_keys.clone().unwrap_or_default();
    for dest in &all_dest_dirs {
        crate::config_merge::apply_config_merge(dest, &snapshot, &ignored);
    }

    if !was_enabled {
        let profile_dir = PathBuf::from(program_path).join("profiles").join(current_profile_id);
        let disabled_base = profile_dir.join("disabled_mods");

        if existing.mod_type == ModType::Ue4ss {
            let src_path = PathBuf::from(&existing.game_path);
            if src_path.exists() {
                if let Some(parent) = src_path.parent() {
                    let mods_txt = parent.join("mods.txt");
                    if mods_txt.exists() {
                        let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &existing.name);
                        if let Some(f_name) = src_path.file_name() {
                            let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &f_name.to_string_lossy());
                        }
                    }
                }
                let enabled_file = src_path.join("enabled.txt");
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
                let file_name = src_path.file_name().unwrap().to_string_lossy().to_string();
                let dest = disabled_base.join("ue4ss").join(&file_name);
                move_path(&src_path, &dest)?;
                existing.disabled_path = dest.to_string_lossy().to_string();
                existing.game_path = String::new();
            }
            existing.enabled = false;
        } else if existing.mod_type == ModType::PalSchema {
            let src_path = PathBuf::from(&existing.game_path);
            let folder_name = crate::profiles::get_mod_folder_name(existing);
            let gp = crate::dependency_checker::build_game_profile(Path::new(game_path));
            let palschema_mods_dir = gp.palschema_mods_dir.clone();
            let palschema_storage_dir = gp.palschema_storage_dir.clone();

            if palschema_mods_dir.exists() {
                if let Ok(entries) = fs::read_dir(&palschema_mods_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let name = path.file_name().unwrap().to_string_lossy().to_string();
                        let clean_name = if name.len() > 4 && name[..3].chars().all(|c| c.is_ascii_digit()) && name.as_bytes()[3] == b'_' {
                            &name[4..]
                        } else {
                            &name
                        };
                        if clean_name.to_lowercase() == folder_name.to_lowercase() {
                            let _ = crate::profiles::remove_junction_or_symlink(&path);
                        }
                    }
                }
            }

            let storage_path = palschema_storage_dir.join(&folder_name);
            let final_src = if storage_path.exists() {
                storage_path
            } else if src_path.exists() {
                src_path
            } else {
                palschema_mods_dir.join(&folder_name)
            };

            if final_src.exists() {
                let dest_dir = disabled_base.join("palschema");
                let _ = fs::create_dir_all(&dest_dir);
                let dest = dest_dir.join(&folder_name);
                move_path(&final_src, &dest)?;
                existing.disabled_path = dest.to_string_lossy().to_string();
                existing.game_path = String::new();
            }
            existing.enabled = false;
        } else if existing.mod_type == ModType::Pak || existing.mod_type == ModType::LogicMods {
            let src_path = PathBuf::from(&existing.game_path);
            if src_path.exists() {
                let mut moved_files = Vec::new();
                if let Some(parent) = src_path.parent() {
                    let file_stem = src_path.file_stem().unwrap().to_string_lossy().to_string();
                    let type_dir = if existing.mod_type == ModType::LogicMods { "logicmods" } else { "pak" };
                    let dest_dir = disabled_base.join(type_dir);
                    let _ = fs::create_dir_all(&dest_dir);

                    for ext in &["pak", "ucas", "utoc"] {
                        let companion = parent.join(format!("{}.{}", file_stem, ext));
                        if companion.exists() {
                            let dest = dest_dir.join(format!("{}.{}", file_stem, ext));
                            move_path(&companion, &dest)?;
                            moved_files.push(dest.to_string_lossy().to_string());
                        }
                    }
                    let sidecar = parent.join(format!("{}.pak.pmm.json", file_stem));
                    if sidecar.exists() {
                        let dest = dest_dir.join(format!("{}.pak.pmm.json", file_stem));
                        let _ = move_path(&sidecar, &dest);
                    }
                }
                existing.disabled_path = moved_files.first().cloned().unwrap_or_default();
                existing.extra_files = moved_files.into_iter().skip(1).collect();
                existing.game_path = String::new();
            }
            existing.enabled = false;
        } else if existing.mod_type == ModType::Hybrid {
            let mut moved_extras = Vec::new();
            for extra in &existing.extra_files {
                let extra_path = PathBuf::from(extra);
                if extra_path.exists() {
                    let file_name = extra_path.file_name().unwrap().to_string_lossy().to_string();
                    let dest_dir = disabled_base.join("hybrid").join("extras");
                    let _ = fs::create_dir_all(&dest_dir);
                    
                    if extra_path.is_dir() {
                        let dest = dest_dir.join(&file_name);
                        move_path(&extra_path, &dest)?;
                        moved_extras.push(dest.to_string_lossy().to_string());
                    } else {
                        let parent = extra_path.parent().unwrap();
                        let stem = extra_path.file_stem().unwrap().to_string_lossy().to_string();
                        let dest = dest_dir.join(&file_name);
                        move_path(&extra_path, &dest)?;
                        moved_extras.push(dest.to_string_lossy().to_string());
                        
                        for c_ext in &["ucas", "utoc"] {
                            let companion = parent.join(format!("{}.{}", stem, c_ext));
                            if companion.exists() {
                                let c_dest = dest_dir.join(format!("{}.{}", stem, c_ext));
                                let _ = move_path(&companion, &c_dest);
                            }
                        }
                        if !file_name.ends_with(".pmm.json") {
                            let sidecar = parent.join(format!("{}.pmm.json", file_name));
                            if sidecar.exists() {
                                let c_dest = dest_dir.join(format!("{}.pmm.json", file_name));
                                let _ = move_path(&sidecar, &c_dest);
                            }
                        }
                    }
                }
            }

            let src_path = PathBuf::from(&existing.game_path);
            if src_path.exists() {
                if let Some(parent) = src_path.parent() {
                    let mods_txt = parent.join("mods.txt");
                    if mods_txt.exists() {
                        let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &existing.name);
                        if let Some(f_name) = src_path.file_name() {
                            let _ = crate::profiles::remove_from_mods_txt(&mods_txt, &f_name.to_string_lossy());
                        }
                    }
                }
                let enabled_file = src_path.join("enabled.txt");
                if enabled_file.exists() {
                    let _ = fs::remove_file(&enabled_file);
                }
                
                let file_name = src_path.file_name().unwrap().to_string_lossy().to_string();
                let dest_dir = disabled_base.join("hybrid");
                let _ = fs::create_dir_all(&dest_dir);
                let dest = dest_dir.join(&file_name);
                move_path(&src_path, &dest)?;
                
                existing.disabled_path = dest.to_string_lossy().to_string();
                existing.game_path = String::new();
            }
            existing.extra_files = moved_extras;
            existing.enabled = false;
        }
    }

    Ok(())
}
