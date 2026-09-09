use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

static PALSCHEMA_CATALOG_CACHE: Mutex<Option<PalSchemaCatalogStatus>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PalSchemaCatalogStatus {
    pub is_available: bool,
    pub total_raw_schemas: usize,
    pub total_domain_schemas: usize,
    pub has_enums: bool,
    pub version: String,
    pub author: String,
    pub source_location: String,
    pub schemas_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PalSchemaDefinition {
    pub uri: String,
    pub file_match: Vec<String>,
    pub schema_json: String,
}

/// Resolves the best available directory containing PalSchema JSON schemas.
pub fn get_palschema_schemas_dir(program_path: &str, game_path: &str) -> PathBuf {
    // Priority 1: User's active game installation (Pal/Binaries/Win64/ue4ss/Mods/PalSchema/schemas)
    if !game_path.is_empty() {
        let game_base = Path::new(game_path);
        let standard_schemas = game_base
            .join("Pal")
            .join("Binaries")
            .join("Win64")
            .join("ue4ss")
            .join("Mods")
            .join("PalSchema")
            .join("schemas");
        if standard_schemas.is_dir() && standard_schemas.join("raw").is_dir() {
            return standard_schemas;
        }

        let workshop_schemas = game_base
            .join("Mods")
            .join("NativeMods")
            .join("UE4SS")
            .join("Mods")
            .join("PalSchema")
            .join("schemas");
        if workshop_schemas.is_dir() && workshop_schemas.join("raw").is_dir() {
            return workshop_schemas;
        }
    }

    // Priority 2: PMM internal resources directory
    let internal = get_internal_palschema_dir(program_path);

    if !internal.is_dir() || !internal.join("raw").is_dir() {
        let schemas_base = if !program_path.is_empty() {
            Path::new(program_path).join("resources").join("schemas")
        } else {
            PathBuf::from("resources").join("schemas")
        };
        if schemas_base.is_dir() {
            // Find any versioned zip like palschema_schemas_0.6.7.zip or palschema_schemas.zip
            if let Ok(entries) = fs::read_dir(&schemas_base) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if fname.starts_with("palschema_schemas") && fname.ends_with(".zip") {
                        let _ = crate::zip_handler::extract_zip_to_temp(&p.to_string_lossy(), &internal);
                        break;
                    }
                }
            }
        }
    }

    if internal.is_dir() {
        return internal;
    }

    internal
}

/// Resolves the internal PMM resources directory for PalSchema schemas.
pub fn get_internal_palschema_dir(program_path: &str) -> PathBuf {
    let schemas_base = if !program_path.is_empty() {
        Path::new(program_path).join("resources").join("schemas")
    } else {
        PathBuf::from("resources").join("schemas")
    };
    schemas_base.join("palschema")
}

/// Returns the status and counts of the PalSchema JSON Schemas catalog.
pub fn get_or_load_palschema_catalog(program_path: &str, game_path: &str) -> PalSchemaCatalogStatus {
    let mut cache = match PALSCHEMA_CATALOG_CACHE.lock() {
        Ok(c) => c,
        Err(_) => {
            return PalSchemaCatalogStatus {
                is_available: false,
                total_raw_schemas: 0,
                total_domain_schemas: 0,
                has_enums: false,
                version: "0.6.7".to_string(),
                author: "Okaetsu".to_string(),
                source_location: "None".to_string(),
                schemas_dir: String::new(),
            };
        }
    };

    if let Some(ref status) = *cache {
        return status.clone();
    }

    let dir = get_palschema_schemas_dir(program_path, game_path);
    let internal = get_internal_palschema_dir(program_path);
    let mut total_raw = 0;
    let mut total_domain = 0;
    let mut has_enums = false;
    let mut is_avail = false;

    let domain_files = ["items.schema.json", "pals.schema.json", "buildings.schema.json", "skins.schema.json", "utility.schema.json"];

    // Auto-supplement: if active schemas directory is live game folder and is missing domain models or raw.schema.json,
    // copy them from PMM internal resources so the live game installation is also complete
    if dir != internal && dir.is_dir() && internal.is_dir() {
        for df in domain_files {
            let target_file = dir.join(df);
            let src_file = internal.join(df);
            if !target_file.is_file() && src_file.is_file() {
                let _ = fs::copy(&src_file, &target_file);
            }
        }
        let target_raw_schema = dir.join("raw.schema.json");
        let src_raw_schema = internal.join("raw.schema.json");
        if !target_raw_schema.is_file() && src_raw_schema.is_file() {
            let _ = fs::copy(&src_raw_schema, &target_raw_schema);
        }
    }

    if dir.is_dir() || internal.is_dir() {
        is_avail = true;
        let raw_dir = if dir.join("raw").is_dir() {
            dir.join("raw")
        } else {
            internal.join("raw")
        };
        if raw_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&raw_dir) {
                total_raw = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
                    .count();
            }
        }

        for df in domain_files {
            if dir.join(df).is_file() || internal.join(df).is_file() {
                total_domain += 1;
            }
        }

        has_enums = dir.join("enums.schema.json").is_file() || internal.join("enums.schema.json").is_file();
    }

    let mut version = String::new();
    let mut author = "Okaetsu".to_string();

    // 1. Check live game installation for palschema.version or Info.json
    if !game_path.is_empty() {
        let p = Path::new(game_path);
        let candidates = [
            p.join("Pal").join("Binaries").join("Win64").join("ue4ss").join("Mods").join("PalSchema"),
            p.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("PalSchema"),
            p.join("Mods").join("ManagedMods").join("PalSchema"),
        ];
        for c in candidates {
            let ver_file = c.join("palschema.version");
            if ver_file.is_file() {
                if let Ok(s) = fs::read_to_string(&ver_file) {
                    let trimmed = s.trim().to_string();
                    if !trimmed.is_empty() {
                        version = trimmed;
                        break;
                    }
                }
            }
            let info_file = c.join("Info.json");
            if info_file.is_file() {
                if let Ok(s) = fs::read_to_string(&info_file) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&s) {
                        if let Some(v) = val.get("version").and_then(|x| x.as_str()) {
                            if !v.is_empty() {
                                version = v.to_string();
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Read schemas/manifest.json if present
    if version.is_empty() {
        let schemas_base = if !program_path.is_empty() {
            Path::new(program_path).join("resources").join("schemas")
        } else {
            PathBuf::from("resources").join("schemas")
        };
        let manifest_path = schemas_base.join("manifest.json");
        if manifest_path.is_file() {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(v) = val.get("latest_palschema_version").and_then(|v| v.as_str()) {
                        version = v.to_string();
                    }
                    if let Some(a) = val.get("author").and_then(|a| a.as_str()) {
                        author = a.to_string();
                    }
                }
            }
        }
    }

    // 3. Fallback to Master Resource Manifest or default latest
    if version.is_empty() {
        if let Some(master) = super::master_manifest::get_or_load_master_manifest(program_path) {
            if let Some(v) = super::master_manifest::resolve_palschema_version(&master, None) {
                version = v;
            }
        }
    }

    if version.is_empty() {
        version = "0.6.7".to_string();
    }

    let source = if dir.to_string_lossy().contains("Palworld") {
        "Game Installation (Live)".to_string()
    } else {
        "PMM Bundled Resources".to_string()
    };

    let status = PalSchemaCatalogStatus {
        is_available: is_avail && (total_raw > 0 || total_domain > 0),
        total_raw_schemas: total_raw,
        total_domain_schemas: total_domain,
        has_enums,
        version,
        author,
        source_location: source,
        schemas_dir: dir.to_string_lossy().to_string(),
    };

    *cache = Some(status.clone());
    status
}

static PALSCHEMA_FULL_DEFS_CACHE: Mutex<Option<Arc<Vec<PalSchemaDefinition>>>> = Mutex::new(None);
static PALSCHEMA_CORE_DEFS_CACHE: Mutex<Option<Arc<Vec<PalSchemaDefinition>>>> = Mutex::new(None);
static PALSCHEMA_TABLE_NAMES_CACHE: Mutex<Option<Arc<HashSet<String>>>> = Mutex::new(None);

/// Clears the in-memory cache to force a re-read from disk (used after GitHub sync).
pub fn invalidate_palschema_catalog_cache() {
    if let Ok(mut cache) = PALSCHEMA_CATALOG_CACHE.lock() {
        *cache = None;
    }
    if let Ok(mut cache) = PALSCHEMA_FULL_DEFS_CACHE.lock() {
        *cache = None;
    }
    if let Ok(mut cache) = PALSCHEMA_CORE_DEFS_CACHE.lock() {
        *cache = None;
    }
    if let Ok(mut cache) = PALSCHEMA_TABLE_NAMES_CACHE.lock() {
        *cache = None;
    }
}

/// Returns a cached set of all available official PalSchema DataTable names.
pub fn get_or_load_palschema_table_names(program_path: &str, game_path: &str) -> Arc<HashSet<String>> {
    if let Ok(cache) = PALSCHEMA_TABLE_NAMES_CACHE.lock() {
        if let Some(ref names) = *cache {
            return Arc::clone(names);
        }
    }

    let dir = get_palschema_schemas_dir(program_path, game_path);
    let mut names = HashSet::new();

    let raw_dir = dir.join("raw");
    if raw_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&raw_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let p = entry.path();
                let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if fname.ends_with(".schema.json") {
                    let clean = fname.trim_end_matches(".schema.json");
                    names.insert(clean.to_string());
                } else if fname.ends_with(".json") {
                    let clean = fname.trim_end_matches(".json");
                    names.insert(clean.to_string());
                }
            }
        }
    }

    // Include high-level domain schema names
    names.insert("items".to_string());
    names.insert("pals".to_string());
    names.insert("buildings".to_string());
    names.insert("skins".to_string());
    names.insert("utility".to_string());
    names.insert("raw".to_string());

    let arc_names = Arc::new(names);
    if let Ok(mut cache) = PALSCHEMA_TABLE_NAMES_CACHE.lock() {
        *cache = Some(Arc::clone(&arc_names));
    }
    arc_names
}

/// Fast in-memory check to verify if a table exists in PalSchema schemas.
pub fn is_valid_palschema_table_name(table_name: &str, program_path: &str, game_path: &str) -> bool {
    let clean = table_name.trim();
    if clean.is_empty() {
        return false;
    }
    let table_names = get_or_load_palschema_table_names(program_path, game_path);
    if table_names.contains(clean) {
        return true;
    }
    for t in table_names.iter() {
        if t.eq_ignore_ascii_case(clean) {
            return true;
        }
    }
    // PalSchema text / localization tables (DT_*Text, DT_*TextData)
    if clean.starts_with("DT_") && (clean.ends_with("Text") || clean.ends_with("TextData")) {
        return true;
    }
    false
}

/// Loads the schemas formatted as definitions for Monaco Editor registration.
/// - include_raw = false: Returns the 7 essential domain schemas (<40KB, <1ms) for instant 0ms editor startup.
/// - include_raw = true: Returns all 481 schemas (including all 474 DataTables) for full Okaetsu RawTable autocompletion.
pub fn load_palschema_definitions_for_monaco(
    program_path: &str,
    game_path: &str,
    include_raw: bool,
) -> Arc<Vec<PalSchemaDefinition>> {
    if !include_raw {
        if let Ok(cache) = PALSCHEMA_CORE_DEFS_CACHE.lock() {
            if let Some(ref defs) = *cache {
                return Arc::clone(defs);
            }
        }
    } else {
        if let Ok(cache) = PALSCHEMA_FULL_DEFS_CACHE.lock() {
            if let Some(ref defs) = *cache {
                return Arc::clone(defs);
            }
        }
    }

    let dir = get_palschema_schemas_dir(program_path, game_path);
    let internal = get_internal_palschema_dir(program_path);
    let mut defs = Vec::new();

    if !dir.is_dir() && !internal.is_dir() {
        return Arc::new(defs);
    }

    // 1. Base Enums schema
    let enums_path = if dir.join("enums.schema.json").is_file() {
        dir.join("enums.schema.json")
    } else {
        internal.join("enums.schema.json")
    };
    if let Ok(content) = fs::read_to_string(&enums_path) {
        defs.push(PalSchemaDefinition {
            uri: "palschema://schemas/enums.schema.json".to_string(),
            file_match: vec![],
            schema_json: content,
        });
    }

    // 2. High-level domain schemas
    let domain_mappings = [
        ("items.schema.json", vec![
            "**/*items*.json".to_string(), "**/*items*.jsonc".to_string(),
            "**/*item*.json".to_string(), "**/*item*.jsonc".to_string(),
            "**/items/*.json".to_string(), "**/items/*.jsonc".to_string(),
        ]),
        ("pals.schema.json", vec![
            "**/*pals*.json".to_string(), "**/*pals*.jsonc".to_string(),
            "**/*pal*.json".to_string(), "**/*pal*.jsonc".to_string(),
            "**/pals/*.json".to_string(), "**/pals/*.jsonc".to_string(),
        ]),
        ("buildings.schema.json", vec![
            "**/*buildings*.json".to_string(), "**/*buildings*.jsonc".to_string(),
            "**/*building*.json".to_string(), "**/*building*.jsonc".to_string(),
            "**/buildings/*.json".to_string(), "**/buildings/*.jsonc".to_string(),
        ]),
        ("skins.schema.json", vec![
            "**/*skins*.json".to_string(), "**/*skins*.jsonc".to_string(),
            "**/*skin*.json".to_string(), "**/*skin*.jsonc".to_string(),
            "**/skins/*.json".to_string(), "**/skins/*.jsonc".to_string(),
        ]),
        ("utility.schema.json", vec![
            "**/*utility*.json".to_string(), "**/*utility*.jsonc".to_string(),
            "**/*utilities*.json".to_string(), "**/*utilities*.jsonc".to_string(),
        ]),
        ("raw.schema.json", vec![
            "**/*DT_*.json".to_string(),
            "**/*DT_*.jsonc".to_string(),
            "**/raw/*.json".to_string(),
            "**/raw/*.jsonc".to_string(),
            "**/*raw*.json".to_string(),
            "**/*raw*.jsonc".to_string(),
            "**/*config*.json".to_string(),
            "**/*config*.jsonc".to_string(),
            "**/*.palschema.json".to_string(),
            "**/*.palschema.jsonc".to_string(),
        ]),
    ];

    for (fname, matches) in domain_mappings {
        let p = if dir.join(fname).is_file() {
            dir.join(fname)
        } else {
            internal.join(fname)
        };
        if let Ok(content) = fs::read_to_string(&p) {
            let enriched = if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("allowComments".to_string(), serde_json::Value::Bool(true));
                    obj.insert("allowTrailingCommas".to_string(), serde_json::Value::Bool(true));
                }
                serde_json::to_string(&val).unwrap_or(content)
            } else {
                content
            };
            defs.push(PalSchemaDefinition {
                uri: format!("palschema://schemas/{}", fname),
                file_match: matches,
                schema_json: enriched,
            });
        }
    }

    // Cache core definitions if !include_raw
    if !include_raw {
        let arc = Arc::new(defs);
        if let Ok(mut cache) = PALSCHEMA_CORE_DEFS_CACHE.lock() {
            *cache = Some(Arc::clone(&arc));
        }
        return arc;
    }

    // 3. Raw DataTables schemas (for relative $ref resolution from raw.schema.json)
    let raw_dir = if dir.join("raw").is_dir() {
        dir.join("raw")
    } else {
        internal.join("raw")
    };
    if raw_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&raw_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|x| x.to_str()) == Some("json") {
                    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                    if let Ok(content) = fs::read_to_string(&path) {
                        let enriched = enrich_table_schema_content(&content, &file_name, program_path);
                        defs.push(PalSchemaDefinition {
                            uri: format!("palschema://schemas/raw/{}", file_name),
                            file_match: vec![],
                            schema_json: enriched,
                        });
                    }
                }
            }
        }
    }

    // 4. Synthetic Blueprints Schema (Dynamic in-memory reflection)
    let bp_index = crate::usmap::get_or_load_blueprint_index(program_path);
    if let Some(ref bpi) = bp_index {
        if !bpi.blueprints.is_empty() {
            let mut class_names: Vec<String> = bpi.blueprints.values().map(|b| b.class_name.clone()).collect();
            class_names.sort();
            let bp_schema = serde_json::json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Palworld Blueprints Configuration Schema",
                "type": "object",
                "allowComments": true,
                "allowTrailingCommas": true,
                "propertyNames": {
                    "enum": class_names
                }
            });
            defs.push(PalSchemaDefinition {
                uri: "palschema://schemas/blueprints.schema.json".to_string(),
                file_match: vec![
                    "**/*blueprint*.json".to_string(),
                    "**/*blueprint*.jsonc".to_string(),
                    "**/blueprints/*.json".to_string(),
                    "**/blueprints/*.jsonc".to_string(),
                ],
                schema_json: bp_schema.to_string(),
            });
        }
    }

    let arc = Arc::new(defs);
    if let Ok(mut cache) = PALSCHEMA_FULL_DEFS_CACHE.lock() {
        *cache = Some(Arc::clone(&arc));
    }
    arc
}

fn enrich_table_schema_content(content: &str, table_name_hint: &str, program_path: &str) -> String {
    let dt_index = crate::usmap::get_or_load_datatable_index(program_path);
    let mut table_name = table_name_hint.trim_end_matches(".schema.json").to_string();
    if table_name.ends_with(".json") {
        table_name = table_name.trim_end_matches(".json").to_string();
    }

    let rows: Option<Vec<String>> = if let Some(ref dti) = dt_index {
        dti.find_table(&table_name).map(|t| t.rows.clone())
    } else {
        None
    };

    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(obj) = val.as_object_mut() {
            obj.insert("allowComments".to_string(), serde_json::Value::Bool(true));
            obj.insert("allowTrailingCommas".to_string(), serde_json::Value::Bool(true));

            if let Some(row_list) = rows {
                if !row_list.is_empty() {
                    let mut prop_names = serde_json::Map::new();
                    prop_names.insert("enum".to_string(), serde_json::json!(row_list));
                    obj.insert("propertyNames".to_string(), serde_json::Value::Object(prop_names));
                }
            }
        }
        serde_json::to_string(&val).unwrap_or_else(|_| content.to_string())
    } else {
        content.to_string()
    }
}

/// Loads a single specific raw DataTable schema on-demand (e.g. for "DT_ItemDataTable").
pub fn load_single_raw_palschema_definition(
    program_path: &str,
    game_path: &str,
    table_name: &str,
) -> Option<PalSchemaDefinition> {
    let dir = get_palschema_schemas_dir(program_path, game_path);
    let clean_table = table_name.trim();
    if clean_table.is_empty() {
        return None;
    }

    let file_name = if clean_table.ends_with(".schema.json") {
        clean_table.to_string()
    } else if clean_table.ends_with(".json") {
        format!("{}.schema.json", clean_table.trim_end_matches(".json"))
    } else {
        format!("{}.schema.json", clean_table)
    };

    let p = dir.join("raw").join(&file_name);
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            let enriched = enrich_table_schema_content(&content, &file_name, program_path);
            let match_pattern = format!("**/*{}*", clean_table.trim_end_matches(".schema.json"));
            return Some(PalSchemaDefinition {
                uri: format!("palschema://schemas/raw/{}", file_name),
                file_match: vec![format!("{}.json", match_pattern), format!("{}.jsonc", match_pattern)],
                schema_json: enriched,
            });
        }
    }

    None
}
