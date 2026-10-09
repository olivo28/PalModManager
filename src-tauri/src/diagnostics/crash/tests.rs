use super::types::*;
use super::xml_parser::*;
use super::heuristics::*;

#[test]
fn test_xml_parser_crash_context() {
    let mock_xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<FGenericCrashContext>
    <GameVersion>0.3.5.0</GameVersion>
    <EngineVersion>5.1.1-0+UE5</EngineVersion>
    <CrashType>Crash</CrashType>
    <ErrorMessage>LowLevelFatalError [.../IPlatformFilePak.cpp] [Line: 4210] Retry was NOT successful</ErrorMessage>
    <SecondsSinceStart>42</SecondsSinceStart>
    <CallStack>Palworld_Win64_Shipping!FAsyncLoadingThread::Tick() [0x142010203]</CallStack>
    <MemoryStats>
        <TotalPhysical>17179869184</TotalPhysical>
        <AvailablePhysical>2147483648</AvailablePhysical>
        <TotalVirtual>34359738368</TotalVirtual>
        <AvailableVirtual>17179869184</AvailableVirtual>
    </MemoryStats>
    <Thread>
        <ThreadName>FAsyncLoadingThread</ThreadName>
        <IsCrashed>true</IsCrashed>
    </Thread>
    <Modules>
        <Module>
            <Name>UE4SS.dll</Name>
            <Path>C:\Palworld\Pal\Binaries\Win64\UE4SS.dll</Path>
        </Module>
    </Modules>
</FGenericCrashContext>
"#;

    let parsed = parse_crash_context_xml(mock_xml);
    assert_eq!(parsed.crash_type, "Crash");
    assert_eq!(parsed.seconds_since_start, 42);
    assert_eq!(parsed.crashed_thread_name.as_deref(), Some("FAsyncLoadingThread"));
    assert!(parsed.error_message.contains("IPlatformFilePak.cpp"));
    assert_eq!(parsed.memory_stats.total_physical_gb, Some(16.0));
    assert_eq!(parsed.memory_stats.available_physical_gb, Some(2.0));
    assert!(!parsed.memory_stats.is_oom_risk);
    assert_eq!(parsed.loaded_modules.len(), 1);
}

#[test]
fn test_heuristics_pak_corruption() {
    let mock_xml = CrashContextXml {
        error_message: "LowLevelFatalError [IPlatformFilePak.cpp] [Line: 123] Retry was not successful".to_string(),
        crash_type: "Crash".to_string(),
        seconds_since_start: 35,
        crashed_thread_name: Some("FAsyncLoadingThread".to_string()),
        crashed_thread_callstack: None,
        callstack_raw: String::new(),
        memory_stats: CrashMemoryStats {
            total_physical_gb: Some(16.0),
            available_physical_gb: Some(8.0),
            total_virtual_gb: None,
            available_virtual_gb: None,
            is_oom_risk: false,
        },
        loaded_modules: Vec::new(),
        game_version: Some("0.3.5".to_string()),
        engine_version: Some("5.1.1".to_string()),
        raw_xml_content: None,
    };

    let report = evaluate_crash_heuristics(
        Some(&mock_xml),
        None,
        &[],
        &[],
        std::path::Path::new("C:/Crashes/UECC-Windows-1234"),
    );

    assert_eq!(report.primary_cause, "mod_pak_corrupt");
    assert_eq!(report.severity, "Critical");
    assert!(report.human_summary.contains(".pak está corrupto"));
}

#[test]
fn test_heuristics_oom_detection() {
    let mock_xml = CrashContextXml {
        error_message: "Ran out of memory allocation".to_string(),
        crash_type: "Crash".to_string(),
        seconds_since_start: 120,
        crashed_thread_name: Some("GameThread".to_string()),
        crashed_thread_callstack: None,
        callstack_raw: String::new(),
        memory_stats: CrashMemoryStats {
            total_physical_gb: Some(8.0),
            available_physical_gb: Some(0.2), // < 500 MB
            total_virtual_gb: None,
            available_virtual_gb: None,
            is_oom_risk: true,
        },
        loaded_modules: Vec::new(),
        game_version: None,
        engine_version: None,
        raw_xml_content: None,
    };

    let report = evaluate_crash_heuristics(
        Some(&mock_xml),
        None,
        &[],
        &[],
        std::path::Path::new("C:/Crashes/UECC-Windows-9999"),
    );

    assert_eq!(report.primary_cause, "out_of_memory");
    assert_eq!(report.severity, "Fatal");
}

#[test]
fn test_parse_specific_crash() {
    let specific_dmp = std::path::PathBuf::from(r"C:\Users\Antikux\AppData\Local\Pal\Saved\Crashes\UECC-Windows-94DBFB374484E72176EB53B26019799E_0000\UEMinidump.dmp");
    let specific_xml = std::path::PathBuf::from(r"C:\Users\Antikux\AppData\Local\Pal\Saved\Crashes\UECC-Windows-94DBFB374484E72176EB53B26019799E_0000\CrashContext.runtime-xml");
    
    let xml_info = if specific_xml.exists() {
        parse_crash_context_file(&specific_xml)
    } else {
        None
    };

    let dmp_info = if specific_dmp.exists() {
        super::dmp_parser::parse_minidump_file(&specific_dmp)
    } else {
        None
    };

    if let Some(ref x) = xml_info {
        println!("XML PARSED MEMORY: Available={:?}, Total={:?}", x.memory_stats.available_physical_gb, x.memory_stats.total_physical_gb);
        println!("XML CRASHED THREAD: {:?}", x.crashed_thread_name);
        println!("XML CRASHED THREAD CALLSTACK: {:?}", x.crashed_thread_callstack.as_ref().map(|s| &s[..s.len().min(300)]));
        println!("XML CALLSTACK RAW: {:?}", &x.callstack_raw[..x.callstack_raw.len().min(300)]);
    }

    if let Some(ref d) = dmp_info {
        println!("DMP EXCEPTION MEANING: {}", d.exception_meaning);
        println!("DMP FAULTING MODULE: {:?}", d.faulting_module);
        println!("DMP TOTAL MODULES: {}", d.raw_modules.len());
        for m in d.raw_modules.iter().take(5) {
            println!("  MODULE: name='{}', base='{}', size={}, full_path='{}'", m.name, m.base_address_hex, m.size_bytes, m.full_path);
        }
    }

    let report = evaluate_crash_heuristics(
        xml_info.as_ref(),
        dmp_info.as_ref(),
        &[],
        &[],
        std::path::Path::new(r"C:\Users\Antikux\AppData\Local\Pal\Saved\Crashes\UECC-Windows-94DBFB374484E72176EB53B26019799E_0000"),
    );

    println!("EVALUATED REPORT: primary_cause={}, origin_category={}, culprit_name={:?}, culprit_type={:?}",
        report.primary_cause, report.origin_category, report.culprit_name, report.culprit_type);

    if xml_info.is_some() {
        assert_eq!(report.origin_category, "pak_mod");
        assert!(report.culprit_name.is_some());
    }
}
