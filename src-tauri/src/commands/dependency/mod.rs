// Dependency commands module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod status;
pub mod safety;
pub mod install;
pub mod vault;
pub mod storage;

// Status & Remote checks
pub use status::{
    check_dependencies, check_dependencies_full,
    check_ue4ss_latest, check_palschema_latest,
    clean_conflict_dlls, reset_workshop_cache,
    compare_versions, empty_status, parse_dmy,
};

// Safety backups
pub use safety::{
    get_safety_backup_info_command,
    trigger_safety_backup_command,
    restore_safety_backup_command,
};

// Installation & Uninstallation
pub use install::{
    install_ue4ss, install_palschema,
    uninstall_ue4ss, uninstall_palschema,
    apply_ue4ss_zip_bytes, apply_palschema_zip_bytes,
    copy_dir_all, find_extracted_root,
};

// Dependency Vault
pub use vault::{
    get_dependency_vault, install_dependency_from_vault,
    install_dependency_from_custom_zip, delete_dependency_vault_entry,
    open_dependency_vault_folder, get_vault_dir,
    sanitize_version_tag, extract_version_from_vault_filename,
    save_to_vault, migrate_legacy_dependency_zips,
};

// Storage & Temp
pub use storage::{
    get_storage_usage_command, clear_temp_downloads_command,
    open_temp_folder_command, open_library_folder_command,
    StorageUsageInfo,
};
