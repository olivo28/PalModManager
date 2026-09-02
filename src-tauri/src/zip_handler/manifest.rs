use std::path::Path;
use super::analysis::analyze_zip;
use super::extraction::read_archive_file;
use super::naming::{detect_folder_name_from_files, is_forbidden, PALSCHEMA_FOLDERS};

pub fn build_manifest_from_files(
    files: &[String],
    filename: &str,
    game_path: &Path,
    pak_destination: Option<&str>,
    custom_display_name: Option<String>,
    modinfo_data: Option<serde_json::Value>,
) -> Result<crate::models::InstallManifest, String> {
    use crate::models::{InstallManifest, FileRoute, RouteType, ModType};

    let mut custom_routes_map = std::collections::HashMap::new();
    if let Some(ref modinfo) = modinfo_data {
        if let Some(routes_arr) = modinfo.get("routes").and_then(|r| r.as_array()) {
            for route_val in routes_arr {
                if let (Some(zip_path), Some(route_type_str)) = (
                    route_val.get("zipPath").and_then(|z| z.as_str()),
                    route_val.get("routeType").and_then(|t| t.as_str()),
                ) {
                    let rtype = match route_type_str.to_lowercase().as_str() {
                        "ue4ss" => RouteType::Ue4ss,
                        "palschema" => RouteType::PalSchema,
                        "pak" => RouteType::Pak,
                        "logicmods" => RouteType::LogicMods,
                        "passthrough" => RouteType::Passthrough,
                        _ => continue,
                    };
                    custom_routes_map.insert(zip_path.replace('\\', "/").to_lowercase(), rtype);
                }
            }
        }
    }

    let parsed_nexus = crate::nexus::parse_mod_filename(filename);
    let binaries_dir = crate::dependency_checker::get_binaries_dir(game_path);
    let is_xbox = binaries_dir.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("wingdk".to_string());

    // 1. Detect if the ZIP contains both steam and xbox tags
    let has_steam_tags = files.iter().any(|f| {
        let fl = f.to_lowercase().replace('\\', "/");
        fl.contains("/(steam)/") || fl.starts_with("(steam)/") ||
        fl.contains("/steam/") || fl.starts_with("steam/") ||
        fl.contains("/win64/") || fl.starts_with("win64/")
    });
    let has_xbox_tags = files.iter().any(|f| {
        let fl = f.to_lowercase().replace('\\', "/");
        fl.contains("/(xbox)/") || fl.starts_with("(xbox)/") ||
        fl.contains("/xbox/") || fl.starts_with("xbox/") ||
        fl.contains("/(gdk)/") || fl.starts_with("(gdk)/") ||
        fl.contains("/gdk/") || fl.starts_with("gdk/") ||
        fl.contains("/wingdk/") || fl.starts_with("wingdk/")
    });
    let has_both_platforms = has_steam_tags && has_xbox_tags;

    let mut folder_name = detect_folder_name_from_files(files, filename);
    let is_forbidden_folder = is_forbidden(&folder_name);
    let is_uuid = folder_name.len() >= 32 && folder_name.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if folder_name.is_empty() || folder_name == "unknown" || folder_name.starts_with("nexus_") || is_uuid || is_forbidden_folder {
        if let Some(ref disp) = custom_display_name {
            let cleaned = crate::installer::clean_zip_name(disp);
            if !cleaned.is_empty() && cleaned != "unknown" && !is_forbidden(&cleaned) {
                folder_name = cleaned;
            }
        }
        if folder_name.is_empty() || is_forbidden(&folder_name) {
            if let Some(ref n_name) = parsed_nexus.name {
                if !n_name.is_empty() && !is_forbidden(n_name) {
                    folder_name = n_name.clone();
                }
            }
        }
        if folder_name.is_empty() || is_forbidden(&folder_name) {
            let fallback = crate::installer::clean_zip_name(filename);
            if !fallback.is_empty() && !is_forbidden(&fallback) {
                folder_name = fallback;
            }
        }
    }
    let folder_name_lower = folder_name.to_lowercase();

    // Collect all distinct UE4SS mod root directory names from the files list
    let mut detected_ue4ss_roots: Vec<String> = Vec::new();
    for file in files {
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        for (i, segment) in segments.iter().enumerate() {
            let lower = segment.to_lowercase();
            if (lower == "scripts" || lower == "dlls" || lower == "enabled.txt") && i > 0 {
                let candidate = segments[i - 1];
                if !is_forbidden(candidate) && !detected_ue4ss_roots.iter().any(|r| r.eq_ignore_ascii_case(candidate)) {
                    detected_ue4ss_roots.push(candidate.to_string());
                }
            }
        }
    }

    let mut routes = Vec::new();
    let mut has_ue4ss = false;
    let mut has_palschema = false;
    let mut has_pak = false;

    // First pass: classify files and build relative paths
    let mut temp_routes = Vec::new();
    for file in files {
        let normalized = file.replace('\\', "/");
        let lower = normalized.to_lowercase();

        // Skip directory-only paths and folders (must contain a file extension to be mapped as a file)
        if normalized.ends_with('/') || normalized.split('/').last().map(|s| !s.contains('.')).unwrap_or(true) {
            continue;
        }

        // Handle the new Workshop route wrapping (Mods/NativeMods/UE4SS/Mods/)
        let normalized_clean = if lower.contains("mods/nativemods/ue4ss/mods/") {
            let idx = lower.find("mods/nativemods/ue4ss/mods/").unwrap();
            normalized[idx + "mods/nativemods/ue4ss/mods/".len()..].to_string()
        } else {
            normalized.clone()
        };
        let lower_clean = normalized_clean.to_lowercase();
        let segments: Vec<&str> = normalized_clean.split('/').filter(|s| !s.is_empty()).collect();

        // Skip inactive platform wrapper files
        if has_both_platforms {
            let is_inactive = if is_xbox {
                segments.iter().any(|&s| {
                    let sl = s.to_lowercase();
                    sl == "(steam)" || sl == "steam" || sl == "win64"
                })
            } else {
                segments.iter().any(|&s| {
                    let sl = s.to_lowercase();
                    sl == "(xbox)" || sl == "xbox" || sl == "(gdk)" || sl == "gdk" || sl == "wingdk"
                })
            };
            if is_inactive {
                continue;
            }
        }

        // Find the folder_name marker.
        // It must be an ancestor root wrapper, NOT an internal child folder inside "scripts", "dlls", "palschema", etc.
        let folder_idx = segments.iter().position(|s| s.to_lowercase() == folder_name_lower);
        let is_valid_root_folder = if let Some(idx) = folder_idx {
            let prior_segments = &segments[..idx];
            let is_inside_subfolder = prior_segments.iter().any(|s| {
                let sl = s.to_lowercase();
                sl == "scripts" || sl == "dlls" || sl == "mods" || sl == "palschema" || PALSCHEMA_FOLDERS.contains(&sl.as_str())
            });
            !is_inside_subfolder
        } else {
            false
        };

        let mut relative_path = if is_valid_root_folder {
            let idx = folder_idx.unwrap();
            segments[idx + 1..].join("/")
        } else {
            // Strip any platform wrapper if present in the first segment
            let parts: Vec<&str> = normalized_clean.split('/').collect();
            if parts.len() > 1 {
                let first_lower = parts[0].to_lowercase();
                let is_wrapper = first_lower == "(steam)" || first_lower == "steam" || first_lower == "win64" ||
                                 first_lower == "(xbox)" || first_lower == "xbox" || first_lower == "(gdk)" || first_lower == "gdk" || first_lower == "wingdk" ||
                                 first_lower.contains("mods folder") || first_lower.contains("mod folder") ||
                                 first_lower.contains("ue4ss mods") || first_lower.contains("palschema mods") ||
                                 first_lower.contains("mods directory") || first_lower.contains("mod directory");
                if is_wrapper {
                    parts[1..].join("/")
                } else {
                    normalized_clean.clone()
                }
            } else {
                normalized_clean.clone()
            }
        };

        // If this is a PalSchema folder path, we must extract relative path starting AFTER the mods/ segment to isolate the schema subdirectory
        if lower_clean.contains("palschema/mods/") {
            if let Some(pos) = lower_clean.find("palschema/mods/") {
                let schema_subpath = &normalized_clean[pos + "palschema/mods/".len()..];
                relative_path = schema_subpath.to_string();
            }
        } else if lower_clean.contains("ue4ss/mods/") {
            if let Some(pos) = lower_clean.find("ue4ss/mods/") {
                let ue4ss_subpath = &normalized_clean[pos + "ue4ss/mods/".len()..];
                let sub_lower = ue4ss_subpath.to_lowercase();
                if sub_lower.starts_with(&format!("{}/", folder_name_lower)) {
                    relative_path = ue4ss_subpath[folder_name_lower.len() + 1..].to_string();
                } else {
                    relative_path = ue4ss_subpath.to_string();
                }
            }
        } else {
            let rel_lower_check = relative_path.to_lowercase();
            if rel_lower_check.starts_with("ue4ss/mods/") {
                relative_path = relative_path["ue4ss/mods/".len()..].to_string();
                let sub_lower = relative_path.to_lowercase();
                if sub_lower.starts_with(&format!("{}/", folder_name_lower)) {
                    relative_path = relative_path[folder_name_lower.len() + 1..].to_string();
                }
            } else if rel_lower_check.starts_with("mods/") {
                relative_path = relative_path["mods/".len()..].to_string();
                let sub_lower = relative_path.to_lowercase();
                if sub_lower.starts_with(&format!("{}/", folder_name_lower)) {
                    relative_path = relative_path[folder_name_lower.len() + 1..].to_string();
                }
            } else if rel_lower_check.starts_with("ue4ss/") {
                relative_path = relative_path["ue4ss/".len()..].to_string();
            } else if rel_lower_check.starts_with("palschema/mods/") {
                relative_path = relative_path["palschema/mods/".len()..].to_string();
            } else if rel_lower_check.starts_with("palschema/") {
                relative_path = relative_path["palschema/".len()..].to_string();
            }
        }

        let rel_lower = relative_path.to_lowercase();
        let rel_segments: Vec<&str> = rel_lower.split('/').collect();

        // Classify RouteType
        let route_type = if let Some(rtype) = custom_routes_map.get(&lower)
            .or_else(|| custom_routes_map.get(&normalized.to_lowercase()))
            .or_else(|| custom_routes_map.get(&rel_lower))
        {
            match rtype {
                RouteType::Ue4ss => has_ue4ss = true,
                RouteType::PalSchema => has_palschema = true,
                RouteType::Pak => has_pak = true,
                RouteType::LogicMods => has_pak = true,
                _ => {}
            }
            rtype.clone()
        } else if rel_lower.ends_with(".pak") || rel_lower.ends_with(".ucas") || rel_lower.ends_with(".utoc") {
            // Check if any ancestor is a PalSchema folder
            let is_standard_game_path = rel_segments.contains(&"content") || rel_segments.contains(&"~mods") || rel_segments.contains(&"logicmods");
            let is_palschema_pak = if is_standard_game_path {
                false
            } else {
                rel_lower.contains("palschema/") || rel_segments.iter().any(|seg| PALSCHEMA_FOLDERS.contains(seg))
            };

            if is_palschema_pak {
                has_palschema = true;
                RouteType::PalSchema
            } else {
                has_pak = true;
                if rel_lower.contains("logicmods") || pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
                    RouteType::LogicMods
                } else {
                    RouteType::Pak
                }
            }
        } else if rel_lower.ends_with(".lua") || rel_lower.ends_with(".dll") {
            let is_ue4ss_code = rel_segments.contains(&"scripts") || rel_segments.contains(&"dlls");
            if is_ue4ss_code {
                has_ue4ss = true;
                RouteType::Ue4ss
            } else {
                has_ue4ss = true;
                RouteType::Ue4ss
            }
        } else {
            let is_in_scripts_or_dlls = rel_segments.contains(&"scripts")
                || rel_segments.contains(&"dlls")
                || lower.contains("/scripts/")
                || lower.contains("/dlls/");

            // Check if this file belongs to PalSchema (palschema folder or loader data category)
            let mut is_palschema_asset = false;
            let is_standard_game_path = rel_segments.contains(&"content") || rel_segments.contains(&"~mods") || rel_segments.contains(&"logicmods");
            if !is_standard_game_path && !is_in_scripts_or_dlls {
                if lower.contains("palschema") || rel_lower.contains("palschema") {
                    is_palschema_asset = true;
                } else {
                    for seg in &rel_segments {
                        if PALSCHEMA_FOLDERS.contains(seg) {
                            is_palschema_asset = true;
                            break;
                        }
                    }
                }
            }

            if is_palschema_asset {
                has_palschema = true;
                RouteType::PalSchema
            } else {
                // Check if this file belongs to a UE4SS mod (inside scripts, dlls, or a detected UE4SS root)
                let is_inside_ue4ss = is_in_scripts_or_dlls
                    || lower.contains("ue4ss/mods/")
                    || lower.contains("nativemods/ue4ss")
                    || rel_lower.ends_with("enabled.txt")
                    || rel_segments.iter().any(|seg| detected_ue4ss_roots.iter().any(|r| r.eq_ignore_ascii_case(seg)));

                if is_inside_ue4ss {
                    has_ue4ss = true;
                    RouteType::Ue4ss
                } else {
                    RouteType::Passthrough
                }
            }
        };

        temp_routes.push((file.clone(), relative_path, route_type));
    }

    // Determine primary destination type
    let primary_route_type = if has_ue4ss {
        RouteType::Ue4ss
    } else if has_palschema {
        RouteType::PalSchema
    } else if has_pak {
        if pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
            RouteType::LogicMods
        } else {
            RouteType::Pak
        }
    } else {
        RouteType::Passthrough
    };

    // Second pass: build absolute dest_path
    let paks_dest_dir = game_path.join("Pal").join("Content").join("Paks").join("~mods");
    let logicmods_dest_dir = game_path.join("Pal").join("Content").join("Paks").join("LogicMods");
    let ue4ss_mods_dest = crate::dependency_checker::get_ue4ss_mods_dir(game_path);
    let palschema_mods_dest = ue4ss_mods_dest.join("PalSchema").join("mods");

    for (zip_path, relative_path, route_type) in temp_routes {
        if relative_path.is_empty() {
            continue;
        }
        let file_name_opt = Path::new(&relative_path).file_name();
        if file_name_opt.is_none() {
            continue;
        }
        let filename = file_name_opt.unwrap();
        let rel_lower = relative_path.to_lowercase();
        let binaries_name = binaries_dir.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| "win64".to_string());
        
        let norm_zip = zip_path.replace('\\', "/");
        let zip_lower = norm_zip.to_lowercase();

        let game_subpath = if let Some(idx) = zip_lower.find("pal/content/paks/") {
            Some(norm_zip[idx..].to_string())
        } else if let Some(idx) = zip_lower.find("pal/binaries/") {
            if route_type == RouteType::Ue4ss || route_type == RouteType::PalSchema || zip_lower.contains("ue4ss/mods/") || zip_lower.contains("mods/nativemods/ue4ss/mods/") {
                None
            } else {
                let sub = &norm_zip[idx..];
                let sub_lower = sub.to_lowercase();
                // Adapt win64/wingdk to target system
                if sub_lower.starts_with("pal/binaries/win64/") && binaries_name == "wingdk" {
                    Some(format!("Pal/Binaries/WinGDK/{}", &sub["pal/binaries/win64/".len()..]))
                } else if sub_lower.starts_with("pal/binaries/wingdk/") && binaries_name == "win64" {
                    Some(format!("Pal/Binaries/Win64/{}", &sub["pal/binaries/wingdk/".len()..]))
                } else {
                    Some(sub.to_string())
                }
            }
        } else if let Some(idx) = rel_lower.find("mods/nativemods/ue4ss/mods/") {
            Some(relative_path[idx..].to_string())
        } else {
            None
        };

        let dest_path = if let Some(ref subpath) = game_subpath {
            game_path.join(subpath)
        } else {
            match route_type {
                RouteType::Ue4ss => {
                    let norm_file = zip_path.replace('\\', "/");
                    let norm_parts: Vec<&str> = norm_file.split('/').filter(|s| !s.is_empty()).collect();

                    let mut matched_root = None;
                    let mut matched_subpath = None;

                    for root in &detected_ue4ss_roots {
                        let root_lower = root.to_lowercase();
                        if let Some(pos) = norm_parts.iter().position(|p| p.to_lowercase() == root_lower) {
                            matched_root = Some(root.clone());
                            matched_subpath = Some(norm_parts[pos + 1..].join("/"));
                            break;
                        }
                    }

                    if let (Some(root), Some(subpath)) = (matched_root, matched_subpath) {
                        let final_sub = if subpath.to_lowercase().ends_with(".lua") && !subpath.contains('/') {
                            format!("Scripts/{}", subpath)
                        } else {
                            subpath
                        };
                        ue4ss_mods_dest.join(&root).join(final_sub)
                    } else {
                        let final_rel = if rel_lower.ends_with(".lua") && !relative_path.contains('/') {
                            format!("Scripts/{}", relative_path)
                        } else {
                            relative_path.clone()
                        };
                        ue4ss_mods_dest.join(&folder_name).join(final_rel)
                    }
                }
                RouteType::PalSchema => {
                    let rel_segments: Vec<&str> = relative_path.split('/').collect();
                    let first_seg_lower = rel_segments.first().map(|s| s.to_lowercase()).unwrap_or_default();
                    
                    // If the relative path starts directly with a PalSchema loader folder (e.g. items/, raw/, pals/), 
                    // it needs the mod folder prepended.
                    // Otherwise, the first segment is ALREADY the mod's specific PalSchema folder (e.g. 000_PassiveTraitExtraction/)!
                    if PALSCHEMA_FOLDERS.contains(&first_seg_lower.as_str()) {
                        palschema_mods_dest.join(&folder_name).join(&relative_path)
                    } else {
                        palschema_mods_dest.join(&relative_path)
                    }
                }
                RouteType::Pak => {
                    paks_dest_dir.join(filename)
                }
                RouteType::LogicMods => {
                    logicmods_dest_dir.join(filename)
                }
                RouteType::Companion => {
                    let target_dir = if pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
                        &logicmods_dest_dir
                    } else {
                        &paks_dest_dir
                    };
                    target_dir.join(filename)
                }
                RouteType::Passthrough => {
                    let rel_lower = relative_path.to_lowercase();
                    if rel_lower.contains("swapjson") {
                        paks_dest_dir.join("SwapJSON").join(filename)
                    } else if rel_lower.contains("alterconfig") {
                        paks_dest_dir.join("AlterConfig").join(filename)
                    } else if rel_lower.contains("json_templates") || rel_lower.contains("jsontemplates") {
                        paks_dest_dir.join("JSON_Templates").join(filename)
                    } else {
                        match primary_route_type {
                            RouteType::Ue4ss => {
                                let norm_file = zip_path.replace('\\', "/");
                                let norm_parts: Vec<&str> = norm_file.split('/').filter(|s| !s.is_empty()).collect();

                                let mut matched_root = None;
                                let mut matched_subpath = None;

                                for root in &detected_ue4ss_roots {
                                    let root_lower = root.to_lowercase();
                                    if let Some(pos) = norm_parts.iter().position(|p| p.to_lowercase() == root_lower) {
                                        matched_root = Some(root.clone());
                                        matched_subpath = Some(norm_parts[pos + 1..].join("/"));
                                        break;
                                    }
                                }

                                if let (Some(root), Some(subpath)) = (matched_root, matched_subpath) {
                                    ue4ss_mods_dest.join(&root).join(subpath)
                                } else {
                                    ue4ss_mods_dest.join(&folder_name).join(&relative_path)
                                }
                            }
                            RouteType::PalSchema => palschema_mods_dest.join(&folder_name).join(&relative_path),
                            RouteType::Pak | RouteType::LogicMods | RouteType::Companion | RouteType::Passthrough => {
                                let target_dir = if primary_route_type == RouteType::LogicMods {
                                    &logicmods_dest_dir
                                } else {
                                    &paks_dest_dir
                                };
                                target_dir.join(filename)
                            }
                        }
                    }
                }
            }
        };

        let is_doc_or_image = {
            let fl = rel_lower.as_str();
            (fl.ends_with(".txt") && !fl.ends_with("enabled.txt") && !fl.ends_with("mod.txt") && !fl.ends_with("info.txt"))
                || fl.ends_with(".md")
                || fl.ends_with(".url")
                || fl.ends_with(".png")
                || fl.ends_with(".jpg")
                || fl.ends_with(".jpeg")
                || fl.ends_with(".gif")
                || fl.ends_with(".pdf")
        };

        if is_doc_or_image && (route_type == RouteType::Pak || route_type == RouteType::LogicMods || route_type == RouteType::Passthrough) {
            continue;
        }

        let dest_str = dest_path.to_string_lossy().to_string();
        #[cfg(windows)]
        let dest_str = dest_str.replace('/', "\\");
        #[cfg(not(windows))]
        let dest_str = dest_str.replace('\\', "/");

        routes.push(FileRoute {
            zip_path,
            dest_path: dest_str,
            route_type,
        });
    }

    let has_altermatic = files.iter().any(|f| {
        let fl = f.to_lowercase();
        fl.contains("swapjson")
            || fl.contains("alterconfig")
            || fl.ends_with(".swap.json")
            || (fl.ends_with(".json") && (fl.contains("skelmesh") || fl.contains("matreplace") || fl.contains("altermatic")))
    });

    // Derive global mod type
    let mod_type = if has_altermatic {
        ModType::Altermatic
    } else {
        match (has_ue4ss, has_palschema, has_pak) {
            (true, false, false) => ModType::Ue4ss,
            (false, true, false) => ModType::PalSchema,
            (false, false, true) => {
                if pak_destination.map(|d| d.to_lowercase() == "logicmods").unwrap_or(false) {
                    ModType::LogicMods
                } else {
                    ModType::Pak
                }
            }
            _ => ModType::Hybrid,
        }
    };

    let mut display_name = custom_display_name
        .filter(|n| !n.trim().is_empty() && !n.starts_with("Mod #") && !n.starts_with("nexus_") && n != "unknown")
        .or(parsed_nexus.name)
        .unwrap_or_else(|| crate::installer::clean_zip_name(filename));

    let mut version = parsed_nexus.version.unwrap_or_else(|| "1.0".to_string());
    let mut nexus_mod_id = parsed_nexus.nexus_id;

    if let Some(ref modinfo) = modinfo_data {
        if let Some(n) = modinfo.get("name").and_then(|n| n.as_str()) {
            display_name = n.to_string();
        }
        if let Some(v) = modinfo.get("version").and_then(|v| v.as_str()) {
            version = v.to_string();
        } else if let Some(v) = modinfo.get("version").and_then(|v| v.as_f64()) {
            version = v.to_string();
        }
        if let Some(id) = modinfo.get("nexusModId").and_then(|id| id.as_u64()) {
            nexus_mod_id = Some(id as u32);
        }
    }

    Ok(InstallManifest {
        folder_name,
        display_name,
        mod_type,
        routes,
        nexus_mod_id,
        nexus_file_id: parsed_nexus.nexus_file_id,
        has_pak,
        has_ue4ss,
        has_palschema,
        version,
    })
}

pub fn build_install_manifest(
    zip_path: &str,
    game_path: &Path,
    pak_destination: Option<&str>,
    custom_display_name: Option<String>,
) -> Result<crate::models::InstallManifest, String> {
    let analysis = analyze_zip(zip_path)?;
    let filename = Path::new(zip_path).file_name().unwrap().to_string_lossy().to_string();
    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            if let Some(content) = read_archive_file(zip_path, target_file) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    modinfo_data = Some(val);
                }
            }
        }
    }
    build_manifest_from_files(&analysis.files, &filename, game_path, pak_destination, custom_display_name, modinfo_data)
}
