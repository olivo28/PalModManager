// Installer engine module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod helpers;
pub mod exists;
pub mod execution;
pub mod install;
pub mod update;

// Re-export helper functions
pub use helpers::{
    clean_zip_name, normalize_path_separator,
    get_ue4ss_component_root, get_palschema_component_root,
    copy_folder_contents, detect_config_local,
    determine_mod_id, normalize_name,
    get_physical_identity, move_path,
};

// Re-export existence checking
pub use exists::check_mod_exists;

// Re-export execution
pub use execution::execute_manifest;

// Re-export install
pub use install::install_mod;

// Re-export update
pub use update::update_mod;
