use std::path::{Path, PathBuf};

// --- LOGIC TO TEST ---

fn sanitize_folder_name(name: &str) -> String {
    let mut cleaned = name.replace(|c: char| {
        c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|'
    }, "");
    cleaned = cleaned.trim().to_string();
    cleaned
}

fn determine_mod_id(nexus_mod_id: Option<u32>, folder_name: &str) -> String {
    if let Some(nexus_id) = nexus_mod_id {
        format!("{}-{}", nexus_id, folder_name)
    } else {
        folder_name.to_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum DetectedModType {
    Ue4ss,
    PalSchema,
    Pak,
    LogicMods,
    Hybrid,
    Unknown,
}

fn analyze_files_layout(files: &[&str]) -> DetectedModType {
    let mut has_lua = false;
    let mut has_json = false;
    let mut has_pak = false;
    let mut in_logicmods = false;

    for file in files {
        let lower = file.to_lowercase();
        if lower.ends_with(".lua") {
            has_lua = true;
        }
        if lower.ends_with(".json") || lower.ends_with(".jsonc") {
            has_json = true;
        }
        if lower.ends_with(".pak") {
            has_pak = true;
        }
        if lower.contains("logicmods") {
            in_logicmods = true;
        }
    }

    if has_pak && (has_lua || has_json) {
        DetectedModType::Hybrid
    } else if has_lua {
        DetectedModType::Ue4ss
    } else if has_pak && in_logicmods {
        DetectedModType::LogicMods
    } else if has_pak {
        DetectedModType::Pak
    } else if has_json {
        DetectedModType::PalSchema
    } else {
        DetectedModType::Unknown
    }
}

// --- TEST SUITE ---

fn assert_eq_custom<T: std::fmt::Debug + PartialEq>(actual: T, expected: T, msg: &str) {
    if actual == expected {
        println!("[ PASS ] {}", msg);
    } else {
        println!("[ FAIL ] {}. Expected: {:?}, Got: {:?}", msg, expected, actual);
    }
}

fn main() {
    println!("==================================================");
    println!("     RUNNING PALMODMANAGER HEURISTIC TESTS        ");
    println!("==================================================");

    // 1. Test Folder Name Sanitization
    println!("\n--- 1. Testing Folder Sanitization ---");
    assert_eq_custom(
        sanitize_folder_name("MelwenMods - Caprity *Grazing?"),
        "MelwenMods - Caprity Grazing".to_string(),
        "Should strip invalid character '*' and '?'"
    );
    assert_eq_custom(
        sanitize_folder_name(" Weapon:Proficiency <Main> | "),
        "WeaponProficiency Main".to_string(),
        "Should strip colons, brackets, pipes, and trim spaces"
    );

    // 2. Test Composite IDs for Varied Nexus Files
    println!("\n--- 2. Testing Composite Mod IDs ---");
    let main_mod_id = determine_mod_id(Some(2306), "ElementalArmorsMain");
    let addon_mod_id = determine_mod_id(Some(2306), "ElementalArmorsOptional");
    assert_eq_custom(
        main_mod_id.clone(),
        "2306-ElementalArmorsMain".to_string(),
        "Main mod composite ID"
    );
    assert_eq_custom(
        addon_mod_id.clone(),
        "2306-ElementalArmorsOptional".to_string(),
        "Addon mod composite ID"
    );
    assert_eq_custom(
        main_mod_id != addon_mod_id,
        true,
        "Composite IDs sharing the same Nexus ID must NOT collide"
    );

    // 3. Test Layout Analysis Heuristics
    println!("\n--- 3. Testing Layout Analysis Heuristics ---");
    
    // Test UE4SS Mod
    let files_ue4ss = vec![
        "ue4ss/Mods/CustomBattleTimer/Scripts/main.lua",
        "ue4ss/Mods/CustomBattleTimer/config.txt"
    ];
    assert_eq_custom(
        analyze_files_layout(&files_ue4ss),
        DetectedModType::Ue4ss,
        "Should detect UE4SS if it has .lua scripts"
    );

    // Test Pak Mod
    let files_pak = vec![
        "Pal/Content/Paks/~mods/SmallerPlantations_P.pak"
    ];
    assert_eq_custom(
        analyze_files_layout(&files_pak),
        DetectedModType::Pak,
        "Should detect Pak if it only contains .pak files"
    );

    // Test LogicMods Mod
    let files_logic = vec![
        "Pal/Content/Paks/LogicMods/WeaponProficiency.pak"
    ];
    assert_eq_custom(
        analyze_files_layout(&files_logic),
        DetectedModType::LogicMods,
        "Should detect LogicMods if .pak is in a logicmods subdirectory"
    );

    // Test Hybrid Mod (Pak + Script)
    let files_hybrid = vec![
        "Pal/Content/Paks/~mods/PalVariety_P.pak",
        "ue4ss/Mods/PalVariety/Scripts/main.lua",
        "ue4ss/Mods/PalVariety/config.json"
    ];
    assert_eq_custom(
        analyze_files_layout(&files_hybrid),
        DetectedModType::Hybrid,
        "Should detect Hybrid if it has both .pak and .lua/.json scripts"
    );

    println!("\n==================================================");
    println!("             TEST RUN COMPLETED                   ");
    println!("==================================================");
}
