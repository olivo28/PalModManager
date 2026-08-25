use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "data")]
pub enum ProtocolStatus {
    Registered { path: String },
    OutdatedPath { current_exe: String, registered_path: String },
    NotRegistered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailedProtocolInfo {
    pub palmodmanager: ProtocolStatus,
    pub nxm: ProtocolStatus,
    pub nxm_handler_name: Option<String>,
    pub nxm_handler_path: Option<String>,
}

const SCHEMES: &[(&str, &str)] = &[
    ("palmodmanager", "URL:PalModManager Protocol"),
    ("nxm", "URL:NXM Protocol"),
];

/// Get the path of the currently running executable
pub fn get_current_exe_path() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))
}

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
fn get_scheme_raw_command(scheme: &str) -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path = format!("Software\\Classes\\{}\\shell\\open\\command", scheme);

    if let Ok(command_key) = hkcu.open_subkey(&key_path) {
        if let Ok(v) = command_key.get_value::<String, _>("") {
            return Some(v);
        }
    }

    // Also check HKEY_CLASSES_ROOT (HKCR) as fallback for Vortex/MO2 system registrations
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    if let Ok(command_key) = hkcr.open_subkey(&format!("{}\\shell\\open\\command", scheme)) {
        if let Ok(v) = command_key.get_value::<String, _>("") {
            return Some(v);
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn check_single_scheme_status(scheme: &str, current_exe: &str) -> ProtocolStatus {
    let command_val = match get_scheme_raw_command(scheme) {
        Some(v) => v,
        None => return ProtocolStatus::NotRegistered,
    };

    let cleaned_registered = command_val
        .trim()
        .trim_matches('"')
        .split("\" \"")
        .next()
        .unwrap_or("")
        .split("\" -")
        .next()
        .unwrap_or("")
        .trim_end_matches(" %1")
        .trim_end_matches("\"%1\"")
        .trim_matches('"')
        .to_string();

    let cur_canonical = current_exe.to_lowercase().replace('/', "\\");
    let reg_canonical = cleaned_registered.to_lowercase().replace('/', "\\");

    if reg_canonical.is_empty() {
        ProtocolStatus::NotRegistered
    } else if cur_canonical == reg_canonical {
        ProtocolStatus::Registered { path: current_exe.to_string() }
    } else {
        ProtocolStatus::OutdatedPath {
            current_exe: current_exe.to_string(),
            registered_path: cleaned_registered,
        }
    }
}

/// Detect human-friendly application name from an executable path
pub fn detect_handler_app_name(raw_command: &str) -> Option<String> {
    let lower = raw_command.to_lowercase();
    if lower.contains("palmodmanager") {
        Some("PalModManager".to_string())
    } else if lower.contains("vortex") {
        Some("Vortex".to_string())
    } else if lower.contains("modorganizer") || lower.contains("mo2") {
        Some("Mod Organizer 2".to_string())
    } else if lower.contains("wrye") {
        Some("Wrye Bash".to_string())
    } else if lower.contains("nmm") || lower.contains("nexusclient") {
        Some("Nexus Mod Manager".to_string())
    } else {
        std::path::Path::new(raw_command)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }
}

/// Get detailed status for both palmodmanager and nxm protocols
pub fn get_detailed_protocol_info() -> DetailedProtocolInfo {
    let current_exe = match get_current_exe_path() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return DetailedProtocolInfo {
            palmodmanager: ProtocolStatus::NotRegistered,
            nxm: ProtocolStatus::NotRegistered,
            nxm_handler_name: None,
            nxm_handler_path: None,
        },
    };

    #[cfg(target_os = "windows")]
    {
        let pmm_status = check_single_scheme_status("palmodmanager", &current_exe);
        let nxm_status = check_single_scheme_status("nxm", &current_exe);

        let raw_nxm_cmd = get_scheme_raw_command("nxm");
        let (nxm_handler_name, nxm_handler_path) = match raw_nxm_cmd {
            Some(cmd) => {
                let cleaned = cmd
                    .trim()
                    .trim_matches('"')
                    .split("\" \"")
                    .next()
                    .unwrap_or("")
                    .split("\" -")
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(" %1")
                    .trim_end_matches("\"%1\"")
                    .trim_matches('"')
                    .to_string();
                let name = detect_handler_app_name(&cleaned);
                (name, Some(cleaned))
            }
            None => (None, None),
        };

        DetailedProtocolInfo {
            palmodmanager: pmm_status,
            nxm: nxm_status,
            nxm_handler_name,
            nxm_handler_path,
        }
    }

    #[cfg(target_os = "linux")]
    {
        let pmm_handler = check_linux_scheme_handler("palmodmanager");
        let nxm_handler = check_linux_scheme_handler("nxm");

        let pmm_status = match pmm_handler {
            Some(ref h) if h.contains("palmodmanager") => ProtocolStatus::Registered { path: current_exe.clone() },
            Some(_) => ProtocolStatus::OutdatedPath {
                current_exe: current_exe.clone(),
                registered_path: pmm_handler.clone().unwrap_or_default(),
            },
            None => ProtocolStatus::NotRegistered,
        };

        let nxm_status = match nxm_handler {
            Some(ref h) if h.contains("palmodmanager") => ProtocolStatus::Registered { path: current_exe.clone() },
            Some(ref h) => ProtocolStatus::OutdatedPath {
                current_exe: current_exe.clone(),
                registered_path: h.clone(),
            },
            None => ProtocolStatus::NotRegistered,
        };

        let (nxm_handler_name, nxm_handler_path) = match nxm_handler {
            Some(ref h) => (detect_handler_app_name(h), Some(h.clone())),
            None => (None, None),
        };

        DetailedProtocolInfo {
            palmodmanager: pmm_status,
            nxm: nxm_status,
            nxm_handler_name,
            nxm_handler_path,
        }
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, URL schemes (palmodmanager:// and nxm://) are defined in Info.plist (CFBundleURLSchemes)
        // and handled via Launch Services / single-instance
        DetailedProtocolInfo {
            palmodmanager: ProtocolStatus::Registered { path: current_exe.clone() },
            nxm: ProtocolStatus::Registered { path: current_exe },
            nxm_handler_name: Some("PalModManager".to_string()),
            nxm_handler_path: None,
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        DetailedProtocolInfo {
            palmodmanager: ProtocolStatus::Registered { path: current_exe.clone() },
            nxm: ProtocolStatus::Registered { path: current_exe },
            nxm_handler_name: Some("PalModManager".to_string()),
            nxm_handler_path: None,
        }
    }
}

/// Check if protocols are registered in registry/xdg/launchservices (palmodmanager protocol check)
pub fn check_protocol_status() -> ProtocolStatus {
    let details = get_detailed_protocol_info();
    details.palmodmanager
}

#[cfg(target_os = "linux")]
fn get_linux_desktop_file_path() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("HOME") {
        let p = PathBuf::from(home).join(".local/share/applications/palmodmanager.desktop");
        Some(p)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn check_linux_scheme_handler(scheme: &str) -> Option<String> {
    let output = std::process::Command::new("xdg-mime")
        .args(["query", "default", &format!("x-scheme-handler/{}", scheme)])
        .output()
        .ok()?;
    if output.status.success() {
        let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !res.is_empty() {
            return Some(res);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn register_linux_protocols(scheme: &str, exe_path_str: &str) -> Result<(), String> {
    if let Some(desktop_path) = get_linux_desktop_file_path() {
        if let Some(parent) = desktop_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=PalModManager\n\
             Comment=Mod Manager & Config Editor for Palworld\n\
             Exec=\"{}\" %u\n\
             Icon=palmodmanager\n\
             Terminal=false\n\
             Categories=Game;Utility;\n\
             MimeType=x-scheme-handler/palmodmanager;x-scheme-handler/nxm;\n",
            exe_path_str
        );
        std::fs::write(&desktop_path, content).map_err(|e| format!("Failed to write desktop file: {}", e))?;
    }

    if scheme == "all" || scheme == "palmodmanager" {
        let _ = std::process::Command::new("xdg-mime")
            .args(["default", "palmodmanager.desktop", "x-scheme-handler/palmodmanager"])
            .status();
    }
    if scheme == "all" || scheme == "nxm" {
        let _ = std::process::Command::new("xdg-mime")
            .args(["default", "palmodmanager.desktop", "x-scheme-handler/nxm"])
            .status();
    }

    if let Some(home) = std::env::var_os("HOME") {
        let app_dir = PathBuf::from(home).join(".local/share/applications");
        let _ = std::process::Command::new("update-desktop-database")
            .arg(app_dir)
            .status();
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn unregister_linux_protocol(scheme: &str) -> Result<(), String> {
    if scheme == "all" || scheme == "nxm" {
        let _ = std::process::Command::new("xdg-mime")
            .args(["default", "", "x-scheme-handler/nxm"])
            .status();
    }
    if scheme == "all" || scheme == "palmodmanager" {
        let _ = std::process::Command::new("xdg-mime")
            .args(["default", "", "x-scheme-handler/palmodmanager"])
            .status();
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn register_single_scheme(scheme: &str, desc: &str, exe_path_str: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path = format!("Software\\Classes\\{}", scheme);
    let (proto_key, _) = hkcu
        .create_subkey(&key_path)
        .map_err(|e| format!("Failed to create registry key {}: {}", key_path, e))?;

    proto_key
        .set_value("", &desc)
        .map_err(|e| format!("Failed to set protocol description for {}: {}", scheme, e))?;

    proto_key
        .set_value("URL Protocol", &"")
        .map_err(|e| format!("Failed to set URL Protocol value for {}: {}", scheme, e))?;

    // DefaultIcon
    let (icon_key, _) = proto_key
        .create_subkey("DefaultIcon")
        .map_err(|e| format!("Failed to create DefaultIcon key: {}", e))?;
    let icon_val = format!("\"{}\",0", exe_path_str);
    let _ = icon_key.set_value("", &icon_val);

    // shell\open\command
    let (cmd_key, _) = proto_key
        .create_subkey("shell\\open\\command")
        .map_err(|e| format!("Failed to create shell\\open\\command key: {}", e))?;

    let cmd_val = format!("\"{}\" \"%1\"", exe_path_str);
    cmd_key
        .set_value("", &cmd_val)
        .map_err(|e| format!("Failed to set command value: {}", e))?;

    crate::logger::log(&format!(
        "register_protocol: Successfully registered {}:// to '{}'",
        scheme, exe_path_str
    ));

    Ok(())
}

/// Register a specific scheme ("palmodmanager", "nxm", or "all")
pub fn register_specific_scheme(scheme: &str) -> Result<String, String> {
    let current_exe = get_current_exe_path()?;
    let exe_path_str = current_exe.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        if scheme == "all" {
            for (s, desc) in SCHEMES {
                register_single_scheme(s, desc, &exe_path_str)?;
            }
        } else if let Some((s, desc)) = SCHEMES.iter().find(|(s, _)| *s == scheme) {
            register_single_scheme(s, desc, &exe_path_str)?;
        } else {
            return Err(format!("Unknown protocol scheme '{}'", scheme));
        }
    }

    #[cfg(target_os = "linux")]
    {
        register_linux_protocols(scheme, &exe_path_str)?;
    }

    #[cfg(target_os = "macos")]
    {
        crate::logger::log(&format!("register_specific_scheme [macOS]: Registered via CFBundleURLSchemes in Info.plist for {}", scheme));
    }

    Ok(exe_path_str)
}

/// Register all protocols (no admin privileges needed)
pub fn register_protocol() -> Result<String, String> {
    register_specific_scheme("all")
}

/// Unregister a specific scheme ("palmodmanager", "nxm", or "all")
pub fn unregister_specific_scheme(scheme: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let classes_key = hkcu
            .open_subkey("Software\\Classes")
            .map_err(|e| format!("Failed to open Software\\Classes: {}", e))?;

        if scheme == "all" {
            for (s, _) in SCHEMES {
                let _ = classes_key.delete_subkey_all(s);
                crate::logger::log(&format!("unregister_protocol: Removed {} protocol key", s));
            }
        } else {
            let _ = classes_key.delete_subkey_all(scheme);
            crate::logger::log(&format!("unregister_protocol: Removed {} protocol key", scheme));
        }
    }

    #[cfg(target_os = "linux")]
    {
        unregister_linux_protocol(scheme)?;
    }

    #[cfg(target_os = "macos")]
    {
        crate::logger::log(&format!("unregister_specific_scheme [macOS]: No-op for {}", scheme));
    }

    Ok(())
}

/// Unregister palmodmanager:// and nxm://
pub fn unregister_protocol() -> Result<(), String> {
    unregister_specific_scheme("all")
}

/// Automatically ensures the OAuth login protocol (palmodmanager://) is registered to the current running executable
pub fn auto_register_if_needed() -> Result<(), String> {
    let current_exe = match get_current_exe_path() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return Ok(()),
    };

    #[cfg(target_os = "windows")]
    {
        let pmm_status = check_single_scheme_status("palmodmanager", &current_exe);
        match pmm_status {
            ProtocolStatus::Registered { .. } => Ok(()),
            ProtocolStatus::NotRegistered | ProtocolStatus::OutdatedPath { .. } => {
                let _ = register_specific_scheme("palmodmanager");
                Ok(())
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let pmm_handler = check_linux_scheme_handler("palmodmanager");
        if pmm_handler.as_deref() != Some("palmodmanager.desktop") {
            let _ = register_linux_protocols("palmodmanager", &current_exe);
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    Ok(())
}


