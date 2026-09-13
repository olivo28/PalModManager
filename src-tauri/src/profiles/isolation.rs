use std::fs;
use std::path::{Path, PathBuf};
use crate::models::Profile;
use super::utils::{copy_dir_all, find_extracted_root};

fn find_vault_archive(
    program_path: &str,
    dep_type: &str,
    preferred_version: Option<&str>,
) -> Option<(PathBuf, String)> {
    let vault_dir = crate::commands::dependency::get_vault_dir(program_path, dep_type);
    if vault_dir.exists() {
        if let Ok(entries) = fs::read_dir(&vault_dir) {
            let mut candidates: Vec<(PathBuf, String, std::time::SystemTime)> = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e.eq_ignore_ascii_case("zip")).unwrap_or(false) {
                    let fname = path.file_name().unwrap().to_string_lossy().to_string();
                    let ver = crate::commands::dependency::extract_version_from_vault_filename(&fname, dep_type);
                    let mtime = entry.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    candidates.push((path, ver, mtime));
                }
            }

            if let Some(target) = preferred_version {
                let clean_target = target.trim();
                if !clean_target.is_empty() && clean_target != "Installed" && clean_target != "None" {
                    if let Some(matched) = candidates.iter().find(|(_, ver, _)| ver.eq_ignore_ascii_case(clean_target)) {
                        return Some((matched.0.clone(), matched.1.clone()));
                    }
                }
            }

            // Fallback to most recently modified in vault
            if let Some(latest) = candidates.into_iter().max_by_key(|(_, _, mtime)| *mtime) {
                return Some((latest.0, latest.1));
            }
        }
    }

    // Legacy fallback at root of dependencies
    let legacy_base = PathBuf::from(program_path).join("mods-library").join("dependencies");
    let legacy_zip = legacy_base.join(format!("{}.zip", dep_type.to_lowercase()));
    if legacy_zip.exists() {
        let ver = fs::read_to_string(legacy_base.join(format!("{}.version", dep_type.to_lowercase())))
            .unwrap_or_default()
            .trim()
            .to_string();
        return Some((legacy_zip, ver));
    }

    None
}

pub fn sync_profile_dependencies(
    game_path: &str,
    program_path: &str,
    target_profile: &Profile,
) -> Result<(), String> {
    if game_path.is_empty() {
        return Ok(());
    }
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));

    // UE4SS
    let dwmapi = win64.join("dwmapi.dll");
    let ue4ss_dir = win64.join("ue4ss");
    if target_profile.ue4ss_enabled {
        if !dwmapi.exists() || !ue4ss_dir.exists() {
            if let Some((cached_zip, ver)) = find_vault_archive(program_path, "ue4ss", target_profile.ue4ss_version.as_deref()) {
                crate::logger::log(&format!("sync_profile_dependencies: Installing UE4SS from vault '{}'...", cached_zip.display()));
                let temp_dir = std::env::temp_dir().join(format!("pmm_sync_ue4ss_{}", uuid::Uuid::new_v4()));
                if let Ok(extracted) = crate::zip_handler::extract_zip_to_temp(&cached_zip.to_string_lossy(), &temp_dir.join("extracted")) {
                    let root = find_extracted_root(&extracted);
                    let (framework_src, dwmapi_src) = {
                        let ue4ss_sub = root.join("ue4ss");
                        if ue4ss_sub.is_dir() {
                            (ue4ss_sub, root.join("dwmapi.dll"))
                        } else {
                            (root.clone(), root.join("dwmapi.dll"))
                        }
                    };
                    if dwmapi_src.exists() {
                        let _ = fs::copy(&dwmapi_src, &win64.join("dwmapi.dll"));
                    }
                    let _ = fs::create_dir_all(&ue4ss_dir);
                    if let Ok(rd) = fs::read_dir(&framework_src) {
                        for entry in rd.filter_map(|e| e.ok()) {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name == "dwmapi.dll" || name == "Mods" || name == "mods" {
                                continue;
                            }
                            let dst = ue4ss_dir.join(&name);
                            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                                let _ = copy_dir_all(&entry.path(), &dst);
                            } else {
                                let _ = fs::copy(&entry.path(), &dst);
                            }
                        }
                    }
                    if !ver.is_empty() {
                        let _ = fs::write(ue4ss_dir.join("ue4ss.version"), &ver);
                    }
                }
                let _ = fs::remove_dir_all(&temp_dir);
            }
        }
    } else {
        if dwmapi.exists() {
            let _ = fs::remove_file(dwmapi);
        }
        if ue4ss_dir.exists() {
            let _ = fs::remove_dir_all(ue4ss_dir);
        }
    }

    // PalSchema
    let palschema_dir = win64.join("ue4ss").join("Mods").join("PalSchema");
    let has_ps_runtime = palschema_dir.join("dlls").join("main.dll").exists()
        || palschema_dir.join("scripts").join("main.lua").exists()
        || palschema_dir.join("main.lua").exists()
        || palschema_dir.join("palschema.version").exists();

    if target_profile.palschema_enabled {
        if !has_ps_runtime {
            if let Some((cached_zip, ver)) = find_vault_archive(program_path, "palschema", target_profile.palschema_version.as_deref()) {
                crate::logger::log(&format!("sync_profile_dependencies: Installing PalSchema from vault '{}'...", cached_zip.display()));
                let temp_dir = std::env::temp_dir().join(format!("pmm_sync_ps_{}", uuid::Uuid::new_v4()));
                if let Ok(extracted) = crate::zip_handler::extract_zip_to_temp(&cached_zip.to_string_lossy(), &temp_dir.join("extracted")) {
                    let root = find_extracted_root(&extracted);
                    let _ = fs::create_dir_all(&palschema_dir);
                    if let Ok(rd) = fs::read_dir(&root) {
                        for entry in rd.filter_map(|e| e.ok()) {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name == "mods" || name == "Mods" {
                                continue;
                            }
                            let dst = palschema_dir.join(&name);
                            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                                let _ = copy_dir_all(&entry.path(), &dst);
                            } else {
                                let _ = fs::copy(&entry.path(), &dst);
                            }
                        }
                    }
                    if !ver.is_empty() {
                        let _ = fs::write(palschema_dir.join("palschema.version"), &ver);
                    }
                }
                let _ = fs::remove_dir_all(&temp_dir);
            }
        }
    } else if palschema_dir.exists() {
        let runtime_files = [
            "enabled.txt", "LICENSE", "palschema.version", "palschema.pmm.json", "palschema.manifest.json"
        ];
        for f in &runtime_files {
            let p = palschema_dir.join(f);
            if p.exists() { let _ = fs::remove_file(p); }
        }
        let _ = fs::remove_dir_all(palschema_dir.join("dlls"));
        let _ = fs::remove_dir_all(palschema_dir.join("scripts"));
        let has_user_mods = palschema_dir.join("mods").exists() && fs::read_dir(palschema_dir.join("mods"))
            .map(|rd| rd.filter_map(|e| e.ok()).count() > 0)
            .unwrap_or(false);
        if !has_user_mods {
            let _ = fs::remove_dir_all(&palschema_dir);
        }
    }

    Ok(())
}
