use std::fs;
use std::path::PathBuf;
use palmodmanager_lib::usmap::parse_usmap_file;

#[test]
fn test_parse_bundled_usmap() {
    let usmap_path = PathBuf::from("../resources/mappings/Palworld_24575825.usmap");
    if usmap_path.exists() {
        let buffer = fs::read(&usmap_path).unwrap();
        eprintln!("USMAP file len = {}", buffer.len());
        let schema = parse_usmap_file(&usmap_path).expect("Should parse Palworld_24575825.usmap cleanly");
        eprintln!("PARSED USMAP RESULT: structs={}, enums={}, names={}", schema.total_structs, schema.total_enums, schema.total_names);
        
        // Search for IsTrigger
        let mut found_props = Vec::new();
        for (sname, ustruct) in &schema.structs {
            for prop in &ustruct.properties {
                if prop.name.to_ascii_lowercase().contains("istrigger") {
                    found_props.push((sname.clone(), prop.name.clone()));
                }
            }
        }
        eprintln!("Found properties matching istrigger: {:?}", found_props);
    }
}

#[test]
fn test_parse_v1_0_4_usmap() {
    let usmap_path = PathBuf::from("../resources/mappings/Palworld_25094871.usmap");
    if usmap_path.exists() {
        let schema = parse_usmap_file(&usmap_path).expect("Should parse Palworld_25094871.usmap cleanly");
        eprintln!("PARSED V1.0.4 USMAP RESULT: structs={}, enums={}, names={}", schema.total_structs, schema.total_enums, schema.total_names);
        assert!(schema.total_structs > 0);
        assert!(schema.total_names > 0);
    }
}

#[test]
fn test_master_manifest_and_dynamic_version_resolution() {
    use palmodmanager_lib::usmap::{
        get_or_load_master_manifest, resolve_game_version, resolve_usmap_relative_path,
        resolve_game_version_refined, resolve_usmap_relative_path_refined, resolve_palschema_version,
    };

    let manifest = get_or_load_master_manifest("").expect("Should load Master Resource Manifest");
    assert_eq!(manifest.latest_game_version, "v1.0.4");
    assert_eq!(manifest.latest_steam_build_id, "25094871");

    // Dynamic resolution by build ID
    let ver_104 = resolve_game_version(&manifest, Some("25094871"));
    assert_eq!(ver_104, "v1.0.4");

    let ver_103 = resolve_game_version(&manifest, Some("24575825"));
    assert_eq!(ver_103, "v1.0.3");

    // Unknown build ID falls back dynamically to latest_game_version
    let ver_unknown = resolve_game_version(&manifest, Some("99999999"));
    assert_eq!(ver_unknown, "v1.0.4");

    // USMAP path resolution
    let usmap_104 = resolve_usmap_relative_path(&manifest, Some("25094871"));
    assert_eq!(usmap_104.as_deref(), Some("mappings/Palworld_25094871.usmap"));

    // Compound key resolution (Steam build ID + UE4SS commit)
    let ver_compound = resolve_game_version_refined(&manifest, Some("25094871"), Some("2281fa31"));
    assert_eq!(ver_compound, "v1.0.4");

    let usmap_compound = resolve_usmap_relative_path_refined(&manifest, Some("25094871"), Some("2281fa31"));
    assert_eq!(usmap_compound.as_deref(), Some("mappings/Palworld_25094871.usmap"));

    // Fallback when commit is different or unknown
    let ver_fallback_commit = resolve_game_version_refined(&manifest, Some("25094871"), Some("unknown_commit"));
    assert_eq!(ver_fallback_commit, "v1.0.4");

    // Independent PalSchema version resolution
    let ps_104 = resolve_palschema_version(&manifest, Some("25094871"));
    assert_eq!(ps_104.as_deref(), Some("0.6.7"));

    let ps_103 = resolve_palschema_version(&manifest, Some("24575825"));
    assert_eq!(ps_103.as_deref(), Some("0.6.6"));
}


