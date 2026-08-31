use std::fs;
use std::path::Path;
use std::collections::HashSet;
use walkdir::WalkDir;

use super::models::{
    SaveBackupSnapshot, ExternalEditDiagnostic, PlayerSaveInfo,
    SaveStorageBreakdown, OrphanedModRef, SaveHealthReport
};
use super::gvas::{decompress_palworld_save, gvas_read_str, gvas_read_int};
use super::discovery::{
    extract_metadata_from_world, load_world_custom_meta, parse_world_options
};

pub fn format_snapshot_timestamp(slot_name: &str) -> String {
    let clean = slot_name.trim_end_matches(".sav");
    if let Some((date_part, time_part)) = clean.split_once('-') {
        let d = date_part.replace('.', "-");
        let t = time_part.replace('.', ":");
        format!("{} {}", d, t)
    } else {
        clean.to_string()
    }
}

/// Lists all auto-backup snapshots stored by Palworld in `backup/world/` (lightweight, zero decompression)
pub fn list_available_backups(world_dir: &Path) -> Vec<SaveBackupSnapshot> {
    let mut snapshots = Vec::new();
    let b_world = world_dir.join("backup").join("world");
    if b_world.exists() {
        if let Ok(entries) = fs::read_dir(b_world) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let slot_name = entry.file_name().to_string_lossy().to_string();
                    let level_file = entry.path().join("Level.sav");
                    if level_file.exists() {
                        let level_size = fs::metadata(&level_file).map(|m| m.len()).unwrap_or(0);
                        let local_exists = world_dir.join("backup").join("local").join(&slot_name).join("LocalData.sav").exists();
                        let formatted_time = format_snapshot_timestamp(&slot_name);

                        snapshots.push(SaveBackupSnapshot {
                            slot_name,
                            timestamp: formatted_time,
                            level_size_bytes: level_size,
                            uncompressed_size_bytes: None,
                            local_data_exists: local_exists,
                            in_game_day: None,
                            player_level: None,
                            host_player_name: None,
                            mod_refs_count: None,
                            is_clean_vanilla: None,
                        });
                    }
                }
            }
        }
    }
    snapshots.sort_by(|a, b| b.slot_name.cmp(&a.slot_name));
    snapshots
}

/// On-demand deep inspection of a single snapshot's internal GVAS properties
pub fn inspect_snapshot_details(world_dir: &Path, slot_name: &str) -> Result<SaveBackupSnapshot, String> {
    let snapshot_dir = world_dir.join("backup").join("world").join(slot_name);
    let level_file = snapshot_dir.join("Level.sav");
    let meta_file = snapshot_dir.join("LevelMeta.sav");

    if !level_file.exists() {
        return Err("Snapshot Level.sav not found".to_string());
    }

    let level_size = fs::metadata(&level_file).map(|m| m.len()).unwrap_or(0);
    let local_exists = world_dir.join("backup").join("local").join(slot_name).join("LocalData.sav").exists();
    let formatted_time = format_snapshot_timestamp(slot_name);

    let (_, in_game_day, player_level, host_player_name, _, _, _, _) =
        extract_metadata_from_world(&snapshot_dir, &meta_file, &level_file);

    let mut uncompressed_size_bytes = None;
    let mut mod_refs_count = None;
    let mut is_clean_vanilla = None;

    if let Ok(raw) = fs::read(&level_file) {
        if let Ok(decompressed) = decompress_palworld_save(&raw) {
            uncompressed_size_bytes = Some(decompressed.len() as u64);
            let text = String::from_utf8_lossy(&decompressed);
            let mut count = 0;
            for line in text.split(|c: char| c == '\0' || c == '\n' || c == '\r' || c == '\"' || c == '\'') {
                let trimmed = line.trim();
                if (trimmed.starts_with("/Game/Mods/") || trimmed.contains("/Mods/") || (trimmed.starts_with("/Game/") && !trimmed.starts_with("/Game/Pal/") && !trimmed.starts_with("/Game/Characters/") && !trimmed.starts_with("/Game/Maps/") && !trimmed.starts_with("/Game/Sound/"))) && trimmed.len() > 10 {
                    count += 1;
                }
            }
            mod_refs_count = Some(count);
            is_clean_vanilla = Some(count == 0);
        }
    }

    Ok(SaveBackupSnapshot {
        slot_name: slot_name.to_string(),
        timestamp: formatted_time,
        level_size_bytes: level_size,
        uncompressed_size_bytes,
        local_data_exists: local_exists,
        in_game_day,
        player_level,
        host_player_name,
        mod_refs_count,
        is_clean_vanilla,
    })
}

/// Detects if the save was modified by external tools (e.g. Pal Editor) or suffered size anomaly
pub fn detect_external_edits_and_anomalies(world_dir: &Path, level_size: u64) -> Option<ExternalEditDiagnostic> {
    let world_id = world_dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let mut tool_name = None;
    let mut editor_backup_count = 0;
    let mut has_editor = false;

    if let Some(parent) = world_dir.parent() {
        let editor_backup_dir = parent.join(".Palworld-Pal-Editor-Backup").join(&world_id);
        if editor_backup_dir.exists() {
            has_editor = true;
            tool_name = Some("Palworld Pal Editor".to_string());
            if let Ok(rd) = fs::read_dir(&editor_backup_dir) {
                editor_backup_count = rd.flatten().filter(|e| e.path().is_dir()).count();
            }
        }
    }

    let backups = list_available_backups(world_dir);
    let latest_backup = backups.first();
    let latest_size = latest_backup.map(|b| b.level_size_bytes).unwrap_or(0);
    let mut size_reduction_pct = None;
    let mut has_size_drop = false;

    if latest_size > 0 && level_size > 0 && latest_size > level_size {
        let diff = latest_size - level_size;
        if diff > (latest_size / 10) {
            has_size_drop = true;
            size_reduction_pct = Some(((diff * 100) / latest_size) as u32);
        }
    }

    if has_editor || has_size_drop {
        let details = if has_editor && has_size_drop {
            format!(
                "Save modified with Palworld Pal Editor ({} editor backups found). Current save ({} KB) is {}% smaller than the latest game auto-backup ({} KB).",
                editor_backup_count,
                level_size / 1024,
                size_reduction_pct.unwrap_or(0),
                latest_size / 1024
            )
        } else if has_editor {
            format!(
                "Save modified with Palworld Pal Editor ({} editor backup folders found).",
                editor_backup_count
            )
        } else {
            format!(
                "Severe file size anomaly: Current save ({} KB) is {}% smaller than the latest game auto-backup ({} KB).",
                level_size / 1024,
                size_reduction_pct.unwrap_or(0),
                latest_size / 1024
            )
        };

        Some(ExternalEditDiagnostic {
            is_modified: true,
            tool_name,
            details,
            editor_backup_count,
            current_size_bytes: level_size,
            latest_backup_size_bytes: latest_size,
            size_reduction_pct,
        })
    } else {
        None
    }
}

/// Parses player files in `Players/` to build player roster with size, timestamps, and level
pub fn parse_player_roster(world_dir: &Path, host_player_uid: Option<&str>) -> Vec<PlayerSaveInfo> {
    let players_dir = world_dir.join("Players");
    let mut roster = Vec::new();
    if !players_dir.exists() {
        return roster;
    }

    if let Ok(entries) = fs::read_dir(&players_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("sav")) {
                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                
                let last_played_date = entry.metadata().ok().and_then(|m| m.modified().ok()).map(|time| {
                    let dt: chrono::DateTime<chrono::Local> = time.into();
                    dt.format("%Y-%m-%d %H:%M").to_string()
                });

                let is_host = stem == "00000000000000000000000000000001" || host_player_uid.map_or(false, |h| h.eq_ignore_ascii_case(&stem));

                let mut is_corrupt = false;
                let mut player_name = None;
                let mut player_level = None;

                if let Ok(bytes) = fs::read(&path) {
                    match decompress_palworld_save(&bytes) {
                        Ok(decompressed) => {
                            let start = decompressed.windows(4).position(|w| w == b"GVAS").unwrap_or(0);
                            player_name = gvas_read_str(&decompressed, "NickName", start);
                            for candidate in &["Level", "PlayerLevel"] {
                                if let Some(lv) = gvas_read_int(&decompressed, candidate, start) {
                                    if lv > 0 && lv <= 65 {
                                        player_level = Some(lv as u32);
                                        break;
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            is_corrupt = true;
                        }
                    }
                } else {
                    is_corrupt = true;
                }

                roster.push(PlayerSaveInfo {
                    player_uid: stem,
                    player_name,
                    player_level,
                    is_host,
                    file_size_bytes: file_size,
                    last_played_date,
                    is_corrupt,
                });
            }
        }
    }

    roster.sort_by(|a, b| {
        if a.is_host && !b.is_host {
            std::cmp::Ordering::Less
        } else if !a.is_host && b.is_host {
            std::cmp::Ordering::Greater
        } else {
            b.file_size_bytes.cmp(&a.file_size_bytes)
        }
    });

    roster
}

/// Calculates disk and uncompressed storage breakdown for a save world
pub fn calculate_storage_breakdown(world_dir: &Path, uncompressed_level_bytes: u64) -> SaveStorageBreakdown {
    let level_sav = world_dir.join("Level.sav");
    let level_sav_bytes = level_sav.metadata().map(|m| m.len()).unwrap_or(0);

    let mut players_dir_bytes = 0;
    let players_dir = world_dir.join("Players");
    if players_dir.exists() {
        for entry in WalkDir::new(players_dir).into_iter().flatten() {
            if entry.file_type().is_file() {
                players_dir_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }

    let mut backups_dir_bytes = 0;
    let backups_dir = world_dir.join("backup");
    if backups_dir.exists() {
        for entry in WalkDir::new(backups_dir).into_iter().flatten() {
            if entry.file_type().is_file() {
                backups_dir_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }

    let mut total_world_bytes = 0;
    for entry in WalkDir::new(world_dir).into_iter().flatten() {
        if entry.file_type().is_file() {
            total_world_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }

    let compression_ratio_pct = if uncompressed_level_bytes > 0 && level_sav_bytes > 0 {
        ((level_sav_bytes as f64 / uncompressed_level_bytes as f64) * 100.0) as u32
    } else {
        100
    };

    SaveStorageBreakdown {
        level_sav_bytes,
        players_dir_bytes,
        backups_dir_bytes,
        total_world_bytes,
        uncompressed_level_bytes,
        compression_ratio_pct,
    }
}

/// Deep inspection of a World Save (analyzes Level.sav for orphaned mod assets, external editor damage, and auto-backups)
pub fn deep_scan_save(world_dir: &str, active_installed_mods: &[String], program_path: Option<&str>) -> Result<SaveHealthReport, String> {
    let world_path = Path::new(world_dir);
    let level_sav = world_path.join("Level.sav");
    let level_meta_sav = world_path.join("LevelMeta.sav");

    if !level_sav.exists() {
        return Err("Level.sav not found in specified world directory".to_string());
    }

    let (
        world_name,
        in_game_day,
        player_level,
        host_player_name,
        host_player_uid,
        backup_count,
        latest_backup_date,
        has_external_edits,
    ) = extract_metadata_from_world(world_path, &level_meta_sav, &level_sav);

    let raw_bytes = fs::read(&level_sav).map_err(|e| format!("Failed to read Level.sav: {e}"))?;
    let (decompressed, compression_type) = if raw_bytes.len() > 12 && raw_bytes.get(8..11) == Some(b"PlM") {
        (
            decompress_palworld_save(&raw_bytes).map_err(|e| format!("Oodle PlM Decompress Error: {e}")),
            "Oodle PlM (UE5 GVAS)".to_string(),
        )
    } else if raw_bytes.len() > 12 && (raw_bytes.get(8..12) == Some(b"PlZ\0") || raw_bytes.get(8..12) == Some(b"PLZ\0")) {
        (
            decompress_palworld_save(&raw_bytes).map_err(|e| format!("Zlib PlZ Decompress Error: {e}")),
            "Zlib PlZ (UE5 GVAS)".to_string(),
        )
    } else {
        (
            decompress_palworld_save(&raw_bytes).map_err(|e| format!("Container Decompress Error: {e}")),
            "Raw UE5 GVAS".to_string(),
        )
    };

    let available_backups = list_available_backups(world_path);
    let can_restore_backup = !available_backups.is_empty();

    let pmm_backups = if let Some(prog_p) = program_path {
        super::repair::list_pmm_world_backups(prog_p, Some(&world_name))
    } else {
        Vec::new()
    };
    let pmm_backup_count = pmm_backups.len();

    let decompressed_bytes = match decompressed {
        Ok(d) => d,
        Err(e) => {
            return Ok(SaveHealthReport {
                world_id: world_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
                world_name,
                level_sav_path: level_sav.to_string_lossy().to_string(),
                host_player_name,
                host_player_uid: host_player_uid.clone(),
                player_level,
                in_game_day,
                is_valid_gvas: false,
                compression_type,
                uncompressed_size: 0,
                health_status: "corrupt".to_string(),
                summary_message: format!("Decompression failed ({e}). Save container is corrupt or truncated."),
                orphaned_mod_refs: Vec::new(),
                raw_mod_paths_found: Vec::new(),
                total_mod_references: 0,
                backup_count,
                pmm_backup_count,
                latest_backup_date,
                available_backups,
                pmm_backups,
                has_external_edits,
                external_edit_details: None,
                can_repair: false,
                can_restore_backup,
                world_options: parse_world_options(world_path),
                player_roster: parse_player_roster(world_path, host_player_uid.as_deref()),
                storage_breakdown: Some(calculate_storage_breakdown(world_path, 0)),
                custom_meta: Some(load_world_custom_meta(world_path)),
            });
        }
    };

    let is_valid_gvas = decompressed_bytes.starts_with(b"GVAS") || decompressed_bytes.windows(4).take(32).any(|w| w == b"GVAS");
    let uncompressed_size = decompressed_bytes.len() as u64;

    let text = String::from_utf8_lossy(&decompressed_bytes);
    let mut raw_mod_paths: Vec<String> = Vec::new();
    let mut mod_ref_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for line in text.split(|c: char| c == '\0' || c == '\n' || c == '\r' || c == '\"' || c == '\'') {
        let trimmed = line.trim();
        if (trimmed.starts_with("/Game/Mods/") || trimmed.contains("/Mods/") || (trimmed.starts_with("/Game/") && !trimmed.starts_with("/Game/Pal/") && !trimmed.starts_with("/Game/Characters/") && !trimmed.starts_with("/Game/Maps/") && !trimmed.starts_with("/Game/Sound/"))) && trimmed.len() > 10 {
            raw_mod_paths.push(trimmed.to_string());
            *mod_ref_counts.entry(trimmed.to_string()).or_insert(0) += 1;
        }
    }

    let mut orphaned_mod_refs: Vec<OrphanedModRef> = Vec::new();
    let active_mods_lower: HashSet<String> = active_installed_mods.iter().map(|m| m.to_lowercase()).collect();

    for (path, count) in mod_ref_counts {
        let parts: Vec<&str> = path.split('/').collect();
        let mod_hint = if parts.len() >= 4 && parts[1] == "Game" && parts[2] == "Mods" {
            parts[3].to_string()
        } else if parts.len() >= 3 && parts[1] == "Game" {
            parts[2].to_string()
        } else {
            "CustomMod".to_string()
        };

        let is_currently_installed = active_mods_lower.iter().any(|m| m.contains(&mod_hint.to_lowercase()) || mod_hint.to_lowercase().contains(m));

        if !is_currently_installed {
            orphaned_mod_refs.push(OrphanedModRef {
                mod_hint_name: mod_hint,
                asset_path: path,
                occurrences: count,
            });
        }
    }

    let mut corrupt_player_files: Vec<String> = Vec::new();
    let players_dir = world_path.join("Players");
    if players_dir.exists() {
        if let Ok(rd) = fs::read_dir(&players_dir) {
            for entry in rd.flatten() {
                if entry.path().extension().map_or(false, |e| e.eq_ignore_ascii_case("sav")) {
                    if let Ok(b) = fs::read(entry.path()) {
                        if decompress_palworld_save(&b).is_err() {
                            corrupt_player_files.push(entry.file_name().to_string_lossy().to_string());
                        }
                    } else {
                        corrupt_player_files.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    let external_edit_details = detect_external_edits_and_anomalies(world_path, raw_bytes.len() as u64);
    let has_ext_edits = external_edit_details.is_some();

    let total_refs = raw_mod_paths.len();
    let has_orphans = !orphaned_mod_refs.is_empty();
    let has_corrupt_players = !corrupt_player_files.is_empty();

    let (health_status, summary_message) = if !is_valid_gvas {
        ("corrupt".to_string(), "Save does not have a valid GVAS header (Truncated or unreadable container).".to_string())
    } else if has_corrupt_players {
        ("corrupt".to_string(), format!("Corrupted character save file(s) detected: {}. Game will crash when player joins.", corrupt_player_files.join(", ")))
    } else if has_ext_edits && has_orphans {
        ("warning".to_string(), format!("External tool modification detected AND {} orphaned mod reference(s) found.", orphaned_mod_refs.len()))
    } else if has_ext_edits {
        ("external_edits".to_string(), "External tool modification and character state desynchronization detected. No mod class issues found.".to_string())
    } else if has_orphans {
        ("warning".to_string(), format!("Found {} orphaned asset reference(s) from uninstalled mods that may cause world load crashes.", orphaned_mod_refs.len()))
    } else {
        ("healthy".to_string(), "Save file is healthy. GVAS structure is valid and no orphaned mod references detected.".to_string())
    };

    let world_options = parse_world_options(world_path);
    let player_roster = parse_player_roster(world_path, host_player_uid.as_deref());
    let storage_breakdown = Some(calculate_storage_breakdown(world_path, uncompressed_size));
    let custom_meta = Some(load_world_custom_meta(world_path));

    Ok(SaveHealthReport {
        world_id: world_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        world_name,
        level_sav_path: level_sav.to_string_lossy().to_string(),
        host_player_name,
        host_player_uid,
        player_level,
        in_game_day,
        is_valid_gvas,
        compression_type,
        uncompressed_size,
        health_status,
        summary_message,
        orphaned_mod_refs,
        raw_mod_paths_found: raw_mod_paths,
        total_mod_references: total_refs,
        backup_count,
        pmm_backup_count,
        latest_backup_date,
        available_backups,
        pmm_backups,
        has_external_edits: has_ext_edits,
        external_edit_details,
        can_repair: has_orphans && is_valid_gvas,
        can_restore_backup,
        world_options,
        player_roster,
        storage_breakdown,
        custom_meta,
    })
}
