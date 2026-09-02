use std::fs;
use std::path::Path;
use zip::read::ZipArchive;
use super::types::{ArchiveFormat, DetectedModType, ZipAnalysis};
use super::detection::{detect_archive_format, find_root_folder, list_7z_files, list_rar_files};

pub fn analyze_zip(zip_path: &str) -> Result<ZipAnalysis, String> {
    let p = Path::new(zip_path);
    let format = detect_archive_format(p);

    let files = match format {
        ArchiveFormat::SevenZip => list_7z_files(zip_path)?,
        ArchiveFormat::Rar => list_rar_files(zip_path)?,
        ArchiveFormat::RawPak => {
            let filename = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            vec![filename]
        }
        _ => {
            // Attempt standard ZIP first
            match fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e)).and_then(|file| {
                ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))
            }) {
                Ok(mut archive) => {
                    let mut list = Vec::new();
                    for i in 0..archive.len() {
                        if let Ok(entry) = archive.by_index(i) {
                            list.push(entry.name().to_string());
                        }
                    }
                    list
                }
                Err(orig_err) => {
                    // Fallback attempt: maybe it's 7z or RAR despite zip extension/header
                    if let Ok(sevenz_list) = list_7z_files(zip_path) {
                        sevenz_list
                    } else if let Ok(rar_list) = list_rar_files(zip_path) {
                        rar_list
                    } else {
                        return Err(orig_err);
                    }
                }
            }
        }
    };

    let mut has_lua = false;
    let mut has_json = false;
    let mut has_pak = false;
    let mut has_dll = false;
    let mut has_info_json = false;
    let mut in_logicmods = false;
    let mut pak_destination_hint = None;

    for name in &files {
        let nl = name.to_lowercase();
        if nl.ends_with(".lua") { has_lua = true; }
        if nl.ends_with(".dll") { has_dll = true; }
        if nl.ends_with(".json") || nl.ends_with(".jsonc") {
            has_json = true;
            if nl.contains("info.json") || nl.contains("modinfo.pmm.json") { has_info_json = true; }
        }
        if nl.ends_with(".pak") { has_pak = true; }
        if nl.contains("logicmods") {
            in_logicmods = true;
            pak_destination_hint = Some("logicmods".to_string());
        }
    }

    // Determine type first, then run content detection with type hint
    let has_palschema_folder = files.iter().any(|f| f.to_lowercase().contains("palschema"));
    let has_palschema_json = files.iter().any(|f| {
        let fl = f.to_lowercase();
        if (fl.ends_with(".json") || fl.ends_with(".jsonc"))
            && !fl.ends_with("info.json")
            && !fl.ends_with("modinfo.json")
            && !fl.ends_with("modinfo.pmm.json")
            && !fl.ends_with("manifest.json")
            && !fl.ends_with("metadata.json")
        {
            let parts: Vec<&str> = fl.split('/').collect();
            // If the JSON is inside scripts/ or dlls/, it is not a PalSchema data table
            if parts.contains(&"scripts") || parts.contains(&"dlls") || fl.contains("/scripts/") || fl.contains("/dlls/") {
                false
            } else if fl.contains("palschema") {
                true
            } else {
                parts.iter().any(|part| {
                    matches!(
                        *part,
                        "pals"
                            | "spawns"
                            | "items"
                            | "blueprints"
                            | "unique"
                            | "enums"
                            | "skins"
                            | "translations"
                            | "raw"
                    )
                })
            }
        } else {
            false
        }
    });
    let has_ue4ss = has_lua || has_dll;
    let has_palschema = has_palschema_folder || has_palschema_json;
    let is_hybrid = (has_ue4ss && has_palschema) || (has_ue4ss && has_pak);

    let has_altermatic = files.iter().any(|f| {
        let fl = f.to_lowercase();
        fl.contains("swapjson")
            || fl.contains("alterconfig")
            || fl.ends_with(".swap.json")
            || (fl.ends_with(".json") && (fl.contains("skelmesh") || fl.contains("matreplace") || fl.contains("altermatic")))
    });

    let detected_type_pre = if is_hybrid {
        DetectedModType::Hybrid
    } else if has_altermatic {
        DetectedModType::Altermatic
    } else if has_palschema_folder || has_palschema || has_palschema_json {
        DetectedModType::PalSchema
    } else if has_lua || has_dll {
        DetectedModType::Ue4ss
    } else if has_pak {
        if in_logicmods {
            DetectedModType::LogicMods
        } else {
            DetectedModType::Pak
        }
    } else if has_json {
        DetectedModType::PalSchema
    } else {
        DetectedModType::Unknown
    };

    let root_folder = find_root_folder(&files);

    Ok(ZipAnalysis {
        detected_type: detected_type_pre,
        has_lua,
        has_json,
        has_palschema_json,
        has_pak,
        has_altermatic,
        has_dll,
        has_info_json,
        pak_destination_hint,
        root_folder,
        files,
    })
}
