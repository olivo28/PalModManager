use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;
use tauri::State;
use crate::installer;
use crate::nexus;
use crate::state::AppState;
use crate::zip_handler;

#[tauri::command]
pub async fn analyze_zip(zip_path: String, state: State<'_, AppState>) -> Result<Value, String> {
    crate::logger::log(&format!("analyze_zip: Analyzing archive '{}'", zip_path));
    let _ = state; // state unused here now

    let analysis = zip_handler::analyze_zip(&zip_path)?;

    let mut nexus_id = zip_handler::extract_nexus_id_from_path(&zip_path)
        .or_else(|| {
            let filename = Path::new(&zip_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            nexus::extract_nexus_id(&filename)
        });

    let detected_type = match analysis.detected_type {
        zip_handler::DetectedModType::Ue4ss => "ue4ss",
        zip_handler::DetectedModType::PalSchema => "palschema",
        zip_handler::DetectedModType::Pak => "pak",
        zip_handler::DetectedModType::LogicMods => "logicmods",
        zip_handler::DetectedModType::Hybrid => "hybrid",
        zip_handler::DetectedModType::Altermatic => "altermatic",
        zip_handler::DetectedModType::Unknown => "unknown",
    };

    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            if let Some(content) = zip_handler::read_archive_file(&zip_path, target_file) {
                if let Ok(val) = serde_json::from_str::<Value>(&content) {
                    modinfo_data = Some(val);
                }
            }
        }
    }

    let mut sidecar_data: Option<Value> = None;
    let pmm_sidecar = PathBuf::from(format!("{}.pmm.json", zip_path));
    let pmm_sidecar_alt = Path::new(&zip_path).with_extension("pmm.json");
    if pmm_sidecar.exists() {
        if let Ok(c) = fs::read_to_string(&pmm_sidecar) {
            sidecar_data = serde_json::from_str(&c).ok();
        }
    } else if pmm_sidecar_alt.exists() {
        if let Ok(c) = fs::read_to_string(&pmm_sidecar_alt) {
            sidecar_data = serde_json::from_str(&c).ok();
        }
    }

    if nexus_id.is_none() {
        if let Some(ref info) = modinfo_data {
            if let Some(id) = info.get("nexusModId").and_then(|id| id.as_u64()).or_else(|| info.get("nexus_mod_id").and_then(|id| id.as_u64())) {
                nexus_id = Some(id as u32);
            } else if let Some(id_str) = info.get("nexusModId").and_then(|id| id.as_str()).or_else(|| info.get("nexus_mod_id").and_then(|id| id.as_str())) {
                if let Ok(id) = id_str.parse::<u32>() {
                    nexus_id = Some(id);
                }
            }
        }
        if nexus_id.is_none() {
            if let Some(ref sc) = sidecar_data {
                if let Some(id) = sc.get("nexusModId").or_else(|| sc.get("modId")).and_then(|id| id.as_u64()) {
                    nexus_id = Some(id as u32);
                } else if let Some(id_str) = sc.get("nexusModId").or_else(|| sc.get("modId")).and_then(|id| id.as_str()) {
                    if let Ok(id) = id_str.parse::<u32>() {
                        nexus_id = Some(id);
                    }
                }
            }
        }
    }

    let detected_version = {
        let from_info = modinfo_data.as_ref().and_then(|info| {
            info.get("version").and_then(|v| v.as_str()).map(|s| s.to_string())
        });
        if from_info.is_some() {
            from_info
        } else if let Some(from_sc) = sidecar_data.as_ref().and_then(|sc| {
            sc.get("version").or_else(|| sc.get("nexusVersion")).and_then(|v| v.as_str()).map(|s| s.to_string())
        }) {
            Some(from_sc)
        } else {
            let filename = Path::new(&zip_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            nexus::parse_mod_filename(&filename).version
        }
    };

    let nexus_info = if let Some(ref sc) = sidecar_data {
        let pic = sc.get("nexusPictureUrl").or_else(|| sc.get("pictureUrl")).and_then(|v| v.as_str());
        let name = sc.get("nexusName").or_else(|| sc.get("name")).and_then(|v| v.as_str());
        let author = sc.get("nexusAuthor").or_else(|| sc.get("author")).and_then(|v| v.as_str());
        let summary = sc.get("nexusSummary").or_else(|| sc.get("summary")).or_else(|| sc.get("description")).and_then(|v| v.as_str());
        let ver = sc.get("version").or_else(|| sc.get("nexusVersion")).and_then(|v| v.as_str());
        if pic.is_some() || name.is_some() || author.is_some() {
            Some(serde_json::json!({
                "modId": nexus_id.unwrap_or(0),
                "name": name.unwrap_or(""),
                "author": author.unwrap_or(""),
                "summary": summary.unwrap_or(""),
                "pictureUrl": pic.unwrap_or(""),
                "version": ver.unwrap_or(""),
                "downloads": sc.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                "endorsements": sc.get("endorsements").and_then(|v| v.as_u64()).unwrap_or(0),
            }))
        } else {
            None
        }
    } else {
        None
    };

    Ok(serde_json::json!({
        "zipPath": zip_path,
        "detectedType": detected_type,
        "hasLua": analysis.has_lua,
        "hasJson": analysis.has_json,
        "hasPalSchemaJson": analysis.has_palschema_json,
        "hasPak": analysis.has_pak,
        "hasAltermatic": analysis.has_altermatic,
        "hasInfoJson": analysis.has_info_json,
        "pakDestinationHint": analysis.pak_destination_hint,
        "rootFolder": analysis.root_folder,
        "fileCount": analysis.files.len(),
        "nexusModId": nexus_id,
        "detectedVersion": detected_version,
        "nexusInfo": nexus_info,
        "modinfo": modinfo_data,
        "files": analysis.files,
    }))
}

#[tauri::command]
pub async fn check_mod_exists_command(
    zip_path: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let analysis = zip_handler::analyze_zip(&zip_path)?;
    let filename = Path::new(&zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let nexus_id = nexus::extract_nexus_id(&filename);

    let mut metadata_folder_name = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            if let Some(content) = zip_handler::read_archive_file(&zip_path, target_file) {
                if let Ok(val) = serde_json::from_str::<Value>(&content) {
                    if let Some(name) = val.get("folderName").and_then(|v| v.as_str()) {
                        metadata_folder_name = Some(name.to_string());
                    } else if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                        metadata_folder_name = Some(name.to_string());
                    }
                }
            }
        }
    }

    let folder_name = metadata_folder_name.unwrap_or_else(|| {
        let detected = zip_handler::detect_folder_name_from_files(&analysis.files, &filename);
        if detected.is_empty() || detected == "UnresolvedMod" {
            installer::clean_zip_name(&filename)
        } else {
            detected
        }
    });

    let data = state.data.lock().map_err(|e| e.to_string())?;

    let mod_type = match analysis.detected_type {
        zip_handler::DetectedModType::Ue4ss => crate::models::ModType::Ue4ss,
        zip_handler::DetectedModType::PalSchema => crate::models::ModType::PalSchema,
        zip_handler::DetectedModType::Pak => crate::models::ModType::Pak,
        zip_handler::DetectedModType::LogicMods => crate::models::ModType::LogicMods,
        zip_handler::DetectedModType::Hybrid => crate::models::ModType::Hybrid,
        zip_handler::DetectedModType::Altermatic => crate::models::ModType::Altermatic,
        _ => crate::models::ModType::Pak,
    };

    let profile_mods = crate::commands::mod_commands::filter_mods_for_current_profile_pub(&data);
    let existing = installer::check_mod_exists(&folder_name, &mod_type, nexus_id, &profile_mods);

    Ok(serde_json::json!({
        "exists": existing.is_some(),
        "modInfo": existing,
        "modFolderName": folder_name,
    }))
}
