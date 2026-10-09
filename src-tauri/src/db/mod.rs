use std::fs;
use std::path::{Path, PathBuf};
use crate::models::AppData;
use crate::profiles;

pub mod crypto;
pub mod backup;
pub mod legacy;

pub use crypto::*;
pub use backup::*;
pub use legacy::*;

pub const DB_FILENAME: &str = "pmm_database.db";
pub const LEGACY_DB_FILENAME: &str = "mod-manager.db";

pub fn get_db_path(program_path: &str) -> PathBuf {
    PathBuf::from(program_path).join(DB_FILENAME)
}

/// Attempts to parse raw string (plain JSON or "enc:...") into AppData.
fn parse_db_contents(contents: &str, program_dir: &Path) -> Result<(AppData, bool), String> {
    let (json, was_migrated) = if contents.starts_with("enc:") {
        crypto::decrypt_string(contents, program_dir)?
    } else {
        (contents.to_string(), false)
    };

    match serde_json::from_str::<AppData>(&json) {
        Ok(data) => Ok((data, was_migrated)),
        Err(e) => {
            // Attempt fallback to legacy field structure
            if let Some(data) = legacy::convert_old_format(&json) {
                Ok((data, true))
            } else {
                Err(format!("JSON deserialization failed: {}", e))
            }
        }
    }
}

/// Loads database with multi-layer resilience:
/// 1. Tries primary pmm_database.db
/// 2. If damaged, isolates to .corrupt.<timestamp> and attempts to recover from .bak.1 -> .bak.2 -> .bak.3
/// 3. Migrates legacy mod-manager.db or mod-manager.json if first time boot
/// 4. Never blindly overwrites user data with empty AppData
pub fn load_db(program_path: &str) -> AppData {
    let program_dir = Path::new(program_path);
    let db_path = get_db_path(program_path);

    // Initial first-time checks: migrate legacy DB filenames
    if !db_path.exists() {
        let legacy_db = program_dir.join(LEGACY_DB_FILENAME);
        if legacy_db.exists() {
            let _ = fs::rename(&legacy_db, &db_path);
        }
    }

    if !db_path.exists() {
        let legacy_json = program_dir.join("mod-manager.json");
        if legacy_json.exists() {
            let mut data = migrate_from_json_file(&legacy_json, program_path);
            profiles::ensure_default_profile(&mut data);
            return data;
        }
        let mut data = AppData::default();
        profiles::ensure_default_profile(&mut data);
        return data;
    }

    // Read live database
    let live_read = fs::read_to_string(&db_path);
    match live_read {
        Ok(contents) => {
            match parse_db_contents(&contents, program_dir) {
                Ok((mut data, needs_resave)) => {
                    profiles::ensure_default_profile(&mut data);
                    let current_ver = data.schema_version;
                    let migration_applied = legacy::run_db_migrations(
                        &mut data,
                        current_ver,
                        crate::models::CURRENT_DB_SCHEMA_VERSION,
                    );
                    if needs_resave || migration_applied {
                        let _ = save_db(program_path, &data);
                    }
                    data
                }
                Err(err) => {
                    crate::logger::log(&format!(
                        "[db::mod] CRITICAL: Primary database corrupted or cannot be decrypted: {}. Initiating recovery...",
                        err
                    ));
                    // Quarantine damaged database
                    backup::quarantine_corrupt_db(&db_path, &err);

                    // Attempt recovery from backups
                    let backups = backup::get_available_backups(&db_path);
                    for bak in backups {
                        crate::logger::log(&format!("[db::mod] Attempting restoration from backup: {:?}", bak));
                        if let Ok(bak_contents) = fs::read_to_string(&bak) {
                            if let Ok((mut recovered_data, _)) = parse_db_contents(&bak_contents, program_dir) {
                                crate::logger::log(&format!("[db::mod] SUCCESS: Restored database from backup {:?}", bak));
                                profiles::ensure_default_profile(&mut recovered_data);
                                let _ = save_db(program_path, &recovered_data);
                                return recovered_data;
                            }
                        }
                    }

                    crate::logger::log("[db::mod] ERROR: All database backups exhausted. Starting in default safe state.");
                    let mut fallback = AppData::default();
                    profiles::ensure_default_profile(&mut fallback);
                    fallback
                }
            }
        }
        Err(e) => {
            crate::logger::log(&format!("[db::mod] Failed to read database file: {}. Falling back to default.", e));
            let mut data = AppData::default();
            profiles::ensure_default_profile(&mut data);
            data
        }
    }
}

fn migrate_from_json_file(legacy_path: &Path, program_path: &str) -> AppData {
    match fs::read_to_string(legacy_path) {
        Ok(contents) => {
            match legacy::convert_old_format(&contents) {
                Some(data) => {
                    let _ = save_db(program_path, &data);
                    let _ = fs::remove_file(legacy_path);
                    data
                }
                None => {
                    crate::logger::log("[db::mod] Failed to convert legacy JSON");
                    AppData::default()
                }
            }
        }
        Err(e) => {
            crate::logger::log(&format!("[db::mod] Failed to read legacy file: {}", e));
            AppData::default()
        }
    }
}

/// Saves database with rotating backup preservation and atomic write guarantees.
pub fn save_db(program_path: &str, data: &AppData) -> Result<(), String> {
    let program_dir = Path::new(program_path);
    let db_path = get_db_path(program_path);

    let json = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Serialize error: {}", e))?;

    let v2_key = crypto::derive_key_v2(program_dir);
    let encrypted = crypto::encrypt_with_key(&json, &v2_key);

    // Rotate existing backups before replacing
    backup::rotate_backups_before_save(&db_path);

    // Atomic write to avoid file truncation during power outage or kill
    backup::atomic_write_file(&db_path, &encrypted)?;

    Ok(())
}
