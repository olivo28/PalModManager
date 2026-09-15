use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unreal_asset::{
    engine_version::EngineVersion,
    exports::Export,
    Asset,
};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTableEntry {
    pub name: String,
    pub struct_name: String,
    pub package: String,
    pub count: usize,
    pub rows: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTableIndex {
    pub total_tables: usize,
    pub total_rows: usize,
    pub tables: BTreeMap<String, DataTableEntry>,
}

#[derive(Serialize)]
struct BlueprintsManifestFile<'a> {
    schema_version: &'a str,
    latest_game_version: &'a str,
    latest_steam_build_id: &'a str,
    updated_at: String,
    blueprints: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct BlueprintManifestEntry<'a> {
    game_version: &'a str,
    steam_build_id: &'a str,
    app_id: u32,
    blueprints_filename: &'a str,
    blueprints_url: String,
    sha256: &'a str,
    file_size_bytes: usize,
    total_blueprints: usize,
    engine_version: &'a str,
    build_id: String,
    source: &'a str,
    is_latest: bool,
}

#[derive(Serialize)]
struct DataTablesManifestFile<'a> {
    schema_version: &'a str,
    latest_game_version: &'a str,
    latest_steam_build_id: &'a str,
    updated_at: String,
    datatables: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct DataTableManifestEntry<'a> {
    game_version: &'a str,
    steam_build_id: &'a str,
    app_id: u32,
    datatables_filename: &'a str,
    datatables_url: String,
    sha256: &'a str,
    file_size_bytes: usize,
    total_tables: usize,
    total_rows: usize,
    engine_version: &'a str,
    build_id: String,
    source: &'a str,
    is_latest: bool,
}

fn strip_pkg_unversioned_flag(uasset_bytes: &[u8]) -> Option<Vec<u8>> {
    if uasset_bytes.len() < 160 {
        return None;
    }
    let mut modified = uasset_bytes.to_vec();
    let flag_offset = 0xE0; // PackageFlags offset in standard UE5 PackageFileSummary
    if flag_offset + 4 <= modified.len() {
        let flags = u32::from_le_bytes(modified[flag_offset..flag_offset + 4].try_into().unwrap());
        if (flags & 0x2000) != 0 {
            let stripped = flags & !0x2000;
            modified[flag_offset..flag_offset + 4].copy_from_slice(&stripped.to_le_bytes());
            return Some(modified);
        }
    }
    None
}

fn extract_blueprints_from_pak(
    _pak: &repak::PakReader,
    _pak_file: &mut File,
    all_files: &[String],
    build_id: &str,
    game_ver: &str,
    out_dir: &Path,
) -> Result<String, String> {
    println!("\n>> [1/2] Extracting Blueprints from .pak...");
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

    println!("   Found {} Blueprint classes.", blueprints.len());

    let bp_dir = out_dir.join("blueprints");
    fs::create_dir_all(&bp_dir).map_err(|e| e.to_string())?;

    let canonical_filename = format!("Palworld_Blueprints_{}.json", build_id);
    let out_file = bp_dir.join(&canonical_filename);

    let index_payload = BlueprintIndex {
        total_blueprints: blueprints.len(),
        blueprints,
    };

    let json_content = serde_json::to_string(&index_payload).map_err(|e| e.to_string())?;
    fs::write(&out_file, &json_content).map_err(|e| e.to_string())?;

    let mut hasher = Sha256::new();
    hasher.update(json_content.as_bytes());
    let sha256_hash = format!("{:x}", hasher.finalize());
    let file_size_bytes = json_content.len();
    let size_kb = (file_size_bytes as f64 / 1024.0).round();

    println!("   ✓ Wrote {:?} ({} KB, SHA-256: {})", out_file, size_kb, sha256_hash);

    let manifest_path = bp_dir.join("manifest.json");
    let existing_manifest: serde_json::Value = if manifest_path.exists() {
        fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({ "blueprints": [] }))
    } else {
        serde_json::json!({ "blueprints": [] })
    };

    let ue4ss_commit = {
        let mut c = "2281fa31".to_string();
        let master_path = out_dir.join("manifest.json");
        if let Ok(content) = fs::read_to_string(&master_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(versions) = v.get("versions").and_then(|a| a.as_array()) {
                    if let Some(found) = versions.iter().find(|ver| ver.get("steam_build_id").and_then(|s| s.as_str()) == Some(build_id)) {
                        if let Some(commit) = found.get("ue4ss_commit").and_then(|s| s.as_str()) {
                            c = commit.to_string();
                        }
                    }
                }
            }
        }
        c
    };

    let new_entry = serde_json::to_value(BlueprintManifestEntry {
        game_version: game_ver,
        steam_build_id: build_id,
        app_id: 1623730,
        blueprints_filename: &canonical_filename,
        blueprints_url: format!("https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/blueprints/{}", canonical_filename),
        sha256: &sha256_hash,
        file_size_bytes,
        total_blueprints: index_payload.total_blueprints,
        engine_version: "UE5.1.1",
        build_id: format!("Pal-5.1.1-0+++UE5+Release-5.1-{}", ue4ss_commit),
        source: "Pal-Windows.pak",
        is_latest: true,
    }).map_err(|e| e.to_string())?;

    let mut list = existing_manifest
        .get("blueprints")
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();

    for item in list.iter_mut() {
        if let Some(obj) = item.as_object_mut() {
            obj.insert("is_latest".to_string(), serde_json::Value::Bool(false));
        }
    }

    list.retain(|item| {
        item.get("steam_build_id").and_then(|s| s.as_str()) != Some(build_id)
    });
    list.insert(0, new_entry);

    let manifest_file = BlueprintsManifestFile {
        schema_version: "1.0.0",
        latest_game_version: game_ver,
        latest_steam_build_id: build_id,
        updated_at: chrono::Utc::now().to_rfc3339(),
        blueprints: list,
    };

    let manifest_str = serde_json::to_string_pretty(&manifest_file).map_err(|e| e.to_string())?;
    fs::write(&manifest_path, manifest_str).map_err(|e| e.to_string())?;
    println!("   ✓ Updated manifest: {:?}", manifest_path);

    Ok(canonical_filename)
}

fn extract_datatables_from_pak(
    pak: &repak::PakReader,
    pak_file: &mut File,
    all_files: &[String],
    build_id: &str,
    game_ver: &str,
    out_dir: &Path,
) -> Result<String, String> {
    println!("\n>> [2/2] Extracting DataTables from .pak...");

    let mut datatable_entries: Vec<String> = Vec::new();
    for entry in all_files {
        let clean = entry.replace('\\', "/");
        if !clean.ends_with(".uasset") {
            continue;
        }

        let file_stem = Path::new(&clean)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let is_dt = clean.contains("/DataTable/")
            || clean.contains("/DT_")
            || file_stem.starts_with("DT_")
            || file_stem.ends_with("DataTable");

        if is_dt {
            datatable_entries.push(clean);
        }
    }

    println!("   Discovered {} candidate DataTable asset(s) in .pak.", datatable_entries.len());

    let mut tables: BTreeMap<String, DataTableEntry> = BTreeMap::new();
    let mut total_rows = 0;

    for dt_path in &datatable_entries {
        let file_stem = Path::new(dt_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if file_stem.is_empty() {
            continue;
        }

        let mut uasset_bytes = Vec::new();
        if pak.read_file(dt_path, pak_file, &mut uasset_bytes).is_err() {
            continue;
        }

        let uexp_path = format!("{}.uexp", dt_path.trim_end_matches(".uasset"));
        let mut uexp_bytes = Vec::new();
        let uexp_opt = if pak.read_file(&uexp_path, pak_file, &mut uexp_bytes).is_ok() && !uexp_bytes.is_empty() {
            Some(uexp_bytes)
        } else {
            None
        };

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let mut parse_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let uasset_cursor = Cursor::new(uasset_bytes.clone());
            let uexp_cursor = uexp_opt.clone().map(Cursor::new);
            Asset::new(uasset_cursor, uexp_cursor, EngineVersion::VER_UE5_1)
        }));

        if parse_res.is_err() || parse_res.as_ref().unwrap().is_err() {
            if let Some(stripped) = strip_pkg_unversioned_flag(&uasset_bytes) {
                parse_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let uasset_cursor = Cursor::new(stripped);
                    let uexp_cursor = uexp_opt.map(Cursor::new);
                    Asset::new(uasset_cursor, uexp_cursor, EngineVersion::VER_UE5_1)
                }));
            }
        }

        std::panic::set_hook(prev_hook);

        if let Ok(Ok(asset)) = parse_res {
            let name_map = asset.get_name_map().get_ref().get_name_map_index_list().to_vec();

            let struct_name = asset.imports.iter()
                .find(|i| i.class_name.get_content() == "ScriptStruct" && !i.object_name.get_content().starts_with("Default__"))
                .map(|i| i.object_name.get_content())
                .or_else(|| {
                    name_map.iter()
                        .find(|n| (n.ends_with("Data") || n.ends_with("MasterData")) && !n.contains('/'))
                        .cloned()
                })
                .unwrap_or_else(|| "TableRow".to_string());

            let mut extracted_rows = Vec::new();
            for export in &asset.asset_data.exports {
                if let Export::RawExport(r) = export {
                    if r.data.len() >= 14 {
                        let num_entries = i32::from_le_bytes(match r.data[10..14].try_into() {
                            Ok(b) => b,
                            Err(_) => continue,
                        }) as usize;

                        if num_entries > 0 && num_entries <= 50_000 {
                            let mut cur = 14;
                            for _ in 0..num_entries {
                                if cur + 8 > r.data.len() {
                                    break;
                                }
                                let name_idx = i32::from_le_bytes(match r.data[cur..cur + 4].try_into() {
                                    Ok(b) => b,
                                    Err(_) => break,
                                });
                                cur += 8;

                                if name_idx >= 0 && (name_idx as usize) < name_map.len() {
                                    let row_key = name_map[name_idx as usize].clone();
                                    extracted_rows.push(row_key);
                                }
                            }
                        }
                    }
                }
            }

            if extracted_rows.is_empty() {
                for name in &name_map {
                    if name.starts_with("DT_")
                        || name == &struct_name
                        || name == "None"
                        || name.contains('/')
                        || name == "DataTable"
                        || name == "ScriptStruct"
                        || name.starts_with("/Script/")
                    {
                        continue;
                    }
                    extracted_rows.push(name.clone());
                }
            }

            let without_ext = dt_path.trim_end_matches(".uasset");
            let row_count = extracted_rows.len();
            total_rows += row_count;

            tables.insert(file_stem.to_string(), DataTableEntry {
                name: file_stem.to_string(),
                struct_name,
                package: without_ext.to_string(),
                count: row_count,
                rows: extracted_rows,
            });
        }
    }

    let dt_count = tables.len();
    println!("   Successfully extracted {} DataTables with {} total rows.", dt_count, total_rows);

    let dt_dir = out_dir.join("datatables");
    fs::create_dir_all(&dt_dir).map_err(|e| e.to_string())?;

    let canonical_filename = format!("Palworld_DataTables_{}.json", build_id);
    let out_file = dt_dir.join(&canonical_filename);

    let index_payload = DataTableIndex {
        total_tables: dt_count,
        total_rows,
        tables,
    };

    let json_content = serde_json::to_string(&index_payload).map_err(|e| e.to_string())?;
    fs::write(&out_file, &json_content).map_err(|e| e.to_string())?;

    let mut hasher = Sha256::new();
    hasher.update(json_content.as_bytes());
    let sha256_hash = format!("{:x}", hasher.finalize());
    let file_size_bytes = json_content.len();
    let size_kb = (file_size_bytes as f64 / 1024.0).round();

    println!("   ✓ Wrote {:?} ({} KB, SHA-256: {})", out_file, size_kb, sha256_hash);

    let manifest_path = dt_dir.join("manifest.json");
    let existing_manifest: serde_json::Value = if manifest_path.exists() {
        fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({ "datatables": [] }))
    } else {
        serde_json::json!({ "datatables": [] })
    };

    let ue4ss_commit = {
        let mut c = "2281fa31".to_string();
        let master_path = out_dir.join("manifest.json");
        if let Ok(content) = fs::read_to_string(&master_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(versions) = v.get("versions").and_then(|a| a.as_array()) {
                    if let Some(found) = versions.iter().find(|ver| ver.get("steam_build_id").and_then(|s| s.as_str()) == Some(build_id)) {
                        if let Some(commit) = found.get("ue4ss_commit").and_then(|s| s.as_str()) {
                            c = commit.to_string();
                        }
                    }
                }
            }
        }
        c
    };

    let new_entry = serde_json::to_value(DataTableManifestEntry {
        game_version: game_ver,
        steam_build_id: build_id,
        app_id: 1623730,
        datatables_filename: &canonical_filename,
        datatables_url: format!("https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/datatables/{}", canonical_filename),
        sha256: &sha256_hash,
        file_size_bytes,
        total_tables: dt_count,
        total_rows,
        engine_version: "UE5.1.1",
        build_id: format!("Pal-5.1.1-0+++UE5+Release-5.1-{}", ue4ss_commit),
        source: "Pal-Windows.pak",
        is_latest: true,
    }).map_err(|e| e.to_string())?;

    let mut list = existing_manifest
        .get("datatables")
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();

    for item in list.iter_mut() {
        if let Some(obj) = item.as_object_mut() {
            obj.insert("is_latest".to_string(), serde_json::Value::Bool(false));
        }
    }

    list.retain(|item| {
        item.get("steam_build_id").and_then(|s| s.as_str()) != Some(build_id)
    });
    list.insert(0, new_entry);

    let manifest_file = DataTablesManifestFile {
        schema_version: "1.0.0",
        latest_game_version: game_ver,
        latest_steam_build_id: build_id,
        updated_at: chrono::Utc::now().to_rfc3339(),
        datatables: list,
    };

    let manifest_str = serde_json::to_string_pretty(&manifest_file).map_err(|e| e.to_string())?;
    fs::write(&manifest_path, manifest_str).map_err(|e| e.to_string())?;
    println!("   ✓ Updated manifest: {:?}", manifest_path);

    Ok(canonical_filename)
}

fn main() {
    println!("=== PalModManager - Live .pak Symbols Extractor ===");

    let mut pak_path = PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Palworld\Pal\Content\Paks\Pal-Windows.pak");
    let mut build_id = String::new();
    let mut game_ver = String::new();
    let mut target = "all".to_string();
    let mut out_dir = if Path::new("resources").exists() {
        PathBuf::from("resources")
    } else if Path::new("../resources").exists() {
        PathBuf::from("../resources")
    } else {
        PathBuf::from("resources")
    };

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--pak" => {
                if i + 1 < args.len() {
                    pak_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--build-id" => {
                if i + 1 < args.len() {
                    build_id = args[i + 1].clone();
                    i += 1;
                }
            }
            "--game-ver" => {
                if i + 1 < args.len() {
                    game_ver = args[i + 1].clone();
                    i += 1;
                }
            }
            "--target" => {
                if i + 1 < args.len() {
                    target = args[i + 1].clone();
                    i += 1;
                }
            }
            "--out-dir" => {
                if i + 1 < args.len() {
                    out_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            _ => {
                if !args[i].starts_with('-') && i == 1 {
                    pak_path = PathBuf::from(&args[i]);
                }
            }
        }
        i += 1;
    }

    // Dynamically detect build_id and game_ver if not provided via CLI
    if build_id.is_empty() {
        let candidate_acfs = [
            pak_path.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|p| p.join("appmanifest_1623730.acf")),
            Some(PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\appmanifest_1623730.acf")),
        ];
        for acf_opt in candidate_acfs.into_iter().flatten() {
            if acf_opt.is_file() {
                if let Ok(content) = fs::read_to_string(&acf_opt) {
                    for line in content.lines() {
                        if line.contains("\"buildid\"") {
                            let parts: Vec<&str> = line.split('"').collect();
                            if parts.len() >= 4 {
                                build_id = parts[3].trim().to_string();
                                break;
                            }
                        }
                    }
                }
            }
            if !build_id.is_empty() {
                break;
            }
        }
    }

    // Dynamic resolution from resources/manifest.json if present
    let master_manifest_path = out_dir.join("manifest.json");
    if let Ok(content) = fs::read_to_string(&master_manifest_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            if build_id.is_empty() {
                if let Some(b) = v.get("latest_steam_build_id").and_then(|s| s.as_str()) {
                    build_id = b.to_string();
                }
            }
            if game_ver.is_empty() {
                if let Some(gv) = v.get("latest_game_version").and_then(|s| s.as_str()) {
                    game_ver = gv.to_string();
                }
            }
        }
    }

    if build_id.is_empty() {
        build_id = "unknown".to_string();
    }
    if game_ver.is_empty() {
        game_ver = format!("v{}", build_id);
    }

    println!("Pak Path:     {:?}", pak_path);
    println!("Target Build: {} ({})", build_id, game_ver);
    println!("Extract Mode: {}", target);
    println!("Out Dir:      {:?}", out_dir);

    if !pak_path.exists() {
        eprintln!("Error: Target pak file not found: {:?}", pak_path);
        std::process::exit(1);
    }

    println!("Opening pak container...");
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

    let all_files: Vec<String> = pak.files().into_iter().map(|s| s.replace('\\', "/")).collect();
    println!("Total indexed files in pak: {}", all_files.len());

    if target == "all" || target == "blueprints" {
        if let Err(e) = extract_blueprints_from_pak(&pak, &mut file, &all_files, &build_id, &game_ver, &out_dir) {
            eprintln!("Error extracting blueprints: {e}");
        }
    }

    if target == "all" || target == "datatables" {
        if let Err(e) = extract_datatables_from_pak(&pak, &mut file, &all_files, &build_id, &game_ver, &out_dir) {
            eprintln!("Error extracting datatables: {e}");
        }
    }

    println!("\n✓ Pak symbol extraction finished successfully!");
}
