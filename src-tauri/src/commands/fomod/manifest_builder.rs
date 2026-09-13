use std::path::{Path, PathBuf};
use crate::models::{FileRoute, InstallManifest, ModType, RouteType};
use super::types::FomodFileEntry;

/// Builds an InstallManifest from selected FOMOD files and folders.
pub fn build_fomod_install_manifest_internal(
    game_path: &Path,
    selected_files: &[FomodFileEntry],
    raw_name: &str,
    custom_folder: Option<String>,
    nexus_id: Option<u32>,
    version: String,
    author: Option<String>,
    summary: Option<String>,
    fomod_choices: Option<std::collections::HashMap<String, Vec<String>>>,
) -> Result<InstallManifest, String> {
    if selected_files.is_empty() {
        return Err("No files selected in FOMOD wizard".to_string());
    }

    let clean_name = raw_name.trim();
    let folder_name = custom_folder.unwrap_or_else(|| {
        clean_name
            .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_")
            .trim_matches('_')
            .to_string()
    });

    let is_wingdk = game_path.join("Pal").join("Binaries").join("WinGDK").exists();

    // Sort files by priority ascending (higher priority overwrites earlier entries)
    let mut sorted_entries = selected_files.to_vec();
    sorted_entries.sort_by_key(|e| e.priority);

    let mut routes = Vec::new();
    let mut has_pak = false;
    let mut has_ue4ss = false;
    let mut has_palschema = false;

    for entry in sorted_entries {
        let clean_source = entry.source.replace('\\', "/").trim_start_matches('/').to_string();
        let mut clean_dest = entry.destination.replace('\\', "/").trim_start_matches('/').to_string();

        if is_wingdk && clean_dest.contains("Win64") {
            clean_dest = clean_dest.replace("Win64", "WinGDK");
        }

        // Determine destination absolute path on disk
        let dest_full_path: PathBuf = if clean_dest.to_lowercase().starts_with("pal/") {
            game_path.join(&clean_dest)
        } else if clean_dest.is_empty() || clean_dest == "." {
            // Unspecified destination: infer from source path
            let src_lower = clean_source.to_lowercase();
            if src_lower.ends_with(".pak") {
                game_path.join("Pal").join("Content").join("Paks").join("~mods")
            } else if src_lower.contains("palschema") || src_lower.contains("blueprints") {
                let bin_folder = if is_wingdk { "WinGDK" } else { "Win64" };
                game_path.join("Pal").join("Binaries").join(bin_folder).join("ue4ss").join("Mods").join("PalSchema").join("mods").join(&folder_name)
            } else {
                let bin_folder = if is_wingdk { "WinGDK" } else { "Win64" };
                game_path.join("Pal").join("Binaries").join(bin_folder).join("ue4ss").join("Mods").join(&folder_name)
            }
        } else {
            // Partial relative destination (e.g. "~mods", "ue4ss/Mods/...")
            let dest_lower = clean_dest.to_lowercase();
            if dest_lower.contains("paks") || dest_lower == "~mods" || dest_lower == "logicmods" {
                game_path.join("Pal").join("Content").join("Paks").join(&clean_dest)
            } else if dest_lower.contains("ue4ss") || dest_lower.contains("mods") {
                let bin_folder = if is_wingdk { "WinGDK" } else { "Win64" };
                game_path.join("Pal").join("Binaries").join(bin_folder).join(&clean_dest)
            } else {
                game_path.join(&clean_dest)
            }
        };

        // Determine route type
        let dest_str = dest_full_path.to_string_lossy().replace('\\', "/");
        let dest_lower = dest_str.to_lowercase();

        let route_type = if dest_lower.contains("palschema") {
            has_palschema = true;
            RouteType::PalSchema
        } else if dest_lower.contains("logicmods") {
            has_pak = true;
            RouteType::LogicMods
        } else if dest_lower.contains("content/paks") || clean_source.to_lowercase().ends_with(".pak") {
            has_pak = true;
            RouteType::Pak
        } else if dest_lower.contains("ue4ss/mods") {
            has_ue4ss = true;
            RouteType::Ue4ss
        } else {
            RouteType::Passthrough
        };

        routes.push(FileRoute {
            zip_path: clean_source,
            dest_path: dest_str,
            route_type,
        });
    }

    let mod_type = if (has_ue4ss && has_palschema) || (has_ue4ss && has_pak) || (has_palschema && has_pak) {
        ModType::Hybrid
    } else if has_palschema {
        ModType::PalSchema
    } else if has_ue4ss {
        ModType::Ue4ss
    } else {
        ModType::Pak
    };

    Ok(InstallManifest {
        folder_name,
        display_name: clean_name.to_string(),
        mod_type,
        routes,
        nexus_mod_id: nexus_id,
        nexus_file_id: None,
        has_pak,
        has_ue4ss,
        has_palschema,
        version,
        author,
        summary,
        picture_url: None,
        fomod_choices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_fomod_install_manifest() {
        let game_path = Path::new("C:/Games/Palworld");
        let selected_files = vec![
            FomodFileEntry {
                source: "mod/paks/mod.pak".to_string(),
                destination: "Pal/Content/Paks/~mods".to_string(),
                priority: 0,
                is_folder: false,
            },
            FomodFileEntry {
                source: "mod/palschema/table.json".to_string(),
                destination: "Pal/Binaries/Win64/ue4ss/Mods/PalSchema/mods/BetterBaseBuilding".to_string(),
                priority: 5,
                is_folder: false,
            },
        ];

        let manifest = build_fomod_install_manifest_internal(
            game_path,
            &selected_files,
            "Better Base Building",
            None,
            Some(5544),
            "1.0.0".to_string(),
            Some("OkaRei".to_string()),
            Some("Description".to_string()),
            None,
        ).expect("manifest build failed");

        assert_eq!(manifest.display_name, "Better Base Building");
        assert_eq!(manifest.folder_name, "Better_Base_Building");
        assert_eq!(manifest.routes.len(), 2);
        assert_eq!(manifest.mod_type, ModType::Hybrid);
        assert!(manifest.has_pak);
        assert!(manifest.has_palschema);
        assert_eq!(manifest.nexus_mod_id, Some(5544));
        assert_eq!(manifest.fomod_choices, None);
    }

    #[test]
    fn test_build_fomod_install_manifest_with_choices() {
        let game_path = Path::new("C:/Games/Palworld");
        let selected_files = vec![
            FomodFileEntry {
                source: "mod/paks/mod.pak".to_string(),
                destination: "Pal/Content/Paks/~mods".to_string(),
                priority: 0,
                is_folder: false,
            },
        ];

        let mut choices = std::collections::HashMap::new();
        choices.insert("Base Dimensions".to_string(), vec!["2x Range".to_string()]);

        let manifest = build_fomod_install_manifest_internal(
            game_path,
            &selected_files,
            "Better Base Building",
            None,
            Some(5544),
            "2.2.0".to_string(),
            Some("OkaRei".to_string()),
            Some("Description".to_string()),
            Some(choices.clone()),
        ).expect("manifest build failed");

        assert_eq!(manifest.fomod_choices, Some(choices));
    }
}
