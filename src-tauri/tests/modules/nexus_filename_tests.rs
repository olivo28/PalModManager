use palmodmanager_lib::nexus::{parse_mod_filename, extract_nexus_id};
use palmodmanager_lib::commands::dependency::extract_version_from_vault_filename;

#[test]
fn test_parse_palvariety_with_alphanumeric_tokens() {
    // Case 1: PalVariety 1to500 with token vcf5TPXlj
    let file1 = "PalVariety Rarity 1to500 V1.0.0 4140 1 2026-08-12T20-44Z vcf5TPXlj.zip";
    let res1 = parse_mod_filename(file1);
    assert_eq!(res1.name, Some("PalVariety Rarity 1to500".to_string()));
    assert_eq!(res1.version, Some("1.0.0".to_string()));
    assert_eq!(res1.nexus_id, Some(4140));
    assert_eq!(res1.nexus_file_id, Some("vcf5TPXlj".to_string()));

    // Case 2: PalVariety Shiny4096 with token QXTyhgia8
    let file2 = "PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip";
    let res2 = parse_mod_filename(file2);
    assert_eq!(res2.name, Some("PalVariety Rarity Shiny4096".to_string()));
    assert_eq!(res2.version, Some("1.1.3".to_string()));
    assert_eq!(res2.nexus_id, Some(4140));
    assert_eq!(res2.nexus_file_id, Some("QXTyhgia8".to_string()));

    println!("  [NEXUS] PalVariety 1to500: name={:?}, version={:?}, nexus_id={:?}, file_id={:?}", res1.name, res1.version, res1.nexus_id, res1.nexus_file_id);
    println!("  [NEXUS] PalVariety Shiny:  name={:?}, version={:?}, nexus_id={:?}, file_id={:?}", res2.name, res2.version, res2.nexus_id, res2.nexus_file_id);
}

#[test]
fn test_parse_pal_insight_and_expedition_tokens() {
    // Case 3: Pal Insight 1.6.0 with token 5V0tE6iaU
    let file3 = "Pal Insight 1.6.0 4638 1.6.0 2026-08-26T17-37Z 5V0tE6iaU.zip";
    let res3 = parse_mod_filename(file3);
    assert_eq!(res3.name, Some("Pal Insight".to_string()));
    assert_eq!(res3.nexus_id, Some(4638));
    assert_eq!(res3.nexus_file_id, Some("5V0tE6iaU".to_string()));

    // Case 4: ExpeditionTimerHUD with token tlYGMmRGl
    let file4 = "ExpeditionTimerHUD 4687 8 2026-08-26T20-02Z tlYGMmRGl.zip";
    let res4 = parse_mod_filename(file4);
    assert_eq!(res4.name, Some("ExpeditionTimerHUD".to_string()));
    assert_eq!(res4.nexus_id, Some(4687));
    assert_eq!(res4.nexus_file_id, Some("tlYGMmRGl".to_string()));

    println!("  [NEXUS] Pal Insight: name={:?}, nexus_id={:?}, file_id={:?}", res3.name, res3.nexus_id, res3.nexus_file_id);
    println!("  [NEXUS] Expedition:  name={:?}, nexus_id={:?}, file_id={:?}", res4.name, res4.nexus_id, res4.nexus_file_id);
}

#[test]
fn test_parse_standard_hyphen_separated_nexus_patterns() {
    // Name-ID-Version-Timestamp
    let hyphen_file = "CarryWeightMod-3866-1-2-1706398042.zip";
    let res = parse_mod_filename(hyphen_file);
    assert_eq!(res.name, Some("CarryWeightMod".to_string()));
    assert_eq!(res.nexus_id, Some(3866));
    assert_eq!(res.version, Some("1.2".to_string()));

    // Hyphen separated with trailing alphanumeric file hash/token
    let hyphen_token_file = "InfiniteStamina-4500-2-0-2026-08-15-X9kLa12b.zip";
    let res_token = parse_mod_filename(hyphen_token_file);
    assert_eq!(res_token.name, Some("InfiniteStamina".to_string()));
    assert_eq!(res_token.nexus_id, Some(4500));
    assert_eq!(res_token.nexus_file_id, Some("X9kLa12b".to_string()));

    println!("  [NEXUS] Hyphen pattern:  name={:?}, nexus_id={:?}, version={:?}", res.name, res.nexus_id, res.version);
    println!("  [NEXUS] With hash token: name={:?}, nexus_id={:?}, file_id={:?}", res_token.name, res_token.nexus_id, res_token.nexus_file_id);
}

#[test]
fn test_parse_platform_tag_cleanup_and_local_unversioned() {
    // Strips platform tags like (Steam), Singleplayer, etc. from mod name
    let steam_tagged = "MapUnlocker (Steam) 4321 1.0.4 2026-08-01T12-00Z.zip";
    let res_steam = parse_mod_filename(steam_tagged);
    assert_eq!(res_steam.name, Some("MapUnlocker".to_string()));
    assert_eq!(res_steam.nexus_id, Some(4321));

    // Plain local archive without Nexus metadata
    let local_file = "MyPersonalTweaks.zip";
    let res_local = parse_mod_filename(local_file);
    assert_eq!(res_local.name, None);
    assert_eq!(res_local.nexus_id, None);
    assert_eq!(res_local.nexus_file_id, None);

    // Internal nexus_ backup name should be ignored
    let internal_file = "nexus_4140.zip";
    let res_internal = parse_mod_filename(internal_file);
    assert_eq!(res_internal.name, None);
    assert_eq!(res_internal.nexus_id, None);

    println!("  [NEXUS] Cleaned platform tag: name={:?}, nexus_id={:?}", res_steam.name, res_steam.nexus_id);
}

#[test]
fn test_extract_nexus_id_and_version_helpers() {
    assert_eq!(extract_nexus_id("PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip"), Some(4140));
    assert_eq!(extract_nexus_id("CarryWeightMod-3866-1-2-1706398042.zip"), Some(3866));
    assert_eq!(extract_nexus_id("RandomUnversionedFile.zip"), None);

    assert_eq!(extract_version_from_vault_filename("UE4SS_v3.0.1.zip", "ue4ss"), "v3.0.1");
    assert_eq!(extract_version_from_vault_filename("PalSchema-0.4.1.zip", "palschema"), "0.4.1");

    println!("  [NEXUS] Extracted Nexus ID from filename: {:?}", extract_nexus_id("CarryWeightMod-3866-1-2-1706398042.zip"));
    println!("  [NEXUS] Vault version extraction: 'UE4SS_v3.0.1.zip' -> 'v3.0.1'");
}
