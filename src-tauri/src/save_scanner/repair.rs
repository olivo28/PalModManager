use std::fs::{self, File};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::models::SaveRepairResult;
use super::gvas::{decompress_palworld_save, compress_palworld_save};
use super::discovery::extract_metadata_from_world;
use super::deep_scan::list_available_backups;

/// Creates an instant manual backup ZIP of the entire save world
pub fn create_manual_world_backup(world_dir: &str, program_path: &str, custom_dest: Option<&str>) -> Result<String, String> {
    let world_path = Path::new(world_dir);
    if !world_path.exists() {
        return Err("World directory does not exist".to_string());
    }

    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let world_id = world_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "world".to_string());

    let level_meta_sav = world_path.join("LevelMeta.sav");
    let level_sav = world_path.join("Level.sav");
    let (world_name, _, _, _, _, _, _, _) = extract_metadata_from_world(world_path, &level_meta_sav, &level_sav);
    
    let safe_name = if !world_name.is_empty() {
        world_name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect::<String>()
    } else {
        world_id.clone()
    };

    let zip_filename = format!("Backup_{}_{}.zip", safe_name, timestamp);

    let target_zip_path = if let Some(dest) = custom_dest {
        PathBuf::from(dest)
    } else {
        let backups_dir = PathBuf::from(program_path).join("backups").join("worlds");
        fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;
        backups_dir.join(zip_filename)
    };

    let zip_file = File::create(&target_zip_path).map_err(|e| format!("Failed to create zip file: {e}"))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(world_path).into_iter().flatten() {
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(world_path).map_err(|e| e.to_string())?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            // Skip backup subfolder to keep manual ZIP compact and fast
            if rel_str.starts_with("backup/") {
                continue;
            }
            zip_writer.start_file(rel_str, options).map_err(|e| e.to_string())?;
            let mut f = File::open(entry.path()).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut zip_writer).map_err(|e| e.to_string())?;
        }
    }
    zip_writer.finish().map_err(|e| e.to_string())?;

    Ok(target_zip_path.to_string_lossy().to_string())
}

/// Prunes old auto-backups in `backup/world` and `backup/local`, keeping only the newest `keep_count`
pub fn prune_world_backups(world_dir: &str, keep_count: usize) -> Result<usize, String> {
    let world_path = Path::new(world_dir);
    let snapshots = list_available_backups(world_path);
    if snapshots.len() <= keep_count {
        return Ok(0);
    }

    let slots_to_delete = &snapshots[keep_count..];
    let mut deleted = 0;

    for slot in slots_to_delete {
        let w_slot = world_path.join("backup").join("world").join(&slot.slot_name);
        if w_slot.exists() {
            let _ = fs::remove_dir_all(&w_slot);
            deleted += 1;
        }
        let l_slot = world_path.join("backup").join("local").join(&slot.slot_name);
        if l_slot.exists() {
            let _ = fs::remove_dir_all(&l_slot);
        }
    }

    Ok(deleted)
}

/// Restores a world save from an automatic snapshot in `backup/world/<slot>/Level.sav`
pub fn restore_save_from_backup(
    world_dir: &str,
    backup_slot: &str,
    program_path: &str,
) -> Result<SaveRepairResult, String> {
    let world_path = Path::new(world_dir);
    let backup_world_file = world_path.join("backup").join("world").join(backup_slot).join("Level.sav");

    if !backup_world_file.exists() {
        return Err(format!("Backup snapshot '{}' does not exist", backup_slot));
    }

    // 1. Create a safety backup ZIP of the current state before replacing
    let backups_dir = PathBuf::from(program_path).join("backups").join("saves");
    fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let world_id = world_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "world".to_string());
    let backup_zip_name = format!("{}_{}_pre_restore.zip", world_id, timestamp);
    let backup_zip_path = backups_dir.join(backup_zip_name);

    {
        let zip_file = File::create(&backup_zip_path).map_err(|e| format!("Failed to create backup zip: {e}"))?;
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for entry in WalkDir::new(world_path).into_iter().flatten() {
            if entry.file_type().is_file() {
                let rel = entry.path().strip_prefix(world_path).map_err(|e| e.to_string())?;
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if rel_str.starts_with("backup/") {
                    continue;
                }
                zip_writer.start_file(rel_str, options).map_err(|e| e.to_string())?;
                let mut f = File::open(entry.path()).map_err(|e| e.to_string())?;
                std::io::copy(&mut f, &mut zip_writer).map_err(|e| e.to_string())?;
            }
        }
        zip_writer.finish().map_err(|e| e.to_string())?;
    }

    // 2. Restore Level.sav
    let dest_level_sav = world_path.join("Level.sav");
    fs::copy(&backup_world_file, &dest_level_sav)
        .map_err(|e| format!("Failed to restore Level.sav: {e}"))?;

    // 3. Restore LocalData.sav if snapshot has one
    let backup_local_file = world_path.join("backup").join("local").join(backup_slot).join("LocalData.sav");
    if backup_local_file.exists() {
        let dest_local_sav = world_path.join("LocalData.sav");
        let _ = fs::copy(&backup_local_file, &dest_local_sav);
    }

    Ok(SaveRepairResult {
        success: true,
        backup_zip_path: backup_zip_path.to_string_lossy().to_string(),
        sanitized_refs_count: 0,
        message: format!("Successfully restored world from snapshot '{}'.", backup_slot),
    })
}

/// Sanitizes orphaned mod references in `Level.sav` and creates a safety backup zip before saving
pub fn repair_and_sanitize_save(world_dir: &str, program_path: &str) -> Result<SaveRepairResult, String> {
    let world_path = Path::new(world_dir);
    let level_sav = world_path.join("Level.sav");

    if !level_sav.exists() {
        return Err("Level.sav does not exist".to_string());
    }

    // 1. Create safety backup ZIP in backups folder
    let backups_dir = PathBuf::from(program_path).join("backups").join("saves");
    fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let world_id = world_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "world".to_string());
    let backup_zip_name = format!("{}_{}_pre_repair.zip", world_id, timestamp);
    let backup_zip_path = backups_dir.join(backup_zip_name);

    {
        let zip_file = File::create(&backup_zip_path).map_err(|e| format!("Failed to create backup zip: {e}"))?;
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for entry in WalkDir::new(world_path).into_iter().flatten() {
            if entry.file_type().is_file() {
                let rel = entry.path().strip_prefix(world_path).map_err(|e| e.to_string())?;
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if rel_str.starts_with("backup/") {
                    continue;
                }
                zip_writer.start_file(rel_str, options)
                    .map_err(|e| e.to_string())?;
                let mut f = File::open(entry.path()).map_err(|e| e.to_string())?;
                std::io::copy(&mut f, &mut zip_writer).map_err(|e| e.to_string())?;
            }
        }
        zip_writer.finish().map_err(|e| e.to_string())?;
    }

    // 2. Read and decompress Level.sav
    let raw_bytes = fs::read(&level_sav).map_err(|e| e.to_string())?;
    let mut decompressed = decompress_palworld_save(&raw_bytes)?;

    // 3. Nullify / Clean dangling "/Game/Mods/" paths in raw GVAS bytes
    let mut sanitized_count = 0;
    let target_prefix = b"/Game/Mods/";
    let replacement_prefix = b"/Game/None/";

    let mut i = 0;
    while i + target_prefix.len() <= decompressed.len() {
        if &decompressed[i..i + target_prefix.len()] == target_prefix {
            decompressed[i..i + replacement_prefix.len()].copy_from_slice(replacement_prefix);
            sanitized_count += 1;
            i += target_prefix.len();
        } else {
            i += 1;
        }
    }

    // 4. Recompress into PLZ/ZLIB format
    let recompressed = compress_palworld_save(&decompressed)?;

    // 5. Overwrite Level.sav atomically
    let temp_sav = world_path.join("Level.sav.tmp");
    fs::write(&temp_sav, &recompressed).map_err(|e| format!("Failed to write sanitized temp save: {e}"))?;
    fs::rename(&temp_sav, &level_sav).map_err(|e| format!("Failed to apply sanitized Level.sav: {e}"))?;

    Ok(SaveRepairResult {
        success: true,
        backup_zip_path: backup_zip_path.to_string_lossy().to_string(),
        sanitized_refs_count: sanitized_count,
        message: format!("Successfully sanitized {} orphaned mod references. Safety backup created.", sanitized_count),
    })
}

/// Lists all manual ZIP backups created by PalModManager in `<program_path>/backups/worlds/`
pub fn list_pmm_world_backups(program_path: &str, world_name_filter: Option<&str>) -> Vec<super::models::PmmWorldBackup> {
    let backups_dir = PathBuf::from(program_path).join("backups").join("worlds");
    if !backups_dir.exists() {
        return Vec::new();
    }

    let mut list = Vec::new();
    let entries = match fs::read_dir(&backups_dir) {
        Ok(e) => e,
        Err(_) => return list,
    };

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() && p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("zip")) {
            let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            let meta = match p.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let created_at = meta.modified()
                .ok()
                .and_then(|t| {
                    let dt: chrono::DateTime<chrono::Local> = t.into();
                    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
                })
                .unwrap_or_default();

            // Extract world name hint from "Backup_<WorldName>_<Timestamp>.zip"
            let world_hint = if fname.starts_with("Backup_") {
                let stripped = &fname[7..]; // Strip "Backup_"
                if let Some(last_underscore) = stripped.rfind('_') {
                    // check if there is another timestamp part
                    let candidate = &stripped[..last_underscore];
                    if let Some(second_last) = candidate.rfind('_') {
                        Some(candidate[..second_last].to_string())
                    } else {
                        Some(candidate.to_string())
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(filter) = world_name_filter {
                if !filter.is_empty() {
                    let filter_clean = filter.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect::<String>();
                    if !fname.to_lowercase().contains(&filter.to_lowercase()) && !fname.to_lowercase().contains(&filter_clean.to_lowercase()) {
                        continue;
                    }
                }
            }

            list.push(super::models::PmmWorldBackup {
                file_name: fname,
                file_path: p.to_string_lossy().to_string(),
                file_size_bytes: meta.len(),
                created_at,
                world_name_hint: world_hint,
            });
        }
    }

    // Sort newest first by created_at or filename
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    list
}

/// Restores a world save from a PMM manual ZIP backup
pub fn restore_pmm_world_backup(
    world_dir: &str,
    backup_file_path: &str,
    program_path: &str,
) -> Result<SaveRepairResult, String> {
    let world_path = Path::new(world_dir);
    if !world_path.exists() {
        return Err("Target world directory does not exist".to_string());
    }

    let backup_path = Path::new(backup_file_path);
    if !backup_path.exists() || !backup_path.is_file() {
        return Err(format!("Backup archive '{}' does not exist", backup_file_path));
    }

    // 1. Create a safety backup ZIP of the active state before replacing
    let pre_restore_dir = PathBuf::from(program_path).join("backups").join("saves");
    fs::create_dir_all(&pre_restore_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let world_id = world_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "world".to_string());
    let safety_zip_name = format!("{}_{}_pre_pmm_restore.zip", world_id, timestamp);
    let safety_zip_path = pre_restore_dir.join(safety_zip_name);

    {
        let zip_file = File::create(&safety_zip_path).map_err(|e| format!("Failed to create safety backup zip: {e}"))?;
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for entry in WalkDir::new(world_path).into_iter().flatten() {
            if entry.file_type().is_file() {
                let rel = entry.path().strip_prefix(world_path).map_err(|e| e.to_string())?;
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if rel_str.starts_with("backup/") {
                    continue;
                }
                zip_writer.start_file(rel_str, options).map_err(|e| e.to_string())?;
                let mut f = File::open(entry.path()).map_err(|e| e.to_string())?;
                std::io::copy(&mut f, &mut zip_writer).map_err(|e| e.to_string())?;
            }
        }
        zip_writer.finish().map_err(|e| e.to_string())?;
    }

    // 2. Unpack backup zip into world_path
    let file = File::open(backup_path).map_err(|e| format!("Failed to open backup archive: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {e}"))?;

    for i in 0..archive.len() {
        let mut item = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = match item.enclosed_name() {
            Some(path) => world_path.join(path),
            None => continue,
        };

        if item.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut item, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    crate::logger::log(&format!("restore_pmm_world_backup: Successfully restored world at '{}' from '{}'", world_path.display(), backup_path.display()));

    Ok(SaveRepairResult {
        success: true,
        backup_zip_path: safety_zip_path.to_string_lossy().to_string(),
        sanitized_refs_count: 0,
        message: format!("Successfully restored world from PMM backup '{}'.", backup_path.file_name().unwrap_or_default().to_string_lossy()),
    })
}

/// Safely deletes a PMM manual backup ZIP
pub fn delete_pmm_world_backup(backup_file_path: &str) -> Result<(), String> {
    let path = Path::new(backup_file_path);
    if !path.exists() {
        return Err("Backup file does not exist".to_string());
    }

    // Security check: ensure file is a .zip
    if !path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("zip")) {
        return Err("File is not a valid zip archive".to_string());
    }

    fs::remove_file(path).map_err(|e| format!("Failed to delete backup file: {e}"))?;
    crate::logger::log(&format!("delete_pmm_world_backup: Deleted backup '{}'", path.display()));
    Ok(())
}
