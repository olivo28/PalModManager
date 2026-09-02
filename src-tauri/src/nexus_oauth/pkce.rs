use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};

pub fn get_client_id() -> &'static str {
    option_env!("NEXUS_CLIENT_ID").unwrap_or("pal_mod_manager")
}

pub fn get_client_secret() -> &'static str {
    option_env!("NEXUS_CLIENT_SECRET").unwrap_or("")
}

pub fn get_redirect_uri() -> &'static str {
    option_env!("NEXUS_REDIRECT_URI").unwrap_or("palmodmanager://oauth/callback")
}

/// Generate a cryptographically random code_verifier and S256 code_challenge
pub fn generate_pkce() -> (String, String) {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
    let verifier = URL_SAFE_NO_PAD.encode(&random_bytes);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(&hash);

    (verifier, challenge)
}
