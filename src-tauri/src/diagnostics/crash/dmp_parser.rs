use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt};
use super::types::{FaultingModuleInfo, MinidumpInfo, RawDmpModule};

// Standard Microsoft Minidump constants
const MINIDUMP_SIGNATURE: u32 = 0x504D444D; // 'MDMP'
const STREAM_TYPE_EXCEPTION: u32 = 6;
const STREAM_TYPE_MODULE_LIST: u32 = 4;

#[derive(Debug, Clone)]
pub struct RawModuleEntry {
    pub base_of_image: u64,
    pub size_of_image: u32,
    pub name: String,
    pub full_path: String,
}

// Parse essential crash records and modules from .dmp file
pub fn parse_minidump_file(path: &Path) -> Option<MinidumpInfo> {
    if !path.exists() {
        return None;
    }

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            crate::logger::log(&format!("Crash Doctor: Failed to open minidump {:?}: {}", path, e));
            return None;
        }
    };

    // Read minidump header
    let signature = file.read_u32::<LittleEndian>().ok()?;
    if signature != MINIDUMP_SIGNATURE {
        crate::logger::log(&format!("Crash Doctor: Invalid minidump signature in {:?}", path));
        return None;
    }

    let _version = file.read_u32::<LittleEndian>().ok()?;
    let stream_count = file.read_u32::<LittleEndian>().ok()? as usize;
    let stream_dir_rva = file.read_u32::<LittleEndian>().ok()? as u64;

    // Locate ExceptionStream and ModuleListStream
    let mut exception_rva: Option<u64> = None;
    let mut module_list_rva: Option<u64> = None;

    if file.seek(SeekFrom::Start(stream_dir_rva)).is_err() {
        return None;
    }

    for _ in 0..stream_count {
        let stream_type = file.read_u32::<LittleEndian>().ok()?;
        let _data_size = file.read_u32::<LittleEndian>().ok()?;
        let rva = file.read_u32::<LittleEndian>().ok()? as u64;

        if stream_type == STREAM_TYPE_EXCEPTION {
            exception_rva = Some(rva);
        } else if stream_type == STREAM_TYPE_MODULE_LIST {
            module_list_rva = Some(rva);
        }
    }

    // 3. Extraer ExceptionRecord
    let mut exception_code = 0u32;
    let mut faulting_address = 0u64;
    let mut thread_id = 0u32;

    if let Some(rva) = exception_rva {
        if file.seek(SeekFrom::Start(rva)).is_ok() {
            thread_id = file.read_u32::<LittleEndian>().unwrap_or(0);
            let _alignment = file.read_u32::<LittleEndian>().unwrap_or(0);
            
            // MINIDUMP_EXCEPTION (ExceptionCode, ExceptionFlags, ExceptionRecord, ExceptionAddress, NumberParameters...)
            exception_code = file.read_u32::<LittleEndian>().unwrap_or(0);
            let _exception_flags = file.read_u32::<LittleEndian>().unwrap_or(0);
            let _exception_record = file.read_u64::<LittleEndian>().unwrap_or(0);
            faulting_address = file.read_u64::<LittleEndian>().unwrap_or(0);
        }
    }

    // 4. Extraer Lista de Módulos (cada MINIDUMP_MODULE mide exactamente 108 bytes)
    let mut modules = Vec::new();
    if let Some(rva) = module_list_rva {
        if file.seek(SeekFrom::Start(rva)).is_ok() {
            if let Ok(number_of_modules) = file.read_u32::<LittleEndian>() {
                for i in 0..number_of_modules {
                    let entry_rva = (rva + 4) + (i as u64 * 108);
                    if file.seek(SeekFrom::Start(entry_rva)).is_err() {
                        break;
                    }

                    // MINIDUMP_MODULE (BaseOfImage: 8, SizeOfImage: 4, CheckSum: 4, TimeDateStamp: 4, ModuleNameRva: 4)
                    let base_of_image = file.read_u64::<LittleEndian>().unwrap_or(0);
                    let size_of_image = file.read_u32::<LittleEndian>().unwrap_or(0);
                    let _checksum = file.read_u32::<LittleEndian>().unwrap_or(0);
                    let _timedatestamp = file.read_u32::<LittleEndian>().unwrap_or(0);
                    let module_name_rva = file.read_u32::<LittleEndian>().unwrap_or(0) as u64;

                    // Leer nombre en UTF-16 a partir de module_name_rva
                    let full_path = read_minidump_string(&mut file, module_name_rva).unwrap_or_default();
                    let name = Path::new(&full_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    modules.push(RawModuleEntry {
                        base_of_image,
                        size_of_image,
                        name,
                        full_path,
                    });
                }
            }
        }
    }

    // 5. Mapear qué módulo contiene la dirección de fallo (faulting_address)
    let faulting_module = resolve_faulting_module(faulting_address, &modules);

    let exception_meaning = match exception_code {
        0xC0000005 => "ACCESS_VIOLATION (Invalid memory access / null pointer)".to_string(),
        0xC00000FD => "STACK_OVERFLOW (Call stack limit exceeded)".to_string(),
        0xC000001D => "ILLEGAL_INSTRUCTION (Unsupported CPU instruction)".to_string(),
        0x80000003 => "BREAKPOINT (Assertion failed / debugger trap)".to_string(),
        0x00004000 => "UNREAL_ASSERTION (FDebug::AssertFailed / LowLevelFatalError)".to_string(),
        0xE06D7363 => "CPP_EXCEPTION (Unhandled C++ exception)".to_string(),
        0xC0000374 => "HEAP_CORRUPTION (Process heap memory damaged)".to_string(),
        _ => format!("Unknown Exception (0x{:08X})", exception_code),
    };

    // 6. Construir lista estructurada de módulos con hex base y categorías
    let raw_modules: Vec<RawDmpModule> = modules
        .iter()
        .map(|m| {
            let is_faulting = faulting_address >= m.base_of_image
                && faulting_address < (m.base_of_image + m.size_of_image as u64);
            let category = categorize_module_name(&m.name, &m.full_path);
            RawDmpModule {
                name: m.name.clone(),
                full_path: m.full_path.clone(),
                base_address_hex: format!("0x{:016X}", m.base_of_image),
                size_bytes: m.size_of_image,
                is_faulting,
                category,
            }
        })
        .collect();

    // 7. Generar texto de diagnóstico raw estructurado
    let mut raw_dump_summary = String::new();
    raw_dump_summary.push_str("================ MINIDUMP FORENSIC SUMMARY ================\n");
    raw_dump_summary.push_str(&format!("Stream Count         : {}\n", stream_count));
    raw_dump_summary.push_str(&format!("Exception Code       : 0x{:08X} ({})\n", exception_code, exception_meaning));
    raw_dump_summary.push_str(&format!("Faulting Address     : 0x{:016X}\n", faulting_address));
    raw_dump_summary.push_str(&format!("Crashed Thread ID    : {}\n", thread_id));
    if let Some(ref fm) = faulting_module {
        raw_dump_summary.push_str(&format!("Faulting Module      : {} (Base: 0x{:016X}, Size: 0x{:X})\n", fm.name, fm.base_address, fm.size));
        raw_dump_summary.push_str(&format!("Faulting Path        : {}\n", fm.full_path));
        raw_dump_summary.push_str(&format!("Relative Offset      : +0x{:X}\n", fm.rip_offset));
    }
    raw_dump_summary.push_str(&format!("Total Loaded Modules : {}\n", modules.len()));
    raw_dump_summary.push_str("===========================================================\n");

    Some(MinidumpInfo {
        exception_code,
        exception_code_hex: format!("0x{:08X}", exception_code),
        exception_meaning,
        faulting_address,
        faulting_address_hex: format!("0x{:016X}", faulting_address),
        thread_id,
        faulting_module,
        total_modules_count: modules.len(),
        raw_modules,
        raw_dump_summary,
    })
}

fn resolve_faulting_module(rip: u64, modules: &[RawModuleEntry]) -> Option<FaultingModuleInfo> {
    for m in modules {
        let base = m.base_of_image;
        let end = base + m.size_of_image as u64;
        if rip >= base && rip < end {
            let category = categorize_module_name(&m.name, &m.full_path);
            return Some(FaultingModuleInfo {
                name: m.name.clone(),
                full_path: m.full_path.clone(),
                base_address: base,
                size: m.size_of_image as u64,
                rip_offset: rip - base,
                category,
            });
        }
    }
    None
}

fn categorize_module_name(name: &str, path: &str) -> String {
    let lower_name = name.to_lowercase();
    let lower_path = path.to_lowercase();

    if lower_name == "ue4ss.dll" || lower_path.contains("ue4ss") {
        "ue4ss".to_string()
    } else if lower_path.contains("palschema") || (lower_name == "main.dll" && lower_path.contains("mods")) {
        "palschema".to_string()
    } else if lower_name.contains("rtss") || lower_name.contains("discordhook") || lower_name.contains("obs-hook") {
        "overlay".to_string()
    } else if lower_name == "dxgi.dll" || lower_name.contains("reshade") || lower_name.contains("d3d12") {
        "directx".to_string()
    } else if lower_name.contains("palworld-win64-shipping") {
        "vanilla".to_string()
    } else if lower_name.contains("kernelbase") || lower_name.contains("ntdll") || lower_name.contains("kernel32") || lower_name.contains("vcruntime140") {
        "system".to_string()
    } else if lower_path.contains("mods") || lower_path.contains("binaries") {
        "mod_pak".to_string()
    } else {
        "unknown".to_string()
    }
}

fn read_minidump_string(file: &mut File, rva: u64) -> Option<String> {
    let current_pos = file.stream_position().ok()?;
    if file.seek(SeekFrom::Start(rva)).is_err() {
        return None;
    }

    let length_bytes = file.read_u32::<LittleEndian>().ok()? as usize;
    if length_bytes == 0 || length_bytes > 4096 {
        let _ = file.seek(SeekFrom::Start(current_pos));
        return None;
    }

    let char_count = length_bytes / 2;
    let mut u16_chars = Vec::with_capacity(char_count);
    for _ in 0..char_count {
        if let Ok(c) = file.read_u16::<LittleEndian>() {
            u16_chars.push(c);
        } else {
            break;
        }
    }

    let _ = file.seek(SeekFrom::Start(current_pos));
    String::from_utf16(&u16_chars).ok()
}
