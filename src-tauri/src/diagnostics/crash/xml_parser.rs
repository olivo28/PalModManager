use std::path::Path;
use super::types::{CrashContextXml, CrashMemoryStats};

// Fast zero-copy parser for Unreal Engine CrashContext.runtime-xml
pub fn parse_crash_context_xml(content: &str) -> CrashContextXml {
    let error_message = extract_tag_value(content, "ErrorMessage").unwrap_or_default();
    let crash_type = extract_tag_value(content, "CrashType").unwrap_or_else(|| "Crash".to_string());
    
    let seconds_since_start = extract_tag_value(content, "SecondsSinceStart")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let game_version = extract_tag_value(content, "GameVersion")
        .or_else(|| extract_tag_value(content, "BuildVersion"));
    let engine_version = extract_tag_value(content, "EngineVersion");

    // Locate crashed thread info marked with <IsCrashed>true</IsCrashed>
    let (crashed_thread_name, crashed_thread_callstack) = extract_crashed_thread_info(content);

    // Extract raw callstack from crashed thread or general tag
    let callstack_raw = extract_tag_value(content, "CallStack")
        .or_else(|| extract_tag_value(content, "CallStackProcess"))
        .or_else(|| extract_tag_value(content, "PCallStack"))
        .or_else(|| crashed_thread_callstack.clone())
        .unwrap_or_default();

    // Physical and virtual memory statistics
    let memory_stats = extract_memory_stats(content);

    // List of loaded modules
    let loaded_modules = extract_modules_list(content);

    CrashContextXml {
        error_message,
        crash_type,
        seconds_since_start,
        crashed_thread_name,
        crashed_thread_callstack,
        callstack_raw,
        memory_stats,
        loaded_modules,
        game_version,
        engine_version,
        raw_xml_content: Some(content.to_string()),
    }
}

pub fn parse_crash_context_file(path: &Path) -> Option<CrashContextXml> {
    if !path.exists() {
        return None;
    }
    match std::fs::read_to_string(path) {
        Ok(content) => Some(parse_crash_context_xml(&content)),
        Err(e) => {
            crate::logger::log(&format!("Crash Doctor: Failed to read XML at {:?}: {}", path, e));
            None
        }
    }
}

fn extract_tag_value(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    
    if let Some(start_idx) = xml.find(&open_tag) {
        let val_start = start_idx + open_tag.len();
        if let Some(end_idx) = xml[val_start..].find(&close_tag) {
            let value = &xml[val_start..val_start + end_idx];
            return Some(clean_xml_entities(value.trim()));
        }
    }
    None
}

fn extract_crashed_thread_info(xml: &str) -> (Option<String>, Option<String>) {
    let mut search_pos = 0;
    while let Some(thread_start) = xml[search_pos..].find("<Thread>") {
        let abs_start = search_pos + thread_start;
        let thread_end = match xml[abs_start..].find("</Thread>") {
            Some(end) => abs_start + end + "</Thread>".len(),
            None => break,
        };
        
        let thread_chunk = &xml[abs_start..thread_end];
        if thread_chunk.contains("<IsCrashed>true</IsCrashed>") {
            let name = extract_tag_value(thread_chunk, "ThreadName")
                .or_else(|| extract_tag_value(thread_chunk, "Name"))
                .or_else(|| Some("CrashedThread".to_string()));
            let callstack = extract_tag_value(thread_chunk, "CallStack")
                .or_else(|| extract_tag_value(thread_chunk, "PCallStack"));
            return (name, callstack);
        }
        search_pos = thread_end;
    }
    (None, None)
}

fn extract_memory_stats(xml: &str) -> CrashMemoryStats {
    let total_phys = extract_tag_value(xml, "TotalPhysical")
        .or_else(|| extract_tag_value(xml, "MemoryStats.TotalPhysical"))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|bytes| bytes / (1024.0 * 1024.0 * 1024.0));

    let avail_phys = extract_tag_value(xml, "AvailablePhysical")
        .or_else(|| extract_tag_value(xml, "MemoryStats.AvailablePhysical"))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|bytes| bytes / (1024.0 * 1024.0 * 1024.0));

    let total_virt = extract_tag_value(xml, "TotalVirtual")
        .or_else(|| extract_tag_value(xml, "MemoryStats.TotalVirtual"))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|bytes| bytes / (1024.0 * 1024.0 * 1024.0));

    let avail_virt = extract_tag_value(xml, "AvailableVirtual")
        .or_else(|| extract_tag_value(xml, "MemoryStats.AvailableVirtual"))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|bytes| bytes / (1024.0 * 1024.0 * 1024.0));

    let is_oom_risk = match avail_phys {
        Some(gb) => gb < 0.5, // Menos de 512 MB disponibles
        None => false,
    };

    CrashMemoryStats {
        total_physical_gb: total_phys,
        available_physical_gb: avail_phys,
        total_virtual_gb: total_virt,
        available_virtual_gb: avail_virt,
        is_oom_risk,
    }
}

fn extract_modules_list(xml: &str) -> Vec<String> {
    let mut modules = Vec::new();
    let mut search_pos = 0;
    while let Some(mod_start) = xml[search_pos..].find("<Module>") {
        let abs_start = search_pos + mod_start;
        let mod_end = match xml[abs_start..].find("</Module>") {
            Some(end) => abs_start + end + "</Module>".len(),
            None => break,
        };

        let mod_chunk = &xml[abs_start..mod_end];
        if let Some(path) = extract_tag_value(mod_chunk, "Path") {
            modules.push(path);
        } else if let Some(name) = extract_tag_value(mod_chunk, "Name") {
            modules.push(name);
        }

        search_pos = mod_end;
    }
    modules
}

fn clean_xml_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
