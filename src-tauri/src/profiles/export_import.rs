use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::{FileOptions, ZipWriter};
use zip::read::ZipArchive;

use crate::models::{AppData, ModFolder, ModInfo, ModType, Profile, DependencyMode};
use crate::profiles::utils::{get_profile_dir, sanitize_profile_id};
use crate::commands::dependency_commands::get_vault_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedModMeta {
    pub id: String,
    pub name: String,
    pub mod_type: String,
    pub version: String,
    pub enabled: bool,
    pub nexus_mod_id: Option<u32>,
    pub nexus_url: Option<String>,
    pub nexus_author: Option<String>,
    pub nexus_summary: Option<String>,
    pub nexus_picture_url: Option<String>,
    pub config_path: Option<String>,
    pub config_type: Option<String>,
    pub mods_txt_order: Option<u32>,
    pub extra_files: Vec<String>,
    pub origin_load_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedDependencyMeta {
    pub dep_type: String, // "UE4SS" or "PalSchema"
    pub version: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePackManifest {
    pub format_version: String,
    pub pmm_version: String,
    pub exported_at: String,
    pub profile_name: String,
    pub profile_id: String,
    pub ue4ss_enabled: bool,
    pub palschema_enabled: bool,
    pub dependency_mode: String,
    pub force_load_order_ue4ss: Option<bool>,
    pub force_load_order_palschema: Option<bool>,
    pub hide_native_mods: Option<bool>,
    pub mod_folders: Vec<ModFolder>,
    pub load_order_metadata: Option<Vec<(String, bool)>>,
    pub dependencies: Vec<ExportedDependencyMeta>,
    pub mods: Vec<ExportedModMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfileResult {
    pub success: bool,
    pub profile_id: String,
    pub profile_name: String,
    pub mod_count: usize,
    pub dependencies_installed: usize,
}

fn bundle_dependency_from_vault(
    program_path: &str,
    dep_key: &str,
    dep_display: &str,
    current_ver: &str,
    zip: &mut ZipWriter<File>,
    options: FileOptions<()>,
    exported_dependencies: &mut Vec<ExportedDependencyMeta>,
) {
    let vault_dir = get_vault_dir(program_path, dep_key);
    if !vault_dir.exists() {
        return;
    }

    let Ok(rd) = fs::read_dir(&vault_dir) else { return; };
    let mut best_zip: Option<PathBuf> = None;

    for entry in rd.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() && p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("zip")) {
            let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if !current_ver.is_empty() && fname.contains(current_ver) {
                best_zip = Some(p);
                break;
            }
            if best_zip.is_none() {
                best_zip = Some(p);
            }
        }
    }

    if let Some(zip_p) = best_zip {
        let fname = zip_p.file_name().unwrap_or_default().to_string_lossy().to_string();
        let zip_entry_name = format!("dependencies/{}/{}", dep_display, fname);
        let _ = zip.start_file(&zip_entry_name, options);
        if let Ok(mut f) = File::open(&zip_p) {
            let mut buf = Vec::new();
            if f.read_to_end(&mut buf).is_ok() {
                let _ = zip.write_all(&buf);
                exported_dependencies.push(ExportedDependencyMeta {
                    dep_type: dep_display.to_string(),
                    version: current_ver.to_string(),
                    filename: fname,
                });
                crate::logger::log(&format!("export_profile_pack: Bundled {} installer '{}'", dep_display, zip_entry_name));
            }
        }
    }
}

/// Exports a complete, autonomous Co-op Profile Pack into a .zip file
pub fn export_profile_pack_internal(
    data: &AppData,
    profile_id: &str,
    target_zip_path: &str,
) -> Result<String, String> {
    let program_path = &data.settings.program_path;
    let game_path = &data.settings.game_path;

    if program_path.is_empty() {
        return Err("Program path is not configured".to_string());
    }

    let profile = data.profiles.iter().find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))?;

    crate::logger::log(&format!("export_profile_pack: Starting export for profile '{}' ({})", profile.name, profile.id));

    let target_path = PathBuf::from(target_zip_path);
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create destination directory: {}", e))?;
    }

    let zip_file = File::create(&target_path)
        .map_err(|e| format!("Failed to create target zip file: {}", e))?;
    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);

    let mut exported_dependencies = Vec::new();
    let mut exported_mods = Vec::new();

    // 1. Export Dependencies from Local Vault if available
    let dep_status = crate::dependency_checker::check_dependencies(game_path);

    if profile.ue4ss_enabled || profile.dependency_mode == DependencyMode::Standard {
        let current_ver = dep_status.ue4ss_version.unwrap_or_default();
        bundle_dependency_from_vault(program_path, "ue4ss", "UE4SS", &current_ver, &mut zip, options, &mut exported_dependencies);
    }

    if profile.palschema_enabled {
        let current_ver = dep_status.palschema_version.unwrap_or_default();
        bundle_dependency_from_vault(program_path, "palschema", "PalSchema", &current_ver, &mut zip, options, &mut exported_dependencies);
    }

    // 2. Export Profile Mods and Custom Configurations
    // Filter mods assigned to profile (or all user mods if profile has no specific list)
    let profile_mods: Vec<&ModInfo> = data.mods.iter().filter(|m| {
        if m.nexus_author.as_deref() == Some("UE4SS Native Mod") {
            return false;
        }
        if !profile.installed_mod_ids.is_empty() {
            profile.installed_mod_ids.iter().any(|id| id.eq_ignore_ascii_case(&m.id) || id.eq_ignore_ascii_case(&m.name))
        } else {
            true
        }
    }).collect();

    for m in profile_mods {
        let is_enabled = profile.enabled_mod_ids.iter().any(|id| id.eq_ignore_ascii_case(&m.id) || id.eq_ignore_ascii_case(&m.name));

        let mod_type_str = match m.mod_type {
            ModType::Ue4ss => "ue4ss",
            ModType::PalSchema => "palschema",
            ModType::Pak => "pak",
            ModType::LogicMods => "logicmods",
            ModType::Hybrid => "hybrid",
            ModType::Altermatic => "altermatic",
        };

        exported_mods.push(ExportedModMeta {
            id: m.id.clone(),
            name: m.name.clone(),
            mod_type: mod_type_str.to_string(),
            version: m.version.clone(),
            enabled: is_enabled,
            nexus_mod_id: m.nexus_mod_id,
            nexus_url: m.nexus_url.clone(),
            nexus_author: m.nexus_author.clone(),
            nexus_summary: m.nexus_summary.clone(),
            nexus_picture_url: m.nexus_picture_url.clone(),
            config_path: m.config_path.clone(),
            config_type: m.config_type.clone(),
            mods_txt_order: m.mods_txt_order,
            extra_files: m.extra_files.clone(),
            origin_load_method: m.origin_load_method.clone(),
        });

        // Determine source path on disk
        let src_path = if !m.game_path.is_empty() && Path::new(&m.game_path).exists() {
            PathBuf::from(&m.game_path)
        } else if !m.disabled_path.is_empty() && Path::new(&m.disabled_path).exists() {
            PathBuf::from(&m.disabled_path)
        } else {
            PathBuf::new()
        };

        if src_path.exists() {
            let cat_folder = match m.mod_type {
                ModType::Ue4ss | ModType::Hybrid => "mods/UE4SS",
                ModType::PalSchema => "mods/PalSchema",
                ModType::Pak => "mods/Paks",
                ModType::LogicMods => "mods/LogicMods",
                ModType::Altermatic => "mods/Altermatic",
            };

            let mut paths_to_pack = vec![(src_path.clone(), cat_folder.to_string())];
            for extra in &m.extra_files {
                let ep = PathBuf::from(extra);
                if ep.exists() {
                    paths_to_pack.push((ep, cat_folder.to_string()));
                }
            }

            for (p, cat) in paths_to_pack {
                if p.is_dir() {
                    let parent_dir = p.parent().unwrap_or(&p);
                    for entry in walkdir::WalkDir::new(&p).into_iter().flatten() {
                        let item_path = entry.path();
                        if item_path.is_file() {
                            if let Ok(rel) = item_path.strip_prefix(parent_dir) {
                                let rel_str = rel.to_string_lossy().replace('\\', "/");
                                let zip_entry_name = format!("{}/{}", cat, rel_str);
                                let _ = zip.start_file(&zip_entry_name, options);
                                if let Ok(mut f) = File::open(item_path) {
                                    let mut buf = Vec::new();
                                    if f.read_to_end(&mut buf).is_ok() {
                                        let _ = zip.write_all(&buf);
                                    }
                                }
                            }
                        }
                    }
                } else if p.is_file() {
                    let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let zip_entry_name = format!("{}/{}", cat, fname);
                    let _ = zip.start_file(&zip_entry_name, options);
                    if let Ok(mut f) = File::open(&p) {
                        let mut buf = Vec::new();
                        if f.read_to_end(&mut buf).is_ok() {
                            let _ = zip.write_all(&buf);
                        }
                    }

                    // Pack companion files (.utoc, .ucas, .sig) if pak
                    if p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak")) {
                        let companions = crate::zip_handler::find_pak_companions(&p);
                        for comp in companions {
                            if comp != p {
                                let comp_name = comp.file_name().unwrap_or_default().to_string_lossy().to_string();
                                let comp_entry_name = format!("{}/{}", cat, comp_name);
                                let _ = zip.start_file(&comp_entry_name, options);
                                if let Ok(mut f) = File::open(&comp) {
                                    let mut buf = Vec::new();
                                    if f.read_to_end(&mut buf).is_ok() {
                                        let _ = zip.write_all(&buf);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Write profile_manifest.json into root of zip
    let manifest = ProfilePackManifest {
        format_version: "1.0.0".to_string(),
        pmm_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        profile_name: profile.name.clone(),
        profile_id: profile.id.clone(),
        ue4ss_enabled: profile.ue4ss_enabled,
        palschema_enabled: profile.palschema_enabled,
        dependency_mode: match profile.dependency_mode {
            DependencyMode::Workshop => "workshop".to_string(),
            DependencyMode::Standard => "standard".to_string(),
            DependencyMode::None => "none".to_string(),
        },
        force_load_order_ue4ss: profile.force_load_order_ue4ss,
        force_load_order_palschema: profile.force_load_order_palschema,
        hide_native_mods: profile.hide_native_mods,
        mod_folders: profile.mod_folders.clone(),
        load_order_metadata: profile.load_order_metadata.clone(),
        dependencies: exported_dependencies,
        mods: exported_mods,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize profile manifest: {}", e))?;

    let _ = zip.start_file("profile_manifest.json", options);
    let _ = zip.write_all(manifest_json.as_bytes());

    zip.finish().map_err(|e| format!("Failed to finalize export zip archive: {}", e))?;

    crate::logger::log(&format!("export_profile_pack: Successfully created co-op pack at '{}'", target_path.display()));
    Ok(target_path.to_string_lossy().to_string())
}

/// Imports a Co-op Profile Pack zip into the local PalModManager setup
pub fn import_profile_pack_internal(
    data: &mut AppData,
    source_zip_path: &str,
    custom_profile_name: Option<String>,
) -> Result<ImportProfileResult, String> {
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();

    if program_path.is_empty() || game_path.is_empty() {
        return Err("Program path or Game path is not configured".to_string());
    }

    let zip_file = File::open(source_zip_path)
        .map_err(|e| format!("Failed to open profile pack zip: {}", e))?;
    let mut archive = ZipArchive::new(zip_file)
        .map_err(|e| format!("Invalid zip archive: {}", e))?;

    // 1. Read and parse profile_manifest.json
    let manifest_content = {
        let mut manifest_file = archive.by_name("profile_manifest.json")
            .map_err(|_| "Invalid profile pack: missing 'profile_manifest.json' in root".to_string())?;
        let mut text = String::new();
        manifest_file.read_to_string(&mut text)
            .map_err(|e| format!("Failed to read profile manifest: {}", e))?;
        text
    };

    let manifest: ProfilePackManifest = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("Failed to parse profile manifest JSON: {}", e))?;

    let final_profile_name = custom_profile_name
        .unwrap_or_else(|| manifest.profile_name.clone());

    // Generate unique profile id
    let mut base_id = sanitize_profile_id(&final_profile_name);
    if base_id.is_empty() {
        base_id = "coop_profile".to_string();
    }
    let mut unique_id = base_id.clone();
    let mut counter = 1;
    while data.profiles.iter().any(|p| p.id == unique_id) {
        unique_id = format!("{}_{}", base_id, counter);
        counter += 1;
    }

    crate::logger::log(&format!("import_profile_pack: Importing profile pack '{}' as ID '{}'", final_profile_name, unique_id));

    // 2. Import Dependency Installers into Local Dependency Vault
    let mut deps_installed_count = 0;
    let total_entries = archive.len();

    for i in 0..total_entries {
        let (file_name, is_file) = {
            let item = archive.by_index(i).map_err(|e| e.to_string())?;
            (item.name().to_string(), item.is_file())
        };

        if is_file && file_name.starts_with("dependencies/") {
            let mut dep_file = archive.by_index(i).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            if dep_file.read_to_end(&mut buf).is_ok() {
                if file_name.starts_with("dependencies/UE4SS/") {
                    let fname = Path::new(&file_name).file_name().unwrap_or_default().to_string_lossy().to_string();
                    let target_vault_file = get_vault_dir(&program_path, "ue4ss").join(&fname);
                    let _ = fs::create_dir_all(target_vault_file.parent().unwrap());
                    let _ = fs::write(&target_vault_file, &buf);
                    deps_installed_count += 1;
                    crate::logger::log(&format!("import_profile_pack: Registered UE4SS installer '{}' in local vault", fname));
                } else if file_name.starts_with("dependencies/PalSchema/") {
                    let fname = Path::new(&file_name).file_name().unwrap_or_default().to_string_lossy().to_string();
                    let target_vault_file = get_vault_dir(&program_path, "palschema").join(&fname);
                    let _ = fs::create_dir_all(target_vault_file.parent().unwrap());
                    let _ = fs::write(&target_vault_file, &buf);
                    deps_installed_count += 1;
                    crate::logger::log(&format!("import_profile_pack: Registered PalSchema installer '{}' in local vault", fname));
                }
            }
        }
    }

    // 3. Extract Mods into Isolated Profile & Game Structure
    let profile_dir = get_profile_dir(&program_path, &unique_id);
    fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("Failed to create profile directory: {}", e))?;

    let game_p = Path::new(&game_path);
    let profile_checker = crate::dependency_checker::build_game_profile(game_p);

    for i in 0..total_entries {
        let (file_name, is_file) = {
            let item = archive.by_index(i).map_err(|e| e.to_string())?;
            (item.name().to_string(), item.is_file())
        };

        if is_file && file_name.starts_with("mods/") {
            let mut item_file = archive.by_index(i).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            if item_file.read_to_end(&mut buf).is_ok() {
                let clean_sub = &file_name[5..]; // Strip "mods/"
                let target_dest: Option<PathBuf> = if clean_sub.starts_with("UE4SS/") {
                    Some(profile_checker.ue4ss_mods_dir.join(&clean_sub[6..]))
                } else if clean_sub.starts_with("PalSchema/") {
                    Some(profile_checker.palschema_mods_dir.join(&clean_sub[10..]))
                } else if clean_sub.starts_with("Paks/") {
                    Some(profile_checker.paks_dir.join(&clean_sub[5..]))
                } else if clean_sub.starts_with("LogicMods/") {
                    Some(profile_checker.logic_mods_dir.join(&clean_sub[10..]))
                } else {
                    None
                };

                if let Some(dest_p) = target_dest {
                    if let Some(parent) = dest_p.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&dest_p, &buf);
                }
            }
        }
    }

    // 4. Register or Update ModInfo in AppData
    let mut installed_mod_ids = Vec::new();
    let mut enabled_mod_ids = Vec::new();

    for m_meta in &manifest.mods {
        installed_mod_ids.push(m_meta.id.clone());
        if m_meta.enabled {
            enabled_mod_ids.push(m_meta.id.clone());
        }

        let m_type = match m_meta.mod_type.as_str() {
            "ue4ss" => ModType::Ue4ss,
            "palschema" => ModType::PalSchema,
            "logicmods" => ModType::LogicMods,
            "hybrid" => ModType::Hybrid,
            _ => ModType::Pak,
        };

        if let Some(existing) = data.mods.iter_mut().find(|m| m.id.eq_ignore_ascii_case(&m_meta.id) || m.name.eq_ignore_ascii_case(&m_meta.name)) {
            existing.version = m_meta.version.clone();
            existing.enabled = m_meta.enabled;
            if m_meta.nexus_mod_id.is_some() {
                existing.nexus_mod_id = m_meta.nexus_mod_id;
            }
            if m_meta.config_path.is_some() {
                existing.config_path = m_meta.config_path.clone();
                existing.config_type = m_meta.config_type.clone();
            }
        } else {
            data.mods.push(ModInfo {
                id: m_meta.id.clone(),
                name: m_meta.name.clone(),
                mod_type: m_type,
                nexus_mod_id: m_meta.nexus_mod_id,
                nexus_url: m_meta.nexus_url.clone(),
                nexus_author: m_meta.nexus_author.clone(),
                nexus_summary: m_meta.nexus_summary.clone(),
                nexus_picture_url: m_meta.nexus_picture_url.clone(),
                nexus_endorsements: None,
                nexus_downloads: None,
                version: m_meta.version.clone(),
                install_date: chrono::Utc::now().to_rfc3339(),
                source_zip: "coop_profile_pack.zip".to_string(),
                config_path: m_meta.config_path.clone(),
                config_type: m_meta.config_type.clone(),
                enabled: m_meta.enabled,
                game_path: String::new(),
                disabled_path: String::new(),
                pak_destination: None,
                has_enabled_txt: false,
                mods_txt_order: m_meta.mods_txt_order,
                extra_files: m_meta.extra_files.clone(),
                nexus_description: None,
                nexus_version_cached: None,
                nexus_cached_at: None,
                nexus_category: None,
                nexus_tags: Vec::new(),
                github_repo: None,
                github_version: None,
                github_cached_at: None,
                update_date: None,
                library_zip: None,
                ignored_version: None,
                nexus_file_id: None,
                ignored_keys: None,
                has_pending_update: None,
                origin_load_method: m_meta.origin_load_method.clone(),
                custom_notes: None,
            });
        }
    }

    // 5. Create Profile Record and add to AppData
    let dep_mode = match manifest.dependency_mode.as_str() {
        "workshop" => DependencyMode::Workshop,
        "standard" => DependencyMode::Standard,
        _ => DependencyMode::None,
    };

    let new_profile = Profile {
        id: unique_id.clone(),
        name: final_profile_name.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        installed_mod_ids: installed_mod_ids.clone(),
        enabled_mod_ids: enabled_mod_ids.clone(),
        ue4ss_enabled: manifest.ue4ss_enabled,
        palschema_enabled: manifest.palschema_enabled,
        dependency_mode: dep_mode,
        mod_folders: manifest.mod_folders.clone(),
        load_order_metadata: manifest.load_order_metadata.clone(),
        force_load_order_ue4ss: manifest.force_load_order_ue4ss,
        force_load_order_palschema: manifest.force_load_order_palschema,
        hide_native_mods: manifest.hide_native_mods,
        ue4ss_version: None,
        palschema_version: None,
        altermatic_version: None,
        unipalui_version: None,
        compatibility_patches: None,
        ue4ss_control_mode: Some("enabled_txt".to_string()),
    };

    if let Ok(json) = serde_json::to_string_pretty(&new_profile) {
        let _ = fs::write(profile_dir.join("profile.json"), json);
    }

    // 6. Switch to newly imported profile immediately
    let target_profile_clone = new_profile.clone();
    data.profiles.push(new_profile);
    let _ = crate::profiles::switch_profile(data, &program_path, &target_profile_clone);

    crate::logger::log(&format!("import_profile_pack: Successfully imported profile '{}' with {} mods", final_profile_name, installed_mod_ids.len()));

    Ok(ImportProfileResult {
        success: true,
        profile_id: unique_id,
        profile_name: final_profile_name,
        mod_count: installed_mod_ids.len(),
        dependencies_installed: deps_installed_count,
    })
}
