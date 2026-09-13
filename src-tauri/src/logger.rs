use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;

#[cfg(target_os = "windows")]
mod win_console {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn AllocConsole() -> i32;
        fn FreeConsole() -> i32;
        fn GetConsoleWindow() -> *mut c_void;
    }

    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hWnd: *mut c_void, nCmdShow: i32) -> i32;
    }

    const SW_HIDE: i32 = 0;
    const SW_SHOW: i32 = 5;

    pub fn set_visible(visible: bool) {
        unsafe {
            let hwnd = GetConsoleWindow();
            if visible {
                if hwnd.is_null() {
                    AllocConsole();
                } else {
                    ShowWindow(hwnd, SW_SHOW);
                }
            } else {
                if !hwnd.is_null() {
                    ShowWindow(hwnd, SW_HIDE);
                    FreeConsole();
                }
            }
        }
    }
}

pub fn set_console_visibility(visible: bool) {
    #[cfg(target_os = "windows")]
    win_console::set_visible(visible);
    #[cfg(not(target_os = "windows"))]
    let _ = visible;
}

fn get_log_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA").ok().map(|l| PathBuf::from(l).join("PalModManager"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".local").join("share").join("PalModManager"))
    }
}

pub fn init_logger() {
    if let Some(log_dir) = get_log_dir() {
        let _ = std::fs::create_dir_all(&log_dir);
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        // Truncate (reset) main app.log file on each application launch
        let log_file = log_dir.join("app.log");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file)
        {
            let header = format!("=== PalModManager Log Session Started [{}] ===\n", timestamp);
            let _ = file.write_all(header.as_bytes());
        }

        // Truncate (reset) technical debug.log file on each application launch
        let debug_file = log_dir.join("debug.log");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&debug_file)
        {
            let header = format!("=== PalModManager Technical Debug Log Started [{}] ===\n\n", timestamp);
            let _ = file.write_all(header.as_bytes());
        }
    }
}

pub fn log(msg: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let log_line = format!("[{}] {}\n", timestamp, msg);
    
    eprint!("{}", log_line);
    
    if let Some(log_dir) = get_log_dir() {
        let log_file = log_dir.join("app.log");
        let _ = std::fs::create_dir_all(&log_dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = file.write_all(log_line.as_bytes());
        }
    }
}

pub fn log_debug(msg: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let log_line = format!("[{}] {}\n", timestamp, msg);

    if let Some(log_dir) = get_log_dir() {
        let log_file = log_dir.join("debug.log");
        let _ = std::fs::create_dir_all(&log_dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = file.write_all(log_line.as_bytes());
        }
    }
}

pub fn log_sav_start(world_name: &str, sav_name: &str, file_size_bytes: u64, file_path: &str) {
    let size_formatted = if file_size_bytes >= 1024 * 1024 {
        format!("{:.2} MB ({} bytes)", file_size_bytes as f64 / (1024.0 * 1024.0), file_size_bytes)
    } else if file_size_bytes >= 1024 {
        format!("{:.1} KB ({} bytes)", file_size_bytes as f64 / 1024.0, file_size_bytes)
    } else {
        format!("{} bytes", file_size_bytes)
    };
    let banner = format!(
        "\n================================================================================\n\
         >>> INICIANDO REVISIÓN: [{}] | Mundo: \"{}\"\n\
             Ruta: {}\n\
             Tamaño en disco: {}\n\
         ================================================================================\n",
        sav_name, world_name, file_path, size_formatted
    );

    if let Some(log_dir) = get_log_dir() {
        let log_file = log_dir.join("debug.log");
        let _ = std::fs::create_dir_all(&log_dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = file.write_all(banner.as_bytes());
        }
    }
}

pub fn log_sav_finish(sav_name: &str, duration_ms: u128, status: &str) {
    let footer = format!(
        "<<< FINALIZADO: [{}] en {} ms ({})\n\
         --------------------------------------------------------------------------------\n",
        sav_name, duration_ms, status
    );

    if let Some(log_dir) = get_log_dir() {
        let log_file = log_dir.join("debug.log");
        let _ = std::fs::create_dir_all(&log_dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = file.write_all(footer.as_bytes());
        }
    }
}
