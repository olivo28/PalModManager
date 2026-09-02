use std::path::Path;

pub const GAME_PATH_SEGMENTS: &[&str] = &[
    "pal", "content", "paks", "~mods", "logicmods",
    "binaries", "win64", "wingdk",
    "ue4ss", "mods",
    "palschema",
];

pub const PALSCHEMA_FOLDERS: &[&str] = &[
    "resources", "enums", "pals", "npcs", "items", "skins",
    "appearance", "buildings", "raw", "blueprints", "helpguide",
    "spawns", "translations", "paks"
];

pub const FORBIDDEN_MOD_NAMES: &[&str] = &[
    "pal", "palworld", "mods", "win64", "wingdk", "binaries", "content", "paks", "~mods",
    "logicmods", "ue4ss", "palschema", "plugins", "scripts", "nativemods",
    "blueprints", "translations", "steam", "(steam)", "xbox", "(xbox)", "gdk", "(gdk)",
    "gamepass", "(gamepass)", "swapjson", "alterconfig", "release", "build", "dist",
];

pub fn is_forbidden(name: &str) -> bool {
    let lower = name.to_lowercase();
    let trimmed = lower.trim_matches(|c: char| c == '(' || c == ')' || c == '[' || c == ']' || c.is_whitespace());
    FORBIDDEN_MOD_NAMES.contains(&lower.as_str())
        || FORBIDDEN_MOD_NAMES.contains(&trimmed)
        || lower.is_empty()
        || lower.starts_with("(steam)")
        || lower.starts_with("(xbox)")
        || lower.starts_with("(gdk)")
        || lower.starts_with("(gamepass)")
        || lower.contains("mods folder")
        || lower.contains("mod folder")
        || lower.contains("ue4ss mods")
        || lower.contains("palschema mods")
        || lower.contains("mods directory")
        || lower.contains("mod directory")
}

pub fn detect_folder_name_from_files(files: &[String], zip_filename: &str) -> String {
    // Strategy 1a: Scan strictly for UE4SS / LogicMods markers (scripts, dlls, logicmods)
    // to ensure the UE4SS mod name is prioritized in hybrid mods (important for mods.txt).
    for file in files {
        if file.ends_with('/') {
            continue;
        }
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                let lower = segment.to_lowercase();
                if lower == "scripts" || lower == "dlls" || lower == "logicmods" {
                    // Climb up candidate parents to find a non-forbidden name
                    let mut j = i as isize - 1;
                    while j >= 0 {
                        let candidate = segments[j as usize];
                        if !is_forbidden(candidate) {
                            return candidate.to_string();
                        }
                        j -= 1;
                    }
                }
            }
        }
    }

    // Strategy 1b: Fallback scan for PalSchema boundary markers.
    for file in files {
        if file.ends_with('/') {
            continue;
        }
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                let lower = segment.to_lowercase();
                if PALSCHEMA_FOLDERS.contains(&lower.as_str()) {
                    // Climb up candidate parents to find a non-forbidden name
                    let mut j = i as isize - 1;
                    while j >= 0 {
                        let candidate = segments[j as usize];
                        if !is_forbidden(candidate) {
                            return candidate.to_string();
                        }
                        j -= 1;
                    }
                }
            }
        }
    }

    // Strategy 2: Fallback to the first non-forbidden, non-file segment
    for file in files {
        if file.ends_with('/') {
            continue;
        }
        let normalized = file.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        let num_segments = segments.len();
        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == num_segments - 1;
            
            // Skip the last segment (leaf file name) to avoid treating batch/txt/png/lua files as folder names
            if is_last {
                let lower = segment.to_lowercase();
                let stem = Path::new(segment).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if lower.ends_with(".dll") {
                    return stem;
                }
                continue;
            }

            let p = Path::new(segment);
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let stem_lower = stem.to_lowercase();
            let lower = segment.to_lowercase();
            if GAME_PATH_SEGMENTS.contains(&lower.as_str()) || GAME_PATH_SEGMENTS.contains(&stem_lower.as_str()) {
                continue;
            }
            // Skip platform wrappers
            if lower == "(steam)" || lower == "steam" || lower == "(xbox)" || lower == "xbox" ||
               lower == "(gdk)" || lower == "gdk" || lower == "wingdk" ||
               stem_lower == "(steam)" || stem_lower == "steam" || stem_lower == "(xbox)" || stem_lower == "xbox" ||
               stem_lower == "(gdk)" || stem_lower == "gdk" || stem_lower == "wingdk" {
                continue;
            }
            // Skip common wrapper/instruction directories
            let contains_wrapper_words = lower.contains("mods folder") || lower.contains("mod folder") ||
                                         lower.contains("ue4ss mods") || lower.contains("palschema mods") ||
                                         lower.contains("mods directory") || lower.contains("mod directory");
            if contains_wrapper_words || stem_lower.contains("mods folder") || stem_lower.contains("mod folder") ||
               stem_lower.contains("ue4ss mods") || stem_lower.contains("palschema mods") ||
               stem_lower.contains("mods directory") || stem_lower.contains("mod directory") {
                continue;
            }
            // Skip generic metadata/documentation files and images
            if lower == "license" || lower == "readme" || lower == "changelog" ||
               lower.ends_with(".txt") || lower.ends_with(".md") || lower.ends_with(".png") ||
               lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".git") ||
               lower.ends_with(".gitattributes") || lower.ends_with(".gitignore") ||
               lower.ends_with(".bat") || lower.ends_with(".sh") || lower.ends_with(".py") {
                continue;
            }
            if PALSCHEMA_FOLDERS.contains(&lower.as_str()) || PALSCHEMA_FOLDERS.contains(&stem_lower.as_str()) {
                break;
            }
            if lower == "scripts" || lower == "dlls" || stem_lower == "scripts" || stem_lower == "dlls" {
                break;
            }
            if lower.ends_with(".dll") {
                return stem;
            }
            if lower.ends_with(".json") || lower.ends_with(".jsonc") || lower.ends_with(".pak") {
                continue;
            }
            return segment.to_string();
        }
    }

    // Strategy 3: Check for .pak file stem (for pure PAK mods)
    for file in files {
        let normalized = file.replace('\\', "/");
        let lower = normalized.to_lowercase();
        if lower.ends_with(".pak") {
            if let Some(leaf) = normalized.split('/').last() {
                let pak_stem = Path::new(leaf).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let cleaned_pak = crate::installer::clean_zip_name(&pak_stem);
                if !cleaned_pak.is_empty() && cleaned_pak != "unknown" && !cleaned_pak.starts_with("nexus_") {
                    return cleaned_pak;
                }
                if !pak_stem.is_empty() {
                    return pak_stem;
                }
            }
        }
    }

    crate::installer::clean_zip_name(zip_filename)
}
