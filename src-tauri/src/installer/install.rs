use std::path::Path;
use chrono::Utc;
use crate::models::ModInfo;
use crate::zip_handler::ZipAnalysis;
use super::execution::execute_manifest;

pub fn install_mod(
    game_path: &str,
    extracted_dir: &Path,
    analysis: &ZipAnalysis,
    zip_filename: &str,
    _nexus_mod_id: Option<u32>,
    _nexus_name: Option<String>,
    nexus_author: Option<String>,
    nexus_summary: Option<String>,
    nexus_picture_url: Option<String>,
    nexus_downloads: Option<u32>,
    nexus_endorsements: Option<u32>,
    pak_destination: Option<&str>,
    custom_name: Option<String>,
    _custom_type: Option<String>,
    nexus_category: Option<String>,
    nexus_tags: Vec<String>,
    force_load_order_ue4ss: bool,
    force_load_order_palschema: bool,
) -> Result<ModInfo, String> {
    let game = Path::new(game_path);
    let now = Utc::now().to_rfc3339();

    // 1. Build manifest
    let mut modinfo_data = None;
    if analysis.has_info_json {
        let info_file_path = analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.pmm.json"))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("modinfo.json")))
            .or_else(|| analysis.files.iter().find(|f| f.to_lowercase().ends_with("info.json")));
        if let Some(target_file) = info_file_path {
            let full_path = extracted_dir.join(target_file);
            if full_path.exists() {
                if let Ok(content) = std::fs::read_to_string(full_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        modinfo_data = Some(val);
                    }
                }
            }
        }
    }

    let manifest = crate::zip_handler::build_manifest_from_files(
        &analysis.files,
        zip_filename,
        game,
        pak_destination,
        custom_name,
        modinfo_data.clone(),
    )?;

    let mut final_author = nexus_author;
    let mut final_summary = nexus_summary;

    if let Some(ref modinfo) = modinfo_data {
        if final_author.is_none() {
            if let Some(auth) = modinfo.get("author").and_then(|a| a.as_str()) {
                final_author = Some(auth.to_string());
            }
        }
        if final_summary.is_none() {
            if let Some(desc) = modinfo.get("description").and_then(|d| d.as_str()) {
                final_summary = Some(desc.to_string());
            }
        }
    }

    // 2. Execute manifest
    let mut mod_info = execute_manifest(
        &manifest,
        extracted_dir,
        game,
        final_author,
        final_summary,
        nexus_picture_url,
        nexus_downloads,
        nexus_endorsements,
        &now,
        force_load_order_ue4ss,
        force_load_order_palschema,
    )?;

    // Set other info
    mod_info.nexus_category = nexus_category;
    mod_info.nexus_tags = nexus_tags;
    mod_info.source_zip = zip_filename.to_string();

    Ok(mod_info)
}
