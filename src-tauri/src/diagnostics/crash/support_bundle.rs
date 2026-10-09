use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;
use super::types::{SupportBundleResult, UnifiedCrashReport};

// Create forensic support ZIP with markdown report, crash dump, engine XML, and manifests
pub fn generate_support_bundle(
    report: &UnifiedCrashReport,
    output_path: &Path,
    game_root: &Path,
) -> SupportBundleResult {
    let target_zip_path = if output_path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("zip")).unwrap_or(false) {
        output_path.to_path_buf()
    } else {
        let timestamp_str = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let zip_filename = format!("PalModManager_Crash_Support_{}.zip", timestamp_str);
        output_path.join(&zip_filename)
    };

    let file = match File::create(&target_zip_path) {
        Ok(f) => f,
        Err(e) => {
            return SupportBundleResult {
                success: false,
                bundle_path: String::new(),
                file_size_bytes: 0,
                bundled_files: Vec::new(),
                error: Some(format!("Failed to create support ZIP: {}", e)),
            };
        }
    };

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let mut bundled_files = Vec::new();

    // Generate markdown report
    let md_content = generate_markdown_report(report);
    if zip.start_file("CRASH_REPORT.md", options).is_ok() {
        let _ = zip.write_all(md_content.as_bytes());
        bundled_files.push("CRASH_REPORT.md".to_string());
    }

    // Bundle CrashContext.runtime-xml if present
    let crash_folder = PathBuf::from(&report.crash_folder);
    let xml_path = crash_folder.join("CrashContext.runtime-xml");
    if xml_path.exists() {
        if let Ok(bytes) = std::fs::read(&xml_path) {
            if zip.start_file("CrashContext.runtime-xml", options).is_ok() {
                let _ = zip.write_all(&bytes);
                bundled_files.push("CrashContext.runtime-xml".to_string());
            }
        }
    }

    // Bundle raw minidump
    let dmp_candidates = [
        crash_folder.join("UEMinidump.dmp"),
        game_root.join("Pal").join("Binaries").join("Win64").join("UE4SS_Crash.dmp"),
        crash_folder.join("UE4SS_Crash.dmp"),
    ];

    for dmp in &dmp_candidates {
        if dmp.exists() {
            if let Ok(bytes) = std::fs::read(dmp) {
                let fname = dmp.file_name().and_then(|n| n.to_str()).unwrap_or("crash.dmp");
                if zip.start_file(fname, options).is_ok() {
                    let _ = zip.write_all(&bytes);
                    bundled_files.push(fname.to_string());
                    break;
                }
            }
        }
    }

    // Bundle raw UE4SS runtime log
    let log_path = game_root.join("Pal").join("Binaries").join("Win64").join("UE4SS.log");
    if log_path.exists() {
        if let Ok(bytes) = std::fs::read(&log_path) {
            if zip.start_file("UE4SS.log", options).is_ok() {
                let _ = zip.write_all(&bytes);
                bundled_files.push("UE4SS.log".to_string());
            }
        }
    }

    // Bundle active mods profile snapshot
    if let Ok(json_str) = serde_json::to_string_pretty(&report.active_mods) {
        if zip.start_file("active_mods_manifest.json", options).is_ok() {
            let _ = zip.write_all(json_str.as_bytes());
            bundled_files.push("active_mods_manifest.json".to_string());
        }
    }

    if let Err(e) = zip.finish() {
        return SupportBundleResult {
            success: false,
            bundle_path: String::new(),
            file_size_bytes: 0,
            bundled_files,
            error: Some(format!("Failed to finalize ZIP archive: {}", e)),
        };
    }

    let file_size_bytes = std::fs::metadata(&target_zip_path)
        .map(|m| m.len())
        .unwrap_or(0);

    SupportBundleResult {
        success: true,
        bundle_path: target_zip_path.to_string_lossy().to_string(),
        file_size_bytes,
        bundled_files,
        error: None,
    }
}

fn generate_markdown_report(report: &UnifiedCrashReport) -> String {
    let mut out = String::new();
    out.push_str("# PalModManager — Crash Forensic Report\n\n");
    out.push_str(&format!("- **Incident Timestamp:** {}\n", report.timestamp));
    out.push_str(&format!("- **Primary Cause Identified:** `{}`\n", report.primary_cause));
    out.push_str(&format!("- **Severity:** `{}`\n", report.severity));
    if let Some(ref c) = report.culprit_name {
        out.push_str(&format!("- **Suspected Culprit:** `{}` ({})\n", c, report.culprit_type.as_deref().unwrap_or("Unknown")));
    }
    out.push_str(&format!("\n## Diagnostic Summary\n{}\n\n", report.human_summary));
    out.push_str(&format!("## Technical Details\n```text\n{}\n```\n\n", report.technical_details));

    if let Some(ref md) = report.minidump {
        out.push_str("## Binary Minidump Forensics\n");
        out.push_str(&format!("- **Exception Code:** `{}` ({})\n", md.exception_code_hex, md.exception_meaning));
        out.push_str(&format!("- **Faulting RIP Address:** `{}`\n", md.faulting_address_hex));
        if let Some(ref fm) = md.faulting_module {
            out.push_str(&format!("- **Faulting Module:** `{}` (Offset: `0x{:X}`, Path: `{}`)\n", fm.name, fm.rip_offset, fm.full_path));
        }
        out.push('\n');
    }

    if !report.recent_log_errors.is_empty() {
        out.push_str("## Recent Engine / UE4SS Errors\n```text\n");
        for line in &report.recent_log_errors {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("```\n\n");
    }

    out.push_str("## Active Mods at Crash Time\n");
    out.push_str("| Mod Name | Type | Version | Nexus ID |\n| :--- | :--- | :--- | :--- |\n");
    for m in &report.active_mods {
        let nexus = m.nexus_id.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());
        out.push_str(&format!("| {} | {} | {} | {} |\n", m.name, m.mod_type, m.version, nexus));
    }

    out
}
