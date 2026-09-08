// Pak file inspection commands (tree, asset, uasset deep, texture decode)
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn inspect_pak_file_tree(
    pak_path: String,
    zip_path: Option<String>,
) -> Result<crate::pak_scanner::PakInspectionResult, String> {
    let p = Path::new(&pak_path);
    if p.exists() {
        return crate::pak_scanner::list_pak_entries_detailed(p);
    }

    // If a parent zip_path is provided and the pak is inside an archive (installation preview)
    if let Some(zp) = zip_path {
        let zip_p = Path::new(&zp);
        if zip_p.exists() {
            let temp_dir = std::env::temp_dir().join("pmm_pak_inspect").join(uuid::Uuid::new_v4().to_string());
            fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

            let result = (|| {
                let target_name = Path::new(&pak_path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .unwrap_or_else(|| pak_path.to_lowercase());

                let lower = zp.to_lowercase();
                if lower.ends_with(".zip") {
                    let file = fs::File::open(&zip_p).map_err(|e| e.to_string())?;
                    let mut archive = zip::read::ZipArchive::new(file).map_err(|e| e.to_string())?;
                    
                    let mut found = false;
                    for i in 0..archive.len() {
                        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
                        let ename = entry.name().replace('\\', "/");
                        let ename_lower = ename.to_lowercase();
                        let entry_fname = Path::new(&ename)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_lowercase())
                            .unwrap_or_default();

                        if entry_fname == target_name
                            || ename_lower.ends_with(&format!("/{}", target_name))
                            || ename.eq_ignore_ascii_case(&pak_path)
                        {
                            let temp_pak = temp_dir.join("temp_inspect.pak");
                            let mut out = fs::File::create(&temp_pak).map_err(|e| e.to_string())?;
                            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(format!("Pak file '{}' not found inside archive", pak_path));
                    }
                } else if lower.ends_with(".7z") {
                    // Extract with sevenz
                    let reader = sevenz_rust::SevenZReader::open(&zip_p, sevenz_rust::Password::empty())
                        .map_err(|e| e.to_string())?;
                    let mut found = false;
                    for entry in reader.archive().files.iter() {
                        let ename = entry.name().replace('\\', "/");
                        let ename_lower = ename.to_lowercase();
                        let entry_fname = Path::new(&ename)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_lowercase())
                            .unwrap_or_default();

                        if entry_fname == target_name
                            || ename_lower.ends_with(&format!("/{}", target_name))
                            || ename.eq_ignore_ascii_case(&pak_path)
                        {
                            sevenz_rust::decompress_file(&zip_p, &temp_dir).map_err(|e| e.to_string())?;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(format!("Pak file '{}' not found in 7z", pak_path));
                    }
                } else {
                    // .rar or other supported archive formats
                    crate::zip_handler::extract_zip_to_temp(&zp, &temp_dir).map_err(|e| e.to_string())?;
                }

                let temp_pak = temp_dir.join("temp_inspect.pak");
                if temp_pak.exists() {
                    crate::pak_scanner::list_pak_entries_detailed(&temp_pak)
                } else {
                    // Search recursively in temp_dir for extracted pak
                    let mut found_pak: Option<PathBuf> = None;
                    for entry in walkdir::WalkDir::new(&temp_dir).into_iter().flatten() {
                        if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                            let fname = entry.file_name().to_string_lossy().to_lowercase();
                            if fname == target_name {
                                found_pak = Some(entry.path().to_path_buf());
                                break;
                            } else if found_pak.is_none() {
                                found_pak = Some(entry.path().to_path_buf());
                            }
                        }
                    }
                    if let Some(target) = found_pak {
                        crate::pak_scanner::list_pak_entries_detailed(&target)
                    } else {
                        Err(format!("Could not extract pak '{}'", pak_path))
                    }
                }
            })();

            let _ = fs::remove_dir_all(&temp_dir);
            return result;
        }
    }

    Err(format!("Pak file not found at '{}'", pak_path))
}

pub fn gather_candidate_paks(
    target_mod: &crate::models::ModInfo,
    game_path: &str,
) -> Vec<PathBuf> {
    let game_dir = Path::new(game_path);
    let mut candidate_paks = Vec::new();

    let mut add_if_pak = |path_str: &str| {
        if path_str.to_lowercase().ends_with(".pak") {
            let resolved = crate::config_merge::resolve_path_in_game(game_dir, path_str);
            if resolved.exists() && !candidate_paks.contains(&resolved) {
                candidate_paks.push(resolved);
            }
        }
    };

    add_if_pak(&target_mod.game_path);
    add_if_pak(&target_mod.disabled_path);
    for extra in &target_mod.extra_files {
        add_if_pak(extra);
    }

    candidate_paks
}

#[tauri::command]
pub fn inspect_mod_pak_contents(
    state: State<'_, AppState>,
    mod_id: String,
) -> Result<Vec<crate::pak_scanner::PakInspectionResult>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    let target_mod = profile_mods.into_iter().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;

    let candidate_paks = gather_candidate_paks(&target_mod, &data.settings.game_path);

    let mut results = Vec::new();
    for pak in candidate_paks {
        match crate::pak_scanner::list_pak_entries_detailed(&pak) {
            Ok(info) => results.push(info),
            Err(e) => {
                crate::logger::log(&format!("inspect_mod_pak_contents: Error reading pak '{:?}': {}", pak, e));
            }
        }
    }

    if results.is_empty() {
        return Err("No valid .pak files found for this mod on disk".to_string());
    }

    Ok(results)
}

#[tauri::command]
pub fn inspect_pak_asset(
    state: State<'_, AppState>,
    mod_id: String,
    asset_internal_path: String,
) -> Result<Vec<String>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    let target_mod = profile_mods.into_iter().find(|m| m.id == mod_id)
        .ok_or_else(|| "Mod not found".to_string())?;

    let candidate_paks = gather_candidate_paks(&target_mod, &data.settings.game_path);

    for pak in candidate_paks {
        if let Ok(entries) = crate::pak_scanner::list_pak_entries(&pak) {
            if entries.iter().any(|e| e.eq_ignore_ascii_case(&asset_internal_path)) {
                return crate::pak_scanner::list_pak_entries(&pak);
            }
        }
    }

    Err(format!("Asset '{}' not found in mod pak archives", asset_internal_path))
}

#[tauri::command]
pub fn inspect_uasset_deep_cmd(
    state: State<'_, AppState>,
    mod_id: Option<String>,
    pak_path: Option<String>,
    asset_internal_path: String,
    zip_path: Option<String>,
) -> Result<crate::pak_scanner::UAssetInspectionDetails, String> {
    // 0. Direct loose file on disk if path exists
    let direct_p = Path::new(&asset_internal_path);
    if direct_p.is_file() && direct_p.extension().map_or(false, |e| e.eq_ignore_ascii_case("uasset")) {
        return crate::pak_scanner::inspect_uasset_deep(direct_p, &asset_internal_path);
    }

    // 1. Direct pak_path if provided
    if let Some(pp) = pak_path.as_deref() {
        let clean_pp = pp.strip_prefix("[Pak] ").unwrap_or(pp).trim();
        let p = Path::new(clean_pp);
        if p.exists() {
            return crate::pak_scanner::inspect_uasset_deep(p, &asset_internal_path);
        }
        if let Ok(data) = state.data.lock() {
            let game_dir = Path::new(&data.settings.game_path);
            let in_mods = game_dir.join("Pal").join("Content").join("Paks").join("~mods").join(clean_pp);
            if in_mods.exists() {
                return crate::pak_scanner::inspect_uasset_deep(&in_mods, &asset_internal_path);
            }
        }
    }

    // 2. Mod ID search
    if let Some(mid) = mod_id.as_deref() {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
        if let Some(target_mod) = profile_mods.into_iter().find(|m| m.id == mid) {
            // Check if asset is a loose file inside the mod structure
            if let Ok(full_path) = crate::commands::config_commands::get_full_mod_file_path(&target_mod, &asset_internal_path) {
                if full_path.is_file() && full_path.extension().map_or(false, |e| e.eq_ignore_ascii_case("uasset")) {
                    return crate::pak_scanner::inspect_uasset_deep(&full_path, &asset_internal_path);
                }
            }

            let candidate_paks = gather_candidate_paks(&target_mod, &data.settings.game_path);

            for pak in candidate_paks {
                if let Ok(entries) = crate::pak_scanner::list_pak_entries(&pak) {
                    if entries.iter().any(|e| e.eq_ignore_ascii_case(&asset_internal_path)) {
                        return crate::pak_scanner::inspect_uasset_deep(&pak, &asset_internal_path);
                    }
                }
            }
        }
    }

    // 3. Zip preview path (Installer modal)
    if let Some(zp) = zip_path {
        let zip_p = Path::new(&zp);
        if zip_p.exists() {
            let temp_dir = std::env::temp_dir().join("pmm_uasset_inspect").join(uuid::Uuid::new_v4().to_string());
            fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

            let result = (|| {
                let lower = zp.to_lowercase();
                if lower.ends_with(".zip") {
                    let file = fs::File::open(&zip_p).map_err(|e| e.to_string())?;
                    let mut archive = zip::read::ZipArchive::new(file).map_err(|e| e.to_string())?;
                    for i in 0..archive.len() {
                        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
                        let ename = entry.name().replace('\\', "/");
                        if ename.to_lowercase().ends_with(".pak") {
                            let temp_pak = temp_dir.join("temp.pak");
                            let mut out = fs::File::create(&temp_pak).map_err(|e| e.to_string())?;
                            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                            if let Ok(details) = crate::pak_scanner::inspect_uasset_deep(&temp_pak, &asset_internal_path) {
                                return Ok(details);
                            }
                        }
                    }
                } else if lower.ends_with(".7z") {
                    if sevenz_rust::decompress_file(&zip_p, &temp_dir).is_ok() {
                        for entry in walkdir::WalkDir::new(&temp_dir).into_iter().flatten() {
                            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                                if let Ok(details) = crate::pak_scanner::inspect_uasset_deep(entry.path(), &asset_internal_path) {
                                    return Ok(details);
                                }
                            }
                        }
                    }
                } else {
                    if crate::zip_handler::extract_zip_to_temp(&zp, &temp_dir).is_ok() {
                        for entry in walkdir::WalkDir::new(&temp_dir).into_iter().flatten() {
                            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                                if let Ok(details) = crate::pak_scanner::inspect_uasset_deep(entry.path(), &asset_internal_path) {
                                    return Ok(details);
                                }
                            }
                        }
                    }
                }
                Err(format!("Asset '{}' not found inside archive pak files", asset_internal_path))
            })();

            let _ = fs::remove_dir_all(&temp_dir);
            return result;
        }
    }

    Err(format!("Could not locate pak containing asset '{}'", asset_internal_path))
}

#[tauri::command]
pub fn decode_uasset_texture_cmd(
    state: State<'_, AppState>,
    mod_id: Option<String>,
    pak_path: Option<String>,
    asset_internal_path: String,
    zip_path: Option<String>,
) -> Result<crate::texture_decoder::TexturePreviewInfo, String> {
    let target_base = {
        let l = asset_internal_path.to_lowercase();
        if l.ends_with(".uasset") {
            l[..l.len() - 7].to_string()
        } else if l.ends_with(".ubulk") || l.ends_with(".uptnl") {
            l[..l.len() - 6].to_string()
        } else if l.ends_with(".uexp") {
            l[..l.len() - 5].to_string()
        } else {
            l
        }
    };

    // 1. Direct pak_path if provided
    if let Some(pp) = pak_path.as_deref() {
        let clean_pp = pp.strip_prefix("[Pak] ").unwrap_or(pp).trim();
        let p = Path::new(clean_pp);
        if p.exists() {
            let names = crate::pak_scanner::list_pak_entries(p).unwrap_or_default();
            return crate::texture_decoder::extract_and_decode_texture(p, &asset_internal_path, &names);
        }
        if let Ok(data) = state.data.lock() {
            let game_dir = Path::new(&data.settings.game_path);
            let in_mods = game_dir.join("Pal").join("Content").join("Paks").join("~mods").join(clean_pp);
            if in_mods.exists() {
                let names = crate::pak_scanner::list_pak_entries(&in_mods).unwrap_or_default();
                return crate::texture_decoder::extract_and_decode_texture(&in_mods, &asset_internal_path, &names);
            }
        }
    }

    // 2. Mod ID search
    if let Some(mid) = mod_id.as_deref() {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
        if let Some(target_mod) = profile_mods.into_iter().find(|m| m.id == mid) {
            let mut candidate_paks = Vec::new();
            if target_mod.game_path.to_lowercase().ends_with(".pak") {
                candidate_paks.push(PathBuf::from(&target_mod.game_path));
            }
            for extra in &target_mod.extra_files {
                if extra.to_lowercase().ends_with(".pak") {
                    candidate_paks.push(PathBuf::from(extra));
                }
            }

            for pak in candidate_paks {
                if pak.exists() {
                    if let Ok(entries) = crate::pak_scanner::list_pak_entries(&pak) {
                        if entries.iter().any(|e| {
                            let el = e.to_lowercase();
                            el == asset_internal_path.to_lowercase() || el.starts_with(&target_base)
                        }) {
                            return crate::texture_decoder::extract_and_decode_texture(&pak, &asset_internal_path, &entries);
                        }
                    }
                }
            }
        }
    }

    // 3. Zip preview path (Installer modal)
    if let Some(zp) = zip_path {
        let zip_p = Path::new(&zp);
        if zip_p.exists() {
            let temp_dir = std::env::temp_dir().join("pmm_tex_decode").join(uuid::Uuid::new_v4().to_string());
            fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

            let result = (|| {
                let lower = zp.to_lowercase();
                if lower.ends_with(".zip") {
                    let file = fs::File::open(&zip_p).map_err(|e| e.to_string())?;
                    let mut archive = zip::read::ZipArchive::new(file).map_err(|e| e.to_string())?;
                    for i in 0..archive.len() {
                        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
                        let ename = entry.name().replace('\\', "/");
                        if ename.to_lowercase().ends_with(".pak") {
                            let temp_pak = temp_dir.join("temp.pak");
                            let mut out = fs::File::create(&temp_pak).map_err(|e| e.to_string())?;
                            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                            let names = crate::pak_scanner::list_pak_entries(&temp_pak).unwrap_or_default();
                            if names.iter().any(|e| e.to_lowercase().starts_with(&target_base)) {
                                if let Ok(tex) = crate::texture_decoder::extract_and_decode_texture(&temp_pak, &asset_internal_path, &names) {
                                    return Ok(tex);
                                }
                            }
                        }
                    }
                } else if lower.ends_with(".7z") {
                    if sevenz_rust::decompress_file(&zip_p, &temp_dir).is_ok() {
                        for entry in walkdir::WalkDir::new(&temp_dir).into_iter().flatten() {
                            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                                let names = crate::pak_scanner::list_pak_entries(entry.path()).unwrap_or_default();
                                if names.iter().any(|e| e.to_lowercase().starts_with(&target_base)) {
                                    if let Ok(tex) = crate::texture_decoder::extract_and_decode_texture(entry.path(), &asset_internal_path, &names) {
                                        return Ok(tex);
                                    }
                                }
                            }
                        }
                    }
                }
                Err("Texture not found in package".to_string())
            })();
            let _ = fs::remove_dir_all(&temp_dir);
            return result;
        }
    }

    Err(format!("Texture asset '{}' not found in mod pak archives", asset_internal_path))
}
