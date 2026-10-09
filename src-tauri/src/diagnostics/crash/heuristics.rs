use std::path::Path;
use super::types::{CrashActionSuggestion, CrashContextXml, MinidumpInfo, UnifiedCrashReport, ActiveModSnapshot};

// Infer probable cause by correlating XML, Minidump, and active mod manifests
pub fn evaluate_crash_heuristics(
    xml: Option<&CrashContextXml>,
    minidump: Option<&MinidumpInfo>,
    active_mods: &[ActiveModSnapshot],
    recent_errors: &[String],
    crash_folder_path: &Path,
) -> UnifiedCrashReport {
    let mut primary_cause = "game_engine".to_string();
    let mut culprit_name: Option<String> = None;
    let mut culprit_type: Option<String> = None;
    let mut human_summary = String::new();
    let mut technical_details = String::new();
    let mut suggestions: Vec<CrashActionSuggestion> = Vec::new();
    let mut severity = "Critical".to_string();

    // Check Out-Of-Memory conditions
    if let Some(x) = xml {
        if x.memory_stats.is_oom_risk {
            primary_cause = "out_of_memory".to_string();
            severity = "Fatal".to_string();
            let avail = x.memory_stats.available_physical_gb.unwrap_or(0.0);
            human_summary = format!(
                "El juego agotó la memoria RAM del sistema ({:.2} GB disponibles al crashear). Ocurrió un fallo por memoria insuficiente.",
                avail
            );
            technical_details = format!(
                "Available Physical RAM: {:.2} GB, Total Physical RAM: {:.2} GB",
                avail,
                x.memory_stats.total_physical_gb.unwrap_or(0.0)
            );
            suggestions.push(CrashActionSuggestion {
                id: "close_apps".to_string(),
                label_key: "crash_doctor.action_close_background_apps".to_string(),
                description_key: "crash_doctor.action_close_background_apps_desc".to_string(),
                action_type: "none".to_string(),
                target_id: None,
                target_path: None,
            });
        }
    }

    // 2. Match faulting module against UE4SS, PalSchema, overlays, and graphics hooks
    if primary_cause == "game_engine" {
        if let Some(md) = minidump {
            if let Some(ref fm) = md.faulting_module {
                match fm.category.as_str() {
                    "ue4ss" => {
                        primary_cause = "ue4ss_script".to_string();
                        culprit_name = Some(fm.name.clone());
                        culprit_type = Some("UE4SS Framework".to_string());
                        human_summary = "Fallo crítico en el motor de scripting UE4SS. Es muy probable que un script Lua o hook C++ esté desactualizado.".to_string();
                        technical_details = format!(
                            "Faulting Module: {} at offset 0x{:X} (Exception: {})",
                            fm.name, fm.rip_offset, md.exception_meaning
                        );
                        suggestions.push(CrashActionSuggestion {
                            id: "update_ue4ss".to_string(),
                            label_key: "crash_doctor.action_check_ue4ss_update".to_string(),
                            description_key: "crash_doctor.action_check_ue4ss_update_desc".to_string(),
                            action_type: "update_ue4ss".to_string(),
                            target_id: None,
                            target_path: None,
                        });
                    }
                    "palschema" => {
                        primary_cause = "palschema_outdated".to_string();
                        culprit_name = Some("PalSchema".to_string());
                        culprit_type = Some("Schema Injector".to_string());
                        human_summary = "Fallo en el inyector de schemas PalSchema. Ocurrió un conflicto de offsets de memoria con la versión actual del juego.".to_string();
                        technical_details = format!("Faulting in PalSchema binary: {}", fm.full_path);
                    }
                    "overlay" => {
                        primary_cause = "overlay_conflict".to_string();
                        culprit_name = Some(fm.name.clone());
                        culprit_type = Some("External Overlay".to_string());
                        human_summary = format!(
                            "Conflicto de inyección causado por una aplicación externa ({}) como RivaTuner, MSI Afterburner o Discord.",
                            fm.name
                        );
                        technical_details = format!("Injected hook crashed: {} ({})", fm.name, fm.full_path);
                    }
                    "directx" => {
                        primary_cause = "directx_error".to_string();
                        culprit_name = Some(fm.name.clone());
                        culprit_type = Some("Graphics Pipeline / ReShade".to_string());
                        human_summary = "Fallo en el pipeline gráfico DirectX 12 o ReShade. Posible incompatibilidad de shaders o inyección gráfica.".to_string();
                        technical_details = format!("DirectX / Hook module: {}", fm.name);
                        suggestions.push(CrashActionSuggestion {
                            id: "purge_reshade".to_string(),
                            label_key: "crash_doctor.action_remove_reshade".to_string(),
                            description_key: "crash_doctor.action_remove_reshade_desc".to_string(),
                            action_type: "purge_reshade".to_string(),
                            target_id: None,
                            target_path: None,
                        });
                    }
                    "vanilla" => {
                        culprit_name = Some("Palworld-Win64-Shipping.exe".to_string());
                        culprit_type = Some("Palworld Game Engine".to_string());
                    }
                    "system" => {
                        // Fault occurred in KERNELBASE.dll / RaiseException (Assertion)
                        culprit_name = Some("Palworld-Win64-Shipping.exe".to_string());
                        culprit_type = Some("Game Assertion / Core Engine".to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    // Fallback: If culprit_name is still None, inspect callstack for UE4SS or Game Engine
    if culprit_name.is_none() {
        if let Some(x) = xml {
            let combined_trace = format!("{} {}", x.callstack_raw, x.crashed_thread_callstack.as_deref().unwrap_or(""));
            let trace_lower = combined_trace.to_lowercase();
            if trace_lower.contains("ue4ss") {
                primary_cause = "ue4ss_script".to_string();
                culprit_name = Some("UE4SS.dll".to_string());
                culprit_type = Some("UE4SS Framework".to_string());
                human_summary = "Fallo crítico en el motor de scripting UE4SS detectado en la traza del hilo que crasheó.".to_string();
            } else if trace_lower.contains("palworld-win64-shipping") || trace_lower.contains("palworld_win64_shipping") {
                culprit_name = Some("Palworld-Win64-Shipping.exe".to_string());
                culprit_type = Some("Palworld Game Engine".to_string());
            }
        }
    }

    // 3. Revisar hilos y mensajes del XML de Unreal Engine
    if let Some(x) = xml {
        let err_lower = x.error_message.to_lowercase();
        let thread_name = x.crashed_thread_name.as_deref().unwrap_or("");

        if err_lower.contains("iplatformfilepak.cpp") || err_lower.contains("retry was not successful") {
            primary_cause = "mod_pak_corrupt".to_string();
            severity = "Critical".to_string();
            culprit_name = Some("Unreal Pak File Loader".to_string());
            culprit_type = Some("Corrupt .PAK Mod".to_string());
            human_summary = "Un archivo de mod .pak está corrupto, bloqueado por el sistema de archivos o empaquetado con compresión incompatible.".to_string();
            technical_details = x.error_message.clone();
        } else if (thread_name == "FAsyncLoadingThread" || thread_name.contains("Worker")) && err_lower.contains("pak") {
            primary_cause = "mod_pak_corrupt".to_string();
            severity = "Critical".to_string();
            culprit_name = Some("Unreal Pak File Loader".to_string());
            culprit_type = Some("Corrupt .PAK Mod".to_string());
            human_summary = "El motor falló al leer un archivo de mod .pak mientras se ejecutaba un hilo de carga.".to_string();
            technical_details = x.error_message.clone();
        } else if thread_name == "FAsyncLoadingThread" && primary_cause == "game_engine" {
            primary_cause = "mod_pak_asset".to_string();
            culprit_name = Some("Async Asset Loader".to_string());
            culprit_type = Some(".PAK Asset Conflict".to_string());
            human_summary = "El juego falló mientras cargaba un modelo 3D, textura o asset visual asíncrono. El culpable suele ser un mod .pak recién añadido.".to_string();
            technical_details = format!("Crashed thread: {} | Message: {}", thread_name, x.error_message);
        } else if x.seconds_since_start < 15 && primary_cause == "game_engine" {
            primary_cause = "early_startup".to_string();
            human_summary = "El juego crasheó de inmediato en los primeros segundos de arranque sin llegar al menú principal. Suele deberse a un mod con dependencias faltantes.".to_string();
            technical_details = format!("Seconds alive: {}s | Type: {}", x.seconds_since_start, x.crash_type);
        }
    }

    // Match active mods against recent engine logs
    for mod_info in active_mods {
        let name_lower = mod_info.name.to_lowercase();
        let id_lower = mod_info.id.to_lowercase();
        
        // Scan recent errors for mod references
        for err in recent_errors {
            let err_l = err.to_lowercase();
            if err_l.contains(&name_lower) || (!id_lower.is_empty() && err_l.contains(&id_lower)) {
                culprit_name = Some(mod_info.name.clone());
                culprit_type = Some(format!("Mod [{}]", mod_info.mod_type));
                suggestions.insert(0, CrashActionSuggestion {
                    id: format!("disable_mod_{}", mod_info.id),
                    label_key: "crash_doctor.action_disable_culprit_mod".to_string(),
                    description_key: "crash_doctor.action_disable_culprit_mod_desc".to_string(),
                    action_type: "disable_mod".to_string(),
                    target_id: Some(mod_info.id.clone()),
                    target_path: None,
                });
                break;
            }
        }
    }

    // Default human summary if unspecified
    if human_summary.is_empty() {
        human_summary = "Fallo general en el ejecutable principal del juego o excepción no controlada.".to_string();
        technical_details = xml.map(|x| x.error_message.clone()).unwrap_or_else(|| "No error message captured".to_string());
    }

    // Default suggestion: Open crash folder
    suggestions.push(CrashActionSuggestion {
        id: "open_folder".to_string(),
        label_key: "crash_doctor.action_open_crash_folder".to_string(),
        description_key: "crash_doctor.action_open_crash_folder_desc".to_string(),
        action_type: "open_folder".to_string(),
        target_id: None,
        target_path: Some(crash_folder_path.to_string_lossy().to_string()),
    });

    // Extract authentic timestamp from minidump file or crash directory metadata
    let timestamp = {
        let candidates = [
            crash_folder_path.join("UEMinidump.dmp"),
            crash_folder_path.join("CrashContext.runtime-xml"),
            crash_folder_path.join("UE4SS_Crash.dmp"),
            crash_folder_path.to_path_buf(),
        ];
        let mut found_time = None;
        for path in &candidates {
            if let Ok(meta) = path.metadata() {
                if let Ok(mod_time) = meta.modified() {
                    let dt: chrono::DateTime<chrono::Local> = mod_time.into();
                    found_time = Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                    break;
                }
            }
        }
        found_time.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    };

    let crash_id = crash_folder_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown_crash")
        .to_string();

    // Compute origin_category for high-level UI classification
    let origin_category = match primary_cause.as_str() {
        "mod_pak_corrupt" | "mod_pak_asset" => "pak_mod".to_string(),
        "ue4ss_script" => "ue4ss".to_string(),
        "out_of_memory" => "out_of_memory".to_string(),
        "directx_error" => "directx".to_string(),
        "overlay_conflict" => "overlay".to_string(),
        "palschema_outdated" => "ue4ss".to_string(),
        _ => "game_engine".to_string(),
    };

    UnifiedCrashReport {
        id: crash_id,
        timestamp,
        crash_folder: crash_folder_path.to_string_lossy().to_string(),
        has_xml: xml.is_some(),
        has_dmp: minidump.is_some(),
        has_ue4ss_log: !recent_errors.is_empty(),
        severity,
        primary_cause,
        culprit_name,
        culprit_type,
        human_summary,
        technical_details,
        origin_category,
        xml_context: xml.cloned(),
        minidump: minidump.cloned(),
        recent_log_errors: recent_errors.to_vec(),
        active_mods: active_mods.to_vec(),
        suggestions,
    }
}
