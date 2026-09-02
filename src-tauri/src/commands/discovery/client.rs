use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use std::time::Duration;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GRAPHQL_ENDPOINT: &str = "https://api.nexusmods.com/v2/graphql";
pub const REST_BASE_URL: &str = "https://api.nexusmods.com/v1";

pub fn get_client(token: Option<&str>) -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Application-Name",
        HeaderValue::from_static("PalModManager"),
    );
    headers.insert(
        "Application-Version",
        HeaderValue::from_static(APP_VERSION),
    );
    if let Some(t) = token {
        if !t.is_empty() {
            if let Ok(hv) = HeaderValue::from_str(&format!("Bearer {}", t)) {
                headers.insert(AUTHORIZATION, hv);
            }
        }
    }
    reqwest::Client::builder()
        .default_headers(headers)
        .user_agent(format!("PalModManager/{} (Tauri App)", APP_VERSION))
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
