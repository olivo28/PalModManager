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

pub fn init_logger() {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let log_dir = PathBuf::from(local_app_data).join("PalModManager");
        let log_file = log_dir.join("app.log");
        let _ = std::fs::create_dir_all(&log_dir);
        // Truncate (reset) log file on each application launch
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file)
        {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let header = format!("=== PalModManager Log Session Started [{}] ===\n", timestamp);
            let _ = file.write_all(header.as_bytes());
        }
    }
}

pub fn log(msg: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let log_line = format!("[{}] {}\n", timestamp, msg);
    
    eprint!("{}", log_line);
    
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let log_dir = PathBuf::from(local_app_data).join("PalModManager");
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
