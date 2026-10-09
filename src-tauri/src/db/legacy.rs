use crate::models::AppData;

const MOD_INFO_MAPPING: &[(&str, &str)] = &[
    ("nexus_mod_id", "nexusModId"),
    ("nexus_url", "nexusUrl"),
    ("nexus_author", "nexusAuthor"),
    ("nexus_summary", "nexusSummary"),
    ("nexus_picture_url", "nexusPictureUrl"),
    ("nexus_endorsements", "nexusEndorsements"),
    ("nexus_downloads", "nexusDownloads"),
    ("install_date", "installDate"),
    ("source_zip", "sourceZip"),
    ("config_path", "configPath"),
    ("config_type", "configType"),
    ("game_path", "gamePath"),
    ("disabled_path", "disabledPath"),
    ("pak_destination", "pakDestination"),
    ("has_enabled_txt", "hasEnabledTxt"),
    ("mods_txt_order", "modsTxtOrder"),
    ("extra_files", "extraFiles"),
    ("nexus_description", "nexusDescription"),
    ("nexus_version_cached", "nexusVersionCached"),
    ("nexus_cached_at", "nexusCachedAt"),
    ("nexus_category", "nexusCategory"),
    ("nexus_tags", "nexusTags"),
    ("origin_load_method", "originLoadMethod"),
];

const SETTINGS_MAPPING: &[(&str, &str)] = &[
    ("game_path", "gamePath"),
    ("program_path", "programPath"),
    ("hide_native_mods", "hideNativeMods"),
    ("custom_data_path", "customDataPath"),
];

fn rename_keys(value: &mut serde_json::Value, mapping: &[(&str, &str)]) {
    if let serde_json::Value::Object(map) = value {
        let old_keys: Vec<String> = map.keys().cloned().collect();
        for old_key in old_keys {
            if let Some(new_key) = mapping.iter().find(|(k, _)| *k == old_key).map(|(_, v)| *v) {
                if let Some(val) = map.remove(&old_key) {
                    map.insert(new_key.to_string(), val);
                }
            }
        }
    }
}

fn fix_mod_type(value: &mut serde_json::Value) {
    if let serde_json::Value::Object(map) = value {
        if let Some(type_val) = map.get("type") {
            if let serde_json::Value::Object(inner) = type_val {
                if let Some(inner_type) = inner.get("type") {
                    if let Some(s) = inner_type.as_str() {
                        map.insert("type".to_string(), serde_json::Value::String(s.to_string()));
                    }
                }
            }
        }
    }
}

pub fn convert_old_format(json_str: &str) -> Option<AppData> {
    let mut root: serde_json::Value = serde_json::from_str(json_str).ok()?;
    if let Some(mods) = root.get_mut("mods").and_then(|m| m.as_array_mut()) {
        for mod_item in mods.iter_mut() {
            rename_keys(mod_item, MOD_INFO_MAPPING);
            fix_mod_type(mod_item);
        }
    }
    if let Some(settings) = root.get_mut("settings") {
        rename_keys(settings, SETTINGS_MAPPING);
    }
    let json_str = serde_json::to_string(&root).ok()?;
    serde_json::from_str::<AppData>(&json_str).ok()
}

pub fn run_db_migrations(data: &mut AppData, from_version: u32, target_version: u32) -> bool {
    if from_version >= target_version {
        return false;
    }

    crate::logger::log(&format!("DB Migration: Upgrading AppData schema from v{} to v{}", from_version, target_version));

    // Migration 1 -> 2:
    // 1. Sanitize config_path: Purge .nexus.json, dotfiles, or modinfo.pmm.json from config_path
    // 2. Re-prioritize component paths: promote UE4SS over PalSchema in hybrid mods
    if from_version < 2 {
        for m in data.mods.iter_mut() {
            if let Some(ref cp) = m.config_path {
                let cp_lower = cp.to_lowercase();
                if cp_lower.ends_with(".nexus.json")
                    || cp_lower.ends_with(".pmm.json")
                    || cp_lower.ends_with("modinfo.pmm.json")
                    || cp_lower.ends_with("enabled.txt")
                    || cp_lower.ends_with("manifest.json")
                    || cp.starts_with('.')
                {
                    crate::logger::log(&format!("DB Migration v2: Purged metadata config_path {:?} from mod '{}'", cp, m.name));
                    m.config_path = None;
                    m.config_type = None;
                }
            }

            let is_palschema = m.game_path.to_lowercase().contains("palschema") || m.game_path.to_lowercase().contains("content/pal");
            if is_palschema {
                if let Some(pos) = m.extra_files.iter().position(|f| f.to_lowercase().contains("mods/ue4ss") || f.to_lowercase().contains("binaries/win64/ue4ss/mods")) {
                    let ue4ss_path = m.extra_files.remove(pos);
                    let old_palschema = std::mem::replace(&mut m.game_path, ue4ss_path);
                    m.extra_files.push(old_palschema);
                    crate::logger::log(&format!("DB Migration v2: Re-prioritized primary folder to UE4SS for mod '{}'", m.name));
                }
            }
        }

        data.schema_version = 2;
    }

    true
}
