use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintEntry {
    pub name: String,
    pub class_name: String,
    pub package: String,
    pub full_path: String,
    pub super_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintIndex {
    pub total_blueprints: usize,
    pub blueprints: HashMap<String, BlueprintEntry>,
}

fn main() {
    println!("=== Palworld Live .pak Blueprint Extractor ===");

    let default_pak = PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Palworld\Pal\Content\Paks\Pal-Windows.pak");
    let pak_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(default_pak);

    if !pak_path.exists() {
        eprintln!("Error: Pal-Windows.pak not found at {:?}", pak_path);
        std::process::exit(1);
    }

    println!("Opening pak archive: {:?}", pak_path);
    let mut file = match File::open(&pak_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open pak file: {e}");
            std::process::exit(1);
        }
    };

    let pak = match repak::PakBuilder::new().reader(&mut file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to read pak header: {e}");
            std::process::exit(1);
        }
    };

    let all_files = pak.files();
    println!("Total entries indexed in .pak: {}", all_files.len());

    let mut blueprints: HashMap<String, BlueprintEntry> = HashMap::new();

    for entry in all_files {
        let clean = entry.replace('\\', "/");
        if !clean.ends_with(".uasset") {
            continue;
        }

        let is_blueprint = clean.contains("/Blueprint/")
            || clean.contains("/PalActorBP/")
            || clean.contains("/WBP_")
            || clean.contains("/BP_")
            || clean.contains("/ABP_");

        if !is_blueprint {
            continue;
        }

        let file_stem = Path::new(&clean)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if file_stem.is_empty() {
            continue;
        }

        // Convert Pal/Content/... to /Game/...
        let without_ext = clean.trim_end_matches(".uasset");
        let mount_path = if without_ext.starts_with("Pal/Content/") {
            format!("/Game/{}", &without_ext["Pal/Content/".len()..])
        } else if without_ext.starts_with("/Pal/Content/") {
            format!("/Game/{}", &without_ext["/Pal/Content/".len()..])
        } else {
            format!("/Game/{}", without_ext)
        };

        let class_name = format!("{}_C", file_stem);
        let full_path = format!("{}.{}", mount_path, class_name);

        let bp_entry = BlueprintEntry {
            name: file_stem.to_string(),
            class_name: class_name.clone(),
            package: without_ext.to_string(),
            full_path,
            super_class: None,
        };

        blueprints.insert(class_name, bp_entry.clone());
        blueprints.insert(file_stem.to_string(), bp_entry);
    }

    println!("Extracted {} unique Blueprint asset classes directly from .pak!", blueprints.len());

    // Resolve output directory
    let out_dir = if Path::new("resources").exists() {
        PathBuf::from("resources/blueprints")
    } else if Path::new("../resources").exists() {
        PathBuf::from("../resources/blueprints")
    } else {
        PathBuf::from("resources/blueprints")
    };

    if !out_dir.exists() {
        let _ = fs::create_dir_all(&out_dir);
    }

    let canonical_filename = "Palworld_Blueprints_24575825.json";
    let out_file = out_dir.join(canonical_filename);

    let index_payload = BlueprintIndex {
        total_blueprints: blueprints.len(),
        blueprints,
    };

    let json_content = match serde_json::to_string(&index_payload) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to serialize index: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = fs::write(&out_file, &json_content) {
        eprintln!("Failed to write index file {:?}: {}", out_file, e);
        std::process::exit(1);
    }

    let mut hasher = Sha256::new();
    hasher.update(json_content.as_bytes());
    let sha256_hash = format!("{:x}", hasher.finalize());
    let file_size_bytes = json_content.len();
    let size_kb = (file_size_bytes as f64 / 1024.0).round();

    println!("Wrote {:?} ({} KB, SHA-256: {})", out_file, size_kb, sha256_hash);

    let manifest_content = serde_json::json!({
        "schema_version": "1.0.0",
        "latest_game_version": "v1.0.3",
        "latest_steam_build_id": "24575825",
        "updated_at": chrono::Utc::now().to_rfc3339(),
        "blueprints": [
            {
                "game_version": "v1.0.3",
                "steam_build_id": "24575825",
                "app_id": 1623730,
                "blueprints_filename": canonical_filename,
                "blueprints_url": format!("https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/blueprints/{}", canonical_filename),
                "sha256": sha256_hash,
                "file_size_bytes": file_size_bytes,
                "total_blueprints": index_payload.total_blueprints,
                "engine_version": "UE5.1.1",
                "build_id": "Pal-5.1.1-0+++UE5+Release-5.1-c838a8ac",
                "source": "Pal-Windows.pak",
                "is_latest": true
            }
        ]
    });

    let manifest_path = out_dir.join("manifest.json");
    if let Ok(manifest_str) = serde_json::to_string_pretty(&manifest_content) {
        let _ = fs::write(&manifest_path, manifest_str);
        println!("Wrote {:?}", manifest_path);
    }

    println!("Blueprint extraction completed successfully!");
}
