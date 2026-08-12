use std::fs;
use std::path::{Path, PathBuf};
use crate::models::Profile;
use super::utils::{copy_dir_all, find_extracted_root};

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
        let cached_zip = PathBuf::from(program_path)
            .join("mods-library")
            .join("dependencies")
            .join("ue4ss.zip");
        if cached_zip.exists() && (!dwmapi.exists() || !ue4ss_dir.exists()) {
            crate::logger::log("sync_profile_dependencies: Installing UE4SS from local library (offline)...");
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
                if let Ok(ver) = fs::read_to_string(PathBuf::from(program_path).join("mods-library").join("dependencies").join("ue4ss.version")) {
                    let _ = fs::write(ue4ss_dir.join("ue4ss.version"), ver.trim());
                }
            }
            let _ = fs::remove_dir_all(&temp_dir);
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
    if target_profile.palschema_enabled {
        let cached_zip = PathBuf::from(program_path)
            .join("mods-library")
            .join("dependencies")
            .join("palschema.zip");
        if cached_zip.exists() && !palschema_dir.exists() {
            crate::logger::log("sync_profile_dependencies: Installing PalSchema from local library (offline)...");
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
                if let Ok(ver) = fs::read_to_string(PathBuf::from(program_path).join("mods-library").join("dependencies").join("palschema.version")) {
                    let _ = fs::write(palschema_dir.join("palschema.version"), ver.trim());
                }
            }
            let _ = fs::remove_dir_all(&temp_dir);
        }
    } else {
        if palschema_dir.exists() {
            let _ = fs::remove_dir_all(palschema_dir);
        }
    }

    Ok(())
}
