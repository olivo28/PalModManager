use palmodmanager_lib::commands::config_archive::{sanitize_archive_key, get_archive_dir};

#[test]
fn test_sanitize_archive_key() {
    assert_eq!(sanitize_archive_key("PalVariety 4x (Shiny)!"), "palvariety_4x_shiny");
    assert_eq!(sanitize_archive_key("---"), "mod");
    assert_eq!(sanitize_archive_key("SimpleMod"), "simplemod");
}

#[test]
fn test_get_archive_dir() {
    let dir = get_archive_dir("C:/pmm", "default", "nexus_1234");
    assert!(dir.to_string_lossy().contains("archived_configs"));
    assert!(dir.to_string_lossy().ends_with("nexus_1234"));
}
