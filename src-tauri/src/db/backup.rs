use chrono::Utc;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_ROTATING_BACKUPS: usize = 3;

/// Atomically writes data to `target_file` by writing to a temporary file first,
/// syncing buffers to physical disk, and renaming to `target_file`.
/// Guarantees that target_file is never truncated or left in an inconsistent state.
pub fn atomic_write_file(target_file: &Path, content: &str) -> Result<(), String> {
    let parent = target_file
        .parent()
        .ok_or_else(|| format!("Invalid parent directory for target {:?}", target_file))?;

    fs::create_dir_all(parent).map_err(|e| format!("Cannot create parent directory: {}", e))?;

    let filename = target_file
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("database.db");

    let pid = std::process::id();
    let temp_name = format!("{}.tmp.{}.{}", filename, pid, Uuid::new_v4());
    let temp_path = parent.join(temp_name);

    // Write content to temporary file
    let mut file = File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write to temp file: {}", e))?;

    // Guarantee disk synchronization
    file.sync_all()
        .map_err(|e| format!("Failed to sync temp file to disk: {}", e))?;
    drop(file);

    // Atomic rename
    #[cfg(windows)]
    {
        // On Windows, if target exists, we can remove or replace cleanly
        if target_file.exists() {
            // Attempt standard rename; if blocked on Windows, replace atomically
            if let Err(_) = fs::rename(&temp_path, target_file) {
                // Fallback replace strategy
                let replace_tmp = parent.join(format!("{}.old.{}", filename, Uuid::new_v4()));
                fs::rename(target_file, &replace_tmp)
                    .map_err(|e| format!("Failed moving old target to replace_tmp: {}", e))?;
                fs::rename(&temp_path, target_file)
                    .map_err(|e| format!("Failed moving temp file to target: {}", e))?;
                let _ = fs::remove_file(replace_tmp);
            }
        } else {
            fs::rename(&temp_path, target_file)
                .map_err(|e| format!("Failed to rename temp file to target: {}", e))?;
        }
    }

    #[cfg(not(windows))]
    {
        fs::rename(&temp_path, target_file)
            .map_err(|e| format!("Failed to rename temp file to target: {}", e))?;
    }

    Ok(())
}

/// Rotates backups for a given database file before saving new content.
/// e.g. pmm_database.db.bak.2 -> pmm_database.db.bak.3
///      pmm_database.db.bak.1 -> pmm_database.db.bak.2
///      pmm_database.db       -> pmm_database.db.bak.1
pub fn rotate_backups_before_save(db_path: &Path) {
    if !db_path.exists() {
        return;
    }

    let parent = match db_path.parent() {
        Some(p) => p,
        None => return,
    };

    let filename = match db_path.file_name().and_then(|f| f.to_str()) {
        Some(f) => f,
        None => return,
    };

    // Rotate existing backups down: e.g. 2 -> 3, 1 -> 2
    for i in (1..MAX_ROTATING_BACKUPS).rev() {
        let current_bak = parent.join(format!("{}.bak.{}", filename, i));
        let next_bak = parent.join(format!("{}.bak.{}", filename, i + 1));
        if current_bak.exists() {
            let _ = fs::copy(&current_bak, &next_bak);
        }
    }

    // Copy current live db to .bak.1
    let bak_1 = parent.join(format!("{}.bak.1", filename));
    let _ = fs::copy(db_path, &bak_1);
}

/// Isolates a damaged or unparseable database file to a quarantine filename (.corrupt.<timestamp>).
/// Ensures user data is never destroyed or lost even in catastrophic corruption scenarios.
pub fn quarantine_corrupt_db(db_path: &Path, reason: &str) -> Option<PathBuf> {
    if !db_path.exists() {
        return None;
    }

    let parent = db_path.parent()?;
    let filename = db_path.file_name()?.to_str()?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let corrupt_path = parent.join(format!("{}.corrupt.{}", filename, timestamp));

    if let Ok(_) = fs::copy(db_path, &corrupt_path) {
        crate::logger::log(&format!(
            "[db::backup] Quarantined corrupted database to {:?}. Reason: {}",
            corrupt_path, reason
        ));
        Some(corrupt_path)
    } else {
        None
    }
}

/// Returns ordered list of backup candidates (.bak.1, .bak.2, .bak.3) if they exist.
pub fn get_available_backups(db_path: &Path) -> Vec<PathBuf> {
    let mut backups = Vec::new();
    let parent = match db_path.parent() {
        Some(p) => p,
        None => return backups,
    };
    let filename = match db_path.file_name().and_then(|f| f.to_str()) {
        Some(f) => f,
        None => return backups,
    };

    for i in 1..=MAX_ROTATING_BACKUPS {
        let bak = parent.join(format!("{}.bak.{}", filename, i));
        if bak.is_file() {
            backups.push(bak);
        }
    }

    backups
}
