use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::read::ZipArchive;
use super::types::ArchiveFormat;
use super::detection::detect_archive_format;

pub fn extract_7z_to_temp(path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(temp_dir).map_err(|e| format!("Cannot create temp dir: {}", e))?;
    sevenz_rust::decompress_file(Path::new(path), temp_dir)
        .map_err(|e| format!("Failed to extract .7z archive: {}", e))?;
    Ok(temp_dir.to_path_buf())
}

pub fn read_7z_file(path: &str, target_file: &str) -> Option<String> {
    let lower_target = target_file.to_lowercase();
    let mut reader = sevenz_rust::SevenZReader::open(Path::new(path), sevenz_rust::Password::empty()).ok()?;
    let mut content = None;
    let _ = reader.for_each_entries(|entry, reader| {
        let entry_name = entry.name().replace('\\', "/").to_lowercase();
        if entry_name == lower_target || entry_name.ends_with(&format!("/{}", lower_target)) {
            let mut buf = Vec::new();
            if reader.read_to_end(&mut buf).is_ok() {
                if let Ok(s) = String::from_utf8(buf) {
                    content = Some(s);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    });
    content
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RarExtractorKind {
    SevenZip,
    Unrar,
}

pub fn find_cross_platform_rar_extractor() -> Option<(PathBuf, RarExtractorKind)> {
    let candidates = [
        ("7zz", RarExtractorKind::SevenZip),
        ("7z", RarExtractorKind::SevenZip),
        ("7za", RarExtractorKind::SevenZip),
        ("unrar", RarExtractorKind::Unrar),
    ];

    for (bin, kind) in candidates {
        let mut cmd = std::process::Command::new(bin);
        cmd.arg("--help");
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        if let Ok(output) = cmd.output() {
            if output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty() {
                return Some((PathBuf::from(bin), kind));
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let win_paths = [
            (r"C:\Program Files\7-Zip\7z.exe", RarExtractorKind::SevenZip),
            (r"C:\Program Files (x86)\7-Zip\7z.exe", RarExtractorKind::SevenZip),
            (r"C:\Program Files\WinRAR\UnRAR.exe", RarExtractorKind::Unrar),
            (r"C:\Program Files\WinRAR\WinRAR.exe", RarExtractorKind::SevenZip),
        ];
        for (p, kind) in win_paths {
            let path = PathBuf::from(p);
            if path.is_file() {
                return Some((path, kind));
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let unix_paths = [
            ("/usr/bin/7zz", RarExtractorKind::SevenZip),
            ("/usr/bin/7z", RarExtractorKind::SevenZip),
            ("/usr/bin/unrar", RarExtractorKind::Unrar),
            ("/usr/local/bin/7zz", RarExtractorKind::SevenZip),
            ("/usr/local/bin/7z", RarExtractorKind::SevenZip),
            ("/usr/local/bin/unrar", RarExtractorKind::Unrar),
            ("/opt/homebrew/bin/7zz", RarExtractorKind::SevenZip),
            ("/opt/homebrew/bin/7z", RarExtractorKind::SevenZip),
            ("/opt/homebrew/bin/unrar", RarExtractorKind::Unrar),
        ];
        for (p, kind) in unix_paths {
            let path = PathBuf::from(p);
            if path.is_file() {
                return Some((path, kind));
            }
        }
    }

    None
}

pub fn extract_rar_with_fallback(path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    if let Some((extractor, kind)) = find_cross_platform_rar_extractor() {
        let mut cmd = std::process::Command::new(&extractor);
        match kind {
            RarExtractorKind::SevenZip => {
                cmd.args(&[
                    "x",
                    "-y",
                    &format!("-o{}", temp_dir.to_string_lossy()),
                    path,
                ]);
            }
            RarExtractorKind::Unrar => {
                let out_dir = format!("{}/", temp_dir.to_string_lossy().trim_end_matches('/'));
                cmd.args(&["x", "-y", "-idq", path, &out_dir]);
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }

        let output = cmd.output().map_err(|e| format!("Failed to run {:?}: {}", extractor, e))?;
        if output.status.success() {
            return Ok(temp_dir.to_path_buf());
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("RAR extraction fallback failed with {:?}: {}", extractor, err.trim()));
        }
    }

    Err("RAR extraction failed: 'tar' is not available and no external extractor (7-Zip, UnRAR) was found on the system.".to_string())
}

pub fn extract_rar_to_temp(path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(temp_dir).map_err(|e| format!("Cannot create temp dir: {}", e))?;
    let mut cmd = std::process::Command::new("tar");
    cmd.args(&["-xf", path, "-C", &temp_dir.to_string_lossy()]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            return Ok(temp_dir.to_path_buf());
        }
    }

    extract_rar_with_fallback(path, temp_dir)
}

pub fn extract_zip_to_temp(zip_path: &str, temp_dir: &Path) -> Result<PathBuf, String> {
    let p = Path::new(zip_path);
    let format = detect_archive_format(p);

    match format {
        ArchiveFormat::SevenZip => extract_7z_to_temp(zip_path, temp_dir),
        ArchiveFormat::Rar => extract_rar_to_temp(zip_path, temp_dir),
        ArchiveFormat::RawPak => {
            fs::create_dir_all(temp_dir).map_err(|e| format!("Cannot create temp dir: {}", e))?;
            let filename = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let dest = temp_dir.join(&filename);
            fs::copy(zip_path, &dest).map_err(|e| format!("Cannot copy raw pak: {}", e))?;
            Ok(temp_dir.to_path_buf())
        }
        _ => {
            fs::create_dir_all(temp_dir).map_err(|e| format!("Cannot create temp dir: {}", e))?;
            let zip_result = fs::File::open(zip_path)
                .map_err(|e| format!("Cannot open zip: {}", e))
                .and_then(|file| ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e)));

            match zip_result {
                Ok(mut archive) => {
                    let mut crc_failed = false;
                    for i in 0..archive.len() {
                        let mut entry = archive.by_index(i).map_err(|e| format!("Cannot read entry: {}", e))?;
                        let outpath = temp_dir.join(entry.mangled_name());
                        if entry.is_dir() {
                            fs::create_dir_all(&outpath).map_err(|e| format!("Cannot create dir: {}", e))?;
                        } else {
                            if let Some(parent) = outpath.parent() {
                                if !parent.exists() {
                                    fs::create_dir_all(parent)
                                        .map_err(|e| format!("Cannot create parent dir: {}", e))?;
                                }
                            }
                            let mut outfile = fs::File::create(&outpath)
                                .map_err(|e| format!("Cannot create file: {}", e))?;
                            let mut buf = Vec::new();
                            match entry.read_to_end(&mut buf) {
                                Ok(_) => {
                                    std::io::Write::write_all(&mut outfile, &buf)
                                        .map_err(|e| format!("Cannot write file: {}", e))?;
                                }
                                Err(e) => {
                                    let msg = e.to_string().to_lowercase();
                                    // CRC/checksum errors: try writing what we got, then fall back to 7z on finish
                                    if msg.contains("checksum") || msg.contains("crc") || msg.contains("invalid") {
                                        crate::logger::log(&format!("extract_zip_to_temp: CRC error on entry {}, will retry with 7z fallback: {}", i, e));
                                        let _ = std::io::Write::write_all(&mut outfile, &buf);
                                        crc_failed = true;
                                    } else {
                                        return Err(format!("Cannot read entry: {}", e));
                                    }
                                }
                            }
                        }
                    }

                    if crc_failed {
                        // CRC mismatch in one or more entries — retry with 7z which is more lenient
                        crate::logger::log("extract_zip_to_temp: Retrying extraction with 7z due to CRC errors");
                        let _ = fs::remove_dir_all(temp_dir);
                        fs::create_dir_all(temp_dir).map_err(|e| format!("Cannot recreate temp dir: {}", e))?;
                        if let Ok(res) = extract_7z_to_temp(zip_path, temp_dir) {
                            return Ok(res);
                        }
                        // If 7z also fails, return temp dir with partially extracted content
                        crate::logger::log("extract_zip_to_temp: 7z fallback also failed, using partial extraction");
                    }

                    Ok(temp_dir.to_path_buf())
                }
                Err(orig_err) => {
                    // EOCD or other structural errors — try 7z and RAR engines
                    if let Ok(res) = extract_7z_to_temp(zip_path, temp_dir) {
                        Ok(res)
                    } else if let Ok(res) = extract_rar_to_temp(zip_path, temp_dir) {
                        Ok(res)
                    } else {
                        Err(orig_err)
                    }
                }
            }
        }
    }
}

/// Find all companion files (.pak, .ucas, .utoc) for a given .pak stem.
pub fn find_pak_companions(pak_path: &Path) -> Vec<PathBuf> {
    let mut companions = Vec::new();
    if let (Some(parent), Some(stem)) = (pak_path.parent(), pak_path.file_stem()) {
        let stem_str = stem.to_string_lossy();
        for ext in &["pak", "ucas", "utoc"] {
            let companion = parent.join(format!("{}.{}", stem_str, ext));
            if companion.exists() {
                companions.push(companion);
            }
        }
    }
    companions
}

pub fn read_archive_file(zip_path: &str, target_file: &str) -> Option<String> {
    let p = Path::new(zip_path);
    let format = detect_archive_format(p);

    match format {
        ArchiveFormat::SevenZip => read_7z_file(zip_path, target_file),
        ArchiveFormat::Rar => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let temp_dir = std::env::temp_dir().join(format!("pmm_temp_{}", timestamp));
            let _ = fs::create_dir_all(&temp_dir);
            let mut cmd = std::process::Command::new("tar");
            cmd.args(&["-xf", zip_path, "-C", &temp_dir.to_string_lossy(), target_file]);
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            
            let mut result = None;
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let extracted_path = temp_dir.join(target_file);
                    if let Ok(content) = fs::read_to_string(&extracted_path) {
                        result = Some(content);
                    }
                }
            }
            let _ = fs::remove_dir_all(&temp_dir);
            result
        }
        _ => {
            let lower_target = target_file.to_lowercase();
            if let Ok(file) = fs::File::open(zip_path) {
                if let Ok(mut archive) = ZipArchive::new(file) {
                    for i in 0..archive.len() {
                        if let Ok(mut entry) = archive.by_index(i) {
                            let name = entry.name().replace('\\', "/").to_lowercase();
                            if name == lower_target || name.ends_with(&format!("/{}", lower_target)) {
                                let mut buf = String::new();
                                if entry.read_to_string(&mut buf).is_ok() {
                                    return Some(buf);
                                }
                            }
                        }
                    }
                }
            }
            if let Some(s) = read_7z_file(zip_path, target_file) {
                return Some(s);
            }
            None
        }
    }
}
