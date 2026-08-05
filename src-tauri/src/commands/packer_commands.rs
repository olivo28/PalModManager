use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::process::Command;
use zip::write::SimpleFileOptions;
use tauri::State;
use crate::state::AppState;


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StagedFile {
    pub source_path: String,
    pub relative_path: String,
    pub size: u64,
    pub target_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub mod_type: String,
}

#[tauri::command]
pub async fn scan_paths_for_packing(paths: Vec<String>) -> Result<Vec<StagedFile>, String> {
    let mut staged_files = Vec::new();

    for path_str in paths {
        let path = Path::new(&path_str);
        if !path.exists() {
            continue;
        }

        if path.is_file() {
            let filename = path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let size = fs::metadata(path)
                .map(|m| m.len())
                .unwrap_or(0);
            staged_files.push(StagedFile {
                source_path: path_str.clone(),
                relative_path: filename.clone(),
                size,
                target_path: filename,
            });
        } else if path.is_dir() {
            // Walk the directory recursively
            let walker = walkdir::WalkDir::new(path);
            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let file_path = entry.path();
                    let relative = file_path.strip_prefix(path.parent().unwrap_or(path))
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|_| file_path.file_name().unwrap().to_string_lossy().into_owned());
                    
                    let size = entry.metadata()
                        .map(|m| m.len())
                        .unwrap_or(0);

                    // For target path, default to relative to the parent folder so the folder name is preserved
                    staged_files.push(StagedFile {
                        source_path: file_path.to_string_lossy().into_owned(),
                        relative_path: relative.clone(),
                        size,
                        target_path: relative.replace('\\', "/"),
                    });
                }
            }
        }
    }

    Ok(staged_files)
}

fn find_executable(name: &str, alternative_paths: &[&str]) -> Option<PathBuf> {
    // 1. Check system PATH by spawning a dummy process
    if Command::new(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
    {
        return Some(PathBuf::from(name));
    }
    // 2. Check alternative locations
    for &p in alternative_paths {
        let path = Path::new(p);
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }
    None
}


#[tauri::command]
pub async fn pack_mod(
    files: Vec<StagedFile>,
    metadata: Option<ModMetadata>,
    output_path: String,
    format: String,
) -> Result<String, String> {
    let output_file = Path::new(&output_path);
    
    // Ensure parent directory exists
    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    // Determine format
    let format_lower = format.to_lowercase();
    if format_lower == "zip" {
        // Native ZIP packaging
        let file = fs::File::create(output_file)
            .map_err(|e| format!("Failed to create output file: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        // Write modinfo.json if metadata is present
        if let Some(ref meta) = metadata {
            let json_str = serde_json::to_string_pretty(meta)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
            zip.start_file("modinfo.json", options)
                .map_err(|e| format!("Failed to start file in zip: {}", e))?;
            use std::io::Write;
            zip.write_all(json_str.as_bytes())
                .map_err(|e| format!("Failed to write metadata to zip: {}", e))?;
        }

        // Add files
        for f in files {
            let data = fs::read(&f.source_path)
                .map_err(|e| format!("Failed to read file {}: {}", f.source_path, e))?;
            
            // Clean path separators to always be forward slashes in ZIP
            let clean_target = f.target_path.replace('\\', "/");
            zip.start_file(clean_target, options)
                .map_err(|e| format!("Failed to start file in zip: {}", e))?;
            use std::io::Write;
            zip.write_all(&data)
                .map_err(|e| format!("Failed to write file to zip: {}", e))?;
        }

        zip.finish().map_err(|e| format!("Failed to finish zip archive: {}", e))?;
        Ok(format!("Successfully packaged mod as ZIP: {}", output_path))
    } else if format_lower == "7z" || format_lower == "rar" {
        // External packer (7z or rar)
        // 1. Create a temp staging folder inside the temp dir
        let temp_dir = std::env::temp_dir().join(format!("pmm_packer_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp staging dir: {}", e))?;

        // 2. Write metadata if present
        if let Some(ref meta) = metadata {
            let json_str = serde_json::to_string_pretty(meta)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
            let meta_path = temp_dir.join("modinfo.json");
            fs::write(meta_path, json_str)
                .map_err(|e| format!("Failed to write temp metadata: {}", e))?;
        }

        // 3. Copy files to target locations under temp_dir
        for f in &files {
            let dest_path = temp_dir.join(&f.target_path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create subdirectories: {}", e))?;
            }
            fs::copy(&f.source_path, &dest_path)
                .map_err(|e| format!("Failed to copy file {} to staging: {}", f.source_path, e))?;
        }

        // Remove output file if it already exists to avoid issues
        if output_file.exists() {
            let _ = fs::remove_file(output_file);
        }

        // 4. Run command
        let res = if format_lower == "7z" {
            let exe = find_executable(
                "7z",
                &[
                    "C:\\Program Files\\7-Zip\\7z.exe",
                    "C:\\Program Files (x86)\\7-Zip\\7z.exe",
                ],
            ).ok_or_else(|| "7-Zip (7z.exe) not found on your system. Please install 7-Zip to use 7z format.".to_string())?;

            // Command: 7z a "output_path" "temp_dir/*"
            let output = Command::new(exe)
                .arg("a")
                .arg("-y")
                .arg(&output_path)
                .arg(format!("{}\\*", temp_dir.to_string_lossy()))
                .output()
                .map_err(|e| format!("Failed to run 7z: {}", e))?;

            if !output.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                let out_msg = String::from_utf8_lossy(&output.stdout);
                Err(format!("7z compression failed. Output: {}\nError: {}", out_msg, err_msg))
            } else {
                Ok(format!("Successfully packaged mod as 7z: {}", output_path))
            }
        } else {
            // RAR
            let exe = find_executable(
                "rar",
                &[
                    "C:\\Program Files\\WinRAR\\Rar.exe",
                    "C:\\Program Files (x86)\\WinRAR\\Rar.exe",
                    "C:\\Program Files\\WinRAR\\WinRAR.exe",
                ],
            ).ok_or_else(|| "WinRAR (Rar.exe) not found on your system. Please install WinRAR to use RAR format.".to_string())?;

            // Command: rar a -r "output_path" "temp_dir/*"
            let output = Command::new(exe)
                .arg("a")
                .arg("-ep1") // exclude base path
                .arg("-r")
                .arg(&output_path)
                .arg(format!("{}\\*", temp_dir.to_string_lossy()))
                .output()
                .map_err(|e| format!("Failed to run rar: {}", e))?;

            if !output.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                let out_msg = String::from_utf8_lossy(&output.stdout);
                Err(format!("RAR compression failed. Output: {}\nError: {}", out_msg, err_msg))
            } else {
                Ok(format!("Successfully packaged mod as RAR: {}", output_path))
            }
        };

        // Cleanup staging
        let _ = fs::remove_dir_all(&temp_dir);
        res
    } else {
        Err(format!("Unsupported format: {}", format))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackerProject {
    pub name: String,
    pub metadata: Option<ModMetadata>,
    pub source_paths: Vec<String>,
    pub target_paths_override: std::collections::HashMap<String, String>,
    pub format: String,
}

fn get_projects_filepath(program_path: &str) -> PathBuf {
    Path::new(program_path).join("pmm_packer_projects.json")
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn save_packer_project(
    projectName: String,
    metadata: Option<ModMetadata>,
    sourcePaths: Vec<String>,
    targetPathsOverride: std::collections::HashMap<String, String>,
    format: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let filepath = get_projects_filepath(&program_path);
    let mut projects: Vec<PackerProject> = if filepath.exists() {
        let content = fs::read_to_string(&filepath)
            .map_err(|e| format!("Failed to read projects file: {}", e))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Remove existing project with same name
    projects.retain(|p| p.name != projectName);

    // Add new project
    projects.push(PackerProject {
        name: projectName.clone(),
        metadata,
        source_paths: sourcePaths,
        target_paths_override: targetPathsOverride,
        format,
    });

    let json_str = serde_json::to_string_pretty(&projects)
        .map_err(|e| format!("Failed to serialize projects: {}", e))?;
    
    fs::write(&filepath, json_str)
        .map_err(|e| format!("Failed to write projects file: {}", e))?;

    Ok(format!("Project '{}' saved successfully", projectName))
}


#[tauri::command]
pub async fn load_packer_projects(state: State<'_, AppState>) -> Result<Vec<PackerProject>, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let filepath = get_projects_filepath(&program_path);
    if !filepath.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&filepath)
        .map_err(|e| format!("Failed to read projects file: {}", e))?;
    let projects: Vec<PackerProject> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse projects file: {}", e))?;

    Ok(projects)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn delete_packer_project(
    projectName: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let filepath = get_projects_filepath(&program_path);
    if !filepath.exists() {
        return Err("No projects file found".to_string());
    }

    let content = fs::read_to_string(&filepath)
        .map_err(|e| format!("Failed to read projects: {}", e))?;
    let mut projects: Vec<PackerProject> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse projects: {}", e))?;

    let original_len = projects.len();
    projects.retain(|p| p.name != projectName);

    if projects.len() == original_len {
        return Err(format!("Project '{}' not found", projectName));
    }

    let json_str = serde_json::to_string_pretty(&projects)
        .map_err(|e| format!("Failed to serialize projects: {}", e))?;
    
    fs::write(&filepath, json_str)
        .map_err(|e| format!("Failed to write projects file: {}", e))?;

    Ok(format!("Project '{}' deleted successfully", projectName))
}

