// Zip and archive handling module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod types;
pub mod detection;
pub mod analysis;
pub mod extraction;
pub mod naming;
pub mod manifest;
pub mod workshop_rule;

// Re-export types
pub use types::{ArchiveFormat, DetectedModType, ZipAnalysis};

// Re-export detection and analysis
pub use detection::{detect_archive_format, extract_nexus_id_from_path, find_resilient_zip_boundary, find_root_folder, list_7z_files, list_rar_files, open_resilient_zip};
pub use analysis::analyze_zip;

// Re-export extraction and reading
pub use extraction::{extract_7z_to_temp, extract_rar_to_temp, extract_zip_to_temp, find_pak_companions, read_7z_file, read_archive_file};

// Re-export naming
pub use naming::{detect_folder_name_from_files, is_forbidden, FORBIDDEN_MOD_NAMES, GAME_PATH_SEGMENTS, PALSCHEMA_FOLDERS};

// Re-export manifest builder
pub use manifest::{build_install_manifest, build_manifest_from_files};
