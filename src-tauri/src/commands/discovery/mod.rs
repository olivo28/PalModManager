// Discovery commands module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod types;
pub mod client;
pub mod categories;
pub mod mods;
pub mod details;
pub mod actions;

// Re-export types
pub use types::{
    DiscoveryModItem, DiscoveryCategory,
    DiscoveryFileItem, DiscoveryModDetails,
    DiscoveryResponse,
};

// Re-export client helpers
pub use client::{get_client, format_file_size, APP_VERSION, GRAPHQL_ENDPOINT, REST_BASE_URL};

// Re-export Tauri commands
pub use categories::get_discovery_categories;
pub use mods::get_discovery_mods;
pub use details::get_discovery_mod_details;
pub use actions::{
    endorse_nexus_mod, abstain_nexus_mod,
    track_nexus_mod, untrack_nexus_mod,
    install_discovery_file,
};
