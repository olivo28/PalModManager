use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

pub fn start_fs_watcher(app_handle: AppHandle, paths_to_watch: Vec<PathBuf>) {
    std::thread::spawn(move || {
        let (tx, rx) = channel();

        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        ) {
            Ok(w) => w,
            Err(e) => {
                crate::logger::log(&format!("Failed to create filesystem watcher: {:?}", e));
                return;
            }
        };

        for path in &paths_to_watch {
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    crate::logger::log(&format!("Watcher failed to watch {:?}: {:?}", path, e));
                } else {
                    crate::logger::log(&format!("Filesystem watcher active on {:?}", path));
                }
            }
        }

        let mut pending_paths: Vec<String> = Vec::new();
        let mut last_event_time = std::time::Instant::now();

        loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(event) => {
                    let should_track = match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => true,
                        _ => false,
                    };
                    if should_track {
                        for p in event.paths {
                            let p_str = p.to_string_lossy().to_string();
                            if !p_str.ends_with('~') && !p_str.ends_with(".tmp") && !p_str.ends_with(".lock") {
                                if !pending_paths.contains(&p_str) {
                                    pending_paths.push(p_str);
                                }
                            }
                        }
                        last_event_time = std::time::Instant::now();
                    }
                }
                Err(_) => {
                    if !pending_paths.is_empty() && last_event_time.elapsed() >= Duration::from_millis(150) {
                        let payload = serde_json::json!({
                            "paths": pending_paths.clone(),
                        });
                        let _ = app_handle.emit("fs:file-changed", payload);
                        pending_paths.clear();
                    }
                }
            }
        }
    });
}
