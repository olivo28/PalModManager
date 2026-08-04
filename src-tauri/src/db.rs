use crate::models::AppData;
use crate::profiles;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use std::fs;
use std::path::PathBuf;

const DB_FILENAME: &str = "pmm_database.db";
const LEGACY_DB_FILENAME: &str = "mod-manager.db";

fn derive_key() -> [u8; 32] {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "default".to_string());
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "user".to_string());
    let salt = format!("{}::{}", hostname, username);

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    salt.hash(&mut hasher);
    let h1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    format!("{}::palmodmanager::v2", salt).hash(&mut hasher2);
    let h2 = hasher2.finish();

    let mut key = [0u8; 32];
    key[..8].copy_from_slice(&h1.to_le_bytes());
    key[8..16].copy_from_slice(&h2.to_le_bytes());
    key[16..24].copy_from_slice(&h1.to_be_bytes());
    key[24..32].copy_from_slice(&h2.to_be_bytes());
    key
}

fn encrypt(plaintext: &str) -> String {
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid key");

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encryption failed");

    let mut output = Vec::with_capacity(12 + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    format!("enc:{}", B64.encode(&output))
}

fn decrypt(encoded: &str) -> Result<String, String> {
    let data_str = encoded
        .strip_prefix("enc:")
        .ok_or("Not encrypted")?;
    let data = B64
        .decode(data_str)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    if data.len() < 12 {
        return Err("Invalid encrypted data".to_string());
    }

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid key");
    let nonce = Nonce::from_slice(&data[..12]);

    let plaintext = cipher
        .decrypt(nonce, &data[12..])
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 error: {}", e))
}

pub fn get_db_path(program_path: &str) -> PathBuf {
    PathBuf::from(program_path).join(DB_FILENAME)
}

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
];

const SETTINGS_MAPPING: &[(&str, &str)] = &[
    ("game_path", "gamePath"),
    ("program_path", "programPath"),
    ("hide_native_mods", "hideNativeMods"),
    ("custom_data_path", "customDataPath"),
];



fn convert_old_format(json_str: &str) -> Option<AppData> {
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

pub fn load_db(program_path: &str) -> AppData {
    let db_path = get_db_path(program_path);
    if !db_path.exists() {
        let legacy_db = PathBuf::from(program_path).join(LEGACY_DB_FILENAME);
        if legacy_db.exists() {
            let _ = fs::rename(&legacy_db, &db_path);
        }
    }

    if !db_path.exists() {
        let legacy = PathBuf::from(program_path).join("mod-manager.json");
        if legacy.exists() {
            let mut data = migrate_from_json(&legacy, program_path);
            profiles::ensure_default_profile(&mut data);
            return data;
        }
        let mut data = AppData::default();
        profiles::ensure_default_profile(&mut data);
        return data;
    }

    match fs::read_to_string(&db_path) {
        Ok(contents) => {
            let json = if contents.starts_with("enc:") {
                match decrypt(&contents) {
                    Ok(j) => j,
                    Err(e) => {
                        eprintln!("Decryption failed: {}", e);
                        return AppData::default();
                    }
                }
            } else {
                contents
            };

            match serde_json::from_str::<AppData>(&json) {
                Ok(mut data) => {
                    profiles::ensure_default_profile(&mut data);
                    data
                }
                Err(e) => {
                    eprintln!("Failed to parse DB with new format: {}", e);
                    match convert_old_format(&json) {
                        Some(mut data) => {
                            profiles::ensure_default_profile(&mut data);
                            let _ = save_db(program_path, &data);
                            data
                        }
                        None => {
                            eprintln!("Failed to parse DB with old format either");
                            let mut data = AppData::default();
                            profiles::ensure_default_profile(&mut data);
                            data
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read DB: {}", e);
            AppData::default()
        }
    }
}

fn migrate_from_json(legacy_path: &std::path::Path, program_path: &str) -> AppData {
    match fs::read_to_string(legacy_path) {
        Ok(contents) => {
            match convert_old_format(&contents) {
                Some(data) => {
                    let _ = save_db(program_path, &data);
                    let _ = fs::remove_file(legacy_path);
                    data
                }
                None => {
                    eprintln!("Failed to convert legacy JSON");
                    AppData::default()
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read legacy file: {}", e);
            AppData::default()
        }
    }
}

pub fn save_db(program_path: &str, data: &AppData) -> Result<(), String> {
    let db_path = get_db_path(program_path);
    let json = serde_json::to_string_pretty(data).map_err(|e| format!("Serialize error: {}", e))?;

    let encrypted = encrypt(&json);

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create DB dir: {}", e))?;
    }

    fs::write(&db_path, encrypted).map_err(|e| format!("Cannot write DB: {}", e))?;
    Ok(())
}
