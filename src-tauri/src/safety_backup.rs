use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub r#type: String, // "file", "ue4ss_mod", "palschema_mod", "pak_manifest"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_name: Option<String>,
    pub relative_to_game: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub had_enabled_txt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mods_txt_line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub timestamp: String,
    pub pmm_version: String,
    pub game_root: String,
    pub ue4ss_mode: String,
    pub entries: Vec<BackupEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyBackupInfo {
    pub exists: bool,
    pub timestamp: Option<String>,
    pub pmm_version: Option<String>,
    pub zip_size_bytes: Option<u64>,
    pub total_entries: usize,
    pub ue4ss_mods_count: usize,
    pub palschema_mods_count: usize,
}

pub fn get_game_root_hash(game_root: &str) -> String {
    let normalized = game_root.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let mut hash: u64 = 5381;
    for byte in normalized.bytes() {
        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u64);
    }
    format!("{:016x}", hash)
}

pub fn get_backup_zip_path(program_path: &str, game_root: &str) -> PathBuf {
    let hash = get_game_root_hash(game_root);
    PathBuf::from(program_path)
        .join("backups")
        .join(format!("{}_initial_state.zip", hash))
}

pub fn has_initial_safety_backup(program_path: &str, game_root: &str) -> bool {
    let zip_path = get_backup_zip_path(program_path, game_root);
    zip_path.exists()
}

pub fn get_safety_backup_info(program_path: &str, game_root: &str) -> Result<SafetyBackupInfo, String> {
    let zip_path = get_backup_zip_path(program_path, game_root);
    if !zip_path.exists() {
        return Ok(SafetyBackupInfo {
            exists: false,
            timestamp: None,
            pmm_version: None,
            zip_size_bytes: None,
            total_entries: 0,
            ue4ss_mods_count: 0,
            palschema_mods_count: 0,
        });
    }

    let file_size = fs::metadata(&zip_path).map(|m| m.len()).ok();
    let file = File::open(&zip_path).map_err(|e| format!("Failed to open backup zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid backup zip: {}", e))?;

    let manifest_file = archive.by_name("snapshot_manifest.json")
        .map_err(|_| "snapshot_manifest.json not found in backup zip".to_string())?;

    let manifest: SnapshotManifest = serde_json::from_reader(manifest_file)
        .map_err(|e| format!("Failed to parse snapshot manifest: {}", e))?;

    let ue4ss_count = manifest.entries.iter().filter(|e| e.r#type == "ue4ss_mod").count();
    let ps_count = manifest.entries.iter().filter(|e| e.r#type == "palschema_mod").count();

    Ok(SafetyBackupInfo {
        exists: true,
        timestamp: Some(manifest.timestamp),
        pmm_version: Some(manifest.pmm_version),
        zip_size_bytes: file_size,
        total_entries: manifest.entries.len(),
        ue4ss_mods_count: ue4ss_count,
        palschema_mods_count: ps_count,
    })
}

pub fn create_initial_safety_backup(game_path: &str, program_path: &str, force: bool) -> Result<bool, String> {
    if game_path.is_empty() {
        return Ok(false);
    }
    let game_root = Path::new(game_path);
    if !game_root.exists() {
        return Ok(false);
    }

    let zip_path = get_backup_zip_path(program_path, game_path);
    if zip_path.exists() && !force {
        crate::logger::log(&format!("create_initial_safety_backup: Backup already exists for {} at {:?}", game_path, zip_path));
        return Ok(false);
    }

    if let Some(parent) = zip_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    crate::logger::log(&format!("create_initial_safety_backup: Taking snapshot for {} -> {:?}", game_path, zip_path));

    let profile = crate::dependency_checker::build_game_profile(game_root);
    let ue4ss_mode_str = match profile.ue4ss_install_mode {
        crate::dependency_checker::UE4SSInstallMode::Standard => "Standard",
        crate::dependency_checker::UE4SSInstallMode::Workshop => "Workshop",
        crate::dependency_checker::UE4SSInstallMode::NotFound => "NotFound",
    };

    let mut entries: Vec<BackupEntry> = Vec::new();

    // Map mods.txt lines for quick reference
    let mut mods_txt_lines: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if profile.mods_txt_path.exists() {
        if let Ok(content) = fs::read_to_string(&profile.mods_txt_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with(';') && !trimmed.starts_with("//") {
                    if let Some(pos) = trimmed.find(':') {
                        let mod_name = trimmed[..pos].trim().to_lowercase();
                        mods_txt_lines.insert(mod_name, trimmed.to_string());
                    }
                }
            }
        }
    }

    // Temporary staging directory to build zip
    let temp_staging = std::env::temp_dir().join(format!("pmm_safety_snap_{}", uuid::Uuid::new_v4()));
    let _ = fs::create_dir_all(&temp_staging);

    // 1. Backup mods.txt
    if profile.mods_txt_path.exists() {
        if let Some(rel) = pathdiff::diff_paths(&profile.mods_txt_path, game_root) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let target_file = temp_staging.join("mods.txt");
            if fs::copy(&profile.mods_txt_path, &target_file).is_ok() {
                entries.push(BackupEntry {
                    r#type: "file".to_string(),
                    mod_name: None,
                    relative_to_game: rel_str,
                    zip_path: Some("mods.txt".to_string()),
                    had_enabled_txt: None,
                    mods_txt_line: None,
                    size: fs::metadata(&profile.mods_txt_path).map(|m| m.len()).ok(),
                });
            }
        }
    }

    // 2. Backup UE4SS-settings.ini if exists
    let ue4ss_ini_path = profile.ue4ss_mods_dir.parent().map(|p| p.join("UE4SS-settings.ini"));
    if let Some(ini_path) = ue4ss_ini_path {
        if ini_path.exists() {
            if let Some(rel) = pathdiff::diff_paths(&ini_path, game_root) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                let target_file = temp_staging.join("UE4SS-settings.ini");
                if fs::copy(&ini_path, &target_file).is_ok() {
                    entries.push(BackupEntry {
                        r#type: "file".to_string(),
                        mod_name: None,
                        relative_to_game: rel_str,
                        zip_path: Some("UE4SS-settings.ini".to_string()),
                        had_enabled_txt: None,
                        mods_txt_line: None,
                        size: fs::metadata(&ini_path).map(|m| m.len()).ok(),
                    });
                }
            }
        }
    }

    // 3. Backup UE4SS Mods
    if profile.ue4ss_mods_dir.exists() {
        let ue4ss_target_dir = temp_staging.join("ue4ss_mods");
        let _ = fs::create_dir_all(&ue4ss_target_dir);

        if let Ok(rd) = fs::read_dir(&profile.ue4ss_mods_dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    continue;
                }
                let mod_name = entry.file_name().to_string_lossy().to_string();
                if mod_name.eq_ignore_ascii_case("PalSchema") || mod_name.eq_ignore_ascii_case("shared") {
                    continue;
                }

                let src_mod_path = entry.path();
                let had_enabled_txt = src_mod_path.join("enabled.txt").exists();
                let mods_txt_line = mods_txt_lines.get(&mod_name.to_lowercase()).cloned();

                let dst_mod_path = ue4ss_target_dir.join(&mod_name);
                let _ = crate::profiles::copy_dir_all(&src_mod_path, &dst_mod_path);

                if let Some(rel) = pathdiff::diff_paths(&src_mod_path, game_root) {
                    entries.push(BackupEntry {
                        r#type: "ue4ss_mod".to_string(),
                        mod_name: Some(mod_name.clone()),
                        relative_to_game: rel.to_string_lossy().replace('\\', "/"),
                        zip_path: Some(format!("ue4ss_mods/{}", mod_name)),
                        had_enabled_txt: Some(had_enabled_txt),
                        mods_txt_line,
                        size: None,
                    });
                }
            }
        }
    }

    // 4. Backup PalSchema Mods (PalSchema/mods ONLY — NOT PalSchema/Storage)
    if profile.palschema_mods_dir.exists() {
        let ps_target_dir = temp_staging.join("palschema_mods");
        let _ = fs::create_dir_all(&ps_target_dir);

        if let Ok(rd) = fs::read_dir(&profile.palschema_mods_dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    continue;
                }
                let mod_name = entry.file_name().to_string_lossy().to_string();
                let src_mod_path = entry.path();
                let dst_mod_path = ps_target_dir.join(&mod_name);

                let _ = crate::profiles::copy_dir_all(&src_mod_path, &dst_mod_path);

                if let Some(rel) = pathdiff::diff_paths(&src_mod_path, game_root) {
                    entries.push(BackupEntry {
                        r#type: "palschema_mod".to_string(),
                        mod_name: Some(mod_name.clone()),
                        relative_to_game: rel.to_string_lossy().replace('\\', "/"),
                        zip_path: Some(format!("palschema_mods/{}", mod_name)),
                        had_enabled_txt: None,
                        mods_txt_line: None,
                        size: None,
                    });
                }
            }
        }
    }

    // 5. Pak Mods Manifest (Manifest Only, no physical copy)
    if profile.paks_dir.exists() {
        if let Ok(rd) = fs::read_dir(&profile.paks_dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                    if let Some(rel) = pathdiff::diff_paths(&path, game_root) {
                        entries.push(BackupEntry {
                            r#type: "pak_manifest".to_string(),
                            mod_name: None,
                            relative_to_game: rel.to_string_lossy().replace('\\', "/"),
                            zip_path: None,
                            had_enabled_txt: None,
                            mods_txt_line: None,
                            size: fs::metadata(&path).map(|m| m.len()).ok(),
                        });
                    }
                }
            }
        }
    }

    // Create manifest
    let manifest = SnapshotManifest {
        timestamp: chrono::Utc::now().to_rfc3339(),
        pmm_version: env!("CARGO_PKG_VERSION").to_string(),
        game_root: game_path.to_string(),
        ue4ss_mode: ue4ss_mode_str.to_string(),
        entries,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(temp_staging.join("snapshot_manifest.json"), manifest_json)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    // Compress temp_staging into the destination zip
    let file = File::create(&zip_path)
        .map_err(|e| format!("Failed to create backup zip file at {:?}: {}", zip_path, e))?;
    let mut zip_writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for entry in WalkDir::new(&temp_staging).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == temp_staging {
            continue;
        }
        let rel = pathdiff::diff_paths(path, &temp_staging)
            .ok_or_else(|| "Failed to compute relative path in staging".to_string())?;
        let name = rel.to_string_lossy().replace('\\', "/");

        if path.is_file() {
            zip_writer.start_file(name, options)
                .map_err(|e| format!("Failed to write file to zip: {}", e))?;
            let mut f = File::open(path).map_err(|e| e.to_string())?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
            zip_writer.write_all(&buffer).map_err(|e| e.to_string())?;
        } else if path.is_dir() {
            zip_writer.add_directory(format!("{}/", name), options)
                .map_err(|e| format!("Failed to add directory to zip: {}", e))?;
        }
    }

    zip_writer.finish().map_err(|e| format!("Failed to finalize backup zip: {}", e))?;
    let _ = fs::remove_dir_all(&temp_staging);

    crate::logger::log(&format!("create_initial_safety_backup: Successfully created backup at {:?}", zip_path));
    Ok(true)
}

pub fn restore_initial_safety_backup(game_path: &str, program_path: &str) -> Result<(), String> {
    if game_path.is_empty() {
        return Err("Game path is not set".to_string());
    }
    let game_root = Path::new(game_path);
    if !game_root.exists() {
        return Err("Game directory does not exist".to_string());
    }

    let zip_path = get_backup_zip_path(program_path, game_path);
    if !zip_path.exists() {
        return Err("No initial safety backup found for this game folder".to_string());
    }

    crate::logger::log(&format!("restore_initial_safety_backup: Restoring from {:?}", zip_path));

    let temp_extract = std::env::temp_dir().join(format!("pmm_safety_restore_{}", uuid::Uuid::new_v4()));
    crate::zip_handler::extract_zip_to_temp(&zip_path.to_string_lossy(), &temp_extract)?;

    let manifest_path = temp_extract.join("snapshot_manifest.json");
    if !manifest_path.exists() {
        let _ = fs::remove_dir_all(&temp_extract);
        return Err("Corrupted backup: snapshot_manifest.json is missing".to_string());
    }

    let manifest_str = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: SnapshotManifest = serde_json::from_str(&manifest_str)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    for entry in &manifest.entries {
        if entry.r#type == "pak_manifest" {
            continue;
        }

        let zip_rel = match &entry.zip_path {
            Some(z) => z,
            None => continue,
        };

        let src = temp_extract.join(zip_rel);
        let dst = game_root.join(&entry.relative_to_game);

        if entry.r#type == "file" {
            if src.exists() {
                if let Some(parent) = dst.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::copy(&src, &dst);
            }
        } else if entry.r#type == "ue4ss_mod" || entry.r#type == "palschema_mod" {
            if src.exists() {
                if dst.exists() {
                    let _ = fs::remove_dir_all(&dst);
                }
                if let Some(parent) = dst.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = crate::profiles::copy_dir_all(&src, &dst);

                // Enforce original enabled.txt state if recorded
                if let Some(had_enabled) = entry.had_enabled_txt {
                    let enabled_txt = dst.join("enabled.txt");
                    if had_enabled {
                        if !enabled_txt.exists() {
                            let _ = fs::write(&enabled_txt, "");
                        }
                    } else if enabled_txt.exists() {
                        let _ = fs::remove_file(&enabled_txt);
                    }
                }
            }
        }
    }

    let _ = fs::remove_dir_all(&temp_extract);
    crate::logger::log("restore_initial_safety_backup: Restoration completed successfully");
    Ok(())
}
