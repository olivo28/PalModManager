use palmodmanager_lib::nexus::parse_mod_filename;

#[test]
fn test_parse_palvariety_filenames() {
    let file1 = "PalVariety Rarity 1to500 V1.0.0 4140 1 2026-08-12T20-44Z vcf5TPXlj.zip";
    let res1 = parse_mod_filename(file1);
    assert_eq!(res1.name, Some("PalVariety Rarity 1to500".to_string()));
    assert_eq!(res1.version, Some("1.0.0".to_string()));
    assert_eq!(res1.nexus_id, Some(4140));
    assert_eq!(res1.nexus_file_id, Some("vcf5TPXlj".to_string()));

    let file2 = "PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip";
    let res2 = parse_mod_filename(file2);
    assert_eq!(res2.name, Some("PalVariety Rarity Shiny4096".to_string()));
    assert_eq!(res2.version, Some("1.1.3".to_string()));
    assert_eq!(res2.nexus_id, Some(4140));
    assert_eq!(res2.nexus_file_id, Some("QXTyhgia8".to_string()));
}
