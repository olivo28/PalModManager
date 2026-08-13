use std::path::Path;
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub async fn launch_game(state: State<'_, AppState>) -> Result<(), String> {
    let game_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.game_path.clone()
    };

    if game_path.is_empty() {
        return Err("Game path is not set in settings".to_string());
    }

    let path = Path::new(&game_path);
    let wingdk = path.join("Pal").join("Binaries").join("WinGDK");
    let is_xbox = wingdk.exists();

    if is_xbox {
        let exe_path = wingdk.join("Palworld-WinGDK-Shipping.exe");
        if exe_path.exists() {
            std::process::Command::new(&exe_path)
                .current_dir(wingdk)
                .spawn()
                .map_err(|e| format!("Failed to launch Game Pass version: {}", e))?;
        } else {
            return Err("Palworld-WinGDK-Shipping.exe not found in Binaries/WinGDK".to_string());
        }
    } else {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("cmd")
                .args(&["/C", "start", "", "steam://run/1623730"])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn()
                .map_err(|e| format!("Failed to launch Steam version: {}", e))?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let win64_exe = path.join("Pal").join("Binaries").join("Win64").join("Palworld-Win64-Shipping.exe");
            if win64_exe.exists() {
                std::process::Command::new(&win64_exe)
                    .current_dir(win64_exe.parent().unwrap())
                    .spawn()
                    .map_err(|e| format!("Failed to launch game: {}", e))?;
            } else {
                return Err("Palworld-Win64-Shipping.exe not found".to_string());
            }
        }
    }

    Ok(())
}
