pub mod models;
pub mod gvas;
pub mod discovery;
pub mod deep_scan;
pub mod repair;

// Re-export all models for seamless backward compatibility
pub use models::*;

// Re-export GVAS functions
pub use gvas::{
    decompress_palworld_save, compress_palworld_save,
    gvas_fstring, gvas_find_property, gvas_read_int,
    gvas_read_int64, gvas_read_str, gvas_read_float, gvas_read_bool,
};

// Re-export discovery functions
pub use discovery::{
    detect_palworld_save_roots, list_save_worlds, load_world_custom_meta,
    save_world_custom_meta, open_world_folder, parse_world_options,
    quick_check_save_health,
};

// Re-export deep scan functions
pub use deep_scan::{
    deep_scan_save, inspect_snapshot_details, list_available_backups,
    format_snapshot_timestamp, detect_external_edits_and_anomalies,
    parse_player_roster, calculate_storage_breakdown,
};

// Re-export repair and backup functions
pub use repair::{
    repair_and_sanitize_save, restore_save_from_backup,
    create_manual_world_backup, prune_world_backups,
    list_pmm_world_backups, restore_pmm_world_backup, delete_pmm_world_backup,
};
