use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

pub const CANONICAL_UE4SS_FILES: &[&str] = &[
    "dwmapi.dll",
    "ue4ss/UE4SS.dll",
    "ue4ss/UE4SS-settings.ini",
    "ue4ss/MemberVariableLayout.ini",
    "ue4ss/LICENSE",
    "ue4ss/ue4ss.version",
    "ue4ss/Mods/mods.txt",
    "ue4ss/Mods/mods.json",
];

pub const CANONICAL_UE4SS_SYSTEM_MODS: &[&str] = &[
    "BPML_GenericFunctions",
    "BPModLoaderMod",
    "CheatManagerEnablerMod",
    "ConsoleCommandsMod",
    "ConsoleEnablerMod",
    "Keybinds",
    "LineTraceMod",
    "shared",
    "SplitScreenMod",
];

pub const CANONICAL_PALSCHEMA_FILES: &[&str] = &[
    "PalSchema/dlls/main.dll",
    "PalSchema/enabled.txt",
    "PalSchema/LICENSE",
    "PalSchema/palschema.version",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyManifest {
    pub dep_type: String,
    pub version: String,
    pub install_date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adopted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_generated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_adopted: Option<bool>,
    pub files: Vec<String>,
}

pub fn ensure_ue4ss_manifest(game_path: &str, detected_version: Option<&str>) -> Option<DependencyManifest> {
    if game_path.is_empty() {
        return None;
    }

    let win64 = crate::dependency_checker::get_binaries_dir(Path::new(game_path));
    let ue4ss_dir = win64.join("ue4ss");
    let manifest_path = ue4ss_dir.join("ue4ss.manifest.json");

    if manifest_path.exists() {
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<DependencyManifest>(&content) {
                return Some(manifest);
            }
        }
    }

    if !win64.join("dwmapi.dll").exists() && !ue4ss_dir.exists() {
        return None;
    }

    let version = detected_version.unwrap_or("installed").to_string();
    let mut files = Vec::new();

    if win64.join("dwmapi.dll").exists() {
        files.push("dwmapi.dll".to_string());
    }

    if ue4ss_dir.exists() {
        if let Ok(entries) = fs::read_dir(&ue4ss_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.eq_ignore_ascii_case("mods") {
                    // Collect system mods inside Mods/
                    let mods_dir = entry.path();
                    if let Ok(mod_entries) = fs::read_dir(&mods_dir) {
                        for m_entry in mod_entries.flatten() {
                            let m_name = m_entry.file_name().to_string_lossy().to_string();
                            let is_system_mod = CANONICAL_UE4SS_SYSTEM_MODS.iter().any(|&s| s.eq_ignore_ascii_case(&m_name));
                            if is_system_mod {
                                collect_relative_files(&m_entry.path(), &ue4ss_dir, &mut files, "ue4ss");
                            } else if m_entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                                // Files directly in Mods/ like mods.txt, mods.json
                                files.push(format!("ue4ss/Mods/{}", m_name));
                            }
                        }
                    }
                } else if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    collect_relative_files(&entry.path(), &ue4ss_dir, &mut files, "ue4ss");
                } else {
                    files.push(format!("ue4ss/{}", name));
                }
            }
        }
    }

    files.sort();
    files.dedup();

    // Inspect real file modification/creation time of UE4SS.dll or directory
    let ue4ss_dll = ue4ss_dir.join("UE4SS.dll");
    let actual_file_time = fs::metadata(&ue4ss_dll)
        .or_else(|_| fs::metadata(&ue4ss_dir))
        .ok()
        .and_then(|m| m.modified().or_else(|_| m.created()).ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let now_str = chrono::Utc::now().to_rfc3339();

    let manifest = DependencyManifest {
        dep_type: "ue4ss".to_string(),
        version,
        install_date: actual_file_time,
        detected_at: Some(now_str.clone()),
        adopted_at: Some(now_str.clone()),
        manifest_generated_at: Some(now_str),
        is_adopted: Some(true),
        files,
    };

    if ue4ss_dir.exists() {
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            let _ = fs::write(&manifest_path, json);
        }
    }

    Some(manifest)
}

pub fn ensure_palschema_manifest(game_path: &str, detected_version: Option<&str>) -> Option<DependencyManifest> {
    if game_path.is_empty() {
        return None;
    }

    let ue4ss_mods_dir = crate::dependency_checker::get_ue4ss_mods_dir(Path::new(game_path));
    let palschema_dir = ue4ss_mods_dir.join("PalSchema");
    let manifest_path = palschema_dir.join("palschema.manifest.json");

    if manifest_path.exists() {
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<DependencyManifest>(&content) {
                return Some(manifest);
            }
        }
    }

    if !palschema_dir.exists() {
        return None;
    }

    let version = detected_version.unwrap_or("installed").to_string();
    let mut files = Vec::new();

    if palschema_dir.exists() {
        if let Ok(entries) = fs::read_dir(&palschema_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let lower = name.to_lowercase();
                // Exclude user mod directories from the dependency manifest
                if lower == "mods" || lower == "storage" {
                    continue;
                }
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    collect_relative_files(&entry.path(), &palschema_dir, &mut files, "PalSchema");
                } else {
                    files.push(format!("PalSchema/{}", name));
                }
            }
        }
    }

    files.sort();
    files.dedup();

    // Inspect real file modification/creation time of main.dll or directory
    let palschema_dll = palschema_dir.join("dlls").join("main.dll");
    let actual_file_time = fs::metadata(&palschema_dll)
        .or_else(|_| fs::metadata(&palschema_dir))
        .ok()
        .and_then(|m| m.modified().or_else(|_| m.created()).ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let now_str = chrono::Utc::now().to_rfc3339();

    let manifest = DependencyManifest {
        dep_type: "palschema".to_string(),
        version,
        install_date: actual_file_time,
        detected_at: Some(now_str.clone()),
        adopted_at: Some(now_str.clone()),
        manifest_generated_at: Some(now_str),
        is_adopted: Some(true),
        files,
    };

    if palschema_dir.exists() {
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            let _ = fs::write(&manifest_path, json);
        }
    }

    Some(manifest)
}

fn collect_relative_files(dir: &Path, base_dir: &Path, files: &mut Vec<String>, prefix: &str) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_relative_files(&path, base_dir, files, prefix);
            } else if let Ok(rel) = path.strip_prefix(base_dir) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                files.push(format!("{}/{}", prefix, rel_str));
            }
        }
    }
}
