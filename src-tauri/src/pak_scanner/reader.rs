use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use super::types::{PakInspectionResult, PakInternalItem};

/// Classifies asset type based on filename / extension
pub fn classify_asset_type(internal_path: &str) -> String {
    let lower = internal_path.to_lowercase();
    let filename = Path::new(internal_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    if filename.starts_with("DT_") || filename.starts_with("dt_") || lower.contains("/datatable/") {
        "DataTable".to_string()
    } else if filename.starts_with("BP_") || filename.starts_with("bp_") || lower.contains("/blueprint/") {
        "Blueprint".to_string()
    } else if filename.starts_with("WBP_") || filename.starts_with("wbp_") || lower.contains("/umg/") || lower.contains("/widget/") {
        "Widget".to_string()
    } else if filename.starts_with("M_") || filename.starts_with("m_") || filename.starts_with("MI_") || filename.starts_with("mi_") || lower.contains("/material/") {
        "Material".to_string()
    } else if lower.ends_with(".ubulk") || lower.contains("/texture/") || lower.contains("/ui/") || filename.starts_with("T_") || filename.starts_with("t_") {
        "Texture".to_string()
    } else if filename.starts_with("SK_") || filename.starts_with("sk_") || filename.starts_with("SM_") || filename.starts_with("sm_") || lower.contains("/model/") || lower.contains("/mesh/") || lower.contains("/skeletalmesh/") {
        "Mesh".to_string()
    } else if lower.ends_with(".wem") || lower.ends_with(".bnk") || lower.contains("/sound/") || lower.contains("/audio/") || filename.starts_with("AkAudio") || filename.starts_with("SB_") {
        "Audio".to_string()
    } else if filename.starts_with("AM_") || filename.starts_with("am_") || filename.starts_with("AS_") || filename.starts_with("as_") || lower.contains("/animation/") || lower.contains("/anim/") {
        "Animation".to_string()
    } else {
        "Asset".to_string()
    }
}

/// Reads the file index and classifies all internal assets using `repak`
pub fn list_pak_entries_detailed(pak_path: &Path) -> Result<PakInspectionResult, String> {
    let mut file = File::open(pak_path).map_err(|e| format!("Failed to open pak file {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;
    
    let pak = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| format!("Failed to read pak header {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;

    let pak_name = pak_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let raw_files: Vec<String> = pak.files().into_iter().map(|s| s.replace('\\', "/")).collect();
    let total_files = raw_files.len();

    let mut summary_by_type: HashMap<String, usize> = HashMap::new();
    let mut files = Vec::with_capacity(total_files);

    for p in raw_files {
        let asset_type = classify_asset_type(&p);
        *summary_by_type.entry(asset_type.clone()).or_insert(0) += 1;
        let name = Path::new(&p)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| p.clone());

        files.push(PakInternalItem {
            path: p,
            name,
            asset_type,
        });
    }

    Ok(PakInspectionResult {
        pak_name,
        total_files,
        files,
        summary_by_type,
    })
}

/// Reads the file index of an Unreal Engine .pak container using pure-Rust `repak`
pub fn list_pak_entries(pak_path: &Path) -> Result<Vec<String>, String> {
    let mut file = File::open(pak_path).map_err(|e| format!("Failed to open pak file {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;
    
    let pak = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| format!("Failed to read pak header {:?}: {}", pak_path.file_name().unwrap_or_default(), e))?;

    let files: Vec<String> = pak.files().into_iter().map(|s| s.replace('\\', "/")).collect();
    Ok(files)
}

/// Extracts raw bytes of a file inside a .pak archive
pub fn extract_pak_entry(pak_path: &Path, entry_path: &str) -> Result<Vec<u8>, String> {
    let mut file = File::open(pak_path).map_err(|e| format!("Failed to open pak file: {e}"))?;
    let pak = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| format!("Failed to read pak header: {e}"))?;

    let clean_target = entry_path.replace('\\', "/").trim_start_matches('/').to_string();
    let all_files = pak.files();

    if let Some(exact_key) = all_files.iter().find(|f| {
        let clean_f = f.replace('\\', "/").trim_start_matches('/').to_string();
        clean_f.eq_ignore_ascii_case(&clean_target)
    }) {
        return pak.get(exact_key, &mut file).map_err(|e| format!("Failed to extract {entry_path}: {e}"));
    }

    if let Some(suffix_key) = all_files.iter().find(|f| {
        let clean_f = f.replace('\\', "/").trim_start_matches('/').to_string();
        clean_f.ends_with(&clean_target) || clean_target.ends_with(&clean_f)
    }) {
        return pak.get(suffix_key, &mut file).map_err(|e| format!("Failed to extract {entry_path}: {e}"));
    }

    pak.get(entry_path, &mut file).map_err(|e| format!("Failed to extract {entry_path}: {e}"))
}
