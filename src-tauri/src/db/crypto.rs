use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const VAULT_TOKEN_FILENAME: &str = ".pmm_vault";

/// Obtains or creates a persistent vault secret token in the given directory.
/// This enables portability across different PCs, OS accounts, or hostnames (e.g. on USB drives).
pub fn get_or_create_vault_token(program_path: &Path) -> [u8; 32] {
    let token_path = program_path.join(VAULT_TOKEN_FILENAME);
    if token_path.is_file() {
        if let Ok(raw) = fs::read(&token_path) {
            if raw.len() == 32 {
                let mut token = [0u8; 32];
                token.copy_from_slice(&raw);
                return token;
            }
        }
    }

    // Generate new high-entropy token
    let mut token = [0u8; 32];
    OsRng.fill_bytes(&mut token);
    let _ = fs::write(&token_path, token);

    // On Windows, set hidden file attribute if possible
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Best effort: hidden attribute
        let _ = std::process::Command::new("attrib")
            .arg("+H")
            .arg(&token_path)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .status();
    }

    token
}

/// Derives a stable 256-bit AES key using SHA-256 KDF.
/// Incorporates the vault token when available, as well as a stable salt.
pub fn derive_key_v2(program_path: &Path) -> [u8; 32] {
    let vault_token = get_or_create_vault_token(program_path);
    let mut hasher = Sha256::new();
    hasher.update(b"PalModManager::Vault::v2::");
    hasher.update(vault_token);
    hasher.update(b"::SecureStorageKey");
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Legacy key derivation function from v1.7.2 and prior (based on DefaultHasher).
/// Kept strictly for backwards compatibility migration.
pub fn derive_legacy_key() -> [u8; 32] {
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

/// Encrypts plaintext string using AES-256-GCM with a fresh 96-bit nonce.
/// Prepends "enc:" prefix and base64 encodes [nonce (12b) + ciphertext + tag (16b)].
pub fn encrypt_with_key(plaintext: &str, key: &[u8; 32]) -> String {
    let cipher = Aes256Gcm::new_from_slice(key).expect("valid 32-byte key");

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

/// Attempts to decrypt payload using the provided 32-byte AES key.
pub fn decrypt_with_key(encoded: &str, key: &[u8; 32]) -> Result<String, String> {
    let data_str = encoded
        .strip_prefix("enc:")
        .ok_or_else(|| "Not encrypted payload (missing 'enc:' prefix)".to_string())?;

    let data = B64
        .decode(data_str)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    if data.len() < 12 {
        return Err("Invalid encrypted data length (less than 12 bytes nonce)".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid cipher key: {}", e))?;
    let nonce = Nonce::from_slice(&data[..12]);

    let plaintext = cipher
        .decrypt(nonce, &data[12..])
        .map_err(|e| format!("Decryption failed (integrity tag mismatch): {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode error: {}", e))
}

/// Decrypts encrypted string with automatic fallback:
/// 1. Tries v2 portable vault key.
/// 2. If it fails, falls back to legacy v1 DefaultHasher key.
/// Returns (decrypted_json, was_migrated_from_legacy).
pub fn decrypt_string(encoded: &str, program_path: &Path) -> Result<(String, bool), String> {
    let v2_key = derive_key_v2(program_path);
    match decrypt_with_key(encoded, &v2_key) {
        Ok(json) => Ok((json, false)),
        Err(err_v2) => {
            // Fallback to legacy key
            let legacy_key = derive_legacy_key();
            match decrypt_with_key(encoded, &legacy_key) {
                Ok(json) => {
                    crate::logger::log("[db::crypto] Successfully decrypted database with legacy KDF. Will migrate to v2 vault on next save.");
                    Ok((json, true))
                }
                Err(err_legacy) => Err(format!(
                    "Decryption failed under both v2 ({}) and legacy ({}) KDF",
                    err_v2, err_legacy
                )),
            }
        }
    }
}
