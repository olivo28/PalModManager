use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub ue4ss_installed: bool,
    pub ue4ss_version: Option<String>,
    /// Human-readable tag of the latest UE4SS release (e.g. "experimental-palworld")
    pub ue4ss_latest_tag: Option<String>,
    /// ISO date of the latest UE4SS release — used internally for comparison
    pub ue4ss_latest_date: Option<String>,
    pub ue4ss_needs_update: bool,
    pub palschema_installed: bool,
    pub palschema_version: Option<String>,
    pub palschema_latest_version: Option<String>,
    pub palschema_needs_update: bool,
}

#[repr(C)]
struct VS_FIXEDFILEINFO {
    dw_signature: u32,
    dw_struc_version: u32,
    dw_file_version_ms: u32,
    dw_file_version_ls: u32,
    dw_product_version_ms: u32,
    dw_product_version_ls: u32,
    dw_file_flags_mask: u32,
    dw_file_flags: u32,
    dw_file_os: u32,
    dw_file_type: u32,
    dw_file_subtype: u32,
    dw_file_date_ms: u32,
    dw_file_date_ls: u32,
}

#[link(name = "version")]
extern "system" {
    fn GetFileVersionInfoSizeW(
        lptstr_filename: *const u16,
        lpdw_handle: *mut u32,
    ) -> u32;
    fn GetFileVersionInfoW(
        lptstr_filename: *const u16,
        dw_handle: u32,
        dw_len: u32,
        lp_data: *mut std::ffi::c_void,
    ) -> i32;
    fn VerQueryValueW(
        p_block: *const std::ffi::c_void,
        lp_sub_block: *const u16,
        lplp_buffer: *mut *mut std::ffi::c_void,
        pu_len: *mut u32,
    ) -> i32;
}

fn get_file_version(path: &str) -> Option<String> {
    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let size = GetFileVersionInfoSizeW(wide.as_ptr(), std::ptr::null_mut());
        if size == 0 { return None; }
        let mut buf = vec![0u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, size, buf.as_mut_ptr() as *mut std::ffi::c_void) == 0 {
            return None;
        }
        let mut info: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut info_len: u32 = 0;
        let backslash: [u16; 2] = [0x5C, 0];
        if VerQueryValueW(
            buf.as_ptr() as *const std::ffi::c_void,
            backslash.as_ptr(),
            &mut info,
            &mut info_len,
        ) == 0 || info.is_null() {
            return None;
        }
        let fixed = &*(info as *const VS_FIXEDFILEINFO);
        let major = fixed.dw_product_version_ms >> 16;
        let minor = fixed.dw_product_version_ms & 0xFFFF;
        let build = fixed.dw_product_version_ls >> 16;
        let patch = fixed.dw_product_version_ls & 0xFFFF;
        let version = format!("{}.{}.{}", major, minor, build);
        let version = if patch > 0 { format!("{}.{}", version, patch) } else { version };
        Some(version)
    }
}

fn get_file_string_version(path: &str, field: &str) -> Option<String> {
    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let size = GetFileVersionInfoSizeW(wide.as_ptr(), std::ptr::null_mut());
        if size == 0 { return None; }
        let mut buf = vec![0u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, size, buf.as_mut_ptr() as *mut std::ffi::c_void) == 0 {
            return None;
        }
        let mut trans: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut trans_len: u32 = 0;
        let trans_key: Vec<u16> = OsStr::new("\\VarFileInfo\\Translation")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        if VerQueryValueW(
            buf.as_ptr() as *const std::ffi::c_void,
            trans_key.as_ptr(),
            &mut trans,
            &mut trans_len,
        ) == 0 || trans.is_null() || trans_len < 4 {
            return None;
        }
        let lang = *(trans as *const u16);
        let codepage = *(trans as *const u16).offset(1);
        let sub_block = format!("\\StringFileInfo\\{:04X}{:04X}\\{}", lang, codepage, field);
        let wide_sub: Vec<u16> = OsStr::new(&sub_block)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut str_buf: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut str_len: u32 = 0;
        if VerQueryValueW(
            buf.as_ptr() as *const std::ffi::c_void,
            wide_sub.as_ptr(),
            &mut str_buf,
            &mut str_len,
        ) == 0 || str_buf.is_null() {
            return None;
        }
        let slice = std::slice::from_raw_parts(str_buf as *const u16, (str_len as usize).saturating_sub(1));
        Some(String::from_utf16_lossy(slice))
    }
}

fn get_file_date(path: &str) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    let secs = duration.as_secs() as i64;
    let dt = chrono::DateTime::from_timestamp(secs, 0)?;
    Some(dt.format("%d.%m.%Y").to_string())
}

pub fn check_dependencies(game_path: &str) -> DependencyStatus {
    let game_path = Path::new(game_path);

    // UE4SS check — detect by dwmapi.dll presence, version = file date or ue4ss.version
    let dwmapi = game_path.join("Pal").join("Binaries").join("Win64").join("dwmapi.dll");
    let ue4ss_installed = dwmapi.exists();
    let ue4ss_version = if ue4ss_installed {
        let version_file = game_path.join("Pal").join("Binaries").join("Win64")
            .join("ue4ss").join("ue4ss.version");
        if version_file.exists() {
            fs::read_to_string(version_file).ok().map(|s| s.trim().to_string())
        } else {
            get_file_date(&dwmapi.to_string_lossy())
        }
    } else {
        None
    };

    // PalSchema check — detect by dlls/main.dll, version = ProductVersion string or palschema.version
    let ps_dll = game_path.join("Pal").join("Binaries").join("Win64")
        .join("ue4ss").join("Mods").join("PalSchema").join("dlls").join("main.dll");
    let palschema_installed = ps_dll.exists();
    let palschema_version = if palschema_installed {
        let version_file = game_path.join("Pal").join("Binaries").join("Win64")
            .join("ue4ss").join("Mods").join("PalSchema").join("palschema.version");
        if version_file.exists() {
            fs::read_to_string(version_file).ok().map(|s| s.trim().to_string())
        } else {
            let dll_path = ps_dll.to_string_lossy();
            get_file_string_version(&dll_path, "ProductVersion")
                .or_else(|| get_file_version(&dll_path))
        }
    } else {
        None
    };

    DependencyStatus {
        ue4ss_installed,
        ue4ss_version,
        ue4ss_latest_tag: None,
        ue4ss_latest_date: None,
        ue4ss_needs_update: false,
        palschema_installed,
        palschema_version,
        palschema_latest_version: None,
        palschema_needs_update: false,
    }
}

/// Returns a tuple: (tag_name, iso_date_string) for the latest UE4SS release.
/// tag_name is what we show to the user; iso_date is used for update comparison.
pub async fn check_ue4ss_latest() -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let tag_name = "experimental-palworld".to_string();

    // Try HTML scraping first to avoid API rate limit
    if let Ok(resp) = client.get("https://github.com/Okaetsu/RE-UE4SS/releases/tag/experimental-palworld").send().await {
        if let Ok(html) = resp.text().await {
            if let Some(time_pos) = html.find("datetime=") {
                let time_start = time_pos + 10;
                if let Some(time_end) = html[time_start..].find('"') {
                    let dt_raw = &html[time_start..time_start + time_end];
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_raw) {
                        let iso_date = dt.format("%d.%m.%Y").to_string();
                        return Ok((tag_name, iso_date));
                    }
                }
            }
        }
    }

    // Fallback: GitHub API
    let url = "https://api.github.com/repos/Okaetsu/RE-UE4SS/releases/tags/experimental-palworld";
    let resp = client.get(url)
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let api_tag = json["tag_name"].as_str().unwrap_or("experimental-palworld").to_string();
    let mut latest: Option<chrono::DateTime<chrono::Utc>> = None;
    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            if let Some(updated) = asset["updated_at"].as_str() {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(updated) {
                    match latest {
                        Some(current) if dt > current => { latest = Some(dt.into()); }
                        None => { latest = Some(dt.into()); }
                        _ => {}
                    }
                }
            }
        }
    }
    if latest.is_none() {
        if let Some(published) = json["published_at"].as_str() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(published) {
                latest = Some(dt.into());
            }
        }
    }
    match latest {
        Some(dt) => Ok((api_tag, dt.format("%d.%m.%Y").to_string())),
        None => Err("Could not determine latest UE4SS date".to_string()),
    }
}

pub async fn check_palschema_latest() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Try HTML redirect first to avoid API rate limit!
    if let Ok(resp) = client.get("https://github.com/Okaetsu/PalSchema/releases/latest").send().await {
        let final_url = resp.url().as_str();
        if final_url.contains("/releases/tag/") {
            if let Some(tag) = final_url.split("/releases/tag/").last() {
                if !tag.is_empty() {
                    return Ok(tag.to_string());
                }
            }
        }
    }

    let url = "https://api.github.com/repos/Okaetsu/PalSchema/releases/latest";
    let resp = client.get(url)
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    let tag = json["tag_name"].as_str()
        .ok_or_else(|| "No tag_name in response".to_string())?;
    Ok(tag.to_string())
}
