pub mod models;
pub mod build_detector;
pub mod sync;
pub mod parser;

pub use models::*;
pub use build_detector::detect_installed_game_build;
pub use sync::{get_mappings_status, sync_mappings_async, get_active_usmap_path};
pub use parser::{parse_usmap_file, UsmapSchema, UsmapStruct, UsmapProperty};

use std::sync::Mutex;

static ACTIVE_SCHEMA_CACHE: Mutex<Option<UsmapSchema>> = Mutex::new(None);

pub fn get_or_load_schema(program_path: &str) -> Option<UsmapSchema> {
    let mut cache = ACTIVE_SCHEMA_CACHE.lock().ok()?;
    if let Some(ref schema) = *cache {
        return Some(schema.clone());
    }

    let usmap_path = get_active_usmap_path(program_path);
    if usmap_path.exists() {
        match parse_usmap_file(&usmap_path) {
            Ok(schema) => {
                crate::logger::log(&format!("Loaded USMAP Schema: {} structs, {} enums, {} names", schema.total_structs, schema.total_enums, schema.total_names));
                *cache = Some(schema.clone());
                Some(schema)
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

pub fn invalidate_schema_cache() {
    if let Ok(mut cache) = ACTIVE_SCHEMA_CACHE.lock() {
        *cache = None;
    }
}
