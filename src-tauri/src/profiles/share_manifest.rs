use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{AppData, ModFolder, ModInfo, ModType, Profile, DependencyMode};
use crate::profiles::diff_engine::{compute_json_delta, apply_json_delta};
use crate::profiles::sanitize_profile_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileShareModConfig {
    pub rel_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_delta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileShareMod {
    pub id: String,
    pub name: String,
    pub mod_type: String,
    pub version: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nexus_mod_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nexus_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nexus_author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nexus_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nexus_picture_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mods_txt_order: Option<u32>,
    pub configs: Vec<ProfileShareModConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileShareManifest {
    pub format_version: String, // "1.0.0"
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
    pub required_dependencies: Vec<String>,
    pub mods: Vec<ProfileShareMod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingModInfo {
    pub id: String,
    pub name: String,
    pub mod_type: String,
    pub version: String,
    pub nexus_mod_id: Option<u32>,
    pub nexus_url: Option<String>,
    pub nexus_author: Option<String>,
    pub nexus_picture_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeProfileManifestResult {
    pub profile_name: String,
    pub profile_id: String,
    pub total_mods: usize,
    pub installed_count: usize,
    pub missing_mods: Vec<MissingModInfo>,
    pub required_dependencies: Vec<String>,
    pub ue4ss_enabled: bool,
    pub palschema_enabled: bool,
}

/// Scans a mod on disk and extracts genuine user configurations or JSON deltas.
/// Enforces strict heuristic filters to prevent capturing logic code.
pub fn extract_mod_customizations(m: &ModInfo) -> Vec<ProfileShareModConfig> {
    let mut configs = Vec::new();
    let mut processed_paths = std::collections::HashSet::new();

    // 1. If mod is a directory mod (UE4SS or PalSchema), scan files inside its own folder only
    let target_dir = if !m.game_path.is_empty() {
        let p = Path::new(&m.game_path);
        if p.is_dir() { Some(p.to_path_buf()) } else { None }
    } else if !m.disabled_path.is_empty() {
        let p = Path::new(&m.disabled_path);
        if p.is_dir() { Some(p.to_path_buf()) } else { None }
    } else {
        None
    };

    let is_palschema = m.mod_type == ModType::PalSchema;

    if let Some(ref dir) = target_dir {
        if dir.exists() {
            for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                let fname = path.file_name().unwrap_or_default().to_string_lossy();
                let fname_lower = fname.to_lowercase();
                let path_str_lower = path.to_string_lossy().to_lowercase().replace('\\', "/");

                // Skip system files, binaries, manifests, schemas, and .bak files themselves
                if fname_lower.starts_with('.')
                    || fname_lower.ends_with(".bak")
                    || fname_lower.ends_with(".bak1")
                    || fname_lower.ends_with(".bak2")
                    || fname_lower == "main.lua"
                    || fname_lower == "enabled.txt"
                    || fname_lower.ends_with(".pmm.json")
                    || fname_lower.ends_with(".manifest.json")
                    || fname_lower.ends_with(".pak")
                    || fname_lower.ends_with(".dll")
                    || fname_lower.ends_with(".exe")
                    || fname_lower.ends_with("manager.lua")
                    || fname_lower.ends_with("handler.lua")
                    || fname_lower.ends_with("helper.lua")
                    || fname_lower.ends_with("service.lua")
                    || path_str_lower.contains("do_not_edit")
                    || path_str_lower.contains("palschema/schemas")
                    || path_str_lower.contains("blueprints")
                    || path_str_lower.contains("tables")
                {
                    continue;
                }

                let rel_path = match path.strip_prefix(dir) {
                    Ok(r) => r.to_string_lossy().replace('\\', "/"),
                    Err(_) => continue,
                };

                let bak_path = path.with_extension(format!("{}.bak", path.extension().unwrap_or_default().to_string_lossy()));
                let has_bak = bak_path.exists() && bak_path.is_file();

                // PalSchema: ONLY capture if user edited the file (has .bak)
                if is_palschema {
                    if has_bak {
                        if let (Ok(orig_text), Ok(mod_text)) = (fs::read_to_string(&bak_path), fs::read_to_string(path)) {
                            let orig_json = serde_json::from_str::<Value>(&orig_text);
                            let mod_json = serde_json::from_str::<Value>(&mod_text);
                            if let (Ok(orig_v), Ok(mod_v)) = (orig_json, mod_json) {
                                if let Some(delta) = compute_json_delta(&orig_v, &mod_v) {
                                    configs.push(ProfileShareModConfig {
                                        rel_path,
                                        content: None,
                                        json_delta: Some(delta),
                                    });
                                }
                            } else {
                                configs.push(ProfileShareModConfig {
                                    rel_path,
                                    content: Some(mod_text),
                                    json_delta: None,
                                });
                            }
                        }
                    }
                    processed_paths.insert(path_str_lower);
                    continue;
                }

                // Non-PalSchema (UE4SS / INI):
                let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                let is_explicit_config = has_bak
                    || ext == "ini"
                    || ext == "cfg"
                    || path_str_lower.contains("/shared/")
                    || path_str_lower.contains("/config/")
                    || fname_lower == "config.lua"
                    || fname_lower == "settings.lua"
                    || fname_lower == "config.json"
                    || fname_lower == "settings.json";

                if is_explicit_config {
                    if let Ok(mod_text) = fs::read_to_string(path) {
                        configs.push(ProfileShareModConfig {
                            rel_path,
                            content: Some(mod_text),
                            json_delta: None,
                        });
                    }
                }
                processed_paths.insert(path_str_lower);
            }
        }
    }

    // 2. Also check specifically declared config_paths if any exist outside mod dir
    if let Some(ref cp_list) = m.config_paths {
        for cp in cp_list {
            let p = Path::new(cp);
            let p_lower = p.to_string_lossy().to_lowercase().replace('\\', "/");
            if p.is_file() && !processed_paths.contains(&p_lower) {
                let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                if let Ok(content) = fs::read_to_string(p) {
                    configs.push(ProfileShareModConfig {
                        rel_path: fname,
                        content: Some(content),
                        json_delta: None,
                    });
                    processed_paths.insert(p_lower);
                }
            }
        }
    }

    configs
}

/// Exports a clean, metadata-only `.pmmprofile` manifest. 100% Nexus TOS compliant.
pub fn export_profile_manifest_internal(
    data: &AppData,
    profile_id: &str,
    target_file_path: &str,
) -> Result<String, String> {
    let profile = data.profiles.iter().find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))?;

    crate::logger::log(&format!("export_profile_manifest: Exporting shareable manifest for '{}'", profile.name));

    let mut required_deps = Vec::new();
    if profile.ue4ss_enabled {
        required_deps.push("UE4SS".to_string());
    }
    if profile.palschema_enabled {
        required_deps.push("PalSchema".to_string());
    }

    let profile_mods: Vec<&ModInfo> = data.mods.iter().filter(|m| {
        if m.nexus_author.as_deref() == Some("UE4SS Native Mod") {
            return false;
        }
        profile.installed_mod_ids.iter().any(|entry| {
            crate::profiles::mod_matches_profile_entry(m, entry)
        })
    }).collect();

    let mut mods = Vec::new();
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

        let configs = extract_mod_customizations(m);

        mods.push(ProfileShareMod {
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
            mods_txt_order: m.mods_txt_order,
            configs,
        });
    }

    let manifest = ProfileShareManifest {
        format_version: "1.0.0".to_string(),
        pmm_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        profile_name: profile.name.clone(),
        profile_id: profile.id.clone(),
        ue4ss_enabled: profile.ue4ss_enabled,
        palschema_enabled: profile.palschema_enabled,
        dependency_mode: match profile.dependency_mode {
            DependencyMode::Standard => "Standard".to_string(),
            DependencyMode::Workshop => "Workshop".to_string(),
            DependencyMode::None => "None".to_string(),
        },
        force_load_order_ue4ss: profile.force_load_order_ue4ss,
        force_load_order_palschema: profile.force_load_order_palschema,
        hide_native_mods: profile.hide_native_mods,
        mod_folders: profile.mod_folders.clone(),
        load_order_metadata: profile.load_order_metadata.clone(),
        required_dependencies: required_deps,
        mods,
    };

    let json_str = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize profile manifest: {}", e))?;

    let path = Path::new(target_file_path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(path, json_str)
        .map_err(|e| format!("Failed to write profile manifest file: {}", e))?;

    crate::logger::log(&format!("export_profile_manifest: Successfully saved to '{}'", target_file_path));
    Ok(target_file_path.to_string())
}

/// Reads a `.pmmprofile` manifest and compares its declarations against local mods.
pub fn analyze_profile_manifest_internal(
    manifest_path: &str,
    data: &AppData,
) -> Result<AnalyzeProfileManifestResult, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|e| format!("Failed to read manifest file: {}", e))?;

    let manifest: ProfileShareManifest = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid profile manifest format: {}", e))?;

    let mut installed_count = 0;
    let mut missing_mods = Vec::new();

    for m in &manifest.mods {
        let is_installed = data.mods.iter().any(|local| {
            if let (Some(req_nexus), Some(local_nexus)) = (m.nexus_mod_id, local.nexus_mod_id) {
                if req_nexus == local_nexus {
                    return true;
                }
            }
            local.name.eq_ignore_ascii_case(&m.name)
        });

        if is_installed {
            installed_count += 1;
        } else {
            missing_mods.push(MissingModInfo {
                id: m.id.clone(),
                name: m.name.clone(),
                mod_type: m.mod_type.clone(),
                version: m.version.clone(),
                nexus_mod_id: m.nexus_mod_id,
                nexus_url: m.nexus_url.clone(),
                nexus_author: m.nexus_author.clone(),
                nexus_picture_url: m.nexus_picture_url.clone(),
            });
        }
    }

    Ok(AnalyzeProfileManifestResult {
        profile_name: manifest.profile_name,
        profile_id: manifest.profile_id,
        total_mods: manifest.mods.len(),
        installed_count,
        missing_mods,
        required_dependencies: manifest.required_dependencies,
        ue4ss_enabled: manifest.ue4ss_enabled,
        palschema_enabled: manifest.palschema_enabled,
    })
}

/// Applies user custom configs and JSON deltas from a `.pmmprofile` onto installed mods.
pub fn apply_profile_customizations_internal(
    manifest_path: &str,
    data: &mut AppData,
    target_mod_id: Option<&str>,
) -> Result<usize, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|e| format!("Failed to read manifest file: {}", e))?;

    let manifest: ProfileShareManifest = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid profile manifest format: {}", e))?;

    let mut applied_count = 0;

    for share_mod in &manifest.mods {
        if let Some(filter_id) = target_mod_id {
            if !share_mod.id.eq_ignore_ascii_case(filter_id) && !share_mod.name.eq_ignore_ascii_case(filter_id) {
                continue;
            }
        }

        let local_mod = data.mods.iter().find(|m| {
            if let (Some(req_n), Some(loc_n)) = (share_mod.nexus_mod_id, m.nexus_mod_id) {
                if req_n == loc_n { return true; }
            }
            m.name.eq_ignore_ascii_case(&share_mod.name)
        });

        let Some(mod_info) = local_mod else { continue; };

        let mod_dir = if !mod_info.game_path.is_empty() {
            let p = Path::new(&mod_info.game_path);
            if p.is_dir() { p.to_path_buf() } else { p.parent().unwrap_or(p).to_path_buf() }
        } else if !mod_info.disabled_path.is_empty() {
            let p = Path::new(&mod_info.disabled_path);
            if p.is_dir() { p.to_path_buf() } else { p.parent().unwrap_or(p).to_path_buf() }
        } else {
            continue;
        };

        if !mod_dir.exists() {
            continue;
        }

        for cfg in &share_mod.configs {
            let target_file = mod_dir.join(&cfg.rel_path);

            // 1. JSON Delta application
            if let Some(ref delta) = cfg.json_delta {
                if target_file.exists() {
                    if let Ok(orig_content) = fs::read_to_string(&target_file) {
                        if let Ok(mut base_json) = serde_json::from_str::<Value>(&orig_content) {
                            if apply_json_delta(&mut base_json, delta).is_ok() {
                                if let Ok(new_json_str) = serde_json::to_string_pretty(&base_json) {
                                    let _ = fs::write(&target_file, new_json_str);
                                    applied_count += 1;
                                    crate::logger::log(&format!("apply_profile_customizations: Applied JSON delta to {:?}", target_file));
                                }
                            }
                        }
                    }
                }
            } else if let Some(ref raw_content) = cfg.content {
                // 2. Raw text config application
                if let Some(parent) = target_file.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::write(&target_file, raw_content).is_ok() {
                    applied_count += 1;
                    crate::logger::log(&format!("apply_profile_customizations: Wrote config to {:?}", target_file));
                }
            }
        }
    }

    Ok(applied_count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProfileManifestResult {
    pub success: bool,
    pub profile_id: String,
    pub profile_name: String,
    pub matched_mods: usize,
    pub customizations_applied: usize,
}

/// Applies a `.pmmprofile` by creating the profile entry, linking local mods, and applying configs.
pub fn apply_profile_manifest_internal(
    manifest_path: &str,
    data: &mut AppData,
    custom_name: Option<String>,
) -> Result<ApplyProfileManifestResult, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|e| format!("Failed to read manifest file: {}", e))?;

    let manifest: ProfileShareManifest = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid profile manifest format: {}", e))?;

    let target_name = custom_name.unwrap_or(manifest.profile_name);
    let mut profile_id = sanitize_profile_id(&target_name);

    // Ensure unique ID if a profile with the same name already exists
    let mut counter = 1;
    let base_id = profile_id.clone();
    while data.profiles.iter().any(|p| p.id == profile_id) {
        profile_id = format!("{}_{}", base_id, counter);
        counter += 1;
    }

    let mut installed_mod_ids = Vec::new();
    let mut enabled_mod_ids = Vec::new();

    for share_mod in &manifest.mods {
        let local_mod = data.mods.iter_mut().find(|m| {
            if let (Some(req_n), Some(loc_n)) = (share_mod.nexus_mod_id, m.nexus_mod_id) {
                if req_n == loc_n { return true; }
            }
            m.name.eq_ignore_ascii_case(&share_mod.name)
        });

        if let Some(m) = local_mod {
            installed_mod_ids.push(m.id.clone());
            if share_mod.enabled {
                enabled_mod_ids.push(m.id.clone());
            }
            if let Some(ord) = share_mod.mods_txt_order {
                m.mods_txt_order = Some(ord);
            }
        }
    }

    let new_profile = Profile {
        id: profile_id.clone(),
        name: target_name.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        installed_mod_ids: installed_mod_ids.clone(),
        enabled_mod_ids,
        ue4ss_enabled: manifest.ue4ss_enabled,
        palschema_enabled: manifest.palschema_enabled,
        dependency_mode: match manifest.dependency_mode.to_lowercase().as_str() {
            "workshop" => DependencyMode::Workshop,
            "standard" => DependencyMode::Standard,
            _ => DependencyMode::None,
        },
        force_load_order_ue4ss: manifest.force_load_order_ue4ss,
        force_load_order_palschema: manifest.force_load_order_palschema,
        hide_native_mods: manifest.hide_native_mods,
        mod_folders: manifest.mod_folders,
        load_order_metadata: manifest.load_order_metadata,
        ue4ss_version: None,
        palschema_version: None,
        altermatic_version: None,
        unipalui_version: None,
        compatibility_patches: None,
        ue4ss_control_mode: None,
    };

    data.profiles.push(new_profile);

    // Apply custom configs
    let customizations_applied = apply_profile_customizations_internal(manifest_path, data, None)?;

    crate::logger::log(&format!(
        "apply_profile_manifest: Created profile '{}' ({}) with {} matched mods and {} customizations applied",
        target_name, profile_id, installed_mod_ids.len(), customizations_applied
    ));

    Ok(ApplyProfileManifestResult {
        success: true,
        profile_id,
        profile_name: target_name,
        matched_mods: installed_mod_ids.len(),
        customizations_applied,
    })
}
