use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::models::{SaveWorldSummary, WorldCustomMeta, WorldOptionSettings};
use super::gvas::{
    decompress_palworld_save, gvas_read_str, gvas_read_int, gvas_read_int64,
    gvas_read_float, gvas_read_bool, gvas_find_property, gvas_fstring
};
use super::deep_scan::{list_available_backups, detect_external_edits_and_anomalies};

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
pub fn list_save_worlds(custom_root: Option<&str>) -> Result<Vec<SaveWorldSummary>, String> {
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
                    let world_id = dir_path.file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "UnknownWorld".to_string());

                    let level_size = fs::metadata(&level_sav).map(|m| m.len()).unwrap_or(0);

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

                    let (health_status, detected_issues) = quick_check_save_health(dir_path, &level_sav);
                    let custom_meta = Some(load_world_custom_meta(dir_path));
                    let world_options = parse_world_options(dir_path);

                    worlds.push(SaveWorldSummary {
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
                        latest_backup_date,
                        has_external_edits,
                        health_status,
                        detected_issues_count: detected_issues,
                        custom_meta,
                        world_options,
                    });
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
    open::that(path).map_err(|e| e.to_string())
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

    // --- 1. LevelMeta.sav: World Name ---
    if meta_path.exists() {
        if let Ok(raw) = fs::read(meta_path) {
            if let Ok(decompressed) = decompress_palworld_save(&raw) {
                let start = decompressed.windows(4).position(|w| w == b"GVAS").unwrap_or(0);
                if let Some(name) = gvas_read_str(&decompressed, "WorldName", start)
                    .or_else(|| gvas_read_str(&decompressed, "SaveDataTitle", start))
                {
                    if !name.is_empty() { world_name = name; }
                }
            }
        }
    }

    // --- 2. Level.sav: InGameDay (GameDateTimeTicks) + Host Player NickName & Level ---
    const MAX_LEVEL_SAV_SCAN: u64 = 40 * 1024 * 1024;
    let level_size = if level_path.exists() {
        fs::metadata(level_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    if level_path.exists() && level_size <= MAX_LEVEL_SAV_SCAN {
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
            }
        }
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
                    }
                }
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
    let bytes = match fs::read(level_sav) {
        Ok(b) => b,
        Err(_) => return ("corrupt".to_string(), 1),
    };

    let decompressed = match decompress_palworld_save(&bytes) {
        Ok(d) => d,
        Err(_) => return ("corrupt".to_string(), 1),
    };

    let is_gvas = decompressed.starts_with(b"GVAS") || decompressed.windows(4).take(32).any(|w| w == b"GVAS");
    if !is_gvas {
        return ("corrupt".to_string(), 1);
    }

    let players_dir = world_dir.join("Players");
    if players_dir.exists() {
        if let Ok(rd) = fs::read_dir(players_dir) {
            for entry in rd.flatten() {
                if entry.path().extension().map_or(false, |e| e.eq_ignore_ascii_case("sav")) {
                    if let Ok(p_b) = fs::read(entry.path()) {
                        if decompress_palworld_save(&p_b).is_err() {
                            return ("corrupt".to_string(), 1);
                        }
                    }
                }
            }
        }
    }

    let text = String::from_utf8_lossy(&decompressed);
    let mut mod_issues = 0;
    if text.contains("/Game/Mods/") || (text.contains("/Mods/") && text.contains("_C")) {
        mod_issues += 1;
    }

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
    let pal_damage_rate = gvas_read_float(&decompressed, "PalDamageRateMultiplier", start)
        .or_else(|| gvas_read_float(&decompressed, "PalDamageRate", start));
    let player_damage_rate = gvas_read_float(&decompressed, "PlayerDamageRateMultiplier", start)
        .or_else(|| gvas_read_float(&decompressed, "PlayerDamageRate", start));
    let player_stomach_decrease_rate = gvas_read_float(&decompressed, "PlayerStomachDecreaceRate", start);
    let player_stamina_decrease_rate = gvas_read_float(&decompressed, "PlayerStaminaDecreaceRate", start);
    let player_auto_hp_regene_rate = gvas_read_float(&decompressed, "PlayerAutoHPRegeneRate", start);
    let player_auto_hp_regene_rate_in_sleeping = gvas_read_float(&decompressed, "PlayerAutoHpRegeneRateInSleeping", start);
    let pal_stomach_decrease_rate = gvas_read_float(&decompressed, "PalStomachDecreaceRate", start);
    let pal_stamina_decrease_rate = gvas_read_float(&decompressed, "PalStaminaDecreaceRate", start);
    let pal_auto_hp_regene_rate = gvas_read_float(&decompressed, "PalAutoHPRegeneRate", start);
    let pal_auto_hp_regene_rate_in_sleeping = gvas_read_float(&decompressed, "PalAutoHpRegeneRateInSleeping", start);
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
    let base_camp_max_num = gvas_read_int(&decompressed, "BaseCampMaxNum", start);
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
    })
}
