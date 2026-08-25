use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use sha2::{Sha256, Digest};
use tauri::{AppHandle, Emitter, State};
use crate::state::AppState;

static DOH_NOTIFICATION_SHOWN: AtomicBool = AtomicBool::new(false);

/// Get the path to the image cache directory.
pub fn get_image_cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("image_cache")
}

/// Calculate total size of the image cache directory in bytes.
#[tauri::command]
pub fn get_image_cache_size(state: State<'_, AppState>) -> Result<u64, String> {
    let data_guard = state.data.lock().map_err(|e| e.to_string())?;
    let cache_dir = get_image_cache_dir(Path::new(&data_guard.settings.program_path));
    
    if !cache_dir.exists() {
        return Ok(0);
    }
    
    let mut total_size = 0u64;
    if let Ok(entries) = fs::read_dir(&cache_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total_size += meta.len();
                }
            }
        }
    }
    Ok(total_size)
}

/// Purge all cached images.
#[tauri::command]
pub fn purge_image_cache(state: State<'_, AppState>) -> Result<(), String> {
    let data_guard = state.data.lock().map_err(|e| e.to_string())?;
    let cache_dir = get_image_cache_dir(Path::new(&data_guard.settings.program_path));
    
    if cache_dir.exists() {
        let _ = fs::remove_dir_all(&cache_dir);
        let _ = fs::create_dir_all(&cache_dir);
    }
    Ok(())
}

/// Resolve a hostname using DNS-over-HTTPS (DoH) via Cloudflare or Google IP endpoints.
async fn resolve_host_doh(host: &str, provider: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()
        .ok()?;

    let (query_url, host_header) = match provider {
        "cloudflare" => (
            format!("https://1.1.1.1/dns-query?name={}&type=A", host),
            "cloudflare-dns.com",
        ),
        "google" => (
            format!("https://8.8.8.8/resolve?name={}&type=A", host),
            "dns.google",
        ),
        _ => return None,
    };

    let resp = client.get(&query_url)
        .header("Host", host_header)
        .header("Accept", "application/dns-json")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    if let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) {
        for ans in answers {
            // Type 1 is A record (IPv4)
            if ans.get("type").and_then(|t| t.as_u64()) == Some(1) {
                if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                    return Some(data.to_string());
                }
            }
        }
    }
    None
}

/// Download an image with specific DNS / fallback configuration.
async fn download_image_bytes(
    url: &str,
    dns_mode: &str,
    app: Option<&AppHandle>,
) -> Result<(Vec<u8>, String), String> {
    let parsed_url = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    let host = parsed_url.host_str().unwrap_or("").to_string();

    let standard_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(7))
        .build()
        .map_err(|e| e.to_string())?;

    // Attempt 1: Standard connection (System DNS) if mode is auto or system
    if dns_mode == "auto" || dns_mode == "system" {
        let resp = standard_client.get(url)
            .header("User-Agent", format!("PalModManager/{}", env!("CARGO_PKG_VERSION")))
            .header("Referer", "https://www.nexusmods.com/")
            .send()
            .await;

        if let Ok(r) = resp {
            if r.status().is_success() {
                let content_type = r.headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("image/jpeg")
                    .to_string();
                if let Ok(bytes) = r.bytes().await {
                    return Ok((bytes.to_vec(), content_type));
                }
            }
        }

        if dns_mode == "system" {
            return Err("Failed to fetch image using system DNS".to_string());
        }
    }

    // Attempt 2: Cloudflare DoH (for auto or cloudflare mode)
    if dns_mode == "auto" || dns_mode == "cloudflare" {
        if let Some(ip) = resolve_host_doh(&host, "cloudflare").await {
            let mut resolved_url = parsed_url.clone();
            if resolved_url.set_host(Some(&ip)).is_ok() {
                let doh_client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(7))
                    .danger_accept_invalid_certs(true) // IP direct connect
                    .build();

                if let Ok(client) = doh_client {
                    let resp = client.get(resolved_url.as_str())
                        .header("Host", &host)
                        .header("User-Agent", format!("PalModManager/{}", env!("CARGO_PKG_VERSION")))
                        .header("Referer", "https://www.nexusmods.com/")
                        .send()
                        .await;

                    if let Ok(r) = resp {
                        if r.status().is_success() {
                            let content_type = r.headers()
                                .get("content-type")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("image/jpeg")
                                .to_string();
                            if let Ok(bytes) = r.bytes().await {
                                // Notify user once per session of DNS recovery
                                if dns_mode == "auto" && !DOH_NOTIFICATION_SHOWN.swap(true, Ordering::SeqCst) {
                                    if let Some(handle) = app {
                                        let _ = handle.emit("dns-fallback-triggered", "Cloudflare DoH (1.1.1.1)");
                                    }
                                }
                                return Ok((bytes.to_vec(), content_type));
                            }
                        }
                    }
                }
            }
        }
    }

    // Attempt 3: Google DoH (for auto or google mode)
    if dns_mode == "auto" || dns_mode == "google" {
        if let Some(ip) = resolve_host_doh(&host, "google").await {
            let mut resolved_url = parsed_url.clone();
            if resolved_url.set_host(Some(&ip)).is_ok() {
                let doh_client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(7))
                    .danger_accept_invalid_certs(true)
                    .build();

                if let Ok(client) = doh_client {
                    let resp = client.get(resolved_url.as_str())
                        .header("Host", &host)
                        .header("User-Agent", format!("PalModManager/{}", env!("CARGO_PKG_VERSION")))
                        .header("Referer", "https://www.nexusmods.com/")
                        .send()
                        .await;

                    if let Ok(r) = resp {
                        if r.status().is_success() {
                            let content_type = r.headers()
                                .get("content-type")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("image/jpeg")
                                .to_string();
                            if let Ok(bytes) = r.bytes().await {
                                if dns_mode == "auto" && !DOH_NOTIFICATION_SHOWN.swap(true, Ordering::SeqCst) {
                                    if let Some(handle) = app {
                                        let _ = handle.emit("dns-fallback-triggered", "Google Public DoH (8.8.8.8)");
                                    }
                                }
                                return Ok((bytes.to_vec(), content_type));
                            }
                        }
                    }
                }
            }
        }
    }

    Err(format!("Could not load image from URL: {}", url))
}

/// Fetch a remote image, cache it on disk, and return the local file path.
#[tauri::command]
pub async fn fetch_and_cache_image(
    url: String,
    dns_mode: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Ok(url);
    }

    let (cache_dir, active_dns_mode) = {
        let data_guard = state.data.lock().map_err(|e| e.to_string())?;
        let dir = get_image_cache_dir(Path::new(&data_guard.settings.program_path));
        let mode = dns_mode.or_else(|| data_guard.settings.dns_resolver.clone()).unwrap_or_else(|| "auto".to_string());
        (dir, mode)
    };

    let _ = fs::create_dir_all(&cache_dir);

    // Generate SHA-256 hash of URL for cache key
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash_hex = format!("{:x}", hasher.finalize());

    // Check if image already exists in cache (.webp, .jpg, .png)
    for ext in &["webp", "jpg", "jpeg", "png"] {
        let cached_path = cache_dir.join(format!("{}.{}", hash_hex, ext));
        if cached_path.exists() {
            return Ok(cached_path.to_string_lossy().to_string());
        }
    }

    // Download bytes with multi-stage fallback
    let (bytes, content_type) = download_image_bytes(&url, &active_dns_mode, Some(&app)).await?;

    let ext = if content_type.contains("webp") {
        "webp"
    } else if content_type.contains("png") {
        "png"
    } else {
        "jpg"
    };

    let file_path = cache_dir.join(format!("{}.{}", hash_hex, ext));
    fs::write(&file_path, &bytes).map_err(|e| format!("Failed to save cached image: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}
