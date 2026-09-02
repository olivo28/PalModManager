// Install commands module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod utils;
pub mod analysis;
pub mod install_standard;
pub mod install_manifest;
pub mod update;
pub mod diff;

// Re-export utils
pub use utils::{check_mod_dependencies, sync_altermatic_helper};

// Re-export analysis
pub use analysis::{analyze_zip, check_mod_exists_command};

// Re-export standard installation
pub use install_standard::install_mod_command;

// Re-export manifest-based installation
pub use install_manifest::{build_install_manifest, install_mod_with_manifest};

// Re-export updates
pub use update::update_mod_command;

// Re-export diff preview & types
pub use diff::{preview_config_diff, ConfigDiff};
