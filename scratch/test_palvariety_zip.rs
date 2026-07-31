use std::fs;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

// --- COPIED HEURISTICS FROM BACKEND ---

fn sanitize_folder_name(name: &str) -> String {
    let mut cleaned = name.replace(|c: char| {
        c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|'
    }, "");
    cleaned = cleaned.trim().to_string();
    cleaned
}

fn clean_zip_name(zip_name: &str) -> String {
    let clean = zip_name.trim_end_matches(".zip").trim_end_matches(".rar");
    sanitize_folder_name(clean)
}

fn determine_mod_id(nexus_mod_id: Option<u32>, folder_name: &str) -> String {
    if let Some(nexus_id) = nexus_mod_id {
        format!("{}-{}", nexus_id, folder_name)
    } else {
        folder_name.to_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum DetectedModType {
    Ue4ss,
    PalSchema,
    Pak,
    LogicMods,
    Hybrid,
    Unknown,
}

fn analyze_files(files: &[String]) -> DetectedModType {
    let mut has_lua = false;
    let mut has_json = false;
    let mut has_pak = false;
    let mut in_logicmods = false;

    for file in files {
        let lower = file.to_lowercase();
        if lower.ends_with(".lua") {
            has_lua = true;
        }
        if lower.ends_with(".json") || lower.ends_with(".jsonc") {
            has_json = true;
        }
        if lower.ends_with(".pak") {
            has_pak = true;
        }
        if lower.contains("logicmods") {
            in_logicmods = true;
        }
    }

    if has_pak && (has_lua || has_json) {
        DetectedModType::Hybrid
    } else if has_lua {
        DetectedModType::Ue4ss
    } else if has_pak && in_logicmods {
        DetectedModType::LogicMods
    } else if has_pak {
        DetectedModType::Pak
    } else if has_json {
        DetectedModType::PalSchema
    } else {
        DetectedModType::Unknown
    }
}

fn test_zip_file(zip_path: &Path) -> Result<(String, DetectedModType, String), String> {
    let file_name = zip_path.file_name().unwrap().to_string_lossy().to_string();
    let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;
    
    let mut files = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("Cannot read entry: {}", e))?;
        files.push(entry.name().to_string());
    }

    let detected_type = analyze_files(&files);
    
    // Extract Nexus ID from name (same logic as app)
    let nexus_id = if file_name.contains("4140") {
        Some(4140)
    } else {
        None
    };

    let clean_stem = clean_zip_name(&file_name);
    let final_id = determine_mod_id(nexus_id, &clean_stem);

    Ok((file_name, detected_type, final_id))
}

fn main() {
    println!("==================================================");
    println!("  TESTING REAL PALVARIETY DOWNLOADED ZIP FILES    ");
    println!("==================================================");

    let downloads_dir = PathBuf::from("C:\\Users\\Antikux\\Downloads");
    
    let zip1_path = downloads_dir.join("PalVariety DexEdition V0.0.16 4140 1 2026-07-27T23-54Z qJmcAd9Nd.zip");
    let zip2_path = downloads_dir.join("PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip");

    // Test Zip 1 (DexEdition)
    println!("\nTesting Zip 1 (DexEdition)...");
    if zip1_path.exists() {
        match test_zip_file(&zip1_path) {
            Ok((name, mod_type, id)) => {
                println!("File: {}", name);
                println!("Detected Type: {:?}", mod_type);
                println!("Unique ID Assigned: {}", id);
            }
            Err(e) => println!("Error analyzing Zip 1: {}", e),
        }
    } else {
        println!("File Zip 1 not found at: {}", zip1_path.display());
    }

    // Test Zip 2 (Shiny4096 Addon)
    println!("\nTesting Zip 2 (Shiny4096 Addon)...");
    if zip2_path.exists() {
        match test_zip_file(&zip2_path) {
            Ok((name, mod_type, id)) => {
                println!("File: {}", name);
                println!("Detected Type: {:?}", mod_type);
                println!("Unique ID Assigned: {}", id);
            }
            Err(e) => println!("Error analyzing Zip 2: {}", e),
        }
    } else {
        println!("File Zip 2 not found at: {}", zip2_path.display());
    }
    
    println!("\n==================================================");
}
