use std::fs;
use std::io::Read;
use std::path::Path;
use super::types::ArchiveFormat;
use super::naming::is_forbidden;

pub fn extract_nexus_id_from_path(zip_path: &str) -> Option<u32> {
    let filename = Path::new(zip_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    crate::nexus::extract_nexus_id(&filename)
}

pub fn find_root_folder(files: &[String]) -> Option<String> {
    for name in files {
        if name.contains('/') {
            let first = name.split('/').next().unwrap_or("");
            if !first.is_empty() && !is_forbidden(first) {
                return Some(first.to_string());
            }
        }
    }
    None
}

pub fn list_7z_files(path: &str) -> Result<Vec<String>, String> {
    let reader = sevenz_rust::SevenZReader::open(Path::new(path), sevenz_rust::Password::empty())
        .map_err(|e| format!("Cannot open .7z archive: {}", e))?;
    let mut files = Vec::new();
    for entry in reader.archive().files.iter() {
        if !entry.is_directory() {
            files.push(entry.name().replace('\\', "/"));
        }
    }
    Ok(files)
}

pub fn list_rar_files(path: &str) -> Result<Vec<String>, String> {
    let mut cmd = std::process::Command::new("tar");
    cmd.args(&["-tf", path]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd.output().map_err(|e| format!("Failed to run tar: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("RAR archive reading failed: {}", err.trim()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(|l| l.trim().replace('\\', "/"))
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn detect_archive_format(path: &Path) -> ArchiveFormat {
    if let Ok(mut file) = fs::File::open(path) {
        let mut header = [0u8; 16];
        if let Ok(n) = file.read(&mut header) {
            if n >= 4 && &header[0..4] == b"PK\x03\x04" {
                return ArchiveFormat::Zip;
            }
            if n >= 6 && &header[0..6] == b"7z\xBC\xAF\x27\x1C" {
                return ArchiveFormat::SevenZip;
            }
            if n >= 6 && &header[0..6] == b"Rar!\x1A\x07" {
                return ArchiveFormat::Rar;
            }
            if n >= 4 && &header[0..4] == b"Rar!" {
                return ArchiveFormat::Rar;
            }
        }
    }

    let lower = path.to_string_lossy().to_lowercase();
    if lower.ends_with(".7z") {
        ArchiveFormat::SevenZip
    } else if lower.ends_with(".rar") {
        ArchiveFormat::Rar
    } else if lower.ends_with(".pak") {
        ArchiveFormat::RawPak
    } else {
        ArchiveFormat::Zip
    }
}
