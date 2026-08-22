use std::fs;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

fn sanitize_folder_name(name: &str) -> String {
    let mut cleaned = name.replace(|c: char| {
        c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|'
    }, "");
    cleaned = cleaned.trim().to_string();
    cleaned
}

fn clean_zip_name(zip_name: &str) -> String {
    let clean = zip_name
        .trim_end_matches(".zip")
        .trim_end_matches(".rar")
        .trim_end_matches(".7z");
    let stem = sanitize_folder_name(clean);
    let words: Vec<&str> = stem.split_whitespace().collect();
    let mut clean_words = Vec::new();
    for word in &words {
        if word.chars().all(|c| c.is_ascii_digit()) {
            break;
        }
        let starts_with_digit = word.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
        if starts_with_digit && word.contains('-') && word.len() > 6 {
            break;
        }
        let word_lower = word.to_lowercase();
        if ["gamepass", "steam", "gdk", "xbox"].contains(&word_lower.as_str()) {
            continue;
        }
        clean_words.push(*word);
    }
    let result = clean_words.join(" ").trim_end_matches(|c: char| c == '(' || c == ' ').trim().to_string();
    if result.len() < 2 { stem } else { result }
}

fn find_palschema_root(json_path: &Path, extracted: &Path) -> PathBuf {
    for ancestor in json_path.ancestors() {
        if let Some(name) = ancestor.file_name().map(|n| n.to_string_lossy().to_lowercase()) {
            if ["pals", "enums", "translations", "raw"].contains(&name.as_str()) {
                if let Some(parent) = ancestor.parent() {
                    if parent != extracted {
                        return parent.to_path_buf();
                    }
                }
            }
        }
    }
    
    let mut current = json_path.parent().unwrap();
    while let Some(parent) = current.parent() {
        if parent == extracted {
            break;
        }
        let parent_name = parent.file_name().unwrap().to_string_lossy().to_lowercase();
        if ["palschema mods folder", "mods", "ue4ss"].contains(&parent_name.as_str()) {
            break;
        }
        current = parent;
    }
    current.to_path_buf()
}

fn extract_archive(archive_path: &Path, temp_extracted: &Path) -> Result<(), String> {
    let lower_path = archive_path.to_string_lossy().to_lowercase();
    if lower_path.ends_with(".rar") || lower_path.ends_with(".7z") {
        println!("Extracting using tar command line...");
        let mut cmd = std::process::Command::new("tar");
        cmd.args(&["-xf", &archive_path.to_string_lossy(), "-C", &temp_extracted.to_string_lossy()]);
        let output = cmd.output().map_err(|e| format!("Failed to execute tar: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("tar command failed: {}", err));
        }
        Ok(())
    } else {
        println!("Extracting using zip-rs crate... (from {})", archive_path.display());
        let file = fs::File::open(archive_path).map_err(|e| format!("Cannot open zip: {}", e))?;
        let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| format!("Cannot read entry: {}", e))?;
            let outpath = temp_extracted.join(entry.mangled_name());
            if entry.is_dir() {
                fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                let mut outfile = fs::File::create(&outpath).unwrap();
                std::io::copy(&mut entry, &mut outfile).unwrap();
            }
        }
        Ok(())
    }
}

fn preprocess_extracted_dir(extracted: &Path, is_xbox: bool) -> Result<(), String> {
    // 1. Collect all file paths
    let mut all_files = Vec::new();
    fn collect_files(dir: &Path, list: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, list);
                } else {
                    list.push(path);
                }
            }
        }
    }
    collect_files(extracted, &mut all_files);

    // Check if the archive contains both platform-specific markers
    let has_steam_tags = all_files.iter().any(|p| {
        let rel = p.strip_prefix(extracted).unwrap().to_string_lossy().to_lowercase().replace('\\', "/");
        rel.contains("/(steam)/") || rel.starts_with("(steam)/") ||
        rel.contains("/steam/") || rel.starts_with("steam/") ||
        rel.contains("/win64/") || rel.starts_with("win64/")
    });

    let has_xbox_tags = all_files.iter().any(|p| {
        let rel = p.strip_prefix(extracted).unwrap().to_string_lossy().to_lowercase().replace('\\', "/");
        rel.contains("/(xbox)/") || rel.starts_with("(xbox)/") ||
        rel.contains("/xbox/") || rel.starts_with("xbox/") ||
        rel.contains("/(gdk)/") || rel.starts_with("(gdk)/") ||
        rel.contains("/gdk/") || rel.starts_with("gdk/") ||
        rel.contains("/wingdk/") || rel.starts_with("wingdk/")
    });

    let has_both_platforms = has_steam_tags && has_xbox_tags;
    println!("  [PLATFORM DETECT] has_steam_tags: {}, has_xbox_tags: {}, has_both: {}", has_steam_tags, has_xbox_tags, has_both_platforms);

    // 2. Identify and delete files belonging to the inactive platform (only if both are present)
    if has_both_platforms {
        for file_path in &all_files {
            if !file_path.exists() { continue; }
            let rel_path = file_path.strip_prefix(extracted).unwrap();
            let rel_lower = rel_path.to_string_lossy().to_lowercase().replace('\\', "/");
            let segments: Vec<&str> = rel_lower.split('/').collect();

            let is_inactive = if is_xbox {
                segments.iter().any(|&s| {
                    s == "(steam)" || s == "steam" || s == "win64"
                })
            } else {
                segments.iter().any(|&s| {
                    s == "(xbox)" || s == "xbox" || s == "(gdk)" || s == "gdk" || s == "wingdk"
                })
            };

            if is_inactive {
                let _ = fs::remove_file(file_path);
            }
        }
    }

    // 3. Normalize wrapper folders for the active platform.
    let mut remaining_files = Vec::new();
    collect_files(extracted, &mut remaining_files);

    for file_path in remaining_files {
        if !file_path.exists() { continue; }
        let rel_path = file_path.strip_prefix(extracted).unwrap();
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");
        let parts: Vec<&str> = rel_str.split('/').collect();
        if parts.len() > 1 {
            let first_lower = parts[0].to_lowercase();
            let is_wrapper = if has_both_platforms {
                if is_xbox {
                    first_lower == "(xbox)" || first_lower == "xbox" || first_lower == "(gdk)" || first_lower == "gdk" || first_lower == "wingdk"
                } else {
                    first_lower == "(steam)" || first_lower == "steam" || first_lower == "win64"
                }
            } else {
                first_lower == "(steam)" || first_lower == "steam" || first_lower == "win64" ||
                first_lower == "(xbox)" || first_lower == "xbox" || first_lower == "(gdk)" || first_lower == "gdk" || first_lower == "wingdk"
            };

            if is_wrapper {
                let target_rel = parts[1..].join("/");
                let target_path = extracted.join(target_rel);
                if let Some(parent) = target_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Err(_) = fs::rename(&file_path, &target_path) {
                    if fs::copy(&file_path, &target_path).is_ok() {
                        let _ = fs::remove_file(&file_path);
                    }
                }
            }
        }
    }

    // 4. Clean up any empty directories inside extracted
    fn clean_empty_dirs(dir: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            let mut subdirs = Vec::new();
            let mut has_files = false;
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    subdirs.push(entry.path());
                } else {
                    has_files = true;
                }
            }
            for subdir in subdirs {
                clean_empty_dirs(&subdir);
            }
            if !has_files {
                if let Ok(mut check) = fs::read_dir(dir) {
                    if check.next().is_none() {
                        let _ = fs::remove_dir(dir);
                    }
                }
            }
        }
    }
    clean_empty_dirs(extracted);

    Ok(())
}

fn simulate_smart_installation(
    zip_path: &Path,
    mock_game_root: &Path,
    is_xbox: bool,
    pak_destination: Option<&str>,
) -> Result<(), String> {
    let file_name = zip_path.file_name().unwrap().to_string_lossy().to_string();
    println!("\n>>> Smart routing simulation for: {} (Simulated Game: {}, Pak Dest Request: {:?})", 
             file_name, if is_xbox { "Xbox/GDK" } else { "Steam" }, pak_destination);

    // Setup extracted temp dir in mock environment
    let temp_extracted = mock_game_root.join("temp_extracted");
    let _ = fs::remove_dir_all(&temp_extracted);
    fs::create_dir_all(&temp_extracted).unwrap();

    // 1. Extract ZIP/7z to temp
    extract_archive(zip_path, &temp_extracted)?;

    // Run platform preprocessing
    preprocess_extracted_dir(&temp_extracted, is_xbox)?;

    // 2. Scan extracted files to collect components
    let mut all_files = Vec::new();
    fn collect_files(dir: &Path, list: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, list);
                } else {
                    list.push(path);
                }
            }
        }
    }
    collect_files(&temp_extracted, &mut all_files);

    // Collect and print all files after preprocessing
    println!("Files after preprocessing (count: {}):", all_files.len());
    for f in &all_files {
        println!("  - {}", f.strip_prefix(&temp_extracted).unwrap().display());
    }

    // Determine type: check if it's Hybrid vs Pak
    let mut has_pak = false;
    let mut has_lua = false;
    let mut has_dll = false;
    let mut has_json = false;
    for file_path in &all_files {
        let ext = file_path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext == "pak" { has_pak = true; }
        if ext == "lua" { has_lua = true; }
        if ext == "dll" { has_dll = true; }
        if ext == "json" || ext == "jsonc" { has_json = true; }
    }

    // 3. Find UE4SS mod roots (Lua files or DLL files)
    let mut ue4ss_roots = Vec::new();
    for file_path in &all_files {
        let ext = file_path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext == "lua" || ext == "dll" {
            let parent = file_path.parent().unwrap();
            let parent_name = parent.file_name().unwrap().to_string_lossy().to_lowercase();
            let root = if parent_name == "scripts" || parent_name == "dlls" {
                parent.parent().unwrap().to_path_buf()
            } else {
                parent.to_path_buf()
            };
            if !ue4ss_roots.contains(&root) {
                ue4ss_roots.push(root);
            }
        }
    }

    // 4. Find PalSchema mod roots
    let mut palschema_roots = Vec::new();
    for file_path in &all_files {
        let ext = file_path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext == "json" || ext == "jsonc" {
            let rel = file_path.strip_prefix(&temp_extracted).unwrap().to_string_lossy().to_lowercase().replace('\\', "/");
            let is_palschema_json = rel.contains("palschema") || 
                rel.contains("/pals/") || rel.starts_with("pals/") ||
                rel.contains("/enums/") || rel.starts_with("enums/") ||
                rel.contains("/translations/") || rel.starts_with("translations/") ||
                rel.contains("/raw/") || rel.starts_with("raw/");
            if is_palschema_json {
                let root = find_palschema_root(file_path, &temp_extracted);
                if !palschema_roots.contains(&root) {
                    palschema_roots.push(root);
                }
            }
        }
    }

    // Determine target mod folder name
    let clean_stem = clean_zip_name(&file_name);
    let detected_folder_name = if ue4ss_roots.len() == 1 {
        let name = ue4ss_roots[0].file_name().unwrap().to_string_lossy().to_string();
        if name.is_empty() || name.to_lowercase() == "temp_extracted" || name.starts_with("palmodmanager_") || ue4ss_roots[0] == temp_extracted {
            clean_stem.clone()
        } else {
            name
        }
    } else if palschema_roots.len() == 1 && ue4ss_roots.is_empty() {
        let name = palschema_roots[0].file_name().unwrap().to_string_lossy().to_string();
        if name.is_empty() || name.to_lowercase() == "temp_extracted" || name.starts_with("palmodmanager_") || palschema_roots[0] == temp_extracted {
            clean_stem.clone()
        } else {
            name
        }
    } else {
        clean_stem.clone()
    };

    let safe_folder_name = sanitize_folder_name(&detected_folder_name);
    let mod_folder_clean = safe_folder_name;

    println!("Detected Component Roots:");
    for r in &ue4ss_roots {
        let rel_display = r.strip_prefix(&temp_extracted)
            .map(|d| d.display().to_string())
            .unwrap_or_else(|_| "Root".to_string());
        println!("  - UE4SS mod root: {}", if rel_display.is_empty() { "Root" } else { &rel_display });
    }
    for r in &palschema_roots {
        let rel_display = r.strip_prefix(&temp_extracted)
            .map(|d| d.display().to_string())
            .unwrap_or_else(|_| "Root".to_string());
        println!("  - PalSchema mod root: {}", if rel_display.is_empty() { "Root" } else { &rel_display });
    }

    // Define mock directories
    let binaries_dir = if is_xbox {
        mock_game_root.join("Pal").join("Binaries").join("WinGDK")
    } else {
        mock_game_root.join("Pal").join("Binaries").join("Win64")
    };
    let paks_dir = mock_game_root.join("Pal").join("Content").join("Paks").join("~mods");
    let logicmods_dir = mock_game_root.join("Pal").join("Content").join("Paks").join("LogicMods");
    let ue4ss_mods_dest = binaries_dir.join("ue4ss").join("Mods");
    let palschema_mods_dest = ue4ss_mods_dest.join("PalSchema").join("mods");

    // Copy/Route logic:
    let is_hybrid = (has_lua || has_dll || has_json) && has_pak;

    if is_hybrid {
        println!("  [TYPE DETECTED] Hybrid");
        for file_path in &all_files {
            let rel_path = file_path.strip_prefix(&temp_extracted).unwrap();
            let rel_lower = rel_path.to_string_lossy().to_lowercase().replace('\\', "/");
            let file_basename = file_path.file_name().unwrap();

            // A. Pak files
            if rel_lower.ends_with(".pak") || rel_lower.ends_with(".ucas") || rel_lower.ends_with(".utoc") {
                let dest_dir = if let Some(dest) = pak_destination {
                    if dest.to_lowercase() == "logicmods" {
                        &logicmods_dir
                    } else {
                        &paks_dir
                    }
                } else {
                    if rel_lower.contains("logicmods") {
                        &logicmods_dir
                    } else {
                        &paks_dir
                    }
                };
                let dest = dest_dir.join(file_basename);
                println!("  [ROUTED PAK] {} -> {}", rel_path.display(), dest.display());
                continue;
            }

            // Longest prefix match routing for UE4SS and PalSchema roots
            let matching_ue4ss = ue4ss_roots.iter().filter(|r| file_path.starts_with(r)).max_by_key(|r| r.as_path().components().count());
            let matching_palschema = palschema_roots.iter().filter(|r| file_path.starts_with(r)).max_by_key(|r| r.as_path().components().count());

            match (matching_ue4ss, matching_palschema) {
                (Some(u_root), Some(p_root)) => {
                    let u_count = u_root.as_path().components().count();
                    let p_count = p_root.as_path().components().count();
                    if p_count > u_count {
                        let file_rel_to_root = file_path.strip_prefix(p_root).unwrap();
                        let dest = palschema_mods_dest.join(&mod_folder_clean).join(file_rel_to_root);
                        println!("  [ROUTED PALSCHEMA] {} -> {}", rel_path.display(), dest.display());
                    } else {
                        let file_rel_to_root = file_path.strip_prefix(u_root).unwrap();
                        let dest = ue4ss_mods_dest.join(&mod_folder_clean).join(file_rel_to_root);
                        println!("  [ROUTED UE4SS] {} -> {}", rel_path.display(), dest.display());
                    }
                }
                (None, Some(p_root)) => {
                    let file_rel_to_root = file_path.strip_prefix(p_root).unwrap();
                    let dest = palschema_mods_dest.join(&mod_folder_clean).join(file_rel_to_root);
                    println!("  [ROUTED PALSCHEMA] {} -> {}", rel_path.display(), dest.display());
                }
                (Some(u_root), None) => {
                    let file_rel_to_root = file_path.strip_prefix(u_root).unwrap();
                    let dest = ue4ss_mods_dest.join(&mod_folder_clean).join(file_rel_to_root);
                    println!("  [ROUTED UE4SS] {} -> {}", rel_path.display(), dest.display());
                }
                (None, None) => {}
            }
        }
    } else {
        println!("  [TYPE DETECTED] Pak/LogicMods Only");
        let dest_subdir = match pak_destination {
            Some(d) if d.to_lowercase() == "logicmods" => "LogicMods",
            _ => "~mods",
        };
        let final_dest_dir = mock_game_root.join("Pal").join("Content").join("Paks").join(dest_subdir);

        // Find pak files
        let mut pak_files = Vec::new();
        for file_path in &all_files {
            if file_path.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("pak".to_string()) {
                pak_files.push(file_path.clone());
            }
        }

        for pak_path in &pak_files {
            let rel_path = pak_path.strip_prefix(&temp_extracted).unwrap();
            let parent = pak_path.parent().unwrap();
            let stem = pak_path.file_stem().unwrap().to_string_lossy();
            
            // Find companions
            let mut companions = vec![pak_path.to_path_buf()];
            for ext in &["ucas", "utoc"] {
                let companion = parent.join(format!("{}.{}", stem, ext));
                if companion.exists() {
                    companions.push(companion);
                }
            }

            for companion in companions {
                let ext = companion.extension().unwrap().to_string_lossy();
                let companion_rel = companion.strip_prefix(&temp_extracted).unwrap();
                let dest = final_dest_dir.join(format!("{}.{}", stem, ext));
                println!("  [ROUTED PAK COMPANION] {} -> {}", companion_rel.display(), dest.display());
            }
        }
    }

    Ok(())
}

fn main() {
    println!("==================================================");
    println!("     SMART ROUTING DRY-RUN SIMULATION             ");
    println!("==================================================");

    let scratch_dir = PathBuf::from("scratch");
    let mock_game = scratch_dir.join("mock_game_environment");
    let _ = fs::remove_dir_all(&mock_game);
    fs::create_dir_all(&mock_game).unwrap();

    let downloads_dir = Path::new("C:\\Users\\Antikux\\Downloads");
    let zip1 = downloads_dir.join("PalShrinkRay V0.2.2 4035 0.2.2 2026-07-30T21-45Z vcf5TPVR0.zip");
    let zip2 = downloads_dir.join("GenshinHairForPalworld GAMEPASS 4546 1.01 2026-08-01T01-30Z b7CDk0BBw.7z");
    let zip3 = downloads_dir.join("UniPalUI V0.01.10 TEST 1894 0.01.10TEST 2026-07-22T19-51Z SNjkWigoO.7z");
    let zip4 = downloads_dir.join("Smart Breeding Planner 4341 2.19.0 2026-08-03T06-50Z Rn4IfbzBg.zip");

    if zip1.exists() {
        simulate_smart_installation(&zip1, &mock_game, false, None).unwrap();
    } else {
        println!("Zip 1 not found: {}", zip1.display());
    }

    if zip2.exists() {
        simulate_smart_installation(&zip2, &mock_game, false, None).unwrap(); // Steam Game, default
        simulate_smart_installation(&zip2, &mock_game, false, Some("LogicMods")).unwrap(); // Steam Game, request LogicMods
    } else {
        println!("Zip 2 not found: {}", zip2.display());
    }

    if zip3.exists() {
        simulate_smart_installation(&zip3, &mock_game, false, None).unwrap(); // Steam Game, default
        simulate_smart_installation(&zip3, &mock_game, false, Some("LogicMods")).unwrap(); // Steam Game, request LogicMods
        simulate_smart_installation(&zip3, &mock_game, true, None).unwrap();  // Xbox Game, default
        simulate_smart_installation(&zip3, &mock_game, true, Some("LogicMods")).unwrap();  // Xbox Game, request LogicMods
    } else {
        println!("Zip 3 not found: {}", zip3.display());
    }

    if zip4.exists() {
        simulate_smart_installation(&zip4, &mock_game, false, None).unwrap(); // Steam Game
    } else {
        println!("Zip 4 not found: {}", zip4.display());
    }
}
