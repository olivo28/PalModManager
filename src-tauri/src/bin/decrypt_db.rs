use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::fs;
use std::path::PathBuf;

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

fn main() {
    println!("=== PALMODMANAGER DB DECRYPTER ===");
    let app_data_dir = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .expect("LOCALAPPDATA env var not set");
    let db_path = app_data_dir.join("PalModManager").join("mod-manager.db");

    if !db_path.exists() {
        println!("Error: DB file not found at: {}", db_path.display());
        return;
    }

    println!("Reading encrypted DB from: {}", db_path.display());
    let contents = match fs::read_to_string(&db_path) {
        Ok(c) => c,
        Err(e) => {
            println!("Error reading file: {}", e);
            return;
        }
    };

    if !contents.starts_with("enc:") {
        println!("Database is not encrypted (plain JSON). Saving directly...");
        let _ = fs::write("db_plain.json", &contents);
        println!("Done! Saved to db_plain.json");
        return;
    }

    let encoded_data = &contents[4..];
    let decoded = match B64.decode(encoded_data) {
        Ok(d) => d,
        Err(e) => {
            println!("Base64 decode error: {}", e);
            return;
        }
    };

    if decoded.len() < 12 {
        println!("Invalid database format (too small).");
        return;
    }

    let nonce_bytes = &decoded[..12];
    let ciphertext = &decoded[12..];

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid key");
    let nonce = Nonce::from_slice(nonce_bytes);

    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => {
            let json_str = String::from_utf8_lossy(&plaintext);
            let pretty_json: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");
            let pretty_str = serde_json::to_string_pretty(&pretty_json).expect("Pretty JSON serialize");
            let _ = fs::write("db_plain.json", pretty_str);
            println!("SUCCESS! Decrypted database saved to: db_plain.json");
        }
        Err(e) => {
            println!("Decryption failed! Error: {}", e);
        }
    }
}
