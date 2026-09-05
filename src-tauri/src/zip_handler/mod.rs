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
pub use detection::{detect_archive_format, extract_nexus_id_from_path, find_resilient_zip_boundary, find_root_folder, list_7z_files, list_rar_files, open_resilient_zip};
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

    #[test]
    fn test_resilient_zip_with_trailing_bytes() {
        use std::io::Write;
        let mut zip_buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::write::ZipWriter::new(&mut zip_buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("test.txt", options).unwrap();
            writer.write_all(b"Hello World").unwrap();
            writer.finish().unwrap();
        }
        let original_zip_bytes = zip_buffer.into_inner();
        let original_len = original_zip_bytes.len();

        // Append 70,000 bytes of trailing garbage (exceeding standard 65KB EOCD comment search window)
        let mut corrupted_bytes = original_zip_bytes.clone();
        corrupted_bytes.extend_from_slice(&vec![0xCC; 70_000]);

        // Verify resilient boundary detects the exact original length
        let detected_boundary = find_resilient_zip_boundary(&corrupted_bytes);
        assert_eq!(detected_boundary, Some(original_len), "Should locate exact EOCD end before trailing bytes");

        // Verify trimmed buffer opens and reads file successfully
        let trimmed_attempt = zip::read::ZipArchive::new(std::io::Cursor::new(&corrupted_bytes[0..detected_boundary.unwrap()]));
        assert!(trimmed_attempt.is_ok(), "Trimmed resilient buffer must open successfully");
        let mut archive = trimmed_attempt.unwrap();
        assert_eq!(archive.len(), 1);
        let mut entry = archive.by_index(0).unwrap();
        let mut text = String::new();
        std::io::Read::read_to_string(&mut entry, &mut text).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_pal_mercy_toggle_assets_routing() {
        let files = vec![
            "PalMercyToggle/Assets/mercy_on.png".to_string(),
            "PalMercyToggle/enabled.txt".to_string(),
            "PalMercyToggle/Scripts/main.lua".to_string(),
            "PalMercyToggle/Scripts/uky_pal_mercy_toggle_config.lua".to_string(),
            "PalMercyToggle/Scripts/uky_pal_mercy_toggle_core.lua".to_string(),
        ];
        let game_path = PathBuf::from("C:/FakeGamePath");
        let manifest = build_manifest_from_files(&files, "PalMercyToggle.zip", &game_path, None, None, None)
            .expect("Should generate manifest");

        assert_eq!(manifest.mod_type, ModType::Ue4ss);
        assert_eq!(manifest.folder_name, "PalMercyToggle");

        let asset_route = manifest.routes.iter().find(|r| r.zip_path.contains("mercy_on.png"));
        assert!(asset_route.is_some(), "mercy_on.png must be present in manifest routes and not dropped");
        let route = asset_route.unwrap();
        assert_eq!(route.route_type, RouteType::Ue4ss);
        assert!(
            route.dest_path.ends_with("PalMercyToggle\\Assets\\mercy_on.png")
                || route.dest_path.ends_with("PalMercyToggle/Assets/mercy_on.png"),
            "Expected PalMercyToggle/Assets/mercy_on.png but got: {}",
            route.dest_path
        );
    }

    #[test]
    fn test_pal_insight_2_0_2_config_root_routing() {
        let files = vec![
            "Pal/Binaries/Win64/ue4ss/Mods/PalInsight/config.lua".to_string(),
            "Pal/Binaries/Win64/ue4ss/Mods/PalInsight/enabled.txt".to_string(),
            "Pal/Binaries/Win64/ue4ss/Mods/PalInsight/Scripts/main.lua".to_string(),
            "Pal/Content/Paks/LogicMods/PalInsightX.pak".to_string(),
        ];
        let game_path = PathBuf::from("C:/FakeGamePath");
        let manifest = build_manifest_from_files(&files, "PalInsight.zip", &game_path, None, None, None)
            .expect("Should generate manifest");

        let config_route = manifest.routes.iter().find(|r| r.zip_path.ends_with("config.lua"))
            .expect("config.lua must be routed");

        assert!(
            !config_route.dest_path.contains("Scripts\\config.lua") && !config_route.dest_path.contains("Scripts/config.lua"),
            "config.lua must NOT be placed into Scripts/ subdirectory when it was at mod root: {}",
            config_route.dest_path
        );
        assert!(
            config_route.dest_path.ends_with("PalInsight\\config.lua") || config_route.dest_path.ends_with("PalInsight/config.lua"),
            "config.lua must remain at root of PalInsight folder: {}",
            config_route.dest_path
        );
    }
}
