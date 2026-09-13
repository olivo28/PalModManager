pub mod types;
pub mod parser;
pub mod manifest_builder;
pub mod version;

use base64::Engine;
use std::path::Path;
use tauri::State;
use crate::models::InstallManifest;
use crate::state::AppState;
use crate::zip_handler;
use self::parser::{parse_fomod_config, parse_fomod_info};
use self::types::{BuildFomodManifestPayload, FomodConfig};

#[tauri::command]
pub async fn get_fomod_config(zip_path: String) -> Result<FomodConfig, String> {
    crate::logger::log(&format!("get_fomod_config: Reading FOMOD for '{}'", zip_path));

    // 1. Read ModuleConfig.xml
    let config_xml = zip_handler::read_archive_file(&zip_path, "fomod/ModuleConfig.xml")
        .or_else(|| zip_handler::read_archive_file(&zip_path, "fomod/moduleconfig.xml"))
        .or_else(|| zip_handler::read_archive_file(&zip_path, "fomod\\ModuleConfig.xml"))
        .or_else(|| zip_handler::read_archive_file(&zip_path, "fomod\\moduleconfig.xml"))
        .ok_or_else(|| "Could not find fomod/ModuleConfig.xml in archive".to_string())?;

    let mut config = parse_fomod_config(&config_xml)?;

    // 2. Read info.xml if present
    if let Some(info_xml) = zip_handler::read_archive_file(&zip_path, "fomod/info.xml")
        .or_else(|| zip_handler::read_archive_file(&zip_path, "fomod/Info.xml"))
        .or_else(|| zip_handler::read_archive_file(&zip_path, "fomod\\info.xml"))
    {
        config.info = Some(parse_fomod_info(&info_xml));
    }

    // 3. Extract banner image if specified
    if let Some(ref img_path) = config.module_image {
        if !img_path.is_empty() {
            if let Some(bytes) = zip_handler::read_archive_file_bytes(&zip_path, img_path) {
                let ext = img_path.split('.').last().unwrap_or("png").to_lowercase();
                let mime = match ext.as_str() {
                    "jpg" | "jpeg" => "image/jpeg",
                    "webp" => "image/webp",
                    "gif" => "image/gif",
                    _ => "image/png",
                };
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                config.banner_base64 = Some(format!("data:{};base64,{}", mime, b64));
            }
        }
    }

    // 4. Extract plugin images if specified
    for step in &mut config.install_steps {
        for group in &mut step.groups {
            for plugin in &mut group.plugins {
                if let Some(ref img_path) = plugin.image {
                    if !img_path.is_empty() {
                        if let Some(bytes) = zip_handler::read_archive_file_bytes(&zip_path, img_path) {
                            let ext = img_path.split('.').last().unwrap_or("png").to_lowercase();
                            let mime = match ext.as_str() {
                                "jpg" | "jpeg" => "image/jpeg",
                                "webp" => "image/webp",
                                "gif" => "image/gif",
                                _ => "image/png",
                            };
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            plugin.image_base64 = Some(format!("data:{};base64,{}", mime, b64));
                        }
                    }
                }
            }
        }
    }

    Ok(config)
}

#[tauri::command]
pub async fn build_fomod_manifest(
    payload: BuildFomodManifestPayload,
    state: State<'_, AppState>,
) -> Result<InstallManifest, String> {
    let game_path_str = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };

    if game_path_str.is_empty() {
        return Err("No game path configured. Set it first.".to_string());
    }

    let game_path = Path::new(&game_path_str);

    let filename = Path::new(&payload.zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let parsed_nexus = crate::nexus::parse_mod_filename(&filename);

    // Prefer custom name -> FOMOD modName/info.name -> parsed Nexus clean name -> file stem
    let clean_name = payload
        .custom_name
        .or(payload.mod_name)
        .or(parsed_nexus.name)
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            Path::new(&payload.zip_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Mod".to_string())
        });

    let folder_name = payload
        .custom_folder
        .filter(|f| !f.trim().is_empty())
        .unwrap_or_else(|| {
            clean_name
                .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_")
                .trim_matches('_')
                .to_string()
        });

    let fomod_info = zip_handler::read_archive_file(&payload.zip_path, "fomod/info.xml")
        .or_else(|| zip_handler::read_archive_file(&payload.zip_path, "fomod/Info.xml"))
        .or_else(|| zip_handler::read_archive_file(&payload.zip_path, "fomod\\info.xml"))
        .map(|xml| parse_fomod_info(&xml));

    let xml_version = fomod_info.as_ref().map(|fi| fi.version.as_str()).filter(|v| !v.trim().is_empty());
    let zip_version = parsed_nexus.version.as_deref();
    let payload_version = payload.version.as_deref();

    let candidate_version = version::resolve_fomod_version_consensus(
        xml_version,
        zip_version,
        payload_version,
    );

    let version = if let Some(pv) = payload.version.as_deref().filter(|v| !v.trim().is_empty()) {
        if crate::commands::nexus_commands::is_version_newer(&candidate_version, pv) {
            pv.to_string()
        } else {
            candidate_version
        }
    } else {
        candidate_version
    };

    let author = payload.author.or_else(|| fomod_info.as_ref().map(|fi| fi.author.clone()).filter(|a| !a.trim().is_empty()));
    let summary = payload.summary.or_else(|| fomod_info.as_ref().map(|fi| fi.description.clone()).filter(|s| !s.trim().is_empty()));

    manifest_builder::build_fomod_install_manifest_internal(
        game_path,
        &payload.selected_files,
        &clean_name,
        Some(folder_name),
        parsed_nexus.nexus_id,
        version,
        author,
        summary,
        payload.fomod_choices,
    )
}
