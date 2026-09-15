use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use walkdir::WalkDir;

use super::models::{SaveWorldSummary, WorldCustomMeta, WorldOptionSettings};
use super::gvas::{
    decompress_palworld_save, gvas_read_str, gvas_read_int, gvas_read_int64,
    gvas_read_float, gvas_read_bool, gvas_find_property, gvas_fstring
};
use super::deep_scan::{
    list_available_backups, detect_external_edits_and_anomalies,
    extract_fstring_asset_paths, is_mod_asset_path
};

#[derive(Clone, Debug)]
struct CachedWorldSummary {
    level_mtime: Option<SystemTime>,
    level_size: u64,
    meta_mtime: Option<SystemTime>,
    summary: SaveWorldSummary,
}

static WORLD_CACHE: Mutex<Option<HashMap<String, CachedWorldSummary>>> = Mutex::new(None);

/// Invalidate cache for a specific world or all worlds
pub fn invalidate_save_world_cache(world_dir: Option<&str>) {
    if let Ok(mut lock) = WORLD_CACHE.lock() {
        if let Some(ref mut map) = *lock {
            if let Some(dir) = world_dir {
                map.remove(dir);
            } else {
                map.clear();
            }
        }
    }
}

/// Detects default Palworld save game folders on Windows and Linux/Proton
pub fn detect_palworld_save_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let steam_saves = PathBuf::from(local_appdata).join("Pal").join("Saved").join("SaveGames");
            if steam_saves.exists() {
                candidates.push(steam_saves);
            }
        }
        // Game Pass saves
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let wgs = PathBuf::from(local_appdata).join("Packages");
            if wgs.exists() {
                for entry in WalkDir::new(wgs).max_depth(3).into_iter().flatten() {
                    if entry.file_name().to_string_lossy().contains("Pocketpair") {
                        let sys_save = entry.path().join("SystemAppData").join("wgs");
                        if sys_save.exists() {
                            candidates.push(sys_save);
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home)
                .join(".local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames");
            if p.exists() {
                candidates.push(p);
            }
        }
    }

    candidates
}

/// Lists all save game worlds inside a base SaveGames directory or custom path
pub fn list_save_worlds(custom_root: Option<&str>, program_path: Option<&str>) -> Result<Vec<SaveWorldSummary>, String> {
    let root_dirs = if let Some(cr) = custom_root {
        vec![PathBuf::from(cr)]
    } else {
        detect_palworld_save_roots()
    };

    let mut worlds = Vec::new();

    for base_dir in root_dirs {
        if !base_dir.exists() {
            continue;
        }

        for entry in WalkDir::new(&base_dir).min_depth(1).max_depth(3).into_iter().flatten() {
            if entry.file_type().is_dir() {
                let dir_path = entry.path();
                let path_str = dir_path.to_string_lossy();
                if path_str.contains("backup") || path_str.contains("Backup") || path_str.contains(".Palworld") {
                    continue;
                }

                let level_sav = dir_path.join("Level.sav");
                let level_meta_sav = dir_path.join("LevelMeta.sav");

                if level_sav.exists() {
                    let level_size = fs::metadata(&level_sav).map(|m| m.len()).unwrap_or(0);
                    let level_mtime = fs::metadata(&level_sav).and_then(|m| m.modified()).ok();
                    let meta_mtime = fs::metadata(&level_meta_sav).and_then(|m| m.modified()).ok();
                    let world_key = dir_path.to_string_lossy().to_string();

                    // Check in-memory cache to eliminate redundant decompressions
                    let cached_hit = if let Ok(lock) = WORLD_CACHE.lock() {
                        lock.as_ref().and_then(|map| map.get(&world_key)).and_then(|cached| {
                            if cached.level_size == level_size
                                && cached.level_mtime == level_mtime
                                && cached.meta_mtime == meta_mtime
                            {
                                let mut s = cached.summary.clone();
                                if let Some(prog_p) = program_path {
                                    s.pmm_backup_count = super::repair::list_pmm_world_backups(prog_p, Some(&s.world_name)).len();
                                }
                                Some(s)
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    };

                    if let Some(cached_summary) = cached_hit {
                        worlds.push(cached_summary);
                        continue;
                    }

                    let world_id = dir_path.file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "UnknownWorld".to_string());

                    let players_dir = dir_path.join("Players");
                    let player_count = if players_dir.exists() {
                        fs::read_dir(&players_dir)
                            .map(|rd| rd.flatten().filter(|p| p.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("sav"))).count())
                            .unwrap_or(0)
                    } else {
                        0
                    };

                    let (
                        world_name,
                        in_game_day,
                        player_level,
                        host_player_name,
                        host_player_uid,
                        backup_count,
                        latest_backup_date,
                        has_external_edits,
                    ) = extract_metadata_from_world(dir_path, &level_meta_sav, &level_sav);

                    let save_date = fs::metadata(&level_sav)
                        .and_then(|m| m.modified())
                        .ok()
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Local> = t.into();
                            dt.format("%Y-%m-%d %H:%M").to_string()
                        });

                    let pmm_backup_count = if let Some(prog_p) = program_path {
                        super::repair::list_pmm_world_backups(prog_p, Some(&world_name)).len()
                    } else {
                        0
                    };

                    let (health_status, detected_issues) = quick_check_save_health(dir_path, &level_sav);
                    let custom_meta = Some(load_world_custom_meta(dir_path));
                    let world_options = parse_world_options(dir_path);

                    let summary = SaveWorldSummary {
                        world_id,
                        world_name,
                        world_dir: dir_path.to_string_lossy().to_string(),
                        host_player_name,
                        host_player_uid,
                        player_level,
                        in_game_day,
                        save_date,
                        level_size_bytes: level_size,
                        player_count,
                        backup_count,
                        pmm_backup_count,
                        latest_backup_date,
                        has_external_edits,
                        health_status,
                        detected_issues_count: detected_issues,
                        custom_meta,
                        world_options,
                    };

                    if let Ok(mut lock) = WORLD_CACHE.lock() {
                        let map = lock.get_or_insert_with(HashMap::new);
                        map.insert(world_key, CachedWorldSummary {
                            level_mtime,
                            level_size,
                            meta_mtime,
                            summary: summary.clone(),
                        });
                    }

                    crate::logger::log(&format!(
                        "[Save Scanner] World '{}': Level.sav ({:.1} MB), {} players, day {}, health: {}",
                        summary.world_name, summary.level_size_bytes as f64 / (1024.0 * 1024.0),
                        summary.player_count, summary.in_game_day.unwrap_or(0), summary.health_status
                    ));

                    worlds.push(summary);
                }
            }
        }
    }

    Ok(worlds)
}

/// Loads custom world metadata (nickname, notes, bound profile, etc.)
pub fn load_world_custom_meta(world_dir: &Path) -> WorldCustomMeta {
    let meta_file = world_dir.join("palmod_world_meta.json");
    if meta_file.exists() {
        if let Ok(content) = fs::read_to_string(&meta_file) {
            if let Ok(meta) = serde_json::from_str::<WorldCustomMeta>(&content) {
                return meta;
            }
        }
    }
    WorldCustomMeta::default()
}

/// Saves custom world metadata
pub fn save_world_custom_meta(world_dir: &Path, meta: &WorldCustomMeta) -> Result<(), String> {
    let meta_file = world_dir.join("palmod_world_meta.json");
    let json_str = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    fs::write(&meta_file, json_str).map_err(|e| e.to_string())?;
    Ok(())
}

/// Opens the save world folder in the native operating system file explorer
pub fn open_world_folder(world_dir: &str) -> Result<(), String> {
    let path = Path::new(world_dir);
    if !path.exists() {
        return Err("Directory does not exist".to_string());
    }
    crate::system_open::open_path_in_system(path)
}

/// Extracts rich metadata from a Palworld save directory
pub fn extract_metadata_from_world(
    world_dir: &Path,
    meta_path: &Path,
    level_path: &Path,
) -> (
    String,                  // world_name
    Option<u32>,             // in_game_day
    Option<u32>,             // player_level
    Option<String>,          // host_player_name
    Option<String>,          // host_player_uid
    usize,                   // backup_count
    Option<String>,          // latest_backup_date
    bool,                    // has_external_edits
) {
    let mut world_name = String::new();
    let mut day: Option<u32> = None;
    let mut level: Option<u32> = None;
    let mut host_name: Option<String> = None;
    let mut host_uid: Option<String> = None;
    let world_folder_name = world_dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "UnknownWorld".to_string());

    // --- 1. LevelMeta.sav: World Name, Host Player Name & Level ---
    if meta_path.exists() {
        let meta_size = fs::metadata(meta_path).map(|m| m.len()).unwrap_or(0);
        let t0 = std::time::Instant::now();
        crate::logger::log_sav_start(&world_folder_name, "LevelMeta.sav", meta_size, &meta_path.to_string_lossy());
        if let Ok(raw) = fs::read(meta_path) {
            if let Ok(decompressed) = decompress_palworld_save(&raw) {
                let start = decompressed.windows(4).position(|w| w == b"GVAS").unwrap_or(0);
                if let Some(name) = gvas_read_str(&decompressed, "WorldName", start)
                    .or_else(|| gvas_read_str(&decompressed, "SaveDataTitle", start))
                {
                    if !name.is_empty() { world_name = name; }
                }
                if let Some(hname) = gvas_read_str(&decompressed, "HostPlayerName", start) {
                    if !hname.is_empty() && host_name.is_none() { host_name = Some(hname); }
                }
                if let Some(hlevel) = gvas_read_int(&decompressed, "HostPlayerLevel", start) {
                    if hlevel > 0 && hlevel < 1000 && level.is_none() { level = Some(hlevel as u32); }
                }
                if let Some(hday) = gvas_read_int(&decompressed, "InGameDay", start) {
                    if hday > 0 && hday < 1_000_000 && day.is_none() { day = Some(hday as u32); }
                }
                crate::logger::log_debug(&format!(
                    "  [PARSED] WorldName: {:?} | HostPlayer: {:?} | Level: {:?} | Day: {:?}",
                    world_name, host_name, level, day
                ));
            }
        }
        crate::logger::log_sav_finish("LevelMeta.sav", t0.elapsed().as_millis(), "OK");
    }

    // --- 2. Level.sav: InGameDay (GameDateTimeTicks) + Host Player NickName & Level ---
    // Only scan Level.sav if LevelMeta.sav didn't provide complete metadata
    let needs_level_scan = world_name.is_empty() || day.is_none() || host_name.is_none() || level.is_none();
    const MAX_LEVEL_SAV_SCAN: u64 = 40 * 1024 * 1024;
    let level_size = if level_path.exists() {
        fs::metadata(level_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    if needs_level_scan && level_path.exists() && level_size <= MAX_LEVEL_SAV_SCAN {
        let t0 = std::time::Instant::now();
        crate::logger::log_sav_start(&world_folder_name, "Level.sav (Metadata Scan)", level_size, &level_path.to_string_lossy());
        if let Ok(raw) = fs::read(level_path) {
            if let Ok(decompressed) = decompress_palworld_save(&raw) {
                let gvas_start = decompressed.windows(4).position(|w| w == b"GVAS").unwrap_or(0);

                if let Some(ticks) = gvas_read_int64(&decompressed, "GameDateTimeTicks", gvas_start) {
                    if ticks > 0 {
                        let calculated_day = (ticks / 864_000_000_000) as u32 + 1;
                        if calculated_day > 0 && calculated_day < 1_000_000 {
                            day = Some(calculated_day);
                        }
                    }
                }
                if day.is_none() {
                    for candidate in &["DayCount", "InGameDay", "Day", "GameDay"] {
                        if let Some(d) = gvas_read_int(&decompressed, candidate, gvas_start) {
                            if d > 0 && d < 100_000 {
                                day = Some(d as u32);
                                break;
                            }
                        }
                    }
                }

                if let Some((_, nick_pos)) = gvas_find_property(&decompressed, "NickName", gvas_start) {
                    if let Some((s, _)) = gvas_fstring(&decompressed, nick_pos) {
                        if !s.is_empty() && s != "None" {
                            host_name = Some(s);
                        }
                    }

                    let win_start = nick_pos.saturating_sub(4096);
                    let win_end = (nick_pos + 2048).min(decompressed.len());
                    let mut scan = win_start;
                    while scan < win_end {
                        if let Some((_, vpos)) = gvas_find_property(&decompressed, "Level", scan) {
                            if vpos < win_end {
                                if let Some(lv) = gvas_read_int(&decompressed, "Level", scan) {
                                    if lv > 0 && lv <= 65 {
                                        level = Some(lv as u32);
                                        break;
                                    }
                                }
                                scan = vpos + 4;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                crate::logger::log_debug(&format!(
                    "  [PARSED] Day: {:?} | HostPlayer: {:?} | Level: {:?}",
                    day, host_name, level
                ));
            }
        }
        crate::logger::log_sav_finish("Level.sav (Metadata Scan)", t0.elapsed().as_millis(), "OK");
    }

    // --- 3. Players/*.sav: Scan Host UID, NickName and Level ---
    let players_dir = world_dir.join("Players");
    if players_dir.exists() {
        if let Ok(entries) = fs::read_dir(&players_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.extension().map_or(false, |e| e.eq_ignore_ascii_case("sav")) { continue; }
                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if stem == "00000000000000000000000000000000" { continue; }

                if host_uid.is_none() {
                    host_uid = Some(stem.clone());
                }

                // If host_name and level are already known, avoid decompressing player files
                if host_name.is_some() && level.is_some() {
                    continue;
                }

                let p_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let p_label = format!("Players/{}.sav", stem);
                let t0 = std::time::Instant::now();
                crate::logger::log_sav_start(&world_folder_name, &p_label, p_size, &path.to_string_lossy());
                if let Ok(raw) = fs::read(&path) {
                    if let Ok(decompressed) = decompress_palworld_save(&raw) {
                        let start = decompressed.windows(4).position(|w| w == b"GVAS").unwrap_or(0);
                        if host_name.is_none() {
                            if let Some(nick) = gvas_read_str(&decompressed, "NickName", start) {
                                host_name = Some(nick);
                            }
                        }
                        if level.is_none() {
                            for candidate in &["Level", "PlayerLevel"] {
                                if let Some(lv) = gvas_read_int(&decompressed, candidate, start) {
                                    if lv > 0 && lv <= 65 {
                                        level = Some(lv as u32);
                                        break;
                                    }
                                }
                            }
                        }
                        crate::logger::log_debug(&format!(
                            "  [PARSED] NickName: {:?} | Level: {:?}",
                            host_name, level
                        ));
                    }
                }
                crate::logger::log_sav_finish(&p_label, t0.elapsed().as_millis(), "OK");
            }
        }
    }

    // --- 4. Backups count & latest date ---
    let backups = list_available_backups(world_dir);
    let backup_count = backups.len();
    let latest_backup_date = backups.first().map(|b| b.timestamp.clone());

    // --- 5. External edits detection ---
    let has_external_edits = detect_external_edits_and_anomalies(world_dir, level_size).is_some();

    (world_name, day, level, host_name, host_uid, backup_count, latest_backup_date, has_external_edits)
}

/// Quick check on header and decompressed structure
pub fn quick_check_save_health(world_dir: &Path, level_sav: &Path) -> (String, usize) {
    let world_folder_name = world_dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "UnknownWorld".to_string());
    let level_size = fs::metadata(level_sav).map(|m| m.len()).unwrap_or(0);
    let t0 = std::time::Instant::now();
    crate::logger::log_sav_start(&world_folder_name, "Level.sav (Health Verification)", level_size, &level_sav.to_string_lossy());

    let bytes = match fs::read(level_sav) {
        Ok(b) => b,
        Err(_) => {
            crate::logger::log_sav_finish("Level.sav (Health Verification)", t0.elapsed().as_millis(), "FAIL: Cannot read file");
            return ("corrupt".to_string(), 1);
        }
    };

    let decompressed = match decompress_palworld_save(&bytes) {
        Ok(d) => d,
        Err(e) => {
            crate::logger::log_sav_finish("Level.sav (Health Verification)", t0.elapsed().as_millis(), &format!("FAIL: {}", e));
            return ("corrupt".to_string(), 1);
        }
    };

    let is_gvas = decompressed.starts_with(b"GVAS") || decompressed.windows(4).take(32).any(|w| w == b"GVAS");
    if !is_gvas {
        crate::logger::log_sav_finish("Level.sav (Health Verification)", t0.elapsed().as_millis(), "FAIL: Header is not GVAS");
        return ("corrupt".to_string(), 1);
    }

    let has_crit_blocks = decompressed.windows("CharacterSaveParameterMap".len()).any(|w| w == b"CharacterSaveParameterMap")
        && decompressed.windows("GroupSaveDataMap".len()).any(|w| w == b"GroupSaveDataMap");
    if !has_crit_blocks {
        crate::logger::log_sav_finish("Level.sav (Health Verification)", t0.elapsed().as_millis(), "FAIL: Missing critical data blocks");
        return ("corrupt".to_string(), 1);
    }

    crate::logger::log_sav_finish("Level.sav (Health Verification)", t0.elapsed().as_millis(), "OK: Healthy GVAS structure");

    let players_dir = world_dir.join("Players");
    if players_dir.exists() {
        if let Ok(rd) = fs::read_dir(players_dir) {
            for entry in rd.flatten() {
                if entry.path().extension().map_or(false, |e| e.eq_ignore_ascii_case("sav")) {
                    let p_stem = entry.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let p_label = format!("Players/{}.sav (Health Verification)", p_stem);
                    let p_size = fs::metadata(entry.path()).map(|m| m.len()).unwrap_or(0);
                    let pt0 = std::time::Instant::now();
                    crate::logger::log_sav_start(&world_folder_name, &p_label, p_size, &entry.path().to_string_lossy());
                    if let Ok(p_b) = fs::read(entry.path()) {
                        if let Err(e) = decompress_palworld_save(&p_b) {
                            crate::logger::log_sav_finish(&p_label, pt0.elapsed().as_millis(), &format!("FAIL: {}", e));
                            return ("corrupt".to_string(), 1);
                        }
                    }
                    crate::logger::log_sav_finish(&p_label, pt0.elapsed().as_millis(), "OK");
                }
            }
        }
    }

    let paths = extract_fstring_asset_paths(&decompressed);
    let mod_issues = paths.iter().filter(|p| is_mod_asset_path(p)).count();

    let has_ext_edits = detect_external_edits_and_anomalies(world_dir, bytes.len() as u64).is_some();

    if has_ext_edits && mod_issues > 0 {
        ("warning".to_string(), mod_issues)
    } else if has_ext_edits {
        ("external_edits".to_string(), 0)
    } else if mod_issues > 0 {
        ("warning".to_string(), mod_issues)
    } else {
        ("healthy".to_string(), 0)
    }
}

/// Parses world difficulty, multipliers, and rules from `WorldOption.sav`
pub fn parse_world_options(world_dir: &Path) -> Option<WorldOptionSettings> {
    let opt_path = world_dir.join("WorldOption.sav");
    let bytes = if opt_path.exists() {
        fs::read(&opt_path).ok()?
    } else {
        return None;
    };

    let decompressed = decompress_palworld_save(&bytes).unwrap_or(bytes);
    let start = decompressed.windows(4).position(|w| w == b"GVAS").unwrap_or(0);


    let difficulty = gvas_read_str(&decompressed, "Difficulty", start);
    let day_time_speed_rate = gvas_read_float(&decompressed, "DayTimeSpeedRate", start);
    let night_time_speed_rate = gvas_read_float(&decompressed, "NightTimeSpeedRate", start);
    let exp_rate = gvas_read_float(&decompressed, "ExpRate", start);
    let pal_capture_rate = gvas_read_float(&decompressed, "PalCaptureRate", start);
    let pal_spawn_num_rate = gvas_read_float(&decompressed, "PalSpawnNumRate", start);
    // "Attack" suffix confirmed by WOPT-DUMP — older saves may use legacy names as fallback
    let pal_damage_rate = gvas_read_float(&decompressed, "PalDamageRateAttack", start)
        .or_else(|| gvas_read_float(&decompressed, "PalDamageRateMultiplier", start))
        .or_else(|| gvas_read_float(&decompressed, "PalDamageRate", start));
    let player_damage_rate = gvas_read_float(&decompressed, "PlayerDamageRateAttack", start)
        .or_else(|| gvas_read_float(&decompressed, "PlayerDamageRateMultiplier", start))
        .or_else(|| gvas_read_float(&decompressed, "PlayerDamageRate", start));
    let player_stomach_decrease_rate = gvas_read_float(&decompressed, "PlayerStomachDecreaceRate", start);
    let player_stamina_decrease_rate = gvas_read_float(&decompressed, "PlayerStaminaDecreaceRate", start);
    let player_auto_hp_regene_rate = gvas_read_float(&decompressed, "PlayerAutoHPRegeneRate", start);
    // "InSleep" (no trailing "ing") confirmed by WOPT-DUMP
    let player_auto_hp_regene_rate_in_sleeping = gvas_read_float(&decompressed, "PlayerAutoHpRegeneRateInSleep", start)
        .or_else(|| gvas_read_float(&decompressed, "PlayerAutoHpRegeneRateInSleeping", start));
    let pal_stomach_decrease_rate = gvas_read_float(&decompressed, "PalStomachDecreaceRate", start);
    let pal_stamina_decrease_rate = gvas_read_float(&decompressed, "PalStaminaDecreaceRate", start);
    let pal_auto_hp_regene_rate = gvas_read_float(&decompressed, "PalAutoHPRegeneRate", start);
    let pal_auto_hp_regene_rate_in_sleeping = gvas_read_float(&decompressed, "PalAutoHpRegeneRateInSleep", start)
        .or_else(|| gvas_read_float(&decompressed, "PalAutoHpRegeneRateInSleeping", start));
    let build_object_damage_rate = gvas_read_float(&decompressed, "BuildObjectDamageRate", start);
    let build_object_deterioration_damage_rate = gvas_read_float(&decompressed, "BuildObjectDeteriorationDamageRate", start);
    let collection_drop_rate = gvas_read_float(&decompressed, "CollectionDropRate", start);
    let collection_object_hp_rate = gvas_read_float(&decompressed, "CollectionObjectHpRate", start);
    let collection_object_respawn_speed_rate = gvas_read_float(&decompressed, "CollectionObjectRespawnSpeedRate", start);
    let enemy_drop_item_rate = gvas_read_float(&decompressed, "EnemyDropItemRate", start);
    let death_penalty_raw = gvas_read_str(&decompressed, "DeathPenalty", start);
    let death_penalty = death_penalty_raw.map(|d| {
        if d.contains("None") {
            "None".to_string()
        } else if d.contains("ItemAndEquipment") {
            "ItemAndEquipment".to_string()
        } else if d.contains("Item") {
            "Item".to_string()
        } else if d.contains("All") {
            "All".to_string()
        } else {
            d
        }
    });
    let enable_player_to_player_damage = gvas_read_bool(&decompressed, "bEnablePlayerToPlayerDamage", start);
    let enable_friendly_fire = gvas_read_bool(&decompressed, "bEnableFriendlyFire", start);
    let enable_invader_enemy = gvas_read_bool(&decompressed, "bEnableInvaderEnemy", start);
    let active_unko = gvas_read_bool(&decompressed, "bActiveUNKO", start);
    let drop_item_max_num = gvas_read_int(&decompressed, "DropItemMaxNum", start);
    // BaseCampMaxNum = primary gameplay limit (single-player base count)
    // BaseCampMaxNumInGuild = per-guild sub-limit (multiplayer), used as fallback
    let base_camp_max_num = gvas_read_int(&decompressed, "BaseCampMaxNum", start)
        .or_else(|| gvas_read_int(&decompressed, "BaseCampMaxNumInGuild", start));
    let base_camp_worker_max_num = gvas_read_int(&decompressed, "BaseCampWorkerMaxNum", start);
    let drop_item_alive_max_hours = gvas_read_float(&decompressed, "DropItemAliveMaxHours", start);
    let guild_player_max_num = gvas_read_int(&decompressed, "GuildPlayerMaxNum", start);
    let pal_egg_hatching_hours = gvas_read_float(&decompressed, "PalEggDefaultHatchingTime", start);
    let work_speed_rate = gvas_read_float(&decompressed, "WorkSpeedRate", start);
    let is_multiplay = gvas_read_bool(&decompressed, "bIsMultiplay", start);
    let is_pvp = gvas_read_bool(&decompressed, "bIsPvP", start);
    let can_pickup_other_guild_death_penalty_drop = gvas_read_bool(&decompressed, "bCanPickupOtherGuildDeathPenaltyDrop", start);
    let enable_non_login_penalty = gvas_read_bool(&decompressed, "bEnableNonLoginPenalty", start);
    let enable_fast_travel = gvas_read_bool(&decompressed, "bEnableFastTravel", start);
    let is_start_location_select_by_map = gvas_read_bool(&decompressed, "bIsStartLocationSelectByMap", start);
    let exist_player_after_logout = gvas_read_bool(&decompressed, "bExistPlayerAfterLogout", start);
    let supply_drop_span = gvas_read_int(&decompressed, "SupplyDropSpan", start);

    // Defense & durability
    let pal_damage_rate_defense = gvas_read_float(&decompressed, "PalDamageRateDefense", start);
    let player_damage_rate_defense = gvas_read_float(&decompressed, "PlayerDamageRateDefense", start);
    let equipment_durability_damage_rate = gvas_read_float(&decompressed, "EquipmentDurabilityDamageRate", start);

    // Farming, items & activities
    let monster_farm_action_speed_rate = gvas_read_float(&decompressed, "MonsterFarmActionSpeedRate", start);
    let fishing_difficulty_rate = gvas_read_float(&decompressed, "FishingDifficultyRate", start);
    let item_corruption_multiplier = gvas_read_float(&decompressed, "ItemCorruptionMultiplier", start);
    let item_weight_rate = gvas_read_float(&decompressed, "ItemWeightRate", start);

    // Building & world limits
    let build_object_hp_rate = gvas_read_float(&decompressed, "BuildObjectHpRate", start);
    let max_building_limit_num = gvas_read_int(&decompressed, "MaxBuildingLimitNum", start);
    let max_building_limit_num_per_player = gvas_read_int(&decompressed, "MaxBuildingLimitNumPerPlayer", start);
    let enable_predator_boss_pal = gvas_read_bool(&decompressed, "EnablePredatorBossPal", start)
        .or_else(|| gvas_read_bool(&decompressed, "bEnablePredatorBossPal", start));
    let randomizer_type = gvas_read_str(&decompressed, "RandomizerType", start);
    let randomizer_seed = gvas_read_str(&decompressed, "RandomizerSeed", start);

    // Multiplayer & guilds
    let base_camp_max_num_in_guild = gvas_read_int(&decompressed, "BaseCampMaxNumInGuild", start);
    let coop_player_max_num = gvas_read_int(&decompressed, "CoopPlayerMaxNum", start);
    let server_player_max_num = gvas_read_int(&decompressed, "ServerPlayerMaxNum", start);
    let guild_rejoin_cooldown_minutes = gvas_read_int(&decompressed, "GuildRejoinCooldownMinutes", start);
    let auto_reset_guild_time_no_online_players = gvas_read_float(&decompressed, "AutoResetGuildTimeNoOnlinePlayers", start);

    Some(WorldOptionSettings {
        exists: true,
        difficulty,
        day_time_speed_rate,
        night_time_speed_rate,
        exp_rate,
        pal_capture_rate,
        pal_spawn_num_rate,
        pal_damage_rate,
        player_damage_rate,
        player_stomach_decrease_rate,
        player_stamina_decrease_rate,
        player_auto_hp_regene_rate,
        player_auto_hp_regene_rate_in_sleeping,
        pal_stomach_decrease_rate,
        pal_stamina_decrease_rate,
        pal_auto_hp_regene_rate,
        pal_auto_hp_regene_rate_in_sleeping,
        build_object_damage_rate,
        build_object_deterioration_damage_rate,
        collection_drop_rate,
        collection_object_hp_rate,
        collection_object_respawn_speed_rate,
        enemy_drop_item_rate,
        death_penalty,
        enable_player_to_player_damage,
        enable_friendly_fire,
        enable_invader_enemy,
        active_unko,
        drop_item_max_num,
        base_camp_max_num,
        base_camp_worker_max_num,
        drop_item_alive_max_hours,
        guild_player_max_num,
        pal_egg_hatching_hours,
        work_speed_rate,
        is_multiplay,
        is_pvp,
        can_pickup_other_guild_death_penalty_drop,
        enable_non_login_penalty,
        enable_fast_travel,
        is_start_location_select_by_map,
        exist_player_after_logout,
        supply_drop_span,
        pal_damage_rate_defense,
        player_damage_rate_defense,
        equipment_durability_damage_rate,
        monster_farm_action_speed_rate,
        fishing_difficulty_rate,
        item_corruption_multiplier,
        item_weight_rate,
        build_object_hp_rate,
        max_building_limit_num,
        max_building_limit_num_per_player,
        enable_predator_boss_pal,
        randomizer_type,
        randomizer_seed,
        base_camp_max_num_in_guild,
        coop_player_max_num,
        server_player_max_num,
        guild_rejoin_cooldown_minutes,
        auto_reset_guild_time_no_online_players,
    })
}
