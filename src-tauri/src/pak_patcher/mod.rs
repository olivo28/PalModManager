use std::fs::{self, File};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use chrono::Local;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatchAssetSelection {
    pub asset_path: String,
    pub source_pak_path: String,
    #[serde(default)]
    pub companion_extensions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatchBuildRequest {
    pub patch_name: String,
    pub target_profile_id: Option<String>,
    pub selections: Vec<PatchAssetSelection>,
    pub is_gamepass: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatchBuildResult {
    pub output_path: String,
    pub patch_name: String,
    pub total_assets_packed: usize,
    pub file_size_bytes: u64,
    pub is_gamepass: bool,
    pub utoc_path: Option<String>,
    pub ucas_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredPatch {
    pub id: String,
    pub profile_id: String,
    pub display_name: String,
    pub pak_filename: String,
    pub pak_path: String,
    pub file_size_bytes: u64,
    pub created_at: String,
    pub is_gamepass: bool,
    #[serde(default)]
    pub resolved_assets: Vec<String>,
    #[serde(default)]
    pub selections: Vec<PatchAssetSelection>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPatchInfo {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub file_name: String,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub created_at: String,
    pub is_gamepass: bool,
    #[serde(default)]
    pub resolved_assets_count: usize,
}

pub fn get_profile_patches_path(program_path: &str, profile_id: &str) -> PathBuf {
    if !program_path.is_empty() && !profile_id.is_empty() {
        crate::profiles::utils::get_profile_dir(program_path, profile_id).join("compatibility_patches.json")
    } else if !program_path.is_empty() {
        PathBuf::from(program_path).join("compatibility_patches.json")
    } else {
        PathBuf::from("compatibility_patches.json")
    }
}

pub fn load_profile_patches_registry(program_path: &str, profile_id: &str) -> Vec<RegisteredPatch> {
    let path = get_profile_patches_path(program_path, profile_id);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(patches) = serde_json::from_str::<Vec<RegisteredPatch>>(&content) {
                return patches;
            }
        }
    }
    Vec::new()
}

pub fn save_profile_patches_registry(program_path: &str, profile_id: &str, patches: &[RegisteredPatch]) {
    let path = get_profile_patches_path(program_path, profile_id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(patches) {
        let _ = fs::write(path, json);
    }
}

/// Builds a manual compatibility .pak patch from user-selected assets using pure-Rust repak
pub async fn build_compatibility_pak_internal(
    request: PatchBuildRequest,
    game_path: &str,
    app_data_dir: &Path,
    program_path: &str,
    profile_id: &str,
) -> Result<PatchBuildResult, String> {
    if request.selections.is_empty() {
        return Err("No assets selected for patch generation".to_string());
    }

    crate::logger::log(&format!(
        "build_compatibility_pak: Building patch '{}' for profile '{}' with {} selected assets",
        request.patch_name,
        profile_id,
        request.selections.len()
    ));

    // 1. Clean and normalize patch filename: enforce zzz_ prefix and _P.pak suffix
    let mut clean_name = request.patch_name.trim().to_string();
    if clean_name.ends_with(".pak") || clean_name.ends_with(".PAK") {
        clean_name = clean_name[..clean_name.len() - 4].to_string();
    }
    if !clean_name.starts_with("zzz_") {
        clean_name = format!("zzz_{clean_name}");
    }
    if !clean_name.ends_with("_P") && !clean_name.ends_with("_p") {
        clean_name = format!("{clean_name}_P");
    }
    let final_pak_filename = format!("{clean_name}.pak");

    // 2. Resolve destination directory in the active mod paks path (~mods subfolder)
    let game_root = Path::new(game_path);
    let paks_root = game_root.join("Pal").join("Content").join("Paks");
    let mods_dir = paks_root.join("~mods");
    let target_dir = if mods_dir.exists() {
        mods_dir
    } else {
        paks_root
    };
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create patch destination directory: {e}"))?;
    }
    let output_pak_path = target_dir.join(&final_pak_filename);

    // 3. Extract and stage selected assets and companion files (.uexp, .ubulk, .uptnl) in memory
    let mut staged_entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut processed_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    for sel in &request.selections {
        let src_pak = Path::new(&sel.source_pak_path);
        if !src_pak.exists() {
            crate::logger::log(&format!(
                "build_compatibility_pak: Warning, source pak not found: {:?}",
                src_pak
            ));
            continue;
        }

        // Primary asset
        let norm_path = sel.asset_path.replace('\\', "/").trim_start_matches('/').to_string();
        if !processed_paths.contains(&norm_path) {
            if let Ok(bytes) = crate::pak_scanner::extract_pak_entry(src_pak, &norm_path) {
                staged_entries.push((norm_path.clone(), bytes));
                processed_paths.insert(norm_path.clone());
            }
        }

        // Detect companion files (.uexp, .ubulk, .uptnl) if base is .uasset
        let base_no_ext = if norm_path.to_lowercase().ends_with(".uasset") {
            &norm_path[..norm_path.len() - 7]
        } else {
            &norm_path
        };

        let companion_extensions = [".uexp", ".ubulk", ".uptnl"];
        for ext in companion_extensions {
            let comp_path = format!("{base_no_ext}{ext}");
            if !processed_paths.contains(&comp_path) {
                if let Ok(bytes) = crate::pak_scanner::extract_pak_entry(src_pak, &comp_path) {
                    staged_entries.push((comp_path.clone(), bytes));
                    processed_paths.insert(comp_path);
                }
            }
        }
    }

    if staged_entries.is_empty() {
        return Err("Failed to extract any selected asset bytes from source paks".to_string());
    }

    // 4. Create new .pak file using repak
    let total_assets_packed = staged_entries.len();
    {
        let mut out_file = File::create(&output_pak_path)
            .map_err(|e| format!("Failed to create output pak file: {e}"))?;

        let mut pak_writer = repak::PakBuilder::new()
            .writer(&mut out_file, repak::Version::V11, "../../../".to_string(), None);

        for (rel_path, data) in &staged_entries {
            pak_writer
                .write_file(rel_path, false, data)
                .map_err(|e| format!("Failed to pack entry '{rel_path}': {e}"))?;
        }

        pak_writer
            .write_index()
            .map_err(|e| format!("Failed to finalize repak index: {e}"))?;
    }

    let file_size_bytes = fs::metadata(&output_pak_path)
        .map(|m| m.len())
        .unwrap_or(0);

    crate::logger::log(&format!(
        "build_compatibility_pak: Successfully wrote '{}' ({} bytes, {} entries)",
        output_pak_path.display(),
        file_size_bytes,
        total_assets_packed
    ));

    // 5. If Xbox Game Pass requested, run retoc conversion to create .utoc and .ucas
    let mut utoc_path = None;
    let mut ucas_path = None;
    if request.is_gamepass {
        match crate::retoc_runner::convert_pak_to_gamepass_zen(&output_pak_path, app_data_dir).await {
            Ok((utoc, ucas)) => {
                crate::logger::log(&format!(
                    "build_compatibility_pak: Game Pass IoStore conversion success: {:?}, {:?}",
                    utoc, ucas
                ));
                utoc_path = Some(utoc.to_string_lossy().to_string());
                ucas_path = Some(ucas.to_string_lossy().to_string());
            }
            Err(e) => {
                crate::logger::log(&format!(
                    "build_compatibility_pak: Warning: Game Pass retoc conversion failed: {e}"
                ));
            }
        }
    }

    // 6. Record patch in the active profile's compatibility_patches.json
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut resolved_list: Vec<String> = processed_paths.into_iter().collect();
    resolved_list.sort();

    let reg_patch = RegisteredPatch {
        id: format!("patch_{}", chrono::Local::now().timestamp_millis()),
        profile_id: profile_id.to_string(),
        display_name: request.patch_name.clone(),
        pak_filename: final_pak_filename.clone(),
        pak_path: output_pak_path.to_string_lossy().to_string(),
        file_size_bytes,
        created_at: now_str,
        is_gamepass: request.is_gamepass,
        resolved_assets: resolved_list,
        selections: request.selections.clone(),
    };

    let mut registry = load_profile_patches_registry(program_path, profile_id);
    registry.retain(|p| p.pak_filename != final_pak_filename);
    registry.push(reg_patch);
    save_profile_patches_registry(program_path, profile_id, &registry);

    Ok(PatchBuildResult {
        output_path: output_pak_path.to_string_lossy().to_string(),
        patch_name: final_pak_filename,
        total_assets_packed,
        file_size_bytes,
        is_gamepass: request.is_gamepass,
        utoc_path,
        ucas_path,
    })
}

/// Lists all generated compatibility patches in the active game paks folders (~mods and Paks) for the current profile
pub fn list_generated_patches_internal(game_path: &str, program_path: &str, profile_id: &str) -> Result<Vec<GeneratedPatchInfo>, String> {
    let paks_dir = Path::new(game_path).join("Pal").join("Content").join("Paks");
    let mods_dir = paks_dir.join("~mods");

    let mut search_dirs = Vec::new();
    if mods_dir.exists() {
        search_dirs.push(mods_dir);
    }
    if paks_dir.exists() {
        search_dirs.push(paks_dir);
    }

    let registry = load_profile_patches_registry(program_path, profile_id);
    let mut patches = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for dir in search_dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() {
                    let filename = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let path_str = p.to_string_lossy().to_string();
                    
                    let reg_entry = registry.iter().find(|r| r.pak_filename == filename || r.pak_path == path_str);
                    let is_patch = reg_entry.is_some() || (filename.starts_with("zzz_") && filename.to_lowercase().ends_with(".pak"));

                    if is_patch {
                        if !seen_paths.insert(path_str.clone()) {
                            continue;
                        }

                        let meta = fs::metadata(&p).ok();
                        let file_size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                        let created_at = reg_entry.map(|r| r.created_at.clone()).unwrap_or_else(|| {
                            meta.and_then(|m| m.modified().ok())
                                .map(|sys_time| {
                                    let dt: chrono::DateTime<Local> = sys_time.into();
                                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                                })
                                .unwrap_or_else(|| "Unknown".to_string())
                        });

                        let parent_dir = p.parent().unwrap_or(&dir);
                        let has_utoc = parent_dir.join(format!("{}.utoc", &filename[..filename.len() - 4])).exists();
                        let display_name = reg_entry.map(|r| r.display_name.clone());
                        let patch_id = reg_entry.map(|r| r.id.clone());
                        let resolved_assets_count = reg_entry.map(|r| r.resolved_assets.len()).unwrap_or(0);

                        patches.push(GeneratedPatchInfo {
                            id: patch_id,
                            display_name,
                            file_name: filename,
                            file_path: path_str,
                            file_size_bytes,
                            created_at,
                            is_gamepass: has_utoc,
                            resolved_assets_count,
                        });
                    }
                }
            }
        }
    }

    patches.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(patches)
}

/// Deletes a generated patch (.pak, plus .utoc/.ucas/.sig if existing) and removes it from the profile registry
pub fn delete_generated_patch_internal(patch_path: &str, program_path: &str, profile_id: &str) -> Result<(), String> {
    let p = Path::new(patch_path);
    let filename = p.file_name().unwrap_or_default().to_string_lossy().to_string();

    crate::logger::log(&format!("delete_generated_patch: Deleting patch {:?} for profile '{}'", p, profile_id));
    if p.exists() {
        fs::remove_file(p).map_err(|e| format!("Failed to delete patch .pak: {e}"))?;

        // Clean companions (.utoc, .ucas, .sig)
        let parent = p.parent().unwrap_or(p);
        if filename.len() >= 4 {
            let base = &filename[..filename.len() - 4];
            for ext in [".utoc", ".ucas", ".sig"] {
                let comp = parent.join(format!("{base}{ext}"));
                if comp.exists() {
                    let _ = fs::remove_file(comp);
                }
            }
        }
    }

    // Clean from profile registry
    let mut registry = load_profile_patches_registry(program_path, profile_id);
    let orig_len = registry.len();
    registry.retain(|r| r.pak_path != patch_path && r.pak_filename != filename);
    if registry.len() != orig_len {
        save_profile_patches_registry(program_path, profile_id, &registry);
    }

    Ok(())
}
