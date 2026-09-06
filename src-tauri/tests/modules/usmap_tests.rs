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
