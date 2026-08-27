use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;

/// URL to download prebuilt retoc binary from latest GitHub release
#[cfg(target_os = "windows")]
const RETOC_ZIP_URL: &str = "https://github.com/trumank/retoc/releases/latest/download/retoc_cli-x86_64-pc-windows-msvc.zip";
#[cfg(target_os = "linux")]
const RETOC_ZIP_URL: &str = "https://github.com/trumank/retoc/releases/latest/download/retoc_cli-x86_64-unknown-linux-gnu.tar.xz";
#[cfg(target_os = "macos")]
const RETOC_ZIP_URL: &str = "https://github.com/trumank/retoc/releases/latest/download/retoc_cli-aarch64-apple-darwin.tar.xz";

/// Resolves the local path to the retoc binary inside tools directory
pub fn get_retoc_bin_path(app_data_dir: &Path) -> PathBuf {
    let tools_dir = app_data_dir.join("tools").join("retoc");
    #[cfg(target_os = "windows")]
    {
        tools_dir.join("retoc.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        tools_dir.join("retoc")
    }
}

/// Ensures the retoc binary is available locally, downloading it on demand if missing
pub async fn ensure_retoc_available(app_data_dir: &Path) -> Result<PathBuf, String> {
    let bin_path = get_retoc_bin_path(app_data_dir);
    if bin_path.exists() {
        return Ok(bin_path);
    }

    let tools_dir = app_data_dir.join("tools").join("retoc");
    fs::create_dir_all(&tools_dir).map_err(|e| format!("Failed to create retoc tools directory: {e}"))?;

    crate::logger::log(&format!("Downloading latest retoc release from {}", RETOC_ZIP_URL));

    let client = reqwest::Client::builder()
        .user_agent("PalModManager")
        .build()
        .map_err(|e| format!("Failed to build HTTP client for retoc download: {e}"))?;

    let response = client
        .get(RETOC_ZIP_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to download retoc: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to download retoc from GitHub (HTTP {})", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read retoc download body: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        let reader = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Failed to open retoc zip: {e}"))?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| format!("Failed to read entry from retoc zip: {e}"))?;
            let name = file.name().to_string();
            if name.ends_with("retoc.exe") || name == "retoc.exe" {
                let mut out = File::create(&bin_path).map_err(|e| format!("Failed to create retoc.exe: {e}"))?;
                std::io::copy(&mut file, &mut out).map_err(|e| format!("Failed to extract retoc.exe: {e}"))?;
                break;
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix-like systems write raw binary and set executable permission
        fs::write(&bin_path, &bytes).map_err(|e| format!("Failed to write retoc binary: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&bin_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&bin_path, perms);
            }
        }
    }

    if bin_path.exists() {
        crate::logger::log(&format!("retoc successfully installed to {:?}", bin_path));
        Ok(bin_path)
    } else {
        Err("Failed to extract retoc executable from downloaded archive".to_string())
    }
}

/// Converts a legacy Unreal Engine .pak archive into Game Pass compatible Zen IoStore containers (.utoc + .ucas)
pub async fn convert_pak_to_gamepass_zen(
    pak_path: &Path,
    app_data_dir: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    if !pak_path.exists() {
        return Err(format!("Pak file does not exist: {:?}", pak_path));
    }

    let retoc_bin = ensure_retoc_available(app_data_dir).await?;

    let utoc_target = pak_path.with_extension("utoc");
    let ucas_target = pak_path.with_extension("ucas");

    crate::logger::log(&format!(
        "Running retoc conversion: {:?} to-zen {:?} {:?} --version UE5_1",
        retoc_bin, pak_path, utoc_target
    ));

    #[cfg(target_os = "windows")]
    let output = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new(&retoc_bin)
            .args([
                "to-zen",
                pak_path.to_str().ok_or("Invalid pak path")?,
                utoc_target.to_str().ok_or("Invalid utoc path")?,
                "--version",
                "UE5_1",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to execute retoc: {e}"))?
    };

    #[cfg(not(target_os = "windows"))]
    let output = Command::new(&retoc_bin)
        .args([
            "to-zen",
            pak_path.to_str().ok_or("Invalid pak path")?,
            utoc_target.to_str().ok_or("Invalid utoc path")?,
            "--version",
            "UE5_1",
        ])
        .output()
        .map_err(|e| format!("Failed to execute retoc: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "retoc conversion failed (exit code {:?}): {}{}",
            output.status.code(),
            stderr,
            stdout
        ));
    }

    if !utoc_target.exists() || !ucas_target.exists() {
        return Err("retoc completed but .utoc or .ucas companion files were not produced".to_string());
    }

    crate::logger::log(&format!(
        "Successfully converted {:?} -> {:?} & {:?}",
        pak_path, utoc_target, ucas_target
    ));

    Ok((utoc_target, ucas_target))
}
