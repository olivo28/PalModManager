pub mod models;
pub mod build_detector;
pub mod sync;
pub mod parser;
pub mod sdk_parser;
pub mod datatable_index;
pub mod blueprint_index;
pub mod palschema_index;

pub use models::*;
pub use build_detector::detect_installed_game_build;
pub use sync::{get_mappings_status, sync_mappings_async, get_active_usmap_path};
pub use parser::{parse_usmap_file, UsmapSchema, UsmapStruct, UsmapProperty};
pub use sdk_parser::{SdkIndex, SdkClassInfo, get_sdk_dir, parse_sdk_directory};
pub use datatable_index::{DataTableIndex, DataTableEntry, get_or_load_datatable_index, get_datatables_dir};
pub use blueprint_index::{BlueprintIndex, BlueprintEntry, get_or_load_blueprint_index, get_blueprints_dir};
pub use palschema_index::{
    PalSchemaCatalogStatus, PalSchemaDefinition, get_palschema_schemas_dir,
    get_or_load_palschema_catalog, invalidate_palschema_catalog_cache,
    load_palschema_definitions_for_monaco, load_single_raw_palschema_definition,
};

use std::sync::{Arc, Mutex};
use std::path::PathBuf;

static ACTIVE_SCHEMA_CACHE: Mutex<Option<Arc<UsmapSchema>>> = Mutex::new(None);
static ACTIVE_SDK_CACHE: Mutex<Option<Arc<SdkIndex>>> = Mutex::new(None);

pub fn get_or_load_schema(program_path: &str) -> Option<Arc<UsmapSchema>> {
    let mut cache = ACTIVE_SCHEMA_CACHE.lock().ok()?;
    if let Some(ref schema) = *cache {
        return Some(Arc::clone(schema));
    }

    let usmap_path = get_active_usmap_path(program_path);
    if usmap_path.exists() {
        match parse_usmap_file(&usmap_path) {
            Ok(schema) => {
                crate::logger::log(&format!("Loaded USMAP Schema: {} structs, {} enums, {} names", schema.total_structs, schema.total_enums, schema.total_names));
                let arc_schema = Arc::new(schema);
                *cache = Some(Arc::clone(&arc_schema));
                Some(arc_schema)
            }
            Err(e) => {
                crate::logger::log(&format!("Failed to parse USMAP schema from {:?}: {}", usmap_path, e));
                None
            }
        }
    } else {
        None
    }
}

pub fn get_or_load_sdk_index(program_path: &str, game_path: &str) -> Option<Arc<SdkIndex>> {
    let mut cache = ACTIVE_SDK_CACHE.lock().ok()?;
    if let Some(ref sdk) = *cache {
        return Some(Arc::clone(sdk));
    }

    // 1. Check internal app storage resources/sdk first
    let sdk_dir = get_sdk_dir(program_path);
    if sdk_dir.exists() {
        if let Ok(idx) = parse_sdk_directory(&sdk_dir, "App Resources (resources/sdk)") {
            let arc_idx = Arc::new(idx);
            *cache = Some(Arc::clone(&arc_idx));
            return Some(arc_idx);
        }
    }

    // 2. Check local game folder UE4SS CXXHeaderDump if game_path is available
    if !game_path.is_empty() {
        let game_cxx = PathBuf::from(game_path)
            .join("Pal")
            .join("Binaries")
            .join("Win64")
            .join("ue4ss")
            .join("CXXHeaderDump");
        if game_cxx.exists() {
            if let Ok(idx) = parse_sdk_directory(&game_cxx, "Local UE4SS (CXXHeaderDump)") {
                let arc_idx = Arc::new(idx);
                *cache = Some(Arc::clone(&arc_idx));
                return Some(arc_idx);
            }
        }

        let ws_cxx = PathBuf::from(game_path)
            .join("Mods")
            .join("NativeMods")
            .join("UE4SS")
            .join("CXXHeaderDump");
        if ws_cxx.exists() {
            if let Ok(idx) = parse_sdk_directory(&ws_cxx, "Workshop UE4SS (CXXHeaderDump)") {
                let arc_idx = Arc::new(idx);
                *cache = Some(Arc::clone(&arc_idx));
                return Some(arc_idx);
            }
        }
    }

    None
}

pub fn invalidate_schema_cache() {
    if let Ok(mut cache) = ACTIVE_SCHEMA_CACHE.lock() {
        *cache = None;
    }
}

pub fn invalidate_sdk_cache() {
    if let Ok(mut cache) = ACTIVE_SDK_CACHE.lock() {
        *cache = None;
    }
}

