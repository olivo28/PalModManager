use crate::dependency_checker;
use crate::state::AppState;
use crate::zip_handler;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

fn empty_status() -> dependency_checker::DependencyStatus {
    dependency_checker::DependencyStatus {
        ue4ss_installed: false,
        ue4ss_version: None,
        ue4ss_latest_tag: None,
        ue4ss_latest_date: None,
        ue4ss_needs_update: false,
        palschema_installed: false,
        palschema_version: None,
        palschema_latest_version: None,
        palschema_needs_update: false,
        game_platform: "Unknown".to_string(),
    }
}

fn parse_dmy(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y").ok()
}

#[tauri::command]
pub fn check_dependencies(state: State<AppState>) -> Result<dependency_checker::DependencyStatus, String> {
    let game_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.game_path.clone()
    };
    if game_path.is_empty() {
        return Ok(empty_status());
    }
    Ok(dependency_checker::check_dependencies(&game_path))
}

#[tauri::command]
pub async fn check_ue4ss_latest() -> Result<String, String> {
    // Return the tag name for display
    let (tag, _date) = dependency_checker::check_ue4ss_latest().await?;
    Ok(tag)
}

#[tauri::command]
pub async fn check_palschema_latest() -> Result<String, String> {
    dependency_checker::check_palschema_latest().await
}

fn compare_versions(local: &str, remote: &str) -> bool {
    let local_clean = local.trim_start_matches('v').trim();
    let remote_clean = remote.trim_start_matches('v').trim();

    let local_parts: Vec<&str> = local_clean.split('.').collect();
    let remote_parts: Vec<&str> = remote_clean.split('.').collect();

    let max_len = std::cmp::max(local_parts.len(), remote_parts.len());
    for i in 0..max_len {
        let l = local_parts.get(i).unwrap_or(&"0");
        let r = remote_parts.get(i).unwrap_or(&"0");

        let l_num: u32 = l.parse().unwrap_or(0);
        let r_num: u32 = r.parse().unwrap_or(0);

        if l_num != r_num {
            return false;
        }
    }
    true
}

#[tauri::command]
pub async fn check_dependencies_full(state: State<'_, AppState>) -> Result<dependency_checker::DependencyStatus, String> {
    let game_path = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        locked.settings.game_path.clone()
    };
    if game_path.is_empty() {
        return Ok(empty_status());
    }

    let mut status = dependency_checker::check_dependencies(&game_path);

    if let Ok((ue4ss_tag, ue4ss_date)) = dependency_checker::check_ue4ss_latest().await {
        status.ue4ss_latest_tag = Some(ue4ss_tag);
        status.ue4ss_latest_date = Some(ue4ss_date.clone());
        // Both local and remote are DD.MM.YYYY dates. Compare as dates.
        status.ue4ss_needs_update = match &status.ue4ss_version {
            Some(local) => {
                match (parse_dmy(local.trim()), parse_dmy(ue4ss_date.trim())) {
                    (Some(l), Some(r)) => l < r,
                    // Local version is not a date (old install) → assume needs update
                    _ => true,
                }
            }
            None => true,
        };
    }

    if let Ok(ps_version) = dependency_checker::check_palschema_latest().await {
        status.palschema_latest_version = Some(ps_version.clone());
        status.palschema_needs_update = match &status.palschema_version {
            Some(local) => {
                let eq = compare_versions(local, &ps_version);
                crate::logger::log(&format!("PalSchema check: local='{}', remote='{}', match={}", local, ps_version, eq));
                !eq
            }
            None => {
                crate::logger::log("PalSchema check: local is None (not installed or version not read)");
                true
            }
        };
    }

    Ok(status)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Cannot create dest dir: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Cannot read source dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();
        let dest_path = dst.join(file_name);
        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| {
                format!("Cannot copy file {}: {}", file_name.to_string_lossy(), e)
            })?;
        }
    }
    Ok(())
}

fn find_extracted_root(src: &Path) -> PathBuf {
    if src.is_dir() {
        let entries: Vec<_> = fs::read_dir(src)
            .ok()
            .into_iter()
            .flat_map(|rd| rd.filter_map(|e| e.ok()))
            .filter(|e| {
                let n = e.file_name();
                n != ".." && n != "." && n != "__MACOSX"
            })
            .collect();
        if entries.len() == 1 && entries[0].file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            return entries[0].path();
        }
    }
    src.to_path_buf()
}

#[tauri::command]
pub async fn install_ue4ss(force_download: bool, state: State<'_, AppState>) -> Result<String, String> {
    crate::logger::log("install_ue4ss: Starting UE4SS installation process...");
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() {
        crate::logger::log("install_ue4ss: Error - Game path not configured.");
        return Err("Game path not set".to_string());
    }

    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let ue4ss_dir = win64.join("ue4ss");

    let lib_dep_dir = PathBuf::from(&program_path).join("mods-library").join("dependencies");
    let cached_zip = lib_dep_dir.join("ue4ss.zip");
    let cached_ver_file = lib_dep_dir.join("ue4ss.version");

    let mut publish_date = String::new();
    let mut zip_bytes = Vec::new();
    let mut use_cache = false;

    if !force_download && cached_zip.exists() && cached_ver_file.exists() {
        if let Ok(bytes) = fs::read(&cached_zip) {
            if let Ok(ver) = fs::read_to_string(&cached_ver_file) {
                crate::logger::log("install_ue4ss: Using cached UE4SS zip from local library.");
                zip_bytes = bytes;
                publish_date = ver.trim().to_string();
                use_cache = true;
            }
        }
    }

    if !use_cache {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let mut asset_url = String::new();
        let mut api_success = false;

        let release_url = "https://api.github.com/repos/Okaetsu/RE-UE4SS/releases/tags/experimental-palworld";
        crate::logger::log(&format!("install_ue4ss: Fetching GitHub API release from {}", release_url));
        if let Ok(resp) = client.get(release_url).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(assets) = json["assets"].as_array() {
                        if let Some(asset) = assets.iter().find(|a| {
                            a["name"].as_str().map_or(false, |n| n.ends_with(".zip") && !n.contains("symbols"))
                        }) {
                            if let Some(url) = asset["browser_download_url"].as_str() {
                                asset_url = url.to_string();
                                let mut latest_asset_dt: Option<chrono::DateTime<chrono::FixedOffset>> = None;
                                for a in assets {
                                    if let Some(updated) = a["updated_at"].as_str() {
                                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(updated) {
                                            match latest_asset_dt {
                                                Some(cur) if dt > cur => latest_asset_dt = Some(dt),
                                                None => latest_asset_dt = Some(dt),
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                if let Some(dt) = latest_asset_dt {
                                    publish_date = dt.format("%d.%m.%Y").to_string();
                                }
                                api_success = true;
                            }
                        }
                    }
                }
            }
        }

        if !api_success {
            crate::logger::log("install_ue4ss: GitHub API rate limited or failed. Using HTML fallback...");
            if let Ok(r) = client.get("https://github.com/Okaetsu/RE-UE4SS/releases/tag/experimental-palworld").send().await {
                if let Ok(html) = r.text().await {
                    let mut search_pos = 0;
                    while let Some(pos) = html[search_pos..].find("/Okaetsu/RE-UE4SS/releases/download/experimental-palworld/") {
                        let start = search_pos + pos;
                        if let Some(end_quote) = html[start..].find('"') {
                            let url_path = &html[start..start + end_quote];
                            search_pos = start + end_quote;
                            let lower = url_path.to_lowercase();
                            if lower.ends_with(".zip") && !lower.contains("symbols") {
                                asset_url = format!("https://github.com{}", url_path);
                                if let Some(time_pos) = html.find("datetime=") {
                                    let time_start = time_pos + 10;
                                    if let Some(time_end) = html[time_start..].find('"') {
                                        let dt_raw = &html[time_start..time_start + time_end];
                                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_raw) {
                                            publish_date = dt.format("%d.%m.%Y").to_string();
                                        }
                                    }
                                }
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        if asset_url.is_empty() {
            return Err("Could not resolve UE4SS download URL".to_string());
        }

        if publish_date.is_empty() {
            publish_date = "installed".to_string();
        }

        crate::logger::log(&format!("install_ue4ss: Downloading ZIP from {}", asset_url));
        let bytes = client.get(&asset_url)
            .send()
            .await
            .map_err(|e| format!("Download failed: {}", e))?
            .bytes()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;
        
        zip_bytes = bytes.to_vec();

        let _ = fs::create_dir_all(&lib_dep_dir);
        let _ = fs::write(&cached_zip, &zip_bytes);
        let _ = fs::write(&cached_ver_file, &publish_date);
    }

    let temp_dir = std::env::temp_dir().join("pmm_ue4ss");
    let zip_path = temp_dir.join("ue4ss.zip");
    crate::logger::log(&format!("install_ue4ss: Saving temporary ZIP to {}", zip_path.display()));
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    fs::write(&zip_path, &zip_bytes).map_err(|e| e.to_string())?;

    crate::logger::log("install_ue4ss: Extracting ZIP archive...");
    let extracted = zip_handler::extract_zip_to_temp(&zip_path.to_string_lossy(), &temp_dir.join("extracted"))?;
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
        let dwmapi_dst = win64.join("dwmapi.dll");
        crate::logger::log(&format!("install_ue4ss: Copying dwmapi.dll to {}", dwmapi_dst.display()));
        fs::copy(&dwmapi_src, &dwmapi_dst).map_err(|e| e.to_string())?;
    }

    if !ue4ss_dir.exists() {
        crate::logger::log(&format!("install_ue4ss: Creating target directory ue4ss at {}", ue4ss_dir.display()));
        fs::create_dir_all(&ue4ss_dir).map_err(|e| format!("Cannot create ue4ss directory: {}", e))?;
    }

    crate::logger::log(&format!("install_ue4ss: Copying framework content to {}", ue4ss_dir.display()));
    if let Ok(rd) = fs::read_dir(&framework_src) {
        for entry in rd.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "dwmapi.dll" || name == "Mods" || name == "mods" { continue; }
            let dst = ue4ss_dir.join(&name);
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                copy_dir_all(&entry.path(), &dst)?;
            } else {
                fs::copy(&entry.path(), &dst).map_err(|e| e.to_string())?;
            }
        }
    }

    // Copiar Mods que vienen por defecto en UE4SS sin sobreescribir la carpeta completa
    let framework_mods = framework_src.join("Mods");
    if framework_mods.exists() {
        let dest_mods = ue4ss_dir.join("Mods");
        let _ = fs::create_dir_all(&dest_mods);
        if let Ok(rd) = fs::read_dir(&framework_mods) {
            for entry in rd.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                let dst = dest_mods.join(&name);
                if !dst.exists() {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        let _ = copy_dir_all(&entry.path(), &dst);
                    } else {
                        let _ = fs::copy(&entry.path(), &dst);
                    }
                }
            }
        }
    }

    let version_file = ue4ss_dir.join("ue4ss.version");
    crate::logger::log(&format!("install_ue4ss: Writing version '{}' to {}", publish_date, version_file.display()));
    let _ = fs::write(&version_file, &publish_date);

    // Escribir enabled.txt vacíos y registrar mods nativos de UE4SS en DB
    let dest_mods = ue4ss_dir.join("Mods");
    let mods_txt_path = ue4ss_dir.join("mods.txt");
    let mut native_mods_to_add: Vec<crate::models::ModInfo> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dest_mods) {
        for entry in rd.filter_map(|e| e.ok()) {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let mod_name = entry.file_name().to_string_lossy().to_string();
                if mod_name.to_lowercase() == "palschema" { continue; } // Omitir PalSchema

                let mod_path = entry.path();
                
                // Determinar el estado de activación en base a mods.txt
                let is_enabled = if mods_txt_path.exists() {
                    let mut found_val = true;
                    if let Ok(content) = fs::read_to_string(&mods_txt_path) {
                        for line in content.lines() {
                            let line_clean = line.trim();
                            if line_clean.starts_with(';') || line_clean.starts_with("//") {
                                continue;
                            }
                            if let Some(pos) = line_clean.find(':') {
                                let name = line_clean[..pos].trim();
                                let val = line_clean[pos+1..].trim();
                                if name.to_lowercase() == mod_name.to_lowercase() {
                                    found_val = val == "1";
                                    break;
                                }
                            } else if line_clean.to_lowercase() == mod_name.to_lowercase() {
                                found_val = true;
                                break;
                            }
                        }
                    }
                    found_val
                } else {
                    true
                };

                native_mods_to_add.push(crate::models::ModInfo {
                    id: mod_name.clone(),
                    name: mod_name.clone(),
                    mod_type: crate::models::ModType::Ue4ss,
                    nexus_mod_id: None,
                    nexus_url: None,
                    nexus_author: Some("UE4SS Native Mod".to_string()),
                    nexus_summary: Some("Core dependency mod installed by UE4SS. Recommended not to disable for safety.".to_string()),
                    nexus_picture_url: None,
                    nexus_endorsements: None,
                    nexus_downloads: None,
                    version: "1.0.0".to_string(),
                    install_date: chrono::Utc::now().to_rfc3339(),
                    source_zip: "ue4ss_framework.zip".to_string(),
                    config_path: None,
                    config_type: Some("auto".to_string()),
                    enabled: is_enabled,
                    game_path: mod_path.to_string_lossy().to_string(),
                    disabled_path: String::new(),
                    pak_destination: None,
                    has_enabled_txt: false,
                    mods_txt_order: None,
                    extra_files: Vec::new(),
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
                    nexus_file_id: None,
                });
            }
        }
    }


    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        for nm in native_mods_to_add {
            if !data.mods.iter().any(|m| m.name.to_lowercase() == nm.name.to_lowercase()) {
                data.mods.push(nm);
            }
        }
        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.ue4ss_enabled = true;
            let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }


    crate::logger::log("install_ue4ss: Cleaning temporary directory...");
    let _ = fs::remove_dir_all(&temp_dir);
    crate::logger::log("install_ue4ss: Installation completed successfully.");
    Ok("UE4SS installed successfully from GitHub (Okaetsu/UE4SS-Palworld)".to_string())
}

#[tauri::command]
pub async fn install_palschema(force_download: bool, state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() {
        return Err("Game path not set".to_string());
    }

    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let palschema_dir = win64.join("ue4ss").join("Mods").join("PalSchema");

    // Check if UE4SS is installed; PalSchema requires UE4SS to operate.
    let dwmapi = win64.join("dwmapi.dll");
    if !dwmapi.exists() {
        return Err("UE4SS is not installed. PalSchema requires UE4SS to operate.".to_string());
    }

    let lib_dep_dir = PathBuf::from(&program_path).join("mods-library").join("dependencies");
    let cached_zip = lib_dep_dir.join("palschema.zip");
    let cached_ver_file = lib_dep_dir.join("palschema.version");

    let tag = match dependency_checker::check_palschema_latest().await {
        Ok(t) => t,
        Err(e) => return Err(format!("Could not determine latest PalSchema tag: {}", e)),
    };

    let mut zip_bytes = Vec::new();
    let mut use_cache = false;

    if !force_download && cached_zip.exists() && cached_ver_file.exists() {
        if let Ok(bytes) = fs::read(&cached_zip) {
            if let Ok(ver) = fs::read_to_string(&cached_ver_file) {
                if ver.trim() == tag.trim() {
                    crate::logger::log("install_palschema: Using cached PalSchema zip from local library.");
                    zip_bytes = bytes;
                    use_cache = true;
                }
            }
        }
    }

    if !use_cache {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let mut asset_url = String::new();
        let mut api_success = false;

        let api_url = format!("https://api.github.com/repos/Okaetsu/PalSchema/releases/tags/{}", tag);
        crate::logger::log(&format!("install_palschema: Fetching GitHub API release from {}", api_url));
        if let Ok(resp) = client.get(&api_url).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(assets) = json["assets"].as_array() {
                        if let Some(asset) = assets.iter().find(|a| {
                            a["name"].as_str().map_or(false, |n| n.ends_with(".zip"))
                        }) {
                            if let Some(url) = asset["browser_download_url"].as_str() {
                                asset_url = url.to_string();
                                api_success = true;
                            }
                        }
                    }
                }
            }
        }

        if !api_success {
            crate::logger::log("install_palschema: GitHub API rate limited or failed. Using HTML fallback...");
            let release_page_url = format!("https://github.com/Okaetsu/PalSchema/releases/tag/{}", tag);
            if let Ok(resp) = client.get(&release_page_url).send().await {
                if let Ok(html) = resp.text().await {
                    let download_prefix = format!("/Okaetsu/PalSchema/releases/download/{}/", tag);
                    let mut search_pos = 0;
                    while let Some(pos) = html[search_pos..].find(&download_prefix) {
                        let start = search_pos + pos;
                        if let Some(end_quote) = html[start..].find('"') {
                            let url_path = &html[start..start + end_quote];
                            search_pos = start + end_quote;
                            if url_path.to_lowercase().ends_with(".zip") {
                                asset_url = format!("https://github.com{}", url_path);
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        if asset_url.is_empty() {
            asset_url = format!("https://github.com/Okaetsu/PalSchema/releases/download/{}/PalSchema_{}.zip", tag, tag.trim_start_matches('v'));
            crate::logger::log(&format!("install_palschema: Fallback crítico - usando URL por defecto {}", asset_url));
        }

        crate::logger::log(&format!("install_palschema: Descargando desde {}", asset_url));
        let resp = client.get(&asset_url)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Download failed. GitHub returned HTTP {}", resp.status()));
        }

        let bytes = resp.bytes()
            .await
            .map_err(|e| format!("Failed to read download bytes: {}", e))?;
        
        zip_bytes = bytes.to_vec();

        let _ = fs::create_dir_all(&lib_dep_dir);
        let _ = fs::write(&cached_zip, &zip_bytes);
        let _ = fs::write(&cached_ver_file, &tag);
    }

    let temp_dir = std::env::temp_dir().join("pmm_palschema");
    let zip_path = temp_dir.join("palschema.zip");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    fs::write(&zip_path, &zip_bytes).map_err(|e| e.to_string())?;

    let extracted = zip_handler::extract_zip_to_temp(&zip_path.to_string_lossy(), &temp_dir.join("extracted"))?;
    let root = find_extracted_root(&extracted);

    if !palschema_dir.exists() {
        fs::create_dir_all(&palschema_dir).map_err(|e| e.to_string())?;
    }
    if let Ok(rd) = fs::read_dir(&root) {
        for entry in rd.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "mods" || name == "Mods" { continue; }
            let dst = palschema_dir.join(&name);
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                copy_dir_all(&entry.path(), &dst)?;
            } else {
                fs::copy(&entry.path(), &dst).map_err(|e| e.to_string())?;
            }
        }
    }

    let version_file = palschema_dir.join("palschema.version");
    crate::logger::log(&format!("install_palschema: Escribiendo versión '{}' en {}", tag, version_file.display()));
    let _ = fs::write(&version_file, &tag);

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.palschema_enabled = true;
            let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    let _ = fs::remove_dir_all(&temp_dir);
    Ok("PalSchema installed successfully from GitHub (Okaetsu/PalSchema)".to_string())
}

#[tauri::command]
pub fn uninstall_ue4ss(state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() { return Err("Game path not set".to_string()); }
    
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let dwmapi = win64.join("dwmapi.dll");
    let ue4ss_dir = win64.join("ue4ss");
    
    if dwmapi.exists() { let _ = fs::remove_file(dwmapi); }
    if ue4ss_dir.exists() { let _ = fs::remove_dir_all(ue4ss_dir); }
    
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.retain(|m| m.mod_type != crate::models::ModType::Ue4ss && m.mod_type != crate::models::ModType::PalSchema);

        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.ue4ss_enabled = false;
            profile.palschema_enabled = false;
            // Also clean non-native UE4SS/PalSchema mods from profile lists
            profile.installed_mod_ids.clear();
            profile.enabled_mod_ids.clear();

            let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }

    
    crate::logger::log("uninstall_ue4ss: UE4SS desinstalado con éxito.");
    Ok("UE4SS uninstalled successfully".to_string())
}

#[tauri::command]
pub fn uninstall_palschema(state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() { return Err("Game path not set".to_string()); }
    
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(&game_path));
    let palschema_dir = win64.join("ue4ss").join("Mods").join("PalSchema");
        
    if palschema_dir.exists() { let _ = fs::remove_dir_all(palschema_dir); }
    
    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        data.mods.retain(|m| m.mod_type != crate::models::ModType::PalSchema);

        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.palschema_enabled = false;
            
            let p_dir = crate::profiles::get_profile_dir(&program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(&program_path, &data_clone);
    }


    
    crate::logger::log("uninstall_palschema: PalSchema desinstalado con éxito.");
    Ok("PalSchema uninstalled successfully".to_string())
}
