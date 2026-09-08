use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::models::{FileRoute, RouteType, ModType};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkshopInstallRule {
    #[serde(rename = "Type")]
    pub rule_type: String,
    #[serde(default)]
    pub is_server: bool,
    #[serde(default)]
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkshopInfoJson {
    #[serde(default)]
    pub mod_name: String,
    #[serde(default)]
    pub package_name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    #[serde(default)]
    pub install_rule: Option<Vec<WorkshopInstallRule>>,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub struct WorkshopResolvedRouting {
    pub folder_name: String,
    pub display_name: String,
    pub version: String,
    pub mod_type: ModType,
    pub routes: Vec<FileRoute>,
}

pub fn try_resolve_workshop_routing(
    info_json_val: &serde_json::Value,
    archive_files: &[String],
    game_path: &Path,
) -> Option<WorkshopResolvedRouting> {
    let info: WorkshopInfoJson = serde_json::from_value(info_json_val.clone()).ok()?;
    let install_rules = info.install_rule.as_ref()?;
    if install_rules.is_empty() {
        return None;
    }

    let package_name = if info.package_name.is_empty() { "WorkshopMod" } else { &info.package_name };
    let display_name = if info.mod_name.is_empty() { package_name.to_string() } else { info.mod_name.clone() };
    let version = if info.version.is_empty() { "1.0".to_string() } else { info.version.clone() };

    let win64 = crate::dependency_checker::get_binaries_dir(game_path);
    let std_ue4ss = win64.join("ue4ss");
    let paks_dir = game_path.join("Pal").join("Content").join("Paks").join("~mods");

    // 1. Detect root prefix in archive (e.g. "3761921027/" if Info.json is located inside a subfolder)
    let root_prefix = archive_files.iter()
        .find(|f| {
            let fl = f.to_lowercase().replace('\\', "/");
            fl == "info.json" || fl.ends_with("/info.json")
        })
        .and_then(|f| {
            let norm = f.replace('\\', "/");
            norm.rfind('/').map(|idx| norm[..=idx].to_string())
        })
        .unwrap_or_default();

    // 2. Collect and sort client rules by target specificity (longest clean target first, catch-all last)
    let mut client_rules: Vec<(&str, String)> = Vec::new();
    for rule in install_rules {
        if rule.is_server {
            // Client game installs skip dedicated server rules
            continue;
        }
        for target in &rule.targets {
            let clean = target.trim_start_matches('.').trim_start_matches('/').trim_start_matches('\\').to_string();
            client_rules.push((rule.rule_type.as_str(), clean));
        }
    }

    client_rules.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    let mut routes = Vec::new();
    let mut handled_files = std::collections::HashSet::new();
    let mut has_lua = false;
    let mut has_palschema = false;
    let mut has_pak = false;
    let mut has_ue4ss = false;

    for (rule_type_raw, clean_target) in client_rules {
        let rule_type = rule_type_raw.to_lowercase();
        let clean_target_trimmed = clean_target.trim_matches('/').trim_matches('\\');
        let clean_target_lower = clean_target_trimmed.to_lowercase();

        for file in archive_files {
            if handled_files.contains(file) {
                continue;
            }

            let file_norm = file.replace('\\', "/");
            let file_rel = if !root_prefix.is_empty() && file_norm.starts_with(&root_prefix) {
                &file_norm[root_prefix.len()..]
            } else {
                &file_norm
            };

            let file_clean = file_rel.trim_start_matches('.').trim_start_matches('/').trim_start_matches('\\');
            let file_lower = file_clean.to_lowercase();

            // Skip directory entries ending with slash
            if file.ends_with('/') || file.ends_with('\\') || file_clean.is_empty() {
                continue;
            }

            // Skip thumbnail, manifest Info.json, and internal metadata
            if file_lower == "info.json"
                || file_lower.ends_with("/info.json")
                || file_lower == "thumbnail.png"
                || file_lower == "thumbnail.jpg"
                || file_lower == "preview.png"
                || file_lower == "preview.jpg"
                || file_lower == ".workshop.json"
                || file_lower.ends_with("/.workshop.json")
            {
                continue;
            }

            // Check if file matches this rule target
            let is_match = if clean_target_lower.is_empty() {
                true
            } else {
                file_lower.starts_with(&clean_target_lower)
            };

            if !is_match {
                continue;
            }

            // Determine sub-path relative to the target
            let sub_path = if clean_target_trimmed.is_empty() {
                file_clean
            } else if file_clean.len() > clean_target_trimmed.len() {
                let rem = &file_clean[clean_target_trimmed.len()..];
                rem.trim_start_matches('/').trim_start_matches('\\')
            } else {
                Path::new(file_clean).file_name().and_then(|n| n.to_str()).unwrap_or(file_clean)
            };

            match rule_type.as_str() {
                "lua" => {
                    has_lua = true;
                    has_ue4ss = true;
                    // Destination: ue4ss/Mods/<package_name>/<sub_path>
                    let dest_rel = if clean_target_lower.starts_with("scripts") || file_lower.starts_with("scripts") {
                        PathBuf::from("Scripts").join(sub_path)
                    } else {
                        PathBuf::from(sub_path)
                    };
                    let dest_full = std_ue4ss.join("Mods").join(package_name).join(dest_rel);
                    routes.push(FileRoute {
                        zip_path: file.clone(),
                        dest_path: dest_full.to_string_lossy().to_string(),
                        route_type: RouteType::Ue4ss,
                    });
                    handled_files.insert(file.clone());
                }
                "palschema" => {
                    has_palschema = true;
                    // Destination: ue4ss/Mods/PalSchema/mods/<package_name>/<sub_path>
                    let dest_full = std_ue4ss.join("Mods").join("PalSchema").join("mods").join(package_name).join(sub_path);
                    routes.push(FileRoute {
                        zip_path: file.clone(),
                        dest_path: dest_full.to_string_lossy().to_string(),
                        route_type: RouteType::PalSchema,
                    });
                    handled_files.insert(file.clone());
                }
                "paks" | "pak" => {
                    has_pak = true;
                    let pak_name = Path::new(file_clean).file_name().and_then(|n| n.to_str()).unwrap_or(file_clean);
                    let dest_full = paks_dir.join(pak_name);
                    routes.push(FileRoute {
                        zip_path: file.clone(),
                        dest_path: dest_full.to_string_lossy().to_string(),
                        route_type: RouteType::Pak,
                    });
                    handled_files.insert(file.clone());
                }
                "ue4ss" => {
                    has_ue4ss = true;
                    // If target is ./UE4SS/Mods, sub_path contains the inner mod directory
                    let dest_full = if clean_target_lower.contains("ue4ss/mods") {
                        std_ue4ss.join("Mods").join(sub_path)
                    } else {
                        std_ue4ss.join("Mods").join(package_name).join(sub_path)
                    };
                    routes.push(FileRoute {
                        zip_path: file.clone(),
                        dest_path: dest_full.to_string_lossy().to_string(),
                        route_type: RouteType::Ue4ss,
                    });
                    handled_files.insert(file.clone());
                }
                _ => {}
            }
        }
    }

    if routes.is_empty() {
        return None;
    }

    let mod_type = if (has_ue4ss || has_lua || has_palschema) && has_pak {
        ModType::Hybrid
    } else if has_palschema {
        ModType::PalSchema
    } else if has_ue4ss || has_lua {
        ModType::Ue4ss
    } else {
        ModType::Pak
    };

    Some(WorkshopResolvedRouting {
        folder_name: package_name.to_string(),
        display_name,
        version,
        mod_type,
        routes,
    })
}

pub fn try_build_workshop_manifest(
    info_json_val: &serde_json::Value,
    archive_files: &[String],
    filename: &str,
    game_path: &Path,
    custom_display_name: Option<&str>,
) -> Option<crate::models::InstallManifest> {
    let resolved = try_resolve_workshop_routing(info_json_val, archive_files, game_path)?;
    let parsed_nexus = crate::nexus::parse_mod_filename(filename);
    let has_pak = resolved.routes.iter().any(|r| r.route_type == RouteType::Pak || r.route_type == RouteType::LogicMods);
    let has_ue4ss = resolved.routes.iter().any(|r| r.route_type == RouteType::Ue4ss);
    let has_palschema = resolved.routes.iter().any(|r| r.route_type == RouteType::PalSchema);

    Some(crate::models::InstallManifest {
        folder_name: resolved.folder_name,
        display_name: custom_display_name.map(|s| s.to_string()).unwrap_or(resolved.display_name),
        mod_type: resolved.mod_type,
        routes: resolved.routes,
        nexus_mod_id: None,
        nexus_file_id: parsed_nexus.nexus_file_id,
        has_pak,
        has_ue4ss,
        has_palschema,
        version: resolved.version,
        author: None,
        summary: None,
        picture_url: None,
    })
}

fn collect_relative_files_recursively(base: &Path, current: &Path, acc: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_relative_files_recursively(base, &p, acc);
            } else if let Ok(rel) = p.strip_prefix(base) {
                acc.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

pub fn activate_workshop_from_rules(
    src_dir: &Path,
    game_path: &Path,
    info: &WorkshopInfoJson,
    force_load_order_ue4ss: bool,
    installed_files: &mut Vec<String>,
    installed_dirs: &mut Vec<String>,
) -> Result<bool, String> {
    let _ = match info.install_rule.as_ref() {
        Some(r) if !r.is_empty() => r,
        _ => return Ok(false),
    };

    let mut relative_files = Vec::new();
    collect_relative_files_recursively(src_dir, src_dir, &mut relative_files);
    if relative_files.is_empty() {
        return Ok(false);
    }

    let info_val = serde_json::to_value(info).map_err(|e| e.to_string())?;
    let resolved = match try_resolve_workshop_routing(&info_val, &relative_files, game_path) {
        Some(r) => r,
        None => return Ok(false),
    };

    let gp = crate::dependency_checker::build_game_profile(game_path);
    let package_name = if info.package_name.is_empty() { "WorkshopMod" } else { &info.package_name };

    // Snapshot config files for Lua / PalSchema if destinations exist
    let dest_lua_dir = gp.ue4ss_mods_dir.join(package_name);
    let dest_palschema_dir = gp.palschema_mods_dir.join(package_name);
    let lua_snapshot = if dest_lua_dir.exists() {
        Some(crate::config_merge::snapshot_configs(&dest_lua_dir, None))
    } else {
        None
    };
    let palschema_snapshot = if dest_palschema_dir.exists() {
        Some(crate::config_merge::snapshot_configs(&dest_palschema_dir, None))
    } else {
        None
    };

    for route in &resolved.routes {
        let src_file = src_dir.join(&route.zip_path);
        let dest_file = PathBuf::from(&route.dest_path);

        if let Some(parent) = dest_file.parent() {
            let _ = std::fs::create_dir_all(parent);
            if let Some(rel_dir) = pathdiff::diff_paths(parent, game_path) {
                let dir_str = rel_dir.to_string_lossy().replace('\\', "/");
                if !dir_str.is_empty() && !installed_dirs.contains(&dir_str) {
                    installed_dirs.push(dir_str);
                }
            }
        }

        std::fs::copy(&src_file, &dest_file)
            .map_err(|e| format!("Failed to copy file {}: {}", route.zip_path, e))?;

        if let Some(rel_file) = pathdiff::diff_paths(&dest_file, game_path) {
            let file_str = rel_file.to_string_lossy().replace('\\', "/");
            if !file_str.is_empty() && !installed_files.contains(&file_str) {
                installed_files.push(file_str);
            }
        }
    }

    // Apply config merge restorations
    if let Some(snap) = lua_snapshot {
        crate::config_merge::apply_config_merge(&dest_lua_dir, &snap, &[]);
    }
    if let Some(snap) = palschema_snapshot {
        crate::config_merge::apply_config_merge(&dest_palschema_dir, &snap, &[]);
    }

    // Handle UE4SS load order if mod has Lua routes
    let has_lua_routes = resolved.routes.iter().any(|r| r.route_type == RouteType::Ue4ss);
    if has_lua_routes {
        if force_load_order_ue4ss {
            let mods_txt = gp.mods_txt_path.clone();
            if mods_txt.exists() {
                let _ = crate::profiles::update_mods_txt_load_order(&mods_txt, package_name, true);
            }
            let enabled_txt = dest_lua_dir.join("enabled.txt");
            if enabled_txt.exists() {
                let _ = std::fs::remove_file(&enabled_txt);
            }
        } else {
            let _ = std::fs::write(dest_lua_dir.join("enabled.txt"), "");
            let mods_txt = gp.mods_txt_path.clone();
            if mods_txt.exists() {
                let _ = crate::profiles::remove_from_mods_txt(&mods_txt, package_name);
            }
        }
    }

    Ok(true)
}


