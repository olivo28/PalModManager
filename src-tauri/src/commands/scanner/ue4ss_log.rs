// UE4SS Runtime Log Reader & Diagnostics
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ue4ssLogEntry {
    pub line_number: usize,
    pub timestamp: Option<String>,
    pub level: String, // "info" | "warning" | "error" | "crash" | "mod"
    pub tag: Option<String>,
    pub message: String,
    pub mod_name: Option<String>,
    pub script_file: Option<String>,
    pub script_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ue4ssModStatus {
    pub name: String,
    pub status: String, // "loaded" | "failed" | "warning" | "disabled"
    pub mod_type: Option<String>, // "palschema" | "lua" | "cpp" | "native"
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ue4ssLogDiagnostics {
    pub file_path: String,
    pub exists: bool,
    pub file_size_bytes: u64,
    pub last_modified: Option<String>,
    pub total_lines: usize,
    pub loaded_mods: Vec<Ue4ssModStatus>,
    pub error_count: usize,
    pub warning_count: usize,
    pub entries: Vec<Ue4ssLogEntry>,
}

/// Locates the active ue4ss.log file across known locations
pub fn find_ue4ss_log_path(game_path: &Path) -> (PathBuf, bool) {
    let binaries = crate::dependency_checker::get_binaries_dir(game_path);

    let candidates = [
        binaries.join("ue4ss.log"),
        binaries.join("UE4SS.log"),
        binaries.join("ue4ss").join("ue4ss.log"),
        binaries.join("ue4ss").join("UE4SS.log"),
        game_path.join("Pal").join("Binaries").join("Win64").join("ue4ss.log"),
        game_path.join("Pal").join("Binaries").join("Win64").join("UE4SS.log"),
        game_path.join("Pal").join("Binaries").join("WinGDK").join("ue4ss.log"),
        game_path.join("Pal").join("Binaries").join("WinGDK").join("UE4SS.log"),
        game_path.join("ue4ss.log"),
        game_path.join("UE4SS.log"),
    ];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_file() {
            return (candidate.clone(), true);
        }
    }

    // Default to the standard location even if it does not exist yet
    (binaries.join("ue4ss.log"), false)
}

/// Parse raw log content into structured diagnostics
pub fn parse_ue4ss_log_content(content: &str, file_path: &Path) -> Ue4ssLogDiagnostics {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let mut entries = Vec::new();
    let mut loaded_mods_map: std::collections::BTreeMap<String, Ue4ssModStatus> = std::collections::BTreeMap::new();
    let mut error_count = 0;
    let mut warning_count = 0;

    for (idx, raw_line) in lines.iter().enumerate() {
        let line_number = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse timestamp: [YYYY-MM-DD HH:MM:SS.mmm] or [HH:MM:SS]
        let mut timestamp = None;
        let mut rest = line;
        if rest.starts_with('[') {
            if let Some(end_bracket) = rest.find(']') {
                let inside = &rest[1..end_bracket];
                // Check if bracket contains digits and colons/hyphens
                if inside.chars().any(|c| c.is_ascii_digit()) && inside.contains(':') {
                    timestamp = Some(inside.to_string());
                    rest = rest[end_bracket + 1..].trim();
                }
            }
        }

        // Parse tag: e.g. [UE4SS], [Mods], [Lua], [Error], [Warning], [Crash]
        let mut tag = None;
        if rest.starts_with('[') {
            if let Some(end_bracket) = rest.find(']') {
                tag = Some(rest[1..end_bracket].to_string());
            }
        }

        let lower = line.to_lowercase();
        let is_crash = lower.contains("[crash]") || lower.contains("unhandled exception") || lower.contains("fatal error");
        let is_error = !is_crash && (
            lower.contains("[error]")
            || lower.contains("error:")
            || lower.contains("lua error:")
            || lower.contains("failed to load")
            || lower.contains("failed to find")
            || lower.contains("was unable to install mod")
            || lower.contains("is not installable")
            || lower.contains("dlls folder must contain")
        );
        let is_warning = !is_crash && !is_error && (lower.contains("[warning]") || lower.contains("warning:"));
        let is_mod_lifecycle = !is_crash && !is_error && !is_warning && (
            lower.contains("[mods]")
            || lower.contains("[palschema]")
            || lower.contains("starting mod")
            || lower.contains("loading mod")
            || lower.contains("loaded mod")
            || lower.contains("starting lua mod")
            || lower.contains("starting c++ mod")
            || lower.contains("starting native mod")
            || lower.contains("mod '")
            || (lower.contains("[lua]") && (lower.contains("loaded") || lower.contains("starting") || lower.contains("registered")))
        );

        let level = if is_crash {
            error_count += 1;
            "crash"
        } else if is_error {
            error_count += 1;
            "error"
        } else if is_warning {
            warning_count += 1;
            "warning"
        } else if is_mod_lifecycle {
            "mod"
        } else {
            "info"
        };

        // Mod discovery and script error detection
        let mut mod_name = None;
        let mut script_file = None;
        let mut script_line = None;

        // Pattern 1.0: C++ or mod load failure
        // e.g. "Failed to load C++ mod ModIntegratedStorageCpp..." or "Failed to load dll <...> for mod PalSchema..."
        if let Some(pos) = line.find("Failed to load C++ mod ") {
            let name = line[pos + 23..].trim_start_matches('\'').split([',', '\'']).next().unwrap_or("").trim().to_string();
            if !name.is_empty() {
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "failed".to_string(),
                    mod_type: Some("cpp".to_string()),
                    details: Some(line.to_string()),
                });
                mod_name = Some(name);
            }
        }
        else if let Some(pos) = line.find("for mod ") {
            let name = line[pos + 8..].trim_start_matches('\'').split([',', '\'']).next().unwrap_or("").trim().to_string();
            if !name.is_empty() {
                let existing_type = loaded_mods_map.get(&name).and_then(|m| m.mod_type.clone());
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "failed".to_string(),
                    mod_type: existing_type.or_else(|| Some("cpp".to_string())),
                    details: Some(line.to_string()),
                });
                mod_name = Some(name);
            }
        }
        else if let Some(pos) = line.find("Was unable to install mod '") {
            let sub = &line[pos + 27..];
            if let Some(end_quote) = sub.find('\'') {
                let name = sub[..end_quote].to_string();
                let existing_type = loaded_mods_map.get(&name).and_then(|m| m.mod_type.clone());
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "failed".to_string(),
                    mod_type: existing_type,
                    details: Some(line.to_string()),
                });
                mod_name = Some(name);
            }
        }
        // Pattern 1.1: PalSchema table mod loading
        // e.g. "[PalSchema] Loading mod: ZZZ_MelwenMods - Upgradable Pal Spheres"
        else if let Some(pos) = line.find("[PalSchema] Loading mod: ") {
            let name = line[pos + "[PalSchema] Loading mod: ".len()..].trim().trim_end_matches(',').to_string();
            if !name.is_empty() {
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "loaded".to_string(),
                    mod_type: Some("palschema".to_string()),
                    details: Some("PalSchema table mod".to_string()),
                });
                mod_name = Some(name);
            }
        }
        // Pattern 1.2: PalSchema engine confirmation
        // e.g. "[PalSchema] PalSchema v0.6.5 by Okaetsu loaded."
        else if line.contains("[PalSchema] PalSchema ") && line.contains("loaded") {
            let name = "PalSchema".to_string();
            loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                name: name.clone(),
                status: "loaded".to_string(),
                mod_type: Some("cpp".to_string()),
                details: Some("PalSchema Engine (C++)".to_string()),
            });
            mod_name = Some(name);
        }
        // Pattern 1.3: UE4SS Starting Lua mod 'ModName'
        else if let Some(pos) = line.find("Starting Lua mod '") {
            let sub = &line[pos + "Starting Lua mod '".len()..];
            if let Some(end_quote) = sub.find('\'') {
                let name = sub[..end_quote].to_string();
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "loaded".to_string(),
                    mod_type: Some("lua".to_string()),
                    details: Some("UE4SS Lua mod".to_string()),
                });
                mod_name = Some(name);
            }
        }
        // Pattern 1.4: UE4SS Starting C++ mod 'ModName' or Starting native mod 'ModName'
        else if let Some(pos) = line.find("Starting C++ mod '") {
            let sub = &line[pos + "Starting C++ mod '".len()..];
            if let Some(end_quote) = sub.find('\'') {
                let name = sub[..end_quote].to_string();
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "loaded".to_string(),
                    mod_type: Some("cpp".to_string()),
                    details: Some("UE4SS C++ native mod".to_string()),
                });
                mod_name = Some(name);
            }
        } else if let Some(pos) = line.find("Starting native mod '") {
            let sub = &line[pos + "Starting native mod '".len()..];
            if let Some(end_quote) = sub.find('\'') {
                let name = sub[..end_quote].to_string();
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "loaded".to_string(),
                    mod_type: Some("native".to_string()),
                    details: Some("UE4SS native mod".to_string()),
                });
                mod_name = Some(name);
            }
        }
        // Pattern 1.5: Generic Starting mod 'ModName'
        else if let Some(pos) = line.find("Starting mod '") {
            let sub = &line[pos + "Starting mod '".len()..];
            if let Some(end_quote) = sub.find('\'') {
                let name = sub[..end_quote].to_string();
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "loaded".to_string(),
                    mod_type: Some("lua".to_string()),
                    details: Some("Initialized successfully".to_string()),
                });
                mod_name = Some(name);
            }
        }
        // Pattern 1.6: Loading mod: ModName
        else if let Some(pos) = line.find("Loading mod: ") {
            let sub = &line[pos + "Loading mod: ".len()..];
            let name = sub.trim().trim_end_matches(',').to_string();
            if !name.is_empty() {
                let mod_type = if line.contains("[PalSchema]") { Some("palschema".to_string()) } else { None };
                loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                    name: name.clone(),
                    status: "loaded".to_string(),
                    mod_type,
                    details: Some("Loading started".to_string()),
                });
                mod_name = Some(name);
            }
        }
        // Pattern 1.7: Mod 'ModName' loaded / disabled in mods.txt / error
        else if let Some(pos) = line.find("Mod '") {
            let sub = &line[pos + "Mod '".len()..];
            if let Some(end_quote) = sub.find('\'') {
                let name = sub[..end_quote].to_string();
                if line.contains("disabled in mods.txt") {
                    loaded_mods_map.entry(name.clone()).or_insert_with(|| Ue4ssModStatus {
                        name: name.clone(),
                        status: "disabled".to_string(),
                        mod_type: Some("lua".to_string()),
                        details: Some("Disabled in mods.txt".to_string()),
                    });
                } else if line.contains("failed") || line.contains("error") {
                    let existing_type = loaded_mods_map.get(&name).and_then(|m| m.mod_type.clone());
                    loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                        name: name.clone(),
                        status: "failed".to_string(),
                        mod_type: existing_type,
                        details: Some(line.to_string()),
                    });
                } else if line.contains("loaded") || line.contains("successfully") {
                    let existing_type = loaded_mods_map.get(&name).and_then(|m| m.mod_type.clone());
                    loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                        name: name.clone(),
                        status: "loaded".to_string(),
                        mod_type: existing_type,
                        details: Some("Loaded".to_string()),
                    });
                }
                mod_name = Some(name);
            }
        }
        // Pattern 1.8: [Lua] [ModName] Loaded ...
        else if let Some(pos) = line.find("[Lua] [") {
            let sub = &line[pos + "[Lua] [".len()..];
            if let Some(end_bracket) = sub.find(']') {
                let name = sub[..end_bracket].trim().to_string();
                if !name.is_empty() {
                    if line.to_lowercase().contains("loaded") {
                        loaded_mods_map.insert(name.clone(), Ue4ssModStatus {
                            name: name.clone(),
                            status: "loaded".to_string(),
                            mod_type: Some("lua".to_string()),
                            details: Some("Loaded successfully".to_string()),
                        });
                    }
                    mod_name = Some(name);
                }
            }
        }

        // Pattern 2: Lua script errors with file and line
        // e.g., "Mods/MyMod/scripts/main.lua:45: attempt to index nil"
        // or "Pal/Binaries/Win64/ue4ss/Mods/MyMod/scripts/main.lua:45:"
        if is_error || is_warning || is_crash {
            if let Some(mods_idx) = line.find("Mods/") {
                let after_mods = &line[mods_idx + 5..];
                let parts: Vec<&str> = after_mods.splitn(3, '/').collect();
                if parts.len() >= 2 {
                    let extracted_mod = parts[0].to_string();
                    mod_name = Some(extracted_mod.clone());

                    // Check for file:line:
                    let remainder = parts[1..].join("/");
                    if let Some(colon_pos) = remainder.find(':') {
                        let path_part = &remainder[..colon_pos];
                        let after_colon = &remainder[colon_pos + 1..];
                        if let Some(second_colon) = after_colon.find(':') {
                            let line_str = &after_colon[..second_colon];
                            if let Ok(ln) = line_str.parse::<usize>() {
                                script_file = Some(path_part.to_string());
                                script_line = Some(ln);
                            }
                        } else {
                            script_file = Some(path_part.to_string());
                        }
                    }

                    // Flag mod status if failed
                    if is_error || is_crash {
                        let existing_type = loaded_mods_map.get(&extracted_mod).and_then(|m| m.mod_type.clone());
                        loaded_mods_map.insert(extracted_mod.clone(), Ue4ssModStatus {
                            name: extracted_mod,
                            status: "failed".to_string(),
                            mod_type: existing_type.or_else(|| Some("lua".to_string())),
                            details: Some(line.to_string()),
                        });
                    }
                }
            }
        }

        entries.push(Ue4ssLogEntry {
            line_number,
            timestamp,
            level: level.to_string(),
            tag,
            message: line.to_string(),
            mod_name,
            script_file,
            script_line,
        });
    }

    let loaded_mods: Vec<Ue4ssModStatus> = loaded_mods_map.into_values().collect();

    Ue4ssLogDiagnostics {
        file_path: file_path.to_string_lossy().to_string(),
        exists: file_path.exists(),
        file_size_bytes: file_path.metadata().map(|m| m.len()).unwrap_or(0),
        last_modified: file_path.metadata().ok().and_then(|m| m.modified().ok()).map(|time| {
            let datetime: chrono::DateTime<chrono::Local> = time.into();
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        }),
        total_lines,
        loaded_mods,
        error_count,
        warning_count,
        entries,
    }
}

/// Resolves target log path: uses custom_path if provided, otherwise discovers game's ue4ss.log
pub fn resolve_ue4ss_log_target(
    custom_path: Option<&str>,
    state: &State<'_, AppState>,
) -> Result<(PathBuf, bool), String> {
    if let Some(cp) = custom_path {
        let trimmed = cp.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            let exists = path.exists() && path.is_file();
            return Ok((path, exists));
        }
    }

    let game_path_str = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };

    if game_path_str.trim().is_empty() {
        return Err("Game path is not configured in settings".to_string());
    }

    let game_path = PathBuf::from(&game_path_str);
    Ok(find_ue4ss_log_path(&game_path))
}

/// Truncates log entries to a given limit while strictly preserving all errors, crashes, and warnings.
pub fn truncate_entries_preserving_severity(entries: Vec<Ue4ssLogEntry>, limit: usize) -> Vec<Ue4ssLogEntry> {
    if entries.len() <= limit { return entries; }

    let (mut priority, normal): (Vec<_>, Vec<_>) = entries.into_iter()
        .partition(|e| e.level == "error" || e.level == "crash" || e.level == "warning");

    let normal_take = if priority.len() >= limit { 0 } else { limit - priority.len() };
    let start_idx = if normal.len() > normal_take { normal.len() - normal_take } else { 0 };

    priority.extend(normal.into_iter().skip(start_idx));
    priority.sort_by_key(|e| e.line_number);
    priority
}

/// Tauri Command: Retrieve structured diagnostics and entries from ue4ss.log
#[tauri::command]
pub async fn get_ue4ss_log_diagnostics(
    max_entries: Option<usize>,
    custom_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Ue4ssLogDiagnostics, String> {
    let (log_path, exists) = resolve_ue4ss_log_target(custom_path.as_deref(), &state)?;

    if !exists {
        return Ok(Ue4ssLogDiagnostics {
            file_path: log_path.to_string_lossy().to_string(),
            exists: false,
            file_size_bytes: 0,
            last_modified: None,
            total_lines: 0,
            loaded_mods: Vec::new(),
            error_count: 0,
            warning_count: 0,
            entries: Vec::new(),
        });
    }

    let content = fs::read_to_string(&log_path)
        .map_err(|e| format!("Failed to read {}: {}", log_path.display(), e))?;

    let mut diag = parse_ue4ss_log_content(&content, &log_path);

    if let Some(limit) = max_entries {
        diag.entries = truncate_entries_preserving_severity(diag.entries, limit);
    }

    Ok(diag)
}

/// Tauri Command: Read raw ue4ss.log content (capped by lines)
#[tauri::command]
pub async fn read_raw_ue4ss_log(
    max_lines: Option<usize>,
    custom_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let (log_path, exists) = resolve_ue4ss_log_target(custom_path.as_deref(), &state)?;

    if !exists {
        return Err(format!("ue4ss.log was not found at {}", log_path.display()));
    }

    let content = fs::read_to_string(&log_path)
        .map_err(|e| format!("Failed to read {}: {}", log_path.display(), e))?;

    if let Some(max) = max_lines {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > max {
            let tail = &lines[lines.len() - max..];
            return Ok(tail.join("\n"));
        }
    }

    Ok(content)
}

/// Tauri Command: Truncate / Clear the active UE4SS log file
#[tauri::command]
pub async fn clear_ue4ss_log(
    custom_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let (log_path, exists) = resolve_ue4ss_log_target(custom_path.as_deref(), &state)?;

    if !exists {
        return Ok(false);
    }

    fs::write(&log_path, "")
        .map_err(|e| format!("Failed to clear {}: {}", log_path.display(), e))?;

    crate::logger::log(&format!("[UE4SS Log] Cleared log file at {}", log_path.display()));
    Ok(true)
}

/// Tauri Command: Copy the physical ue4ss.log file to the system clipboard (CF_HDROP / FileDrop)
#[tauri::command]
pub async fn copy_ue4ss_log_file_to_clipboard(
    custom_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let (log_path, exists) = resolve_ue4ss_log_target(custom_path.as_deref(), &state)?;

    if !exists {
        return Err("ue4ss.log file does not exist yet".to_string());
    }

    let abs_path = log_path.canonicalize().unwrap_or(log_path);

    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let path_str = abs_path.to_string_lossy().replace('\'', "''");
            let cmd = format!("Set-Clipboard -Path '{}'", path_str);

            let output = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("Failed to run clipboard command: {}", e))?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Set-Clipboard error: {}", err));
            }
            crate::logger::log(&format!("[UE4SS Log] Copied log file to clipboard: {}", abs_path.display()));
            Ok(true)
        }
        #[cfg(target_os = "macos")]
        {
            let path_str = abs_path.to_string_lossy().replace('\"', "\\\"");
            let script = format!("set the clipboard to POSIX file \"{}\"", path_str);
            let output = std::process::Command::new("osascript")
                .args(["-e", &script])
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err("Failed to copy file to clipboard on macOS".to_string());
            }
            Ok(true)
        }
        #[cfg(target_os = "linux")]
        {
            let uri = format!("file://{}", abs_path.display());
            let wl = std::process::Command::new("wl-copy")
                .args(["-t", "text/uri-list", &uri])
                .output();
            if wl.is_ok() && wl.unwrap().status.success() {
                return Ok(true);
            }
            let xclip = std::process::Command::new("xclip")
                .args(["-selection", "clipboard", "-t", "text/uri-list"])
                .stdin(std::process::Stdio::piped())
                .spawn();
            if let Ok(mut child) = xclip {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(uri.as_bytes());
                }
                let _ = child.wait();
                return Ok(true);
            }
            Ok(true)
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Tauri Command: Reveal the active ue4ss.log in the operating system's file manager
#[tauri::command]
pub async fn reveal_ue4ss_log_in_explorer(
    custom_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let (log_path, exists) = resolve_ue4ss_log_target(custom_path.as_deref(), &state)?;

    if !exists {
        return Err("ue4ss.log file does not exist yet".to_string());
    }

    let abs_path = log_path.canonicalize().unwrap_or(log_path);

    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let path_str = abs_path.to_string_lossy().to_string();
            let _ = std::process::Command::new("explorer")
                .args(["/select,", &path_str])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
            Ok(true)
        }
        #[cfg(not(windows))]
        {
            if let Some(parent) = abs_path.parent() {
                open::that(parent).map_err(|e| e.to_string())?;
                Ok(true)
            } else {
                Err("Failed to resolve parent directory".to_string())
            }
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[cfg(test)]
#[path = "ue4ss_log_tests.rs"]
mod tests;
