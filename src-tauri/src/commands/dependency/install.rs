use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use crate::dependency_checker;
use crate::state::AppState;
use crate::zip_handler;
use super::vault::{migrate_legacy_dependency_zips, sanitize_version_tag, save_to_vault};

pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
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

pub fn find_extracted_root(src: &Path) -> PathBuf {
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

pub async fn apply_ue4ss_zip_bytes(
    zip_bytes: &[u8],
    publish_date: &str,
    program_path: &str,
    game_path: &str,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let ue4ss_dir = win64.join("ue4ss");

    let temp_dir = std::env::temp_dir().join("pmm_ue4ss");
    let zip_path = temp_dir.join("ue4ss.zip");
    crate::logger::log(&format!("install_ue4ss: Saving temporary ZIP to {}", zip_path.display()));
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    fs::write(&zip_path, zip_bytes).map_err(|e| e.to_string())?;

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
    let _ = fs::write(&version_file, publish_date);

    // Escribir enabled.txt vacíos y registrar mods nativos de UE4SS en DB
    let dest_mods = ue4ss_dir.join("Mods");
    let mods_txt_path = ue4ss_dir.join("mods.txt");
    let mut native_mods_to_add: Vec<crate::models::ModInfo> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dest_mods) {
        for entry in rd.filter_map(|e| e.ok()) {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let mod_name = entry.file_name().to_string_lossy().to_string();
                if mod_name.to_lowercase() == "palschema" { continue; }

                let mod_path = entry.path();
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
                    ignored_keys: None,
                    has_pending_update: None,
                    origin_load_method: Some("mods_txt".to_string()),
                    custom_notes: None,
                });
            }
        }
    }

    {
        let clean_ver = sanitize_version_tag(publish_date, "ue4ss");
        let version_file = ue4ss_dir.join("ue4ss.version");
        let _ = fs::write(&version_file, &clean_ver);

        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        for nm in native_mods_to_add {
            if !data.mods.iter().any(|m| m.name.to_lowercase() == nm.name.to_lowercase()) {
                data.mods.push(nm);
            }
        }
        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.ue4ss_enabled = true;
            profile.dependency_mode = crate::models::DependencyMode::Standard;
            let p_dir = crate::profiles::get_profile_dir(program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
            let ue4ss_backup = p_dir.join("ue4ss");
            let dwmapi_backup = p_dir.join("dwmapi.dll");
            if ue4ss_dir.exists() {
                let _ = copy_dir_all(&ue4ss_dir, &ue4ss_backup);
            }
            if win64.join("dwmapi.dll").exists() {
                let _ = fs::copy(win64.join("dwmapi.dll"), &dwmapi_backup);
            }
            crate::logger::log(&format!("install_ue4ss: Updated profile '{}' with ue4ss_enabled=true, dependency_mode=Standard, and synced backup.", profile.id));
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(program_path, &data_clone);
    }

    crate::logger::log("install_ue4ss: Cleaning temporary directory...");
    let _ = fs::remove_dir_all(&temp_dir);
    crate::logger::log("install_ue4ss: Installation completed successfully.");
    Ok(format!("UE4SS ({}) installed successfully.", publish_date))
}

pub async fn apply_palschema_zip_bytes(
    zip_bytes: &[u8],
    tag: &str,
    program_path: &str,
    game_path: &str,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let palschema_dir = if win64.join("dwmapi.dll").exists() {
        win64.join("ue4ss").join("Mods").join("PalSchema")
    } else {
        Path::new(game_path).join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("PalSchema")
    };

    let temp_dir = std::env::temp_dir().join("pmm_palschema");
    let zip_path = temp_dir.join("palschema.zip");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    fs::write(&zip_path, zip_bytes).map_err(|e| e.to_string())?;

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

    let clean_tag = sanitize_version_tag(tag, "palschema");
    let version_file = palschema_dir.join("palschema.version");
    crate::logger::log(&format!("install_palschema: Escribiendo versión '{}' en {}", clean_tag, version_file.display()));
    let _ = fs::write(&version_file, &clean_tag);

    {
        let mut data = state.data.lock().map_err(|e| e.to_string())?;
        let current_profile_id = data.current_profile_id.clone();
        if let Some(profile) = data.profiles.iter_mut().find(|p| p.id == current_profile_id) {
            profile.palschema_enabled = true;
            if profile.dependency_mode == crate::models::DependencyMode::None {
                profile.dependency_mode = crate::models::DependencyMode::Standard;
            }
            let p_dir = crate::profiles::get_profile_dir(program_path, &profile.id);
            if let Ok(json) = serde_json::to_string_pretty(profile) {
                let _ = fs::write(p_dir.join("profile.json"), json);
            }
            let palschema_backup = p_dir.join("palschema");
            if palschema_dir.exists() {
                let _ = copy_dir_all(&palschema_dir, &palschema_backup);
            }
            crate::logger::log(&format!("install_palschema: Updated profile '{}' with palschema_enabled=true and synced backup.", profile.id));
        }
        let data_clone = data.clone();
        drop(data);
        let _ = crate::db::save_db(program_path, &data_clone);
    }

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(format!("PalSchema ({}) installed successfully.", tag))
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

    let dep_status = crate::dependency_checker::check_dependencies(&game_path);
    if dep_status.ue4ss_installed && !force_download {
        crate::logger::log("install_ue4ss: UE4SS is already installed and force_download is false. Skipping installation.");
        return Ok("UE4SS is already installed.".to_string());
    }

    migrate_legacy_dependency_zips(&program_path);

    let client = reqwest::Client::builder()
        .user_agent("PalModManager/1.7.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut asset_url = String::new();
    let mut publish_date = String::new();
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
                            let mut latest_html_dt: Option<chrono::DateTime<chrono::Utc>> = None;
                            let mut cursor = 0;
                            while let Some(pos) = html[cursor..].find("datetime=\"") {
                                let time_start = cursor + pos + 10;
                                if let Some(len) = html[time_start..].find('"') {
                                    let dt_raw = &html[time_start..time_start + len];
                                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_raw) {
                                        let dt_utc: chrono::DateTime<chrono::Utc> = dt.into();
                                        match latest_html_dt {
                                            Some(cur) if dt_utc > cur => { latest_html_dt = Some(dt_utc); }
                                            None => { latest_html_dt = Some(dt_utc); }
                                            _ => {}
                                        }
                                    }
                                    cursor = time_start + len;
                                } else {
                                    break;
                                }
                            }
                            if let Some(dt) = latest_html_dt {
                                publish_date = dt.format("%d.%m.%Y").to_string();
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
        publish_date = chrono::Utc::now().format("%d.%m.%Y").to_string();
    }

    crate::logger::log(&format!("install_ue4ss: Downloading ZIP from {}", asset_url));
    let bytes = client.get(&asset_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    
    let zip_bytes = bytes.to_vec();

    // Save into versioned vault
    let _ = save_to_vault(&program_path, "ue4ss", &publish_date, &zip_bytes);

    apply_ue4ss_zip_bytes(&zip_bytes, &publish_date, &program_path, &game_path, &state).await
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

    let dep_status = crate::dependency_checker::check_dependencies(&game_path);
    if dep_status.palschema_installed && !force_download {
        crate::logger::log("install_palschema: PalSchema is already installed and force_download is false. Skipping installation.");
        return Ok("PalSchema is already installed.".to_string());
    }

    if !dep_status.ue4ss_installed {
        return Err("UE4SS is not installed. PalSchema requires UE4SS to operate.".to_string());
    }

    migrate_legacy_dependency_zips(&program_path);

    let tag = match dependency_checker::check_palschema_latest().await {
        Ok(t) => t,
        Err(e) => return Err(format!("Could not determine latest PalSchema tag: {}", e)),
    };

    let client = reqwest::Client::builder()
        .user_agent("PalModManager/1.7.0")
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
    
    let zip_bytes = bytes.to_vec();

    // Save into versioned vault
    let _ = save_to_vault(&program_path, "palschema", &tag, &zip_bytes);

    apply_palschema_zip_bytes(&zip_bytes, &tag, &program_path, &game_path, &state).await
}

#[tauri::command]
pub fn uninstall_ue4ss(state: State<'_, AppState>) -> Result<String, String> {
    let (game_path, program_path) = {
        let locked = state.data.lock().map_err(|e| e.to_string())?;
        (locked.settings.game_path.clone(), locked.settings.program_path.clone())
    };
    if game_path.is_empty() { return Err("Game path not set".to_string()); }

    // Guard: Workshop installations cannot be uninstalled from PMM
    let game_profile = crate::dependency_checker::build_game_profile(Path::new(&game_path));
    if game_profile.ue4ss_install_mode == crate::dependency_checker::UE4SSInstallMode::Workshop {
        return Err("UE4SS is managed by Steam Workshop. To uninstall, unsubscribe from the mod in Steam.".to_string());
    }

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

    // Guard: Workshop installations cannot be uninstalled from PMM
    let game_profile = crate::dependency_checker::build_game_profile(Path::new(&game_path));
    if game_profile.ue4ss_install_mode == crate::dependency_checker::UE4SSInstallMode::Workshop {
        return Err("PalSchema is managed by Steam Workshop. To uninstall, unsubscribe from the mod in Steam.".to_string());
    }

    // Use the profile's resolved palschema path (handles both Standard and edge cases)
    let palschema_dir = game_profile.ue4ss_mods_dir.join("PalSchema");
        
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
