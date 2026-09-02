// Scanner engine module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod meta;
pub mod ue4ss;
pub mod palschema;
pub mod paks;
pub mod disabled;
pub mod merge;
pub mod engine;

// Re-export meta loader
pub use meta::load_pmm_meta;

// Re-export scanners
pub use ue4ss::scan_ue4ss_mods;
pub use palschema::scan_palschema_mods;
pub use paks::scan_pak_mods;
pub use disabled::scan_disabled_mods;

// Re-export merger & main engine
pub use merge::merge_scan_with_db;
pub use engine::scan_mods_internal;
