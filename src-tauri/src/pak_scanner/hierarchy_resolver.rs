use std::collections::HashMap;
use std::path::Path;

/// Traces the inheritance hierarchy for an asset/class name using the loaded USMAP schema
pub fn resolve_class_hierarchy(class_or_struct_name: &str) -> Vec<String> {
    let mut hierarchy = Vec::new();
    let schema = match crate::usmap::get_or_load_schema("") {
        Some(s) => s,
        None => return hierarchy,
    };

    let mut current = class_or_struct_name.to_string();
    let mut visited = std::collections::HashSet::new();

    while !current.is_empty() && visited.insert(current.clone()) {
        hierarchy.push(current.clone());
        if let Some(st) = schema.find_struct(&current) {
            if let Some(ref parent) = st.super_type {
                if !parent.is_empty() && parent != "None" {
                    current = parent.clone();
                    continue;
                }
            }
        }
        break;
    }

    hierarchy
}

/// Checks imported assets against known vanilla Blueprint / Engine paths
pub fn verify_vanilla_references(imports: &[super::types::UAssetImportItem]) -> HashMap<String, bool> {
    let mut results = HashMap::new();

    for imp in imports {
        let path = if imp.class_package.starts_with("/Game/") {
            &imp.class_package
        } else {
            &imp.object_name
        };

        // Vanilla references in Palworld reside under /Engine/, /Script/, or official /Game/Pal/ paths
        let is_vanilla = path.starts_with("/Engine/")
            || path.starts_with("/Script/")
            || (path.starts_with("/Game/Pal/") && !path.contains("LogicMods") && !path.contains("~mods"));

        results.insert(path.clone(), is_vanilla);
    }

    results
}

/// Checks if an authentic `.original.bak` backup file exists for a given `.pak` file path
pub fn check_pak_backup_exists(pak_path: &Path) -> bool {
    let bak_path = pak_path.with_extension("pak.original.bak");
    bak_path.exists()
}
