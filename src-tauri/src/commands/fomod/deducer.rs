use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use crate::models::ModInfo;
use super::types::{FomodConfig, FomodPlugin};

#[derive(Debug, Clone)]
pub struct DiskFileInfo {
    pub relative_path: String,
    pub file_name: String,
    pub size: u64,
    pub full_path: PathBuf,
}

/// Gathers all files installed on disk for the given mod
pub fn collect_mod_disk_files(game_path: &Path, mod_info: &ModInfo) -> Vec<DiskFileInfo> {
    let mut files = Vec::new();
    let mut roots = Vec::new();

    if !mod_info.game_path.is_empty() {
        let p = crate::config_merge::resolve_path_in_game(game_path, &mod_info.game_path);
        if p.exists() {
            roots.push(p);
        }
    }
    if !mod_info.disabled_path.is_empty() {
        let p = crate::config_merge::resolve_path_in_game(game_path, &mod_info.disabled_path);
        if p.exists() && !roots.contains(&p) {
            roots.push(p);
        }
    }
    for extra in &mod_info.extra_files {
        let p = crate::config_merge::resolve_path_in_game(game_path, extra);
        if p.exists() && !roots.contains(&p) {
            roots.push(p);
        }
    }

    for root in roots {
        if root.is_file() {
            let file_name = root.file_name().unwrap_or_default().to_string_lossy().to_string();
            let size = root.metadata().map(|m| m.len()).unwrap_or(0);
            files.push(DiskFileInfo {
                relative_path: file_name.clone(),
                file_name,
                size,
                full_path: root.clone(),
            });
        } else if root.is_dir() {
            for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let full_path = entry.path().to_path_buf();
                    let file_name = full_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let relative_path = full_path
                        .strip_prefix(&root)
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| file_name.clone());
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                    files.push(DiskFileInfo {
                        relative_path,
                        file_name,
                        size,
                        full_path,
                    });
                }
            }
        }
    }

    files
}

/// Correlates files currently installed on disk with the candidate options inside the FOMOD ZIP
pub fn deduce_fomod_choices_from_disk(
    game_path: &Path,
    mod_info: &ModInfo,
    config: &FomodConfig,
    zip_path: &Path,
) -> HashMap<String, Vec<String>> {
    let mut deduced_choices: HashMap<String, Vec<String>> = HashMap::new();

    let disk_files = collect_mod_disk_files(game_path, mod_info);
    if disk_files.is_empty() {
        crate::logger::log(&format!("[fomod_deducer] No files on disk found for mod '{}'", mod_info.name));
        return deduced_choices;
    }

    // Open zip archive to index entry names and uncompressed sizes
    let Ok(zip_file) = fs::File::open(zip_path) else {
        crate::logger::log(&format!("[fomod_deducer] Failed to open zip at {:?}", zip_path));
        return deduced_choices;
    };
    let mut archive = match zip::ZipArchive::new(zip_file) {
        Ok(a) => a,
        Err(e) => {
            crate::logger::log(&format!("[fomod_deducer] Failed to read zip archive: {}", e));
            return deduced_choices;
        }
    };

    // Index zip entries by normalized path -> (index, uncompressed_size)
    let mut zip_index: HashMap<String, (usize, u64)> = HashMap::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().replace('\\', "/").trim_start_matches('/').to_lowercase();
            if !file.is_dir() && !name.is_empty() {
                zip_index.insert(name, (i, file.size()));
            }
        }
    }

    for step in &config.install_steps {
        for group in &step.groups {
            let mut plugin_scores: Vec<(&FomodPlugin, u32)> = Vec::new();

            for plugin in &group.plugins {
                if plugin.type_descriptor == "NotUsable" {
                    continue;
                }

                let mut score = 0u32;

                for file_entry in &plugin.files {
                    let clean_source = file_entry.source.replace('\\', "/").trim_start_matches('/').trim_end_matches('/').to_lowercase();
                    
                    // Skip common fallback folders since they don't represent unique plugin options
                    if clean_source == "common" {
                        continue;
                    }

                    if file_entry.is_folder {
                        let prefix = format!("{}/", clean_source);
                        let matching_entries: Vec<(String, usize, u64)> = zip_index
                            .iter()
                            .filter(|(k, _)| k.starts_with(&prefix))
                            .map(|(k, v)| (k.clone(), v.0, v.1))
                            .collect();

                        for (entry_path, zip_idx, entry_size) in matching_entries {
                            let entry_file_name = Path::new(&entry_path)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_lowercase();

                            for df in &disk_files {
                                if df.file_name.to_lowercase() == entry_file_name {
                                    score += 10;
                                    if df.size == entry_size {
                                        score += 100;
                                        // Quick byte comparison for small files (< 2MB)
                                        if df.size < 2 * 1024 * 1024 {
                                            if let Ok(mut zf) = archive.by_index(zip_idx) {
                                                let mut z_bytes = Vec::with_capacity(entry_size as usize);
                                                if zf.read_to_end(&mut z_bytes).is_ok() {
                                                    if let Ok(d_bytes) = fs::read(&df.full_path) {
                                                        if z_bytes == d_bytes {
                                                            score += 1000;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Standalone file entry
                        let entry_file_name = Path::new(&clean_source)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase();

                        let zip_meta = zip_index.get(&clean_source);

                        for df in &disk_files {
                            if df.file_name.to_lowercase() == entry_file_name {
                                score += 10;
                                if let Some(&(zip_idx, entry_size)) = zip_meta {
                                    if df.size == entry_size {
                                        score += 100;
                                        if df.size < 2 * 1024 * 1024 {
                                            if let Ok(mut zf) = archive.by_index(zip_idx) {
                                                let mut z_bytes = Vec::with_capacity(entry_size as usize);
                                                if zf.read_to_end(&mut z_bytes).is_ok() {
                                                    if let Ok(d_bytes) = fs::read(&df.full_path) {
                                                        if z_bytes == d_bytes {
                                                            score += 1000;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if score > 0 {
                    plugin_scores.push((plugin, score));
                }
            }

            if plugin_scores.is_empty() {
                continue;
            }

            let is_single = group.group_type == "SelectExactlyOne" || group.group_type == "SelectAtMostOne";
            if is_single {
                plugin_scores.sort_by(|a, b| b.1.cmp(&a.1));
                let best = plugin_scores[0].0;
                deduced_choices.insert(group.name.clone(), vec![best.name.clone()]);
            } else {
                let chosen_names: Vec<String> = plugin_scores.iter().map(|(p, _)| p.name.clone()).collect();
                deduced_choices.insert(group.name.clone(), chosen_names);
            }
        }
    }

    crate::logger::log(&format!(
        "[fomod_deducer] Successfully deduced {} group choices from disk for mod '{}'",
        deduced_choices.len(),
        mod_info.name
    ));

    deduced_choices
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use crate::commands::fomod::types::*;

    #[test]
    fn test_deduce_fomod_choices_matching_filenames() {
        let temp_dir = std::env::temp_dir().join(format!("test_deduce_{}", uuid::Uuid::new_v4()));
        let mod_dir = temp_dir.join("PalSchema").join("mods").join("BetterBaseBuilding");
        fs::create_dir_all(&mod_dir.join("blueprints")).unwrap();

        let ranch_file = mod_dir.join("blueprints").join("BuildingResize Ranch X 0.5.jsonc");
        fs::write(&ranch_file, "{\"Scale\": 0.5}").unwrap();

        // Create a test zip file
        let zip_path = temp_dir.join("test_fomod.zip");
        {
            let file = fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();

            zip.start_file("options/ranch/0.4/BetterBaseBuilding/blueprints/BuildingResize Ranch X 0.4.jsonc", options).unwrap();
            zip.write_all(b"{\"Scale\": 0.4}").unwrap();

            zip.start_file("options/ranch/0.5/BetterBaseBuilding/blueprints/BuildingResize Ranch X 0.5.jsonc", options).unwrap();
            zip.write_all(b"{\"Scale\": 0.5}").unwrap();

            zip.finish().unwrap();
        }

        let config = FomodConfig {
            module_name: "BetterBaseBuilding".to_string(),
            module_image: None,
            info: None,
            required_install_files: vec![],
            install_steps: vec![FomodStep {
                name: "Resizes".to_string(),
                visible: None,
                groups: vec![FomodGroup {
                    name: "Ranch".to_string(),
                    group_type: "SelectExactlyOne".to_string(),
                    plugins: vec![
                        FomodPlugin {
                            id: "1".to_string(),
                            name: "0.4x".to_string(),
                            description: "".to_string(),
                            image: None,
                            image_base64: None,
                            type_descriptor: "Optional".to_string(),
                            files: vec![FomodFileEntry {
                                source: "options/ranch/0.4".to_string(),
                                destination: "Pal/Binaries/Win64/ue4ss/Mods/PalSchema/mods".to_string(),
                                priority: 0,
                                is_folder: true,
                            }],
                            condition_flags: vec![],
                        },
                        FomodPlugin {
                            id: "2".to_string(),
                            name: "0.5x (recommended)".to_string(),
                            description: "".to_string(),
                            image: None,
                            image_base64: None,
                            type_descriptor: "Optional".to_string(),
                            files: vec![FomodFileEntry {
                                source: "options/ranch/0.5".to_string(),
                                destination: "Pal/Binaries/Win64/ue4ss/Mods/PalSchema/mods".to_string(),
                                priority: 0,
                                is_folder: true,
                            }],
                            condition_flags: vec![],
                        },
                    ],
                }],
            }],
            conditional_file_installs: vec![],
            banner_base64: None,
        };

        let mod_info = ModInfo {
            id: "test".to_string(),
            name: "BetterBaseBuilding".to_string(),
            mod_type: crate::models::ModType::PalSchema,
            nexus_mod_id: None,
            nexus_url: None,
            nexus_author: None,
            nexus_summary: None,
            nexus_picture_url: None,
            nexus_endorsements: None,
            nexus_downloads: None,
            version: "2.1".to_string(),
            install_date: "".to_string(),
            source_zip: "".to_string(),
            config_path: None,
            config_paths: None,
            config_type: None,
            enabled: true,
            game_path: mod_dir.to_string_lossy().to_string(),
            disabled_path: "".to_string(),
            pak_destination: None,
            has_enabled_txt: false,
            mods_txt_order: None,
            extra_files: vec![],
            nexus_description: None,
            nexus_version_cached: None,
            nexus_cached_at: None,
            nexus_category: None,
            nexus_tags: vec![],
            github_repo: None,
            github_version: None,
            github_cached_at: None,
            update_date: None,
            library_zip: None,
            ignored_version: None,
            nexus_file_id: None,
            ignored_keys: None,
            has_pending_update: None,
            origin_load_method: None,
            custom_notes: None,
            original_name: None,
            custom_name: None,
            fomod_choices: None,
        };

        let deduced = deduce_fomod_choices_from_disk(&temp_dir, &mod_info, &config, &zip_path);
        assert_eq!(deduced.get("Ranch"), Some(&vec!["0.5x (recommended)".to_string()]));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
