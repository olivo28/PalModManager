// Profile actions module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod folder_name;
pub mod mods_txt;
pub mod disable;
pub mod enable;
pub mod reconcile;
pub mod sync;

// Re-export folder name
pub use folder_name::get_mod_folder_name;

// Re-export mods_txt helpers
pub use mods_txt::{update_mods_txt_load_order, remove_from_mods_txt};

// Re-export disable and enable operations
pub use disable::disable_mod_internal;
pub use enable::enable_mod_internal;

// Re-export reconcile and sync
pub use reconcile::reconcile_ue4ss_control_mode;
pub use sync::{clean_mods_txt_native_only, sync_mods_txt_sections};
