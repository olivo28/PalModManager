use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::Duration;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager};

static CRASH_WATCHER_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_crash_watcher_enabled(enabled: bool) {
    CRASH_WATCHER_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn is_crash_watcher_enabled() -> bool {
    CRASH_WATCHER_ENABLED.load(Ordering::Relaxed)
}

// Reactive background watcher monitoring Unreal Engine and UE4SS crash directories
pub fn start_crash_watcher(app_handle: AppHandle, game_root: PathBuf) {
    std::thread::spawn(move || {
        let (tx, rx) = channel();

        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                crate::logger::log(&format!("Crash Doctor: Failed to initialize watcher: {:?}", e));
                return;
            }
        };

        // Unreal Engine 5 crashes directory in %LOCALAPPDATA%/Pal/Saved/Crashes/
        let ue_crash_dir = get_unreal_crash_dir(&game_root);
        if let Some(ref dir) = ue_crash_dir {
            if dir.exists() {
                let _ = watcher.watch(dir, RecursiveMode::Recursive);
                crate::logger::log(&format!("Crash Doctor: Watching UE5 crash directory: {:?}", dir));
            }
        }

        // UE4SS binaries directory (Pal/Binaries/Win64/)
        let binaries_dir = game_root.join("Pal").join("Binaries").join("Win64");
        if binaries_dir.exists() {
            let _ = watcher.watch(&binaries_dir, RecursiveMode::NonRecursive);
            crate::logger::log(&format!("Crash Doctor: Watching UE4SS binaries directory: {:?}", binaries_dir));
        }

        let mut last_detected_time = std::time::Instant::now() - Duration::from_secs(60);

        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(event) => {
                    if !is_crash_watcher_enabled() {
                        continue;
                    }

                    let is_creation = match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => true,
                        _ => false,
                    };

                    if is_creation && last_detected_time.elapsed() >= Duration::from_secs(5) {
                        for path in event.paths {
                            let p_str = path.to_string_lossy().to_lowercase();
                            if p_str.contains("crashcontext.runtime-xml") 
                                || p_str.ends_with(".dmp") 
                                || p_str.contains("uecc-windows") {
                                
                                last_detected_time = std::time::Instant::now();
                                crate::logger::log(&format!("Crash Doctor: Detected new crash artifact: {:?}", path));

                                // Auto-restaurar ventana Tauri a primer plano
                                focus_and_alert_window(&app_handle);

                                // Emite el evento 'crash:detected' al frontend
                                let crash_folder = if path.is_file() {
                                    path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
                                } else {
                                    path.to_string_lossy().to_string()
                                };

                                let _ = app_handle.emit("crash:detected", serde_json::json!({
                                    "crashFolder": crash_folder,
                                    "artifact": path.to_string_lossy().to_string(),
                                }));
                                break;
                            }
                        }
                    }
                }
                Err(_) => {
                    // Timeout normal del loop
                }
            }
        }
    });
}

fn focus_and_alert_window(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
        crate::logger::log("Crash Doctor: Restored main window to foreground upon game crash.");
    }
}

pub fn get_unreal_crash_dir(game_root: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data).join("Pal").join("Saved").join("Crashes");
            if p.exists() {
                return Some(p);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // En Proton / Steam Deck: compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/Crashes/
        let mut curr = game_root.to_path_buf();
        while let Some(parent) = curr.parent() {
            if parent.ends_with("steamapps") {
                let proton_crashes = parent
                    .join("compatdata")
                    .join("1623730")
                    .join("pfx")
                    .join("drive_c")
                    .join("users")
                    .join("steamuser")
                    .join("AppData")
                    .join("Local")
                    .join("Pal")
                    .join("Saved")
                    .join("Crashes");
                if proton_crashes.exists() {
                    return Some(proton_crashes);
                }
            }
            curr = parent.to_path_buf();
        }
    }

    // Fallback: revisar dentro de game_root/Pal/Saved/Crashes
    let internal = game_root.join("Pal").join("Saved").join("Crashes");
    if internal.exists() {
        return Some(internal);
    }

    None
}
