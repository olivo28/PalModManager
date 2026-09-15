use std::path::Path;

/// Opens a file or directory path in the system's native file explorer/manager.
/// In Linux/AppImage, explicitly strips LD_LIBRARY_PATH from the child process environment
/// so that the host file manager (Dolphin, Nautilus, Thunar, etc.) executes using host system libraries
/// instead of conflicting with bundled AppImage libraries.
pub fn open_path_in_system(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        open::that(path).map_err(|e| format!("Failed to open path: {}", e))
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open path: {}", e))?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(path);
        cmd.env_remove("LD_LIBRARY_PATH");
        cmd.spawn().map_err(|e| format!("Failed to open path via xdg-open: {}", e))?;
        Ok(())
    }
}

/// Opens an external URL or URI scheme in the default browser / scheme handler.
/// In Linux/AppImage, explicitly strips LD_LIBRARY_PATH to avoid symbol collision with host browsers.
pub fn open_url_in_system(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(url);
        cmd.env_remove("LD_LIBRARY_PATH");
        cmd.spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
}
