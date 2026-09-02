// Zip and archive handling module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod types;
pub mod detection;
pub mod analysis;
pub mod extraction;
pub mod naming;
pub mod manifest;

// Re-export types
pub use types::{ArchiveFormat, DetectedModType, ZipAnalysis};

// Re-export detection and analysis
pub use detection::{detect_archive_format, extract_nexus_id_from_path, find_root_folder, list_7z_files, list_rar_files};
pub use analysis::analyze_zip;

// Re-export extraction and reading
pub use extraction::{extract_7z_to_temp, extract_rar_to_temp, extract_zip_to_temp, find_pak_companions, read_7z_file, read_archive_file};

// Re-export naming
pub use naming::{detect_folder_name_from_files, is_forbidden, FORBIDDEN_MOD_NAMES, GAME_PATH_SEGMENTS, PALSCHEMA_FOLDERS};

// Re-export manifest builder
pub use manifest::{build_install_manifest, build_manifest_from_files};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::models::{ModType, RouteType};

    #[test]
    fn test_pal_insight_manifest_routing() {
        let zip_file = "C:/Users/Antikux/Downloads/Pal Insight 1.6.0 4638 1.6.0 2026-08-26T17-37Z 5V0tE6iaU.zip";
        if !std::path::Path::new(zip_file).exists() {
            return;
        }
        let game_path = PathBuf::from("C:/FakeGamePath");
        let manifest = build_install_manifest(zip_file, &game_path, None, None).expect("Should build manifest");

        println!("Pal Insight Manifest: folder_name={}, version={}, mod_type={:?}", manifest.folder_name, manifest.version, manifest.mod_type);
        for r in &manifest.routes {
            println!("  ROUTE: {} -> {}", r.zip_path, r.dest_path);
        }

        // Verify PalInsight and PalInsightSettings are routed separately without nesting
        let has_mangled_nesting = manifest.routes.iter().any(|r| {
            r.dest_path.contains("PalInsightSettings\\PalInsight") || r.dest_path.contains("PalInsightSettings/PalInsight")
        });
        assert!(!has_mangled_nesting, "Should not nest PalInsight under PalInsightSettings!");

        let has_palinsight_lua = manifest.routes.iter().any(|r| {
            r.dest_path.ends_with("PalInsight\\Scripts\\main.lua") || r.dest_path.ends_with("PalInsight/Scripts/main.lua")
        });
        assert!(has_palinsight_lua, "PalInsight main.lua must be in PalInsight/Scripts/main.lua");

        let has_settings_lua = manifest.routes.iter().any(|r| {
            r.dest_path.ends_with("PalInsightSettings\\Scripts\\main.lua") || r.dest_path.ends_with("PalInsightSettings/Scripts/main.lua")
        });
        assert!(has_settings_lua, "PalInsightSettings main.lua must be in PalInsightSettings/Scripts/main.lua");

        let has_pak = manifest.routes.iter().any(|r| {
            r.dest_path.contains("LogicMods") && r.dest_path.ends_with("PalInsightX.pak")
        });
        assert!(has_pak, "PalInsightX.pak must be in LogicMods");
    }

    #[test]
    fn test_expedition_timer_hud_manifest_routing() {
        let zip_file = "C:/Users/Antikux/Downloads/ExpeditionTimerHUD 4687 8 2026-08-26T20-02Z tlYGMmRGl.zip";
        if !std::path::Path::new(zip_file).exists() {
            return;
        }
        let game_path = PathBuf::from("C:/FakeGamePath");
        let analysis = analyze_zip(zip_file).expect("Should analyze zip");
        println!("Analysis: {:?}", analysis);
        let manifest = build_install_manifest(zip_file, &game_path, None, None).expect("Should build manifest");

        println!("ExpeditionTimerHUD Manifest: folder_name={}, version={}, mod_type={:?}", manifest.folder_name, manifest.version, manifest.mod_type);
        for r in &manifest.routes {
            println!("  ROUTE: {} -> {} (type: {:?})", r.zip_path, r.dest_path, r.route_type);
        }
    }

    #[test]
    fn test_raid_and_trade_revival_routing() {
        let files = vec![
            "Pal/Binaries/Win64/ue4ss/Mods/PalSchema/mods/RaidTradeRevivalContent/raw/AllNewRaidGroups.json".to_string(),
            "Pal/Binaries/Win64/ue4ss/Mods/PalSchema/mods/RaidTradeRevivalContent/raw/ExpandedVanillaRaidFamilies.json".to_string(),
            "Pal/Binaries/Win64/ue4ss/Mods/RaidTradeRevival/Scripts/main.lua".to_string(),
            "Pal/Binaries/Win64/ue4ss/Mods/RaidTradeRevival/config.txt".to_string(),
            "Pal/Binaries/Win64/ue4ss/Mods/RaidTradeRevival/enabled.txt".to_string(),
        ];
        let game_path = PathBuf::from("C:/FakeGamePath");
        let manifest = build_manifest_from_files(&files, "RaidTradeRevival.zip", &game_path, None, None, None)
            .expect("Should generate manifest");

        assert_eq!(manifest.mod_type, ModType::Hybrid);
        assert_eq!(manifest.folder_name, "RaidTradeRevival");

        for r in &manifest.routes {
            if r.zip_path.ends_with("AllNewRaidGroups.json") {
                assert_eq!(r.route_type, RouteType::PalSchema);
                assert!(
                    r.dest_path.contains("PalSchema\\mods\\RaidTradeRevivalContent\\raw\\AllNewRaidGroups.json")
                    || r.dest_path.contains("PalSchema/mods/RaidTradeRevivalContent/raw/AllNewRaidGroups.json"),
                    "PalSchema table must route to PalSchema/mods/RaidTradeRevivalContent, got: {}",
                    r.dest_path
                );
            }
            if r.zip_path.ends_with("main.lua") {
                assert_eq!(r.route_type, RouteType::Ue4ss);
                assert!(
                    r.dest_path.contains("RaidTradeRevival\\Scripts\\main.lua")
                    || r.dest_path.contains("RaidTradeRevival/Scripts/main.lua"),
                    "UE4SS script must route to RaidTradeRevival/Scripts/main.lua, got: {}",
                    r.dest_path
                );
            }
        }
    }
}
